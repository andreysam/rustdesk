//! Individual unattended credentials. Plaintext exists only during HTTPS enrollment.
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use hbb_common::{
    anyhow::{anyhow, bail, Context},
    config::{Config, Config2},
    log,
    password_security::symmetric_crypt,
    rand::{rngs::OsRng, RngCore},
    sodiumoxide::{base64, crypto::sign},
    tokio, ResultType,
};
use sha2::{Digest, Sha256};

const BINDING_OPTION: &str = "novodoc-credential-binding-v1";
const API_PATH: &str = "/credentials/v1/devices/enroll";
static STARTED: AtomicBool = AtomicBool::new(false);

#[derive(serde_derive::Serialize)]
struct Enrollment {
    device_id: String,
    public_key: String,
    timestamp: u64,
    nonce: String,
    password: String,
    signature: String,
}

#[derive(serde_derive::Deserialize)]
struct EnrollmentResult {
    device_id: String,
    password: String,
}

fn api_url(server: &str) -> ResultType<url::Url> {
    let server = server.trim();
    if server.is_empty() {
        bail!("RUSTDESK_SERVER_URL is required for device enrollment");
    }
    let mut url = url::Url::parse(&if server.contains("://") {
        server.to_owned()
    } else {
        format!("https://{server}")
    })?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !matches!(url.path(), "" | "/")
    {
        bail!("RUSTDESK_SERVER_URL must contain only the RustDesk server host");
    }
    url.set_scheme("https")
        .map_err(|_| anyhow!("Invalid HTTPS host"))?;
    // Rendezvous ports (e.g. 21116) are not the HTTPS API port.
    url.set_port(None)
        .map_err(|_| anyhow!("Invalid HTTPS port"))?;
    url.set_path(API_PATH);
    Ok(url)
}

fn signing_message(id: &str, timestamp: u64, nonce: &str, password: &str) -> String {
    let digest = hex::encode(Sha256::digest(password.as_bytes()));
    format!("novodoc-enroll-v1\n{id}\n{timestamp}\n{nonce}\n{digest}")
}

fn credential_binding(
    url: &url::Url,
    id: &str,
    public_key: &[u8],
    h1: &[u8],
    salt: &str,
) -> String {
    let mut hash = Sha256::new();
    for part in [
        url.as_str().as_bytes(),
        id.as_bytes(),
        public_key,
        h1,
        salt.as_bytes(),
    ] {
        hash.update(part);
        hash.update([0]);
    }
    hex::encode(hash.finalize())
}

/// Also checked at authentication time so changing ID/config cannot reuse old credentials.
pub(crate) fn password_ready() -> bool {
    let Ok(url) = api_url(crate::builtin_server::credential_server()) else {
        return false;
    };
    let (storage, salt) = Config::get_local_permanent_password_storage_and_salt();
    let Some(h1) = hbb_common::config::decode_permanent_password_h1_from_storage(&storage) else {
        return false;
    };
    let Some((_, public_key)) = Config::get_existing_key_pair() else {
        return false;
    };
    !salt.is_empty()
        && Config::get_option(BINDING_OPTION)
            == credential_binding(&url, &Config::get_id(), &public_key, &h1, &salt)
}

fn prepare_enrollment() -> ResultType<Enrollment> {
    let id = Config::get_id();
    if !(6..=64).contains(&id.len())
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        bail!("Device ID is not ready for enrollment");
    }
    let (secret, public_key) = Config::get_key_pair();
    let key = sign::SecretKey::from_slice(&secret)
        .ok_or_else(|| anyhow!("Invalid device signing key"))?;
    let mut random = [0u8; 32];
    OsRng.try_fill_bytes(&mut random)?;
    let password = base64::encode(random, base64::Variant::Original);
    OsRng.try_fill_bytes(&mut random)?;
    let nonce = hex::encode(random);
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let message = signing_message(&id, timestamp, &nonce, &password);
    let signature = sign::sign_detached(message.as_bytes(), &key);
    Ok(Enrollment {
        device_id: id,
        public_key: base64::encode(public_key, base64::Variant::Original),
        timestamp,
        nonce,
        password,
        signature: base64::encode(signature.to_bytes(), base64::Variant::Original),
    })
}

fn persist_credential(
    url: &url::Url,
    id: &str,
    public_key: &str,
    result: EnrollmentResult,
) -> ResultType<()> {
    let decoded = base64::decode(result.password.as_bytes(), base64::Variant::Original)
        .map_err(|_| anyhow!("Invalid enrolled password encoding"))?;
    if result.device_id != id || decoded.len() != 32 || Config::get_id() != id {
        bail!("Enrollment response does not match the current device");
    }
    let current_public_key = Config::get_key_pair().1;
    if base64::encode(&current_public_key, base64::Variant::Original) != public_key {
        bail!("Device identity changed during enrollment");
    }
    let salt = Config::get_salt();
    let h1 = hbb_common::config::compute_permanent_password_h1(&result.password, &salt);
    let hash_storage = format!("00{}", base64::encode(h1, base64::Variant::Original));
    let encrypted = symmetric_crypt(hash_storage.as_bytes(), true)
        .map_err(|_| anyhow!("Cannot encrypt the local password hash"))?;
    let storage = format!("01{}", base64::encode(encrypted, base64::Variant::Original));
    // This internal provisioning path bypasses the UI prohibition on password changes.
    Config::set_permanent_password_storage_for_sync(&storage, &salt)?;
    // Config::store logs write errors without returning them; verify before marking ready.
    let saved: hbb_common::toml::Value =
        hbb_common::toml::from_str(&std::fs::read_to_string(Config::file())?)?;
    if saved.get("password").and_then(|v| v.as_str()) != Some(storage.as_str())
        || saved.get("salt").and_then(|v| v.as_str()) != Some(salt.as_str())
    {
        bail!("The individual password hash was not persisted");
    }
    let binding = credential_binding(url, id, &current_public_key, &h1, &salt);
    Config::set_option(BINDING_OPTION.to_owned(), binding.clone());
    let saved_options = (|| -> ResultType<hbb_common::toml::Value> {
        Ok(hbb_common::toml::from_str(&std::fs::read_to_string(
            Config2::file(),
        )?)?)
    })();
    if saved_options.as_ref().ok().and_then(|saved| {
        saved
            .get("options")
            .and_then(|v| v.get(BINDING_OPTION))
            .and_then(|v| v.as_str())
    }) != Some(binding.as_str())
    {
        // Do not declare success when only the in-memory marker was updated.
        Config::set_option(BINDING_OPTION.to_owned(), String::new());
        bail!("Enrollment marker was not persisted");
    }
    Ok(())
}

async fn enroll(client: &reqwest::Client, url: &url::Url) -> ResultType<()> {
    let request = tokio::task::spawn_blocking(prepare_enrollment).await??;
    let body = serde_json::to_vec(&request)?;
    let mut response = client
        .post(url.clone())
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await
        .context("Credential backend request failed")?;
    if !response.status().is_success() {
        // Never log response bodies: they may contain credentials.
        bail!(
            "Credential backend returned HTTP {}",
            response.status().as_u16()
        );
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if bytes.len() + chunk.len() > 8192 {
            bail!("Credential backend response is too large");
        }
        bytes.extend_from_slice(&chunk);
    }
    let result: EnrollmentResult = serde_json::from_slice(&bytes)
        .map_err(|_| anyhow!("Invalid credential backend response"))?;
    let url = url.clone();
    tokio::task::spawn_blocking(move || {
        persist_credential(&url, &request.device_id, &request.public_key, result)
    })
    .await??;
    Ok(())
}

pub(crate) fn start() {
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    tokio::spawn(async {
        let url = match api_url(crate::builtin_server::credential_server()) {
            Ok(url) => url,
            Err(err) => {
                log::error!("Device enrollment disabled: {err}");
                return;
            }
        };
        let client = match reqwest::Client::builder()
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(15))
            .build()
        {
            Ok(client) => client,
            Err(err) => {
                log::error!("Cannot initialize credential backend client: {err}");
                return;
            }
        };
        loop {
            match tokio::task::spawn_blocking(password_ready).await {
                Ok(true) => {}
                Ok(false) => match enroll(&client, &url).await {
                    Ok(()) => log::info!("Individual device credential enrolled"),
                    Err(err) => log::warn!("Device enrollment will retry: {err}"),
                },
                Err(err) => log::error!("Cannot check device enrollment: {err}"),
            }
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_signature_vector() {
        let seed = sign::Seed::from_slice(&[7; 32]).unwrap();
        let (_, key) = sign::keypair_from_seed(&seed);
        let password = base64::encode([0x42; 32], base64::Variant::Original);
        let message = signing_message("123456789", 2000000000, &"ab".repeat(32), &password);
        assert_eq!(hex::encode(sign::sign_detached(message.as_bytes(), &key).to_bytes()), "1ad01007eab79e3a5a46a8662bfebc89485101f060b4a61b3bc8db19ca7a1565becfd780b37bc334a3ee509932ff8f57c2c5907d1c1a820cdd2e68859ecedf07");
    }

    #[test]
    fn endpoint_uses_https_domain_and_fixed_path() {
        for server in [
            "rd.example.test",
            "http://rd.example.test:21116/",
            "https://rd.example.test/",
        ] {
            assert_eq!(
                api_url(server).unwrap().as_str(),
                "https://rd.example.test/credentials/v1/devices/enroll"
            );
        }
        for server in [
            "",
            "https://user:secret@rd.example.test",
            "https://rd.example.test/other",
            "https://rd.example.test/?q=1",
            "ftp://rd.example.test",
        ] {
            assert!(api_url(server).is_err());
        }
    }

    #[test]
    fn binding_changes_with_identity_or_credential() {
        let url = api_url("rd.example.test").unwrap();
        let original = credential_binding(&url, "123456789", &[1; 32], &[2; 32], "salt");
        assert_ne!(
            original,
            credential_binding(&url, "123456780", &[1; 32], &[2; 32], "salt")
        );
        assert_ne!(
            original,
            credential_binding(&url, "123456789", &[3; 32], &[2; 32], "salt")
        );
        assert_ne!(
            original,
            credential_binding(&url, "123456789", &[1; 32], &[4; 32], "salt")
        );
    }
}

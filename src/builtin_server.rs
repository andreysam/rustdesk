use hbb_common::config::{self, keys};

const ID_SERVER: &str = option_env!("RUSTDESK_SERVER_URL").unwrap_or("");
const KEY: &str = option_env!("RUSTDESK_SERVER_KEY").unwrap_or("");
const RELAY_SERVER: &str = option_env!("RUSTDESK_SERVER_RELAY").unwrap_or("");
const API_SERVER: &str = option_env!("RUSTDESK_SERVER_API").unwrap_or("");

pub struct ApplyOnDrop;

impl Drop for ApplyOnDrop {
    fn drop(&mut self) {
        apply_builtin_server();
    }
}

pub fn apply_builtin_server() {
    apply_builtin_server_values(ID_SERVER, KEY, RELAY_SERVER, API_SERVER);
}

fn normalize_server_host(raw: &str) -> String {
    let s = raw.trim();
    let s = s
        .strip_prefix("https://")
        .or_else(|| s.strip_prefix("http://"))
        .unwrap_or(s);
    s.trim_end_matches('/').to_string()
}

fn apply_builtin_server_values(id: &str, key: &str, relay: &str, api: &str) {
    let id = normalize_server_host(id);
    if id.is_empty() {
        return;
    }
    {
        let mut overwrite = config::OVERWRITE_SETTINGS.write().unwrap();
        overwrite.insert(keys::OPTION_CUSTOM_RENDEZVOUS_SERVER.to_string(), id);
        let key = key.trim();
        if !key.is_empty() {
            overwrite.insert(keys::OPTION_KEY.to_string(), key.to_string());
        }
        let relay = normalize_server_host(relay);
        if !relay.is_empty() {
            overwrite.insert(keys::OPTION_RELAY_SERVER.to_string(), relay);
        }
        let api = api.trim().trim_end_matches('/');
        if !api.is_empty() {
            overwrite.insert(keys::OPTION_API_SERVER.to_string(), api.to_string());
        }
    }
    {
        let mut builtin = config::BUILTIN_SETTINGS.write().unwrap();
        builtin.insert(
            keys::OPTION_HIDE_NETWORK_SETTINGS.to_string(),
            "Y".to_string(),
        );
        builtin.insert(
            keys::OPTION_HIDE_SERVER_SETTINGS.to_string(),
            "Y".to_string(),
        );
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    crate::ui_interface::refresh_options();
}

#[cfg(test)]
mod tests {
    use super::*;
    use hbb_common::config::{BUILTIN_SETTINGS, OVERWRITE_SETTINGS};
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn snapshot() -> (
        std::collections::HashMap<String, String>,
        std::collections::HashMap<String, String>,
    ) {
        (
            OVERWRITE_SETTINGS.read().unwrap().clone(),
            BUILTIN_SETTINGS.read().unwrap().clone(),
        )
    }

    fn restore(
        overwrite: std::collections::HashMap<String, String>,
        builtin: std::collections::HashMap<String, String>,
    ) {
        *OVERWRITE_SETTINGS.write().unwrap() = overwrite;
        *BUILTIN_SETTINGS.write().unwrap() = builtin;
    }

    #[test]
    fn normalize_strips_scheme_and_slash() {
        assert_eq!(
            normalize_server_host("https://hbbs.example.com/"),
            "hbbs.example.com"
        );
        assert_eq!(normalize_server_host("http://10.0.0.1"), "10.0.0.1");
        assert_eq!(
            normalize_server_host("  hbbs.example.com  "),
            "hbbs.example.com"
        );
        assert_eq!(normalize_server_host(""), "");
    }

    #[test]
    fn empty_id_does_not_touch_maps() {
        let _lock = TEST_LOCK.lock().unwrap();
        let (before_o, before_b) = snapshot();
        apply_builtin_server_values("", "k", "relay.example.com", "https://api.example.com");
        let (after_o, after_b) = snapshot();
        assert_eq!(after_o, before_o);
        assert_eq!(after_b, before_b);
        restore(before_o, before_b);
    }

    #[test]
    fn applies_overwrite_and_hides_network() {
        let _lock = TEST_LOCK.lock().unwrap();
        let (before_o, before_b) = snapshot();
        apply_builtin_server_values("https://hbbs.example.com/", "test-key", "", "");
        {
            let overwrite = OVERWRITE_SETTINGS.read().unwrap();
            assert_eq!(
                overwrite
                    .get(keys::OPTION_CUSTOM_RENDEZVOUS_SERVER)
                    .map(String::as_str),
                Some("hbbs.example.com")
            );
            assert_eq!(
                overwrite.get(keys::OPTION_KEY).map(String::as_str),
                Some("test-key")
            );
            assert!(!overwrite.contains_key(keys::OPTION_RELAY_SERVER));
            assert!(!overwrite.contains_key(keys::OPTION_API_SERVER));
        }
        {
            let builtin = BUILTIN_SETTINGS.read().unwrap();
            assert_eq!(
                builtin
                    .get(keys::OPTION_HIDE_NETWORK_SETTINGS)
                    .map(String::as_str),
                Some("Y")
            );
            assert_eq!(
                builtin
                    .get(keys::OPTION_HIDE_SERVER_SETTINGS)
                    .map(String::as_str),
                Some("Y")
            );
        }
        restore(before_o, before_b);
    }

    #[test]
    fn optional_relay_and_api_are_written() {
        let _lock = TEST_LOCK.lock().unwrap();
        let (before_o, before_b) = snapshot();
        apply_builtin_server_values(
            "hbbs.example.com",
            "test-key",
            "https://hbbr.example.com/",
            "https://api.example.com/",
        );
        let overwrite = OVERWRITE_SETTINGS.read().unwrap();
        assert_eq!(
            overwrite.get(keys::OPTION_RELAY_SERVER).map(String::as_str),
            Some("hbbr.example.com")
        );
        assert_eq!(
            overwrite.get(keys::OPTION_API_SERVER).map(String::as_str),
            Some("https://api.example.com")
        );
        drop(overwrite);
        restore(before_o, before_b);
    }

    #[test]
    fn apply_is_idempotent() {
        let _lock = TEST_LOCK.lock().unwrap();
        let (before_o, before_b) = snapshot();
        apply_builtin_server_values("hbbs.example.com", "k", "", "");
        let mid = snapshot();
        apply_builtin_server_values("hbbs.example.com", "k", "", "");
        let end = snapshot();
        assert_eq!(mid, end);
        restore(before_o, before_b);
    }

    #[test]
    fn compile_time_constants_lock_when_present() {
        let _lock = TEST_LOCK.lock().unwrap();
        let (before_o, before_b) = snapshot();
        apply_builtin_server();
        if normalize_server_host(ID_SERVER).is_empty() {
            let (after_o, after_b) = snapshot();
            assert_eq!(after_o, before_o);
            assert_eq!(after_b, before_b);
            restore(before_o, before_b);
            return;
        }
        let overwrite = OVERWRITE_SETTINGS.read().unwrap();
        assert!(overwrite.contains_key(keys::OPTION_CUSTOM_RENDEZVOUS_SERVER));
        assert!(!overwrite
            .get(keys::OPTION_CUSTOM_RENDEZVOUS_SERVER)
            .map(|s| s.is_empty())
            .unwrap_or(true));
        if !KEY.trim().is_empty() {
            assert!(overwrite.contains_key(keys::OPTION_KEY));
        }
        drop(overwrite);
        let builtin = BUILTIN_SETTINGS.read().unwrap();
        assert_eq!(
            builtin
                .get(keys::OPTION_HIDE_NETWORK_SETTINGS)
                .map(String::as_str),
            Some("Y")
        );
        drop(builtin);
        restore(before_o, before_b);
    }
}

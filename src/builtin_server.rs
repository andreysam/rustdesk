use hbb_common::config::{self, keys};

const ID_SERVER: &str = match option_env!("RUSTDESK_SERVER_URL") {
    Some(v) => v,
    None => "",
};
const KEY: &str = match option_env!("RUSTDESK_SERVER_KEY") {
    Some(v) => v,
    None => "",
};
const RELAY_SERVER: &str = match option_env!("RUSTDESK_SERVER_RELAY") {
    Some(v) => v,
    None => "",
};
const API_SERVER: &str = match option_env!("RUSTDESK_SERVER_API") {
    Some(v) => v,
    None => "",
};
const CLIENT_PASSWORD: &str = match option_env!("RUSTDESK_CLIENT_PASSWORD") {
    Some(v) => v,
    None => "",
};

pub struct ApplyOnDrop;

impl Drop for ApplyOnDrop {
    fn drop(&mut self) {
        apply_builtin_server();
    }
}

pub fn apply_builtin_server() {
    apply_builtin_server_values(ID_SERVER, KEY, RELAY_SERVER, API_SERVER);
    apply_unattended_client_values(CLIENT_PASSWORD);
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

fn apply_unattended_client_values(password: &str) {
    {
        let mut hard = config::HARD_SETTINGS.write().unwrap();
        hard.insert("conn-type".to_string(), "incoming".to_string());
        hard.insert("disable-account".to_string(), "Y".to_string());
        let password = password.trim();
        if !password.is_empty() {
            hard.insert("password".to_string(), password.to_string());
        }
    }
    {
        let mut overwrite = config::OVERWRITE_SETTINGS.write().unwrap();
        overwrite.insert(keys::OPTION_ACCESS_MODE.to_string(), "full".to_string());
        overwrite.insert(
            keys::OPTION_VERIFICATION_METHOD.to_string(),
            "use-permanent-password".to_string(),
        );
        overwrite.insert(keys::OPTION_APPROVE_MODE.to_string(), "password".to_string());
    }
    {
        let mut builtin = config::BUILTIN_SETTINGS.write().unwrap();
        builtin.insert(
            keys::OPTION_HIDE_SECURITY_SETTINGS.to_string(),
            "Y".to_string(),
        );
        builtin.insert(
            keys::OPTION_DISABLE_CHANGE_PERMANENT_PASSWORD.to_string(),
            "Y".to_string(),
        );
        builtin.insert(
            keys::OPTION_REMOVE_PRESET_PASSWORD_WARNING.to_string(),
            "Y".to_string(),
        );
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    crate::ui_interface::refresh_options();
}

#[cfg(test)]
mod tests {
    use super::*;
    use hbb_common::config::{BUILTIN_SETTINGS, HARD_SETTINGS, OVERWRITE_SETTINGS};
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn snapshot_server() -> (
        std::collections::HashMap<String, String>,
        std::collections::HashMap<String, String>,
    ) {
        (
            OVERWRITE_SETTINGS.read().unwrap().clone(),
            BUILTIN_SETTINGS.read().unwrap().clone(),
        )
    }

    fn restore_server(
        overwrite: std::collections::HashMap<String, String>,
        builtin: std::collections::HashMap<String, String>,
    ) {
        *OVERWRITE_SETTINGS.write().unwrap() = overwrite;
        *BUILTIN_SETTINGS.write().unwrap() = builtin;
    }

    fn snapshot_unattended() -> (
        std::collections::HashMap<String, String>,
        std::collections::HashMap<String, String>,
        std::collections::HashMap<String, String>,
    ) {
        (
            HARD_SETTINGS.read().unwrap().clone(),
            OVERWRITE_SETTINGS.read().unwrap().clone(),
            BUILTIN_SETTINGS.read().unwrap().clone(),
        )
    }

    fn restore_unattended(
        hard: std::collections::HashMap<String, String>,
        overwrite: std::collections::HashMap<String, String>,
        builtin: std::collections::HashMap<String, String>,
    ) {
        *HARD_SETTINGS.write().unwrap() = hard;
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
        let (before_o, before_b) = snapshot_server();
        apply_builtin_server_values("", "k", "relay.example.com", "https://api.example.com");
        let (after_o, after_b) = snapshot_server();
        assert_eq!(after_o, before_o);
        assert_eq!(after_b, before_b);
        restore_server(before_o, before_b);
    }

    #[test]
    fn applies_overwrite_and_hides_network() {
        let _lock = TEST_LOCK.lock().unwrap();
        let (before_o, before_b) = snapshot_server();
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
        restore_server(before_o, before_b);
    }

    #[test]
    fn optional_relay_and_api_are_written() {
        let _lock = TEST_LOCK.lock().unwrap();
        let (before_o, before_b) = snapshot_server();
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
        restore_server(before_o, before_b);
    }

    #[test]
    fn apply_is_idempotent() {
        let _lock = TEST_LOCK.lock().unwrap();
        let (before_o, before_b) = snapshot_server();
        apply_builtin_server_values("hbbs.example.com", "k", "", "");
        let mid = snapshot_server();
        apply_builtin_server_values("hbbs.example.com", "k", "", "");
        let end = snapshot_server();
        assert_eq!(mid, end);
        restore_server(before_o, before_b);
    }

    #[test]
    fn compile_time_constants_lock_when_present() {
        let _lock = TEST_LOCK.lock().unwrap();
        let (before_o, before_b) = snapshot_server();
        apply_builtin_server_values(ID_SERVER, KEY, RELAY_SERVER, API_SERVER);
        if normalize_server_host(ID_SERVER).is_empty() {
            let (after_o, after_b) = snapshot_server();
            assert_eq!(after_o, before_o);
            assert_eq!(after_b, before_b);
            restore_server(before_o, before_b);
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
        restore_server(before_o, before_b);
    }

    #[test]
    fn unattended_sets_incoming_full_access_and_hides_security() {
        let _lock = TEST_LOCK.lock().unwrap();
        let (before_h, before_o, before_b) = snapshot_unattended();
        apply_unattended_client_values("");
        {
            let hard = HARD_SETTINGS.read().unwrap();
            assert_eq!(hard.get("conn-type").map(String::as_str), Some("incoming"));
            assert_eq!(hard.get("disable-account").map(String::as_str), Some("Y"));
            assert!(!hard.contains_key("password"));
        }
        {
            let overwrite = OVERWRITE_SETTINGS.read().unwrap();
            assert_eq!(
                overwrite.get(keys::OPTION_ACCESS_MODE).map(String::as_str),
                Some("full")
            );
            assert_eq!(
                overwrite
                    .get(keys::OPTION_VERIFICATION_METHOD)
                    .map(String::as_str),
                Some("use-permanent-password")
            );
            assert_eq!(
                overwrite.get(keys::OPTION_APPROVE_MODE).map(String::as_str),
                Some("password")
            );
        }
        {
            let builtin = BUILTIN_SETTINGS.read().unwrap();
            assert_eq!(
                builtin
                    .get(keys::OPTION_HIDE_SECURITY_SETTINGS)
                    .map(String::as_str),
                Some("Y")
            );
            assert_eq!(
                builtin
                    .get(keys::OPTION_DISABLE_CHANGE_PERMANENT_PASSWORD)
                    .map(String::as_str),
                Some("Y")
            );
            assert_eq!(
                builtin
                    .get(keys::OPTION_REMOVE_PRESET_PASSWORD_WARNING)
                    .map(String::as_str),
                Some("Y")
            );
        }
        restore_unattended(before_h, before_o, before_b);
    }

    #[test]
    fn unattended_writes_preset_password_when_non_empty() {
        let _lock = TEST_LOCK.lock().unwrap();
        let (before_h, before_o, before_b) = snapshot_unattended();
        apply_unattended_client_values("test-client-password");
        let hard = HARD_SETTINGS.read().unwrap();
        assert_eq!(
            hard.get("password").map(String::as_str),
            Some("test-client-password")
        );
        drop(hard);
        restore_unattended(before_h, before_o, before_b);
    }

    #[test]
    fn unattended_empty_password_does_not_write_password_key() {
        let _lock = TEST_LOCK.lock().unwrap();
        let (before_h, before_o, before_b) = snapshot_unattended();
        apply_unattended_client_values("   ");
        assert!(!HARD_SETTINGS.read().unwrap().contains_key("password"));
        restore_unattended(before_h, before_o, before_b);
    }

    #[test]
    fn apply_builtin_server_includes_unattended_when_password_empty() {
        let _lock = TEST_LOCK.lock().unwrap();
        let (before_h, before_o, before_b) = snapshot_unattended();
        apply_builtin_server();
        assert_eq!(
            HARD_SETTINGS
                .read()
                .unwrap()
                .get("conn-type")
                .map(String::as_str),
            Some("incoming")
        );
        restore_unattended(before_h, before_o, before_b);
    }
}

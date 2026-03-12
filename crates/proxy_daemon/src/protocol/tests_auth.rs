#[cfg(test)]
mod tests {
    use crate::protocol::*;

    #[test]
    fn test_proxy_auth_config_debug_masks_password() {
        let config = ProxyAuthConfig {
            enabled: true,
            username: "admin".to_string(),
            password: "super_secret_password".to_string(),
        };
        let debug_output = format!("{:?}", config);
        assert!(
            !debug_output.contains("super_secret_password"),
            "Debug output must not contain the actual password"
        );
        assert!(
            debug_output.contains("****"),
            "Debug output must mask the password with ****"
        );
        assert!(debug_output.contains("admin"));
    }

    #[test]
    fn test_proxy_auth_config_serde_roundtrip() {
        let config = ProxyAuthConfig {
            enabled: true,
            username: "admin".to_string(),
            password: "secret123".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ProxyAuthConfig = serde_json::from_str(&json).unwrap();
        assert!(deserialized.enabled);
        assert_eq!(deserialized.username, "admin");
        assert_eq!(deserialized.password, "secret123");
    }

    #[test]
    fn test_proxy_auth_expected_basic_header() {
        let config = ProxyAuthConfig {
            enabled: true,
            username: "admin".to_string(),
            password: "secret".to_string(),
        };
        let header = config.expected_basic_header();
        assert_eq!(header, "Basic YWRtaW46c2VjcmV0");
    }

    #[test]
    fn test_proxy_auth_validate_success() {
        let config = ProxyAuthConfig {
            enabled: true,
            username: "admin".to_string(),
            password: "secret".to_string(),
        };
        assert!(config.validate_proxy_auth(Some("Basic YWRtaW46c2VjcmV0")));
    }

    #[test]
    fn test_proxy_auth_validate_failure_wrong_credentials() {
        let config = ProxyAuthConfig {
            enabled: true,
            username: "admin".to_string(),
            password: "secret".to_string(),
        };
        assert!(!config.validate_proxy_auth(Some("Basic d3Jvbmc6Y3JlZHM=")));
    }

    #[test]
    fn test_proxy_auth_validate_failure_no_header() {
        let config = ProxyAuthConfig {
            enabled: true,
            username: "admin".to_string(),
            password: "secret".to_string(),
        };
        assert!(!config.validate_proxy_auth(None));
    }

    #[test]
    fn test_proxy_auth_validate_disabled_always_passes() {
        let config = ProxyAuthConfig {
            enabled: false,
            username: "admin".to_string(),
            password: "secret".to_string(),
        };
        assert!(config.validate_proxy_auth(None));
        assert!(config.validate_proxy_auth(Some("garbage")));
    }

    #[test]
    fn test_proxy_auth_validate_empty_username_always_passes() {
        let config = ProxyAuthConfig {
            enabled: true,
            username: String::new(),
            password: "secret".to_string(),
        };
        assert!(config.validate_proxy_auth(None));
    }

    #[test]
    fn test_update_proxy_auth_command_serialize() {
        let cmd = ClientCommand::UpdateProxyAuth {
            config: ProxyAuthConfig {
                enabled: true,
                username: "user".to_string(),
                password: "pass".to_string(),
            },
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("update_proxy_auth"));
        assert!(json.contains("user"));

        let deserialized: ClientCommand = serde_json::from_str(&json).unwrap();
        match deserialized {
            ClientCommand::UpdateProxyAuth { config } => {
                assert!(config.enabled);
                assert_eq!(config.username, "user");
                assert_eq!(config.password, "pass");
            }
            _ => panic!("Expected UpdateProxyAuth"),
        }
    }
}

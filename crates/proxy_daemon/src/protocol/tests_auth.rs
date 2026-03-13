mod tests {
    use crate::protocol::*;

    fn basic_config() -> ProxyAuthConfig {
        ProxyAuthConfig {
            enabled: true,
            method: AuthMethod::Basic,
            username: "admin".to_string(),
            password: "secret".to_string(),
            token: None,
        }
    }

    fn bearer_config() -> ProxyAuthConfig {
        ProxyAuthConfig {
            enabled: true,
            method: AuthMethod::Bearer,
            username: String::new(),
            password: String::new(),
            token: Some("my-bearer-token".to_string()),
        }
    }

    fn api_key_config() -> ProxyAuthConfig {
        ProxyAuthConfig {
            enabled: true,
            method: AuthMethod::ApiKey,
            username: String::new(),
            password: String::new(),
            token: Some("my-api-key-123".to_string()),
        }
    }

    #[test]
    fn test_proxy_auth_config_debug_masks_password() {
        let config = ProxyAuthConfig {
            enabled: true,
            method: AuthMethod::Basic,
            username: "admin".to_string(),
            password: "super_secret_password".to_string(),
            token: Some("secret_token".to_string()),
        };
        let debug_output = format!("{:?}", config);
        assert!(
            !debug_output.contains("super_secret_password"),
            "Debug output must not contain the actual password"
        );
        assert!(
            !debug_output.contains("secret_token"),
            "Debug output must not contain the actual token"
        );
        assert!(
            debug_output.contains("****"),
            "Debug output must mask the password with ****"
        );
        assert!(debug_output.contains("admin"));
    }

    #[test]
    fn test_proxy_auth_config_serde_roundtrip() {
        let config = basic_config();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ProxyAuthConfig = serde_json::from_str(&json).unwrap();
        assert!(deserialized.enabled);
        assert_eq!(deserialized.username, "admin");
        assert_eq!(deserialized.password, "secret");
        assert_eq!(deserialized.method, AuthMethod::Basic);
    }

    #[test]
    fn test_proxy_auth_config_serde_default_method() {
        let json = r#"{"enabled":true,"username":"admin","password":"pass"}"#;
        let config: ProxyAuthConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.method, AuthMethod::Basic);
        assert!(config.token.is_none());
    }

    #[test]
    fn test_proxy_auth_expected_basic_header() {
        let config = basic_config();
        let header = config.expected_basic_header();
        assert_eq!(header, "Basic YWRtaW46c2VjcmV0");
    }

    // --- Basic 인증 테스트 ---

    #[test]
    fn test_basic_auth_validate_success() {
        let config = basic_config();
        assert!(config.validate_proxy_auth(Some("Basic YWRtaW46c2VjcmV0")));
    }

    #[test]
    fn test_basic_auth_validate_failure_wrong_credentials() {
        let config = basic_config();
        assert!(!config.validate_proxy_auth(Some("Basic d3Jvbmc6Y3JlZHM=")));
    }

    #[test]
    fn test_basic_auth_validate_failure_no_header() {
        let config = basic_config();
        assert!(!config.validate_proxy_auth(None));
    }

    #[test]
    fn test_basic_auth_validate_disabled_always_passes() {
        let mut config = basic_config();
        config.enabled = false;
        assert!(config.validate_proxy_auth(None));
        assert!(config.validate_proxy_auth(Some("garbage")));
    }

    #[test]
    fn test_basic_auth_validate_empty_username_always_passes() {
        let mut config = basic_config();
        config.username = String::new();
        assert!(config.validate_proxy_auth(None));
    }

    // --- Bearer 인증 테스트 ---

    #[test]
    fn test_bearer_auth_validate_success() {
        let config = bearer_config();
        assert!(config.validate_proxy_auth(Some("Bearer my-bearer-token")));
    }

    #[test]
    fn test_bearer_auth_validate_failure_wrong_token() {
        let config = bearer_config();
        assert!(!config.validate_proxy_auth(Some("Bearer wrong-token")));
    }

    #[test]
    fn test_bearer_auth_validate_failure_no_header() {
        let config = bearer_config();
        assert!(!config.validate_proxy_auth(None));
    }

    #[test]
    fn test_bearer_auth_validate_failure_basic_format() {
        let config = bearer_config();
        assert!(!config.validate_proxy_auth(Some("Basic YWRtaW46c2VjcmV0")));
    }

    #[test]
    fn test_bearer_auth_empty_token_always_passes() {
        let mut config = bearer_config();
        config.token = Some(String::new());
        assert!(config.validate_proxy_auth(None));
    }

    #[test]
    fn test_bearer_auth_no_token_always_passes() {
        let mut config = bearer_config();
        config.token = None;
        assert!(config.validate_proxy_auth(None));
    }

    // --- ApiKey 인증 테스트 ---

    #[test]
    fn test_api_key_validate_success() {
        let config = api_key_config();
        assert!(config.validate_proxy_auth(Some("ApiKey my-api-key-123")));
    }

    #[test]
    fn test_api_key_validate_failure_wrong_key() {
        let config = api_key_config();
        assert!(!config.validate_proxy_auth(Some("ApiKey wrong-key")));
    }

    #[test]
    fn test_api_key_validate_failure_no_header() {
        let config = api_key_config();
        assert!(!config.validate_proxy_auth(None));
    }

    #[test]
    fn test_api_key_empty_token_always_passes() {
        let mut config = api_key_config();
        config.token = Some(String::new());
        assert!(config.validate_proxy_auth(None));
    }

    #[test]
    fn test_api_key_no_token_always_passes() {
        let mut config = api_key_config();
        config.token = None;
        assert!(config.validate_proxy_auth(None));
    }

    // --- AuthMethod serde 테스트 ---

    #[test]
    fn test_auth_method_serde_roundtrip() {
        for method in [AuthMethod::Basic, AuthMethod::Bearer, AuthMethod::ApiKey] {
            let json = serde_json::to_string(&method).unwrap();
            let deserialized: AuthMethod = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, method);
        }
    }

    // --- Command 직렬화 테스트 ---

    #[test]
    fn test_update_proxy_auth_command_serialize() {
        let cmd = ClientCommand::UpdateProxyAuth {
            config: basic_config(),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("update_proxy_auth"));
        assert!(json.contains("admin"));

        let deserialized: ClientCommand = serde_json::from_str(&json).unwrap();
        match deserialized {
            ClientCommand::UpdateProxyAuth { config } => {
                assert!(config.enabled);
                assert_eq!(config.username, "admin");
                assert_eq!(config.password, "secret");
                assert_eq!(config.method, AuthMethod::Basic);
            }
            _ => panic!("Expected UpdateProxyAuth"),
        }
    }

    #[test]
    fn test_update_proxy_auth_bearer_command_serialize() {
        let cmd = ClientCommand::UpdateProxyAuth {
            config: bearer_config(),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let deserialized: ClientCommand = serde_json::from_str(&json).unwrap();
        match deserialized {
            ClientCommand::UpdateProxyAuth { config } => {
                assert_eq!(config.method, AuthMethod::Bearer);
                assert_eq!(config.token.unwrap(), "my-bearer-token");
            }
            _ => panic!("Expected UpdateProxyAuth"),
        }
    }

    #[test]
    fn test_update_proxy_auth_api_key_command_serialize() {
        let cmd = ClientCommand::UpdateProxyAuth {
            config: api_key_config(),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let deserialized: ClientCommand = serde_json::from_str(&json).unwrap();
        match deserialized {
            ClientCommand::UpdateProxyAuth { config } => {
                assert_eq!(config.method, AuthMethod::ApiKey);
                assert_eq!(config.token.unwrap(), "my-api-key-123");
            }
            _ => panic!("Expected UpdateProxyAuth"),
        }
    }
}

mod tests {
    use crate::protocol::*;

    #[test]
    fn test_client_cert_config_serde_roundtrip() {
        let config = ClientCertConfig {
            cert_path: "/path/to/cert.pem".to_string(),
            key_path: "/path/to/key.pem".to_string(),
            enabled: true,
            domain_certs: vec![],
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ClientCertConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.cert_path, "/path/to/cert.pem");
        assert_eq!(deserialized.key_path, "/path/to/key.pem");
        assert!(deserialized.enabled);
    }

    #[test]
    fn test_update_client_certificate_command_serialize() {
        let cmd = ClientCommand::UpdateClientCertificate {
            config: Some(ClientCertConfig {
                cert_path: "/tmp/client.crt".to_string(),
                key_path: "/tmp/client.key".to_string(),
                enabled: true,
                domain_certs: vec![],
            }),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("update_client_certificate"));
        assert!(json.contains("/tmp/client.crt"));

        let deserialized: ClientCommand = serde_json::from_str(&json).unwrap();
        match deserialized {
            ClientCommand::UpdateClientCertificate { config } => {
                let config = config.unwrap();
                assert_eq!(config.cert_path, "/tmp/client.crt");
                assert_eq!(config.key_path, "/tmp/client.key");
                assert!(config.enabled);
            }
            _ => panic!("Expected UpdateClientCertificate"),
        }
    }

    #[test]
    fn test_update_client_certificate_command_none() {
        let cmd = ClientCommand::UpdateClientCertificate { config: None };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("update_client_certificate"));

        let deserialized: ClientCommand = serde_json::from_str(&json).unwrap();
        match deserialized {
            ClientCommand::UpdateClientCertificate { config } => {
                assert!(config.is_none());
            }
            _ => panic!("Expected UpdateClientCertificate"),
        }
    }

    #[test]
    fn test_update_request_client_cert_command_serialize() {
        let cmd = ClientCommand::UpdateRequestClientCert {
            config: Some(RequestClientCertConfig {
                enabled: true,
                ca_cert_path: None,
                required: false,
            }),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("update_request_client_cert"));

        let deserialized: ClientCommand = serde_json::from_str(&json).unwrap();
        match deserialized {
            ClientCommand::UpdateRequestClientCert { config } => {
                let config = config.unwrap();
                assert!(config.enabled);
                assert!(!config.required);
            }
            _ => panic!("Expected UpdateRequestClientCert"),
        }
    }

    #[test]
    fn test_update_request_client_cert_command_none() {
        let cmd = ClientCommand::UpdateRequestClientCert { config: None };
        let json = serde_json::to_string(&cmd).unwrap();
        let deserialized: ClientCommand = serde_json::from_str(&json).unwrap();
        match deserialized {
            ClientCommand::UpdateRequestClientCert { config } => {
                assert!(config.is_none());
            }
            _ => panic!("Expected UpdateRequestClientCert"),
        }
    }

    #[test]
    fn test_health_check_command_serialize() {
        let cmd = ClientCommand::HealthCheck;
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("health_check"));
        let deserialized: ClientCommand = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, ClientCommand::HealthCheck));
    }
}

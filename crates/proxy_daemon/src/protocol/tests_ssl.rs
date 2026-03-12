#[cfg(test)]
mod tests {
    use crate::protocol::*;

    #[test]
    fn test_ssl_proxying_entry_serde_roundtrip() {
        let entry = SslProxyingEntry {
            pattern: "*.example.com".to_string(),
            enabled: true,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: SslProxyingEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.pattern, "*.example.com");
        assert!(deserialized.enabled);
    }

    #[test]
    fn test_update_ssl_proxying_list_command_serialize() {
        let cmd = ClientCommand::UpdateSslProxyingList {
            mode: SslProxyingMode::Blacklist,
            entries: vec![
                SslProxyingEntry {
                    pattern: "example.com".to_string(),
                    enabled: true,
                },
                SslProxyingEntry {
                    pattern: "*.api.io:8443".to_string(),
                    enabled: false,
                },
            ],
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("update_ssl_proxying_list"));
        assert!(json.contains("example.com"));
        assert!(json.contains("blacklist"));

        let deserialized: ClientCommand = serde_json::from_str(&json).unwrap();
        match deserialized {
            ClientCommand::UpdateSslProxyingList { mode, entries } => {
                assert_eq!(mode, SslProxyingMode::Blacklist);
                assert_eq!(entries.len(), 2);
                assert_eq!(entries[0].pattern, "example.com");
                assert!(entries[0].enabled);
                assert_eq!(entries[1].pattern, "*.api.io:8443");
                assert!(!entries[1].enabled);
            }
            _ => panic!("Expected UpdateSslProxyingList"),
        }
    }

    #[test]
    fn test_update_ssl_proxying_list_backward_compat() {
        let json = r#"{"cmd":"update_ssl_proxying_list","entries":[]}"#;
        let cmd: ClientCommand = serde_json::from_str(json).unwrap();
        match cmd {
            ClientCommand::UpdateSslProxyingList { mode, entries } => {
                assert_eq!(mode, SslProxyingMode::Blacklist);
                assert!(entries.is_empty());
            }
            _ => panic!("Expected UpdateSslProxyingList"),
        }
    }

    #[test]
    fn test_ssl_proxying_list_updated_message_serialize() {
        let msg = DaemonMessage::SslProxyingListUpdated {
            mode: SslProxyingMode::Whitelist,
            entries: vec![SslProxyingEntry {
                pattern: "*.example.com".to_string(),
                enabled: true,
            }],
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("ssl_proxying_list_updated"));
        assert!(json.contains("whitelist"));

        let deserialized: DaemonMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            DaemonMessage::SslProxyingListUpdated { mode, entries } => {
                assert_eq!(mode, SslProxyingMode::Whitelist);
                assert_eq!(entries.len(), 1);
                assert_eq!(entries[0].pattern, "*.example.com");
            }
            _ => panic!("Expected SslProxyingListUpdated"),
        }
    }

    #[test]
    fn test_ssl_proxying_mode_serde() {
        assert_eq!(
            serde_json::to_string(&SslProxyingMode::Blacklist).unwrap(),
            r#""blacklist""#
        );
        assert_eq!(
            serde_json::to_string(&SslProxyingMode::Whitelist).unwrap(),
            r#""whitelist""#
        );
        let mode: SslProxyingMode = serde_json::from_str(r#""whitelist""#).unwrap();
        assert_eq!(mode, SslProxyingMode::Whitelist);
    }

    #[test]
    fn test_certificate_info_serde_roundtrip() {
        let info = CertificateInfo {
            subject_cn: Some("example.com".to_string()),
            issuer_cn: Some("My CA".to_string()),
            organization: Some("Example Inc".to_string()),
            sans_dns: vec!["example.com".to_string(), "*.example.com".to_string()],
            sans_ip: vec!["127.0.0.1".to_string()],
            not_before: "2024-01-01T00:00:00Z".to_string(),
            not_after: "2025-01-01T00:00:00Z".to_string(),
            serial_number: "01:02:03".to_string(),
            fingerprint_sha256: "AA:BB:CC".to_string(),
            is_ca: false,
            chain_length: 1,
        };
        let json = serde_json::to_string(&info).unwrap();
        let deserialized: CertificateInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.subject_cn, Some("example.com".to_string()));
        assert_eq!(deserialized.sans_dns.len(), 2);
        assert_eq!(deserialized.chain_length, 1);
        assert!(!deserialized.is_ca);
    }

    #[test]
    fn test_domain_client_cert_config_serde_roundtrip() {
        let config = DomainClientCertConfig {
            domain_pattern: "*.example.com".to_string(),
            cert_path: "/path/to/cert.pem".to_string(),
            key_path: "/path/to/key.pem".to_string(),
            enabled: true,
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: DomainClientCertConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.domain_pattern, "*.example.com");
        assert!(deserialized.enabled);
    }

    #[test]
    fn test_client_cert_config_with_domain_certs_serde() {
        let config = ClientCertConfig {
            cert_path: "/default/cert.pem".to_string(),
            key_path: "/default/key.pem".to_string(),
            enabled: true,
            domain_certs: vec![
                DomainClientCertConfig {
                    domain_pattern: "*.api.com".to_string(),
                    cert_path: "/api/cert.pem".to_string(),
                    key_path: "/api/key.pem".to_string(),
                    enabled: true,
                },
                DomainClientCertConfig {
                    domain_pattern: "internal.corp".to_string(),
                    cert_path: "/corp/cert.pem".to_string(),
                    key_path: "/corp/key.pem".to_string(),
                    enabled: false,
                },
            ],
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ClientCertConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.domain_certs.len(), 2);
        assert_eq!(deserialized.domain_certs[0].domain_pattern, "*.api.com");
        assert!(!deserialized.domain_certs[1].enabled);
    }

    #[test]
    fn test_client_cert_config_backward_compat_no_domain_certs() {
        let json = r#"{"cert_path":"/cert.pem","key_path":"/key.pem","enabled":true}"#;
        let config: ClientCertConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.cert_path, "/cert.pem");
        assert!(config.domain_certs.is_empty());
    }

    #[test]
    fn test_request_client_cert_config_serde_roundtrip() {
        let config = RequestClientCertConfig {
            enabled: true,
            ca_cert_path: Some("/path/to/ca.pem".to_string()),
            required: true,
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: RequestClientCertConfig = serde_json::from_str(&json).unwrap();
        assert!(deserialized.enabled);
        assert_eq!(
            deserialized.ca_cert_path,
            Some("/path/to/ca.pem".to_string())
        );
        assert!(deserialized.required);
    }

    #[test]
    fn test_request_client_cert_config_defaults() {
        let json = r#"{"enabled":true}"#;
        let config: RequestClientCertConfig = serde_json::from_str(json).unwrap();
        assert!(config.enabled);
        assert!(config.ca_cert_path.is_none());
        assert!(!config.required);
    }
}

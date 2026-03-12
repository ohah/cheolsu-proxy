mod auth;
mod breakpoint_types;
mod command;
mod host_mapping;
mod intercept;
mod ssl;

pub use auth::ProxyAuthConfig;
pub use breakpoint_types::{BreakpointAction, BreakpointData, BreakpointPhase, BreakpointRule};
pub use command::{ClientCommand, DaemonMessage, ProxyLockInfo};
pub use host_mapping::HostMapping;
pub use intercept::{InterceptAction, InterceptRule, RewriteTarget, ServerReplayEntry};
pub use ssl::{
    CertificateInfo, ClientCertConfig, DomainClientCertConfig, RequestClientCertConfig,
    SslProxyingEntry, SslProxyingMode, TlsPassthroughEntry,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_breakpoint_rule_serde_roundtrip() {
        let rule = BreakpointRule {
            id: "rule_1".to_string(),
            pattern: "*api.example.com*".to_string(),
            break_on_request: true,
            break_on_response: false,
            enabled: true,
        };
        let json = serde_json::to_string(&rule).unwrap();
        let deserialized: BreakpointRule = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, rule.id);
        assert_eq!(deserialized.pattern, rule.pattern);
        assert_eq!(deserialized.break_on_request, rule.break_on_request);
        assert_eq!(deserialized.break_on_response, rule.break_on_response);
        assert_eq!(deserialized.enabled, rule.enabled);
    }

    #[test]
    fn test_breakpoint_data_serde_roundtrip() {
        let data = BreakpointData {
            method: "POST".to_string(),
            url: "https://api.example.com/users".to_string(),
            headers: HashMap::from([
                ("Content-Type".to_string(), "application/json".to_string()),
                ("Authorization".to_string(), "Bearer token".to_string()),
            ]),
            body: Some("{\"name\": \"test\"}".to_string()),
            status: Some(200),
        };
        let json = serde_json::to_string(&data).unwrap();
        let deserialized: BreakpointData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.method, data.method);
        assert_eq!(deserialized.url, data.url);
        assert_eq!(deserialized.headers.len(), 2);
        assert_eq!(deserialized.body, data.body);
        assert_eq!(deserialized.status, data.status);
    }

    #[test]
    fn test_breakpoint_action_forward_serde() {
        let action = BreakpointAction::Forward;
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"forward\""));
        let deserialized: BreakpointAction = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, BreakpointAction::Forward));
    }

    #[test]
    fn test_breakpoint_action_modify_and_forward_serde() {
        let action = BreakpointAction::ModifyAndForward {
            headers: Some(HashMap::from([(
                "X-Custom".to_string(),
                "value".to_string(),
            )])),
            body: Some("modified body".to_string()),
            status: Some(201),
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"modify_and_forward\""));
        let deserialized: BreakpointAction = serde_json::from_str(&json).unwrap();
        match deserialized {
            BreakpointAction::ModifyAndForward {
                headers,
                body,
                status,
            } => {
                assert_eq!(headers.unwrap().get("X-Custom").unwrap(), "value");
                assert_eq!(body.unwrap(), "modified body");
                assert_eq!(status.unwrap(), 201);
            }
            _ => panic!("Expected ModifyAndForward variant"),
        }
    }

    #[test]
    fn test_breakpoint_action_drop_serde() {
        let action = BreakpointAction::Drop;
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"drop\""));
        let deserialized: BreakpointAction = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, BreakpointAction::Drop));
    }

    #[test]
    fn test_breakpoint_action_abort_serde() {
        let action = BreakpointAction::Abort;
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"abort\""));
        let deserialized: BreakpointAction = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, BreakpointAction::Abort));
    }

    #[test]
    fn test_host_mapping_serialize_deserialize_roundtrip() {
        let mapping = HostMapping {
            id: "hm_1".to_string(),
            source_host: "*.api.example.com".to_string(),
            source_port: Some(443),
            target_host: "192.168.1.100".to_string(),
            target_port: Some(8443),
            enabled: true,
        };
        let json = serde_json::to_string(&mapping).unwrap();
        let deserialized: HostMapping = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "hm_1");
        assert_eq!(deserialized.source_host, "*.api.example.com");
        assert_eq!(deserialized.source_port, Some(443));
        assert_eq!(deserialized.target_host, "192.168.1.100");
        assert_eq!(deserialized.target_port, Some(8443));
        assert!(deserialized.enabled);
    }

    #[test]
    fn test_host_mapping_roundtrip_no_ports() {
        let mapping = HostMapping {
            id: "hm_2".to_string(),
            source_host: "example.com".to_string(),
            source_port: None,
            target_host: "10.0.0.1".to_string(),
            target_port: None,
            enabled: false,
        };
        let json = serde_json::to_string(&mapping).unwrap();
        let deserialized: HostMapping = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "hm_2");
        assert!(deserialized.source_port.is_none());
        assert!(deserialized.target_port.is_none());
        assert!(!deserialized.enabled);
    }

    #[test]
    fn test_update_host_mappings_command_serialize() {
        let cmd = ClientCommand::UpdateHostMappings {
            mappings: vec![
                HostMapping {
                    id: "hm_1".to_string(),
                    source_host: "api.example.com".to_string(),
                    source_port: None,
                    target_host: "10.0.0.1".to_string(),
                    target_port: Some(8080),
                    enabled: true,
                },
                HostMapping {
                    id: "hm_2".to_string(),
                    source_host: "*.staging.com".to_string(),
                    source_port: Some(443),
                    target_host: "192.168.1.50".to_string(),
                    target_port: None,
                    enabled: false,
                },
            ],
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("update_host_mappings"));
        assert!(json.contains("api.example.com"));
        assert!(json.contains("*.staging.com"));

        let deserialized: ClientCommand = serde_json::from_str(&json).unwrap();
        match deserialized {
            ClientCommand::UpdateHostMappings { mappings } => {
                assert_eq!(mappings.len(), 2);
                assert_eq!(mappings[0].id, "hm_1");
                assert_eq!(mappings[1].source_host, "*.staging.com");
            }
            _ => panic!("Expected UpdateHostMappings"),
        }
    }

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
    fn test_client_certificate_updated_message_serialize() {
        let msg = DaemonMessage::ClientCertificateUpdated {
            config: Some(ClientCertConfig {
                cert_path: "/path/cert.pem".to_string(),
                key_path: "/path/key.pem".to_string(),
                enabled: true,
                domain_certs: vec![],
            }),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("client_certificate_updated"));

        let deserialized: DaemonMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            DaemonMessage::ClientCertificateUpdated { config } => {
                let config = config.unwrap();
                assert_eq!(config.cert_path, "/path/cert.pem");
            }
            _ => panic!("Expected ClientCertificateUpdated"),
        }
    }

    #[test]
    fn test_daemon_message_disconnected_serialization() {
        let msg = DaemonMessage::Disconnected {
            reason: "daemon process killed".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"], "disconnected");
        assert_eq!(parsed["reason"], "daemon process killed");
    }

    #[test]
    fn test_daemon_message_disconnected_deserialization() {
        let json = r#"{"type":"disconnected","reason":"connection lost"}"#;
        let msg: DaemonMessage = serde_json::from_str(json).unwrap();
        match msg {
            DaemonMessage::Disconnected { reason } => {
                assert_eq!(reason, "connection lost");
            }
            _ => panic!("Expected Disconnected"),
        }
    }

    #[test]
    fn test_daemon_message_disconnected_roundtrip() {
        let msg = DaemonMessage::Disconnected {
            reason: "프로세스 강제 종료".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: DaemonMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            DaemonMessage::Disconnected { reason } => {
                assert_eq!(reason, "프로세스 강제 종료");
            }
            _ => panic!("Expected Disconnected"),
        }
    }

    #[test]
    fn test_daemon_message_reconnected_serialization() {
        let msg = DaemonMessage::Reconnected;
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"], "reconnected");
    }

    #[test]
    fn test_daemon_message_reconnected_deserialization() {
        let json = r#"{"type":"reconnected"}"#;
        let msg: DaemonMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, DaemonMessage::Reconnected));
    }

    #[test]
    fn test_disconnected_reconnected_sequence_protocol() {
        let messages = vec![
            DaemonMessage::Disconnected {
                reason: "daemon killed".to_string(),
            },
            DaemonMessage::Reconnected,
        ];

        let mut wire = String::new();
        for msg in &messages {
            wire.push_str(&serde_json::to_string(msg).unwrap());
            wire.push('\n');
        }

        let parsed: Vec<DaemonMessage> = wire
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();

        assert_eq!(parsed.len(), 2);
        match &parsed[0] {
            DaemonMessage::Disconnected { reason } => {
                assert_eq!(reason, "daemon killed");
            }
            _ => panic!("Expected Disconnected"),
        }
        assert!(matches!(parsed[1], DaemonMessage::Reconnected));
    }

    #[test]
    fn test_health_check_command_serialize() {
        let cmd = ClientCommand::HealthCheck;
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("health_check"));
        let deserialized: ClientCommand = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, ClientCommand::HealthCheck));
    }

    #[test]
    fn test_health_check_result_message_serialize() {
        let msg = DaemonMessage::HealthCheckResult {
            uptime_secs: 3600,
            active_connections: 5,
            total_transactions: 1234,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("health_check_result"));

        let deserialized: DaemonMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            DaemonMessage::HealthCheckResult {
                uptime_secs,
                active_connections,
                total_transactions,
            } => {
                assert_eq!(uptime_secs, 3600);
                assert_eq!(active_connections, 5);
                assert_eq!(total_transactions, 1234);
            }
            _ => panic!("Expected HealthCheckResult"),
        }
    }

    #[test]
    fn test_health_check_result_roundtrip() {
        let json = r#"{"type":"health_check_result","uptime_secs":120,"active_connections":0,"total_transactions":42}"#;
        let msg: DaemonMessage = serde_json::from_str(json).unwrap();
        match msg {
            DaemonMessage::HealthCheckResult {
                uptime_secs,
                active_connections,
                total_transactions,
            } => {
                assert_eq!(uptime_secs, 120);
                assert_eq!(active_connections, 0);
                assert_eq!(total_transactions, 42);
            }
            _ => panic!("Expected HealthCheckResult"),
        }
    }

    #[test]
    fn test_host_mappings_updated_message_serialize() {
        let msg = DaemonMessage::HostMappingsUpdated {
            mappings: vec![HostMapping {
                id: "hm_1".to_string(),
                source_host: "example.com".to_string(),
                source_port: None,
                target_host: "127.0.0.1".to_string(),
                target_port: Some(3000),
                enabled: true,
            }],
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("host_mappings_updated"));
        assert!(json.contains("example.com"));

        let deserialized: DaemonMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            DaemonMessage::HostMappingsUpdated { mappings } => {
                assert_eq!(mappings.len(), 1);
                assert_eq!(mappings[0].target_host, "127.0.0.1");
                assert_eq!(mappings[0].target_port, Some(3000));
            }
            _ => panic!("Expected HostMappingsUpdated"),
        }
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
    fn test_request_client_cert_updated_message_serialize() {
        let msg = DaemonMessage::RequestClientCertUpdated {
            config: Some(RequestClientCertConfig {
                enabled: true,
                ca_cert_path: Some("/ca.pem".to_string()),
                required: true,
            }),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("request_client_cert_updated"));

        let deserialized: DaemonMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            DaemonMessage::RequestClientCertUpdated { config } => {
                let config = config.unwrap();
                assert!(config.required);
            }
            _ => panic!("Expected RequestClientCertUpdated"),
        }
    }
}

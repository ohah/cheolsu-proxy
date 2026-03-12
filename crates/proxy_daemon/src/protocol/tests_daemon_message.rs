#[cfg(test)]
mod tests {
    use crate::protocol::*;

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

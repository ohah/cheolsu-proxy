#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use crate::daemon::accept_loop::ClientCountGuard;
    use crate::daemon::lifecycle::{app_support_dir, lock_file_path, uds_socket_path};
    use crate::protocol::{
        ClientCommand, DaemonMessage, InterceptAction, InterceptRule, ProxyLockInfo,
    };

    #[test]
    fn test_client_command_subscribe_serialization() {
        let cmd = ClientCommand::Subscribe;
        let json = serde_json::to_string(&cmd).unwrap();
        assert_eq!(json, r#"{"cmd":"subscribe"}"#);
    }

    #[test]
    fn test_client_command_subscribe_deserialization() {
        let json = r#"{"cmd":"subscribe"}"#;
        let cmd: ClientCommand = serde_json::from_str(json).unwrap();
        assert!(matches!(cmd, ClientCommand::Subscribe));
    }

    #[test]
    fn test_client_command_stop_serialization() {
        let cmd = ClientCommand::Stop;
        let json = serde_json::to_string(&cmd).unwrap();
        assert_eq!(json, r#"{"cmd":"stop"}"#);
    }

    #[test]
    fn test_client_command_stop_deserialization() {
        let json = r#"{"cmd":"stop"}"#;
        let cmd: ClientCommand = serde_json::from_str(json).unwrap();
        assert!(matches!(cmd, ClientCommand::Stop));
    }

    #[test]
    fn test_daemon_message_status_serialization() {
        let msg = DaemonMessage::Status {
            running: true,
            port: 8100,
            protocol_version: crate::protocol::PROTOCOL_VERSION,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"], "status");
        assert_eq!(parsed["running"], true);
        assert_eq!(parsed["port"], 8100);
        assert_eq!(
            parsed["protocol_version"],
            crate::protocol::PROTOCOL_VERSION
        );
    }

    #[test]
    fn test_daemon_message_status_deserialization() {
        let json = r#"{"type":"status","running":true,"port":8100}"#;
        let msg: DaemonMessage = serde_json::from_str(json).unwrap();
        match msg {
            DaemonMessage::Status {
                running,
                port,
                protocol_version,
            } => {
                assert!(running);
                assert_eq!(port, 8100);
                assert_eq!(protocol_version, 0); // default when not present
            }
            _ => panic!("Expected Status"),
        }
    }

    #[test]
    fn test_invalid_command_deserialization_fails() {
        let json = r#"{"cmd":"unknown_command"}"#;
        let result = serde_json::from_str::<ClientCommand>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_message_deserialization_fails() {
        let json = r#"{"type":"unknown_type"}"#;
        let result = serde_json::from_str::<DaemonMessage>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_proxy_lock_info_serialization_roundtrip() {
        let info = ProxyLockInfo {
            pid: 12345,
            port: 8100,
            uds_path: "/tmp/proxy.sock".to_string(),
        };
        let json = serde_json::to_string_pretty(&info).unwrap();
        let parsed: ProxyLockInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.pid, 12345);
        assert_eq!(parsed.port, 8100);
        assert_eq!(parsed.uds_path, "/tmp/proxy.sock");
    }

    #[test]
    fn test_proxy_lock_info_fields() {
        let json = r#"{"pid":99999,"port":9090,"uds_path":"/var/run/test.sock"}"#;
        let info: ProxyLockInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.pid, 99999);
        assert_eq!(info.port, 9090);
        assert_eq!(info.uds_path, "/var/run/test.sock");
    }

    #[test]
    fn test_stale_lock_with_dead_pid() {
        let tmp = tempfile::TempDir::new().unwrap();
        let lock_path = tmp.path().join("proxy.lock");
        let sock_path = tmp.path().join("proxy.sock");

        let info = ProxyLockInfo {
            pid: 4_000_000,
            port: 8100,
            uds_path: sock_path.to_string_lossy().to_string(),
        };
        let json = serde_json::to_string_pretty(&info).unwrap();
        std::fs::write(&lock_path, &json).unwrap();

        std::fs::write(&sock_path, "fake").unwrap();

        use nix::sys::signal::kill;
        use nix::unistd::Pid;
        let pid = Pid::from_raw(4_000_000);
        assert!(kill(pid, None).is_err(), "PID 4000000 should not exist");

        let contents = std::fs::read_to_string(&lock_path).unwrap();
        let parsed: ProxyLockInfo = serde_json::from_str(&contents).unwrap();
        assert_eq!(parsed.pid, 4_000_000);
    }

    #[test]
    fn test_lock_info_with_current_pid_is_alive() {
        use nix::sys::signal::kill;
        use nix::unistd::Pid;

        let pid = Pid::from_raw(std::process::id() as i32);
        assert!(kill(pid, None).is_ok(), "Current process should be alive");
    }

    #[test]
    fn test_app_support_dir_is_under_data_dir() {
        let dir = app_support_dir().unwrap();
        let data = dirs::data_dir().unwrap();
        assert!(dir.starts_with(&data));
        assert!(dir.ends_with("com.cheolsu-proxy"));
    }

    #[test]
    fn test_lock_file_path_ends_with_proxy_lock() {
        let path = lock_file_path().unwrap();
        assert!(path.ends_with("proxy.lock"));
    }

    #[test]
    fn test_uds_socket_path_ends_with_proxy_sock() {
        let path = uds_socket_path().unwrap();
        assert!(path.ends_with("proxy.sock"));
    }

    #[test]
    fn test_newline_delimited_protocol_multiple_messages() {
        let messages = vec![
            DaemonMessage::Status {
                running: true,
                port: 8100,
                protocol_version: crate::protocol::PROTOCOL_VERSION,
            },
            DaemonMessage::Status {
                running: false,
                port: 8100,
                protocol_version: crate::protocol::PROTOCOL_VERSION,
            },
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
            DaemonMessage::Status { running, port, .. } => {
                assert!(*running);
                assert_eq!(*port, 8100);
            }
            _ => panic!("Expected Status"),
        }
        match &parsed[1] {
            DaemonMessage::Status { running, port, .. } => {
                assert!(!*running);
                assert_eq!(*port, 8100);
            }
            _ => panic!("Expected Status"),
        }
    }

    #[test]
    fn test_mixed_commands_newline_protocol() {
        let commands = vec![ClientCommand::Subscribe, ClientCommand::Stop];

        let mut wire = String::new();
        for cmd in &commands {
            wire.push_str(&serde_json::to_string(cmd).unwrap());
            wire.push('\n');
        }

        let parsed: Vec<ClientCommand> = wire
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();

        assert_eq!(parsed.len(), 2);
        assert!(matches!(parsed[0], ClientCommand::Subscribe));
        assert!(matches!(parsed[1], ClientCommand::Stop));
    }

    #[test]
    fn test_intercept_rule_block_serialization() {
        let rule = InterceptRule {
            id: "r1".to_string(),
            name: "Block ads".to_string(),
            enabled: true,
            pattern: "*ads.example.com*".to_string(),
            method: None,
            action: InterceptAction::Block {
                status_code: 403,
                body: "Blocked".to_string(),
            },
        };
        let json = serde_json::to_string(&rule).unwrap();
        let parsed: InterceptRule = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "r1");
        assert_eq!(parsed.pattern, "*ads.example.com*");
        assert!(parsed.enabled);
        match parsed.action {
            InterceptAction::Block { status_code, body } => {
                assert_eq!(status_code, 403);
                assert_eq!(body, "Blocked");
            }
            _ => panic!("Expected Block action"),
        }
    }

    #[test]
    fn test_intercept_rule_modify_response_serialization() {
        let mut headers = std::collections::HashMap::new();
        headers.insert("X-Test".to_string(), "value".to_string());
        let rule = InterceptRule {
            id: "r2".to_string(),
            name: "Modify API".to_string(),
            enabled: true,
            pattern: "*api.example.com*".to_string(),
            method: Some("GET".to_string()),
            action: InterceptAction::ModifyResponse {
                set_status: Some(200),
                add_headers: headers,
                remove_headers: vec!["X-Remove".to_string()],
                set_body: Some(r#"{"mocked":true}"#.to_string()),
            },
        };
        let json = serde_json::to_string(&rule).unwrap();
        let parsed: InterceptRule = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.pattern, "*api.example.com*");
        assert_eq!(parsed.method, Some("GET".to_string()));
        match parsed.action {
            InterceptAction::ModifyResponse {
                set_status,
                set_body,
                ..
            } => {
                assert_eq!(set_status, Some(200));
                assert_eq!(set_body, Some(r#"{"mocked":true}"#.to_string()));
            }
            _ => panic!("Expected ModifyResponse action"),
        }
    }

    #[test]
    fn test_update_intercept_rules_command_serialization() {
        let rules = vec![InterceptRule {
            id: "r1".to_string(),
            name: "Test".to_string(),
            enabled: true,
            pattern: "*test.com*".to_string(),
            method: None,
            action: InterceptAction::Block {
                status_code: 403,
                body: String::new(),
            },
        }];
        let cmd = ClientCommand::UpdateInterceptRules {
            rules: rules.clone(),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["cmd"], "update_intercept_rules");
        assert_eq!(parsed["rules"].as_array().unwrap().len(), 1);

        let parsed_cmd: ClientCommand = serde_json::from_str(&json).unwrap();
        match parsed_cmd {
            ClientCommand::UpdateInterceptRules { rules } => {
                assert_eq!(rules.len(), 1);
                assert_eq!(rules[0].id, "r1");
            }
            _ => panic!("Expected UpdateInterceptRules"),
        }
    }

    #[test]
    fn test_intercept_rule_json_deserialization() {
        let json = r#"{
            "id": "r1",
            "name": "Block",
            "enabled": true,
            "pattern": "*ads*",
            "action": {
                "type": "block",
                "status_code": 403,
                "body": "No"
            }
        }"#;
        let rule: InterceptRule = serde_json::from_str(json).unwrap();
        assert_eq!(rule.id, "r1");
        assert_eq!(rule.pattern, "*ads*");
        assert!(rule.method.is_none());
    }

    // --- ClientCountGuard 테스트 ---

    /// ClientCountGuard Drop 시 카운트가 1 감소하는지 검증
    #[tokio::test]
    async fn test_client_count_guard_drop_decrements_count() {
        let count = Arc::new(AtomicUsize::new(3));
        let (shutdown_tx, _shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);

        {
            let _guard = ClientCountGuard {
                client_count: count.clone(),
                shutdown_tx: shutdown_tx.clone(),
            };
            assert_eq!(count.load(Ordering::SeqCst), 3);
        }
        // Drop 후 카운트가 2로 감소해야 함
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    /// 여러 Guard가 순차적으로 Drop될 때 카운트가 정확히 감소하는지 검증
    #[tokio::test]
    async fn test_client_count_guard_multiple_drops() {
        let count = Arc::new(AtomicUsize::new(3));
        let (shutdown_tx, _shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);

        let guard1 = ClientCountGuard {
            client_count: count.clone(),
            shutdown_tx: shutdown_tx.clone(),
        };
        let guard2 = ClientCountGuard {
            client_count: count.clone(),
            shutdown_tx: shutdown_tx.clone(),
        };
        let guard3 = ClientCountGuard {
            client_count: count.clone(),
            shutdown_tx: shutdown_tx.clone(),
        };

        drop(guard1);
        assert_eq!(count.load(Ordering::SeqCst), 2);

        drop(guard2);
        assert_eq!(count.load(Ordering::SeqCst), 1);

        drop(guard3);
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }

    /// 카운트가 0이 될 때 shutdown 시그널이 전송되는지 검증
    #[tokio::test]
    async fn test_client_count_guard_sends_shutdown_when_zero() {
        let count = Arc::new(AtomicUsize::new(1));
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);

        {
            let _guard = ClientCountGuard {
                client_count: count.clone(),
                shutdown_tx: shutdown_tx.clone(),
            };
        }
        // 카운트가 1→0이 되었으므로 shutdown 시그널이 전송되어야 함
        let result = shutdown_rx.try_recv();
        assert!(
            result.is_ok(),
            "카운트가 0이 되면 shutdown 시그널이 전송되어야 함"
        );
    }

    /// 카운트가 0보다 클 때는 shutdown 시그널이 전송되지 않는지 검증
    #[tokio::test]
    async fn test_client_count_guard_no_shutdown_when_remaining() {
        let count = Arc::new(AtomicUsize::new(2));
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);

        {
            let _guard = ClientCountGuard {
                client_count: count.clone(),
                shutdown_tx: shutdown_tx.clone(),
            };
        }
        // 카운트가 2→1이므로 shutdown 시그널이 전송되면 안 됨
        let result = shutdown_rx.try_recv();
        assert!(
            result.is_err(),
            "남은 클라이언트가 있으면 shutdown 시그널이 전송되면 안 됨"
        );
    }

    /// 카운트가 이미 0인 상태에서 Drop되면 언더플로우 방지 후 0으로 복구되는지 검증
    #[tokio::test]
    async fn test_client_count_guard_underflow_protection() {
        let count = Arc::new(AtomicUsize::new(0));
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);

        {
            let _guard = ClientCountGuard {
                client_count: count.clone(),
                shutdown_tx: shutdown_tx.clone(),
            };
        }
        // 언더플로우 방지: 0으로 복구되어야 함
        assert_eq!(
            count.load(Ordering::SeqCst),
            0,
            "언더플로우 시 카운트가 0으로 복구되어야 함"
        );
        // 언더플로우 상황에서는 shutdown 시그널이 전송되면 안 됨
        let result = shutdown_rx.try_recv();
        assert!(
            result.is_err(),
            "언더플로우 시에는 shutdown 시그널이 전송되면 안 됨"
        );
    }

    /// bounded channel backpressure: shutdown 채널이 가득 찬 상태에서 try_send 동작 검증
    #[tokio::test]
    async fn test_client_count_guard_shutdown_channel_full() {
        let count = Arc::new(AtomicUsize::new(1));
        // 용량 1인 채널을 미리 채워놓기
        let (shutdown_tx, _shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);
        shutdown_tx.try_send(()).unwrap(); // 채널을 가득 채움

        {
            let _guard = ClientCountGuard {
                client_count: count.clone(),
                shutdown_tx: shutdown_tx.clone(),
            };
        }
        // 채널이 가득 차도 패닉 없이 정상적으로 Drop이 완료되어야 함
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }

    /// 다중 클라이언트 시나리오: 하나씩 종료 시 마지막에만 shutdown 전송
    #[tokio::test]
    async fn test_client_count_guard_multi_client_last_triggers_shutdown() {
        let count = Arc::new(AtomicUsize::new(3));
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);

        let guard1 = ClientCountGuard {
            client_count: count.clone(),
            shutdown_tx: shutdown_tx.clone(),
        };
        let guard2 = ClientCountGuard {
            client_count: count.clone(),
            shutdown_tx: shutdown_tx.clone(),
        };
        let guard3 = ClientCountGuard {
            client_count: count.clone(),
            shutdown_tx: shutdown_tx.clone(),
        };

        // 첫 번째 클라이언트 종료 (3→2)
        drop(guard1);
        assert!(shutdown_rx.try_recv().is_err());

        // 두 번째 클라이언트 종료 (2→1)
        drop(guard2);
        assert!(shutdown_rx.try_recv().is_err());

        // 마지막 클라이언트 종료 (1→0) → shutdown 전송
        drop(guard3);
        assert!(
            shutdown_rx.try_recv().is_ok(),
            "마지막 클라이언트 종료 시 shutdown 전송"
        );
    }

    #[test]
    fn test_intercept_rule_modify_request_deserialization() {
        let json = r#"{
            "id": "r3",
            "name": "Add header",
            "enabled": true,
            "pattern": "*api.test.com*",
            "method": "POST",
            "action": {
                "type": "modify_request",
                "add_headers": {"Authorization": "Bearer token123"},
                "remove_headers": ["Cookie"]
            }
        }"#;
        let rule: InterceptRule = serde_json::from_str(json).unwrap();
        assert_eq!(rule.id, "r3");
        assert_eq!(rule.pattern, "*api.test.com*");
        assert_eq!(rule.method, Some("POST".to_string()));
        match rule.action {
            InterceptAction::ModifyRequest {
                add_headers,
                remove_headers,
                set_body,
            } => {
                assert_eq!(add_headers.get("Authorization").unwrap(), "Bearer token123");
                assert_eq!(remove_headers, vec!["Cookie"]);
                assert!(set_body.is_none());
            }
            _ => panic!("Expected ModifyRequest"),
        }
    }
}

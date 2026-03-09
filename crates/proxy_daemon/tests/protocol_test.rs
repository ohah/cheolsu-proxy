//! 프로토콜 직렬화/역직렬화 통합 테스트
//!
//! ClientCommand와 DaemonMessage의 JSON 인코딩을 검증합니다.

use proxy_daemon::{
    ClientCommand, DaemonMessage, InterceptAction, InterceptRule, ServerReplayEntry,
    UpstreamProxyAuth, UpstreamProxyConfig,
};
use std::collections::HashMap;

// ─── ClientCommand 직렬화 ───────────────────────────────────

#[test]
fn client_command_subscribe_roundtrip() {
    let cmd = ClientCommand::Subscribe;
    let json = serde_json::to_string(&cmd).unwrap();
    let parsed: ClientCommand = serde_json::from_str(&json).unwrap();
    assert!(matches!(parsed, ClientCommand::Subscribe));
}

#[test]
fn client_command_stop_roundtrip() {
    let cmd = ClientCommand::Stop;
    let json = serde_json::to_string(&cmd).unwrap();
    assert!(json.contains("stop"));
    let parsed: ClientCommand = serde_json::from_str(&json).unwrap();
    assert!(matches!(parsed, ClientCommand::Stop));
}

#[test]
fn client_command_ws_inject_roundtrip() {
    let cmd = ClientCommand::WsInject {
        connection_id: "ws://example.com/chat".to_string(),
        direction: "to_server".to_string(),
        payload: "hello".to_string(),
        is_binary: false,
    };
    let json = serde_json::to_string(&cmd).unwrap();
    let parsed: ClientCommand = serde_json::from_str(&json).unwrap();
    match parsed {
        ClientCommand::WsInject {
            connection_id,
            direction,
            payload,
            is_binary,
        } => {
            assert_eq!(connection_id, "ws://example.com/chat");
            assert_eq!(direction, "to_server");
            assert_eq!(payload, "hello");
            assert!(!is_binary);
        }
        _ => panic!("Expected WsInject"),
    }
}

#[test]
fn client_command_load_script_with_path() {
    let cmd = ClientCommand::LoadScript {
        path: Some("/home/user/script.ts".to_string()),
        code: None,
    };
    let json = serde_json::to_string(&cmd).unwrap();
    let parsed: ClientCommand = serde_json::from_str(&json).unwrap();
    match parsed {
        ClientCommand::LoadScript { path, code } => {
            assert_eq!(path.unwrap(), "/home/user/script.ts");
            assert!(code.is_none());
        }
        _ => panic!("Expected LoadScript"),
    }
}

#[test]
fn client_command_load_script_with_code() {
    let cmd = ClientCommand::LoadScript {
        path: None,
        code: Some("cheolsu.onRequest((req) => ({ action: 'forward' }))".to_string()),
    };
    let json = serde_json::to_string(&cmd).unwrap();
    let parsed: ClientCommand = serde_json::from_str(&json).unwrap();
    match parsed {
        ClientCommand::LoadScript { path, code } => {
            assert!(path.is_none());
            assert!(code.unwrap().contains("onRequest"));
        }
        _ => panic!("Expected LoadScript"),
    }
}

#[test]
fn client_command_unload_script_roundtrip() {
    let cmd = ClientCommand::UnloadScript;
    let json = serde_json::to_string(&cmd).unwrap();
    let parsed: ClientCommand = serde_json::from_str(&json).unwrap();
    assert!(matches!(parsed, ClientCommand::UnloadScript));
}

#[test]
fn client_command_update_upstream_proxy_with_auth() {
    let cmd = ClientCommand::UpdateUpstreamProxy {
        config: Some(UpstreamProxyConfig {
            host: "proxy.corp.com".to_string(),
            port: 8080,
            auth: Some(UpstreamProxyAuth {
                username: "user".to_string(),
                password: "pass".to_string(),
            }),
            bypass: vec!["localhost".to_string(), "*.internal.com".to_string()],
        }),
    };
    let json = serde_json::to_string(&cmd).unwrap();
    let parsed: ClientCommand = serde_json::from_str(&json).unwrap();
    match parsed {
        ClientCommand::UpdateUpstreamProxy { config } => {
            let config = config.unwrap();
            assert_eq!(config.host, "proxy.corp.com");
            assert_eq!(config.port, 8080);
            assert!(config.auth.is_some());
            assert_eq!(config.bypass.len(), 2);
        }
        _ => panic!("Expected UpdateUpstreamProxy"),
    }
}

#[test]
fn client_command_update_upstream_proxy_none() {
    let cmd = ClientCommand::UpdateUpstreamProxy { config: None };
    let json = serde_json::to_string(&cmd).unwrap();
    let parsed: ClientCommand = serde_json::from_str(&json).unwrap();
    match parsed {
        ClientCommand::UpdateUpstreamProxy { config } => {
            assert!(config.is_none());
        }
        _ => panic!("Expected UpdateUpstreamProxy"),
    }
}

#[test]
fn client_command_update_server_replay() {
    let entries = vec![
        ServerReplayEntry {
            id: "entry-1".to_string(),
            method: "GET".to_string(),
            url: "https://api.test.com/users".to_string(),
            status: 200,
            headers: HashMap::from([("content-type".to_string(), "application/json".to_string())]),
            body: Some(r#"{"users":[]}"#.to_string()),
        },
        ServerReplayEntry {
            id: "entry-2".to_string(),
            method: "POST".to_string(),
            url: "https://api.test.com/users".to_string(),
            status: 201,
            headers: HashMap::new(),
            body: None,
        },
    ];
    let cmd = ClientCommand::UpdateServerReplay {
        entries: entries.clone(),
    };
    let json = serde_json::to_string(&cmd).unwrap();
    let parsed: ClientCommand = serde_json::from_str(&json).unwrap();
    match parsed {
        ClientCommand::UpdateServerReplay { entries } => {
            assert_eq!(entries.len(), 2);
            assert_eq!(entries[0].id, "entry-1");
            assert_eq!(entries[1].status, 201);
        }
        _ => panic!("Expected UpdateServerReplay"),
    }
}

// ─── DaemonMessage 직렬화 ───────────────────────────────────

#[test]
fn daemon_message_status_roundtrip() {
    let msg = DaemonMessage::Status {
        running: true,
        port: 8100,
    };
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("status"));
    let parsed: DaemonMessage = serde_json::from_str(&json).unwrap();
    match parsed {
        DaemonMessage::Status { running, port } => {
            assert!(running);
            assert_eq!(port, 8100);
        }
        _ => panic!("Expected Status"),
    }
}

#[test]
fn daemon_message_ws_inject_result_success() {
    let msg = DaemonMessage::WsInjectResult {
        success: true,
        error: None,
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: DaemonMessage = serde_json::from_str(&json).unwrap();
    match parsed {
        DaemonMessage::WsInjectResult { success, error } => {
            assert!(success);
            assert!(error.is_none());
        }
        _ => panic!("Expected WsInjectResult"),
    }
}

#[test]
fn daemon_message_ws_inject_result_error() {
    let msg = DaemonMessage::WsInjectResult {
        success: false,
        error: Some("Connection not found".to_string()),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: DaemonMessage = serde_json::from_str(&json).unwrap();
    match parsed {
        DaemonMessage::WsInjectResult { success, error } => {
            assert!(!success);
            assert_eq!(error.unwrap(), "Connection not found");
        }
        _ => panic!("Expected WsInjectResult"),
    }
}

#[test]
fn daemon_message_script_result() {
    let msg = DaemonMessage::ScriptResult {
        success: true,
        error: None,
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: DaemonMessage = serde_json::from_str(&json).unwrap();
    assert!(matches!(
        parsed,
        DaemonMessage::ScriptResult {
            success: true,
            error: None
        }
    ));
}

#[test]
fn daemon_message_script_log() {
    let msg = DaemonMessage::ScriptLog {
        level: "info".to_string(),
        message: "Script loaded".to_string(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: DaemonMessage = serde_json::from_str(&json).unwrap();
    match parsed {
        DaemonMessage::ScriptLog { level, message } => {
            assert_eq!(level, "info");
            assert_eq!(message, "Script loaded");
        }
        _ => panic!("Expected ScriptLog"),
    }
}

#[test]
fn daemon_message_script_status() {
    let msg = DaemonMessage::ScriptStatus {
        active: true,
        path: Some("/tmp/script.ts".to_string()),
        message: "Script activated".to_string(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: DaemonMessage = serde_json::from_str(&json).unwrap();
    match parsed {
        DaemonMessage::ScriptStatus {
            active,
            path,
            message,
        } => {
            assert!(active);
            assert_eq!(path.unwrap(), "/tmp/script.ts");
            assert_eq!(message, "Script activated");
        }
        _ => panic!("Expected ScriptStatus"),
    }
}

// ─── 뉴라인 프로토콜 ───────────────────────────────────────

#[test]
fn newline_protocol_multiple_commands() {
    let commands = vec![
        ClientCommand::Subscribe,
        ClientCommand::Stop,
        ClientCommand::UnloadScript,
    ];

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

    assert_eq!(parsed.len(), 3);
    assert!(matches!(parsed[0], ClientCommand::Subscribe));
    assert!(matches!(parsed[1], ClientCommand::Stop));
    assert!(matches!(parsed[2], ClientCommand::UnloadScript));
}

#[test]
fn newline_protocol_mixed_messages() {
    let messages: Vec<DaemonMessage> = vec![
        DaemonMessage::Status {
            running: true,
            port: 8100,
        },
        DaemonMessage::ScriptResult {
            success: true,
            error: None,
        },
        DaemonMessage::ScriptStatus {
            active: true,
            path: None,
            message: "loaded".to_string(),
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

    assert_eq!(parsed.len(), 3);
}

// ─── 에러 케이스 ────────────────────────────────────────────

#[test]
fn invalid_command_json_returns_error() {
    let result = serde_json::from_str::<ClientCommand>(r#"{"cmd":"nonexistent"}"#);
    assert!(result.is_err());
}

#[test]
fn invalid_message_json_returns_error() {
    let result = serde_json::from_str::<DaemonMessage>(r#"{"type":"nonexistent"}"#);
    assert!(result.is_err());
}

#[test]
fn empty_json_returns_error() {
    assert!(serde_json::from_str::<ClientCommand>("{}").is_err());
    assert!(serde_json::from_str::<DaemonMessage>("{}").is_err());
}

#[test]
fn malformed_json_returns_error() {
    assert!(serde_json::from_str::<ClientCommand>("not json").is_err());
    assert!(serde_json::from_str::<DaemonMessage>("{broken").is_err());
}

// ─── 복합 직렬화 시나리오 ───────────────────────────────────

#[test]
fn intercept_rules_command_with_all_action_types() {
    let rules = vec![
        InterceptRule {
            id: "r1".to_string(),
            name: "Block".to_string(),
            enabled: true,
            pattern: "*ads*".to_string(),
            method: None,
            action: InterceptAction::Block {
                status_code: 403,
                body: "blocked".to_string(),
            },
        },
        InterceptRule {
            id: "r2".to_string(),
            name: "Modify Req".to_string(),
            enabled: true,
            pattern: "*api*".to_string(),
            method: Some("POST".to_string()),
            action: InterceptAction::ModifyRequest {
                add_headers: HashMap::from([("X-Auth".to_string(), "token".to_string())]),
                remove_headers: vec!["Cookie".to_string()],
                set_body: Some(r#"{"patched":true}"#.to_string()),
            },
        },
        InterceptRule {
            id: "r3".to_string(),
            name: "Modify Res".to_string(),
            enabled: false,
            pattern: "*data*".to_string(),
            method: None,
            action: InterceptAction::ModifyResponse {
                set_status: Some(200),
                add_headers: HashMap::new(),
                remove_headers: vec!["x-powered-by".to_string()],
                set_body: None,
            },
        },
        InterceptRule {
            id: "r4".to_string(),
            name: "Map Local".to_string(),
            enabled: true,
            pattern: "*mock*".to_string(),
            method: None,
            action: InterceptAction::MapLocal {
                file_path: "/tmp/mock.json".to_string(),
                status_code: 200,
                headers: HashMap::new(),
            },
        },
        InterceptRule {
            id: "r5".to_string(),
            name: "Map Remote".to_string(),
            enabled: true,
            pattern: "*prod*".to_string(),
            method: None,
            action: InterceptAction::MapRemote {
                target_url: "http://localhost:3000".to_string(),
                preserve_path: false,
            },
        },
    ];

    let cmd = ClientCommand::UpdateInterceptRules { rules };
    let json = serde_json::to_string(&cmd).unwrap();
    let parsed: ClientCommand = serde_json::from_str(&json).unwrap();
    match parsed {
        ClientCommand::UpdateInterceptRules { rules } => {
            assert_eq!(rules.len(), 5);
            assert!(matches!(rules[0].action, InterceptAction::Block { .. }));
            assert!(matches!(
                rules[1].action,
                InterceptAction::ModifyRequest { .. }
            ));
            assert!(matches!(
                rules[2].action,
                InterceptAction::ModifyResponse { .. }
            ));
            assert!(matches!(rules[3].action, InterceptAction::MapLocal { .. }));
            assert!(matches!(rules[4].action, InterceptAction::MapRemote { .. }));
        }
        _ => panic!("Expected UpdateInterceptRules"),
    }
}

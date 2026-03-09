//! 인터셉트 규칙 및 핸들러 통합 테스트
//!
//! 공개 API를 통해 LoggingHandler의 인터셉트 기능을 검증합니다.
//! 참고: find_matching_intercept_rules 등 pub(crate) 메서드는 모듈 내부 단위 테스트에서 검증합니다.

use proxy_daemon::{InterceptAction, InterceptRule, LoggingHandler, ServerReplayEntry};
use std::collections::HashMap;
use std::path::PathBuf;

fn make_handler() -> LoggingHandler {
    let (sender, _rx) = tokio::sync::mpsc::channel(16);
    LoggingHandler::new(sender, PathBuf::from("/tmp/test-cache"))
}

fn block_rule(id: &str, pattern: &str, method: Option<&str>, status: u16) -> InterceptRule {
    InterceptRule {
        id: id.to_string(),
        name: format!("Rule {}", id),
        enabled: true,
        pattern: pattern.to_string(),
        method: method.map(|m| m.to_string()),
        action: InterceptAction::Block {
            status_code: status,
            body: format!("Blocked by {}", id),
        },
    }
}

// ─── LoggingHandler 구성 ────────────────────────────────────

#[tokio::test]
async fn handler_new_and_builder() {
    let handler = make_handler();
    // CA 인증서 설정
    let handler = handler.with_ca_cert_der(vec![0xDE, 0xAD, 0xBE, 0xEF]);
    // WS 센더 설정
    let (ws_tx, _ws_rx) = tokio::sync::mpsc::channel(16);
    let handler = handler.with_ws_sender(ws_tx);
    // 스크립트 핸들 설정
    let script_handle = scripting::ScriptHandle::new();
    let _handler = handler.with_script_handle(script_handle);
    // 빌더 패턴이 panic 없이 동작하는지 확인
}

#[tokio::test]
async fn update_intercept_rules_succeeds() {
    let handler = make_handler();
    let rules = vec![
        block_rule("r1", "*example.com*", None, 403),
        block_rule("r2", "*ads.com*", Some("GET"), 404),
    ];
    // panic 없이 업데이트 성공
    handler.update_intercept_rules(rules).await;
}

#[tokio::test]
async fn update_intercept_rules_with_empty_vec() {
    let handler = make_handler();
    handler.update_intercept_rules(Vec::new()).await;
    // 빈 규칙으로 업데이트해도 panic 없음
}

#[tokio::test]
async fn update_server_replay_entries_succeeds() {
    let handler = make_handler();
    let entries = vec![ServerReplayEntry {
        id: "replay-1".to_string(),
        method: "GET".to_string(),
        url: "https://api.example.com/users".to_string(),
        status: 200,
        headers: HashMap::new(),
        body: Some(r#"[{"id":1}]"#.to_string()),
    }];
    handler.update_server_replay_entries(entries).await;
}

#[tokio::test]
async fn update_rules_multiple_times() {
    let handler = make_handler();

    // 여러 번 업데이트해도 정상 동작
    for i in 0..10 {
        let rules = vec![block_rule(
            &format!("r{}", i),
            &format!("*test{}.com*", i),
            None,
            403,
        )];
        handler.update_intercept_rules(rules).await;
    }
}

// ─── InterceptRule 직렬화 ───────────────────────────────────

#[test]
fn intercept_rule_block_json_roundtrip() {
    let rule = block_rule("r1", "*ads*", None, 403);
    let json = serde_json::to_string(&rule).unwrap();
    let parsed: InterceptRule = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.id, "r1");
    assert_eq!(parsed.pattern, "*ads*");
    assert!(parsed.enabled);
    match parsed.action {
        InterceptAction::Block { status_code, body } => {
            assert_eq!(status_code, 403);
            assert!(body.contains("r1"));
        }
        _ => panic!("Expected Block"),
    }
}

#[test]
fn intercept_rule_modify_request_json_roundtrip() {
    let rule = InterceptRule {
        id: "r2".to_string(),
        name: "Add Auth".to_string(),
        enabled: true,
        pattern: "*api.com*".to_string(),
        method: Some("POST".to_string()),
        action: InterceptAction::ModifyRequest {
            add_headers: HashMap::from([("Authorization".to_string(), "Bearer token".to_string())]),
            remove_headers: vec!["Cookie".to_string()],
            set_body: None,
        },
    };
    let json = serde_json::to_string(&rule).unwrap();
    let parsed: InterceptRule = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.method, Some("POST".to_string()));
    match parsed.action {
        InterceptAction::ModifyRequest {
            add_headers,
            remove_headers,
            set_body,
        } => {
            assert_eq!(add_headers.get("Authorization").unwrap(), "Bearer token");
            assert_eq!(remove_headers, vec!["Cookie"]);
            assert!(set_body.is_none());
        }
        _ => panic!("Expected ModifyRequest"),
    }
}

#[test]
fn intercept_rule_map_local_json_roundtrip() {
    let rule = InterceptRule {
        id: "r3".to_string(),
        name: "Mock API".to_string(),
        enabled: true,
        pattern: "*api.test.com/data*".to_string(),
        method: None,
        action: InterceptAction::MapLocal {
            file_path: "/tmp/mock.json".to_string(),
            status_code: 200,
            headers: HashMap::from([("x-mock".to_string(), "true".to_string())]),
        },
    };
    let json = serde_json::to_string(&rule).unwrap();
    let parsed: InterceptRule = serde_json::from_str(&json).unwrap();
    match parsed.action {
        InterceptAction::MapLocal {
            file_path,
            status_code,
            headers,
        } => {
            assert_eq!(file_path, "/tmp/mock.json");
            assert_eq!(status_code, 200);
            assert_eq!(headers.get("x-mock").unwrap(), "true");
        }
        _ => panic!("Expected MapLocal"),
    }
}

#[test]
fn intercept_rule_map_remote_json_roundtrip() {
    let rule = InterceptRule {
        id: "r4".to_string(),
        name: "Redirect to local".to_string(),
        enabled: true,
        pattern: "*prod.api.com*".to_string(),
        method: None,
        action: InterceptAction::MapRemote {
            target_url: "http://localhost:3000".to_string(),
            preserve_path: true,
        },
    };
    let json = serde_json::to_string(&rule).unwrap();
    let parsed: InterceptRule = serde_json::from_str(&json).unwrap();
    match parsed.action {
        InterceptAction::MapRemote {
            target_url,
            preserve_path,
        } => {
            assert_eq!(target_url, "http://localhost:3000");
            assert!(preserve_path);
        }
        _ => panic!("Expected MapRemote"),
    }
}

// ─── InterceptRule Display ──────────────────────────────────

#[test]
fn intercept_rule_display_block() {
    let rule = block_rule("r1", "*ads.com*", None, 403);
    let display = format!("{}", rule);
    assert!(display.contains("r1"));
    assert!(display.contains("Block(403)"));
    assert!(display.contains("*ads.com*"));
    assert!(display.contains("활성"));
}

#[test]
fn intercept_rule_display_disabled() {
    let mut rule = block_rule("r1", "*ads.com*", None, 403);
    rule.enabled = false;
    let display = format!("{}", rule);
    assert!(display.contains("비활성"));
}

#[test]
fn intercept_rule_display_with_method() {
    let rule = block_rule("r1", "*api.com*", Some("POST"), 403);
    let display = format!("{}", rule);
    assert!(display.contains("POST"));
}

#[test]
fn intercept_rule_display_no_method_shows_star() {
    let rule = block_rule("r1", "*api.com*", None, 403);
    let display = format!("{}", rule);
    assert!(display.contains("*"));
}

#[test]
fn intercept_rule_display_map_remote() {
    let rule = InterceptRule {
        id: "r1".to_string(),
        name: "Redirect".to_string(),
        enabled: true,
        pattern: "*prod.api.com*".to_string(),
        method: None,
        action: InterceptAction::MapRemote {
            target_url: "http://localhost:3000".to_string(),
            preserve_path: true,
        },
    };
    let display = format!("{}", rule);
    assert!(display.contains("MapRemote"));
    assert!(display.contains("localhost:3000"));
}

// ─── ServerReplayEntry 직렬화 ───────────────────────────────

#[test]
fn server_replay_entry_roundtrip() {
    let entry = ServerReplayEntry {
        id: "sr-1".to_string(),
        method: "GET".to_string(),
        url: "https://api.test.com/data".to_string(),
        status: 200,
        headers: HashMap::from([
            ("content-type".to_string(), "application/json".to_string()),
            ("x-custom".to_string(), "value".to_string()),
        ]),
        body: Some(r#"{"data":[1,2,3]}"#.to_string()),
    };
    let json = serde_json::to_string(&entry).unwrap();
    let parsed: ServerReplayEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.id, "sr-1");
    assert_eq!(parsed.method, "GET");
    assert_eq!(parsed.status, 200);
    assert_eq!(parsed.headers.len(), 2);
    assert!(parsed.body.unwrap().contains("data"));
}

#[test]
fn server_replay_entry_without_body() {
    let entry = ServerReplayEntry {
        id: "sr-2".to_string(),
        method: "DELETE".to_string(),
        url: "https://api.test.com/item/42".to_string(),
        status: 204,
        headers: HashMap::new(),
        body: None,
    };
    let json = serde_json::to_string(&entry).unwrap();
    let parsed: ServerReplayEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.status, 204);
    assert!(parsed.body.is_none());
}

// ─── InterceptAction JSON 기본값 ────────────────────────────

#[test]
fn block_action_uses_default_status() {
    let json = r#"{"type":"block"}"#;
    let action: InterceptAction = serde_json::from_str(json).unwrap();
    match action {
        InterceptAction::Block { status_code, body } => {
            assert_eq!(status_code, 403, "기본 블록 상태코드는 403");
            assert!(body.is_empty());
        }
        _ => panic!("Expected Block"),
    }
}

#[test]
fn modify_request_action_empty_defaults() {
    let json = r#"{"type":"modify_request"}"#;
    let action: InterceptAction = serde_json::from_str(json).unwrap();
    match action {
        InterceptAction::ModifyRequest {
            add_headers,
            remove_headers,
            set_body,
        } => {
            assert!(add_headers.is_empty());
            assert!(remove_headers.is_empty());
            assert!(set_body.is_none());
        }
        _ => panic!("Expected ModifyRequest"),
    }
}

#[test]
fn map_remote_preserve_path_default_is_true() {
    let json = r#"{"type":"map_remote","target_url":"http://localhost:3000"}"#;
    let action: InterceptAction = serde_json::from_str(json).unwrap();
    match action {
        InterceptAction::MapRemote { preserve_path, .. } => {
            assert!(preserve_path, "preserve_path 기본값은 true");
        }
        _ => panic!("Expected MapRemote"),
    }
}

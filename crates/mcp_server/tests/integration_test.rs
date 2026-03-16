use cheolsu_proxy_mcp::helpers::{
    format_size, next_breakpoint_id, next_mapping_id, next_rule_id, next_server_replay_id,
};
use cheolsu_proxy_mcp::store::{Store, MAX_SSE_EVENTS, MAX_TRANSACTIONS, MAX_WS_MESSAGES};
use proxy_daemon::{
    BreakpointRule, HostMapping, InterceptAction, InterceptRule, ServerReplayEntry,
};
use proxy_v2_models::{
    RequestInfo, SseConnectionEvent, SseEventInfo, WsConnectionEvent, WsContentType, WsDirection,
    WsMessageInfo, WsMessageType,
};

// ─── 헬퍼 ───────────────────────────────────────────────────

/// serde_json을 이용해 private 필드를 가진 ClientRequest를 생성
fn make_client_request(id: &str, method: &str, uri: &str) -> proxy_v2_models::ClientRequest {
    serde_json::from_value(serde_json::json!({
        "method": method,
        "uri": uri,
        "version": "HTTP/1.1",
        "headers": {},
        "body": null,
        "time": 1000,
        "id": id,
        "data_type": "Unknown",
        "body_json": null,
        "file_path": null,
        "body_size": 0
    }))
    .expect("ClientRequest deserialization failed")
}

/// serde_json을 이용해 private 필드를 가진 ClientResponse를 생성
fn make_client_response(id: &str, status: u16) -> proxy_v2_models::ClientResponse {
    serde_json::from_value(serde_json::json!({
        "status": status,
        "version": "HTTP/1.1",
        "headers": {},
        "body": null,
        "time": 1001,
        "id": id,
        "data_type": "Unknown",
        "body_json": null,
        "file_path": null,
        "body_size": 0
    }))
    .expect("ClientResponse deserialization failed")
}

fn make_request_info(id: &str, method: &str, uri: &str, status: u16) -> RequestInfo {
    RequestInfo(
        Some(make_client_request(id, method, uri)),
        Some(make_client_response(id, status)),
        None,
    )
}

fn make_sse_event(connection_id: &str, seq: u64, data: &str) -> SseEventInfo {
    SseEventInfo {
        connection_id: connection_id.to_string(),
        sequence: seq,
        event_type: Some("message".to_string()),
        data: data.to_string(),
        id: None,
        retry: None,
        size: data.len(),
        time: 1000 + seq as i64,
    }
}

fn make_ws_message(connection_id: &str, seq: u64, payload: &str) -> WsMessageInfo {
    WsMessageInfo {
        connection_id: connection_id.to_string(),
        sequence: seq,
        direction: WsDirection::ClientToServer,
        message_type: WsMessageType::Text,
        payload: payload.to_string(),
        size: payload.len(),
        time: 1000 + seq as i64,
        is_binary: false,
        content_type: WsContentType::Plain,
        mqtt_version: None,
    }
}

// ─── Store 기본 테스트 ──────────────────────────────────────

#[test]
fn store_new_is_empty() {
    let store = Store::new();
    assert_eq!(store.transactions.lock().len(), 0);
    assert_eq!(store.ws_messages.lock().len(), 0);
    assert_eq!(store.ws_connections.lock().len(), 0);
    assert_eq!(store.rules.lock().len(), 0);
    assert_eq!(store.breakpoint_rules.lock().len(), 0);
    assert_eq!(store.host_mappings.lock().len(), 0);
    assert_eq!(store.sse_events.lock().len(), 0);
    assert_eq!(store.sse_connections.lock().len(), 0);
    assert_eq!(store.server_replay_entries.lock().len(), 0);
}

#[test]
fn store_push_transaction() {
    let store = Store::new();
    let info = make_request_info("tx1", "GET", "https://example.com/api", 200);
    store.push_transaction(info);

    let txns = store.transactions.lock();
    assert_eq!(txns.len(), 1);
    let req = txns[0].0.as_ref().unwrap();
    assert_eq!(req.id(), "tx1");
    assert_eq!(req.method().as_str(), "GET");
    assert_eq!(req.uri().to_string(), "https://example.com/api");
}

#[test]
fn store_push_multiple_transactions() {
    let store = Store::new();
    for i in 0..5 {
        let info = make_request_info(
            &format!("tx{}", i),
            "POST",
            &format!("https://example.com/{}", i),
            201,
        );
        store.push_transaction(info);
    }

    let txns = store.transactions.lock();
    assert_eq!(txns.len(), 5);
    // VecDeque 순서: 처음 삽입된 것이 front
    assert_eq!(txns[0].0.as_ref().unwrap().id(), "tx0");
    assert_eq!(txns[4].0.as_ref().unwrap().id(), "tx4");
}

#[test]
fn store_transaction_capacity_eviction() {
    let store = Store::new();
    // MAX_TRANSACTIONS 이상 삽입 시 오래된 것부터 제거
    for i in 0..MAX_TRANSACTIONS + 10 {
        let info = make_request_info(
            &format!("tx{}", i),
            "GET",
            &format!("https://example.com/{}", i),
            200,
        );
        store.push_transaction(info);
    }

    let txns = store.transactions.lock();
    assert_eq!(txns.len(), MAX_TRANSACTIONS);
    // 가장 오래된(처음 10개)은 제거됨
    let first_id = txns.front().unwrap().0.as_ref().unwrap().id().to_string();
    assert_eq!(first_id, "tx10");
    let last_id = txns.back().unwrap().0.as_ref().unwrap().id().to_string();
    assert_eq!(last_id, format!("tx{}", MAX_TRANSACTIONS + 9));
}

// ─── WebSocket 메시지 테스트 ────────────────────────────────

#[test]
fn store_push_ws_message() {
    let store = Store::new();
    let msg = make_ws_message("ws://example.com/ws", 1, "hello");
    store.push_ws_message(msg);

    let msgs = store.ws_messages.lock();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].payload, "hello");
    assert_eq!(msgs[0].connection_id, "ws://example.com/ws");
}

#[test]
fn store_ws_message_capacity_eviction() {
    let store = Store::new();
    for i in 0..MAX_WS_MESSAGES + 5 {
        let msg = make_ws_message("ws://test", i as u64, &format!("msg{}", i));
        store.push_ws_message(msg);
    }

    let msgs = store.ws_messages.lock();
    assert_eq!(msgs.len(), MAX_WS_MESSAGES);
    assert_eq!(msgs.front().unwrap().payload, "msg5");
}

// ─── WebSocket 연결 이벤트 테스트 ───────────────────────────

#[test]
fn store_push_ws_connection() {
    let store = Store::new();
    let event = WsConnectionEvent::Connected {
        connection_id: "conn1".to_string(),
        uri: "ws://example.com".to_string(),
        time: 1000,
    };
    store.push_ws_connection(event);

    let conns = store.ws_connections.lock();
    assert_eq!(conns.len(), 1);
    match &conns[0] {
        WsConnectionEvent::Connected {
            connection_id, uri, ..
        } => {
            assert_eq!(connection_id, "conn1");
            assert_eq!(uri, "ws://example.com");
        }
        _ => panic!("expected Connected event"),
    }
}

#[test]
fn store_push_ws_connection_disconnect() {
    let store = Store::new();
    store.push_ws_connection(WsConnectionEvent::Connected {
        connection_id: "conn1".to_string(),
        uri: "ws://example.com".to_string(),
        time: 1000,
    });
    store.push_ws_connection(WsConnectionEvent::Disconnected {
        connection_id: "conn1".to_string(),
        time: 2000,
    });

    let conns = store.ws_connections.lock();
    assert_eq!(conns.len(), 2);
}

// ─── 인터셉트 규칙 CRUD 테스트 ─────────────────────────────

#[test]
fn store_rules_crud() {
    let store = Store::new();

    // Create
    let rule = InterceptRule {
        id: "r1".to_string(),
        name: "Block ads".to_string(),
        enabled: true,
        pattern: "*.ads.example.com/*".to_string(),
        method: None,
        action: InterceptAction::Block {
            status_code: 403,
            body: "Blocked".to_string(),
        },
    };
    store.rules.lock().push(rule);

    // Read
    {
        let rules = store.rules.lock();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].name, "Block ads");
        assert_eq!(rules[0].pattern, "*.ads.example.com/*");
    }

    // Update
    {
        let mut rules = store.rules.lock();
        rules[0].enabled = false;
        rules[0].name = "Block ads (disabled)".to_string();
    }
    {
        let rules = store.rules.lock();
        assert!(!rules[0].enabled);
        assert_eq!(rules[0].name, "Block ads (disabled)");
    }

    // Delete
    {
        let mut rules = store.rules.lock();
        rules.retain(|r| r.id != "r1");
    }
    assert_eq!(store.rules.lock().len(), 0);
}

#[test]
fn store_multiple_rules() {
    let store = Store::new();

    let actions = vec![
        (
            "r1",
            "Block",
            InterceptAction::Block {
                status_code: 403,
                body: String::new(),
            },
        ),
        (
            "r2",
            "Modify",
            InterceptAction::ModifyResponse {
                set_status: Some(200),
                add_headers: Default::default(),
                remove_headers: vec![],
                set_body: None,
            },
        ),
        (
            "r3",
            "MapRemote",
            InterceptAction::MapRemote {
                target_url: "https://staging.example.com".to_string(),
                preserve_path: true,
            },
        ),
    ];

    for (id, name, action) in actions {
        store.rules.lock().push(InterceptRule {
            id: id.to_string(),
            name: name.to_string(),
            enabled: true,
            pattern: "*.example.com/*".to_string(),
            method: None,
            action,
        });
    }

    assert_eq!(store.rules.lock().len(), 3);

    // 특정 규칙만 삭제
    store.rules.lock().retain(|r| r.id != "r2");
    assert_eq!(store.rules.lock().len(), 2);

    let rules = store.rules.lock();
    assert_eq!(rules[0].id, "r1");
    assert_eq!(rules[1].id, "r3");
}

// ─── 브레이크포인트 규칙 테스트 ─────────────────────────────

#[test]
fn store_breakpoint_rules_crud() {
    let store = Store::new();

    let bp = BreakpointRule {
        id: "bp1".to_string(),
        pattern: "*.api.example.com/*".to_string(),
        break_on_request: true,
        break_on_response: false,
        enabled: true,
    };
    store.breakpoint_rules.lock().push(bp);

    assert_eq!(store.breakpoint_rules.lock().len(), 1);
    assert_eq!(
        store.breakpoint_rules.lock()[0].pattern,
        "*.api.example.com/*"
    );

    // Delete
    store.breakpoint_rules.lock().retain(|r| r.id != "bp1");
    assert_eq!(store.breakpoint_rules.lock().len(), 0);
}

// ─── 호스트 매핑 테스트 ─────────────────────────────────────

#[test]
fn store_host_mappings_crud() {
    let store = Store::new();

    let mapping = HostMapping {
        id: "hm1".to_string(),
        source_host: "api.example.com".to_string(),
        source_port: None,
        target_host: "127.0.0.1".to_string(),
        target_port: Some(8080),
        enabled: true,
    };
    store.host_mappings.lock().push(mapping);

    {
        let mappings = store.host_mappings.lock();
        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].source_host, "api.example.com");
        assert_eq!(mappings[0].target_host, "127.0.0.1");
        assert_eq!(mappings[0].target_port, Some(8080));
    }

    // Update
    store.host_mappings.lock()[0].target_port = Some(9090);
    assert_eq!(store.host_mappings.lock()[0].target_port, Some(9090));

    // Delete
    store.host_mappings.lock().retain(|m| m.id != "hm1");
    assert_eq!(store.host_mappings.lock().len(), 0);
}

#[test]
fn store_host_mapping_wildcard_pattern() {
    let store = Store::new();

    store.host_mappings.lock().push(HostMapping {
        id: "hm_wild".to_string(),
        source_host: "*.api.example.com".to_string(),
        source_port: Some(443),
        target_host: "10.0.0.1".to_string(),
        target_port: None,
        enabled: true,
    });

    let mappings = store.host_mappings.lock();
    assert!(mappings[0].source_host.starts_with('*'));
    assert_eq!(mappings[0].source_port, Some(443));
    assert_eq!(mappings[0].target_port, None);
}

// ─── 헬퍼 함수 테스트 ──────────────────────────────────────

#[test]
fn format_size_bytes() {
    assert_eq!(format_size(0), "0B");
    assert_eq!(format_size(100), "100B");
    assert_eq!(format_size(1023), "1023B");
}

#[test]
fn format_size_kilobytes() {
    assert_eq!(format_size(1024), "1.0KB");
    assert_eq!(format_size(1536), "1.5KB");
    assert_eq!(format_size(10240), "10.0KB");
}

#[test]
fn format_size_megabytes() {
    assert_eq!(format_size(1024 * 1024), "1.0MB");
    assert_eq!(format_size(1024 * 1024 * 5), "5.0MB");
}

#[test]
fn next_rule_id_increments() {
    let id1 = next_rule_id();
    let id2 = next_rule_id();
    assert!(id1.starts_with("mcp_"));
    assert!(id2.starts_with("mcp_"));
    assert_ne!(id1, id2);
}

#[test]
fn next_breakpoint_id_increments() {
    let id1 = next_breakpoint_id();
    let id2 = next_breakpoint_id();
    assert!(id1.starts_with("mcp_bp_"));
    assert!(id2.starts_with("mcp_bp_"));
    assert_ne!(id1, id2);
}

#[test]
fn next_mapping_id_increments() {
    let id1 = next_mapping_id();
    let id2 = next_mapping_id();
    assert!(id1.starts_with("hm_"));
    assert!(id2.starts_with("hm_"));
    assert_ne!(id1, id2);
}

// ─── Store 스레드 안전성 테스트 ─────────────────────────────

#[test]
fn store_concurrent_access() {
    use std::sync::Arc;
    use std::thread;

    let store = Arc::new(Store::new());
    let mut handles = vec![];

    // 여러 스레드에서 동시에 트랜잭션 추가
    for i in 0..10 {
        let store = store.clone();
        handles.push(thread::spawn(move || {
            for j in 0..10 {
                let id = format!("tx_{}_{}", i, j);
                let info = make_request_info(&id, "GET", "https://example.com", 200);
                store.push_transaction(info);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(store.transactions.lock().len(), 100);
}

#[test]
fn store_concurrent_ws_messages() {
    use std::sync::Arc;
    use std::thread;

    let store = Arc::new(Store::new());
    let mut handles = vec![];

    for i in 0..10 {
        let store = store.clone();
        handles.push(thread::spawn(move || {
            for j in 0..10 {
                let msg = make_ws_message("ws://test", (i * 10 + j) as u64, "data");
                store.push_ws_message(msg);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(store.ws_messages.lock().len(), 100);
}

// ─── Store Clone 테스트 ─────────────────────────────────────

#[test]
fn store_clone_shares_data() {
    let store1 = Store::new();
    let store2 = store1.clone();

    let info = make_request_info("tx_shared", "GET", "https://example.com", 200);
    store1.push_transaction(info);

    // clone된 store에서도 같은 데이터가 보여야 함 (Arc 공유)
    assert_eq!(store2.transactions.lock().len(), 1);
    assert_eq!(
        store2.transactions.lock()[0].0.as_ref().unwrap().id(),
        "tx_shared"
    );
}

// ─── RequestInfo 없는 경우 테스트 ───────────────────────────

#[test]
fn store_push_partial_request_info() {
    let store = Store::new();

    // request만 있고 response가 없는 경우
    let info = RequestInfo(
        Some(make_client_request(
            "tx_partial",
            "GET",
            "https://example.com",
        )),
        None,
        None,
    );
    store.push_transaction(info);

    let txns = store.transactions.lock();
    assert_eq!(txns.len(), 1);
    assert!(txns[0].0.is_some());
    assert!(txns[0].1.is_none());
}

#[test]
fn store_push_empty_request_info() {
    let store = Store::new();

    // request도 response도 없는 경우
    let info = RequestInfo(None, None, None);
    store.push_transaction(info);

    let txns = store.transactions.lock();
    assert_eq!(txns.len(), 1);
    assert!(txns[0].0.is_none());
    assert!(txns[0].1.is_none());
}

// ─── InterceptRule 다양한 액션 타입 테스트 ──────────────────

#[test]
fn intercept_rule_block_action() {
    let store = Store::new();
    store.rules.lock().push(InterceptRule {
        id: next_rule_id(),
        name: "Block".to_string(),
        enabled: true,
        pattern: "*.example.com/*".to_string(),
        method: Some("GET".to_string()),
        action: InterceptAction::Block {
            status_code: 403,
            body: "Forbidden".to_string(),
        },
    });

    let rules = store.rules.lock();
    match &rules[0].action {
        InterceptAction::Block { status_code, body } => {
            assert_eq!(*status_code, 403);
            assert_eq!(body, "Forbidden");
        }
        _ => panic!("expected Block action"),
    }
    assert_eq!(rules[0].method.as_deref(), Some("GET"));
}

#[test]
fn intercept_rule_modify_request_action() {
    let store = Store::new();
    let mut add_headers = std::collections::HashMap::new();
    add_headers.insert("X-Custom".to_string(), "value".to_string());

    store.rules.lock().push(InterceptRule {
        id: next_rule_id(),
        name: "Modify request".to_string(),
        enabled: true,
        pattern: "*".to_string(),
        method: None,
        action: InterceptAction::ModifyRequest {
            add_headers: add_headers.clone(),
            remove_headers: vec!["Cookie".to_string()],
            set_body: Some("new body".to_string()),
        },
    });

    let rules = store.rules.lock();
    match &rules[0].action {
        InterceptAction::ModifyRequest {
            add_headers,
            remove_headers,
            set_body,
        } => {
            assert_eq!(add_headers.get("X-Custom").unwrap(), "value");
            assert_eq!(remove_headers, &vec!["Cookie".to_string()]);
            assert_eq!(set_body.as_deref(), Some("new body"));
        }
        _ => panic!("expected ModifyRequest action"),
    }
}

#[test]
fn intercept_rule_map_local_action() {
    let store = Store::new();
    store.rules.lock().push(InterceptRule {
        id: next_rule_id(),
        name: "Map local".to_string(),
        enabled: true,
        pattern: "*.api.example.com/data".to_string(),
        method: None,
        action: InterceptAction::MapLocal {
            file_path: "/tmp/mock.json".to_string(),
            status_code: 200,
            headers: Default::default(),
        },
    });

    let rules = store.rules.lock();
    match &rules[0].action {
        InterceptAction::MapLocal {
            file_path,
            status_code,
            ..
        } => {
            assert_eq!(file_path, "/tmp/mock.json");
            assert_eq!(*status_code, 200);
        }
        _ => panic!("expected MapLocal action"),
    }
}

#[test]
fn intercept_rule_map_remote_action() {
    let store = Store::new();
    store.rules.lock().push(InterceptRule {
        id: next_rule_id(),
        name: "Map remote".to_string(),
        enabled: true,
        pattern: "api.example.com/*".to_string(),
        method: None,
        action: InterceptAction::MapRemote {
            target_url: "https://staging.example.com".to_string(),
            preserve_path: true,
        },
    });

    let rules = store.rules.lock();
    match &rules[0].action {
        InterceptAction::MapRemote {
            target_url,
            preserve_path,
        } => {
            assert_eq!(target_url, "https://staging.example.com");
            assert!(*preserve_path);
        }
        _ => panic!("expected MapRemote action"),
    }
}

// ─── 복합 시나리오 테스트 ───────────────────────────────────

#[test]
fn full_workflow_scenario() {
    let store = Store::new();

    // 1. 트래픽 캡처
    for i in 0..5 {
        let info = make_request_info(
            &format!("tx{}", i),
            if i % 2 == 0 { "GET" } else { "POST" },
            &format!("https://api.example.com/v1/resource/{}", i),
            200,
        );
        store.push_transaction(info);
    }
    assert_eq!(store.transactions.lock().len(), 5);

    // 2. 인터셉트 규칙 추가
    store.rules.lock().push(InterceptRule {
        id: "rule_1".to_string(),
        name: "Block analytics".to_string(),
        enabled: true,
        pattern: "*.analytics.example.com/*".to_string(),
        method: None,
        action: InterceptAction::Block {
            status_code: 403,
            body: String::new(),
        },
    });

    // 3. 브레이크포인트 추가
    store.breakpoint_rules.lock().push(BreakpointRule {
        id: "bp_1".to_string(),
        pattern: "*.api.example.com/v1/auth/*".to_string(),
        break_on_request: true,
        break_on_response: true,
        enabled: true,
    });

    // 4. 호스트 매핑 추가
    store.host_mappings.lock().push(HostMapping {
        id: "hm_1".to_string(),
        source_host: "api.example.com".to_string(),
        source_port: None,
        target_host: "staging.example.com".to_string(),
        target_port: None,
        enabled: true,
    });

    // 5. WebSocket 메시지 추가
    for i in 0..3 {
        store.push_ws_message(make_ws_message(
            "ws://api.example.com/ws",
            i,
            &format!("{{\"type\":\"ping\",\"seq\":{}}}", i),
        ));
    }

    // 6. WebSocket 연결 추가
    store.push_ws_connection(WsConnectionEvent::Connected {
        connection_id: "ws_conn_1".to_string(),
        uri: "ws://api.example.com/ws".to_string(),
        time: 1000,
    });

    // 전체 상태 검증
    assert_eq!(store.transactions.lock().len(), 5);
    assert_eq!(store.rules.lock().len(), 1);
    assert_eq!(store.breakpoint_rules.lock().len(), 1);
    assert_eq!(store.host_mappings.lock().len(), 1);
    assert_eq!(store.ws_messages.lock().len(), 3);
    assert_eq!(store.ws_connections.lock().len(), 1);

    // 7. 트래픽 클리어
    store.transactions.lock().clear();
    store.ws_messages.lock().clear();
    store.ws_connections.lock().clear();
    assert_eq!(store.transactions.lock().len(), 0);
    assert_eq!(store.ws_messages.lock().len(), 0);
    assert_eq!(store.ws_connections.lock().len(), 0);

    // 규칙은 유지
    assert_eq!(store.rules.lock().len(), 1);
    assert_eq!(store.breakpoint_rules.lock().len(), 1);
    assert_eq!(store.host_mappings.lock().len(), 1);
}

// ─── 트래픽 검색 시뮬레이션 테스트 ─────────────────────────

#[test]
fn search_traffic_by_method() {
    let store = Store::new();
    store.push_transaction(make_request_info("tx1", "GET", "https://a.com/1", 200));
    store.push_transaction(make_request_info("tx2", "POST", "https://a.com/2", 201));
    store.push_transaction(make_request_info("tx3", "GET", "https://a.com/3", 200));
    store.push_transaction(make_request_info("tx4", "PUT", "https://a.com/4", 204));

    let txns = store.transactions.lock();
    let get_count = txns
        .iter()
        .filter(|info| {
            info.0
                .as_ref()
                .map(|r| r.method().as_str() == "GET")
                .unwrap_or(false)
        })
        .count();
    assert_eq!(get_count, 2);
}

#[test]
fn search_traffic_by_host() {
    let store = Store::new();
    store.push_transaction(make_request_info(
        "tx1",
        "GET",
        "https://api.example.com/users",
        200,
    ));
    store.push_transaction(make_request_info(
        "tx2",
        "GET",
        "https://cdn.other.com/img.png",
        200,
    ));
    store.push_transaction(make_request_info(
        "tx3",
        "GET",
        "https://api.example.com/posts",
        200,
    ));

    let txns = store.transactions.lock();
    let example_count = txns
        .iter()
        .filter(|info| {
            info.0
                .as_ref()
                .map(|r| r.uri().to_string().contains("example.com"))
                .unwrap_or(false)
        })
        .count();
    assert_eq!(example_count, 2);
}

#[test]
fn search_traffic_by_status() {
    let store = Store::new();
    store.push_transaction(make_request_info("tx1", "GET", "https://a.com/ok", 200));
    store.push_transaction(make_request_info(
        "tx2",
        "GET",
        "https://a.com/not-found",
        404,
    ));
    store.push_transaction(make_request_info("tx3", "GET", "https://a.com/error", 500));
    store.push_transaction(make_request_info("tx4", "GET", "https://a.com/ok2", 200));

    let txns = store.transactions.lock();
    let error_count = txns
        .iter()
        .filter(|info| {
            info.1
                .as_ref()
                .map(|r| r.status().as_u16() >= 400)
                .unwrap_or(false)
        })
        .count();
    assert_eq!(error_count, 2);
}

#[test]
fn search_traffic_by_path() {
    let store = Store::new();
    store.push_transaction(make_request_info(
        "tx1",
        "GET",
        "https://api.com/v1/users",
        200,
    ));
    store.push_transaction(make_request_info(
        "tx2",
        "POST",
        "https://api.com/v1/users",
        201,
    ));
    store.push_transaction(make_request_info(
        "tx3",
        "GET",
        "https://api.com/v2/posts",
        200,
    ));

    let txns = store.transactions.lock();
    let v1_users_count = txns
        .iter()
        .filter(|info| {
            info.0
                .as_ref()
                .map(|r| r.uri().to_string().contains("/v1/users"))
                .unwrap_or(false)
        })
        .count();
    assert_eq!(v1_users_count, 2);
}

// ─── WebSocket 메시지 필터링 시뮬레이션 테스트 ──────────────

#[test]
fn filter_ws_messages_by_connection() {
    let store = Store::new();
    store.push_ws_message(make_ws_message("ws://a.com/ws", 1, "a1"));
    store.push_ws_message(make_ws_message("ws://b.com/ws", 2, "b1"));
    store.push_ws_message(make_ws_message("ws://a.com/ws", 3, "a2"));

    let msgs = store.ws_messages.lock();
    let a_count = msgs
        .iter()
        .filter(|m| m.connection_id.contains("a.com"))
        .count();
    assert_eq!(a_count, 2);
}

// ─── 규칙 조회 시뮬레이션 ──────────────────────────────────

#[test]
fn rule_remove_nonexistent() {
    let store = Store::new();
    store.rules.lock().push(InterceptRule {
        id: "r1".to_string(),
        name: "Rule 1".to_string(),
        enabled: true,
        pattern: "*".to_string(),
        method: None,
        action: InterceptAction::Block {
            status_code: 403,
            body: String::new(),
        },
    });

    let before = store.rules.lock().len();
    store.rules.lock().retain(|r| r.id != "nonexistent");
    let after = store.rules.lock().len();

    // 존재하지 않는 ID로 삭제해도 기존 규칙은 유지
    assert_eq!(before, after);
    assert_eq!(after, 1);
}

// ─── SSE 이벤트 테스트 ──────────────────────────────────

#[test]
fn store_push_sse_event() {
    let store = Store::new();
    let event = make_sse_event("sse://example.com/events", 1, "hello SSE");
    store.push_sse_event(event);

    let events = store.sse_events.lock();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].data, "hello SSE");
    assert_eq!(events[0].connection_id, "sse://example.com/events");
    assert_eq!(events[0].event_type, Some("message".to_string()));
}

#[test]
fn store_push_multiple_sse_events() {
    let store = Store::new();
    for i in 0..5 {
        store.push_sse_event(make_sse_event("sse://test", i, &format!("event{}", i)));
    }

    let events = store.sse_events.lock();
    assert_eq!(events.len(), 5);
    assert_eq!(events[0].data, "event0");
    assert_eq!(events[4].data, "event4");
}

#[test]
fn store_sse_event_capacity_eviction() {
    let store = Store::new();
    for i in 0..(MAX_SSE_EVENTS + 10) as u64 {
        store.push_sse_event(make_sse_event("sse://test", i, &format!("evt{}", i)));
    }

    let events = store.sse_events.lock();
    assert_eq!(events.len(), MAX_SSE_EVENTS);
    // 가장 오래된 10개는 제거됨
    assert_eq!(events.front().unwrap().sequence, 10);
}

#[test]
fn store_push_sse_connection() {
    let store = Store::new();
    store.push_sse_connection(SseConnectionEvent::Connected {
        connection_id: "sse_conn1".to_string(),
        uri: "https://example.com/events".to_string(),
        time: 1000,
    });

    let conns = store.sse_connections.lock();
    assert_eq!(conns.len(), 1);
    match &conns[0] {
        SseConnectionEvent::Connected {
            connection_id, uri, ..
        } => {
            assert_eq!(connection_id, "sse_conn1");
            assert_eq!(uri, "https://example.com/events");
        }
        _ => panic!("expected Connected event"),
    }
}

#[test]
fn store_push_sse_connection_disconnect() {
    let store = Store::new();
    store.push_sse_connection(SseConnectionEvent::Connected {
        connection_id: "sse_conn1".to_string(),
        uri: "https://example.com/events".to_string(),
        time: 1000,
    });
    store.push_sse_connection(SseConnectionEvent::Disconnected {
        connection_id: "sse_conn1".to_string(),
        time: 2000,
    });

    let conns = store.sse_connections.lock();
    assert_eq!(conns.len(), 2);
}

#[test]
fn filter_sse_events_by_connection() {
    let store = Store::new();
    store.push_sse_event(make_sse_event("sse://a.com/events", 1, "a1"));
    store.push_sse_event(make_sse_event("sse://b.com/events", 2, "b1"));
    store.push_sse_event(make_sse_event("sse://a.com/events", 3, "a2"));

    let events = store.sse_events.lock();
    let a_count = events
        .iter()
        .filter(|e| e.connection_id.contains("a.com"))
        .count();
    assert_eq!(a_count, 2);
}

#[test]
fn store_concurrent_sse_events() {
    use std::sync::Arc;
    use std::thread;

    let store = Arc::new(Store::new());
    let mut handles = vec![];

    for i in 0..10 {
        let store = store.clone();
        handles.push(thread::spawn(move || {
            for j in 0..10 {
                let event = make_sse_event("sse://test", (i * 10 + j) as u64, "data");
                store.push_sse_event(event);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(store.sse_events.lock().len(), 100);
}

// ─── Server Replay 테스트 ───────────────────────────────

#[test]
fn store_server_replay_crud() {
    let store = Store::new();

    // Create
    let entry = ServerReplayEntry {
        id: "sr_1".to_string(),
        method: "GET".to_string(),
        url: "https://api.example.com/users".to_string(),
        status: 200,
        headers: std::collections::HashMap::new(),
        body: Some(r#"[{"id":1,"name":"test"}]"#.to_string()),
    };
    store.server_replay_entries.lock().push(entry);

    // Read
    {
        let entries = store.server_replay_entries.lock();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].method, "GET");
        assert_eq!(entries[0].url, "https://api.example.com/users");
        assert_eq!(entries[0].status, 200);
        assert!(entries[0].body.is_some());
    }

    // Update
    {
        let mut entries = store.server_replay_entries.lock();
        entries[0].status = 201;
    }
    assert_eq!(store.server_replay_entries.lock()[0].status, 201);

    // Delete
    store
        .server_replay_entries
        .lock()
        .retain(|e| e.id != "sr_1");
    assert_eq!(store.server_replay_entries.lock().len(), 0);
}

#[test]
fn store_multiple_server_replay_entries() {
    let store = Store::new();

    for i in 0..3 {
        store.server_replay_entries.lock().push(ServerReplayEntry {
            id: format!("sr_{}", i),
            method: "GET".to_string(),
            url: format!("https://api.example.com/resource/{}", i),
            status: 200,
            headers: std::collections::HashMap::new(),
            body: None,
        });
    }

    assert_eq!(store.server_replay_entries.lock().len(), 3);

    // 특정 엔트리만 삭제
    store
        .server_replay_entries
        .lock()
        .retain(|e| e.id != "sr_1");
    assert_eq!(store.server_replay_entries.lock().len(), 2);

    let entries = store.server_replay_entries.lock();
    assert_eq!(entries[0].id, "sr_0");
    assert_eq!(entries[1].id, "sr_2");
}

#[test]
fn store_server_replay_clear() {
    let store = Store::new();
    for i in 0..5 {
        store.server_replay_entries.lock().push(ServerReplayEntry {
            id: format!("sr_{}", i),
            method: "POST".to_string(),
            url: "https://api.example.com/data".to_string(),
            status: 201,
            headers: std::collections::HashMap::new(),
            body: Some("{}".to_string()),
        });
    }
    assert_eq!(store.server_replay_entries.lock().len(), 5);

    store.server_replay_entries.lock().clear();
    assert_eq!(store.server_replay_entries.lock().len(), 0);
}

// ─── next_server_replay_id 테스트 ───────────────────────

#[test]
fn next_server_replay_id_increments() {
    let id1 = next_server_replay_id();
    let id2 = next_server_replay_id();
    assert!(id1.starts_with("sr_"));
    assert!(id2.starts_with("sr_"));
    assert_ne!(id1, id2);
}

// ─── SSE + Server Replay Clone 공유 테스트 ──────────────

#[test]
fn store_clone_shares_sse_and_replay() {
    let store1 = Store::new();
    let store2 = store1.clone();

    store1.push_sse_event(make_sse_event("sse://test", 1, "shared"));
    store1.server_replay_entries.lock().push(ServerReplayEntry {
        id: "sr_shared".to_string(),
        method: "GET".to_string(),
        url: "https://example.com".to_string(),
        status: 200,
        headers: std::collections::HashMap::new(),
        body: None,
    });

    // clone된 store에서도 같은 데이터가 보여야 함
    assert_eq!(store2.sse_events.lock().len(), 1);
    assert_eq!(store2.sse_events.lock()[0].data, "shared");
    assert_eq!(store2.server_replay_entries.lock().len(), 1);
    assert_eq!(store2.server_replay_entries.lock()[0].id, "sr_shared");
}

// ─── full_workflow에 SSE + Server Replay 추가 ───────────

#[test]
fn full_workflow_with_sse_and_replay() {
    let store = Store::new();

    // 1. 트래픽 캡처
    for i in 0..3 {
        store.push_transaction(make_request_info(
            &format!("tx{}", i),
            "GET",
            &format!("https://api.example.com/v1/{}", i),
            200,
        ));
    }

    // 2. SSE 이벤트 캡처
    for i in 0..3 {
        store.push_sse_event(make_sse_event(
            "sse://api.example.com/stream",
            i,
            &format!(r#"{{"type":"update","id":{}}}"#, i),
        ));
    }
    store.push_sse_connection(SseConnectionEvent::Connected {
        connection_id: "sse_conn_1".to_string(),
        uri: "https://api.example.com/stream".to_string(),
        time: 1000,
    });

    // 3. Server Replay 설정
    store.server_replay_entries.lock().push(ServerReplayEntry {
        id: "sr_1".to_string(),
        method: "GET".to_string(),
        url: "https://api.example.com/v1/0".to_string(),
        status: 200,
        headers: std::collections::HashMap::new(),
        body: Some(r#"{"cached":true}"#.to_string()),
    });

    // 4. 전체 상태 검증
    assert_eq!(store.transactions.lock().len(), 3);
    assert_eq!(store.sse_events.lock().len(), 3);
    assert_eq!(store.sse_connections.lock().len(), 1);
    assert_eq!(store.server_replay_entries.lock().len(), 1);

    // 5. 트래픽 클리어 (SSE도 포함)
    store.transactions.lock().clear();
    store.sse_events.lock().clear();
    store.sse_connections.lock().clear();
    assert_eq!(store.transactions.lock().len(), 0);
    assert_eq!(store.sse_events.lock().len(), 0);
    assert_eq!(store.sse_connections.lock().len(), 0);

    // Server Replay 엔트리는 유지
    assert_eq!(store.server_replay_entries.lock().len(), 1);
}

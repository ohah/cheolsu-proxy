use proxy_daemon::{InterceptAction, InterceptRule};
use proxy_v2_models::{RequestInfo, WsConnectionEvent, WsDirection, WsMessageInfo};
use rmcp::{handler::server::wrapper::Parameters, model::*};

use crate::helpers::{format_size, next_rule_id, read_body_text, tool_error, tool_ok};
use crate::params::*;
use crate::server::CheolsuMcpServer;
use crate::store::{Store, MAX_TRANSACTIONS, MAX_WS_MESSAGES};

fn make_block_rule(id: &str) -> InterceptRule {
    InterceptRule {
        id: id.to_string(),
        name: format!("Rule {}", id),
        enabled: true,
        pattern: "*.example.com/*".to_string(),
        method: None,
        action: InterceptAction::Block {
            status_code: 403,
            body: String::new(),
        },
    }
}

fn make_empty_request_info() -> RequestInfo {
    RequestInfo(None, None)
}

fn make_ws_message(connection_id: &str) -> WsMessageInfo {
    WsMessageInfo {
        connection_id: connection_id.to_string(),
        sequence: 0,
        direction: WsDirection::ClientToServer,
        message_type: proxy_v2_models::WsMessageType::Text,
        payload: "hello".to_string(),
        size: 5,
        time: 0,
        is_binary: false,
        content_type: proxy_v2_models::WsContentType::Plain,
        mqtt_version: None,
    }
}

fn extract_text(result: &CallToolResult) -> &str {
    match &result.content[0].raw {
        rmcp::model::RawContent::Text(t) => t.text.as_str(),
        _ => panic!("Expected text content"),
    }
}

#[test]
fn test_store_new_is_empty() {
    let store = Store::new();
    assert_eq!(store.transactions.lock().len(), 0);
    assert_eq!(store.ws_messages.lock().len(), 0);
    assert_eq!(store.ws_connections.lock().len(), 0);
    assert_eq!(store.rules.lock().len(), 0);
}

#[test]
fn test_store_transactions_max_capacity() {
    let store = Store::new();
    for _ in 0..MAX_TRANSACTIONS + 100 {
        store.push_transaction(make_empty_request_info());
    }
    assert_eq!(store.transactions.lock().len(), MAX_TRANSACTIONS);
}

#[test]
fn test_store_ws_messages_max_capacity() {
    let store = Store::new();
    for _ in 0..MAX_WS_MESSAGES + 100 {
        store.push_ws_message(make_ws_message("test"));
    }
    assert_eq!(store.ws_messages.lock().len(), MAX_WS_MESSAGES);
}

#[test]
fn test_store_ws_connections_push() {
    let store = Store::new();
    let event = WsConnectionEvent::Connected {
        connection_id: "conn1".to_string(),
        uri: "wss://example.com".to_string(),
        time: 0,
    };
    store.push_ws_connection(event);
    assert_eq!(store.ws_connections.lock().len(), 1);
}

#[test]
fn test_store_rules_sync() {
    let store = Store::new();
    let rules = vec![make_block_rule("r1"), make_block_rule("r2")];
    *store.rules.lock() = rules;
    assert_eq!(store.rules.lock().len(), 2);
}

#[test]
fn test_broadcast_sync_preserves_app_rules_on_mcp_add() {
    let store = Store::new();
    *store.rules.lock() = vec![make_block_rule("uuid-1"), make_block_rule("uuid-2")];
    store.rules.lock().push(make_block_rule("mcp_0"));
    let guard = store.rules.lock();
    assert_eq!(guard.len(), 3);
    assert!(guard.iter().any(|r| r.id == "uuid-1"));
    assert!(guard.iter().any(|r| r.id == "uuid-2"));
    assert!(guard.iter().any(|r| r.id == "mcp_0"));
}

#[test]
fn test_broadcast_sync_updates_full_rules() {
    let store = Store::new();
    store.rules.lock().push(make_block_rule("mcp_0"));
    *store.rules.lock() = vec![
        make_block_rule("uuid-1"),
        make_block_rule("uuid-2"),
        make_block_rule("mcp_0"),
    ];
    assert_eq!(store.rules.lock().len(), 3);
}

#[test]
fn test_broadcast_sync_remove_mcp_rule() {
    let store = Store::new();
    *store.rules.lock() = vec![
        make_block_rule("uuid-1"),
        make_block_rule("mcp_0"),
        make_block_rule("mcp_1"),
    ];
    store.rules.lock().retain(|r| r.id != "mcp_1");
    let guard = store.rules.lock();
    assert_eq!(guard.len(), 2);
    assert!(guard.iter().any(|r| r.id == "uuid-1"));
    assert!(guard.iter().any(|r| r.id == "mcp_0"));
}

#[test]
fn test_broadcast_initial_empty_then_sync() {
    let store = Store::new();
    assert_eq!(store.rules.lock().len(), 0);
    *store.rules.lock() = vec![make_block_rule("uuid-1"), make_block_rule("uuid-2")];
    store.rules.lock().push(make_block_rule("mcp_0"));
    assert_eq!(store.rules.lock().len(), 3);
}

#[test]
fn test_format_size_bytes() {
    assert_eq!(format_size(0), "0B");
    assert_eq!(format_size(512), "512B");
    assert_eq!(format_size(1023), "1023B");
}

#[test]
fn test_format_size_kilobytes() {
    assert_eq!(format_size(1024), "1.0KB");
    assert_eq!(format_size(1536), "1.5KB");
    assert_eq!(format_size(10240), "10.0KB");
}

#[test]
fn test_format_size_megabytes() {
    assert_eq!(format_size(1024 * 1024), "1.0MB");
    assert_eq!(format_size(5 * 1024 * 1024), "5.0MB");
}

#[test]
fn test_next_rule_id_increments() {
    let id1 = next_rule_id();
    let id2 = next_rule_id();
    assert!(id1.starts_with("mcp_"));
    assert!(id2.starts_with("mcp_"));
    assert_ne!(id1, id2);
}

#[test]
fn test_read_body_text_none_path() {
    let result = read_body_text(&None, &proxy_v2_models::DataType::Json);
    assert_eq!(result, "(body not available)");
}

#[test]
fn test_read_body_text_nonexistent_file() {
    let path = Some("/nonexistent/path/body.bin".to_string());
    let result = read_body_text(&path, &proxy_v2_models::DataType::Json);
    assert_eq!(result, "(file read error)");
}

#[test]
fn test_read_body_text_binary_type() {
    let tmp = std::env::temp_dir().join("mcp_test_binary");
    std::fs::write(&tmp, b"\x00\x01\x02").unwrap();
    let path = Some(tmp.to_string_lossy().to_string());
    let result = read_body_text(&path, &proxy_v2_models::DataType::Image);
    assert!(result.starts_with("(binary,"));
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn test_read_body_text_valid_json() {
    let tmp = std::env::temp_dir().join("mcp_test_json");
    std::fs::write(&tmp, r#"{"key":"value"}"#).unwrap();
    let path = Some(tmp.to_string_lossy().to_string());
    let result = read_body_text(&path, &proxy_v2_models::DataType::Json);
    assert_eq!(result, r#"{"key":"value"}"#);
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn test_read_body_text_truncates_large() {
    let tmp = std::env::temp_dir().join("mcp_test_large");
    let large = "a".repeat(20000);
    std::fs::write(&tmp, &large).unwrap();
    let path = Some(tmp.to_string_lossy().to_string());
    let result = read_body_text(&path, &proxy_v2_models::DataType::Text);
    assert!(result.contains("truncated"));
    assert!(result.len() < 20000);
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn test_tool_ok_returns_success() {
    let result = tool_ok("test message").unwrap();
    assert!(!result.is_error.unwrap_or(false));
    assert_eq!(result.content.len(), 1);
}

#[test]
fn test_tool_error_returns_error() {
    let result = tool_error("error message").unwrap();
    assert!(result.is_error.unwrap_or(false));
    assert_eq!(result.content.len(), 1);
}

#[test]
fn test_search_traffic_params_all_none() {
    let json = r#"{}"#;
    let params: SearchTrafficParams = serde_json::from_str(json).unwrap();
    assert!(params.host.is_none());
    assert!(params.method.is_none());
    assert!(params.status.is_none());
    assert!(params.path.is_none());
    assert!(params.limit.is_none());
}

#[test]
fn test_search_traffic_params_with_filters() {
    let json = r#"{"host":"example.com","method":"GET","status":200,"path":"/api","limit":10}"#;
    let params: SearchTrafficParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.host.unwrap(), "example.com");
    assert_eq!(params.method.unwrap(), "GET");
    assert_eq!(params.status.unwrap(), 200);
    assert_eq!(params.path.unwrap(), "/api");
    assert_eq!(params.limit.unwrap(), 10);
}

#[test]
fn test_get_transaction_params() {
    let json = r#"{"id":"txn_123"}"#;
    let params: GetTransactionParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.id, "txn_123");
}

#[test]
fn test_get_ws_messages_params_empty() {
    let json = r#"{}"#;
    let params: GetWsMessagesParams = serde_json::from_str(json).unwrap();
    assert!(params.connection_id.is_none());
    assert!(params.limit.is_none());
}

#[test]
fn test_replay_request_params() {
    let json = r#"{"method":"POST","url":"https://api.com/test","headers":{"Content-Type":"application/json"},"body":"{\"key\":1}"}"#;
    let params: ReplayRequestParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.method, "POST");
    assert_eq!(params.url, "https://api.com/test");
    assert_eq!(
        params
            .headers
            .as_ref()
            .unwrap()
            .get("Content-Type")
            .unwrap(),
        "application/json"
    );
    assert!(params.body.is_some());
}

#[test]
fn test_add_rule_params_block() {
    let json = r#"{"name":"Block ads","pattern":"*ads*","action_type":"block","status_code":403}"#;
    let params: AddRuleParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.name, "Block ads");
    assert_eq!(params.pattern, "*ads*");
    assert_eq!(params.action_type, "block");
    assert_eq!(params.status_code.unwrap(), 403);
}

#[test]
fn test_add_rule_params_map_local() {
    let json = r#"{"name":"Map Local","pattern":"*api*","action_type":"map_local","file_path":"/tmp/mock.json"}"#;
    let params: AddRuleParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.action_type, "map_local");
    assert_eq!(params.file_path.unwrap(), "/tmp/mock.json");
}

#[test]
fn test_add_rule_params_map_remote() {
    let json = r#"{"name":"Map Remote","pattern":"*api*","action_type":"map_remote","target_url":"https://staging.api.com","preserve_path":false}"#;
    let params: AddRuleParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.action_type, "map_remote");
    assert_eq!(params.target_url.unwrap(), "https://staging.api.com");
    assert_eq!(params.preserve_path.unwrap(), false);
}

#[test]
fn test_remove_rule_params() {
    let json = r#"{"id":"mcp_0"}"#;
    let params: RemoveRuleParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.id, "mcp_0");
}

#[test]
fn test_intercept_rules_updated_serialization() {
    use proxy_daemon::DaemonMessage;

    let rules = vec![make_block_rule("r1")];
    let msg = DaemonMessage::InterceptRulesUpdated {
        rules: rules.clone(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["type"], "intercept_rules_updated");
    assert_eq!(parsed["rules"].as_array().unwrap().len(), 1);

    let roundtrip: DaemonMessage = serde_json::from_str(&json).unwrap();
    match roundtrip {
        DaemonMessage::InterceptRulesUpdated { rules } => {
            assert_eq!(rules.len(), 1);
            assert_eq!(rules[0].id, "r1");
        }
        _ => panic!("Expected InterceptRulesUpdated"),
    }
}

#[test]
fn test_server_creation_without_daemon() {
    let store = Store::new();
    let server = CheolsuMcpServer::new(store, None);
    assert!(server.daemon_conn.try_lock().unwrap().is_none());
}

#[tokio::test]
async fn test_server_proxy_status_no_daemon() {
    let store = Store::new();
    let server = CheolsuMcpServer::new(store, None);
    let result = server.proxy_status().await.unwrap();
    assert!(!result.is_error.unwrap_or(false));
    let text = extract_text(&result);
    assert!(text.contains("MCP connected: false"));
}

#[tokio::test]
async fn test_server_clear_traffic() {
    let store = Store::new();
    store
        .transactions
        .lock()
        .push_back(make_empty_request_info());
    store.ws_messages.lock().push_back(make_ws_message("c1"));

    let server = CheolsuMcpServer::new(store.clone(), None);
    let result = server.clear_traffic().await.unwrap();
    assert!(!result.is_error.unwrap_or(false));
    assert_eq!(store.transactions.lock().len(), 0);
    assert_eq!(store.ws_messages.lock().len(), 0);
}

#[tokio::test]
async fn test_server_search_traffic_empty() {
    let store = Store::new();
    let server = CheolsuMcpServer::new(store, None);
    let params = SearchTrafficParams {
        host: None,
        method: None,
        status: None,
        path: None,
        limit: None,
    };
    let result = server.search_traffic(Parameters(params)).await.unwrap();
    assert!(!result.is_error.unwrap_or(false));
    let text = extract_text(&result);
    assert!(text.contains("No matching"));
}

#[tokio::test]
async fn test_server_get_transaction_not_found() {
    let store = Store::new();
    let server = CheolsuMcpServer::new(store, None);
    let params = GetTransactionParams {
        id: "nonexistent".to_string(),
    };
    let result = server.get_transaction(Parameters(params)).await.unwrap();
    assert!(result.is_error.unwrap_or(false));
}

#[tokio::test]
async fn test_server_get_ws_messages_empty() {
    let store = Store::new();
    let server = CheolsuMcpServer::new(store, None);
    let params = GetWsMessagesParams {
        connection_id: None,
        limit: None,
    };
    let result = server
        .get_websocket_messages(Parameters(params))
        .await
        .unwrap();
    assert!(!result.is_error.unwrap_or(false));
    let text = extract_text(&result);
    assert!(text.contains("No WebSocket"));
}

#[tokio::test]
async fn test_server_list_rules_empty() {
    let store = Store::new();
    let server = CheolsuMcpServer::new(store, None);
    let result = server.list_rules().await.unwrap();
    assert!(!result.is_error.unwrap_or(false));
    let text = extract_text(&result);
    assert!(text.contains("No intercept rules"));
}

#[tokio::test]
async fn test_server_list_rules_with_rules() {
    let store = Store::new();
    *store.rules.lock() = vec![make_block_rule("r1"), make_block_rule("r2")];
    let server = CheolsuMcpServer::new(store, None);
    let result = server.list_rules().await.unwrap();
    let text = extract_text(&result);
    assert!(text.contains("2 rules"));
}

#[tokio::test]
async fn test_server_diff_transactions_not_found() {
    let store = Store::new();
    let server = CheolsuMcpServer::new(store, None);
    let params = DiffTransactionsParams {
        transaction_id_a: "nonexistent_a".to_string(),
        transaction_id_b: "nonexistent_b".to_string(),
    };
    let result = server.diff_transactions(Parameters(params)).await.unwrap();
    assert!(result.is_error.unwrap_or(false));
    let text = extract_text(&result);
    assert!(text.contains("not found"));
}

#[tokio::test]
async fn test_server_remove_rule_not_found() {
    let store = Store::new();
    let server = CheolsuMcpServer::new(store, None);
    let params = RemoveRuleParams {
        id: "nonexistent".to_string(),
    };
    let result = server.remove_rule(Parameters(params)).await.unwrap();
    assert!(result.is_error.unwrap_or(false));
}

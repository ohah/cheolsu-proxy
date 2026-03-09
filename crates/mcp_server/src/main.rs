mod connection;
mod helpers;
mod params;
mod store;

use std::sync::Arc;

use anyhow::Result;
use proxy_daemon::{
    is_daemon_running, ClientCommand, DaemonConnection, InterceptAction, InterceptRule,
};
use proxy_v2_models::WsDirection;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    tool, tool_handler, tool_router,
    transport::stdio,
    ErrorData as McpError, ServerHandler, ServiceExt,
};
use tokio::sync::Mutex as TokioMutex;
use tracing_subscriber::EnvFilter;

use connection::try_connect_daemon;
use helpers::{format_size, next_rule_id, read_body_text, tool_error, tool_ok};
use params::*;
use store::Store;

// ─── MCP Server ─────────────────────────────────────────────

#[derive(Clone)]
struct CheolsuMcpServer {
    store: Store,
    daemon_conn: Arc<TokioMutex<Option<DaemonConnection>>>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl CheolsuMcpServer {
    fn new(store: Store, conn: Option<DaemonConnection>) -> Self {
        Self {
            store,
            daemon_conn: Arc::new(TokioMutex::new(conn)),
            tool_router: Self::tool_router(),
        }
    }

    async fn send_rules(&self) -> Result<(), String> {
        let conn_guard = self.daemon_conn.lock().await;
        let Some(conn) = conn_guard.as_ref() else {
            return Err("Not connected to proxy daemon".to_string());
        };
        let rules = self.store.rules.lock().clone();
        conn.send_command(&ClientCommand::UpdateInterceptRules { rules })
            .await
            .map_err(|e| e.to_string())
    }

    #[tool(
        description = "Search captured HTTP traffic. Filters by host, method, status code, or URL path. Returns a summary list with transaction IDs."
    )]
    async fn search_traffic(
        &self,
        Parameters(p): Parameters<SearchTrafficParams>,
    ) -> Result<CallToolResult, McpError> {
        let txns = self.store.transactions.lock();
        let limit = p.limit.unwrap_or(50);

        let results: Vec<String> = txns
            .iter()
            .rev()
            .filter(|info| {
                let Some(req) = &info.0 else { return false };
                let uri = req.uri().to_string();

                if let Some(ref host) = p.host {
                    if !uri.to_lowercase().contains(&host.to_lowercase()) {
                        return false;
                    }
                }
                if let Some(ref method) = p.method {
                    if !req.method().as_str().eq_ignore_ascii_case(method) {
                        return false;
                    }
                }
                if let Some(status) = p.status {
                    match &info.1 {
                        Some(res) if res.status().as_u16() == status => {}
                        _ => return false,
                    }
                }
                if let Some(ref path) = p.path {
                    if !uri.to_lowercase().contains(&path.to_lowercase()) {
                        return false;
                    }
                }
                true
            })
            .take(limit)
            .map(|info| {
                let req = info.0.as_ref().unwrap();
                let status = info
                    .1
                    .as_ref()
                    .map(|r| r.status().as_u16().to_string())
                    .unwrap_or_else(|| "pending".to_string());
                let size = info
                    .1
                    .as_ref()
                    .map(|r| format_size(r.body_size()))
                    .unwrap_or_default();
                let dtype = info
                    .1
                    .as_ref()
                    .map(|r| format!("{:?}", r.data_type()))
                    .unwrap_or_default();
                format!(
                    "[{}] {} {} → {} ({}) {}",
                    req.id(),
                    req.method(),
                    req.uri(),
                    status,
                    size,
                    dtype,
                )
            })
            .collect();

        if results.is_empty() {
            tool_ok("No matching transactions found.")
        } else {
            tool_ok(format!(
                "Found {} transactions (most recent first):\n\n{}",
                results.len(),
                results.join("\n")
            ))
        }
    }

    #[tool(
        description = "Get full details of a specific HTTP transaction including request/response headers and body."
    )]
    async fn get_transaction(
        &self,
        Parameters(p): Parameters<GetTransactionParams>,
    ) -> Result<CallToolResult, McpError> {
        let txns = self.store.transactions.lock();
        let info = txns
            .iter()
            .find(|info| info.0.as_ref().map(|r| r.id() == p.id).unwrap_or(false));

        let Some(info) = info else {
            return tool_error(format!("Transaction '{}' not found.", p.id));
        };

        let mut out = String::new();

        if let Some(req) = &info.0 {
            out.push_str(&format!(
                "## Request\n{} {} {:?}\n\n",
                req.method(),
                req.uri(),
                req.version()
            ));
            out.push_str("### Headers\n```\n");
            for (name, value) in req.headers().iter() {
                out.push_str(&format!(
                    "{}: {}\n",
                    name,
                    value.to_str().unwrap_or("<binary>")
                ));
            }
            out.push_str("```\n\n");
            out.push_str(&format!(
                "### Body ({}, {:?})\n",
                format_size(req.body_size()),
                req.data_type()
            ));
            if req.body_size() == 0 {
                out.push_str("(empty)\n");
            } else {
                let body = read_body_text(req.file_path(), req.data_type());
                out.push_str(&format!("```\n{}\n```\n", body));
            }
        }

        if let Some(res) = &info.1 {
            out.push_str(&format!(
                "\n## Response\n{} {:?}\n\n",
                res.status().as_u16(),
                res.version()
            ));
            out.push_str("### Headers\n```\n");
            for (name, value) in res.headers().iter() {
                out.push_str(&format!(
                    "{}: {}\n",
                    name,
                    value.to_str().unwrap_or("<binary>")
                ));
            }
            out.push_str("```\n\n");
            out.push_str(&format!(
                "### Body ({}, {:?})\n",
                format_size(res.body_size()),
                res.data_type()
            ));
            if res.body_size() == 0 {
                out.push_str("(empty)\n");
            } else {
                let body = read_body_text(res.file_path(), res.data_type());
                out.push_str(&format!("```\n{}\n```\n", body));
            }
        }

        tool_ok(out)
    }

    #[tool(description = "Get captured WebSocket messages, optionally filtered by connection URI.")]
    async fn get_websocket_messages(
        &self,
        Parameters(p): Parameters<GetWsMessagesParams>,
    ) -> Result<CallToolResult, McpError> {
        let msgs = self.store.ws_messages.lock();
        let limit = p.limit.unwrap_or(100);

        let results: Vec<String> = msgs
            .iter()
            .rev()
            .filter(|msg| {
                p.connection_id.as_ref().map_or(true, |cid| {
                    msg.connection_id
                        .to_lowercase()
                        .contains(&cid.to_lowercase())
                })
            })
            .take(limit)
            .map(|msg| {
                let dir = match msg.direction {
                    WsDirection::ClientToServer => "→",
                    WsDirection::ServerToClient => "←",
                };
                let payload = if msg.payload.len() > 200 {
                    format!("{}...", &msg.payload[..200])
                } else {
                    msg.payload.clone()
                };
                format!(
                    "{} {:?} ({} bytes) [{}]: {}",
                    dir, msg.message_type, msg.size, msg.connection_id, payload,
                )
            })
            .collect();

        if results.is_empty() {
            tool_ok("No WebSocket messages captured.")
        } else {
            tool_ok(format!(
                "Found {} messages:\n\n{}",
                results.len(),
                results.join("\n\n")
            ))
        }
    }

    #[tool(
        description = "Send an HTTP request directly (bypassing the proxy). Useful for testing and replaying captured requests."
    )]
    async fn replay_request(
        &self,
        Parameters(p): Parameters<ReplayRequestParams>,
    ) -> Result<CallToolResult, McpError> {
        let client = match reqwest::Client::builder()
            .no_proxy()
            .danger_accept_invalid_certs(true)
            .build()
        {
            Ok(c) => c,
            Err(e) => return tool_error(format!("Failed to create HTTP client: {}", e)),
        };

        let method: reqwest::Method = match p.method.parse() {
            Ok(m) => m,
            Err(e) => return tool_error(format!("Invalid HTTP method: {}", e)),
        };

        let mut builder = client.request(method, &p.url);
        if let Some(headers) = p.headers {
            for (k, v) in headers {
                builder = builder.header(k, v);
            }
        }
        if let Some(body) = p.body {
            builder = builder.body(body);
        }

        let start = std::time::Instant::now();
        let response = match builder.send().await {
            Ok(r) => r,
            Err(e) => return tool_error(format!("Request failed: {}", e)),
        };
        let elapsed = start.elapsed();

        let status = response.status().as_u16();
        let headers: Vec<String> = response
            .headers()
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v.to_str().unwrap_or("<binary>")))
            .collect();
        let body_bytes = match response.bytes().await {
            Ok(b) => b,
            Err(e) => return tool_error(format!("Failed to read response body: {}", e)),
        };

        let body_text = String::from_utf8(body_bytes.to_vec())
            .unwrap_or_else(|_| format!("<binary, {} bytes>", body_bytes.len()));
        let body_display = if body_text.len() > 10000 {
            format!(
                "{}...\n(truncated, {} total)",
                &body_text[..10000],
                format_size(body_text.len())
            )
        } else {
            body_text
        };

        tool_ok(format!(
            "## Response\nStatus: {}\nTime: {:.0?}\nSize: {}\n\n### Headers\n```\n{}\n```\n\n### Body\n```\n{}\n```",
            status,
            elapsed,
            format_size(body_bytes.len()),
            headers.join("\n"),
            body_display,
        ))
    }

    #[tool(description = "List all current intercept rules (block, modify, map local/remote).")]
    async fn list_rules(&self) -> Result<CallToolResult, McpError> {
        let rules = self.store.rules.lock();
        if rules.is_empty() {
            return tool_ok("No intercept rules configured.");
        }
        let list: Vec<String> = rules.iter().map(|r| format!("  {}", r)).collect();
        tool_ok(format!("{} rules:\n\n{}", rules.len(), list.join("\n")))
    }

    #[tool(
        description = "Add a new intercept rule. Supports: block, modify_request, modify_response, map_local, map_remote."
    )]
    async fn add_rule(
        &self,
        Parameters(p): Parameters<AddRuleParams>,
    ) -> Result<CallToolResult, McpError> {
        let action = match p.action_type.as_str() {
            "block" => InterceptAction::Block {
                status_code: p.status_code.unwrap_or(403),
                body: p.response_body.unwrap_or_default(),
            },
            "modify_request" => InterceptAction::ModifyRequest {
                add_headers: p.add_headers.unwrap_or_default(),
                remove_headers: p.remove_headers.unwrap_or_default(),
                set_body: p.response_body,
            },
            "modify_response" => InterceptAction::ModifyResponse {
                set_status: p.status_code,
                add_headers: p.add_headers.unwrap_or_default(),
                remove_headers: p.remove_headers.unwrap_or_default(),
                set_body: p.response_body,
            },
            "map_local" => {
                let Some(file_path) = p.file_path else {
                    return tool_error("file_path is required for map_local");
                };
                InterceptAction::MapLocal {
                    file_path,
                    status_code: p.status_code.unwrap_or(200),
                    headers: p.add_headers.unwrap_or_default(),
                }
            }
            "map_remote" => {
                let Some(target_url) = p.target_url else {
                    return tool_error("target_url is required for map_remote");
                };
                InterceptAction::MapRemote {
                    target_url,
                    preserve_path: p.preserve_path.unwrap_or(true),
                }
            }
            other => {
                return tool_error(format!(
                    "Unknown action_type '{}'. Use: block, modify_request, modify_response, map_local, map_remote",
                    other
                ));
            }
        };

        let id = next_rule_id();
        let rule = InterceptRule {
            id: id.clone(),
            name: p.name,
            enabled: true,
            pattern: p.pattern,
            method: p.method,
            action,
        };

        self.store.rules.lock().push(rule);

        match self.send_rules().await {
            Ok(()) => tool_ok(format!("Rule '{}' added successfully.", id)),
            Err(e) => tool_error(format!(
                "Rule added locally but failed to sync with daemon: {}",
                e
            )),
        }
    }

    #[tool(description = "Remove an intercept rule by its ID.")]
    async fn remove_rule(
        &self,
        Parameters(p): Parameters<RemoveRuleParams>,
    ) -> Result<CallToolResult, McpError> {
        let removed = {
            let mut rules = self.store.rules.lock();
            let before = rules.len();
            rules.retain(|r| r.id != p.id);
            rules.len() < before
        };

        if !removed {
            return tool_error(format!("Rule '{}' not found.", p.id));
        }

        match self.send_rules().await {
            Ok(()) => tool_ok(format!("Rule '{}' removed.", p.id)),
            Err(e) => tool_error(format!(
                "Rule removed locally but failed to sync with daemon: {}",
                e
            )),
        }
    }

    #[tool(
        description = "Load a JavaScript/TypeScript script to intercept and modify proxy traffic. Provide either a file path or inline code. The script can use cheolsu.onRequest(), cheolsu.onResponse(), cheolsu.onWebSocketMessage() hooks."
    )]
    async fn load_script(
        &self,
        Parameters(p): Parameters<LoadScriptParams>,
    ) -> Result<CallToolResult, McpError> {
        if p.path.is_none() && p.code.is_none() {
            return tool_error("Either 'path' or 'code' must be provided.");
        }
        let conn_guard = self.daemon_conn.lock().await;
        let Some(conn) = conn_guard.as_ref() else {
            return tool_error("Not connected to proxy daemon.");
        };
        let cmd = ClientCommand::LoadScript {
            path: p.path.clone(),
            code: p.code.clone(),
        };
        match conn.send_command(&cmd).await {
            Ok(()) => {
                let source = if let Some(ref path) = p.path {
                    format!("file '{}'", path)
                } else {
                    "inline code".to_string()
                };
                tool_ok(format!("Script loaded from {}.", source))
            }
            Err(e) => tool_error(format!("Failed to load script: {}", e)),
        }
    }

    #[tool(description = "Unload the currently loaded proxy script.")]
    async fn unload_script(
        &self,
        #[allow(unused_variables)] Parameters(_p): Parameters<UnloadScriptParams>,
    ) -> Result<CallToolResult, McpError> {
        let conn_guard = self.daemon_conn.lock().await;
        let Some(conn) = conn_guard.as_ref() else {
            return tool_error("Not connected to proxy daemon.");
        };
        match conn.send_command(&ClientCommand::UnloadScript).await {
            Ok(()) => tool_ok("Script unloaded."),
            Err(e) => tool_error(format!("Failed to unload script: {}", e)),
        }
    }

    #[tool(description = "Check proxy daemon status and traffic statistics.")]
    async fn proxy_status(&self) -> Result<CallToolResult, McpError> {
        let connected = self.daemon_conn.lock().await.is_some();
        let txn_count = self.store.transactions.lock().len();
        let ws_msg_count = self.store.ws_messages.lock().len();
        let ws_conn_count = self.store.ws_connections.lock().len();
        let rule_count = self.store.rules.lock().len();
        let daemon_running = is_daemon_running().is_some();

        tool_ok(format!(
            "Daemon running: {}\nMCP connected: {}\nCaptured transactions: {}\nWebSocket connections: {}\nWebSocket messages: {}\nIntercept rules: {}",
            daemon_running, connected, txn_count, ws_conn_count, ws_msg_count, rule_count,
        ))
    }

    #[tool(description = "Clear all captured traffic data from memory.")]
    async fn clear_traffic(&self) -> Result<CallToolResult, McpError> {
        self.store.transactions.lock().clear();
        self.store.ws_messages.lock().clear();
        self.store.ws_connections.lock().clear();
        tool_ok("All captured traffic cleared.")
    }
}

#[tool_handler]
impl ServerHandler for CheolsuMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::from_build_env())
            .with_instructions(
                "Cheolsu Proxy MCP Server — search/inspect captured HTTP & WebSocket traffic, replay requests, and manage intercept rules. Start the Cheolsu Proxy app first.".to_string(),
            )
    }
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use proxy_daemon::{InterceptAction, InterceptRule};
    use proxy_v2_models::{RequestInfo, WsConnectionEvent, WsDirection, WsMessageInfo};
    use store::{MAX_TRANSACTIONS, MAX_WS_MESSAGES};

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

    // ─── Store tests ────────────────────────────────────────

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

    // ─── broadcast 동기화 시뮬레이션 테스트 ─────────────────

    #[test]
    fn test_broadcast_sync_preserves_app_rules_on_mcp_add() {
        let store = Store::new();
        *store.rules.lock() = vec![make_block_rule("uuid-1"), make_block_rule("uuid-2")];
        store.rules.lock().push(make_block_rule("mcp_0"));
        let rules = store.rules.lock().clone();
        assert_eq!(rules.len(), 3);
        assert!(rules.iter().any(|r| r.id == "uuid-1"));
        assert!(rules.iter().any(|r| r.id == "uuid-2"));
        assert!(rules.iter().any(|r| r.id == "mcp_0"));
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
        let rules = store.rules.lock().clone();
        assert_eq!(rules.len(), 3);
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
        let rules = store.rules.lock().clone();
        assert_eq!(rules.len(), 2);
        assert!(rules.iter().any(|r| r.id == "uuid-1"));
        assert!(rules.iter().any(|r| r.id == "mcp_0"));
    }

    #[test]
    fn test_broadcast_initial_empty_then_sync() {
        let store = Store::new();
        assert_eq!(store.rules.lock().len(), 0);
        *store.rules.lock() = vec![make_block_rule("uuid-1"), make_block_rule("uuid-2")];
        store.rules.lock().push(make_block_rule("mcp_0"));
        let rules = store.rules.lock().clone();
        assert_eq!(rules.len(), 3);
    }

    // ─── Helper function tests ──────────────────────────────

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

    // ─── Tool error/ok helpers ──────────────────────────────

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

    // ─── Parameter deserialization tests ─────────────────────

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
        let json =
            r#"{"name":"Block ads","pattern":"*ads*","action_type":"block","status_code":403}"#;
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

    // ─── InterceptRulesUpdated broadcast test ───────────────

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

    // ─── MCP Server creation test ───────────────────────────

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
    async fn test_server_remove_rule_not_found() {
        let store = Store::new();
        let server = CheolsuMcpServer::new(store, None);
        let params = RemoveRuleParams {
            id: "nonexistent".to_string(),
        };
        let result = server.remove_rule(Parameters(params)).await.unwrap();
        assert!(result.is_error.unwrap_or(false));
    }
}

// ─── Main ───────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting Cheolsu Proxy MCP server");

    let store = Store::new();
    let conn = try_connect_daemon(&store).await;
    let server = CheolsuMcpServer::new(store, conn);

    let service = server.serve(stdio()).await.inspect_err(|e| {
        tracing::error!("Server error: {:?}", e);
    })?;

    service.waiting().await?;
    Ok(())
}

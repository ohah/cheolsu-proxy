mod connection;
mod helpers;
mod params;
mod store;

use std::sync::Arc;

use anyhow::Result;
use proxy_daemon::{
    diff_headers, diff_json, diff_text, format_diff_text, is_daemon_running, BodyDiff,
    BreakpointAction, BreakpointRule, ClientCommand, DaemonConnection, HostMapping,
    InterceptAction, InterceptRule, SessionFile, TrafficDiff, TransactionPartDiff,
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
use helpers::{
    format_size, next_breakpoint_id, next_mapping_id, next_rule_id, read_body_text, tool_error,
    tool_ok,
};
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

    async fn send_host_mappings(&self) -> Result<(), String> {
        let conn_guard = self.daemon_conn.lock().await;
        let Some(conn) = conn_guard.as_ref() else {
            return Err("Not connected to proxy daemon".to_string());
        };
        let mappings = self.store.host_mappings.lock().clone();
        conn.send_command(&ClientCommand::UpdateHostMappings { mappings })
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

        // RequestInfo is a tuple struct: RequestInfo(Option<ClientRequest>, Option<ClientResponse>)
        // Access via .0 (request) and .1 (response) as no named accessor methods are defined.
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

    #[tool(
        description = "Compare two captured HTTP transactions (request + response) and show differences. Useful for regression testing and comparing API responses before/after deployment."
    )]
    async fn diff_transactions(
        &self,
        Parameters(p): Parameters<DiffTransactionsParams>,
    ) -> Result<CallToolResult, McpError> {
        let txns = self.store.transactions.lock();

        let txn_a = txns.iter().find(|info| {
            info.0
                .as_ref()
                .map(|r| r.id() == p.transaction_id_a)
                .unwrap_or(false)
        });
        let txn_b = txns.iter().find(|info| {
            info.0
                .as_ref()
                .map(|r| r.id() == p.transaction_id_b)
                .unwrap_or(false)
        });

        let Some(txn_a) = txn_a else {
            return tool_error(format!("Transaction '{}' not found.", p.transaction_id_a));
        };
        let Some(txn_b) = txn_b else {
            return tool_error(format!("Transaction '{}' not found.", p.transaction_id_b));
        };

        let request_diff = match (&txn_a.0, &txn_b.0) {
            (Some(req_a), Some(req_b)) => {
                let method_diff = if req_a.method() != req_b.method() {
                    Some((req_a.method().to_string(), req_b.method().to_string()))
                } else {
                    None
                };

                let url_diff = if req_a.uri().to_string() != req_b.uri().to_string() {
                    Some((req_a.uri().to_string(), req_b.uri().to_string()))
                } else {
                    None
                };

                let (header_diffs, body_diff) = diff_part(
                    req_a.headers(),
                    req_b.headers(),
                    req_a.body().map(|b| b.as_ref()),
                    req_b.body().map(|b| b.as_ref()),
                    req_a.body_size(),
                    req_b.body_size(),
                    req_a.file_path(),
                    req_b.file_path(),
                    req_a.data_type(),
                    req_b.data_type(),
                );

                if method_diff.is_none()
                    && url_diff.is_none()
                    && header_diffs.is_empty()
                    && body_diff.is_none()
                {
                    None
                } else {
                    Some(TransactionPartDiff {
                        method_diff,
                        url_diff,
                        status_diff: None,
                        header_diffs,
                        body_diff,
                    })
                }
            }
            _ => None,
        };

        let response_diff = match (&txn_a.1, &txn_b.1) {
            (Some(res_a), Some(res_b)) => {
                let status_diff = if res_a.status() != res_b.status() {
                    Some((res_a.status().as_u16(), res_b.status().as_u16()))
                } else {
                    None
                };

                let (header_diffs, body_diff) = diff_part(
                    res_a.headers(),
                    res_b.headers(),
                    res_a.body().map(|b| b.as_ref()),
                    res_b.body().map(|b| b.as_ref()),
                    res_a.body_size(),
                    res_b.body_size(),
                    res_a.file_path(),
                    res_b.file_path(),
                    res_a.data_type(),
                    res_b.data_type(),
                );

                if status_diff.is_none() && header_diffs.is_empty() && body_diff.is_none() {
                    None
                } else {
                    Some(TransactionPartDiff {
                        method_diff: None,
                        url_diff: None,
                        status_diff,
                        header_diffs,
                        body_diff,
                    })
                }
            }
            _ => None,
        };

        let diff = TrafficDiff {
            request_diff,
            response_diff,
        };

        tool_ok(format_diff_text(&diff))
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

    async fn send_breakpoint_rules(&self) -> Result<(), String> {
        let conn_guard = self.daemon_conn.lock().await;
        let Some(conn) = conn_guard.as_ref() else {
            return Err("Not connected to proxy daemon".to_string());
        };
        let rules = self.store.breakpoint_rules.lock().clone();
        conn.send_command(&ClientCommand::UpdateBreakpointRules { rules })
            .await
            .map_err(|e| e.to_string())
    }

    #[tool(
        description = "List all current breakpoint rules. Breakpoints pause matching requests/responses for manual inspection and editing."
    )]
    async fn list_breakpoints(&self) -> Result<CallToolResult, McpError> {
        let rules = self.store.breakpoint_rules.lock();
        if rules.is_empty() {
            return tool_ok("No breakpoint rules configured.");
        }
        let list: Vec<String> = rules.iter().map(|r| format!("  {}", r)).collect();
        tool_ok(format!(
            "{} breakpoint rules:\n\n{}",
            rules.len(),
            list.join("\n")
        ))
    }

    #[tool(
        description = "List all host mappings (DNS spoofing / remote host mapping rules). Maps source hosts to target hosts/IPs for testing without modifying hosts file."
    )]
    async fn list_host_mappings(&self) -> Result<CallToolResult, McpError> {
        let mappings = self.store.host_mappings.lock();
        if mappings.is_empty() {
            return tool_ok("No host mappings configured.");
        }
        let list: Vec<String> = mappings.iter().map(|m| format!("  {}", m)).collect();
        tool_ok(format!(
            "{} host mappings:\n\n{}",
            mappings.len(),
            list.join("\n")
        ))
    }

    #[tool(
        description = "Add a breakpoint rule. When a matching request or response is intercepted, it will be paused for manual editing. Use list_pending_breakpoints to see paused items and resolve_breakpoint to continue."
    )]
    async fn add_breakpoint(
        &self,
        Parameters(p): Parameters<AddBreakpointParams>,
    ) -> Result<CallToolResult, McpError> {
        let id = next_breakpoint_id();
        let rule = BreakpointRule {
            id: id.clone(),
            pattern: p.pattern,
            break_on_request: p.break_on_request.unwrap_or(true),
            break_on_response: p.break_on_response.unwrap_or(false),
            enabled: true,
        };

        self.store.breakpoint_rules.lock().push(rule);

        match self.send_breakpoint_rules().await {
            Ok(()) => tool_ok(format!("Breakpoint '{}' added successfully.", id)),
            Err(e) => tool_error(format!(
                "Breakpoint added locally but failed to sync with daemon: {}",
                e
            )),
        }
    }

    #[tool(description = "Remove a breakpoint rule by its ID.")]
    async fn remove_breakpoint(
        &self,
        Parameters(p): Parameters<RemoveBreakpointParams>,
    ) -> Result<CallToolResult, McpError> {
        let removed = {
            let mut rules = self.store.breakpoint_rules.lock();
            let before = rules.len();
            rules.retain(|r| r.id != p.id);
            rules.len() < before
        };

        if !removed {
            return tool_error(format!("Breakpoint '{}' not found.", p.id));
        }

        match self.send_breakpoint_rules().await {
            Ok(()) => tool_ok(format!("Breakpoint '{}' removed.", p.id)),
            Err(e) => tool_error(format!(
                "Breakpoint removed locally but failed to sync with daemon: {}",
                e
            )),
        }
    }

    #[tool(
        description = "Add a host mapping rule (DNS spoofing). Maps requests for a source host to a different target host/IP. Supports wildcard patterns (e.g., *.api.example.com). The original Host header is preserved so the target server routes to the correct virtual host."
    )]
    async fn add_host_mapping(
        &self,
        Parameters(p): Parameters<AddHostMappingParams>,
    ) -> Result<CallToolResult, McpError> {
        let id = next_mapping_id();
        let mapping = HostMapping {
            id: id.clone(),
            source_host: p.source_host,
            source_port: p.source_port,
            target_host: p.target_host,
            target_port: p.target_port,
            enabled: true,
        };

        self.store.host_mappings.lock().push(mapping);

        match self.send_host_mappings().await {
            Ok(()) => tool_ok(format!("Host mapping '{}' added successfully.", id)),
            Err(e) => tool_error(format!(
                "Host mapping added locally but failed to sync with daemon: {}",
                e
            )),
        }
    }

    #[tool(description = "Remove a host mapping rule by its ID.")]
    async fn remove_host_mapping(
        &self,
        Parameters(p): Parameters<RemoveHostMappingParams>,
    ) -> Result<CallToolResult, McpError> {
        let removed = {
            let mut mappings = self.store.host_mappings.lock();
            let before = mappings.len();
            mappings.retain(|m| m.id != p.id);
            mappings.len() < before
        };

        if !removed {
            return tool_error(format!("Host mapping '{}' not found.", p.id));
        }

        match self.send_host_mappings().await {
            Ok(()) => tool_ok(format!("Host mapping '{}' removed.", p.id)),
            Err(e) => tool_error(format!(
                "Host mapping removed locally but failed to sync with daemon: {}",
                e
            )),
        }
    }

    #[tool(
        description = "List currently paused (pending) breakpoints waiting for resolution. Returns breakpoint IDs that can be used with resolve_breakpoint."
    )]
    async fn list_pending_breakpoints(&self) -> Result<CallToolResult, McpError> {
        // Pending breakpoints are tracked in the daemon; we read from events.
        // For now, list what we know from recent BreakpointHit events.
        tool_ok(
            "Pending breakpoints are shown as 'breakpoint_hit' events in the daemon stream. \
             Use the breakpoint ID from those events with resolve_breakpoint to continue."
                .to_string(),
        )
    }

    #[tool(
        description = "Resolve a pending breakpoint. Choose an action: 'forward' (pass through as-is), 'modify_and_forward' (edit headers/body/status then forward), 'drop' (discard), or 'abort' (return error)."
    )]
    async fn resolve_breakpoint(
        &self,
        Parameters(p): Parameters<ResolveBreakpointParams>,
    ) -> Result<CallToolResult, McpError> {
        let action = match p.action.as_str() {
            "forward" => BreakpointAction::Forward,
            "modify_and_forward" => BreakpointAction::ModifyAndForward {
                headers: p.headers,
                body: p.body,
                status: p.status,
            },
            "drop" => BreakpointAction::Drop,
            "abort" => BreakpointAction::Abort,
            other => {
                return tool_error(format!(
                    "Unknown action '{}'. Use: forward, modify_and_forward, drop, abort",
                    other
                ));
            }
        };

        let conn_guard = self.daemon_conn.lock().await;
        let Some(conn) = conn_guard.as_ref() else {
            return tool_error("Not connected to proxy daemon.");
        };
        let cmd = ClientCommand::ResolveBreakpoint {
            id: p.id.clone(),
            action,
        };
        match conn.send_command(&cmd).await {
            Ok(()) => tool_ok(format!("Breakpoint '{}' resolved.", p.id)),
            Err(e) => tool_error(format!("Failed to resolve breakpoint: {}", e)),
        }
    }

    #[tool(
        description = "Save captured traffic to a .cheolsu session file. Use .cheolsu.gz extension for gzip compression. Optionally filter by URL substring."
    )]
    async fn save_session(
        &self,
        Parameters(p): Parameters<SaveSessionParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = proxy_daemon::ensure_extension(&p.path);

        let transactions: Vec<proxy_v2_models::RequestInfo> = {
            let txns = self.store.transactions.lock();
            if let Some(ref filter) = p.filter {
                let filter_lower = filter.to_lowercase();
                txns.iter()
                    .filter(|info| {
                        info.0
                            .as_ref()
                            .map(|req| req.uri().to_string().to_lowercase().contains(&filter_lower))
                            .unwrap_or(false)
                    })
                    .cloned()
                    .collect()
            } else {
                txns.iter().cloned().collect()
            }
        };

        let ws_messages: Vec<proxy_v2_models::WsMessageInfo> =
            self.store.ws_messages.lock().iter().cloned().collect();
        let rules: Vec<InterceptRule> = self.store.rules.lock().clone();

        let mut session = SessionFile::from_traffic(
            0, // port unknown from MCP server
            &transactions,
            &ws_messages,
            &rules,
            &[],
            None,
        );

        if let Some(name) = p.name {
            session.metadata.name = Some(name);
        }
        if let Some(desc) = p.description {
            session.metadata.description = Some(desc);
        }

        let file_path = std::path::Path::new(&path);
        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    return tool_error(format!("Failed to create directory: {}", e));
                }
            }
        }

        match session.save(file_path) {
            Ok(()) => tool_ok(format!(
                "Session saved to '{}' ({} transactions, {} WebSocket messages).",
                path,
                transactions.len(),
                ws_messages.len(),
            )),
            Err(e) => tool_error(format!("Failed to save session: {}", e)),
        }
    }

    #[tool(
        description = "Load a session file (.cheolsu, .cheolsu.gz) or import a HAR file (.har) into the traffic viewer. By default replaces current traffic; set append=true to add to existing."
    )]
    async fn load_session(
        &self,
        Parameters(p): Parameters<LoadSessionParams>,
    ) -> Result<CallToolResult, McpError> {
        let file_path = std::path::Path::new(&p.path);
        if !file_path.exists() {
            return tool_error(format!("File not found: {}", p.path));
        }

        let is_har = p.path.to_lowercase().ends_with(".har");

        let (transactions, ws_messages, rules) = if is_har {
            match proxy_daemon::import_har_file(file_path) {
                Ok(txns) => (txns, Vec::new(), Vec::new()),
                Err(e) => return tool_error(format!("Failed to import HAR file: {}", e)),
            }
        } else {
            match SessionFile::load(file_path) {
                Ok(session) => {
                    let txns = session.extract_transactions();
                    let ws = session.websocket_messages;
                    let rules = session.intercept_rules;
                    (txns, ws, rules)
                }
                Err(e) => return tool_error(format!("Failed to load session: {}", e)),
            }
        };

        let txn_count = transactions.len();
        let ws_count = ws_messages.len();
        let rule_count = rules.len();

        if !p.append {
            self.store.transactions.lock().clear();
            self.store.ws_messages.lock().clear();
            self.store.ws_connections.lock().clear();
        }

        for txn in transactions {
            self.store.push_transaction(txn);
        }
        for msg in ws_messages {
            self.store.push_ws_message(msg);
        }

        if !rules.is_empty() {
            let mut current_rules = self.store.rules.lock();
            for rule in rules {
                if !current_rules.iter().any(|r| r.id == rule.id) {
                    current_rules.push(rule);
                }
            }
        }

        let mode = if p.append { "appended" } else { "loaded" };
        let format_name = if is_har { "HAR" } else { "session" };

        tool_ok(format!(
            "{} {} from '{}': {} transactions, {} WebSocket messages, {} rules.",
            format_name, mode, p.path, txn_count, ws_count, rule_count,
        ))
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

/// Compute header diffs and body diff for a transaction part.
fn diff_part(
    headers_a: &http::HeaderMap,
    headers_b: &http::HeaderMap,
    body_a: Option<&[u8]>,
    body_b: Option<&[u8]>,
    size_a: usize,
    size_b: usize,
    file_path_a: &Option<String>,
    file_path_b: &Option<String>,
    data_type_a: &proxy_v2_models::DataType,
    data_type_b: &proxy_v2_models::DataType,
) -> (Vec<proxy_daemon::HeaderDiff>, Option<BodyDiff>) {
    let extract = |h: &http::HeaderMap| -> Vec<(String, String)> {
        h.iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("<binary>").to_string()))
            .collect()
    };
    let header_diffs = diff_headers(&extract(headers_a), &extract(headers_b));
    let body_diff = compute_body_diff(
        body_a,
        body_b,
        size_a,
        size_b,
        file_path_a,
        file_path_b,
        data_type_a,
        data_type_b,
    );
    (header_diffs, body_diff)
}

fn compute_body_diff(
    body_a: Option<&[u8]>,
    body_b: Option<&[u8]>,
    size_a: usize,
    size_b: usize,
    file_path_a: &Option<String>,
    file_path_b: &Option<String>,
    data_type_a: &proxy_v2_models::DataType,
    data_type_b: &proxy_v2_models::DataType,
) -> Option<BodyDiff> {
    let bytes_a = body_a
        .map(|b| b.to_vec())
        .or_else(|| {
            file_path_a.as_ref().and_then(|p| {
                std::fs::read(p)
                    .map_err(|e| {
                        tracing::warn!("Failed to read body file {}: {}", p, e);
                        e
                    })
                    .ok()
            })
        })
        .unwrap_or_default();
    let bytes_b = body_b
        .map(|b| b.to_vec())
        .or_else(|| {
            file_path_b.as_ref().and_then(|p| {
                std::fs::read(p)
                    .map_err(|e| {
                        tracing::warn!("Failed to read body file {}: {}", p, e);
                        e
                    })
                    .ok()
            })
        })
        .unwrap_or_default();

    if bytes_a == bytes_b {
        return None;
    }

    let is_json = matches!(
        data_type_a,
        proxy_v2_models::DataType::Json | proxy_v2_models::DataType::GraphQL
    ) && matches!(
        data_type_b,
        proxy_v2_models::DataType::Json | proxy_v2_models::DataType::GraphQL
    );

    if is_json {
        if let (Ok(text_a), Ok(text_b)) =
            (std::str::from_utf8(&bytes_a), std::str::from_utf8(&bytes_b))
        {
            if let (Ok(json_a), Ok(json_b)) = (
                serde_json::from_str::<serde_json::Value>(text_a),
                serde_json::from_str::<serde_json::Value>(text_b),
            ) {
                return Some(diff_json(&json_a, &json_b));
            }
        }
    }

    let is_text = data_type_a.is_text_based() && data_type_b.is_text_based();
    if is_text {
        if let (Ok(text_a), Ok(text_b)) =
            (std::str::from_utf8(&bytes_a), std::str::from_utf8(&bytes_b))
        {
            return Some(diff_text(text_a, text_b));
        }
    }

    Some(BodyDiff::Binary {
        old_size: size_a,
        new_size: size_b,
    })
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

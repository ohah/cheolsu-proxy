use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use anyhow::Result;
use proxy_daemon::{
    connect_to_daemon, is_daemon_running, ClientCommand, DaemonConnection, DaemonMessage,
    InterceptAction, InterceptRule,
};
use proxy_v2_models::{RequestInfo, WsConnectionEvent, WsDirection, WsMessageInfo};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
    ErrorData as McpError, ServerHandler, ServiceExt,
};
use serde::Deserialize;
use tokio::sync::Mutex as TokioMutex;
use tracing_subscriber::EnvFilter;

const MAX_TRANSACTIONS: usize = 1000;
const MAX_WS_MESSAGES: usize = 5000;

// ─── Store ──────────────────────────────────────────────────

#[derive(Clone)]
struct Store {
    transactions: Arc<std::sync::Mutex<VecDeque<RequestInfo>>>,
    ws_messages: Arc<std::sync::Mutex<VecDeque<WsMessageInfo>>>,
    ws_connections: Arc<std::sync::Mutex<Vec<WsConnectionEvent>>>,
    rules: Arc<std::sync::Mutex<Vec<InterceptRule>>>,
}

impl Store {
    fn new() -> Self {
        Self {
            transactions: Arc::new(std::sync::Mutex::new(VecDeque::with_capacity(
                MAX_TRANSACTIONS,
            ))),
            ws_messages: Arc::new(std::sync::Mutex::new(VecDeque::with_capacity(
                MAX_WS_MESSAGES,
            ))),
            ws_connections: Arc::new(std::sync::Mutex::new(Vec::new())),
            rules: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    fn push_transaction(&self, info: RequestInfo) {
        let mut txns = self.transactions.lock().unwrap();
        if txns.len() >= MAX_TRANSACTIONS {
            txns.pop_front();
        }
        txns.push_back(info);
    }

    fn push_ws_message(&self, msg: WsMessageInfo) {
        let mut msgs = self.ws_messages.lock().unwrap();
        if msgs.len() >= MAX_WS_MESSAGES {
            msgs.pop_front();
        }
        msgs.push_back(msg);
    }

    fn push_ws_connection(&self, event: WsConnectionEvent) {
        self.ws_connections.lock().unwrap().push(event);
    }
}

// ─── Tool Parameters ────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SearchTrafficParams {
    /// Filter by hostname or URL substring
    host: Option<String>,
    /// Filter by HTTP method (GET, POST, etc.)
    method: Option<String>,
    /// Filter by response status code
    status: Option<u16>,
    /// Filter by URL path substring
    path: Option<String>,
    /// Maximum results to return (default: 50)
    limit: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GetTransactionParams {
    /// Transaction ID (from search_traffic results)
    id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GetWsMessagesParams {
    /// Filter by connection URI substring
    connection_id: Option<String>,
    /// Maximum results (default: 100)
    limit: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ReplayRequestParams {
    /// HTTP method (GET, POST, PUT, DELETE, etc.)
    method: String,
    /// Full URL to send the request to
    url: String,
    /// Request headers as key-value pairs
    headers: Option<HashMap<String, String>>,
    /// Request body content
    body: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct AddRuleParams {
    /// Rule display name
    name: String,
    /// URL pattern with wildcards (e.g., *.example.com/api/*)
    pattern: String,
    /// HTTP method filter (optional)
    method: Option<String>,
    /// Action: "block", "modify_request", "modify_response", "map_local", "map_remote"
    action_type: String,
    /// Status code (for block: default 403, for modify_response)
    status_code: Option<u16>,
    /// Body content (for block/modify_request/modify_response)
    response_body: Option<String>,
    /// Headers to add
    add_headers: Option<HashMap<String, String>>,
    /// Header names to remove
    remove_headers: Option<Vec<String>>,
    /// Local file path (required for map_local)
    file_path: Option<String>,
    /// Target URL (required for map_remote)
    target_url: Option<String>,
    /// Preserve original path for map_remote (default: true)
    preserve_path: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct RemoveRuleParams {
    /// Rule ID to remove
    id: String,
}

// ─── Helpers ────────────────────────────────────────────────

fn tool_error(msg: impl Into<String>) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::error(vec![Content::text(msg.into())]))
}

fn tool_ok(msg: impl Into<String>) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![Content::text(msg.into())]))
}

fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn next_rule_id() -> String {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    format!("mcp_{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}

fn read_body_text(file_path: &Option<String>, data_type: &proxy_v2_models::DataType) -> String {
    let Some(path) = file_path else {
        return "(body not available)".to_string();
    };
    let Ok(bytes) = std::fs::read(path) else {
        return "(file read error)".to_string();
    };
    if !data_type.is_text_based() {
        return format!("(binary, {:?})", data_type);
    }
    match String::from_utf8(bytes) {
        Ok(text) => {
            if text.len() > 10000 {
                format!(
                    "{}...\n(truncated, {} total)",
                    &text[..10000],
                    format_size(text.len())
                )
            } else {
                text
            }
        }
        Err(_) => "(binary data)".to_string(),
    }
}

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
        let rules = self.store.rules.lock().unwrap().clone();
        conn.send_command(&ClientCommand::UpdateInterceptRules { rules })
            .await
    }

    #[tool(
        description = "Search captured HTTP traffic. Filters by host, method, status code, or URL path. Returns a summary list with transaction IDs."
    )]
    async fn search_traffic(
        &self,
        Parameters(p): Parameters<SearchTrafficParams>,
    ) -> Result<CallToolResult, McpError> {
        let txns = self.store.transactions.lock().unwrap();
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
        let txns = self.store.transactions.lock().unwrap();
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
        let msgs = self.store.ws_messages.lock().unwrap();
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
        let rules = self.store.rules.lock().unwrap();
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

        self.store.rules.lock().unwrap().push(rule);

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
            let mut rules = self.store.rules.lock().unwrap();
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

    #[tool(description = "Check proxy daemon status and traffic statistics.")]
    async fn proxy_status(&self) -> Result<CallToolResult, McpError> {
        let connected = self.daemon_conn.lock().await.is_some();
        let txn_count = self.store.transactions.lock().unwrap().len();
        let ws_msg_count = self.store.ws_messages.lock().unwrap().len();
        let ws_conn_count = self.store.ws_connections.lock().unwrap().len();
        let rule_count = self.store.rules.lock().unwrap().len();
        let daemon_running = is_daemon_running().is_some();

        tool_ok(format!(
            "Daemon running: {}\nMCP connected: {}\nCaptured transactions: {}\nWebSocket connections: {}\nWebSocket messages: {}\nIntercept rules: {}",
            daemon_running, connected, txn_count, ws_conn_count, ws_msg_count, rule_count,
        ))
    }

    #[tool(description = "Clear all captured traffic data from memory.")]
    async fn clear_traffic(&self) -> Result<CallToolResult, McpError> {
        self.store.transactions.lock().unwrap().clear();
        self.store.ws_messages.lock().unwrap().clear();
        self.store.ws_connections.lock().unwrap().clear();
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

// ─── Daemon Connection ──────────────────────────────────────

async fn try_connect_daemon(store: &Store) -> Option<DaemonConnection> {
    if is_daemon_running().is_none() {
        return None;
    }

    let store = store.clone();
    match connect_to_daemon(move |msg| match msg {
        DaemonMessage::Event { data } => store.push_transaction(data),
        DaemonMessage::WsMessage { data } => store.push_ws_message(data),
        DaemonMessage::WsConnection { data } => store.push_ws_connection(data),
        DaemonMessage::InterceptRulesUpdated { rules } => {
            *store.rules.lock().unwrap() = rules;
        }
        _ => {}
    })
    .await
    {
        Ok(conn) => {
            tracing::info!("Connected to proxy daemon on port {}", conn.port);
            Some(conn)
        }
        Err(e) => {
            tracing::warn!("Failed to connect to daemon: {}", e);
            None
        }
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

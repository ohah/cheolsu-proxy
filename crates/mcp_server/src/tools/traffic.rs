use proxy_daemon::{format_diff_text, TrafficDiff, TransactionPartDiff};
use proxy_v2_models::WsDirection;
use rmcp::{handler::server::wrapper::Parameters, model::*, tool, ErrorData as McpError};

use crate::helpers::{format_size, read_body_text, tool_error, tool_ok};
use crate::params::*;
use crate::server::CheolsuMcpServer;
use crate::tools::diff_part;

impl CheolsuMcpServer {
    #[tool(
        description = "Search captured HTTP traffic. Filters by host, method, status code, or URL path. Returns a summary list with transaction IDs."
    )]
    pub(crate) async fn search_traffic(
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
            .filter_map(|info| {
                let req = info.0.as_ref()?;
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
                Some(format!(
                    "[{}] {} {} → {} ({}) {}",
                    req.id(),
                    req.method(),
                    req.uri(),
                    status,
                    size,
                    dtype,
                ))
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
    pub(crate) async fn get_transaction(
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
    pub(crate) async fn get_websocket_messages(
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
        description = "Compare two captured HTTP transactions (request + response) and show differences. Useful for regression testing and comparing API responses before/after deployment."
    )]
    pub(crate) async fn diff_transactions(
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
    pub(crate) async fn proxy_status(&self) -> Result<CallToolResult, McpError> {
        let connected = self.daemon_conn.lock().await.is_some();
        let txn_count = self.store.transactions.lock().len();
        let ws_msg_count = self.store.ws_messages.lock().len();
        let ws_conn_count = self.store.ws_connections.lock().len();
        let rule_count = self.store.rules.lock().len();
        let daemon_running = proxy_daemon::is_daemon_running().is_some();

        tool_ok(format!(
            "Daemon running: {}\nMCP connected: {}\nCaptured transactions: {}\nWebSocket connections: {}\nWebSocket messages: {}\nIntercept rules: {}",
            daemon_running, connected, txn_count, ws_conn_count, ws_msg_count, rule_count,
        ))
    }

    #[tool(description = "Clear all captured traffic data from memory.")]
    pub(crate) async fn clear_traffic(&self) -> Result<CallToolResult, McpError> {
        self.store.transactions.lock().clear();
        self.store.ws_messages.lock().clear();
        self.store.ws_connections.lock().clear();
        tool_ok("All captured traffic cleared.")
    }
}

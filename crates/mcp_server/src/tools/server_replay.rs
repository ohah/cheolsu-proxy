use proxy_daemon::ServerReplayEntry;
use rmcp::{handler::server::wrapper::Parameters, model::*, tool, ErrorData as McpError};

use crate::helpers::{next_server_replay_id, tool_error, tool_ok};
use crate::params::*;
use crate::server::CheolsuMcpServer;

impl CheolsuMcpServer {
    #[tool(
        description = "List all server replay entries. When enabled, matching incoming requests will receive the cached response instead of being forwarded to the upstream server."
    )]
    pub(crate) async fn list_server_replay(&self) -> Result<CallToolResult, McpError> {
        let entries = self.store.server_replay_entries.lock();
        if entries.is_empty() {
            return tool_ok("No server replay entries configured.");
        }
        let list: Vec<String> = entries
            .iter()
            .map(|e| {
                format!(
                    "  [{}] {} {} → {} (body: {})",
                    e.id,
                    e.method,
                    e.url,
                    e.status,
                    e.body
                        .as_ref()
                        .map(|b| format!("{} chars", b.len()))
                        .unwrap_or_else(|| "none".to_string()),
                )
            })
            .collect();
        tool_ok(format!(
            "{} server replay entries:\n\n{}",
            entries.len(),
            list.join("\n")
        ))
    }

    #[tool(
        description = "Add a captured transaction to server replay. The response from this transaction will be returned for matching future requests instead of forwarding to the upstream server."
    )]
    pub(crate) async fn add_server_replay(
        &self,
        Parameters(p): Parameters<AddServerReplayParams>,
    ) -> Result<CallToolResult, McpError> {
        let entry = {
            let txns = self.store.transactions.lock();
            let info = txns.iter().find(|info| {
                info.request
                    .as_ref()
                    .map(|r| r.id() == p.transaction_id)
                    .unwrap_or(false)
            });

            let Some(info) = info else {
                return tool_error(format!("Transaction '{}' not found.", p.transaction_id));
            };

            let Some(req) = &info.request else {
                return tool_error("Transaction has no request data.");
            };

            let Some(res) = &info.response else {
                return tool_error(
                    "Transaction has no response data. Only completed transactions can be added to server replay.",
                );
            };

            let headers: std::collections::HashMap<String, String> = res
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("<binary>").to_string()))
                .collect();

            let body = if res.body_size() > 0 && res.data_type().is_text_based() {
                res.file_path()
                    .as_ref()
                    .and_then(|p| std::fs::read(p).ok())
                    .and_then(|bytes| String::from_utf8(bytes).ok())
                    .or_else(|| res.body().and_then(|b| String::from_utf8(b.to_vec()).ok()))
            } else {
                None
            };

            ServerReplayEntry {
                id: next_server_replay_id(),
                method: req.method().to_string(),
                url: req.uri().to_string(),
                status: res.status().as_u16(),
                headers,
                body,
            }
        };

        let id = entry.id.clone();
        let url = entry.url.clone();
        self.store.server_replay_entries.lock().push(entry);

        match self.send_server_replay().await {
            Ok(()) => tool_ok(format!("Server replay entry '{}' added for {}.", id, url)),
            Err(e) => tool_error(format!(
                "Entry added locally but failed to sync with daemon: {}",
                e
            )),
        }
    }

    #[tool(description = "Remove a server replay entry by its ID.")]
    pub(crate) async fn remove_server_replay(
        &self,
        Parameters(p): Parameters<RemoveServerReplayParams>,
    ) -> Result<CallToolResult, McpError> {
        let removed = {
            let mut entries = self.store.server_replay_entries.lock();
            let before = entries.len();
            entries.retain(|e| e.id != p.id);
            entries.len() < before
        };

        if !removed {
            return tool_error(format!("Server replay entry '{}' not found.", p.id));
        }

        match self.send_server_replay().await {
            Ok(()) => tool_ok(format!("Server replay entry '{}' removed.", p.id)),
            Err(e) => tool_error(format!(
                "Entry removed locally but failed to sync with daemon: {}",
                e
            )),
        }
    }

    #[tool(description = "Clear all server replay entries.")]
    pub(crate) async fn clear_server_replay(&self) -> Result<CallToolResult, McpError> {
        self.store.server_replay_entries.lock().clear();
        match self.send_server_replay().await {
            Ok(()) => tool_ok("All server replay entries cleared."),
            Err(e) => tool_error(format!(
                "Entries cleared locally but failed to sync with daemon: {}",
                e
            )),
        }
    }
}

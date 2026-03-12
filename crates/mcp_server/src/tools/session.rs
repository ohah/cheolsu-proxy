use proxy_daemon::{InterceptRule, SessionFile};
use rmcp::{handler::server::wrapper::Parameters, model::*, tool, ErrorData as McpError};

use crate::helpers::{tool_error, tool_ok};
use crate::params::*;
use crate::server::CheolsuMcpServer;

impl CheolsuMcpServer {
    #[tool(
        description = "Save captured traffic to a .cheolsu session file. Use .cheolsu.gz extension for gzip compression. Optionally filter by URL substring."
    )]
    pub(crate) async fn save_session(
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

        let ws_messages: Vec<proxy_v2_models::WsMessageInfo> = {
            let guard = self.store.ws_messages.lock();
            guard.iter().cloned().collect()
        };
        let rules: Vec<InterceptRule> = {
            let guard = self.store.rules.lock();
            guard.clone()
        };

        let mut session =
            SessionFile::from_traffic(0, &transactions, &ws_messages, &rules, &[], None);

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
    pub(crate) async fn load_session(
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

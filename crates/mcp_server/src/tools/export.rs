use rmcp::{handler::server::wrapper::Parameters, model::*, tool, ErrorData as McpError};

use crate::helpers::{tool_error, tool_ok};
use crate::params::*;
use crate::server::CheolsuMcpServer;

impl CheolsuMcpServer {
    #[tool(
        description = "Export captured HTTP traffic as a HAR (HTTP Archive) 1.2 JSON file. Optionally filter by host or path. Saves to the specified file path."
    )]
    pub(crate) async fn export_har(
        &self,
        Parameters(p): Parameters<ExportHarParams>,
    ) -> Result<CallToolResult, McpError> {
        let txns = self.store.transactions.lock();

        let filtered: Vec<proxy_v2_models::RequestInfo> = txns
            .iter()
            .filter(|info| {
                let Some(req) = &info.request else {
                    return false;
                };
                let uri = req.uri().to_string();

                if let Some(ref host) = p.host {
                    if !uri.to_lowercase().contains(&host.to_lowercase()) {
                        return false;
                    }
                }
                if let Some(ref path) = p.path {
                    if !uri.to_lowercase().contains(&path.to_lowercase()) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        if filtered.is_empty() {
            return tool_ok("No matching transactions found.");
        }

        let count = filtered.len();
        let json_str = match proxy_v2_models::har::build_har_json(&filtered) {
            Ok(s) => s,
            Err(e) => return tool_error(format!("Failed to serialize HAR: {}", e)),
        };

        let path = &p.output_path;
        let file_path = std::path::Path::new(path);
        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    return tool_error(format!("Failed to create directory: {}", e));
                }
            }
        }

        match std::fs::write(file_path, &json_str) {
            Ok(()) => tool_ok(format!("HAR file saved to '{}' ({} entries).", path, count)),
            Err(e) => tool_error(format!("Failed to write HAR file: {}", e)),
        }
    }
}

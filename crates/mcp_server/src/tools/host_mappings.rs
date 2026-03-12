use proxy_daemon::HostMapping;
use rmcp::{handler::server::wrapper::Parameters, model::*, tool, ErrorData as McpError};

use crate::helpers::{next_mapping_id, tool_error, tool_ok};
use crate::params::*;
use crate::server::CheolsuMcpServer;

impl CheolsuMcpServer {
    #[tool(
        description = "List all host mappings (DNS spoofing / remote host mapping rules). Maps source hosts to target hosts/IPs for testing without modifying hosts file."
    )]
    pub(crate) async fn list_host_mappings(&self) -> Result<CallToolResult, McpError> {
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
        description = "Add a host mapping rule (DNS spoofing). Maps requests for a source host to a different target host/IP. Supports wildcard patterns (e.g., *.api.example.com). The original Host header is preserved so the target server routes to the correct virtual host."
    )]
    pub(crate) async fn add_host_mapping(
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
    pub(crate) async fn remove_host_mapping(
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
}

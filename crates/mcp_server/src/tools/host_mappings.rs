use proxy_daemon::HostMapping;
use rmcp::{handler::server::wrapper::Parameters, model::*, tool, ErrorData as McpError};

use crate::helpers::{add_and_sync, list_items, next_mapping_id, remove_and_sync};
use crate::params::*;
use crate::server::CheolsuMcpServer;

impl CheolsuMcpServer {
    #[tool(
        description = "List all host mappings (DNS spoofing / remote host mapping rules). Maps source hosts to target hosts/IPs for testing without modifying hosts file."
    )]
    pub(crate) async fn list_host_mappings(&self) -> Result<CallToolResult, McpError> {
        list_items(
            &self.store.host_mappings,
            "host mappings",
            "No host mappings configured.",
        )
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

        add_and_sync(
            &self.store.host_mappings,
            mapping,
            &id,
            "Host mapping",
            || self.send_host_mappings(),
        )
        .await
    }

    #[tool(description = "Remove a host mapping rule by its ID.")]
    pub(crate) async fn remove_host_mapping(
        &self,
        Parameters(p): Parameters<RemoveHostMappingParams>,
    ) -> Result<CallToolResult, McpError> {
        remove_and_sync(
            &self.store.host_mappings,
            &p.id,
            |m| &m.id,
            "Host mapping",
            || self.send_host_mappings(),
        )
        .await
    }
}

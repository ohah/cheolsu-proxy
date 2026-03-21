use rmcp::{handler::server::wrapper::Parameters, model::*, tool, ErrorData as McpError};

use crate::helpers::op_result_to_mcp;
use crate::params::*;
use crate::server::CheolsuMcpServer;

impl CheolsuMcpServer {
    #[tool(
        description = "List all host mappings (DNS spoofing / remote host mapping rules). Maps source hosts to target hosts/IPs for testing without modifying hosts file."
    )]
    pub(crate) async fn list_host_mappings(&self) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::host_mappings::list_host_mappings(
            &self.ops_ctx(),
        ))
    }

    #[tool(
        description = "Add a host mapping rule (DNS spoofing). Maps requests for a source host to a different target host/IP. Supports wildcard patterns (e.g., *.api.example.com). The original Host header is preserved so the target server routes to the correct virtual host."
    )]
    pub(crate) async fn add_host_mapping(
        &self,
        Parameters(p): Parameters<AddHostMappingParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::host_mappings::add_host_mapping(&self.ops_ctx(), p).await)
    }

    #[tool(description = "Remove a host mapping rule by its ID.")]
    pub(crate) async fn remove_host_mapping(
        &self,
        Parameters(p): Parameters<RemoveHostMappingParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::host_mappings::remove_host_mapping(&self.ops_ctx(), p).await)
    }
}

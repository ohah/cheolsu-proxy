use rmcp::{handler::server::wrapper::Parameters, model::*, tool, ErrorData as McpError};

use crate::helpers::op_result_to_mcp;
use crate::params::*;
use crate::server::CheolsuMcpServer;

impl CheolsuMcpServer {
    #[tool(
        description = "List all server replay entries. When enabled, matching incoming requests will receive the cached response instead of being forwarded to the upstream server."
    )]
    pub(crate) async fn list_server_replay(&self) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::server_replay::list_server_replay(
            &self.ops_ctx(),
        ))
    }

    #[tool(
        description = "Add a captured transaction to server replay. The response from this transaction will be returned for matching future requests instead of forwarding to the upstream server."
    )]
    pub(crate) async fn add_server_replay(
        &self,
        Parameters(p): Parameters<AddServerReplayParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::server_replay::add_server_replay(&self.ops_ctx(), p).await)
    }

    #[tool(description = "Remove a server replay entry by its ID.")]
    pub(crate) async fn remove_server_replay(
        &self,
        Parameters(p): Parameters<RemoveServerReplayParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::server_replay::remove_server_replay(&self.ops_ctx(), p).await)
    }

    #[tool(description = "Clear all server replay entries.")]
    pub(crate) async fn clear_server_replay(&self) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::server_replay::clear_server_replay(&self.ops_ctx()).await)
    }
}

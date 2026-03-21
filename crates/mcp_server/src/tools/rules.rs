use rmcp::{handler::server::wrapper::Parameters, model::*, tool, ErrorData as McpError};

use crate::helpers::op_result_to_mcp;
use crate::params::*;
use crate::server::CheolsuMcpServer;

impl CheolsuMcpServer {
    #[tool(description = "List all current intercept rules (block, modify, map local/remote).")]
    pub(crate) async fn list_rules(&self) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::rules::list_rules(&self.ops_ctx()))
    }

    #[tool(
        description = "Add a new intercept rule. Supports: block, modify_request, modify_response, map_local, map_remote."
    )]
    pub(crate) async fn add_rule(
        &self,
        Parameters(p): Parameters<AddRuleParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::rules::add_rule(&self.ops_ctx(), p).await)
    }

    #[tool(description = "Remove an intercept rule by its ID.")]
    pub(crate) async fn remove_rule(
        &self,
        Parameters(p): Parameters<RemoveRuleParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::rules::remove_rule(&self.ops_ctx(), p).await)
    }
}

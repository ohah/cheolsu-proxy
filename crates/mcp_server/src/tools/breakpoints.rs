use rmcp::{handler::server::wrapper::Parameters, model::*, tool, ErrorData as McpError};

use crate::helpers::op_result_to_mcp;
use crate::params::*;
use crate::server::CheolsuMcpServer;

impl CheolsuMcpServer {
    #[tool(
        description = "List all current breakpoint rules. Breakpoints pause matching requests/responses for manual inspection and editing."
    )]
    pub(crate) async fn list_breakpoints(&self) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::breakpoints::list_breakpoints(&self.ops_ctx()))
    }

    #[tool(
        description = "Add a breakpoint rule. When a matching request or response is intercepted, it will be paused for manual editing. Use list_pending_breakpoints to see paused items and resolve_breakpoint to continue."
    )]
    pub(crate) async fn add_breakpoint(
        &self,
        Parameters(p): Parameters<AddBreakpointParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::breakpoints::add_breakpoint(&self.ops_ctx(), p).await)
    }

    #[tool(description = "Remove a breakpoint rule by its ID.")]
    pub(crate) async fn remove_breakpoint(
        &self,
        Parameters(p): Parameters<RemoveBreakpointParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::breakpoints::remove_breakpoint(&self.ops_ctx(), p).await)
    }

    #[tool(
        description = "List currently paused (pending) breakpoints waiting for resolution. Returns breakpoint IDs that can be used with resolve_breakpoint."
    )]
    pub(crate) async fn list_pending_breakpoints(&self) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::breakpoints::list_pending_breakpoints())
    }

    #[tool(
        description = "Resolve a pending breakpoint. Choose an action: 'forward' (pass through as-is), 'modify_and_forward' (edit headers/body/status then forward), 'drop' (discard), or 'abort' (return error)."
    )]
    pub(crate) async fn resolve_breakpoint(
        &self,
        Parameters(p): Parameters<ResolveBreakpointParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::breakpoints::resolve_breakpoint(&self.ops_ctx(), p).await)
    }
}

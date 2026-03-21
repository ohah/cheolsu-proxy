use rmcp::{handler::server::wrapper::Parameters, model::*, tool, ErrorData as McpError};

use crate::helpers::op_result_to_mcp;
use crate::params::*;
use crate::server::CheolsuMcpServer;

impl CheolsuMcpServer {
    #[tool(
        description = "Search captured HTTP traffic. Filters by host, method, status code, or URL path. Returns a summary list with transaction IDs."
    )]
    pub(crate) async fn search_traffic(
        &self,
        Parameters(p): Parameters<SearchTrafficParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::traffic::search_traffic(&self.ops_ctx(), p))
    }

    #[tool(
        description = "Get full details of a specific HTTP transaction including request/response headers and body."
    )]
    pub(crate) async fn get_transaction(
        &self,
        Parameters(p): Parameters<GetTransactionParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::traffic::get_transaction(&self.ops_ctx(), p))
    }

    #[tool(description = "Get captured WebSocket messages, optionally filtered by connection URI.")]
    pub(crate) async fn get_websocket_messages(
        &self,
        Parameters(p): Parameters<GetWsMessagesParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::traffic::get_websocket_messages(
            &self.ops_ctx(),
            p,
        ))
    }

    #[tool(
        description = "Compare two captured HTTP transactions (request + response) and show differences. Useful for regression testing and comparing API responses before/after deployment."
    )]
    pub(crate) async fn diff_transactions(
        &self,
        Parameters(p): Parameters<DiffTransactionsParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::traffic::diff_transactions(&self.ops_ctx(), p))
    }

    #[tool(description = "Check proxy daemon status and traffic statistics.")]
    pub(crate) async fn proxy_status(&self) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::traffic::proxy_status(&self.ops_ctx()).await)
    }

    #[tool(description = "Clear all captured traffic data from memory.")]
    pub(crate) async fn clear_traffic(&self) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::traffic::clear_traffic(&self.ops_ctx()))
    }

    #[tool(
        description = "Generate an OpenAPI 3.0 specification from captured HTTP traffic. Automatically infers path parameters, request/response schemas from JSON bodies, and groups endpoints by method. Useful for documenting undocumented APIs."
    )]
    pub(crate) async fn generate_openapi_spec(
        &self,
        Parameters(p): Parameters<GenerateOpenApiParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::traffic::generate_openapi_spec(
            &self.ops_ctx(),
            p,
        ))
    }
}

use rmcp::{handler::server::wrapper::Parameters, model::*, tool, ErrorData as McpError};

use crate::helpers::op_result_to_mcp;
use crate::params::*;
use crate::server::CheolsuMcpServer;

impl CheolsuMcpServer {
    #[tool(
        description = "Get captured Server-Sent Events (SSE), optionally filtered by connection URI. Returns event type, data, and connection info."
    )]
    pub(crate) async fn get_sse_events(
        &self,
        Parameters(p): Parameters<GetSseEventsParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::sse::get_sse_events(&self.ops_ctx(), p))
    }
}

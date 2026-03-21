use rmcp::{handler::server::wrapper::Parameters, model::*, tool, ErrorData as McpError};

use crate::helpers::op_result_to_mcp;
use crate::params::*;
use crate::server::CheolsuMcpServer;

impl CheolsuMcpServer {
    #[tool(
        description = "Replay multiple captured HTTP transactions in sequence. Provide transaction IDs from search_traffic results. Optionally add delay between requests."
    )]
    pub(crate) async fn replay_sequence(
        &self,
        Parameters(p): Parameters<ReplaySequenceParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::replay::replay_sequence(&self.ops_ctx(), p).await)
    }

    #[tool(
        description = "Repeat an HTTP request multiple times with configurable concurrency and delay. Returns aggregated statistics (success/failure count, avg/min/max time, RPS)."
    )]
    pub(crate) async fn advanced_repeat(
        &self,
        Parameters(p): Parameters<AdvancedRepeatParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::replay::advanced_repeat(p).await)
    }
}

use rmcp::{handler::server::wrapper::Parameters, model::*, tool, ErrorData as McpError};

use crate::helpers::op_result_to_mcp;
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
        op_result_to_mcp(cheolsu_ops::export::export_har(&self.ops_ctx(), p))
    }
}

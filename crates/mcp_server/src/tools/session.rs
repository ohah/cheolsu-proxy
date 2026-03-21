use rmcp::{handler::server::wrapper::Parameters, model::*, tool, ErrorData as McpError};

use crate::helpers::op_result_to_mcp;
use crate::params::*;
use crate::server::CheolsuMcpServer;

impl CheolsuMcpServer {
    #[tool(
        description = "Save captured traffic to a .cheolsu session file. Use .cheolsu.gz extension for gzip compression. Optionally filter by URL substring."
    )]
    pub(crate) async fn save_session(
        &self,
        Parameters(p): Parameters<SaveSessionParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::session::save_session(&self.ops_ctx(), p))
    }

    #[tool(
        description = "Load a session file (.cheolsu, .cheolsu.gz) or import a HAR file (.har) into the traffic viewer. By default replaces current traffic; set append=true to add to existing."
    )]
    pub(crate) async fn load_session(
        &self,
        Parameters(p): Parameters<LoadSessionParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::session::load_session(&self.ops_ctx(), p))
    }
}

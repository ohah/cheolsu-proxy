use rmcp::{handler::server::wrapper::Parameters, model::*, tool, ErrorData as McpError};

use crate::helpers::op_result_to_mcp;
use crate::params::*;
use crate::server::CheolsuMcpServer;

impl CheolsuMcpServer {
    #[tool(
        description = "Load a JavaScript/TypeScript script to intercept and modify proxy traffic. Provide either a file path or inline code. The script can use cheolsu.onRequest(), cheolsu.onResponse(), cheolsu.onWebSocketMessage() hooks."
    )]
    pub(crate) async fn load_script(
        &self,
        Parameters(p): Parameters<LoadScriptParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::script::load_script(&self.ops_ctx(), p).await)
    }

    #[tool(description = "Unload the currently loaded proxy script.")]
    pub(crate) async fn unload_script(
        &self,
        #[allow(unused_variables)] Parameters(_p): Parameters<UnloadScriptParams>,
    ) -> Result<CallToolResult, McpError> {
        op_result_to_mcp(cheolsu_ops::script::unload_script(&self.ops_ctx()).await)
    }
}

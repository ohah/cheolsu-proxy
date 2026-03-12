use proxy_daemon::ClientCommand;
use rmcp::{handler::server::wrapper::Parameters, model::*, tool, ErrorData as McpError};

use crate::helpers::{tool_error, tool_ok};
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
        if p.path.is_none() && p.code.is_none() {
            return tool_error("Either 'path' or 'code' must be provided.");
        }
        let conn_guard = self.daemon_conn.lock().await;
        let Some(conn) = conn_guard.as_ref() else {
            return tool_error("Not connected to proxy daemon.");
        };
        let cmd = ClientCommand::LoadScript {
            path: p.path.clone(),
            code: p.code.clone(),
        };
        match conn.send_command(&cmd).await {
            Ok(()) => {
                let source = if let Some(ref path) = p.path {
                    format!("file '{}'", path)
                } else {
                    "inline code".to_string()
                };
                tool_ok(format!("Script loaded from {}.", source))
            }
            Err(e) => tool_error(format!("Failed to load script: {}", e)),
        }
    }

    #[tool(description = "Unload the currently loaded proxy script.")]
    pub(crate) async fn unload_script(
        &self,
        #[allow(unused_variables)] Parameters(_p): Parameters<UnloadScriptParams>,
    ) -> Result<CallToolResult, McpError> {
        let conn_guard = self.daemon_conn.lock().await;
        let Some(conn) = conn_guard.as_ref() else {
            return tool_error("Not connected to proxy daemon.");
        };
        match conn.send_command(&ClientCommand::UnloadScript).await {
            Ok(()) => tool_ok("Script unloaded."),
            Err(e) => tool_error(format!("Failed to unload script: {}", e)),
        }
    }
}

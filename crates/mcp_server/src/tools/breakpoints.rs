use proxy_daemon::{BreakpointAction, BreakpointRule, ClientCommand};
use rmcp::{handler::server::wrapper::Parameters, model::*, tool, ErrorData as McpError};

use crate::helpers::{
    add_and_sync, list_items, next_breakpoint_id, remove_and_sync, tool_error, tool_ok,
    with_daemon_conn,
};
use crate::params::*;
use crate::server::CheolsuMcpServer;

impl CheolsuMcpServer {
    #[tool(
        description = "List all current breakpoint rules. Breakpoints pause matching requests/responses for manual inspection and editing."
    )]
    pub(crate) async fn list_breakpoints(&self) -> Result<CallToolResult, McpError> {
        list_items(
            &self.store.breakpoint_rules,
            "breakpoint rules",
            "No breakpoint rules configured.",
        )
    }

    #[tool(
        description = "Add a breakpoint rule. When a matching request or response is intercepted, it will be paused for manual editing. Use list_pending_breakpoints to see paused items and resolve_breakpoint to continue."
    )]
    pub(crate) async fn add_breakpoint(
        &self,
        Parameters(p): Parameters<AddBreakpointParams>,
    ) -> Result<CallToolResult, McpError> {
        let id = next_breakpoint_id();
        let rule = BreakpointRule {
            id: id.clone(),
            pattern: p.pattern,
            break_on_request: p.break_on_request.unwrap_or(true),
            break_on_response: p.break_on_response.unwrap_or(false),
            enabled: true,
        };

        add_and_sync(
            &self.store.breakpoint_rules,
            rule,
            &id,
            "Breakpoint",
            || self.send_breakpoint_rules(),
        )
        .await
    }

    #[tool(description = "Remove a breakpoint rule by its ID.")]
    pub(crate) async fn remove_breakpoint(
        &self,
        Parameters(p): Parameters<RemoveBreakpointParams>,
    ) -> Result<CallToolResult, McpError> {
        remove_and_sync(
            &self.store.breakpoint_rules,
            &p.id,
            |r| &r.id,
            "Breakpoint",
            || self.send_breakpoint_rules(),
        )
        .await
    }

    #[tool(
        description = "List currently paused (pending) breakpoints waiting for resolution. Returns breakpoint IDs that can be used with resolve_breakpoint."
    )]
    pub(crate) async fn list_pending_breakpoints(&self) -> Result<CallToolResult, McpError> {
        tool_ok(
            "Pending breakpoints are shown as 'breakpoint_hit' events in the daemon stream. \
             Use the breakpoint ID from those events with resolve_breakpoint to continue."
                .to_string(),
        )
    }

    #[tool(
        description = "Resolve a pending breakpoint. Choose an action: 'forward' (pass through as-is), 'modify_and_forward' (edit headers/body/status then forward), 'drop' (discard), or 'abort' (return error)."
    )]
    pub(crate) async fn resolve_breakpoint(
        &self,
        Parameters(p): Parameters<ResolveBreakpointParams>,
    ) -> Result<CallToolResult, McpError> {
        let action = match p.action.as_str() {
            "forward" => BreakpointAction::Forward,
            "modify_and_forward" => BreakpointAction::ModifyAndForward {
                headers: p.headers,
                body: p.body,
                status: p.status,
            },
            "drop" => BreakpointAction::Drop,
            "abort" => BreakpointAction::Abort,
            other => {
                return tool_error(format!(
                    "Unknown action '{}'. Use: forward, modify_and_forward, drop, abort",
                    other
                ));
            }
        };

        let cmd = ClientCommand::ResolveBreakpoint {
            id: p.id.clone(),
            action,
        };
        match with_daemon_conn(&self.daemon_conn, &cmd).await {
            Ok(()) => tool_ok(format!("Breakpoint '{}' resolved.", p.id)),
            Err(e) => tool_error(format!("Failed to resolve breakpoint: {}", e)),
        }
    }
}

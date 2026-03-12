use proxy_daemon::{BreakpointAction, BreakpointRule, ClientCommand};
use rmcp::{handler::server::wrapper::Parameters, model::*, tool, ErrorData as McpError};

use crate::helpers::{next_breakpoint_id, tool_error, tool_ok};
use crate::params::*;
use crate::server::CheolsuMcpServer;

impl CheolsuMcpServer {
    #[tool(
        description = "List all current breakpoint rules. Breakpoints pause matching requests/responses for manual inspection and editing."
    )]
    pub(crate) async fn list_breakpoints(&self) -> Result<CallToolResult, McpError> {
        let rules = self.store.breakpoint_rules.lock();
        if rules.is_empty() {
            return tool_ok("No breakpoint rules configured.");
        }
        let list: Vec<String> = rules.iter().map(|r| format!("  {}", r)).collect();
        tool_ok(format!(
            "{} breakpoint rules:\n\n{}",
            rules.len(),
            list.join("\n")
        ))
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

        self.store.breakpoint_rules.lock().push(rule);

        match self.send_breakpoint_rules().await {
            Ok(()) => tool_ok(format!("Breakpoint '{}' added successfully.", id)),
            Err(e) => tool_error(format!(
                "Breakpoint added locally but failed to sync with daemon: {}",
                e
            )),
        }
    }

    #[tool(description = "Remove a breakpoint rule by its ID.")]
    pub(crate) async fn remove_breakpoint(
        &self,
        Parameters(p): Parameters<RemoveBreakpointParams>,
    ) -> Result<CallToolResult, McpError> {
        let removed = {
            let mut rules = self.store.breakpoint_rules.lock();
            let before = rules.len();
            rules.retain(|r| r.id != p.id);
            rules.len() < before
        };

        if !removed {
            return tool_error(format!("Breakpoint '{}' not found.", p.id));
        }

        match self.send_breakpoint_rules().await {
            Ok(()) => tool_ok(format!("Breakpoint '{}' removed.", p.id)),
            Err(e) => tool_error(format!(
                "Breakpoint removed locally but failed to sync with daemon: {}",
                e
            )),
        }
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

        let conn_guard = self.daemon_conn.lock().await;
        let Some(conn) = conn_guard.as_ref() else {
            return tool_error("Not connected to proxy daemon.");
        };
        let cmd = ClientCommand::ResolveBreakpoint {
            id: p.id.clone(),
            action,
        };
        match conn.send_command(&cmd).await {
            Ok(()) => tool_ok(format!("Breakpoint '{}' resolved.", p.id)),
            Err(e) => tool_error(format!("Failed to resolve breakpoint: {}", e)),
        }
    }
}

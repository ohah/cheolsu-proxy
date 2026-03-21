use proxy_daemon::{BreakpointAction, BreakpointRule, ClientCommand};

use crate::context::OpsContext;
use crate::helpers::{add_and_sync, list_items, remove_and_sync, with_daemon_conn};
use crate::id::next_breakpoint_id;
use crate::params::*;
use crate::result::OpResult;

pub fn list_breakpoints(ctx: &OpsContext) -> OpResult {
    list_items(
        &ctx.store.breakpoint_rules,
        "breakpoint rules",
        "No breakpoint rules configured.",
    )
}

pub async fn add_breakpoint(ctx: &OpsContext, p: AddBreakpointParams) -> OpResult {
    let id = next_breakpoint_id();
    let rule = BreakpointRule {
        id: id.clone(),
        pattern: p.pattern,
        break_on_request: p.break_on_request.unwrap_or(true),
        break_on_response: p.break_on_response.unwrap_or(false),
        enabled: true,
    };

    add_and_sync(&ctx.store.breakpoint_rules, rule, &id, "Breakpoint", || {
        ctx.send_breakpoint_rules()
    })
    .await
}

pub async fn remove_breakpoint(ctx: &OpsContext, p: RemoveBreakpointParams) -> OpResult {
    remove_and_sync(
        &ctx.store.breakpoint_rules,
        &p.id,
        |r| &r.id,
        "Breakpoint",
        || ctx.send_breakpoint_rules(),
    )
    .await
}

pub fn list_pending_breakpoints() -> OpResult {
    OpResult::ok(
        "Pending breakpoints are shown as 'breakpoint_hit' events in the daemon stream. \
         Use the breakpoint ID from those events with resolve_breakpoint to continue.",
    )
}

pub async fn resolve_breakpoint(ctx: &OpsContext, p: ResolveBreakpointParams) -> OpResult {
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
            return OpResult::err(format!(
                "Unknown action '{}'. Use: forward, modify_and_forward, drop, abort",
                other
            ));
        }
    };

    let cmd = ClientCommand::ResolveBreakpoint {
        id: p.id.clone(),
        action,
    };
    match with_daemon_conn(&ctx.daemon_conn, &cmd).await {
        Ok(()) => OpResult::ok(format!("Breakpoint '{}' resolved.", p.id)),
        Err(e) => OpResult::err(format!("Failed to resolve breakpoint: {}", e)),
    }
}

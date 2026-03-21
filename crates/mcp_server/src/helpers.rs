use cheolsu_ops::result::OpResult;
use rmcp::model::*;
use rmcp::ErrorData as McpError;

#[allow(unused_imports)]
pub use cheolsu_ops::helpers::{compute_time_stats, format_size, read_body_text};
#[allow(unused_imports)]
pub use cheolsu_ops::id::{
    next_breakpoint_id, next_mapping_id, next_rule_id, next_server_replay_id,
};

pub fn tool_error(msg: impl Into<String>) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::error(vec![Content::text(msg.into())]))
}

pub fn tool_ok(msg: impl Into<String>) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![Content::text(msg.into())]))
}

pub fn op_result_to_mcp(r: OpResult) -> Result<CallToolResult, McpError> {
    match r {
        OpResult::Ok(msg) => tool_ok(msg),
        OpResult::Err(msg) => tool_error(msg),
    }
}

use std::fmt::Display;
use std::sync::Arc;

use cheolsu_ops::result::OpResult;
use parking_lot::Mutex;
use rmcp::model::*;
use rmcp::ErrorData as McpError;

// cheolsu_ops 헬퍼를 re-export
pub use cheolsu_ops::helpers::{compute_time_stats, format_size, read_body_text, with_daemon_conn};
pub use cheolsu_ops::id::{
    next_breakpoint_id, next_mapping_id, next_reverse_proxy_id, next_rule_id, next_server_replay_id,
};

// ─── rmcp 전용 함수 ─────────────────────────────────────

pub fn tool_error(msg: impl Into<String>) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::error(vec![Content::text(msg.into())]))
}

pub fn tool_ok(msg: impl Into<String>) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![Content::text(msg.into())]))
}

/// OpResult → Result<CallToolResult, McpError> 변환
pub fn op_result_to_mcp(r: OpResult) -> Result<CallToolResult, McpError> {
    match r {
        OpResult::Ok(msg) => tool_ok(msg),
        OpResult::Err(msg) => tool_error(msg),
    }
}

// ─── 아직 변환되지 않은 tools를 위한 MCP 래퍼 ───────────
// Phase 2, 3에서 각 tool이 cheolsu_ops로 이동되면 제거될 함수들

/// Store의 리스트 항목을 포맷팅하여 반환 (MCP 반환 타입)
pub fn list_items<T: Display>(
    items: &Arc<Mutex<Vec<T>>>,
    label: &str,
    empty_msg: &str,
) -> Result<CallToolResult, McpError> {
    op_result_to_mcp(cheolsu_ops::helpers::list_items(items, label, empty_msg))
}

/// ID로 항목을 제거하고 sync 함수를 호출 (MCP 반환 타입)
pub async fn remove_and_sync<T, F, Fut>(
    items: &Arc<Mutex<Vec<T>>>,
    id: &str,
    id_extractor: impl Fn(&T) -> &str,
    item_label: &str,
    sync_fn: F,
) -> Result<CallToolResult, McpError>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
{
    op_result_to_mcp(
        cheolsu_ops::helpers::remove_and_sync(items, id, id_extractor, item_label, sync_fn).await,
    )
}

/// 항목을 추가하고 sync 함수를 호출 (MCP 반환 타입)
pub async fn add_and_sync<T, F, Fut>(
    items: &Arc<Mutex<Vec<T>>>,
    item: T,
    id: &str,
    item_label: &str,
    sync_fn: F,
) -> Result<CallToolResult, McpError>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
{
    op_result_to_mcp(cheolsu_ops::helpers::add_and_sync(items, item, id, item_label, sync_fn).await)
}

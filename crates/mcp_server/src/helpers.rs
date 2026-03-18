use std::fmt::Display;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;
use proxy_daemon::{ClientCommand, DaemonConnection};
use rmcp::model::*;
use rmcp::ErrorData as McpError;
use tokio::sync::Mutex as TokioMutex;

pub fn tool_error(msg: impl Into<String>) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::error(vec![Content::text(msg.into())]))
}

pub fn tool_ok(msg: impl Into<String>) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![Content::text(msg.into())]))
}

pub fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

pub fn next_rule_id() -> String {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    format!("mcp_{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}

pub fn next_breakpoint_id() -> String {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    format!("mcp_bp_{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}

pub fn next_mapping_id() -> String {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    format!("hm_{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}

pub fn next_reverse_proxy_id() -> String {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    format!("rp_{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}

pub fn next_server_replay_id() -> String {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    format!("sr_{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}

/// Store의 리스트 항목을 포맷팅하여 반환하는 공통 헬퍼.
/// 비어있으면 `empty_msg`, 있으면 `label` + 아이템 목록을 반환합니다.
pub fn list_items<T: Display>(
    items: &Arc<Mutex<Vec<T>>>,
    label: &str,
    empty_msg: &str,
) -> Result<CallToolResult, McpError> {
    let guard = items.lock();
    if guard.is_empty() {
        return tool_ok(empty_msg);
    }
    let list: Vec<String> = guard.iter().map(|item| format!("  {}", item)).collect();
    tool_ok(format!("{} {}:\n\n{}", guard.len(), label, list.join("\n")))
}

/// ID로 항목을 제거하고 sync 함수를 호출하는 공통 헬퍼.
/// 제거 성공 시 `sync_fn`을 호출하고 결과를 반환합니다.
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
    let removed = {
        let mut guard = items.lock();
        let before = guard.len();
        guard.retain(|item| id_extractor(item) != id);
        guard.len() < before
    };

    if !removed {
        return tool_error(format!("{} '{}' not found.", item_label, id));
    }

    match sync_fn().await {
        Ok(()) => tool_ok(format!("{} '{}' removed.", item_label, id)),
        Err(e) => tool_error(format!(
            "{} removed locally but failed to sync with daemon: {}",
            item_label, e
        )),
    }
}

/// 항목을 추가하고 sync 함수를 호출하는 공통 헬퍼.
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
    items.lock().push(item);

    match sync_fn().await {
        Ok(()) => tool_ok(format!("{} '{}' added successfully.", item_label, id)),
        Err(e) => tool_error(format!(
            "{} added locally but failed to sync with daemon: {}",
            item_label, e
        )),
    }
}

/// daemon_conn을 잠그고 연결을 확인한 뒤 커맨드를 전송하는 공통 헬퍼.
pub async fn with_daemon_conn(
    daemon_conn: &Arc<TokioMutex<Option<DaemonConnection>>>,
    cmd: &ClientCommand,
) -> Result<(), String> {
    let conn_guard = daemon_conn.lock().await;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Not connected to proxy daemon".to_string())?;
    conn.send_command(cmd).await.map_err(|e| e.to_string())
}

/// 응답 시간 통계를 계산합니다.
/// `times`가 비어있으면 (0.0, 0.0, 0.0)을 반환하여 NaN을 방지합니다.
pub fn compute_time_stats(times: &[f64]) -> (f64, f64, f64) {
    if times.is_empty() {
        (0.0, 0.0, 0.0)
    } else {
        (
            times.iter().sum::<f64>() / times.len() as f64,
            times.iter().cloned().fold(f64::MAX, f64::min),
            times.iter().cloned().fold(0.0f64, f64::max),
        )
    }
}

pub fn read_body_text(file_path: &Option<String>, data_type: &proxy_v2_models::DataType) -> String {
    let Some(path) = file_path else {
        return "(body not available)".to_string();
    };
    let Ok(bytes) = std::fs::read(path) else {
        return "(file read error)".to_string();
    };
    if !data_type.is_text_based() {
        return format!("(binary, {:?})", data_type);
    }
    match String::from_utf8(bytes) {
        Ok(text) => {
            if text.len() > 10000 {
                format!(
                    "{}...\n(truncated, {} total)",
                    &text[..10000],
                    format_size(text.len())
                )
            } else {
                text
            }
        }
        Err(_) => "(binary data)".to_string(),
    }
}

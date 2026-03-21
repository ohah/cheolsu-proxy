use std::collections::VecDeque;
use std::fmt::Display;
use std::sync::Arc;

use parking_lot::Mutex;
use proxy_daemon::{ClientCommand, DaemonConnection};
use proxy_v2_models::RequestInfo;
use tokio::sync::Mutex as TokioMutex;

use crate::result::OpResult;

pub fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// 응답 시간 통계를 계산한다. (avg, min, max)
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
        Ok(text) => truncate_text(text, 10000),
        Err(_) => "(binary data)".to_string(),
    }
}

/// UTF-8 경계를 존중하여 텍스트를 잘라낸다.
pub fn truncate_text(text: String, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text;
    }
    // UTF-8 문자 경계에서 자르기
    let mut end = max_bytes;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    format!(
        "{}...\n(truncated, {} total)",
        &text[..end],
        format_size(text.len())
    )
}

/// VecDeque에서 request ID로 트랜잭션을 찾는다.
pub fn find_transaction_by_id<'a>(
    txns: &'a VecDeque<RequestInfo>,
    id: &str,
) -> Option<&'a RequestInfo> {
    txns.iter()
        .find(|info| info.request.as_ref().map(|r| r.id() == id).unwrap_or(false))
}

/// replay용 reqwest Client를 생성한다.
pub fn build_replay_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .no_proxy()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))
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

/// Store의 리스트 항목을 포맷팅하여 반환하는 공통 헬퍼.
pub fn list_items<T: Display>(
    items: &Arc<Mutex<Vec<T>>>,
    label: &str,
    empty_msg: &str,
) -> OpResult {
    let guard = items.lock();
    if guard.is_empty() {
        return OpResult::ok(empty_msg);
    }
    let list: Vec<String> = guard.iter().map(|item| format!("  {}", item)).collect();
    OpResult::ok(format!("{} {}:\n\n{}", guard.len(), label, list.join("\n")))
}

/// ID로 항목을 제거하고 sync 함수를 호출하는 공통 헬퍼.
pub async fn remove_and_sync<T, F, Fut>(
    items: &Arc<Mutex<Vec<T>>>,
    id: &str,
    id_extractor: impl Fn(&T) -> &str,
    item_label: &str,
    sync_fn: F,
) -> OpResult
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
        return OpResult::err(format!("{} '{}' not found.", item_label, id));
    }

    match sync_fn().await {
        Ok(()) => OpResult::ok(format!("{} '{}' removed.", item_label, id)),
        Err(e) => OpResult::err(format!(
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
) -> OpResult
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
{
    items.lock().push(item);

    match sync_fn().await {
        Ok(()) => OpResult::ok(format!("{} '{}' added successfully.", item_label, id)),
        Err(e) => OpResult::err(format!(
            "{} added locally but failed to sync with daemon: {}",
            item_label, e
        )),
    }
}

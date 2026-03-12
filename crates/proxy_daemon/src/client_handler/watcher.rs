use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::{broadcast, Mutex};
use tracing::{error, info};

use crate::protocol::DaemonMessage;

/// 파일 감시 시작 (스크립트 변경 시 자동 리로드)
pub(super) fn start_file_watcher(
    path: String,
    script_handle: scripting::ScriptHandle,
    writer: Arc<Mutex<tokio::net::unix::OwnedWriteHalf>>,
    watched_path: Arc<Mutex<Option<String>>>,
    event_tx: broadcast::Sender<String>,
) {
    use notify::{EventKind, RecursiveMode, Watcher};

    tokio::spawn(async move {
        let (notify_tx, mut notify_rx) = tokio::sync::mpsc::channel(16);

        let mut watcher = match notify::recommended_watcher(move |res: Result<notify::Event, _>| {
            if let Ok(event) = res {
                if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                    let _ = notify_tx.blocking_send(());
                }
            }
        }) {
            Ok(w) => w,
            Err(e) => {
                error!("Failed to create file watcher: {}", e);
                return;
            }
        };

        if let Err(e) = watcher.watch(std::path::Path::new(&path), RecursiveMode::NonRecursive) {
            error!("Failed to watch file {}: {}", path, e);
            return;
        }

        info!("[Script] Watching file: {}", path);

        // debounce: 최소 500ms 간격으로 리로드
        let mut last_reload = std::time::Instant::now();

        while notify_rx.recv().await.is_some() {
            // 감시 경로가 해제되었으면 종료
            // 로컬 변수에 클론하여 레이스 컨디션 방지
            let current_path = watched_path.lock().await.clone();
            if current_path.as_deref() != Some(&path) {
                break;
            }

            // debounce
            let now = std::time::Instant::now();
            if now.duration_since(last_reload) < std::time::Duration::from_millis(500) {
                continue;
            }
            last_reload = now;

            // 짧은 대기 (에디터가 파일 쓰기를 완료할 시간)
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;

            info!("[Script] File changed, reloading: {}", path);

            let reload_result = script_handle.load_file(&path).await;

            let (msg, level) = match &reload_result {
                Ok(()) => (
                    format!("스크립트 리로드 완료: {}", path),
                    "info".to_string(),
                ),
                Err(e) => (format!("스크립트 리로드 실패: {}", e), "error".to_string()),
            };

            // 로그 메시지 전송
            let log_msg = DaemonMessage::ScriptLog {
                level,
                message: msg.clone(),
            };
            if let Ok(json) = serde_json::to_string(&log_msg) {
                let mut line = json;
                line.push('\n');
                let mut w = writer.lock().await;
                let _ = w.write_all(line.as_bytes()).await;
                let _ = w.flush().await;
            }

            // 스크립트 상태 브로드캐스트
            let status_msg = DaemonMessage::ScriptStatus {
                active: reload_result.is_ok(),
                path: Some(path.clone()),
                message: msg,
            };
            if let Ok(json) = serde_json::to_string(&status_msg) {
                let _ = event_tx.send(json);
            }
        }

        info!("[Script] File watcher stopped: {}", path);
    });
}

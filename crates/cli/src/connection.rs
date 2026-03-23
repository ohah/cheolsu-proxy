use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use cheolsu_ops::context::{OpsContext, OpsStore};
use proxy_daemon::{connect_to_daemon, is_daemon_running};
use tokio::sync::Mutex as TokioMutex;

pub async fn connect() -> Result<OpsContext, String> {
    if is_daemon_running().is_none() {
        return Err("Cheolsu Proxy 데몬이 실행 중이 아닙니다. 먼저 프록시를 시작해주세요.".into());
    }

    let store = OpsStore::new();
    let store_clone = store.clone();
    let received = Arc::new(AtomicBool::new(false));
    let received_clone = received.clone();

    let conn = connect_to_daemon(move |msg| {
        received_clone.store(true, Ordering::Relaxed);
        store_clone.handle_daemon_message(msg);
    })
    .await
    .map_err(|e| format!("데몬 연결 실패: {}", e))?;

    // 초기 데이터 동기화 대기: 메시지가 오면 즉시 진행, 최대 500ms
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(500);
    loop {
        if received.load(Ordering::Relaxed) {
            // 첫 메시지 수신 후 추가 50ms 대기 (연속 메시지 수신)
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    Ok(OpsContext {
        store,
        daemon_conn: Arc::new(TokioMutex::new(Some(conn))),
    })
}

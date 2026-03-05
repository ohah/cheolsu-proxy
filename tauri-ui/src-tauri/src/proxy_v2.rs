use proxy_daemon::{clean_old_cache, ClientCommand, DaemonConnection};
use std::net::SocketAddr;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime, State};
use tauri_plugin_store::{JsonValue, StoreExt};
use tokio::sync::Mutex;

/// 프록시 상태: daemon과의 연결을 관리
pub type ProxyV2State = Arc<Mutex<Option<DaemonConnection>>>;

/// 프록시 시작 결과를 나타내는 구조체
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProxyStartResult {
    pub status: bool,
    pub message: String,
}

/// 프록시 시작: daemon에 연결하여 이벤트를 GUI로 포워딩
#[tauri::command]
pub async fn start_proxy_v2<R: Runtime>(
    app: AppHandle<R>,
    proxy: State<'_, ProxyV2State>,
    addr: SocketAddr,
) -> Result<ProxyStartResult, ProxyStartResult> {
    let proxy_guard = proxy.lock().await;
    if proxy_guard.is_some() {
        let already_running_message = format!(
            "프록시가 이미 포트 {}에서 실행 중입니다. 시스템 프록시 설정을 127.0.0.1:{}로 변경하세요",
            addr.port(),
            addr.port()
        );
        return Ok(ProxyStartResult {
            status: true,
            message: already_running_message,
        });
    }
    drop(proxy_guard);

    let port = addr.port();
    let host = addr.ip().to_string();

    let app_clone = app.clone();
    let conn = match proxy_daemon::ensure_daemon(port, &host, move |event| {
        let _ = app_clone.emit("proxy_event", event);
    })
    .await
    {
        Ok(conn) => conn,
        Err(e) => {
            let error_msg = format!("Daemon 연결 실패: {}", e);
            eprintln!("{}", error_msg);
            return Err(ProxyStartResult {
                status: false,
                message: error_msg,
            });
        }
    };

    // 세션 데이터를 daemon에 전송
    if let Ok(store) = app.store("session.json") {
        let sessions = store.get("sessions").unwrap_or_default();
        let cmd = ClientCommand::UpdateSessions { data: sessions };
        if let Err(e) = conn.send_command(&cmd).await {
            eprintln!("Failed to send initial sessions: {}", e);
        }
    }

    let mut proxy_guard = proxy.lock().await;
    proxy_guard.replace(conn);

    let log_path = proxy_daemon::daemon::app_support_dir()
        .map(|d| d.join("daemon.log").display().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let success_message = format!(
        "프록시가 포트 {}에서 성공적으로 시작되었습니다. 시스템 프록시 설정을 127.0.0.1:{}로 변경하세요 (로그: {})",
        port, port, log_path
    );

    println!("{}", success_message);
    Ok(ProxyStartResult {
        status: true,
        message: success_message,
    })
}

/// 프록시 중지: daemon과의 연결 해제
#[tauri::command]
pub async fn stop_proxy_v2(proxy: tauri::State<'_, ProxyV2State>) -> Result<(), String> {
    let mut proxy_guard = proxy.lock().await;

    if let Some(conn) = proxy_guard.take() {
        conn.disconnect().await;
        println!("Daemon 연결 해제 완료");
    } else {
        return Err("프록시가 실행 중이 아닙니다".to_string());
    }

    Ok(())
}

/// 프록시 상태 확인
#[tauri::command]
pub async fn proxy_v2_status(proxy: tauri::State<'_, ProxyV2State>) -> Result<bool, String> {
    Ok(proxy.lock().await.is_some())
}

/// 파일에서 body 데이터 읽기
#[tauri::command]
pub async fn read_body_file(file_path: String) -> Result<Vec<u8>, String> {
    std::fs::read(&file_path).map_err(|e| format!("파일 읽기 실패: {} - {}", file_path, e))
}

/// 오래된 캐시 정리 명령어
#[tauri::command]
pub async fn clean_old_proxy_cache(days: u64) -> Result<String, String> {
    match clean_old_cache(days) {
        Ok(_) => Ok(format!(
            "{}일 이상 된 캐시가 성공적으로 정리되었습니다",
            days
        )),
        Err(e) => Err(format!("오래된 캐시 정리 실패: {}", e)),
    }
}

/// 세션 데이터 변경 시 UDS를 통해 daemon에 전달
#[tauri::command]
pub async fn store_changed_v2<R: Runtime>(
    app: AppHandle<R>,
    proxy: State<'_, ProxyV2State>,
) -> Result<(), String> {
    let proxy_guard = proxy.lock().await;

    if proxy_guard.is_none() {
        println!("store_changed_v2: Proxy가 실행 중이 아니므로 세션 업데이트를 무시합니다");
        return Ok(());
    }

    let store = app.store("session.json").map_err(|e| e.to_string())?;
    let sessions = store.get("sessions").unwrap_or_default();

    let session_count = if let JsonValue::Array(arr) = &sessions {
        arr.len()
    } else {
        0
    };

    println!("세션 데이터 업데이트: {} 개의 세션", session_count);

    if let Some(conn) = proxy_guard.as_ref() {
        let cmd = ClientCommand::UpdateSessions { data: sessions };
        conn.send_command(&cmd).await?;
        println!("Daemon에 세션 데이터 업데이트 완료");
    }

    Ok(())
}

use proxy_daemon::{clean_old_cache, ClientCommand, DaemonConnection, InterceptRule};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime, State};
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

/// 인터셉트 규칙 업데이트
#[tauri::command]
pub async fn update_intercept_rules_v2(
    proxy: tauri::State<'_, ProxyV2State>,
    rules: Vec<InterceptRule>,
) -> Result<(), String> {
    let proxy_guard = proxy.lock().await;

    if let Some(conn) = proxy_guard.as_ref() {
        let cmd = ClientCommand::UpdateInterceptRules { rules };
        conn.send_command(&cmd).await?;
        println!("Daemon에 인터셉트 규칙 업데이트 완료");
    } else {
        return Err("프록시가 실행 중이 아닙니다".to_string());
    }

    Ok(())
}

/// 리플레이 요청 파라미터
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReplayRequestParams {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

/// 리플레이 응답 결과
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReplayResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub body_size: usize,
    pub elapsed_ms: u64,
}

/// 시퀀스 리플레이 결과 (개별 요청 결과)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SequenceReplayResult {
    pub index: usize,
    pub url: String,
    pub method: String,
    pub response: Option<ReplayResponse>,
    pub error: Option<String>,
}

/// HTTP 요청 리플레이 (프록시를 우회하여 직접 전송)
#[tauri::command]
pub async fn replay_request(params: ReplayRequestParams) -> Result<ReplayResponse, String> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| format!("HTTP 클라이언트 생성 실패: {}", e))?;

    let method: reqwest::Method = params
        .method
        .parse()
        .map_err(|e| format!("잘못된 HTTP 메서드: {}", e))?;

    let mut request_builder = client.request(method, &params.url);

    for (key, value) in &params.headers {
        // hop-by-hop 헤더 제외
        let lower = key.to_lowercase();
        if matches!(
            lower.as_str(),
            "host"
                | "connection"
                | "keep-alive"
                | "proxy-authenticate"
                | "proxy-authorization"
                | "te"
                | "trailers"
                | "transfer-encoding"
                | "upgrade"
        ) {
            continue;
        }
        request_builder = request_builder.header(key.as_str(), value.as_str());
    }

    if let Some(body) = params.body {
        request_builder = request_builder.body(body);
    }

    let start = std::time::Instant::now();
    let response = request_builder
        .send()
        .await
        .map_err(|e| format!("요청 전송 실패: {}", e))?;
    let elapsed_ms = start.elapsed().as_millis() as u64;

    let status = response.status().as_u16();
    let headers: HashMap<String, String> = response
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let body_bytes = response
        .bytes()
        .await
        .map_err(|e| format!("응답 본문 읽기 실패: {}", e))?;
    let body_size = body_bytes.len();

    // 텍스트로 변환 시도, 실패하면 base64
    let body = if body_size == 0 {
        None
    } else {
        Some(String::from_utf8(body_bytes.to_vec()).unwrap_or_else(|_| {
            let engine = base64_engine();
            format!("base64:{}", base64_encode(&engine, &body_bytes))
        }))
    };

    Ok(ReplayResponse {
        status,
        headers,
        body,
        body_size,
        elapsed_ms,
    })
}

/// 시퀀스 리플레이 (여러 요청을 순서대로 전송)
#[tauri::command]
pub async fn replay_sequence(
    requests: Vec<ReplayRequestParams>,
) -> Result<Vec<SequenceReplayResult>, String> {
    let mut results = Vec::new();

    for (index, params) in requests.into_iter().enumerate() {
        let url = params.url.clone();
        let method = params.method.clone();

        match replay_request(params).await {
            Ok(response) => {
                results.push(SequenceReplayResult {
                    index,
                    url,
                    method,
                    response: Some(response),
                    error: None,
                });
            }
            Err(e) => {
                results.push(SequenceReplayResult {
                    index,
                    url,
                    method,
                    response: None,
                    error: Some(e),
                });
            }
        }
    }

    Ok(results)
}

fn base64_engine() -> base64::engine::GeneralPurpose {
    use base64::engine::general_purpose::STANDARD;
    STANDARD
}

fn base64_encode(engine: &base64::engine::GeneralPurpose, data: &[u8]) -> String {
    use base64::Engine;
    engine.encode(data)
}

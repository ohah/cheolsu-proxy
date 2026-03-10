use proxy_daemon::{
    clean_old_cache, diff_headers, diff_json, diff_text, get_local_ips, is_text_data_type,
    BodyDiff, BreakpointAction, BreakpointRule, ClientCommand, DaemonConnection, DaemonMessage,
    HostMapping, InterceptRule, ProxyAuthConfig, ServerReplayEntry, SslProxyingEntry,
    ThrottleConfig, TrafficDiff, TransactionPartDiff, UpstreamProxyConfig,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime, State};
use tokio::sync::Mutex;

// ============================================================
// 데드락 진단용 플래그 — 하나씩 false로 바꿔가며 테스트
// 모두 true인 상태가 현재(문제 있는) 상태
// ============================================================
/// Group A: event-emitter 전용 스레드 사용 (false = tokio task에서 직접 emit)
const DIAG_USE_EVENT_EMITTER_THREAD: bool = false;
/// Group B: 빠른 설정/새 설정 커맨드 (false = 즉시 Ok 반환)
const DIAG_ENABLE_NEW_SETTINGS: bool = false;
/// Group C: 세션 자동저장/복원 (false = 즉시 Ok/None 반환)
const DIAG_ENABLE_AUTO_SESSION: bool = false;
/// Group D: advanced_repeat (false = 즉시 빈 결과 반환)
const DIAG_ENABLE_ADVANCED_REPEAT: bool = false;
/// Group E: 이벤트 포워딩 (false = app.emit 호출 자체를 하지 않음)
const DIAG_ENABLE_EVENT_FORWARDING: bool = false;
/// Group F: 프록시 daemon 연결 (false = start_proxy_v2가 연결 없이 성공 반환)
const DIAG_ENABLE_PROXY_CONNECTION: bool = false;

/// DaemonMessage를 app.emit()으로 전달하는 헬퍼
fn emit_daemon_message<R: Runtime>(app: &AppHandle<R>, msg: DaemonMessage) {
    if !DIAG_ENABLE_EVENT_FORWARDING {
        return; // Group E: 이벤트 포워딩 완전 차단
    }
    match msg {
        DaemonMessage::Event { data } => {
            let _ = app.emit("proxy_event", data);
        }
        DaemonMessage::WsMessage { data } => {
            let _ = app.emit("ws_message", data);
        }
        DaemonMessage::WsConnection { data } => {
            let _ = app.emit("ws_connection", data);
        }
        DaemonMessage::InterceptRulesUpdated { rules } => {
            let _ = app.emit("intercept_rules_updated", rules);
        }
        DaemonMessage::ScriptLog { level, message } => {
            let _ = app.emit(
                "script_log",
                serde_json::json!({ "level": level, "message": message }),
            );
        }
        DaemonMessage::ScriptStatus {
            active,
            path,
            message,
        } => {
            let _ = app.emit(
                "script_status",
                serde_json::json!({ "active": active, "path": path, "message": message }),
            );
        }
        DaemonMessage::ScriptResult { success, error } => {
            let _ = app.emit(
                "script_result",
                serde_json::json!({ "success": success, "error": error }),
            );
        }
        DaemonMessage::BreakpointRulesUpdated { rules } => {
            let _ = app.emit("breakpoint_rules_updated", rules);
        }
        DaemonMessage::BreakpointHit {
            id,
            transaction_id,
            phase,
            data,
        } => {
            let _ = app.emit(
                "breakpoint_hit",
                serde_json::json!({ "id": id, "transaction_id": transaction_id, "phase": phase, "data": data }),
            );
        }
        DaemonMessage::HostMappingsUpdated { mappings } => {
            let _ = app.emit("host_mappings_updated", mappings);
        }
        DaemonMessage::SslProxyingListUpdated { entries } => {
            let _ = app.emit("ssl_proxying_list_updated", entries);
        }
        _ => {}
    }
}

/// hop-by-hop 헤더인지 대소문자 무시하고 확인
fn is_hop_by_hop_header(name: &str) -> bool {
    const HOP_BY_HOP: &[&str] = &[
        "host",
        "connection",
        "keep-alive",
        "proxy-authenticate",
        "proxy-authorization",
        "te",
        "trailers",
        "transfer-encoding",
        "upgrade",
    ];
    HOP_BY_HOP.iter().any(|h| h.eq_ignore_ascii_case(name))
}

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

    // [DIAG] Group F=true이지만 F=false와 동일하게 즉시 반환 (대조 실험)
    // 이게 멀쩡하면 → start_proxy_v2와 무관한 문제 (프론트엔드 or 타이밍)
    // 이게 터지면 → 재현이 비결정적이거나 다른 원인
    tracing::warn!("[DIAG-F-CONTROL] Group F=true이지만 즉시 반환");
    return Ok(ProxyStartResult {
        status: true,
        message: format!("[DIAG] 프록시 포트 {} (대조 실험, 연결 없음)", port),
    });
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
    tokio::task::spawn_blocking(move || {
        std::fs::read(&file_path).map_err(|e| format!("파일 읽기 실패: {} - {}", file_path, e))
    })
    .await
    .map_err(|e| format!("파일 읽기 태스크 실패: {}", e))?
}

/// 오래된 캐시 정리 명령어
#[tauri::command]
pub async fn clean_old_proxy_cache(days: u64) -> Result<String, String> {
    tokio::task::spawn_blocking(move || match clean_old_cache(days) {
        Ok(_) => Ok(format!(
            "{}일 이상 된 캐시가 성공적으로 정리되었습니다",
            days
        )),
        Err(e) => Err(format!("오래된 캐시 정리 실패: {}", e)),
    })
    .await
    .map_err(|e| format!("캐시 정리 태스크 실패: {}", e))?
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

/// WebSocket 메시지 주입 파라미터
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WsInjectParams {
    pub connection_id: String,
    pub direction: String,
    pub payload: String,
    pub is_binary: bool,
}

/// WebSocket 메시지 주입 (활성 연결에 메시지 전송)
#[tauri::command]
pub async fn ws_inject_message(
    proxy: State<'_, ProxyV2State>,
    params: WsInjectParams,
) -> Result<(), String> {
    let proxy_guard = proxy.lock().await;

    if let Some(conn) = proxy_guard.as_ref() {
        let cmd = ClientCommand::WsInject {
            connection_id: params.connection_id,
            direction: params.direction,
            payload: params.payload,
            is_binary: params.is_binary,
        };
        conn.send_command(&cmd).await?;
        Ok(())
    } else {
        Err("프록시가 실행 중이 아닙니다".to_string())
    }
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
        if is_hop_by_hop_header(key) {
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

/// 고급 반복 실행 파라미터
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdvancedRepeatParams {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub iterations: usize,
    pub concurrency: usize,
    pub delay_ms: u64,
}

/// 고급 반복 실행 진행 이벤트
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdvancedRepeatProgress {
    pub completed: usize,
    pub total: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub last_status: Option<u16>,
    pub last_elapsed_ms: Option<u64>,
}

/// 고급 반복 실행 최종 결과
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdvancedRepeatResult {
    pub total: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub min_time_ms: u64,
    pub max_time_ms: u64,
    pub avg_time_ms: f64,
    pub total_time_ms: u64,
    pub requests_per_second: f64,
    pub status_codes: HashMap<u16, usize>,
}

/// 고급 반복 실행 (N회 반복 + 동시성 제어)
#[tauri::command]
pub async fn advanced_repeat<R: Runtime>(
    app: AppHandle<R>,
    params: AdvancedRepeatParams,
) -> Result<AdvancedRepeatResult, String> {
    if !DIAG_ENABLE_ADVANCED_REPEAT {
        tracing::warn!("[DIAG] advanced_repeat 비활성화됨");
        return Ok(AdvancedRepeatResult {
            total: 0,
            success_count: 0,
            failure_count: 0,
            min_time_ms: 0,
            max_time_ms: 0,
            avg_time_ms: 0.0,
            total_time_ms: 0,
            requests_per_second: 0.0,
            status_codes: HashMap::new(),
        });
    }
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::Semaphore;

    let iterations = params.iterations.max(1).min(10000);
    let concurrency = params.concurrency.max(1).min(100);
    let delay_ms = params.delay_ms;

    let client = Arc::new(
        reqwest::Client::builder()
            .no_proxy()
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(|e| format!("HTTP 클라이언트 생성 실패: {}", e))?,
    );

    let method: reqwest::Method = params
        .method
        .parse()
        .map_err(|e| format!("잘못된 HTTP 메서드: {}", e))?;

    // hop-by-hop 헤더 필터링
    let filtered_headers: HashMap<String, String> = params
        .headers
        .into_iter()
        .filter(|(key, _)| !is_hop_by_hop_header(key))
        .collect();

    let semaphore = Arc::new(Semaphore::new(concurrency));
    let success_count = Arc::new(AtomicUsize::new(0));
    let failure_count = Arc::new(AtomicUsize::new(0));
    let completed = Arc::new(AtomicUsize::new(0));
    let elapsed_times = Arc::new(Mutex::new(Vec::with_capacity(iterations)));
    let status_codes = Arc::new(Mutex::new(HashMap::<u16, usize>::new()));

    let total_start = std::time::Instant::now();

    let mut handles = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| format!("세마포어 획득 실패: {}", e))?;

        let client = client.clone();
        let method = method.clone();
        let url = params.url.clone();
        let headers = filtered_headers.clone();
        let body = params.body.clone();
        let success_count = success_count.clone();
        let failure_count = failure_count.clone();
        let completed = completed.clone();
        let elapsed_times = elapsed_times.clone();
        let status_codes = status_codes.clone();
        let app = app.clone();
        let total = iterations;

        let handle = tokio::spawn(async move {
            let mut request_builder = client.request(method, &url);

            for (key, value) in &headers {
                request_builder = request_builder.header(key.as_str(), value.as_str());
            }

            if let Some(body) = body {
                request_builder = request_builder.body(body);
            }

            let start = std::time::Instant::now();
            let result = request_builder.send().await;
            let elapsed_ms = start.elapsed().as_millis() as u64;

            let (_is_success, status) = match result {
                Ok(response) => {
                    let status = response.status().as_u16();
                    // body를 소비하여 커넥션 반환
                    let _ = response.bytes().await;
                    let ok = (200..300).contains(&status);
                    if ok {
                        success_count.fetch_add(1, Ordering::Relaxed);
                    } else {
                        failure_count.fetch_add(1, Ordering::Relaxed);
                    }
                    // 상태 코드 카운트
                    {
                        let mut codes = status_codes.lock().await;
                        *codes.entry(status).or_insert(0) += 1;
                    }
                    (ok, Some(status))
                }
                Err(_) => {
                    failure_count.fetch_add(1, Ordering::Relaxed);
                    (false, None)
                }
            };

            {
                let mut times = elapsed_times.lock().await;
                times.push(elapsed_ms);
            }

            let done = completed.fetch_add(1, Ordering::Relaxed) + 1;

            // 진행 이벤트 전송
            let _ = app.emit(
                "advanced_repeat_progress",
                AdvancedRepeatProgress {
                    completed: done,
                    total,
                    success_count: success_count.load(Ordering::Relaxed),
                    failure_count: failure_count.load(Ordering::Relaxed),
                    last_status: status,
                    last_elapsed_ms: Some(elapsed_ms),
                },
            );

            drop(permit);

            // 배치 간 딜레이: permit 해제 후 딜레이를 적용하여 다음 요청 시작 전 대기
            if delay_ms > 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
            }
        });

        handles.push(handle);
    }

    // 모든 작업 완료 대기 (JoinError 발생 시 failure_count 증가)
    for handle in handles {
        if handle.await.is_err() {
            failure_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    let total_time_ms = total_start.elapsed().as_millis() as u64;
    let times = elapsed_times.lock().await;
    let codes = status_codes.lock().await;

    let min_time_ms = times.iter().copied().min().unwrap_or(0);
    let max_time_ms = times.iter().copied().max().unwrap_or(0);
    let avg_time_ms = if times.is_empty() {
        0.0
    } else {
        times.iter().sum::<u64>() as f64 / times.len() as f64
    };
    let requests_per_second = if total_time_ms > 0 {
        iterations as f64 / (total_time_ms as f64 / 1000.0)
    } else {
        0.0
    };

    Ok(AdvancedRepeatResult {
        total: iterations,
        success_count: success_count.load(Ordering::Relaxed),
        failure_count: failure_count.load(Ordering::Relaxed),
        min_time_ms,
        max_time_ms,
        avg_time_ms,
        total_time_ms,
        requests_per_second,
        status_codes: codes.clone(),
    })
}

/// Breakpoint 규칙 업데이트
#[tauri::command]
pub async fn update_breakpoint_rules(
    proxy: tauri::State<'_, ProxyV2State>,
    rules: Vec<BreakpointRule>,
) -> Result<(), String> {
    if !DIAG_ENABLE_NEW_SETTINGS {
        tracing::warn!("[DIAG] update_breakpoint_rules 비활성화됨");
        return Ok(());
    }
    let proxy_guard = proxy.lock().await;

    if let Some(conn) = proxy_guard.as_ref() {
        let cmd = ClientCommand::UpdateBreakpointRules { rules };
        conn.send_command(&cmd).await?;
        println!("Daemon에 breakpoint 규칙 업데이트 완료");
    } else {
        return Err("프록시가 실행 중이 아닙니다".to_string());
    }

    Ok(())
}

/// 대기 중인 breakpoint 해제
#[tauri::command]
pub async fn resolve_breakpoint(
    proxy: tauri::State<'_, ProxyV2State>,
    id: String,
    action: BreakpointAction,
) -> Result<(), String> {
    if !DIAG_ENABLE_NEW_SETTINGS {
        tracing::warn!("[DIAG] resolve_breakpoint 비활성화됨");
        return Ok(());
    }
    let proxy_guard = proxy.lock().await;

    if let Some(conn) = proxy_guard.as_ref() {
        let cmd = ClientCommand::ResolveBreakpoint { id, action };
        conn.send_command(&cmd).await?;
        println!("Daemon에 breakpoint 해제 완료");
    } else {
        return Err("프록시가 실행 중이 아닙니다".to_string());
    }

    Ok(())
}

/// 앱 번들 내부의 sidecar 바이너리를 ~/.cheolsu/bin/에 복사하고 격리 속성 제거
/// macOS에서 Gatekeeper가 앱 번들 내부 바이너리의 외부 실행을 차단하는 문제 해결
fn install_sidecar_binary(
    app: &AppHandle<impl Runtime>,
    sidecar_base: &str,
    dest_name: &str,
) -> Result<String, String> {
    use tauri::Manager;

    let current_exe =
        std::env::current_exe().map_err(|e| format!("Failed to get current exe: {}", e))?;
    let exe_dir = current_exe
        .parent()
        .ok_or_else(|| "실행 파일의 부모 디렉토리를 찾을 수 없습니다".to_string())?;

    // 개발 모드: target 디렉토리에서 직접 사용
    if cfg!(dev) {
        let bin_path = exe_dir.join(dest_name);
        if bin_path.exists() {
            return Ok(bin_path.display().to_string());
        }
    }

    // 프로덕션 모드: 앱 번들(Contents/MacOS/)에서 ~/.cheolsu/bin/으로 복사
    // Tauri는 번들링 시 target triple 접미사를 제거하므로 sidecar_base 그대로 사용
    let source = exe_dir.join(sidecar_base);

    if !source.exists() {
        return Err(format!(
            "Sidecar 바이너리가 존재하지 않습니다: {}",
            source.display()
        ));
    }

    // ~/.cheolsu/bin/ 디렉토리 생성
    let home = app
        .path()
        .home_dir()
        .map_err(|e| format!("홈 디렉토리를 찾을 수 없습니다: {}", e))?;
    let bin_dir = home.join(".cheolsu").join("bin");
    std::fs::create_dir_all(&bin_dir).map_err(|e| format!("디렉토리 생성 실패: {}", e))?;

    let dest = bin_dir.join(dest_name);

    // 소스와 대상이 같은지 확인 (이미 설치됨 + 동일 버전)
    let needs_copy = if dest.exists() {
        let src_meta = std::fs::metadata(&source).ok();
        let dst_meta = std::fs::metadata(&dest).ok();
        match (src_meta, dst_meta) {
            (Some(s), Some(d)) => s.len() != d.len(),
            _ => true,
        }
    } else {
        true
    };

    if needs_copy {
        std::fs::copy(&source, &dest).map_err(|e| format!("바이너리 복사 실패: {}", e))?;
    }

    // macOS: 격리 속성 제거 + 실행 권한 설정
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("실행 권한 설정 실패: {}", e))?;
    }
    #[cfg(target_os = "macos")]
    {
        // Gatekeeper 격리 속성 제거 (다운로드된 앱 번들에서 복사된 경우 필요)
        let _ = std::process::Command::new("xattr")
            .args(["-cr", &dest.display().to_string()])
            .output();
    }

    Ok(dest.display().to_string())
}

/// MCP 서버 바이너리의 절대 경로 반환 (앱 번들에서 ~/.cheolsu/bin/으로 자동 설치)
#[tauri::command]
pub fn get_mcp_server_path(app: AppHandle<impl Runtime>) -> Result<String, String> {
    install_sidecar_binary(&app, "cheolsu-proxy-mcp", "cheolsu-proxy-mcp")
}

/// Upstream proxy 설정 업데이트
#[tauri::command]
pub async fn update_upstream_proxy(
    proxy: State<'_, ProxyV2State>,
    config: Option<UpstreamProxyConfig>,
) -> Result<(), String> {
    let proxy_guard = proxy.lock().await;

    if let Some(conn) = proxy_guard.as_ref() {
        let cmd = ClientCommand::UpdateUpstreamProxy { config };
        conn.send_command(&cmd).await?;
        tracing::info!("Daemon에 upstream proxy 설정 업데이트 완료");
    } else {
        return Err("프록시가 실행 중이 아닙니다".to_string());
    }

    Ok(())
}

/// 프록시 인증 설정 업데이트
#[tauri::command]
pub async fn update_proxy_auth(
    proxy: State<'_, ProxyV2State>,
    config: ProxyAuthConfig,
) -> Result<(), String> {
    if !DIAG_ENABLE_NEW_SETTINGS {
        tracing::warn!("[DIAG] update_proxy_auth 비활성화됨");
        return Ok(());
    }
    let proxy_guard = proxy.lock().await;

    if let Some(conn) = proxy_guard.as_ref() {
        let cmd = ClientCommand::UpdateProxyAuth { config };
        conn.send_command(&cmd).await?;
        tracing::info!("Daemon에 프록시 인증 설정 업데이트 완료");
    } else {
        return Err("프록시가 실행 중이 아닙니다".to_string());
    }

    Ok(())
}

/// 스로틀링 설정 업데이트
#[tauri::command]
pub async fn update_throttle(
    proxy: State<'_, ProxyV2State>,
    config: Option<ThrottleConfig>,
) -> Result<(), String> {
    let proxy_guard = proxy.lock().await;

    if let Some(conn) = proxy_guard.as_ref() {
        let cmd = ClientCommand::UpdateThrottle { config };
        conn.send_command(&cmd).await?;
        tracing::info!("Daemon에 스로틀링 설정 업데이트 완료");
    } else {
        return Err("프록시가 실행 중이 아닙니다".to_string());
    }

    Ok(())
}

/// 서버 리플레이 엔트리 업데이트
#[tauri::command]
pub async fn update_server_replay(
    proxy: State<'_, ProxyV2State>,
    entries: Vec<ServerReplayEntry>,
) -> Result<(), String> {
    let proxy_guard = proxy.lock().await;

    if let Some(conn) = proxy_guard.as_ref() {
        let cmd = ClientCommand::UpdateServerReplay { entries };
        conn.send_command(&cmd).await?;
        tracing::info!("Daemon에 서버 리플레이 엔트리 업데이트 완료");
    } else {
        return Err("프록시가 실행 중이 아닙니다".to_string());
    }

    Ok(())
}

/// 호스트 매핑 업데이트
#[tauri::command]
pub async fn update_host_mappings(
    proxy: State<'_, ProxyV2State>,
    mappings: Vec<HostMapping>,
) -> Result<(), String> {
    let proxy_guard = proxy.lock().await;

    if let Some(conn) = proxy_guard.as_ref() {
        let cmd = ClientCommand::UpdateHostMappings { mappings };
        conn.send_command(&cmd).await?;
        tracing::info!("Daemon에 호스트 매핑 업데이트 완료");
    } else {
        return Err("프록시가 실행 중이 아닙니다".to_string());
    }

    Ok(())
}

/// SSL Proxying 화이트리스트 업데이트
#[tauri::command]
pub async fn update_ssl_proxying_list(
    proxy: State<'_, ProxyV2State>,
    entries: Vec<SslProxyingEntry>,
) -> Result<(), String> {
    if !DIAG_ENABLE_NEW_SETTINGS {
        tracing::warn!("[DIAG] update_ssl_proxying_list 비활성화됨");
        return Ok(());
    }
    let proxy_guard = proxy.lock().await;

    if let Some(conn) = proxy_guard.as_ref() {
        let cmd = ClientCommand::UpdateSslProxyingList { entries };
        conn.send_command(&cmd).await?;
        tracing::info!("Daemon에 SSL Proxying 화이트리스트 업데이트 완료");
    } else {
        return Err("프록시가 실행 중이 아닙니다".to_string());
    }

    Ok(())
}

/// 클라이언트 인증서 설정 업데이트 (mTLS)
#[tauri::command]
pub async fn update_client_certificate(
    proxy: State<'_, ProxyV2State>,
    config: Option<proxy_daemon::ClientCertConfig>,
) -> Result<(), String> {
    if !DIAG_ENABLE_NEW_SETTINGS {
        tracing::warn!("[DIAG] update_client_certificate 비활성화됨");
        return Ok(());
    }
    // 설정이 활성화된 경우 유효성 검증 (blocking I/O를 spawn_blocking으로 처리)
    if let Some(ref cert_config) = config {
        if cert_config.enabled {
            let cert_config_clone = cert_config.clone();
            tokio::task::spawn_blocking(move || {
                proxy_daemon::validate_client_cert_config(&cert_config_clone)
                    .map_err(|e| format!("인증서 검증 실패: {}", e))
            })
            .await
            .map_err(|e| format!("검증 태스크 실패: {}", e))??;
        }
    }

    let proxy_guard = proxy.lock().await;

    if let Some(conn) = proxy_guard.as_ref() {
        let cmd = ClientCommand::UpdateClientCertificate { config };
        conn.send_command(&cmd).await?;
        tracing::info!("Daemon에 클라이언트 인증서 설정 업데이트 완료");
    } else {
        return Err("프록시가 실행 중이 아닙니다".to_string());
    }

    Ok(())
}

/// 빠른 설정 업데이트 (No Caching, Block Cookies, No Gzip)
#[tauri::command]
pub async fn update_quick_settings(
    proxy: State<'_, ProxyV2State>,
    no_caching: bool,
    block_cookies: bool,
    no_gzip: bool,
) -> Result<(), String> {
    if !DIAG_ENABLE_NEW_SETTINGS {
        tracing::warn!("[DIAG] update_quick_settings 비활성화됨");
        return Ok(());
    }
    let proxy_guard = proxy.lock().await;

    if let Some(conn) = proxy_guard.as_ref() {
        let cmd = ClientCommand::UpdateQuickSettings {
            no_caching,
            block_cookies,
            no_gzip,
        };
        conn.send_command(&cmd).await?;
        tracing::info!(
            "Daemon에 빠른 설정 업데이트 완료: no_caching={}, block_cookies={}, no_gzip={}",
            no_caching,
            block_cookies,
            no_gzip
        );
    } else {
        return Err("프록시가 실행 중이 아닙니다".to_string());
    }

    Ok(())
}

/// 스크립트 로드
#[tauri::command]
pub async fn load_script(
    proxy: State<'_, ProxyV2State>,
    path: Option<String>,
    code: Option<String>,
) -> Result<(), String> {
    let proxy_guard = proxy.lock().await;

    if let Some(conn) = proxy_guard.as_ref() {
        let cmd = ClientCommand::LoadScript { path, code };
        conn.send_command(&cmd).await?;
        tracing::info!("Daemon에 스크립트 로드 요청 완료");
    } else {
        return Err("프록시가 실행 중이 아닙니다".to_string());
    }

    Ok(())
}

/// 스크립트 언로드
#[tauri::command]
pub async fn unload_script(proxy: State<'_, ProxyV2State>) -> Result<(), String> {
    let proxy_guard = proxy.lock().await;

    if let Some(conn) = proxy_guard.as_ref() {
        let cmd = ClientCommand::UnloadScript;
        conn.send_command(&cmd).await?;
        tracing::info!("Daemon에 스크립트 언로드 요청 완료");
    } else {
        return Err("프록시가 실행 중이 아닙니다".to_string());
    }

    Ok(())
}

/// macOS에서 osascript를 사용하여 관리자 권한으로 셸 명령 실행
/// 네이티브 비밀번호 입력 팝업이 표시됨 (VS Code 방식)
#[cfg(target_os = "macos")]
fn run_with_admin_privileges(shell_cmd: &str) -> Result<(), String> {
    let script = format!(
        r#"do shell script "{}" with administrator privileges"#,
        shell_cmd.replace('\\', "\\\\").replace('"', "\\\"")
    );
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| format!("osascript 실행 실패: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("User canceled") || stderr.contains("-128") {
            Err("사용자가 취소했습니다".to_string())
        } else {
            Err(format!("관리자 권한 명령 실패: {}", stderr.trim()))
        }
    }
}

/// 터미널 명령어(cheolsu) 설치: ~/.cheolsu/bin/에 복사 후 /usr/local/bin/cheolsu에 심볼릭 링크 생성
#[tauri::command]
pub fn install_cli(app: AppHandle<impl Runtime>) -> Result<String, String> {
    // 앱 번들에서 ~/.cheolsu/bin/cheolsu로 복사 (격리 속성 제거 포함)
    let tui_path = install_sidecar_binary(&app, "cheolsu", "cheolsu")?;
    let link_path = "/usr/local/bin/cheolsu";
    let link = std::path::Path::new(link_path);

    // 기존 링크/파일 제거
    let needs_admin = if link.exists() || link.is_symlink() {
        std::fs::remove_file(link).is_err()
    } else {
        false
    };

    #[cfg(unix)]
    {
        if needs_admin || std::os::unix::fs::symlink(&tui_path, link).is_err() {
            #[cfg(target_os = "macos")]
            {
                let cmd = format!("rm -f {} && ln -sf {} {}", link_path, tui_path, link_path);
                run_with_admin_privileges(&cmd)?;
            }

            #[cfg(not(target_os = "macos"))]
            {
                return Err("심볼릭 링크 생성 실패: sudo 권한이 필요합니다".to_string());
            }
        }
    }

    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(&tui_path, link)
            .map_err(|e| format!("심볼릭 링크 생성 실패: {}", e))?;
    }

    Ok(format!(
        "터미널 명령어가 설치되었습니다: {} -> {}",
        link_path, tui_path
    ))
}

/// 터미널 명령어(cheolsu) 제거: /usr/local/bin/cheolsu 심볼릭 링크 삭제
#[tauri::command]
pub fn uninstall_cli() -> Result<String, String> {
    let link_path = "/usr/local/bin/cheolsu";
    let link = std::path::Path::new(link_path);

    if !link.exists() && !link.is_symlink() {
        return Err("터미널 명령어가 설치되어 있지 않습니다".to_string());
    }

    if std::fs::remove_file(link).is_err() {
        #[cfg(target_os = "macos")]
        {
            let cmd = format!("rm -f {}", link_path);
            run_with_admin_privileges(&cmd)?;
        }

        #[cfg(not(target_os = "macos"))]
        {
            return Err("제거 실패: sudo 권한이 필요합니다".to_string());
        }
    }

    Ok("터미널 명령어가 제거되었습니다".to_string())
}

/// 터미널 명령어 설치 상태 확인
#[tauri::command]
pub fn check_cli_installed() -> bool {
    let link_path = std::path::Path::new("/usr/local/bin/cheolsu");
    link_path.exists()
}

/// CA 인증서 저장 디렉토리를 반환합니다.
fn get_ca_storage_dir() -> Result<std::path::PathBuf, String> {
    #[cfg(target_os = "macos")]
    {
        let home =
            std::env::var("HOME").map_err(|_| "HOME 환경 변수를 찾을 수 없습니다".to_string())?;
        let dir = std::path::PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("com.cheolsu-proxy");
        std::fs::create_dir_all(&dir).map_err(|e| format!("디렉토리 생성 실패: {}", e))?;
        Ok(dir)
    }

    #[cfg(target_os = "windows")]
    {
        let local_app_data = std::env::var("LOCALAPPDATA")
            .map_err(|_| "LOCALAPPDATA 환경 변수를 찾을 수 없습니다".to_string())?;
        let dir = std::path::PathBuf::from(local_app_data).join("com.cheolsu-proxy");
        std::fs::create_dir_all(&dir).map_err(|e| format!("디렉토리 생성 실패: {}", e))?;
        Ok(dir)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("현재 macOS와 Windows만 지원합니다".to_string())
    }
}

/// CA 인증서 경로를 반환합니다.
#[tauri::command]
pub fn get_ca_cert_path() -> Result<String, String> {
    let storage_dir = get_ca_storage_dir()?;
    let cer_path = storage_dir.join("cheolsu-proxy.cer");
    if cer_path.exists() {
        Ok(cer_path.to_string_lossy().to_string())
    } else {
        Err("CA 인증서가 아직 생성되지 않았습니다. 프록시를 먼저 실행해주세요.".to_string())
    }
}

/// CA 인증서가 시스템에 신뢰 설치되어 있는지 확인합니다.
#[tauri::command]
pub fn check_ca_installed() -> Result<bool, String> {
    let storage_dir = get_ca_storage_dir()?;
    let cer_path = storage_dir.join("cheolsu-proxy.cer");

    if !cer_path.exists() {
        return Ok(false);
    }

    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("security")
            .args(["find-certificate", "-c", "Cheolsu Proxy", "-Z"])
            .output()
            .map_err(|e| format!("security 명령 실행 실패: {}", e))?;

        Ok(output.status.success()
            && String::from_utf8_lossy(&output.stdout).contains("Cheolsu Proxy"))
    }

    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("certutil")
            .args(["-verifystore", "Root", "Cheolsu Proxy Root CA"])
            .output()
            .map_err(|e| format!("certutil 명령 실행 실패: {}", e))?;

        Ok(output.status.success())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Ok(false)
    }
}

/// CA 인증서를 시스템에 신뢰 인증서로 설치합니다.
#[tauri::command]
pub fn install_ca_cert() -> Result<String, String> {
    let storage_dir = get_ca_storage_dir()?;
    let cer_path = storage_dir.join("cheolsu-proxy.cer");

    if !cer_path.exists() {
        return Err(
            "CA 인증서가 아직 생성되지 않았습니다. 프록시를 먼저 실행해주세요.".to_string(),
        );
    }

    #[cfg(target_os = "macos")]
    {
        let keychain_path = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join("Library/Keychains/login.keychain-db");

        // 1단계: login 키체인에 인증서 추가 (관리자 권한 불필요)
        let add_output = std::process::Command::new("security")
            .args(["add-certificates", "-k"])
            .arg(&keychain_path)
            .arg(&cer_path)
            .output()
            .map_err(|e| format!("security 명령 실행 실패: {}", e))?;

        // 이미 존재하는 경우는 무시 (-25299 에러코드 또는 "already in" 메시지)
        if !add_output.status.success() {
            let stderr = String::from_utf8_lossy(&add_output.stderr);
            if !stderr.contains("-25299") && !stderr.contains("already in") {
                return Err(format!("키체인에 인증서 추가 실패: {}", stderr.trim()));
            }
        }

        // 2단계: 인증서 신뢰 설정 (사용자 도메인, 관리자 권한 불필요)
        let trust_output = std::process::Command::new("security")
            .args(["add-trusted-cert", "-p", "ssl", "-k"])
            .arg(&keychain_path)
            .arg(&cer_path)
            .output()
            .map_err(|e| format!("security 명령 실행 실패: {}", e))?;

        if trust_output.status.success() {
            Ok("CA 인증서가 키체인에 신뢰 인증서로 설치되었습니다.".to_string())
        } else {
            let stderr = String::from_utf8_lossy(&trust_output.stderr);
            Err(format!("인증서 신뢰 설정 실패: {}", stderr.trim()))
        }
    }

    #[cfg(target_os = "windows")]
    {
        let cer_path_str = cer_path.to_string_lossy().to_string();
        let output = std::process::Command::new("certutil")
            .args(["-addstore", "-user", "Root", &cer_path_str])
            .output()
            .map_err(|e| format!("certutil 실행 실패: {}", e))?;

        if output.status.success() {
            Ok("CA 인증서가 신뢰할 수 있는 루트 인증 기관에 설치되었습니다.".to_string())
        } else {
            Err(format!(
                "인증서 설치 실패: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("현재 이 OS에서는 자동 설치를 지원하지 않습니다.".to_string())
    }
}

/// 시스템에서 CA 인증서를 제거합니다.
#[tauri::command]
pub fn uninstall_ca_cert() -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        let keychain_path = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join("Library/Keychains/login.keychain-db");

        // login 키체인에서 인증서 삭제
        let output = std::process::Command::new("security")
            .args(["delete-certificate", "-c", "Cheolsu Proxy Root CA", "-t"])
            .arg(&keychain_path)
            .output()
            .map_err(|e| format!("security 명령 실행 실패: {}", e))?;

        if output.status.success() {
            Ok("CA 인증서가 키체인에서 제거되었습니다.".to_string())
        } else {
            // CN이 다를 수 있으므로 "Cheolsu Proxy"로도 시도
            let output2 = std::process::Command::new("security")
                .args(["delete-certificate", "-c", "Cheolsu Proxy", "-t"])
                .arg(&keychain_path)
                .output()
                .map_err(|e| format!("security 명령 실행 실패: {}", e))?;

            if output2.status.success() {
                Ok("CA 인증서가 키체인에서 제거되었습니다.".to_string())
            } else {
                Err("키체인에서 인증서를 찾을 수 없습니다.".to_string())
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("certutil")
            .args(["-delstore", "-user", "Root", "Cheolsu Proxy Root CA"])
            .output()
            .map_err(|e| format!("certutil 실행 실패: {}", e))?;

        if output.status.success() {
            Ok("CA 인증서가 제거되었습니다.".to_string())
        } else {
            Err(format!(
                "인증서 제거 실패: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("현재 이 OS에서는 자동 제거를 지원하지 않습니다.".to_string())
    }
}

/// 인증서 다운로드 정보 (URL + QR 코드)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CertDownloadInfo {
    /// 프록시 포트
    pub port: u16,
    /// 로컬 네트워크 IP 주소 목록
    pub local_ips: Vec<String>,
    /// 인증서 다운로드 URL (http://cheolsu.proxy/ssl)
    pub download_url: String,
    /// 직접 접속 가능한 URL (http://{ip}:{port} 경유 필요)
    pub direct_url: String,
    /// QR 코드 PNG 이미지 (base64 인코딩)
    pub qr_code_base64: String,
}

/// QR 코드를 PNG base64 문자열로 생성합니다.
fn generate_qr_code_base64(data: &str) -> Result<String, String> {
    use image::Luma;
    use qrcode::QrCode;

    let code = QrCode::new(data.as_bytes()).map_err(|e| format!("QR 코드 생성 실패: {}", e))?;

    let image = code.render::<Luma<u8>>().quiet_zone(true).build();

    let mut png_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_bytes);
    image
        .write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| format!("PNG 인코딩 실패: {}", e))?;

    use base64::Engine;
    Ok(base64::engine::general_purpose::STANDARD.encode(&png_bytes))
}

/// 외부 기기용 인증서 다운로드 정보를 반환합니다.
/// QR 코드에는 프록시 경유 인증서 다운로드 URL이 포함됩니다.
#[tauri::command]
pub fn get_cert_download_info(port: u16) -> Result<CertDownloadInfo, String> {
    let local_ips = get_local_ips();
    let download_url = "http://cheolsu.proxy/ssl".to_string();

    let primary_ip = local_ips
        .first()
        .cloned()
        .unwrap_or("127.0.0.1".to_string());
    let direct_url = format!("http://cheolsu.proxy/ssl (proxy: {}:{})", primary_ip, port);

    // QR 코드에는 직접 접속 가능한 URL을 포함 (모바일에서 스캔 시 바로 열림)
    let qr_content = format!("http://{}:{}/ssl", primary_ip, port);
    let qr_code_base64 = generate_qr_code_base64(&qr_content)?;

    Ok(CertDownloadInfo {
        port,
        local_ips,
        download_url,
        direct_url,
        qr_code_base64,
    })
}

/// 두 트랜잭션의 diff 비교를 위한 파라미터
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiffTransactionData {
    pub method: Option<String>,
    pub uri: Option<String>,
    pub status: Option<u16>,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub body_size: usize,
    pub data_type: Option<String>,
}

/// DiffTransactionData 두 개를 비교하여 TransactionPartDiff를 생성하는 헬퍼 함수.
/// `is_request`가 true이면 method/url을 비교하고, false이면 status를 비교합니다.
fn diff_transaction_part(
    a: &DiffTransactionData,
    b: &DiffTransactionData,
    is_request: bool,
) -> Option<TransactionPartDiff> {
    let method_diff = if is_request {
        match (&a.method, &b.method) {
            (Some(ma), Some(mb)) if ma != mb => Some((ma.clone(), mb.clone())),
            _ => None,
        }
    } else {
        None
    };

    let url_diff = if is_request {
        match (&a.uri, &b.uri) {
            (Some(ua), Some(ub)) if ua != ub => Some((ua.clone(), ub.clone())),
            _ => None,
        }
    } else {
        None
    };

    let status_diff = if !is_request {
        match (a.status, b.status) {
            (Some(sa), Some(sb)) if sa != sb => Some((sa, sb)),
            _ => None,
        }
    } else {
        None
    };

    let header_diffs = diff_headers(&a.headers, &b.headers);

    let body_diff = compute_body_diff_from_strings(
        a.body.as_deref(),
        b.body.as_deref(),
        a.body_size,
        b.body_size,
        a.data_type.as_deref(),
        b.data_type.as_deref(),
    );

    if method_diff.is_none()
        && url_diff.is_none()
        && status_diff.is_none()
        && header_diffs.is_empty()
        && body_diff.is_none()
    {
        None
    } else {
        Some(TransactionPartDiff {
            method_diff,
            url_diff,
            status_diff,
            header_diffs,
            body_diff,
        })
    }
}

/// 두 트랜잭션 비교 결과 반환
/// request 부분은 method/url/headers/body를 비교하고,
/// response 부분은 status/headers/body를 모두 비교합니다.
#[tauri::command]
pub async fn diff_transactions(
    transaction_a: DiffTransactionData,
    transaction_b: DiffTransactionData,
) -> Result<TrafficDiff, String> {
    let request_diff = diff_transaction_part(&transaction_a, &transaction_b, true);
    let response_diff = diff_transaction_part(&transaction_a, &transaction_b, false);

    Ok(TrafficDiff {
        request_diff,
        response_diff,
    })
}

/// 두 트랜잭션의 전체 비교 (request + response 모두 포함)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiffTransactionPair {
    pub request: Option<DiffTransactionData>,
    pub response: Option<DiffTransactionData>,
}

/// 두 트랜잭션의 전체 비교 결과 반환
#[tauri::command]
pub async fn diff_transaction_pairs(
    pair_a: DiffTransactionPair,
    pair_b: DiffTransactionPair,
) -> Result<TrafficDiff, String> {
    let request_diff = match (&pair_a.request, &pair_b.request) {
        (Some(req_a), Some(req_b)) => diff_transaction_part(req_a, req_b, true),
        _ => None,
    };

    let response_diff = match (&pair_a.response, &pair_b.response) {
        (Some(res_a), Some(res_b)) => diff_transaction_part(res_a, res_b, false),
        _ => None,
    };

    Ok(TrafficDiff {
        request_diff,
        response_diff,
    })
}

fn compute_body_diff_from_strings(
    body_a: Option<&str>,
    body_b: Option<&str>,
    size_a: usize,
    size_b: usize,
    data_type_a: Option<&str>,
    data_type_b: Option<&str>,
) -> Option<BodyDiff> {
    let text_a = body_a.unwrap_or("");
    let text_b = body_b.unwrap_or("");

    if text_a == text_b {
        return None;
    }

    let is_json = matches!(data_type_a, Some("Json" | "GraphQL"))
        && matches!(data_type_b, Some("Json" | "GraphQL"));

    if is_json {
        if let (Ok(json_a), Ok(json_b)) = (
            serde_json::from_str::<serde_json::Value>(text_a),
            serde_json::from_str::<serde_json::Value>(text_b),
        ) {
            return Some(diff_json(&json_a, &json_b));
        }
    }

    let is_text = data_type_a.map(|t| is_text_data_type(t)).unwrap_or(true)
        && data_type_b.map(|t| is_text_data_type(t)).unwrap_or(true);

    if is_text && !text_a.is_empty() && !text_b.is_empty() {
        return Some(diff_text(text_a, text_b));
    }

    Some(BodyDiff::Binary {
        old_size: size_a,
        new_size: size_b,
    })
}

/// HAR 파일 내보내기 (지정된 경로에 JSON 문자열 저장)
#[tauri::command]
pub async fn export_har_file(path: String, content: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        std::fs::write(&path, content).map_err(|e| format!("HAR 파일 저장 실패: {} - {}", path, e))
    })
    .await
    .map_err(|e| format!("HAR 저장 태스크 실패: {}", e))?
}

/// 세션 저장: 프론트엔드에서 전달받은 트랜잭션 데이터를 .cheolsu 파일로 직접 저장
#[tauri::command]
pub async fn save_session(path: String, transactions_json: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        use proxy_daemon::RequestInfo;
        use proxy_daemon::SessionFile;

        let file_path = proxy_daemon::ensure_extension(&path);

        let transactions: Vec<RequestInfo> = serde_json::from_str(&transactions_json)
            .map_err(|e| format!("트랜잭션 역직렬화 실패: {}", e))?;

        let session = SessionFile::from_traffic(0, &transactions, &[], &[], &[], None);
        session
            .save(std::path::Path::new(&file_path))
            .map_err(|e| format!("세션 저장 실패: {}", e))?;

        Ok(())
    })
    .await
    .map_err(|e| format!("세션 저장 태스크 실패: {}", e))?
}

/// 세션 로드 결과 (트랜잭션 데이터 포함)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoadSessionResult {
    pub name: Option<String>,
    pub description: Option<String>,
    pub transaction_count: usize,
    pub transactions_json: String,
}

/// 세션 불러오기: .cheolsu 파일에서 트래픽 로드하여 트랜잭션 데이터 반환
#[tauri::command]
pub async fn load_session(path: String) -> Result<LoadSessionResult, String> {
    tokio::task::spawn_blocking(move || {
        use proxy_daemon::SessionFile;

        let session = SessionFile::load(std::path::Path::new(&path))
            .map_err(|e| format!("세션 로드 실패: {}", e))?;

        let transactions = session.extract_transactions();
        let transactions_json = serde_json::to_string(&transactions)
            .map_err(|e| format!("트랜잭션 직렬화 실패: {}", e))?;

        Ok(LoadSessionResult {
            name: session.metadata.name,
            description: session.metadata.description,
            transaction_count: session.transactions.len(),
            transactions_json,
        })
    })
    .await
    .map_err(|e| format!("세션 로드 태스크 실패: {}", e))?
}

/// HAR 파일 가져오기: HAR 파일에서 트래픽을 읽어서 트랜잭션 데이터 반환
#[tauri::command]
pub async fn import_har_file_cmd(path: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let transactions = proxy_daemon::import_har_file(std::path::Path::new(&path))
            .map_err(|e| format!("HAR 가져오기 실패: {}", e))?;

        serde_json::to_string(&transactions).map_err(|e| format!("트랜잭션 직렬화 실패: {}", e))
    })
    .await
    .map_err(|e| format!("HAR 가져오기 태스크 실패: {}", e))?
}

/// app_data_dir 기반 자동 저장 파일 경로 생성 (테스트 가능한 순수 함수)
pub fn build_autosave_path(data_dir: &std::path::Path) -> std::path::PathBuf {
    data_dir.join("autosave.cheolsu.gz")
}

/// 자동 저장 경로 생성: app_data_dir/autosave.cheolsu.gz
pub fn get_autosave_path(app: &AppHandle<impl Runtime>) -> Result<std::path::PathBuf, String> {
    use tauri::Manager;
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("앱 데이터 디렉토리 조회 실패: {}", e))?;
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("앱 데이터 디렉토리 생성 실패: {}", e))?;
    Ok(build_autosave_path(&data_dir))
}

/// 자동 세션 저장: 현재 트랜잭션을 app_data_dir/autosave.cheolsu.gz에 저장
#[tauri::command]
pub async fn autosave_session(
    app: AppHandle<impl Runtime>,
    transactions_json: String,
) -> Result<(), String> {
    if !DIAG_ENABLE_AUTO_SESSION {
        tracing::warn!("[DIAG] autosave_session 비활성화됨");
        return Ok(());
    }
    let file_path = get_autosave_path(&app)?;

    tokio::task::spawn_blocking(move || {
        use proxy_daemon::RequestInfo;
        use proxy_daemon::SessionFile;

        let transactions: Vec<RequestInfo> = serde_json::from_str(&transactions_json)
            .map_err(|e| format!("트랜잭션 역직렬화 실패: {}", e))?;

        let session = SessionFile::from_traffic(0, &transactions, &[], &[], &[], None);
        session
            .save(&file_path)
            .map_err(|e| format!("자동 세션 저장 실패: {}", e))?;

        tracing::info!("자동 세션 저장 완료: {:?}", file_path);
        Ok(())
    })
    .await
    .map_err(|e| format!("자동 세션 저장 태스크 실패: {}", e))?
}

/// 자동 세션 복원: app_data_dir/autosave.cheolsu.gz에서 세션 로드
#[tauri::command]
pub async fn autoload_session(
    app: AppHandle<impl Runtime>,
) -> Result<Option<LoadSessionResult>, String> {
    if !DIAG_ENABLE_AUTO_SESSION {
        tracing::warn!("[DIAG] autoload_session 비활성화됨");
        return Ok(None);
    }
    let file_path = get_autosave_path(&app)?;

    tokio::task::spawn_blocking(move || {
        use proxy_daemon::SessionFile;

        if !file_path.exists() {
            return Ok(None);
        }

        match SessionFile::load(&file_path) {
            Ok(session) => {
                let transactions = session.extract_transactions();
                let transactions_json = serde_json::to_string(&transactions)
                    .map_err(|e| format!("트랜잭션 직렬화 실패: {}", e))?;

                tracing::info!(
                    "자동 세션 복원 완료: {} 트랜잭션",
                    session.transactions.len()
                );

                Ok(Some(LoadSessionResult {
                    name: session.metadata.name,
                    description: session.metadata.description,
                    transaction_count: session.transactions.len(),
                    transactions_json,
                }))
            }
            Err(e) => {
                tracing::warn!("자동 세션 복원 실패 (무시): {}", e);
                Ok(None)
            }
        }
    })
    .await
    .map_err(|e| format!("세션 복원 태스크 실패: {}", e))?
}

fn base64_engine() -> base64::engine::GeneralPurpose {
    use base64::engine::general_purpose::STANDARD;
    STANDARD
}

fn base64_encode(engine: &base64::engine::GeneralPurpose, data: &[u8]) -> String {
    use base64::Engine;
    engine.encode(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_qr_code_base64_returns_valid_base64() {
        let result = generate_qr_code_base64("http://192.168.1.1:8100/ssl");
        assert!(result.is_ok(), "QR 코드 생성 실패: {:?}", result.err());

        let base64_str = result.unwrap();
        assert!(!base64_str.is_empty(), "base64 문자열이 비어있음");

        // 유효한 base64인지 확인
        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD.decode(&base64_str);
        assert!(decoded.is_ok(), "유효한 base64가 아님");

        // PNG 매직 바이트 확인
        let bytes = decoded.unwrap();
        assert!(bytes.len() > 8, "PNG 데이터가 너무 짧음");
        assert_eq!(&bytes[..4], b"\x89PNG", "PNG 매직 바이트가 아님");
    }

    #[test]
    fn generate_qr_code_base64_contains_logo() {
        // 로고가 합성되면 이미지 크기가 로고 없는 것보다 커야 함
        let result = generate_qr_code_base64("http://test.local/ssl");
        assert!(result.is_ok());

        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(result.unwrap())
            .unwrap();
        // 로고가 합성된 QR코드 PNG는 일반적으로 수 KB 이상
        assert!(
            bytes.len() > 1000,
            "로고가 합성된 QR코드가 너무 작음: {} bytes",
            bytes.len()
        );
    }

    #[test]
    fn get_cert_download_info_returns_valid_info() {
        let result = get_cert_download_info(8100);
        assert!(result.is_ok(), "인증서 다운로드 정보 생성 실패");

        let info = result.unwrap();
        assert_eq!(info.port, 8100);
        assert_eq!(info.download_url, "http://cheolsu.proxy/ssl");
        assert!(!info.local_ips.is_empty(), "로컬 IP가 비어있음");
        assert!(!info.qr_code_base64.is_empty(), "QR 코드가 비어있음");
        assert!(info.direct_url.contains("cheolsu.proxy/ssl"));
    }

    #[test]
    fn cert_download_info_struct_fields() {
        let info = CertDownloadInfo {
            port: 9090,
            local_ips: vec!["192.168.1.1".to_string()],
            download_url: "http://cheolsu.proxy/ssl".to_string(),
            direct_url: "http://cheolsu.proxy/ssl (proxy: 192.168.1.1:9090)".to_string(),
            qr_code_base64: "dGVzdA==".to_string(),
        };
        assert_eq!(info.port, 9090);
        assert_eq!(info.local_ips.len(), 1);
    }

    #[test]
    fn build_autosave_path_creates_correct_path() {
        let data_dir = std::path::Path::new("/tmp/cheolsu-proxy");
        let path = build_autosave_path(data_dir);
        assert_eq!(
            path,
            std::path::PathBuf::from("/tmp/cheolsu-proxy/autosave.cheolsu.gz")
        );
    }

    #[test]
    fn build_autosave_path_with_various_dirs() {
        // 일반 경로
        let path = build_autosave_path(std::path::Path::new("/home/user/.local/share/cheolsu"));
        assert!(path.to_string_lossy().ends_with("autosave.cheolsu.gz"));
        assert!(path.to_string_lossy().contains("cheolsu"));

        // 상대 경로
        let path = build_autosave_path(std::path::Path::new("data"));
        assert_eq!(path, std::path::PathBuf::from("data/autosave.cheolsu.gz"));
    }
}

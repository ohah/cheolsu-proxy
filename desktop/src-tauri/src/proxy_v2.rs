use proxy_daemon::{
    clean_old_cache, ClientCommand, DaemonConnection, DaemonMessage, InterceptRule,
    ServerReplayEntry, UpstreamProxyConfig,
};
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
    let conn = match proxy_daemon::ensure_daemon(port, &host, move |msg| match msg {
        DaemonMessage::Event { data } => {
            let _ = app_clone.emit("proxy_event", data);
        }
        DaemonMessage::WsMessage { data } => {
            let _ = app_clone.emit("ws_message", data);
        }
        DaemonMessage::WsConnection { data } => {
            let _ = app_clone.emit("ws_connection", data);
        }
        DaemonMessage::InterceptRulesUpdated { rules } => {
            let _ = app_clone.emit("intercept_rules_updated", rules);
        }
        DaemonMessage::ScriptLog { level, message } => {
            let _ = app_clone.emit(
                "script_log",
                serde_json::json!({ "level": level, "message": message }),
            );
        }
        DaemonMessage::ScriptStatus {
            active,
            path,
            message,
        } => {
            let _ = app_clone.emit(
                "script_status",
                serde_json::json!({ "active": active, "path": path, "message": message }),
            );
        }
        DaemonMessage::ScriptResult { success, error } => {
            let _ = app_clone.emit(
                "script_result",
                serde_json::json!({ "success": success, "error": error }),
            );
        }
        _ => {}
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

/// MCP 서버 바이너리의 절대 경로 반환
#[tauri::command]
pub fn get_mcp_server_path(app: AppHandle<impl Runtime>) -> Result<String, String> {
    use tauri::Manager;

    // 개발 모드: target/debug 또는 target/release에서 직접 찾기
    if cfg!(dev) {
        let current_exe =
            std::env::current_exe().map_err(|e| format!("Failed to get current exe: {}", e))?;
        // current_exe: target/debug/cheolsu-proxy → 같은 디렉토리에 cheolsu-proxy-mcp
        if let Some(dir) = current_exe.parent() {
            let mcp_path = dir.join("cheolsu-proxy-mcp");
            if mcp_path.exists() {
                return Ok(mcp_path.display().to_string());
            }
        }
    }

    // 프로덕션 모드: Tauri 리소스 경로에서 sidecar 찾기
    let target_triple = env!("TAURI_ENV_TARGET_TRIPLE");
    let sidecar_name = format!("binaries/cheolsu-proxy-mcp-{target_triple}");
    app.path()
        .resolve(&sidecar_name, tauri::path::BaseDirectory::Resource)
        .map(|p| p.display().to_string())
        .map_err(|e| format!("Failed to resolve MCP server path: {}", e))
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

/// TUI CLI 바이너리의 절대 경로 반환
fn resolve_tui_path(app: &AppHandle<impl Runtime>) -> Result<String, String> {
    use tauri::Manager;

    // 개발 모드
    if cfg!(dev) {
        let current_exe =
            std::env::current_exe().map_err(|e| format!("Failed to get current exe: {}", e))?;
        if let Some(dir) = current_exe.parent() {
            let tui_path = dir.join("cheolsu");
            if tui_path.exists() {
                return Ok(tui_path.display().to_string());
            }
        }
    }

    // 프로덕션 모드
    let target_triple = env!("TAURI_ENV_TARGET_TRIPLE");
    let sidecar_name = format!("binaries/cheolsu-{target_triple}");
    app.path()
        .resolve(&sidecar_name, tauri::path::BaseDirectory::Resource)
        .map(|p| p.display().to_string())
        .map_err(|e| format!("Failed to resolve TUI path: {}", e))
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

/// 터미널 명령어(cheolsu) 설치: /usr/local/bin/cheolsu에 심볼릭 링크 생성
#[tauri::command]
pub fn install_cli(app: AppHandle<impl Runtime>) -> Result<String, String> {
    let tui_path = resolve_tui_path(&app)?;
    let link_path = "/usr/local/bin/cheolsu";

    // 먼저 직접 시도, 실패하면 관리자 권한으로 재시도
    let link = std::path::Path::new(link_path);

    // 기존 링크 제거 + 새 링크 생성을 하나의 셸 명령으로
    let needs_admin = if link.exists() || link.is_symlink() {
        std::fs::remove_file(link).is_err()
    } else {
        false
    };

    #[cfg(unix)]
    {
        if needs_admin || std::os::unix::fs::symlink(&tui_path, link).is_err() {
            // 직접 생성 실패 → 관리자 권한으로 재시도
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

    // 먼저 직접 시도, 실패하면 관리자 권한으로 재시도
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

    let cer_path_str = cer_path.to_string_lossy().to_string();

    #[cfg(target_os = "macos")]
    {
        let cmd = format!(
            "security add-trusted-cert -r trustRoot -k ~/Library/Keychains/login.keychain-db \"{}\"",
            cer_path_str
        );
        run_with_admin_privileges(&cmd)?;
        Ok("CA 인증서가 키체인에 신뢰 인증서로 설치되었습니다.".to_string())
    }

    #[cfg(target_os = "windows")]
    {
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
        let cmd = "security delete-certificate -c \"Cheolsu Proxy Root CA\" ~/Library/Keychains/login.keychain-db 2>/dev/null; security delete-certificate -c \"Cheolsu Proxy Root CA\" 2>/dev/null; true";
        run_with_admin_privileges(cmd)?;
        Ok("CA 인증서가 키체인에서 제거되었습니다.".to_string())
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

/// HAR 파일 내보내기 (지정된 경로에 JSON 문자열 저장)
#[tauri::command]
pub async fn export_har_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content).map_err(|e| format!("HAR 파일 저장 실패: {} - {}", path, e))
}

fn base64_engine() -> base64::engine::GeneralPurpose {
    use base64::engine::general_purpose::STANDARD;
    STANDARD
}

fn base64_encode(engine: &base64::engine::GeneralPurpose, data: &[u8]) -> String {
    use base64::Engine;
    engine.encode(data)
}

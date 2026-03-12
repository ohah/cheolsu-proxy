use super::state::{ProxyStartResult, ProxyV2State};
use proxy_daemon::{
    clean_old_cache, BreakpointRule, ClientCommand, CommandSender, DaemonMessage, HostMapping,
    ProxyAuthConfig, ServerReplayEntry, SslProxyingEntry, SslProxyingMode, ThrottleConfig,
    UpstreamProxyConfig,
};
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

#[tauri::command]
pub(crate) async fn start_proxy_v2<R: Runtime>(
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

    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<DaemonMessage>(2048);

    let app_emitter = app.clone();
    let tray_state: Arc<crate::tray::TrayState> =
        app.state::<Arc<crate::tray::TrayState>>().inner().clone();
    std::thread::Builder::new()
        .name("event-emitter".into())
        .spawn(move || {
            while let Some(msg) = event_rx.blocking_recv() {
                if matches!(msg, DaemonMessage::Event { .. }) {
                    tray_state
                        .transaction_count
                        .fetch_add(1, Ordering::Relaxed);
                }
                let app = app_emitter.clone();
                let _ = app_emitter.run_on_main_thread(move || {
                    match msg {
                        DaemonMessage::Event { data } => {
                            if let Err(e) = app.emit("proxy_event", data) {
                                eprintln!("emit proxy_event 실패: {}", e);
                            }
                        }
                        DaemonMessage::WsMessage { data } => {
                            if let Err(e) = app.emit("ws_message", data) {
                                eprintln!("emit ws_message 실패: {}", e);
                            }
                        }
                        DaemonMessage::WsConnection { data } => {
                            if let Err(e) = app.emit("ws_connection", data) {
                                eprintln!("emit ws_connection 실패: {}", e);
                            }
                        }
                        DaemonMessage::InterceptRulesUpdated { rules } => {
                            if let Err(e) = app.emit("intercept_rules_updated", rules) {
                                eprintln!("emit intercept_rules_updated 실패: {}", e);
                            }
                        }
                        DaemonMessage::ScriptLog { level, message } => {
                            if let Err(e) = app.emit(
                                "script_log",
                                serde_json::json!({ "level": level, "message": message }),
                            ) {
                                eprintln!("emit script_log 실패: {}", e);
                            }
                        }
                        DaemonMessage::ScriptStatus {
                            active,
                            path,
                            message,
                        } => {
                            if let Err(e) = app.emit(
                                "script_status",
                                serde_json::json!({ "active": active, "path": path, "message": message }),
                            ) {
                                eprintln!("emit script_status 실패: {}", e);
                            }
                        }
                        DaemonMessage::ScriptResult { success, error } => {
                            if let Err(e) = app.emit(
                                "script_result",
                                serde_json::json!({ "success": success, "error": error }),
                            ) {
                                eprintln!("emit script_result 실패: {}", e);
                            }
                        }
                        DaemonMessage::BreakpointRulesUpdated { rules } => {
                            if let Err(e) = app.emit("breakpoint_rules_updated", rules) {
                                eprintln!("emit breakpoint_rules_updated 실패: {}", e);
                            }
                        }
                        DaemonMessage::BreakpointHit {
                            id,
                            transaction_id,
                            phase,
                            data,
                        } => {
                            if let Err(e) = app.emit(
                                "breakpoint_hit",
                                serde_json::json!({ "id": id, "transaction_id": transaction_id, "phase": phase, "data": data }),
                            ) {
                                eprintln!("emit breakpoint_hit 실패: {}", e);
                            }
                        }
                        DaemonMessage::HostMappingsUpdated { mappings } => {
                            if let Err(e) = app.emit("host_mappings_updated", mappings) {
                                eprintln!("emit host_mappings_updated 실패: {}", e);
                            }
                        }
                        DaemonMessage::SslProxyingListUpdated { mode, entries } => {
                            if let Err(e) = app.emit(
                                "ssl_proxying_list_updated",
                                serde_json::json!({ "mode": mode, "entries": entries }),
                            ) {
                                eprintln!("emit ssl_proxying_list_updated 실패: {}", e);
                            }
                        }
                        DaemonMessage::TlsPassthroughUpdated { entries } => {
                            if let Err(e) = app.emit("tls_passthrough_updated", entries) {
                                eprintln!("emit tls_passthrough_updated 실패: {}", e);
                            }
                        }
                        DaemonMessage::Disconnected { reason } => {
                            eprintln!("데몬 연결 끊김: {}", reason);
                            if let Err(e) = app.emit(
                                "daemon_disconnected",
                                serde_json::json!({ "reason": reason }),
                            ) {
                                eprintln!("emit daemon_disconnected 실패: {}", e);
                            }
                        }
                        DaemonMessage::Reconnected => {
                            eprintln!("데몬 재연결 성공");
                            if let Err(e) = app.emit("daemon_reconnected", ()) {
                                eprintln!("emit daemon_reconnected 실패: {}", e);
                            }
                        }
                        _ => {}
                    }
                });
            }
        })
        .expect("event-emitter 스레드 생성 실패");

    let conn = match proxy_daemon::ensure_daemon(port, &host, move |msg| {
        if let Err(e) = event_tx.try_send(msg) {
            eprintln!("이벤트 채널 전송 실패 (백프레셔): {}", e);
        }
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

    if let Some(ref conn) = *proxy_guard {
        let sender = conn.command_sender();
        sync_stored_settings_to_daemon(&app, &sender).await;
    }

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

fn read_store_state<R: Runtime>(app: &AppHandle<R>, store_name: &str) -> Option<serde_json::Value> {
    let app_data = app.path().app_data_dir().ok()?;
    let store_path = app_data.join(format!("{}.json", store_name));
    let content = std::fs::read_to_string(&store_path).ok()?;
    let store_json: serde_json::Value = serde_json::from_str(&content).ok()?;
    let state_str = store_json.get("state")?.as_str()?;
    let zustand_wrapper: serde_json::Value = serde_json::from_str(state_str).ok()?;
    zustand_wrapper.get("state").cloned()
}

async fn sync_stored_settings_to_daemon<R: Runtime>(app: &AppHandle<R>, sender: &CommandSender) {
    if let Some(settings) = read_store_state(app, "cheolsu-app-settings") {
        let no_caching = settings
            .get("quickSettingsNoCaching")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let block_cookies = settings
            .get("quickSettingsBlockCookies")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let no_gzip = settings
            .get("quickSettingsNoGzip")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let _ = sender
            .send_command(&ClientCommand::UpdateQuickSettings {
                no_caching,
                block_cookies,
                no_gzip,
            })
            .await;

        if let Some(auth) = settings.get("proxyAuthConfig") {
            if let Ok(config) = serde_json::from_value::<ProxyAuthConfig>(auth.clone()) {
                if config.enabled {
                    let _ = sender
                        .send_command(&ClientCommand::UpdateProxyAuth { config })
                        .await;
                }
            }
        }

        if let Some(throttle_cfg) = settings.get("throttleConfig") {
            let enabled = throttle_cfg
                .get("enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if enabled {
                let preset = throttle_cfg
                    .get("preset")
                    .and_then(|v| v.as_str())
                    .unwrap_or("none");
                let throttle_config = if preset == "custom" {
                    let dl = throttle_cfg
                        .get("download")
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(0);
                    let ul = throttle_cfg
                        .get("upload")
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(0);
                    let latency = throttle_cfg
                        .get("latency")
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(0);
                    Some(ThrottleConfig {
                        enabled: true,
                        download_rate: if dl > 0 { Some(dl * 1024) } else { None },
                        upload_rate: if ul > 0 { Some(ul * 1024) } else { None },
                        latency_ms: latency,
                    })
                } else {
                    None
                };
                if let Some(config) = throttle_config {
                    let _ = sender
                        .send_command(&ClientCommand::UpdateThrottle {
                            config: Some(config),
                        })
                        .await;
                }
            }
        }

        if let Some(strategy) = settings.get("connectionStrategy").and_then(|v| v.as_str()) {
            let _ = sender
                .send_command(&ClientCommand::UpdateConnectionStrategy {
                    strategy: strategy.to_string(),
                })
                .await;
        }

        if let Some(upstream) = settings.get("upstreamProxyConfig") {
            let up_enabled = upstream
                .get("enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if up_enabled {
                if let Ok(config) = serde_json::from_value::<UpstreamProxyConfig>(upstream.clone())
                {
                    let _ = sender
                        .send_command(&ClientCommand::UpdateUpstreamProxy {
                            config: Some(config),
                        })
                        .await;
                }
            }
        }
    }

    if let Some(ssl) = read_store_state(app, "cheolsu-ssl-proxying") {
        let mode: SslProxyingMode = ssl
            .get("mode")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        let entries: Vec<SslProxyingEntry> = ssl
            .get("entries")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        let _ = sender
            .send_command(&ClientCommand::UpdateSslProxyingList { mode, entries })
            .await;
    }

    if let Some(hm) = read_store_state(app, "cheolsu-host-mappings") {
        let mappings: Vec<HostMapping> = hm
            .get("hostMappings")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        if !mappings.is_empty() {
            let _ = sender
                .send_command(&ClientCommand::UpdateHostMappings { mappings })
                .await;
        }
    }

    if let Some(bp) = read_store_state(app, "cheolsu-breakpoint-rules") {
        let rules: Vec<BreakpointRule> = bp
            .get("rules")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        if !rules.is_empty() {
            let _ = sender
                .send_command(&ClientCommand::UpdateBreakpointRules { rules })
                .await;
        }
    }

    if let Some(sr) = read_store_state(app, "cheolsu-server-replay") {
        let is_enabled = sr
            .get("isEnabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if is_enabled {
            let entries: Vec<ServerReplayEntry> = sr
                .get("entries")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            if !entries.is_empty() {
                let _ = sender
                    .send_command(&ClientCommand::UpdateServerReplay { entries })
                    .await;
            }
        }
    }

    tracing::info!("저장된 설정을 데몬에 동기화 완료");
}

#[tauri::command]
pub(crate) async fn stop_proxy_v2(
    proxy: tauri::State<'_, ProxyV2State>,
    tray_state: tauri::State<'_, Arc<crate::tray::TrayState>>,
) -> Result<(), String> {
    let conn = {
        let mut guard = proxy.lock().await;
        guard.take()
    };
    if let Some(conn) = conn {
        conn.disconnect().await;
        tray_state.transaction_count.store(0, Ordering::Relaxed);
        println!("Daemon 연결 해제 완료");
    } else {
        return Err("프록시가 실행 중이 아닙니다".to_string());
    }

    Ok(())
}

#[tauri::command]
pub(crate) async fn proxy_v2_status(proxy: tauri::State<'_, ProxyV2State>) -> Result<bool, String> {
    Ok(proxy.lock().await.is_some())
}

#[tauri::command]
pub(crate) async fn read_body_file(file_path: String) -> Result<Vec<u8>, String> {
    tokio::task::spawn_blocking(move || {
        std::fs::read(&file_path).map_err(|e| format!("파일 읽기 실패: {} - {}", file_path, e))
    })
    .await
    .map_err(|e| format!("파일 읽기 태스크 실패: {}", e))?
}

#[tauri::command]
pub(crate) async fn clean_old_proxy_cache(days: u64) -> Result<String, String> {
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

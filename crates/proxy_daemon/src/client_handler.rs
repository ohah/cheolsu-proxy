use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::{broadcast, watch, Mutex};
use tracing::{error, info, warn};

use crate::breakpoint::BreakpointManager;
use crate::protocol::{
    BreakpointRule, ClientCommand, DaemonMessage, HostMapping, InterceptRule, ServerReplayEntry,
    SslProxyingEntry,
};
use proxyapi_v2::throttle::ThrottleConfig;
use proxyapi_v2::upstream_proxy::UpstreamProxyConfig;
use proxyapi_v2::websocket_registry::WebSocketRegistry;

pub async fn handle_client(
    stream: UnixStream,
    mut event_rx: broadcast::Receiver<String>,
    intercept_tx: watch::Sender<Vec<InterceptRule>>,
    upstream_tx: watch::Sender<Option<UpstreamProxyConfig>>,
    server_replay_tx: watch::Sender<Vec<ServerReplayEntry>>,
    throttle_tx: watch::Sender<Option<ThrottleConfig>>,
    breakpoint_tx: watch::Sender<Vec<BreakpointRule>>,
    breakpoint_manager: BreakpointManager,
    host_mapping_tx: watch::Sender<Vec<HostMapping>>,
    ssl_proxying_tx: watch::Sender<Vec<SslProxyingEntry>>,
    event_tx: broadcast::Sender<String>,
    port: u16,
    ws_registry: WebSocketRegistry,
    script_handle: scripting::ScriptHandle,
    quick_settings: std::sync::Arc<tokio::sync::RwLock<crate::handler::QuickSettings>>,
) {
    let (reader, writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let writer = Arc::new(Mutex::new(writer));

    {
        let status_msg = DaemonMessage::Status {
            running: true,
            port,
        };
        let mut line = serde_json::to_string(&status_msg).unwrap_or_default();
        line.push('\n');
        let mut w = writer.lock().await;
        let _ = w.write_all(line.as_bytes()).await;
        let _ = w.flush().await;

        // 연결 직후 현재 인터셉트 규칙을 전송 (빈 목록도 유효한 상태)
        let current_rules = intercept_tx.borrow().clone();
        let rules_msg = DaemonMessage::InterceptRulesUpdated {
            rules: current_rules,
        };
        let mut rules_line = serde_json::to_string(&rules_msg).unwrap_or_default();
        rules_line.push('\n');
        let _ = w.write_all(rules_line.as_bytes()).await;
        let _ = w.flush().await;

        let current_mappings = host_mapping_tx.borrow().clone();
        let mappings_msg = DaemonMessage::HostMappingsUpdated {
            mappings: current_mappings,
        };
        let mut mappings_line = serde_json::to_string(&mappings_msg).unwrap_or_default();
        mappings_line.push('\n');
        let _ = w.write_all(mappings_line.as_bytes()).await;
        let _ = w.flush().await;

        let current_ssl_proxying = ssl_proxying_tx.borrow().clone();
        let ssl_msg = DaemonMessage::SslProxyingListUpdated {
            entries: current_ssl_proxying,
        };
        let mut ssl_line = serde_json::to_string(&ssl_msg).unwrap_or_default();
        ssl_line.push('\n');
        let _ = w.write_all(ssl_line.as_bytes()).await;
        let _ = w.flush().await;
    }

    // 스크립트 로그 전달 태스크
    let writer_logs = writer.clone();
    let mut log_rx = script_handle.subscribe_logs();
    let log_task = tokio::spawn(async move {
        while let Ok(entry) = log_rx.recv().await {
            let msg = DaemonMessage::ScriptLog {
                level: entry.level,
                message: entry.message,
            };
            if let Ok(json) = serde_json::to_string(&msg) {
                let mut line = json;
                line.push('\n');
                let mut w = writer_logs.lock().await;
                if w.write_all(line.as_bytes()).await.is_err() {
                    break;
                }
                let _ = w.flush().await;
            }
        }
    });

    // 파일 감시 태스크 (스크립트 파일 변경 시 자동 리로드)
    let watched_path: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    let writer_events = writer.clone();
    let subscribed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let subscribed_clone = subscribed.clone();

    let event_task = tokio::spawn(async move {
        loop {
            match event_rx.recv().await {
                Ok(msg) => {
                    if !subscribed_clone.load(std::sync::atomic::Ordering::Relaxed) {
                        continue;
                    }
                    let mut line = msg;
                    line.push('\n');
                    let mut w = writer_events.lock().await;
                    if w.write_all(line.as_bytes()).await.is_err() {
                        break;
                    }
                    if w.flush().await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!("Client lagged by {} messages", n);
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    let mut line_buf = String::new();
    loop {
        line_buf.clear();
        match reader.read_line(&mut line_buf).await {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = line_buf.trim();
                if trimmed.is_empty() {
                    continue;
                }
                match serde_json::from_str::<ClientCommand>(trimmed) {
                    Ok(ClientCommand::Subscribe) => {
                        subscribed.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                    Ok(ClientCommand::UpdateInterceptRules { rules }) => {
                        info!("Intercept rules updated from client: {} rules", rules.len());
                        // 다른 클라이언트에게 규칙 변경 broadcast
                        let broadcast_msg = DaemonMessage::InterceptRulesUpdated {
                            rules: rules.clone(),
                        };
                        if let Ok(json) = serde_json::to_string(&broadcast_msg) {
                            let _ = event_tx.send(json);
                        }
                        let _ = intercept_tx.send(rules);
                    }
                    Ok(ClientCommand::WsInject {
                        connection_id,
                        direction,
                        payload,
                        is_binary,
                    }) => {
                        use tokio_tungstenite::tungstenite::Message;

                        let msg = if is_binary {
                            use base64::Engine;
                            match base64::engine::general_purpose::STANDARD.decode(&payload) {
                                Ok(bytes) => Message::Binary(bytes.into()),
                                Err(e) => {
                                    let result = DaemonMessage::WsInjectResult {
                                        success: false,
                                        error: Some(format!("Base64 decode failed: {}", e)),
                                    };
                                    let mut line =
                                        serde_json::to_string(&result).unwrap_or_default();
                                    line.push('\n');
                                    let mut w = writer.lock().await;
                                    let _ = w.write_all(line.as_bytes()).await;
                                    let _ = w.flush().await;
                                    continue;
                                }
                            }
                        } else {
                            Message::Text(payload.into())
                        };

                        let result = match direction.as_str() {
                            "to_client" => ws_registry.inject_to_client(&connection_id, msg).await,
                            "to_server" => ws_registry.inject_to_server(&connection_id, msg).await,
                            _ => Err(format!("Invalid direction: {}", direction)),
                        };

                        let response = match result {
                            Ok(()) => DaemonMessage::WsInjectResult {
                                success: true,
                                error: None,
                            },
                            Err(e) => DaemonMessage::WsInjectResult {
                                success: false,
                                error: Some(e),
                            },
                        };
                        let mut line = serde_json::to_string(&response).unwrap_or_default();
                        line.push('\n');
                        let mut w = writer.lock().await;
                        let _ = w.write_all(line.as_bytes()).await;
                        let _ = w.flush().await;
                    }
                    Ok(ClientCommand::UpdateUpstreamProxy { config }) => {
                        info!(
                            "Upstream proxy config updated: {:?}",
                            config.as_ref().map(|c| c.address())
                        );
                        let _ = upstream_tx.send(config);
                    }
                    Ok(ClientCommand::UpdateThrottle { config }) => {
                        info!(
                            "Throttle config updated: enabled={:?}",
                            config.as_ref().map(|c| c.enabled)
                        );
                        let _ = throttle_tx.send(config);
                    }
                    Ok(ClientCommand::UpdateHostMappings { mappings }) => {
                        info!(
                            "Host mappings updated from client: {} mappings",
                            mappings.len()
                        );
                        let broadcast_msg = DaemonMessage::HostMappingsUpdated {
                            mappings: mappings.clone(),
                        };
                        if let Ok(json) = serde_json::to_string(&broadcast_msg) {
                            let _ = event_tx.send(json);
                        }
                        let _ = host_mapping_tx.send(mappings);
                    }
                    Ok(ClientCommand::UpdateSslProxyingList { entries }) => {
                        info!(
                            "SSL Proxying list updated from client: {} entries",
                            entries.len()
                        );
                        let broadcast_msg = DaemonMessage::SslProxyingListUpdated {
                            entries: entries.clone(),
                        };
                        if let Ok(json) = serde_json::to_string(&broadcast_msg) {
                            let _ = event_tx.send(json);
                        }
                        let _ = ssl_proxying_tx.send(entries);
                    }
                    Ok(ClientCommand::UpdateQuickSettings {
                        no_caching,
                        block_cookies,
                        no_gzip,
                    }) => {
                        info!(
                            "Quick settings updated: no_caching={}, block_cookies={}, no_gzip={}",
                            no_caching, block_cookies, no_gzip
                        );
                        {
                            let mut settings = quick_settings.write().await;
                            settings.no_caching = no_caching;
                            settings.block_cookies = block_cookies;
                            settings.no_gzip = no_gzip;
                        }
                    }
                    Ok(ClientCommand::UpdateServerReplay { entries }) => {
                        info!("Server replay entries updated: {} entries", entries.len());
                        let _ = server_replay_tx.send(entries);
                    }
                    Ok(ClientCommand::LoadScript { path, code }) => {
                        let result: Result<(), String> = if let Some(file_path) = &path {
                            script_handle
                                .load_file(file_path)
                                .await
                                .map_err(|e| e.to_string())
                        } else if let Some(script_code) = &code {
                            // JS로 먼저 시도, 실패 시 TS로 트랜스파일
                            match script_handle.load_code(script_code).await {
                                Ok(()) => Ok(()),
                                Err(_) => script_handle
                                    .load_ts_code(script_code)
                                    .await
                                    .map_err(|e| e.to_string()),
                            }
                        } else {
                            Err("path 또는 code 중 하나가 필요합니다".to_string())
                        };

                        let response = match &result {
                            Ok(()) => {
                                info!("Script loaded successfully");
                                // 파일 감시 시작
                                if let Some(file_path) = &path {
                                    let mut wp = watched_path.lock().await;
                                    *wp = Some(file_path.clone());
                                    start_file_watcher(
                                        file_path.clone(),
                                        script_handle.clone(),
                                        writer.clone(),
                                        watched_path.clone(),
                                        event_tx.clone(),
                                    );
                                }
                                // 스크립트 상태 브로드캐스트
                                let status_msg = DaemonMessage::ScriptStatus {
                                    active: true,
                                    path: path.clone(),
                                    message: "스크립트 로드 완료".to_string(),
                                };
                                if let Ok(json) = serde_json::to_string(&status_msg) {
                                    let _ = event_tx.send(json);
                                }
                                DaemonMessage::ScriptResult {
                                    success: true,
                                    error: None,
                                }
                            }
                            Err(e) => {
                                error!("Script load failed: {}", e);
                                DaemonMessage::ScriptResult {
                                    success: false,
                                    error: Some(e.clone()),
                                }
                            }
                        };
                        let mut line = serde_json::to_string(&response).unwrap_or_default();
                        line.push('\n');
                        let mut w = writer.lock().await;
                        let _ = w.write_all(line.as_bytes()).await;
                        let _ = w.flush().await;
                    }
                    Ok(ClientCommand::UnloadScript) => {
                        script_handle.unload().await;
                        {
                            let mut wp = watched_path.lock().await;
                            *wp = None;
                        }
                        info!("Script unloaded");
                        // 스크립트 상태 브로드캐스트
                        let status_msg = DaemonMessage::ScriptStatus {
                            active: false,
                            path: None,
                            message: "스크립트 언로드됨".to_string(),
                        };
                        if let Ok(json) = serde_json::to_string(&status_msg) {
                            let _ = event_tx.send(json);
                        }
                        let response = DaemonMessage::ScriptResult {
                            success: true,
                            error: None,
                        };
                        let mut line = serde_json::to_string(&response).unwrap_or_default();
                        line.push('\n');
                        let mut w = writer.lock().await;
                        let _ = w.write_all(line.as_bytes()).await;
                        let _ = w.flush().await;
                    }
                    Ok(ClientCommand::UpdateBreakpointRules { rules }) => {
                        info!(
                            "Breakpoint rules updated from client: {} rules",
                            rules.len()
                        );
                        let broadcast_msg = DaemonMessage::BreakpointRulesUpdated {
                            rules: rules.clone(),
                        };
                        if let Ok(json) = serde_json::to_string(&broadcast_msg) {
                            let _ = event_tx.send(json);
                        }
                        let _ = breakpoint_tx.send(rules);
                    }
                    Ok(ClientCommand::ResolveBreakpoint { id, action }) => {
                        info!("Resolving breakpoint: {} -> {:?}", id, action);
                        if let Err(e) = breakpoint_manager.resolve(&id, action).await {
                            warn!("Failed to resolve breakpoint: {}", e);
                        }
                    }
                    Ok(ClientCommand::SaveSession { path, filter }) => {
                        info!(
                            "SaveSession command received: path={}, filter={:?}",
                            path, filter
                        );
                        // NOTE: SaveSession/LoadSession are primarily handled by MCP server directly.
                        // This handler sends an acknowledgment message but does not perform the actual save/load.
                        let msg = DaemonMessage::SessionSaved {
                            path: path.clone(),
                            transaction_count: 0,
                        };
                        let mut line = serde_json::to_string(&msg).unwrap_or_default();
                        line.push('\n');
                        let mut w = writer.lock().await;
                        let _ = w.write_all(line.as_bytes()).await;
                        let _ = w.flush().await;
                    }
                    Ok(ClientCommand::LoadSession { path }) => {
                        info!("LoadSession command received: path={}", path);
                        // NOTE: SaveSession/LoadSession are primarily handled by MCP server directly.
                        // This handler sends an acknowledgment message but does not perform the actual save/load.
                        let msg = DaemonMessage::SessionLoaded {
                            path: path.clone(),
                            transaction_count: 0,
                        };
                        let mut line = serde_json::to_string(&msg).unwrap_or_default();
                        line.push('\n');
                        let mut w = writer.lock().await;
                        let _ = w.write_all(line.as_bytes()).await;
                        let _ = w.flush().await;
                    }
                    Ok(ClientCommand::Stop) => {
                        break;
                    }
                    Err(e) => {
                        error!("Invalid command from client: {} ({})", trimmed, e);
                    }
                }
            }
            Err(e) => {
                error!("Client read error: {}", e);
                break;
            }
        }
    }

    event_task.abort();
    log_task.abort();
}

/// 파일 감시 시작 (스크립트 변경 시 자동 리로드)
fn start_file_watcher(
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
            {
                let wp = watched_path.lock().await;
                if wp.as_deref() != Some(&path) {
                    break;
                }
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

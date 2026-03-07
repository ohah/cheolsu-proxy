use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{broadcast, watch, Mutex};
use tracing::{error, info, warn};

use crate::handler::{create_hybrid_client, LoggingHandler};
use crate::protocol::{ClientCommand, DaemonMessage, ProxyLockInfo};
use crate::system_proxy::set_proxy;
use proxyapi_v2::upstream_proxy::UpstreamProxyConfig;
use proxyapi_v2::websocket_registry::WebSocketRegistry;

pub fn app_support_dir() -> Result<PathBuf, String> {
    dirs::data_dir()
        .ok_or_else(|| "Cannot find data directory".to_string())
        .map(|dir| dir.join("com.cheolsu-proxy"))
}

pub fn lock_file_path() -> Result<PathBuf, String> {
    Ok(app_support_dir()?.join("proxy.lock"))
}

pub fn uds_socket_path() -> Result<PathBuf, String> {
    Ok(app_support_dir()?.join("proxy.sock"))
}

fn write_lock_file(port: u16, uds_path: &str) -> Result<(), String> {
    let dir = app_support_dir()?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let info = ProxyLockInfo {
        pid: std::process::id(),
        port,
        uds_path: uds_path.to_string(),
    };
    let json = serde_json::to_string_pretty(&info).map_err(|e| e.to_string())?;
    std::fs::write(lock_file_path()?, json).map_err(|e| e.to_string())
}

fn remove_lock_file() {
    if let Ok(path) = lock_file_path() {
        let _ = std::fs::remove_file(path);
    }
}

fn remove_uds_socket(path: &Path) {
    let _ = std::fs::remove_file(path);
}

/// Checks if a stale lock file exists and cleans it up.
/// Returns true if the lock was stale and cleaned up (or didn't exist).
pub fn check_and_cleanup_stale_lock() -> bool {
    let lock_path = match lock_file_path() {
        Ok(p) => p,
        Err(_) => return true,
    };
    if !lock_path.exists() {
        return true;
    }

    let contents = match std::fs::read_to_string(&lock_path) {
        Ok(c) => c,
        Err(_) => {
            let _ = std::fs::remove_file(&lock_path);
            return true;
        }
    };

    let info: ProxyLockInfo = match serde_json::from_str(&contents) {
        Ok(i) => i,
        Err(_) => {
            let _ = std::fs::remove_file(&lock_path);
            return true;
        }
    };

    use nix::sys::signal::kill;
    use nix::unistd::Pid;
    let pid = Pid::from_raw(info.pid as i32);
    if kill(pid, None).is_err() {
        warn!(
            "Stale lock file detected (PID {} is dead), cleaning up",
            info.pid
        );
        let _ = std::fs::remove_file(&lock_path);
        let _ = std::fs::remove_file(&info.uds_path);
        return true;
    }

    false
}

// --- Daemon Entry Point ---

/// Runs the daemon process. This function never returns (calls std::process::exit).
pub fn run_daemon(port: u16, host: String) -> ! {
    // Initialize tracing so error!/info!/warn! output to stderr.
    // Use try_init to avoid panic if a subscriber is already set (e.g., by Tauri).
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");

    let exit_code = rt.block_on(async move { daemon_main(port, host).await });
    std::process::exit(exit_code)
}

async fn daemon_main(port: u16, host: String) -> i32 {
    if !check_and_cleanup_stale_lock() {
        error!("Another daemon is already running. Exiting.");
        return 1;
    }

    let uds_path = match uds_socket_path() {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to get UDS socket path: {}", e);
            return 1;
        }
    };
    let uds_path_str = uds_path.to_string_lossy().to_string();

    remove_uds_socket(&uds_path);

    if let Some(parent) = uds_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    if let Err(e) = write_lock_file(port, &uds_path_str) {
        error!("Failed to write lock file: {}", e);
        return 1;
    }

    let (event_tx, _) = broadcast::channel::<String>(256);

    let client_count = Arc::new(AtomicUsize::new(0));

    let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);

    let (intercept_tx, intercept_rx) =
        watch::channel::<Vec<crate::protocol::InterceptRule>>(Vec::new());

    let (upstream_tx, upstream_rx) = watch::channel::<Option<UpstreamProxyConfig>>(None);

    let (server_replay_tx, server_replay_rx) =
        watch::channel::<Vec<crate::protocol::ServerReplayEntry>>(Vec::new());

    let ws_registry = WebSocketRegistry::new();
    let script_handle = scripting::ScriptHandle::new();

    let addr: std::net::SocketAddr = format!("{}:{}", host, port)
        .parse()
        .expect("Invalid host:port");

    if let Err(e) = set_proxy(true, port) {
        error!("Failed to set system proxy: {}", e);
    }

    let event_tx_proxy = event_tx.clone();
    let registry_for_proxy = ws_registry.clone();
    let script_handle_for_proxy = script_handle.clone();
    let proxy_handle = tokio::spawn(async move {
        if let Err(code) = run_proxy(
            addr,
            event_tx_proxy,
            intercept_rx,
            upstream_rx,
            server_replay_rx,
            registry_for_proxy,
            script_handle_for_proxy,
        )
        .await
        {
            error!("Proxy error: {}", code);
        }
    });

    let uds_listener = match UnixListener::bind(&uds_path) {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind UDS: {}", e);
            cleanup(port, &uds_path);
            return 1;
        }
    };

    let log_path = app_support_dir()
        .map(|d| d.join("daemon.log"))
        .unwrap_or_default();
    info!(
        "Daemon started (PID {}, proxy={}:{}, uds={}, log={})",
        std::process::id(),
        host,
        port,
        uds_path_str,
        log_path.display()
    );

    let shutdown_tx_ctrlc = shutdown_tx.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        warn!("Ctrl+C received, shutting down daemon...");
        let _ = shutdown_tx_ctrlc.send(()).await;
    });

    // SIGTERM 시그널 처리 (프로세스 kill 시 프록시 설정 복원)
    let shutdown_tx_term = shutdown_tx.clone();
    tokio::spawn(async move {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to register SIGTERM handler");
        sigterm.recv().await;
        warn!("SIGTERM received, shutting down daemon...");
        let _ = shutdown_tx_term.send(()).await;
    });

    loop {
        tokio::select! {
            accept_result = uds_listener.accept() => {
                match accept_result {
                    Ok((stream, _addr)) => {
                        let count = client_count.fetch_add(1, Ordering::SeqCst) + 1;
                        info!("Client connected (total: {})", count);

                        let event_rx = event_tx.subscribe();
                        let event_tx_clone = event_tx.clone();
                        let client_count_clone = client_count.clone();
                        let shutdown_tx_clone = shutdown_tx.clone();
                        let intercept_tx_clone = intercept_tx.clone();
                        let upstream_tx_clone = upstream_tx.clone();
                        let server_replay_tx_clone = server_replay_tx.clone();
                        let registry_clone = ws_registry.clone();
                        let script_handle_clone = script_handle.clone();

                        tokio::spawn(async move {
                            handle_client(stream, event_rx, intercept_tx_clone, upstream_tx_clone, server_replay_tx_clone, event_tx_clone, port, registry_clone, script_handle_clone)
                                .await;

                            let remaining = client_count_clone.fetch_sub(1, Ordering::SeqCst) - 1;
                            info!("Client disconnected (remaining: {})", remaining);

                            if remaining == 0 {
                                info!("No clients remaining, shutting down daemon...");
                                let _ = shutdown_tx_clone.send(()).await;
                            }
                        });
                    }
                    Err(e) => {
                        error!("UDS accept error: {}", e);
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                info!("Shutdown signal received");
                break;
            }
        }
    }

    proxy_handle.abort();
    cleanup(port, &uds_path);
    info!("Daemon stopped");
    0
}

fn cleanup(port: u16, uds_path: &Path) {
    if let Err(e) = set_proxy(false, port) {
        error!("Failed to unset system proxy: {}", e);
    }
    remove_lock_file();
    remove_uds_socket(uds_path);
}

// --- Proxy Runner ---

async fn run_proxy(
    addr: std::net::SocketAddr,
    event_tx: broadcast::Sender<String>,
    mut intercept_rx: watch::Receiver<Vec<crate::protocol::InterceptRule>>,
    upstream_rx: watch::Receiver<Option<UpstreamProxyConfig>>,
    mut server_replay_rx: watch::Receiver<Vec<crate::protocol::ServerReplayEntry>>,
    ws_registry: WebSocketRegistry,
    script_handle: scripting::ScriptHandle,
) -> Result<(), String> {
    use proxyapi_v2::builder::ProxyBuilder;
    use proxyapi_v2::certificate_authority::{
        build_ca, generate_session_hash, get_cache_storage_dir,
    };
    use tokio::net::TcpListener;

    let ca = build_ca().map_err(|e| format!("CA build failed: {}", e))?;

    let session_hash = generate_session_hash();
    let cache_dir =
        get_cache_storage_dir(&session_hash).map_err(|e| format!("Cache dir failed: {}", e))?;

    let (tx, mut rx) = tokio::sync::mpsc::channel::<proxy_v2_models::RequestInfo>(256);
    let (tunnel_tx, mut tunnel_rx) =
        tokio::sync::mpsc::channel::<proxy_v2_models::RequestInfo>(100);

    let (ws_tx, mut ws_rx) = tokio::sync::mpsc::channel::<crate::handler::WsEvent>(256);

    let handler = LoggingHandler::new(tx.clone(), cache_dir)
        .with_ws_sender(ws_tx)
        .with_script_handle(script_handle);

    // 인터셉트 규칙 초기값 로드
    {
        let rules = intercept_rx.borrow().clone();
        handler.update_intercept_rules(rules).await;
    }

    let handler_for_intercept_updates = handler.clone();
    tokio::spawn(async move {
        while intercept_rx.changed().await.is_ok() {
            let rules = intercept_rx.borrow().clone();
            handler_for_intercept_updates
                .update_intercept_rules(rules)
                .await;
        }
    });

    // 서버 리플레이 엔트리 초기값 로드
    {
        let entries = server_replay_rx.borrow().clone();
        handler.update_server_replay_entries(entries).await;
    }

    let handler_for_replay_updates = handler.clone();
    tokio::spawn(async move {
        while server_replay_rx.changed().await.is_ok() {
            let entries = server_replay_rx.borrow().clone();
            handler_for_replay_updates
                .update_server_replay_entries(entries)
                .await;
        }
    });

    let initial_upstream = upstream_rx.borrow().clone();

    let hybrid_client =
        create_hybrid_client(upstream_rx).map_err(|e| format!("Client creation failed: {}", e))?;

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Port {} bind failed: {}", addr.port(), e))?;

    // TLS 자동 학습 바이패스 초기화
    let passthrough_path = app_support_dir()
        .ok()
        .map(|dir| dir.join("tls_passthrough.json"));
    let tls_passthrough = proxyapi_v2::tls_passthrough::TlsPassthrough::new(passthrough_path);

    let proxy_ctx = proxyapi_v2::ProxyContext {
        tunnel_event_sender: Some(tunnel_tx),
        tls_passthrough: Some(tls_passthrough),
        websocket_registry: Some(ws_registry),
        upstream_proxy: initial_upstream,
        ..Default::default()
    };

    let proxy_builder = ProxyBuilder::new()
        .with_listener(listener)
        .with_ca(ca)
        .with_client(hybrid_client)
        .with_http_handler(handler.clone())
        .with_websocket_handler(handler.clone())
        .with_proxy_context(proxy_ctx)
        .build()
        .map_err(|e| format!("Proxy build failed: {}", e))?;

    info!("Proxy listening on {}", addr);

    let event_tx_http = event_tx.clone();
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let json = serde_json::to_string(&event).unwrap_or_default();
            let msg = serde_json::to_string(&DaemonMessage::Event { data: event }).unwrap_or(json);
            let _ = event_tx_http.send(msg);
        }
    });

    let event_tx_tunnel = event_tx.clone();
    tokio::spawn(async move {
        while let Some(tunnel_event) = tunnel_rx.recv().await {
            let msg = serde_json::to_string(&DaemonMessage::Event { data: tunnel_event })
                .unwrap_or_default();
            let _ = event_tx_tunnel.send(msg);
        }
    });

    let event_tx_ws = event_tx.clone();
    tokio::spawn(async move {
        while let Some(ws_event) = ws_rx.recv().await {
            let msg = match ws_event {
                crate::handler::WsEvent::Message(info) => {
                    serde_json::to_string(&DaemonMessage::WsMessage { data: info })
                }
                crate::handler::WsEvent::Connection(event) => {
                    serde_json::to_string(&DaemonMessage::WsConnection { data: event })
                }
            };
            if let Ok(msg) = msg {
                let _ = event_tx_ws.send(msg);
            }
        }
    });

    proxy_builder
        .start()
        .await
        .map_err(|e| format!("Proxy start failed: {}", e))?;

    Ok(())
}

// --- Client Handler ---

pub async fn handle_client(
    stream: UnixStream,
    mut event_rx: broadcast::Receiver<String>,
    intercept_tx: watch::Sender<Vec<crate::protocol::InterceptRule>>,
    upstream_tx: watch::Sender<Option<UpstreamProxyConfig>>,
    server_replay_tx: watch::Sender<Vec<crate::protocol::ServerReplayEntry>>,
    event_tx: broadcast::Sender<String>,
    port: u16,
    ws_registry: WebSocketRegistry,
    script_handle: scripting::ScriptHandle,
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
    }

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
                    Ok(ClientCommand::UpdateServerReplay { entries }) => {
                        info!("Server replay entries updated: {} entries", entries.len());
                        let _ = server_replay_tx.send(entries);
                    }
                    Ok(ClientCommand::LoadScript { path, code }) => {
                        let result = if let Some(file_path) = &path {
                            script_handle.load_file(file_path).await
                        } else if let Some(script_code) = &code {
                            // JS로 먼저 시도, 실패 시 TS로 트랜스파일
                            match script_handle.load_code(script_code).await {
                                Ok(()) => Ok(()),
                                Err(_) => script_handle.load_ts_code(script_code).await,
                            }
                        } else {
                            Err("path 또는 code 중 하나가 필요합니다".to_string())
                        };

                        let response = match result {
                            Ok(()) => {
                                info!("Script loaded successfully");
                                DaemonMessage::ScriptResult {
                                    success: true,
                                    error: None,
                                }
                            }
                            Err(e) => {
                                error!("Script load failed: {}", e);
                                DaemonMessage::ScriptResult {
                                    success: false,
                                    error: Some(e),
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
                        info!("Script unloaded");
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_client_command_subscribe_serialization() {
        let cmd = ClientCommand::Subscribe;
        let json = serde_json::to_string(&cmd).unwrap();
        assert_eq!(json, r#"{"cmd":"subscribe"}"#);
    }

    #[test]
    fn test_client_command_subscribe_deserialization() {
        let json = r#"{"cmd":"subscribe"}"#;
        let cmd: ClientCommand = serde_json::from_str(json).unwrap();
        assert!(matches!(cmd, ClientCommand::Subscribe));
    }

    #[test]
    fn test_client_command_stop_serialization() {
        let cmd = ClientCommand::Stop;
        let json = serde_json::to_string(&cmd).unwrap();
        assert_eq!(json, r#"{"cmd":"stop"}"#);
    }

    #[test]
    fn test_client_command_stop_deserialization() {
        let json = r#"{"cmd":"stop"}"#;
        let cmd: ClientCommand = serde_json::from_str(json).unwrap();
        assert!(matches!(cmd, ClientCommand::Stop));
    }

    #[test]
    fn test_daemon_message_status_serialization() {
        let msg = DaemonMessage::Status {
            running: true,
            port: 8100,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"], "status");
        assert_eq!(parsed["running"], true);
        assert_eq!(parsed["port"], 8100);
    }

    #[test]
    fn test_daemon_message_status_deserialization() {
        let json = r#"{"type":"status","running":true,"port":8100}"#;
        let msg: DaemonMessage = serde_json::from_str(json).unwrap();
        match msg {
            DaemonMessage::Status { running, port } => {
                assert!(running);
                assert_eq!(port, 8100);
            }
            _ => panic!("Expected Status"),
        }
    }

    #[test]
    fn test_invalid_command_deserialization_fails() {
        let json = r#"{"cmd":"unknown_command"}"#;
        let result = serde_json::from_str::<ClientCommand>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_message_deserialization_fails() {
        let json = r#"{"type":"unknown_type"}"#;
        let result = serde_json::from_str::<DaemonMessage>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_proxy_lock_info_serialization_roundtrip() {
        let info = ProxyLockInfo {
            pid: 12345,
            port: 8100,
            uds_path: "/tmp/proxy.sock".to_string(),
        };
        let json = serde_json::to_string_pretty(&info).unwrap();
        let parsed: ProxyLockInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.pid, 12345);
        assert_eq!(parsed.port, 8100);
        assert_eq!(parsed.uds_path, "/tmp/proxy.sock");
    }

    #[test]
    fn test_proxy_lock_info_fields() {
        let json = r#"{"pid":99999,"port":9090,"uds_path":"/var/run/test.sock"}"#;
        let info: ProxyLockInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.pid, 99999);
        assert_eq!(info.port, 9090);
        assert_eq!(info.uds_path, "/var/run/test.sock");
    }

    #[test]
    fn test_stale_lock_with_dead_pid() {
        let tmp = TempDir::new().unwrap();
        let lock_path = tmp.path().join("proxy.lock");
        let sock_path = tmp.path().join("proxy.sock");

        let info = ProxyLockInfo {
            pid: 4_000_000,
            port: 8100,
            uds_path: sock_path.to_string_lossy().to_string(),
        };
        let json = serde_json::to_string_pretty(&info).unwrap();
        std::fs::write(&lock_path, &json).unwrap();

        std::fs::write(&sock_path, "fake").unwrap();

        use nix::sys::signal::kill;
        use nix::unistd::Pid;
        let pid = Pid::from_raw(4_000_000);
        assert!(kill(pid, None).is_err(), "PID 4000000 should not exist");

        let contents = std::fs::read_to_string(&lock_path).unwrap();
        let parsed: ProxyLockInfo = serde_json::from_str(&contents).unwrap();
        assert_eq!(parsed.pid, 4_000_000);
    }

    #[test]
    fn test_lock_info_with_current_pid_is_alive() {
        use nix::sys::signal::kill;
        use nix::unistd::Pid;

        let pid = Pid::from_raw(std::process::id() as i32);
        assert!(kill(pid, None).is_ok(), "Current process should be alive");
    }

    #[test]
    fn test_app_support_dir_is_under_data_dir() {
        let dir = app_support_dir().unwrap();
        let data = dirs::data_dir().unwrap();
        assert!(dir.starts_with(&data));
        assert!(dir.ends_with("com.cheolsu-proxy"));
    }

    #[test]
    fn test_lock_file_path_ends_with_proxy_lock() {
        let path = lock_file_path().unwrap();
        assert!(path.ends_with("proxy.lock"));
    }

    #[test]
    fn test_uds_socket_path_ends_with_proxy_sock() {
        let path = uds_socket_path().unwrap();
        assert!(path.ends_with("proxy.sock"));
    }

    #[test]
    fn test_newline_delimited_protocol_multiple_messages() {
        let messages = vec![
            DaemonMessage::Status {
                running: true,
                port: 8100,
            },
            DaemonMessage::Status {
                running: false,
                port: 8100,
            },
        ];

        let mut wire = String::new();
        for msg in &messages {
            wire.push_str(&serde_json::to_string(msg).unwrap());
            wire.push('\n');
        }

        let parsed: Vec<DaemonMessage> = wire
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();

        assert_eq!(parsed.len(), 2);
        match &parsed[0] {
            DaemonMessage::Status { running, port } => {
                assert!(*running);
                assert_eq!(*port, 8100);
            }
            _ => panic!("Expected Status"),
        }
        match &parsed[1] {
            DaemonMessage::Status { running, port } => {
                assert!(!*running);
                assert_eq!(*port, 8100);
            }
            _ => panic!("Expected Status"),
        }
    }

    #[test]
    fn test_mixed_commands_newline_protocol() {
        let commands = vec![ClientCommand::Subscribe, ClientCommand::Stop];

        let mut wire = String::new();
        for cmd in &commands {
            wire.push_str(&serde_json::to_string(cmd).unwrap());
            wire.push('\n');
        }

        let parsed: Vec<ClientCommand> = wire
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();

        assert_eq!(parsed.len(), 2);
        assert!(matches!(parsed[0], ClientCommand::Subscribe));
        assert!(matches!(parsed[1], ClientCommand::Stop));
    }

    #[test]
    fn test_intercept_rule_block_serialization() {
        use crate::protocol::{InterceptAction, InterceptRule};
        let rule = InterceptRule {
            id: "r1".to_string(),
            name: "Block ads".to_string(),
            enabled: true,
            pattern: "*ads.example.com*".to_string(),
            method: None,
            action: InterceptAction::Block {
                status_code: 403,
                body: "Blocked".to_string(),
            },
        };
        let json = serde_json::to_string(&rule).unwrap();
        let parsed: InterceptRule = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "r1");
        assert_eq!(parsed.pattern, "*ads.example.com*");
        assert!(parsed.enabled);
        match parsed.action {
            InterceptAction::Block { status_code, body } => {
                assert_eq!(status_code, 403);
                assert_eq!(body, "Blocked");
            }
            _ => panic!("Expected Block action"),
        }
    }

    #[test]
    fn test_intercept_rule_modify_response_serialization() {
        use crate::protocol::{InterceptAction, InterceptRule};
        let mut headers = std::collections::HashMap::new();
        headers.insert("X-Test".to_string(), "value".to_string());
        let rule = InterceptRule {
            id: "r2".to_string(),
            name: "Modify API".to_string(),
            enabled: true,
            pattern: "*api.example.com*".to_string(),
            method: Some("GET".to_string()),
            action: InterceptAction::ModifyResponse {
                set_status: Some(200),
                add_headers: headers,
                remove_headers: vec!["X-Remove".to_string()],
                set_body: Some(r#"{"mocked":true}"#.to_string()),
            },
        };
        let json = serde_json::to_string(&rule).unwrap();
        let parsed: InterceptRule = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.pattern, "*api.example.com*");
        assert_eq!(parsed.method, Some("GET".to_string()));
        match parsed.action {
            InterceptAction::ModifyResponse {
                set_status,
                set_body,
                ..
            } => {
                assert_eq!(set_status, Some(200));
                assert_eq!(set_body, Some(r#"{"mocked":true}"#.to_string()));
            }
            _ => panic!("Expected ModifyResponse action"),
        }
    }

    #[test]
    fn test_update_intercept_rules_command_serialization() {
        use crate::protocol::{InterceptAction, InterceptRule};
        let rules = vec![InterceptRule {
            id: "r1".to_string(),
            name: "Test".to_string(),
            enabled: true,
            pattern: "*test.com*".to_string(),
            method: None,
            action: InterceptAction::Block {
                status_code: 403,
                body: String::new(),
            },
        }];
        let cmd = ClientCommand::UpdateInterceptRules {
            rules: rules.clone(),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["cmd"], "update_intercept_rules");
        assert_eq!(parsed["rules"].as_array().unwrap().len(), 1);

        let parsed_cmd: ClientCommand = serde_json::from_str(&json).unwrap();
        match parsed_cmd {
            ClientCommand::UpdateInterceptRules { rules } => {
                assert_eq!(rules.len(), 1);
                assert_eq!(rules[0].id, "r1");
            }
            _ => panic!("Expected UpdateInterceptRules"),
        }
    }

    #[test]
    fn test_intercept_rule_json_deserialization() {
        let json = r#"{
            "id": "r1",
            "name": "Block",
            "enabled": true,
            "pattern": "*ads*",
            "action": {
                "type": "block",
                "status_code": 403,
                "body": "No"
            }
        }"#;
        let rule: crate::protocol::InterceptRule = serde_json::from_str(json).unwrap();
        assert_eq!(rule.id, "r1");
        assert_eq!(rule.pattern, "*ads*");
        assert!(rule.method.is_none());
    }

    #[test]
    fn test_intercept_rule_modify_request_deserialization() {
        let json = r#"{
            "id": "r3",
            "name": "Add header",
            "enabled": true,
            "pattern": "*api.test.com*",
            "method": "POST",
            "action": {
                "type": "modify_request",
                "add_headers": {"Authorization": "Bearer token123"},
                "remove_headers": ["Cookie"]
            }
        }"#;
        let rule: crate::protocol::InterceptRule = serde_json::from_str(json).unwrap();
        assert_eq!(rule.id, "r3");
        assert_eq!(rule.pattern, "*api.test.com*");
        assert_eq!(rule.method, Some("POST".to_string()));
        match rule.action {
            crate::protocol::InterceptAction::ModifyRequest {
                add_headers,
                remove_headers,
                set_body,
            } => {
                assert_eq!(add_headers.get("Authorization").unwrap(), "Bearer token123");
                assert_eq!(remove_headers, vec!["Cookie"]);
                assert!(set_body.is_none());
            }
            _ => panic!("Expected ModifyRequest"),
        }
    }
}

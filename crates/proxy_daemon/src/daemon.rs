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

    let (session_tx, session_rx) = watch::channel(serde_json::Value::Array(Vec::new()));

    let addr: std::net::SocketAddr = format!("{}:{}", host, port)
        .parse()
        .expect("Invalid host:port");

    if let Err(e) = set_proxy(true, port) {
        error!("Failed to set system proxy: {}", e);
    }

    let event_tx_proxy = event_tx.clone();
    let proxy_handle = tokio::spawn(async move {
        if let Err(code) = run_proxy(addr, event_tx_proxy, session_rx).await {
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
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
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
                        let client_count_clone = client_count.clone();
                        let shutdown_tx_clone = shutdown_tx.clone();
                        let session_tx_clone = session_tx.clone();

                        tokio::spawn(async move {
                            handle_client(stream, event_rx, session_tx_clone, port)
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
    mut session_rx: watch::Receiver<serde_json::Value>,
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

    let handler = LoggingHandler::new(tx.clone(), cache_dir);

    {
        let sessions = session_rx.borrow().clone();
        handler.update_sessions(sessions).await;
    }

    let handler_for_updates = handler.clone();
    tokio::spawn(async move {
        while session_rx.changed().await.is_ok() {
            let sessions = session_rx.borrow().clone();
            handler_for_updates.update_sessions(sessions).await;
        }
    });

    let hybrid_client =
        create_hybrid_client().map_err(|e| format!("Client creation failed: {}", e))?;

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Port {} bind failed: {}", addr.port(), e))?;

    let proxy_builder = ProxyBuilder::new()
        .with_listener(listener)
        .with_ca(ca)
        .with_client(hybrid_client)
        .with_http_handler(handler.clone())
        .with_websocket_handler(handler.clone())
        .with_tunnel_event_sender(tunnel_tx)
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
    session_tx: watch::Sender<serde_json::Value>,
    port: u16,
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
                    Ok(ClientCommand::UpdateSessions { data }) => {
                        let _ = session_tx.send(data);
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
    fn test_client_command_update_sessions_serialization() {
        let data = serde_json::json!([{"url": "https://example.com", "method": "GET"}]);
        let cmd = ClientCommand::UpdateSessions { data: data.clone() };
        let json = serde_json::to_string(&cmd).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["cmd"], "update_sessions");
        assert_eq!(parsed["data"], data);
    }

    #[test]
    fn test_client_command_update_sessions_deserialization() {
        let json = r#"{"cmd":"update_sessions","data":[{"url":"https://example.com"}]}"#;
        let cmd: ClientCommand = serde_json::from_str(json).unwrap();
        match cmd {
            ClientCommand::UpdateSessions { data } => {
                assert!(data.is_array());
                assert_eq!(data.as_array().unwrap().len(), 1);
            }
            _ => panic!("Expected UpdateSessions"),
        }
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
        let commands = vec![
            ClientCommand::Subscribe,
            ClientCommand::UpdateSessions {
                data: serde_json::json!([]),
            },
            ClientCommand::Stop,
        ];

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

        assert_eq!(parsed.len(), 3);
        assert!(matches!(parsed[0], ClientCommand::Subscribe));
        assert!(matches!(parsed[1], ClientCommand::UpdateSessions { .. }));
        assert!(matches!(parsed[2], ClientCommand::Stop));
    }
}

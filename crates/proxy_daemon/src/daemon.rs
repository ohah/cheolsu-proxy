use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::net::UnixListener;
use tokio::sync::{broadcast, watch};
use tracing::{error, info, warn};

use crate::breakpoint::BreakpointManager;
use crate::client_handler::handle_client;
use crate::error::DaemonError;
use crate::protocol::ProxyLockInfo;
use crate::proxy_runner::run_proxy;
use crate::system_proxy::set_proxy;
use proxyapi_v2::throttle::ThrottleConfig;
use proxyapi_v2::upstream_proxy::UpstreamProxyConfig;
use proxyapi_v2::websocket_registry::WebSocketRegistry;

pub fn app_support_dir() -> Result<PathBuf, DaemonError> {
    dirs::data_dir()
        .ok_or(DaemonError::DataDirNotFound)
        .map(|dir| dir.join("com.cheolsu-proxy"))
}

pub fn lock_file_path() -> Result<PathBuf, DaemonError> {
    Ok(app_support_dir()?.join("proxy.lock"))
}

pub fn uds_socket_path() -> Result<PathBuf, DaemonError> {
    Ok(app_support_dir()?.join("proxy.sock"))
}

fn write_lock_file(port: u16, uds_path: &str) -> Result<(), DaemonError> {
    let dir = app_support_dir()?;
    std::fs::create_dir_all(&dir)?;
    let info = ProxyLockInfo {
        pid: std::process::id(),
        port,
        uds_path: uds_path.to_string(),
    };
    let json = serde_json::to_string_pretty(&info)?;
    std::fs::write(lock_file_path()?, json)?;
    Ok(())
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

    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("Failed to create tokio runtime: {}", e);
            std::process::exit(1);
        }
    };

    let exit_code = rt.block_on(async move { daemon_main(port, host).await });
    std::process::exit(exit_code)
}

/// 파일시스템 초기화: stale lock 정리, UDS 경로 확보, lock 파일 작성.
fn init_filesystem(port: u16) -> Result<(PathBuf, String), i32> {
    if !check_and_cleanup_stale_lock() {
        error!("Another daemon is already running. Exiting.");
        return Err(1);
    }

    let uds_path = match uds_socket_path() {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to get UDS socket path: {}", e);
            return Err(1);
        }
    };
    let uds_path_str = uds_path.to_string_lossy().to_string();

    remove_uds_socket(&uds_path);

    if let Some(parent) = uds_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    if let Err(e) = write_lock_file(port, &uds_path_str) {
        error!("Failed to write lock file: {}", e);
        return Err(1);
    }

    Ok((uds_path, uds_path_str))
}

/// 데몬에서 사용하는 채널 및 공유 상태를 묶는 컨텍스트
struct DaemonContext {
    event_tx: broadcast::Sender<String>,
    client_count: Arc<AtomicUsize>,
    shutdown_tx: tokio::sync::mpsc::Sender<()>,
    intercept_tx: watch::Sender<Vec<crate::protocol::InterceptRule>>,
    upstream_tx: watch::Sender<Option<UpstreamProxyConfig>>,
    server_replay_tx: watch::Sender<Vec<crate::protocol::ServerReplayEntry>>,
    throttle_tx: watch::Sender<Option<ThrottleConfig>>,
    breakpoint_tx: watch::Sender<Vec<crate::protocol::BreakpointRule>>,
    breakpoint_manager: BreakpointManager,
    ws_registry: WebSocketRegistry,
    script_handle: scripting::ScriptHandle,
}

/// 프록시 태스크를 스폰합니다.
fn spawn_proxy_task(
    addr: std::net::SocketAddr,
    event_tx: broadcast::Sender<String>,
    intercept_rx: watch::Receiver<Vec<crate::protocol::InterceptRule>>,
    upstream_rx: watch::Receiver<Option<UpstreamProxyConfig>>,
    server_replay_rx: watch::Receiver<Vec<crate::protocol::ServerReplayEntry>>,
    throttle_rx: watch::Receiver<Option<ThrottleConfig>>,
    breakpoint_rx: watch::Receiver<Vec<crate::protocol::BreakpointRule>>,
    breakpoint_manager: BreakpointManager,
    ws_registry: WebSocketRegistry,
    script_handle: scripting::ScriptHandle,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(code) = run_proxy(
            addr,
            event_tx,
            intercept_rx,
            upstream_rx,
            server_replay_rx,
            throttle_rx,
            breakpoint_rx,
            breakpoint_manager,
            ws_registry,
            script_handle,
        )
        .await
        {
            error!("Proxy error: {}", code);
        }
    })
}

/// Ctrl+C 및 SIGTERM 시그널 핸들러를 등록합니다.
fn spawn_signal_handlers(shutdown_tx: tokio::sync::mpsc::Sender<()>) {
    let shutdown_tx_ctrlc = shutdown_tx.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        warn!("Ctrl+C received, shutting down daemon...");
        let _ = shutdown_tx_ctrlc.send(()).await;
    });

    let shutdown_tx_term = shutdown_tx;
    tokio::spawn(async move {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sigterm) => {
                sigterm.recv().await;
                warn!("SIGTERM received, shutting down daemon...");
                let _ = shutdown_tx_term.send(()).await;
            }
            Err(e) => {
                error!("Failed to register SIGTERM handler: {}", e);
            }
        }
    });
}

/// UDS accept 루프를 실행합니다. shutdown 시그널 수신 시 종료.
async fn run_accept_loop(
    uds_listener: UnixListener,
    shutdown_rx: &mut tokio::sync::mpsc::Receiver<()>,
    ctx: &DaemonContext,
    port: u16,
) {
    loop {
        tokio::select! {
            accept_result = uds_listener.accept() => {
                match accept_result {
                    Ok((stream, _addr)) => {
                        let count = ctx.client_count.fetch_add(1, Ordering::SeqCst) + 1;
                        info!("Client connected (total: {})", count);

                        let event_rx = ctx.event_tx.subscribe();
                        let event_tx_clone = ctx.event_tx.clone();
                        let client_count_clone = ctx.client_count.clone();
                        let shutdown_tx_clone = ctx.shutdown_tx.clone();
                        let intercept_tx_clone = ctx.intercept_tx.clone();
                        let upstream_tx_clone = ctx.upstream_tx.clone();
                        let server_replay_tx_clone = ctx.server_replay_tx.clone();
                        let throttle_tx_clone = ctx.throttle_tx.clone();
                        let breakpoint_tx_clone = ctx.breakpoint_tx.clone();
                        let breakpoint_mgr_clone = ctx.breakpoint_manager.clone();
                        let registry_clone = ctx.ws_registry.clone();
                        let script_handle_clone = ctx.script_handle.clone();

                        tokio::spawn(async move {
                            handle_client(stream, event_rx, intercept_tx_clone, upstream_tx_clone, server_replay_tx_clone, throttle_tx_clone, breakpoint_tx_clone, breakpoint_mgr_clone, event_tx_clone, port, registry_clone, script_handle_clone)
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
}

async fn daemon_main(port: u16, host: String) -> i32 {
    let (uds_path, uds_path_str) = match init_filesystem(port) {
        Ok(result) => result,
        Err(code) => return code,
    };

    let (event_tx, _) = broadcast::channel::<String>(256);
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);
    let (intercept_tx, intercept_rx) =
        watch::channel::<Vec<crate::protocol::InterceptRule>>(Vec::new());
    let (upstream_tx, upstream_rx) = watch::channel::<Option<UpstreamProxyConfig>>(None);
    let (server_replay_tx, server_replay_rx) =
        watch::channel::<Vec<crate::protocol::ServerReplayEntry>>(Vec::new());
    let (throttle_tx, throttle_rx) = watch::channel::<Option<ThrottleConfig>>(None);
    let (breakpoint_tx, breakpoint_rx) =
        watch::channel::<Vec<crate::protocol::BreakpointRule>>(Vec::new());
    let breakpoint_manager = BreakpointManager::new(event_tx.clone());

    let addr: std::net::SocketAddr = match format!("{}:{}", host, port).parse() {
        Ok(addr) => addr,
        Err(e) => {
            error!("Invalid host:port {}:{} - {}", host, port, e);
            cleanup(port, &uds_path);
            return 1;
        }
    };

    if let Err(e) = set_proxy(true, port) {
        error!("Failed to set system proxy: {}", e);
    }

    let ws_registry = WebSocketRegistry::new();
    let script_handle = scripting::ScriptHandle::new();

    let proxy_handle = spawn_proxy_task(
        addr,
        event_tx.clone(),
        intercept_rx,
        upstream_rx,
        server_replay_rx,
        throttle_rx,
        breakpoint_rx,
        breakpoint_manager.clone(),
        ws_registry.clone(),
        script_handle.clone(),
    );

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

    spawn_signal_handlers(shutdown_tx.clone());

    let ctx = DaemonContext {
        event_tx,
        client_count: Arc::new(AtomicUsize::new(0)),
        shutdown_tx,
        intercept_tx,
        upstream_tx,
        server_replay_tx,
        throttle_tx,
        breakpoint_tx,
        breakpoint_manager,
        ws_registry,
        script_handle,
    };

    run_accept_loop(uds_listener, &mut shutdown_rx, &ctx, port).await;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{ClientCommand, DaemonMessage};
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

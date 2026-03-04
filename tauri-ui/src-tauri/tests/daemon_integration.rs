use proxy_daemon::daemon::handle_client;
use proxy_daemon::{ClientCommand, DaemonMessage, ProxyLockInfo};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::broadcast;

/// Helper: create a temp UDS path (returns TempDir to prevent premature cleanup)
fn temp_uds_path() -> (std::path::PathBuf, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("test.sock");
    (path, dir)
}

/// Helper: start a minimal UDS server that uses handle_client
async fn start_test_server(
    uds_path: &std::path::Path,
    event_tx: broadcast::Sender<String>,
    session_tx: tokio::sync::watch::Sender<serde_json::Value>,
    port: u16,
) -> (
    tokio::task::JoinHandle<()>,
    Arc<AtomicUsize>,
    tokio::sync::mpsc::Sender<()>,
) {
    let listener = UnixListener::bind(uds_path).unwrap();
    let client_count = Arc::new(AtomicUsize::new(0));
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);

    let cc = client_count.clone();
    let stx = shutdown_tx.clone();
    let etx = event_tx.clone();

    let handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((stream, _)) => {
                            let count = cc.fetch_add(1, Ordering::SeqCst) + 1;
                            let _ = count;

                            let event_rx = etx.subscribe();
                            let cc2 = cc.clone();
                            let stx2 = stx.clone();
                            let session_tx2 = session_tx.clone();

                            tokio::spawn(async move {
                                handle_client(stream, event_rx, session_tx2, port).await;
                                let remaining = cc2.fetch_sub(1, Ordering::SeqCst) - 1;
                                if remaining == 0 {
                                    let _ = stx2.send(()).await;
                                }
                            });
                        }
                        Err(_) => break,
                    }
                }
                _ = shutdown_rx.recv() => {
                    break;
                }
            }
        }
    });

    (handle, client_count, shutdown_tx)
}

// ============================================================================
// Integration Tests
// ============================================================================

#[tokio::test]
async fn test_client_receives_status_on_connect() {
    let (uds_path, _tmp) = temp_uds_path();
    let (event_tx, _) = broadcast::channel::<String>(16);
    let (session_tx, _session_rx) = tokio::sync::watch::channel(serde_json::json!([]));

    let (server_handle, _, shutdown_tx) =
        start_test_server(&uds_path, event_tx, session_tx, 8100).await;

    // Connect as client
    let stream = UnixStream::connect(&uds_path).await.unwrap();
    let (reader, _writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(reader);

    // First message should be a status message
    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();

    let msg: DaemonMessage = serde_json::from_str(line.trim()).unwrap();
    match msg {
        DaemonMessage::Status { running, port } => {
            assert!(running);
            assert_eq!(port, 8100);
        }
        _ => panic!("Expected Status message, got {:?}", msg),
    }

    // Cleanup
    let _ = shutdown_tx.send(()).await;
    server_handle.abort();
}

#[tokio::test]
async fn test_subscribe_and_receive_events() {
    let (uds_path, _tmp) = temp_uds_path();
    let (event_tx, _) = broadcast::channel::<String>(16);
    let (session_tx, _session_rx) = tokio::sync::watch::channel(serde_json::json!([]));

    let (server_handle, _, shutdown_tx) =
        start_test_server(&uds_path, event_tx.clone(), session_tx, 8100).await;

    // Connect as client
    let stream = UnixStream::connect(&uds_path).await.unwrap();
    let (reader, writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(reader);
    let mut writer = writer;

    // Read initial status
    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();
    assert!(line.contains("status"));

    // Send subscribe command
    let sub_cmd = serde_json::to_string(&ClientCommand::Subscribe).unwrap();
    writer
        .write_all(format!("{}\n", sub_cmd).as_bytes())
        .await
        .unwrap();
    writer.flush().await.unwrap();

    // Give the server a moment to process subscribe
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Broadcast an event from the server side
    let test_event_msg = r#"{"type":"status","running":true,"port":9999}"#;
    event_tx.send(test_event_msg.to_string()).unwrap();

    // Client should receive the event
    let mut event_line = String::new();
    let read_result = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        reader.read_line(&mut event_line),
    )
    .await;

    assert!(read_result.is_ok(), "Should receive event within timeout");
    let bytes_read = read_result.unwrap().unwrap();
    assert!(bytes_read > 0, "Should have read some bytes");
    assert!(
        event_line.contains("9999"),
        "Event should contain port 9999"
    );

    // Cleanup
    let _ = shutdown_tx.send(()).await;
    server_handle.abort();
}

#[tokio::test]
async fn test_unsubscribed_client_does_not_receive_events() {
    let (uds_path, _tmp) = temp_uds_path();
    let (event_tx, _) = broadcast::channel::<String>(16);
    let (session_tx, _session_rx) = tokio::sync::watch::channel(serde_json::json!([]));

    let (server_handle, _, shutdown_tx) =
        start_test_server(&uds_path, event_tx.clone(), session_tx, 8100).await;

    // Connect as client but do NOT subscribe
    let stream = UnixStream::connect(&uds_path).await.unwrap();
    let (reader, _writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(reader);

    // Read initial status
    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();

    // Broadcast an event
    let _ = event_tx.send(r#"{"type":"status","running":true,"port":1111}"#.to_string());

    // Client should NOT receive the event (timeout expected)
    let mut event_line = String::new();
    let read_result = tokio::time::timeout(
        std::time::Duration::from_millis(300),
        reader.read_line(&mut event_line),
    )
    .await;

    assert!(
        read_result.is_err(),
        "Unsubscribed client should not receive events (should timeout)"
    );

    // Cleanup
    let _ = shutdown_tx.send(()).await;
    server_handle.abort();
}

#[tokio::test]
async fn test_update_sessions_command() {
    let (uds_path, _tmp) = temp_uds_path();
    let (event_tx, _) = broadcast::channel::<String>(16);
    let (session_tx, session_rx) = tokio::sync::watch::channel(serde_json::json!([]));

    let (server_handle, _, shutdown_tx) =
        start_test_server(&uds_path, event_tx, session_tx, 8100).await;

    // Connect as client
    let stream = UnixStream::connect(&uds_path).await.unwrap();
    let (reader, writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(reader);
    let mut writer = writer;

    // Read initial status
    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();

    // Send update_sessions command
    let new_sessions = serde_json::json!([
        {"url": "https://api.example.com/users", "method": "GET"},
        {"url": "https://api.example.com/posts", "method": "POST"}
    ]);
    let cmd = ClientCommand::UpdateSessions {
        data: new_sessions.clone(),
    };
    let cmd_json = serde_json::to_string(&cmd).unwrap();
    writer
        .write_all(format!("{}\n", cmd_json).as_bytes())
        .await
        .unwrap();
    writer.flush().await.unwrap();

    // Give the server time to process
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Verify sessions were updated via watch channel
    let current_sessions = session_rx.borrow().clone();
    assert_eq!(current_sessions, new_sessions);

    // Cleanup
    let _ = shutdown_tx.send(()).await;
    server_handle.abort();
}

#[tokio::test]
async fn test_multiple_clients_connect() {
    let (uds_path, _tmp) = temp_uds_path();
    let (event_tx, _) = broadcast::channel::<String>(16);
    let (session_tx, _session_rx) = tokio::sync::watch::channel(serde_json::json!([]));

    let (server_handle, client_count, shutdown_tx) =
        start_test_server(&uds_path, event_tx.clone(), session_tx, 8100).await;

    // Connect client 1
    let stream1 = UnixStream::connect(&uds_path).await.unwrap();
    let (r1, mut w1) = tokio::io::split(stream1);
    let mut r1 = BufReader::new(r1);
    let mut line1 = String::new();
    r1.read_line(&mut line1).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert_eq!(client_count.load(Ordering::SeqCst), 1);

    // Connect client 2
    let stream2 = UnixStream::connect(&uds_path).await.unwrap();
    let (r2, mut w2) = tokio::io::split(stream2);
    let mut r2 = BufReader::new(r2);
    let mut line2 = String::new();
    r2.read_line(&mut line2).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert_eq!(client_count.load(Ordering::SeqCst), 2);

    // Both subscribe
    let sub = format!(
        "{}\n",
        serde_json::to_string(&ClientCommand::Subscribe).unwrap()
    );
    w1.write_all(sub.as_bytes()).await.unwrap();
    w1.flush().await.unwrap();
    w2.write_all(sub.as_bytes()).await.unwrap();
    w2.flush().await.unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Broadcast event — both should receive it
    let _ = event_tx.send(r#"{"type":"status","running":true,"port":7777}"#.to_string());

    let mut ev1 = String::new();
    let mut ev2 = String::new();

    let res1 =
        tokio::time::timeout(std::time::Duration::from_secs(2), r1.read_line(&mut ev1)).await;
    let res2 =
        tokio::time::timeout(std::time::Duration::from_secs(2), r2.read_line(&mut ev2)).await;

    assert!(res1.is_ok() && res1.unwrap().unwrap() > 0);
    assert!(res2.is_ok() && res2.unwrap().unwrap() > 0);
    assert!(ev1.contains("7777"));
    assert!(ev2.contains("7777"));

    // Cleanup
    let _ = shutdown_tx.send(()).await;
    server_handle.abort();
}

#[tokio::test]
async fn test_client_disconnect_decrements_count() {
    let (uds_path, _tmp) = temp_uds_path();
    let (event_tx, _) = broadcast::channel::<String>(16);
    let (session_tx, _session_rx) = tokio::sync::watch::channel(serde_json::json!([]));

    let (server_handle, client_count, shutdown_tx) =
        start_test_server(&uds_path, event_tx, session_tx, 8100).await;

    // Connect client
    let stream = UnixStream::connect(&uds_path).await.unwrap();
    let (reader, _writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert_eq!(client_count.load(Ordering::SeqCst), 1);

    // Drop the client (disconnect)
    drop(reader);
    drop(_writer);

    // Wait for server to detect disconnection
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Client count should be 0
    assert_eq!(client_count.load(Ordering::SeqCst), 0);

    // Cleanup
    let _ = shutdown_tx.send(()).await;
    server_handle.abort();
}

#[tokio::test]
async fn test_auto_shutdown_when_all_clients_disconnect() {
    let (uds_path, _tmp) = temp_uds_path();
    let (event_tx, _) = broadcast::channel::<String>(16);
    let (session_tx, _session_rx) = tokio::sync::watch::channel(serde_json::json!([]));

    let (server_handle, client_count, _shutdown_tx) =
        start_test_server(&uds_path, event_tx, session_tx, 8100).await;

    // Connect two clients
    let stream1 = UnixStream::connect(&uds_path).await.unwrap();
    let stream2 = UnixStream::connect(&uds_path).await.unwrap();

    // Read status from both
    let (r1, w1) = tokio::io::split(stream1);
    let mut r1 = BufReader::new(r1);
    let mut l1 = String::new();
    r1.read_line(&mut l1).await.unwrap();

    let (r2, w2) = tokio::io::split(stream2);
    let mut r2 = BufReader::new(r2);
    let mut l2 = String::new();
    r2.read_line(&mut l2).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert_eq!(client_count.load(Ordering::SeqCst), 2);

    // Disconnect client 1
    drop(r1);
    drop(w1);
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert_eq!(client_count.load(Ordering::SeqCst), 1);

    // Disconnect client 2 — should trigger auto-shutdown
    drop(r2);
    drop(w2);
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert_eq!(client_count.load(Ordering::SeqCst), 0);

    // Server should have received shutdown signal and exited
    let server_done = tokio::time::timeout(std::time::Duration::from_secs(2), server_handle).await;
    assert!(
        server_done.is_ok(),
        "Server should shut down after all clients disconnect"
    );
}

#[tokio::test]
async fn test_stop_command_disconnects_client() {
    let (uds_path, _tmp) = temp_uds_path();
    let (event_tx, _) = broadcast::channel::<String>(16);
    let (session_tx, _session_rx) = tokio::sync::watch::channel(serde_json::json!([]));

    let (server_handle, client_count, _shutdown_tx) =
        start_test_server(&uds_path, event_tx, session_tx, 8100).await;

    // Connect client
    let stream = UnixStream::connect(&uds_path).await.unwrap();
    let (reader, writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(reader);
    let mut writer = writer;

    // Read status
    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert_eq!(client_count.load(Ordering::SeqCst), 1);

    // Send stop command
    let stop = format!("{}\n", serde_json::to_string(&ClientCommand::Stop).unwrap());
    writer.write_all(stop.as_bytes()).await.unwrap();
    writer.flush().await.unwrap();

    // Wait for server to process stop and disconnect
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert_eq!(client_count.load(Ordering::SeqCst), 0);

    // Server should auto-shutdown (single client stop → 0 remaining)
    let server_done = tokio::time::timeout(std::time::Duration::from_secs(2), server_handle).await;
    assert!(server_done.is_ok(), "Server should shut down after stop");
}

// --- Lock File Integration Tests ---

#[test]
fn test_lock_file_write_and_read() {
    let tmp = tempfile::TempDir::new().unwrap();
    let lock_path = tmp.path().join("proxy.lock");

    let info = ProxyLockInfo {
        pid: std::process::id(),
        port: 8100,
        uds_path: "/tmp/test.sock".to_string(),
    };
    let json = serde_json::to_string_pretty(&info).unwrap();
    std::fs::write(&lock_path, &json).unwrap();

    // Read back
    let contents = std::fs::read_to_string(&lock_path).unwrap();
    let parsed: ProxyLockInfo = serde_json::from_str(&contents).unwrap();
    assert_eq!(parsed.pid, std::process::id());
    assert_eq!(parsed.port, 8100);
    assert_eq!(parsed.uds_path, "/tmp/test.sock");
}

#[test]
fn test_stale_lock_detection_with_dead_pid() {
    let tmp = tempfile::TempDir::new().unwrap();
    let lock_path = tmp.path().join("proxy.lock");
    let sock_path = tmp.path().join("proxy.sock");

    // Write lock with a definitely-dead PID
    let info = ProxyLockInfo {
        pid: 4_000_000,
        port: 8100,
        uds_path: sock_path.to_string_lossy().to_string(),
    };
    std::fs::write(&lock_path, serde_json::to_string(&info).unwrap()).unwrap();
    std::fs::write(&sock_path, "fake").unwrap();

    // PID should be dead
    use nix::sys::signal::kill;
    use nix::unistd::Pid;
    assert!(kill(Pid::from_raw(4_000_000), None).is_err());

    // Lock file and socket exist
    assert!(lock_path.exists());
    assert!(sock_path.exists());
}

#[test]
fn test_alive_pid_is_not_stale() {
    // Current process should be alive
    use nix::sys::signal::kill;
    use nix::unistd::Pid;
    let pid = Pid::from_raw(std::process::id() as i32);
    assert!(kill(pid, None).is_ok());
}

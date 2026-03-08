use std::io::Write;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::Mutex;

use crate::daemon::{check_and_cleanup_stale_lock, lock_file_path, uds_socket_path};
use crate::error::DaemonError;
use crate::protocol::{ClientCommand, DaemonMessage, ProxyLockInfo};

/// A connection to the daemon process.
pub struct DaemonConnection {
    writer: Mutex<tokio::io::WriteHalf<UnixStream>>,
    /// Background task that reads events from the UDS and forwards them.
    event_task: Option<tokio::task::JoinHandle<()>>,
    pub port: u16,
}

impl Drop for DaemonConnection {
    fn drop(&mut self) {
        if let Some(task) = self.event_task.take() {
            task.abort();
        }
    }
}

impl DaemonConnection {
    /// Disconnect from the daemon (does NOT send stop command).
    pub async fn disconnect(mut self) {
        if let Some(task) = self.event_task.take() {
            task.abort();
        }
    }

    /// Send a command to the daemon.
    pub async fn send_command(&self, cmd: &ClientCommand) -> Result<(), DaemonError> {
        let mut line = serde_json::to_string(cmd)?;
        line.push('\n');
        let mut w = self.writer.lock().await;
        w.write_all(line.as_bytes())
            .await
            .map_err(|e| DaemonError::Uds(format!("write: {}", e)))?;
        w.flush()
            .await
            .map_err(|e| DaemonError::Uds(format!("flush: {}", e)))?;
        Ok(())
    }
}

/// Check if a daemon is running by reading the lock file and verifying the PID.
pub fn is_daemon_running() -> Option<ProxyLockInfo> {
    let lock_path = lock_file_path().ok()?;
    if !lock_path.exists() {
        return None;
    }

    let contents = std::fs::read_to_string(&lock_path).ok()?;
    let info: ProxyLockInfo = serde_json::from_str(&contents).ok()?;

    use nix::sys::signal::kill;
    use nix::unistd::Pid;
    let pid = Pid::from_raw(info.pid as i32);
    if kill(pid, None).is_ok() {
        Some(info)
    } else {
        None
    }
}

/// Spawn a new daemon process using the current executable.
fn spawn_daemon(port: u16, host: &str) -> Result<(), DaemonError> {
    let exe = std::env::current_exe()?;

    let mut child = std::process::Command::new(exe)
        .arg("--daemon")
        .arg("--port")
        .arg(port.to_string())
        .arg("--host")
        .arg(host)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let log_file = daemon_log_file();

    // stdout → 로그 파일만 (터미널 출력 시 TUI 화면 깨짐)
    if let Some(stdout) = child.stdout.take() {
        let log_file_clone = log_file.as_ref().and_then(|f| f.try_clone().ok());
        std::thread::spawn(move || {
            log_stream(stdout, log_file_clone);
        });
    }

    // stderr → 로그 파일만
    if let Some(stderr) = child.stderr.take() {
        let log_file_clone = log_file.as_ref().and_then(|f| f.try_clone().ok());
        std::thread::spawn(move || {
            log_stream(stderr, log_file_clone);
        });
    }

    Ok(())
}

/// 스트림을 로그 파일에만 출력합니다. (터미널 출력 시 TUI 화면 깨짐 방지)
fn log_stream(source: impl std::io::Read, mut log_file: Option<std::fs::File>) {
    use std::io::BufRead;
    let reader = std::io::BufReader::new(source);
    for line in reader.lines() {
        let Ok(line) = line else { break };
        if let Some(ref mut f) = log_file {
            let _ = writeln!(f, "{}", line);
        }
    }
}

/// Daemon 로그 파일을 생성하여 반환합니다.
fn daemon_log_file() -> Option<std::fs::File> {
    let log_dir = crate::daemon::app_support_dir().ok()?;
    std::fs::create_dir_all(&log_dir).ok()?;
    let log_path = log_dir.join("daemon.log");
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .ok()
}

/// Ensure a daemon is running and connect to it.
/// If no daemon is running, spawns one and waits for it to be ready.
/// Returns a `DaemonConnection` with an active event-forwarding task.
///
/// `on_message` is called for each `DaemonMessage` received from the daemon.
pub async fn ensure_daemon<F>(
    port: u16,
    host: &str,
    on_message: F,
) -> Result<DaemonConnection, DaemonError>
where
    F: Fn(DaemonMessage) + Send + 'static,
{
    if is_daemon_running().is_none() {
        check_and_cleanup_stale_lock();

        spawn_daemon(port, host)?;

        let mut ready = false;
        for _ in 0..50 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if is_daemon_running().is_some() {
                ready = true;
                break;
            }
        }
        if !ready {
            return Err(DaemonError::Daemon(
                "Daemon did not start within 5 seconds".to_string(),
            ));
        }
    }

    // Wait for the UDS socket file to actually exist.
    let uds_path = uds_socket_path()?;
    let mut socket_ready = false;
    for _ in 0..50 {
        if uds_path.exists() {
            socket_ready = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    if !socket_ready {
        return Err(DaemonError::Daemon(
            "UDS socket was not created within 5 seconds".to_string(),
        ));
    }

    connect_to_daemon(on_message).await
}

/// Connect to an already-running daemon.
/// `on_message` is called for each `DaemonMessage` received.
pub async fn connect_to_daemon<F>(on_message: F) -> Result<DaemonConnection, DaemonError>
where
    F: Fn(DaemonMessage) + Send + 'static,
{
    let uds_path = uds_socket_path()?;
    let stream = UnixStream::connect(&uds_path)
        .await
        .map_err(|e| DaemonError::Uds(format!("connect: {}", e)))?;

    let (reader, writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(reader);

    let mut first_line = String::new();
    reader
        .read_line(&mut first_line)
        .await
        .map_err(|e| DaemonError::Uds(format!("read status: {}", e)))?;

    let port = match serde_json::from_str::<DaemonMessage>(first_line.trim()) {
        Ok(DaemonMessage::Status { port, .. }) => port,
        _ => 8100,
    };

    let writer = Mutex::new(writer);

    {
        let mut sub_line = serde_json::to_string(&ClientCommand::Subscribe).unwrap_or_default();
        sub_line.push('\n');
        let mut w = writer.lock().await;
        w.write_all(sub_line.as_bytes())
            .await
            .map_err(|e| DaemonError::Uds(format!("send subscribe: {}", e)))?;
        w.flush()
            .await
            .map_err(|e| DaemonError::Uds(format!("flush subscribe: {}", e)))?;
    }

    let event_task = tokio::spawn(async move {
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
                    match serde_json::from_str::<DaemonMessage>(trimmed) {
                        Ok(msg) => {
                            on_message(msg);
                        }
                        Err(e) => {
                            eprintln!("Failed to parse daemon message: {} ({})", trimmed, e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Daemon UDS read error: {}", e);
                    break;
                }
            }
        }
    });

    Ok(DaemonConnection {
        writer,
        event_task: Some(event_task),
        port,
    })
}

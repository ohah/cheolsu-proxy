use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::Mutex;

use crate::daemon::{check_and_cleanup_stale_lock, lock_file_path, uds_socket_path};
use crate::error::DaemonError;
use crate::protocol::{ClientCommand, DaemonMessage, ProxyLockInfo};

/// 공유 커맨드 전송 로직. Arc clone 없이 참조로 호출 가능.
async fn send_command_inner(
    writer: &Mutex<Option<tokio::io::WriteHalf<UnixStream>>>,
    is_connected: &AtomicBool,
    cmd: &ClientCommand,
) -> Result<(), DaemonError> {
    if !is_connected.load(Ordering::Acquire) {
        return Err(DaemonError::Uds(
            "데몬에 연결되어 있지 않습니다".to_string(),
        ));
    }

    let mut line = serde_json::to_string(cmd)?;
    line.push('\n');
    let mut guard = writer.lock().await;
    let w = guard.as_mut().ok_or_else(|| {
        DaemonError::Uds("데몬에 연결되어 있지 않습니다 (writer 없음)".to_string())
    })?;
    w.write_all(line.as_bytes())
        .await
        .map_err(|e| DaemonError::Uds(format!("write: {}", e)))?;
    w.flush()
        .await
        .map_err(|e| DaemonError::Uds(format!("flush: {}", e)))?;
    Ok(())
}

/// DaemonConnection에서 추출 가능한 클론 가능한 커맨드 전송 핸들.
/// 외부 mutex를 잡지 않고도 데몬에 명령을 보낼 수 있습니다.
#[derive(Clone)]
pub struct CommandSender {
    writer: Arc<Mutex<Option<tokio::io::WriteHalf<UnixStream>>>>,
    is_connected: Arc<AtomicBool>,
}

impl CommandSender {
    /// Send a command to the daemon.
    pub async fn send_command(&self, cmd: &ClientCommand) -> Result<(), DaemonError> {
        send_command_inner(&self.writer, &self.is_connected, cmd).await
    }
}

/// A connection to the daemon process.
pub struct DaemonConnection {
    writer: Arc<Mutex<Option<tokio::io::WriteHalf<UnixStream>>>>,
    /// Background task that reads events from the UDS and forwards them.
    event_task: Option<tokio::task::JoinHandle<()>>,
    /// 현재 데몬에 연결되어 있는지 여부
    is_connected: Arc<AtomicBool>,
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
    /// 현재 데몬에 연결되어 있는지 확인합니다.
    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::Acquire)
    }

    /// Disconnect from the daemon (does NOT send stop command).
    pub async fn disconnect(mut self) {
        if let Some(task) = self.event_task.take() {
            task.abort();
        }
    }

    /// 클론 가능한 커맨드 전송 핸들을 반환합니다.
    pub fn command_sender(&self) -> CommandSender {
        CommandSender {
            writer: self.writer.clone(),
            is_connected: self.is_connected.clone(),
        }
    }

    /// Send a command to the daemon.
    pub async fn send_command(&self, cmd: &ClientCommand) -> Result<(), DaemonError> {
        send_command_inner(&self.writer, &self.is_connected, cmd).await
    }

    /// 데몬에 헬스체크 요청을 전송합니다.
    /// 결과는 on_message 콜백으로 DaemonMessage::HealthCheckResult가 전달됩니다.
    pub async fn health_check(&self) -> Result<(), DaemonError> {
        self.send_command(&ClientCommand::HealthCheck).await
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

/// 지수 백오프로 조건을 대기합니다. 조건 충족 시 true, 타임아웃 시 false 반환.
async fn wait_with_backoff<F>(condition: F, timeout_secs: u64) -> bool
where
    F: Fn() -> bool,
{
    let mut delay = std::time::Duration::from_millis(100);
    let max_delay = std::time::Duration::from_secs(2);
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    loop {
        if condition() {
            return true;
        }
        if tokio::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(delay).await;
        delay = (delay * 2).min(max_delay);
    }
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
    F: Fn(DaemonMessage) + Send + Sync + 'static,
{
    if is_daemon_running().is_none() {
        check_and_cleanup_stale_lock();

        spawn_daemon(port, host)?;

        if !wait_with_backoff(|| is_daemon_running().is_some(), 15).await {
            return Err(DaemonError::Daemon(
                "Daemon did not start within 15 seconds".to_string(),
            ));
        }
    }

    // Wait for the UDS socket file to actually exist.
    let uds_path = uds_socket_path()?;
    if !wait_with_backoff(|| uds_path.exists(), 15).await {
        return Err(DaemonError::Daemon(
            "UDS socket was not created within 15 seconds".to_string(),
        ));
    }

    connect_to_daemon(on_message).await
}

/// UDS에 연결하고 Subscribe 명령을 보내는 내부 헬퍼.
/// 성공 시 (reader, writer, port)를 반환합니다.
async fn establish_connection() -> Result<
    (
        BufReader<tokio::io::ReadHalf<UnixStream>>,
        tokio::io::WriteHalf<UnixStream>,
        u16,
    ),
    DaemonError,
> {
    let uds_path = uds_socket_path()?;
    let stream = UnixStream::connect(&uds_path)
        .await
        .map_err(|e| DaemonError::Uds(format!("connect: {}", e)))?;

    let (reader, mut writer) = tokio::io::split(stream);
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

    // Subscribe 명령 전송
    let mut sub_line = serde_json::to_string(&ClientCommand::Subscribe).unwrap_or_default();
    sub_line.push('\n');
    writer
        .write_all(sub_line.as_bytes())
        .await
        .map_err(|e| DaemonError::Uds(format!("send subscribe: {}", e)))?;
    writer
        .flush()
        .await
        .map_err(|e| DaemonError::Uds(format!("flush subscribe: {}", e)))?;

    Ok((reader, writer, port))
}

/// Connect to an already-running daemon.
/// `on_message` is called for each `DaemonMessage` received.
pub async fn connect_to_daemon<F>(on_message: F) -> Result<DaemonConnection, DaemonError>
where
    F: Fn(DaemonMessage) + Send + Sync + 'static,
{
    let (reader, writer, port) = establish_connection().await?;

    let is_connected = Arc::new(AtomicBool::new(true));
    let shared_writer: Arc<Mutex<Option<tokio::io::WriteHalf<UnixStream>>>> =
        Arc::new(Mutex::new(Some(writer)));

    let event_is_connected = is_connected.clone();
    let event_writer = shared_writer.clone();

    let event_task = tokio::spawn(async move {
        run_event_loop(reader, &on_message, &event_is_connected, &event_writer).await;
    });

    Ok(DaemonConnection {
        writer: shared_writer,
        event_task: Some(event_task),
        is_connected,
        port,
    })
}

/// 이벤트 루프: 메시지를 읽고, 연결 끊김 시 지수 백오프로 재연결을 시도합니다.
async fn run_event_loop<F>(
    mut reader: BufReader<tokio::io::ReadHalf<UnixStream>>,
    on_message: &F,
    is_connected: &Arc<AtomicBool>,
    writer: &Arc<Mutex<Option<tokio::io::WriteHalf<UnixStream>>>>,
) where
    F: Fn(DaemonMessage) + Send + Sync + 'static,
{
    let mut line_buf = String::new();

    loop {
        // 메시지 읽기 루프
        let disconnect_reason = loop {
            line_buf.clear();
            match reader.read_line(&mut line_buf).await {
                Ok(0) => break "EOF: 데몬 연결 종료".to_string(),
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
                Err(e) => break format!("UDS 읽기 오류: {}", e),
            }
        };

        // 연결 끊김 감지
        is_connected.store(false, Ordering::Release);
        {
            let mut w = writer.lock().await;
            *w = None;
        }
        on_message(DaemonMessage::Disconnected {
            reason: disconnect_reason.clone(),
        });
        eprintln!("Daemon 연결 끊김: {}", disconnect_reason);

        // 지수 백오프로 재연결 시도 (최대 60회, 약 5분)
        let mut backoff_ms: u64 = 100;
        const MAX_BACKOFF_MS: u64 = 5000;
        const MAX_RETRIES: u32 = 60;
        let mut retries: u32 = 0;

        let reconnected = loop {
            if retries >= MAX_RETRIES {
                eprintln!(
                    "Daemon 재연결 최대 시도 횟수({}) 초과, 이벤트 루프 종료",
                    MAX_RETRIES
                );
                break false;
            }
            retries += 1;

            tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;

            match establish_connection().await {
                Ok((new_reader, new_writer, _port)) => {
                    // 재연결 성공
                    reader = new_reader;
                    {
                        let mut w = writer.lock().await;
                        *w = Some(new_writer);
                    }
                    is_connected.store(true, Ordering::Release);
                    on_message(DaemonMessage::Reconnected);
                    eprintln!("Daemon 재연결 성공");
                    break true;
                }
                Err(e) => {
                    eprintln!(
                        "Daemon 재연결 실패 ({}/{}회, {}ms 후 재시도): {}",
                        retries,
                        MAX_RETRIES,
                        backoff_ms.min(MAX_BACKOFF_MS),
                        e
                    );
                    backoff_ms = (backoff_ms * 2).min(MAX_BACKOFF_MS);
                }
            }
        };

        if !reconnected {
            // 재연결 실패 시 이벤트 루프 종료
            return;
        }
    }
}

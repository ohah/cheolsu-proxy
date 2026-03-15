//! 클라이언트 핸들러 모듈: Unix 소켓을 통한 클라이언트 커맨드 처리

mod commands;
mod watcher;

use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::{broadcast, Mutex};
use tracing::{debug, error, warn};

use crate::breakpoint::BreakpointManager;
use crate::daemon::{DaemonChannels, DaemonMetrics};
use crate::protocol::{ClientCommand, DaemonMessage, ProxyAuthConfig, PROTOCOL_VERSION};
use proxyapi_v2::websocket_registry::WebSocketRegistry;

use commands::CommandState;

/// ClientHandlerContext와 CommandState에서 공유하는 데몬 상태.
/// 두 구조체 모두 이 필드들을 동일하게 사용하므로 중복을 제거한다.
#[derive(Clone)]
pub(crate) struct SharedDaemonState {
    pub(crate) channels: DaemonChannels,
    pub(crate) breakpoint_manager: BreakpointManager,
    pub(crate) event_tx: broadcast::Sender<String>,
    pub(crate) ws_registry: WebSocketRegistry,
    pub(crate) script_handle: scripting::ScriptHandle,
    pub(crate) quick_settings: Arc<tokio::sync::RwLock<crate::handler::QuickSettings>>,
    pub(crate) proxy_auth: Arc<tokio::sync::RwLock<Option<ProxyAuthConfig>>>,
    pub(crate) metrics: DaemonMetrics,
    pub(crate) client_count: Arc<std::sync::atomic::AtomicUsize>,
    pub(crate) tls_passthrough: proxyapi_v2::tls_passthrough::TlsPassthrough,
    pub(crate) connection_strategy: Arc<std::sync::atomic::AtomicU8>,
}

/// handle_client에 전달되는 채널/상태를 묶는 컨텍스트 구조체.
/// DaemonContext와 유사하지만, 클라이언트 핸들러에서 필요한 필드만 포함한다.
pub(crate) struct ClientHandlerContext {
    pub(crate) event_rx: broadcast::Receiver<String>,
    pub(crate) port: u16,
    pub(crate) shared: SharedDaemonState,
}

pub(crate) async fn handle_client(stream: UnixStream, ctx: ClientHandlerContext) {
    let ClientHandlerContext {
        mut event_rx,
        port,
        shared,
    } = ctx;
    let (reader, writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let writer = Arc::new(Mutex::new(writer));

    {
        let status_msg = DaemonMessage::Status {
            running: true,
            port,
            protocol_version: PROTOCOL_VERSION,
        };
        let mut line = serde_json::to_string(&status_msg).unwrap_or_default();
        line.push('\n');
        let mut w = writer.lock().await;
        let _ = w.write_all(line.as_bytes()).await;
        let _ = w.flush().await;

        // 연결 직후 현재 인터셉트 규칙을 전송 (빈 목록도 유효한 상태)
        let current_rules = shared.channels.intercept_tx.borrow().clone();
        let rules_msg = DaemonMessage::InterceptRulesUpdated {
            rules: current_rules,
        };
        let mut rules_line = serde_json::to_string(&rules_msg).unwrap_or_default();
        rules_line.push('\n');
        let _ = w.write_all(rules_line.as_bytes()).await;
        let _ = w.flush().await;

        let current_mappings = shared.channels.host_mapping_tx.borrow().clone();
        let mappings_msg = DaemonMessage::HostMappingsUpdated {
            mappings: current_mappings,
        };
        let mut mappings_line = serde_json::to_string(&mappings_msg).unwrap_or_default();
        mappings_line.push('\n');
        let _ = w.write_all(mappings_line.as_bytes()).await;
        let _ = w.flush().await;

        let current_reverse_proxy = shared.channels.reverse_proxy_tx.borrow().clone();
        let reverse_proxy_msg = DaemonMessage::ReverseProxyRulesUpdated {
            rules: current_reverse_proxy,
        };
        let mut reverse_proxy_line = serde_json::to_string(&reverse_proxy_msg).unwrap_or_default();
        reverse_proxy_line.push('\n');
        let _ = w.write_all(reverse_proxy_line.as_bytes()).await;
        let _ = w.flush().await;

        let (current_ssl_mode, current_ssl_entries) =
            shared.channels.ssl_proxying_tx.borrow().clone();
        let ssl_msg = DaemonMessage::SslProxyingListUpdated {
            mode: current_ssl_mode,
            entries: current_ssl_entries,
        };
        let mut ssl_line = serde_json::to_string(&ssl_msg).unwrap_or_default();
        ssl_line.push('\n');
        let _ = w.write_all(ssl_line.as_bytes()).await;
        let _ = w.flush().await;

        let current_client_cert = shared.channels.client_cert_tx.borrow().clone();
        let cert_msg = DaemonMessage::ClientCertificateUpdated {
            config: current_client_cert,
        };
        let mut cert_line = serde_json::to_string(&cert_msg).unwrap_or_default();
        cert_line.push('\n');
        let _ = w.write_all(cert_line.as_bytes()).await;
        let _ = w.flush().await;
    }

    // 스크립트 로그 전달 태스크
    let writer_logs = writer.clone();
    let mut log_rx = shared.script_handle.subscribe_logs();
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
        // broadcast 채널 lagged 복구 전략:
        // - broadcast 채널은 고정 크기 버퍼를 사용하므로, 소비자가 생산 속도를 따라잡지 못하면
        //   오래된 메시지가 덮어씌워지고 Lagged 에러가 발생한다.
        // - 일시적인 lag는 정상적이며 (예: 클라이언트 처리 지연, 일시적인 이벤트 폭주),
        //   이 경우 자동으로 최신 위치에서 수신을 재개한다.
        // - 연속적으로 lag가 발생하면 소비자가 근본적으로 처리량을 감당하지 못하는 것이므로,
        //   MAX_LAGGED_THRESHOLD를 초과하면 수신자를 재생성하여 최신 위치로 강제 리셋한다.
        const MAX_LAGGED_THRESHOLD: u64 = 1000;
        // 로그 출력 최소 간격 (동일한 경고가 반복되는 것을 방지)
        const LOG_RATE_LIMIT: std::time::Duration = std::time::Duration::from_secs(10);

        let mut lagged_count: u64 = 0;
        let mut total_skipped: u64 = 0;
        let mut last_lag_log_time = std::time::Instant::now() - LOG_RATE_LIMIT;

        loop {
            match event_rx.recv().await {
                Ok(msg) => {
                    // 정상 수신 시 lagged 카운트 리셋
                    lagged_count = 0;
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
                    // Lagged 에러 발생: 버퍼가 가득 차서 n개의 메시지가 스킵됨.
                    // tokio broadcast는 Lagged 에러 후 자동으로 최신 tail 위치로 이동하므로
                    // 다음 recv()는 가장 최근 메시지부터 수신한다.
                    lagged_count += 1;
                    total_skipped += n;

                    // 시간 기반 로그 제한: 짧은 시간에 동일 경고가 반복되는 것을 방지
                    let now = std::time::Instant::now();
                    if now.duration_since(last_lag_log_time) >= LOG_RATE_LIMIT {
                        if lagged_count >= 10 {
                            warn!(
                                "broadcast 채널 Lagged (연속 {}회, 누적 스킵 {}건, 이번 스킵 {}건)",
                                lagged_count, total_skipped, n
                            );
                        } else {
                            debug!(
                                "broadcast 채널 Lagged (연속 {}회, 누적 스킵 {}건, 이번 스킵 {}건)",
                                lagged_count, total_skipped, n
                            );
                        }
                        last_lag_log_time = now;
                    }

                    // 연속 lag 횟수가 임계값을 초과하면 수신자를 재생성하여 복구 시도.
                    // 기존 수신자는 내부 위치가 꼬여 있을 수 있으므로, event_tx에서
                    // 새 수신자를 구독하여 최신 위치로 강제 리셋한다.
                    if lagged_count >= MAX_LAGGED_THRESHOLD {
                        error!(
                            "broadcast 채널 연속 lag {}회 초과 (누적 스킵 {}건) — 수신자 재생성으로 복구 시도",
                            lagged_count, total_skipped
                        );
                        // event_tx는 move되어 접근 불가하므로, 여기서는 기존 수신자를
                        // resubscribe()로 최신 tail 위치에 재구독한다.
                        event_rx = event_rx.resubscribe();
                        lagged_count = 0;
                        total_skipped = 0;
                    }

                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    // CommandState 생성 — handle_command에서 필요한 모든 공유 상태를 하나로 묶는다.
    let cmd_state = CommandState {
        writer: writer.clone(),
        shared,
        subscribed,
        watched_path,
    };

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
                    Ok(cmd) => {
                        let should_stop = commands::handle_command(cmd, &cmd_state).await;
                        if should_stop {
                            break;
                        }
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

    // event_task와 log_task는 broadcast recv/writer만 수행하므로
    // abort()로 안전하게 종료 가능 (상태 변경 없음)
    event_task.abort();
    log_task.abort();
}

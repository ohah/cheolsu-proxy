//! 데몬 프로세스의 진입점 및 핵심 구조체를 정의하는 모듈
//!
//! - `lifecycle` — 락 파일 관리, 파일시스템 초기화, stale lock 정리
//! - `accept_loop` — UDS accept 루프 및 클라이언트 연결 관리
//! - `proxy_task` — 프록시 태스크 스폰 및 시그널 핸들러

pub(crate) mod accept_loop;
pub(crate) mod lifecycle;
mod proxy_task;

#[cfg(test)]
mod tests;

// 공개 API 재수출
pub use lifecycle::{
    app_support_dir, check_and_cleanup_stale_lock, lock_file_path, uds_socket_path,
};

use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use tokio::net::UnixListener;
use tokio::sync::{broadcast, watch};
use tracing::{error, info, warn};
use tracing_subscriber::prelude::*;

use crate::breakpoint::BreakpointManager;
use crate::error::DaemonError;
use crate::handler::QuickSettings;
use crate::metrics_aggregator::MetricsAggregator;
use crate::protocol::DaemonMessage;
use crate::system_proxy::set_proxy;
use proxyapi_v2::metrics::MetricsCollector;
use proxyapi_v2::throttle::ThrottleConfig;
use proxyapi_v2::upstream_proxy::UpstreamProxyConfig;
use proxyapi_v2::websocket_registry::WebSocketRegistry;

use accept_loop::run_accept_loop;
use lifecycle::{init_filesystem, remove_lock_file, remove_uds_socket};
use proxy_task::{spawn_proxy_task, spawn_signal_handlers};

/// 동시 연결 수 기본 제한값
const DEFAULT_MAX_CONCURRENT_CONNECTIONS: usize = 1024;
/// 요청 바디 크기 기본 제한값 (100MB)
const DEFAULT_MAX_BODY_SIZE: usize = 100 * 1024 * 1024;

/// 프록시 설정 전파용 watch 채널 송신자 그룹
#[derive(Clone)]
pub(crate) struct DaemonChannels {
    pub(crate) intercept_tx: watch::Sender<Vec<crate::protocol::InterceptRule>>,
    pub(crate) upstream_tx: watch::Sender<Option<UpstreamProxyConfig>>,
    pub(crate) server_replay_tx: watch::Sender<Vec<crate::protocol::ServerReplayEntry>>,
    pub(crate) throttle_tx: watch::Sender<Option<ThrottleConfig>>,
    pub(crate) breakpoint_tx: watch::Sender<Vec<crate::protocol::BreakpointRule>>,
    pub(crate) host_mapping_tx: watch::Sender<Vec<crate::protocol::HostMapping>>,
    pub(crate) ssl_proxying_tx: watch::Sender<(
        crate::protocol::SslProxyingMode,
        Vec<crate::protocol::SslProxyingEntry>,
    )>,
    pub(crate) client_cert_tx: watch::Sender<Option<crate::protocol::ClientCertConfig>>,
    pub(crate) request_client_cert_tx:
        watch::Sender<Option<crate::protocol::RequestClientCertConfig>>,
}

/// 데몬 메트릭/통계 정보 그룹
#[derive(Clone)]
pub(crate) struct DaemonMetrics {
    /// 데몬 시작 시각 (헬스체크용 uptime 계산)
    pub(crate) started_at: std::time::Instant,
    /// 총 트랜잭션 수 (헬스체크용)
    pub(crate) total_transactions: Arc<AtomicU64>,
    pub(crate) metrics_aggregator: Arc<MetricsAggregator>,
}

/// 데몬에서 사용하는 채널 및 공유 상태를 묶는 컨텍스트
pub(crate) struct DaemonContext {
    pub(crate) event_tx: broadcast::Sender<String>,
    pub(crate) client_count: Arc<AtomicUsize>,
    pub(crate) shutdown_tx: tokio::sync::mpsc::Sender<()>,
    pub(crate) channels: DaemonChannels,
    pub(crate) metrics: DaemonMetrics,
    pub(crate) breakpoint_manager: BreakpointManager,
    pub(crate) ws_registry: WebSocketRegistry,
    pub(crate) script_handle: scripting::ScriptHandle,
    pub(crate) quick_settings: Arc<tokio::sync::RwLock<QuickSettings>>,
    pub(crate) proxy_auth: Arc<tokio::sync::RwLock<Option<crate::protocol::ProxyAuthConfig>>>,
    pub(crate) tls_passthrough: proxyapi_v2::tls_passthrough::TlsPassthrough,
    pub(crate) connection_strategy: Arc<std::sync::atomic::AtomicU8>,
}

// --- 데몬 진입점 ---

/// Runs the daemon process. This function never returns (calls std::process::exit).
pub fn run_daemon(port: u16, host: String) -> ! {
    // tracing 초기화: stderr 출력 + 파일 로깅 (일별 로테이션)
    // try_init을 사용하여 이미 subscriber가 설정된 경우(예: Tauri) 패닉 방지
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(true);

    // 로그 디렉토리 생성 및 파일 로깅 레이어 설정
    // _guard를 유지하여 non_blocking writer가 프로세스 종료 시까지 플러시되도록 함
    let (file_layer, _guard) = app_support_dir()
        .map(|dir| dir.join("logs"))
        .and_then(|log_dir| {
            std::fs::create_dir_all(&log_dir).map_err(|e| {
                eprintln!(
                    "로그 디렉토리 생성 실패 ({}): {} — stderr만 사용합니다",
                    log_dir.display(),
                    e
                );
                DaemonError::DataDirNotFound
            })?;
            // 일별 로테이션, 최대 7일 보관, non_blocking으로 I/O blocking 방지
            let file_appender = tracing_appender::rolling::RollingFileAppender::builder()
                .rotation(tracing_appender::rolling::Rotation::DAILY)
                .filename_prefix("cheolsu-proxy-daemon")
                .filename_suffix("log")
                .max_log_files(7)
                .build(&log_dir)
                .map_err(|e| {
                    eprintln!("로그 파일 생성 실패: {} — stderr만 사용합니다", e);
                    DaemonError::DataDirNotFound
                })?;
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            Ok((
                Some(
                    tracing_subscriber::fmt::layer()
                        .with_writer(non_blocking)
                        .with_ansi(false),
                ),
                Some(guard),
            ))
        })
        .unwrap_or((None, None));

    let _ = tracing_subscriber::registry()
        .with(env_filter)
        .with(stderr_layer)
        .with(file_layer)
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

async fn daemon_main(port: u16, host: String) -> i32 {
    let (uds_path, uds_path_str) = match init_filesystem(port) {
        Ok(result) => result,
        Err(code) => return code,
    };

    let (event_tx, _) = broadcast::channel::<String>(1024);
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
    let (host_mapping_tx, host_mapping_rx) =
        watch::channel::<Vec<crate::protocol::HostMapping>>(Vec::new());
    let (ssl_proxying_tx, ssl_proxying_rx) =
        watch::channel::<(
            crate::protocol::SslProxyingMode,
            Vec<crate::protocol::SslProxyingEntry>,
        )>((crate::protocol::SslProxyingMode::default(), Vec::new()));
    let (client_cert_tx, client_cert_rx) =
        watch::channel::<Option<crate::protocol::ClientCertConfig>>(None);
    let (request_client_cert_tx, request_client_cert_rx) =
        watch::channel::<Option<crate::protocol::RequestClientCertConfig>>(None);

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
    let quick_settings = Arc::new(tokio::sync::RwLock::new(QuickSettings::default()));
    let proxy_auth = Arc::new(tokio::sync::RwLock::new(
        None::<crate::protocol::ProxyAuthConfig>,
    ));

    let max_concurrent_connections: Option<usize> = Some(DEFAULT_MAX_CONCURRENT_CONNECTIONS);
    let max_body_size: Option<usize> = Some(DEFAULT_MAX_BODY_SIZE);

    // 연결 전략: 런타임 변경을 위한 공유 AtomicU8 (0=Lazy 기본값)
    let connection_strategy = Arc::new(std::sync::atomic::AtomicU8::new(0));

    // 메트릭 수집기 초기화
    let (metric_event_tx, metric_event_rx) = proxyapi_v2::metrics::metric_event_channel(1024);
    let metrics_collector = Arc::new(MetricsCollector::new(metric_event_tx));
    let metrics_aggregator = Arc::new(MetricsAggregator::new(metrics_collector.clone()));
    let _metrics_handle = metrics_aggregator.spawn_aggregation_loop(metric_event_rx);

    // TLS 자동 학습 바이패스 초기화 (변경 알림 채널 포함)
    let (tls_change_tx, mut tls_change_rx) = tokio::sync::mpsc::channel::<Vec<(String, u32)>>(64);
    let passthrough_path = app_support_dir()
        .ok()
        .map(|dir| dir.join("tls_passthrough.json"));
    let tls_passthrough = proxyapi_v2::tls_passthrough::TlsPassthrough::new(passthrough_path)
        .with_change_notifier(tls_change_tx);

    // TLS passthrough 변경 시 이벤트 브로드캐스트
    {
        let event_tx_tls = event_tx.clone();
        tokio::spawn(async move {
            while let Some(entries) = tls_change_rx.recv().await {
                let tls_entries: Vec<crate::protocol::TlsPassthroughEntry> = entries
                    .into_iter()
                    .map(
                        |(host, failure_count)| crate::protocol::TlsPassthroughEntry {
                            host,
                            failure_count,
                        },
                    )
                    .collect();
                let msg = DaemonMessage::TlsPassthroughUpdated {
                    entries: tls_entries,
                };
                if let Ok(json) = serde_json::to_string(&msg) {
                    let _ = event_tx_tls.send(json);
                }
            }
        });
    }

    let (proxy_handle, proxy_shutdown_tx) = spawn_proxy_task(
        addr,
        event_tx.clone(),
        intercept_rx,
        upstream_rx,
        server_replay_rx,
        throttle_rx,
        breakpoint_rx,
        breakpoint_manager.clone(),
        host_mapping_rx,
        ssl_proxying_rx,
        client_cert_rx,
        ws_registry.clone(),
        script_handle.clone(),
        quick_settings.clone(),
        proxy_auth.clone(),
        max_concurrent_connections,
        max_body_size,
        tls_passthrough.clone(),
        request_client_cert_rx,
        connection_strategy.clone(),
        metrics_collector.clone(),
    );

    let uds_listener = match UnixListener::bind(&uds_path) {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind UDS: {}", e);
            cleanup(port, &uds_path);
            return 1;
        }
    };

    let log_dir = app_support_dir()
        .map(|d| d.join("logs"))
        .unwrap_or_default();
    info!(
        "Daemon started (PID {}, proxy={}:{}, uds={}, log_dir={})",
        std::process::id(),
        host,
        port,
        uds_path_str,
        log_dir.display()
    );

    let signal_handles = spawn_signal_handlers(shutdown_tx.clone());

    let started_at = std::time::Instant::now();
    let total_transactions = Arc::new(AtomicU64::new(0));

    // 트랜잭션 카운터: broadcast 채널을 구독하여 Event 메시지를 카운트
    {
        let mut counter_rx = event_tx.subscribe();
        let total_tx_clone = total_transactions.clone();
        tokio::spawn(async move {
            loop {
                match counter_rx.recv().await {
                    Ok(msg) => {
                        // WARNING: JSON 문자열 패턴 매칭으로 트랜잭션을 카운팅하고 있음.
                        // 이 방식은 fragile하며, JSON 직렬화 포맷이 변경되거나(예: 공백 추가,
                        // 키 순서 변경) 다른 메시지에 동일 패턴이 포함될 경우 오탐/누락이 발생할 수 있음.
                        // TODO: 구조화된 enum 매칭이나 별도 카운팅 채널로 개선 필요.
                        if msg.contains(r#""type":"event""#) {
                            total_tx_clone.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }

    let ctx = DaemonContext {
        event_tx,
        client_count: Arc::new(AtomicUsize::new(0)),
        shutdown_tx,
        channels: DaemonChannels {
            intercept_tx,
            upstream_tx,
            server_replay_tx,
            throttle_tx,
            breakpoint_tx,
            host_mapping_tx,
            ssl_proxying_tx,
            client_cert_tx,
            request_client_cert_tx,
        },
        metrics: DaemonMetrics {
            started_at,
            total_transactions,
            metrics_aggregator,
        },
        breakpoint_manager,
        ws_registry,
        script_handle,
        quick_settings,
        proxy_auth,
        tls_passthrough,
        connection_strategy,
    };

    run_accept_loop(uds_listener, &mut shutdown_rx, &ctx, port).await;

    // 프록시 태스크에 graceful shutdown 신호 전송
    info!("Sending graceful shutdown signal to proxy task...");
    let _ = proxy_shutdown_tx.send(());

    // 최대 5초 대기 후 강제 종료
    tokio::select! {
        result = proxy_handle => {
            match result {
                Ok(()) => info!("Proxy task shut down gracefully"),
                Err(e) => warn!("Proxy task panicked during shutdown: {}", e),
            }
        }
        _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
            warn!("Proxy task did not shut down within 5 seconds");
            // select! 분기에서 다른 future(proxy_handle)는 자동으로 drop되어 abort됨
        }
    }

    // 시그널 핸들러 태스크 정리
    for handle in signal_handles {
        handle.abort();
    }

    cleanup(port, &uds_path);
    info!("Daemon stopped");
    0
}

/// 데몬 종료 시 시스템 프록시 해제, 락 파일 및 UDS 소켓 삭제
fn cleanup(port: u16, uds_path: &Path) {
    if let Err(e) = set_proxy(false, port) {
        error!("Failed to unset system proxy: {}", e);
    }
    remove_lock_file();
    remove_uds_socket(uds_path);
}

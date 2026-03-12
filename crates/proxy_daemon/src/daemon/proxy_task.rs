//! 프록시 태스크 스폰 및 시그널 핸들러 등록

use std::sync::Arc;

use tokio::sync::watch;
use tracing::{error, warn};

use crate::breakpoint::BreakpointManager;
use crate::handler::QuickSettings;
use crate::proxy_runner::run_proxy;
use proxyapi_v2::metrics::MetricsCollector;
use proxyapi_v2::throttle::ThrottleConfig;
use proxyapi_v2::upstream_proxy::UpstreamProxyConfig;
use proxyapi_v2::websocket_registry::WebSocketRegistry;

/// 프록시 태스크를 스폰합니다.
/// 반환값: (JoinHandle, 종료 신호 송신자)
pub(super) fn spawn_proxy_task(
    addr: std::net::SocketAddr,
    event_tx: tokio::sync::broadcast::Sender<String>,
    intercept_rx: watch::Receiver<Vec<crate::protocol::InterceptRule>>,
    upstream_rx: watch::Receiver<Option<UpstreamProxyConfig>>,
    server_replay_rx: watch::Receiver<Vec<crate::protocol::ServerReplayEntry>>,
    throttle_rx: watch::Receiver<Option<ThrottleConfig>>,
    breakpoint_rx: watch::Receiver<Vec<crate::protocol::BreakpointRule>>,
    breakpoint_manager: BreakpointManager,
    host_mapping_rx: watch::Receiver<Vec<crate::protocol::HostMapping>>,
    ssl_proxying_rx: watch::Receiver<(
        crate::protocol::SslProxyingMode,
        Vec<crate::protocol::SslProxyingEntry>,
    )>,
    client_cert_rx: watch::Receiver<Option<crate::protocol::ClientCertConfig>>,
    ws_registry: WebSocketRegistry,
    script_handle: scripting::ScriptHandle,
    quick_settings: Arc<tokio::sync::RwLock<QuickSettings>>,
    proxy_auth: Arc<tokio::sync::RwLock<Option<crate::protocol::ProxyAuthConfig>>>,
    max_concurrent_connections: Option<usize>,
    max_body_size: Option<usize>,
    tls_passthrough: proxyapi_v2::tls_passthrough::TlsPassthrough,
    request_client_cert_rx: watch::Receiver<Option<crate::protocol::RequestClientCertConfig>>,
    connection_strategy: Arc<std::sync::atomic::AtomicU8>,
    metrics_collector: Arc<MetricsCollector>,
) -> (
    tokio::task::JoinHandle<()>,
    tokio::sync::oneshot::Sender<()>,
) {
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let handle = tokio::spawn(async move {
        if let Err(code) = run_proxy(
            addr,
            event_tx,
            intercept_rx,
            upstream_rx,
            server_replay_rx,
            throttle_rx,
            breakpoint_rx,
            breakpoint_manager,
            host_mapping_rx,
            ssl_proxying_rx,
            client_cert_rx,
            ws_registry,
            script_handle,
            quick_settings,
            proxy_auth,
            shutdown_rx,
            max_concurrent_connections,
            max_body_size,
            tls_passthrough,
            request_client_cert_rx,
            connection_strategy,
            metrics_collector,
        )
        .await
        {
            error!("Proxy error: {}", code);
        }
    });
    (handle, shutdown_tx)
}

/// Ctrl+C 및 SIGTERM 시그널 핸들러를 등록합니다.
/// 반환된 JoinHandle을 보관하여 패닉 시 감지할 수 있도록 합니다.
pub(super) fn spawn_signal_handlers(
    shutdown_tx: tokio::sync::mpsc::Sender<()>,
) -> Vec<tokio::task::JoinHandle<()>> {
    let mut handles = Vec::with_capacity(2);

    let shutdown_tx_ctrlc = shutdown_tx.clone();
    handles.push(tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        warn!("Ctrl+C received, shutting down daemon...");
        let _ = shutdown_tx_ctrlc.send(()).await;
    }));

    let shutdown_tx_term = shutdown_tx;
    handles.push(tokio::spawn(async move {
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
    }));

    handles
}

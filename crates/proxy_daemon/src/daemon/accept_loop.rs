//! UDS accept 루프 및 클라이언트 연결 관리

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use tokio::net::UnixListener;
use tracing::{error, info, warn};

use crate::client_handler::{handle_client, ClientHandlerContext, SharedDaemonState};

use super::DaemonContext;

/// 클라이언트 연결 카운트를 자동으로 감소시키는 Drop guard.
/// 태스크가 정상 종료, 패닉, abort 등 어떤 방식으로 끝나더라도 카운트가 감소됩니다.
pub(super) struct ClientCountGuard {
    pub(super) client_count: Arc<AtomicUsize>,
    pub(super) shutdown_tx: tokio::sync::mpsc::Sender<()>,
}

impl Drop for ClientCountGuard {
    fn drop(&mut self) {
        let prev = self.client_count.fetch_sub(1, Ordering::AcqRel);
        if prev == 0 {
            // 언더플로우 방지: 이미 0이면 복구
            self.client_count.store(0, Ordering::Release);
            warn!("ClientCountGuard: 언더플로우 감지, 카운트를 0으로 복구");
            return;
        }
        let remaining = prev - 1;
        info!("Client disconnected. Remaining clients: {}", remaining);
        if remaining == 0 {
            info!("No clients remaining, shutting down daemon...");
            let _ = self.shutdown_tx.try_send(());
        }
    }
}

/// UDS accept 루프를 실행합니다. shutdown 시그널 수신 시 종료.
pub(super) async fn run_accept_loop(
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
                        let count = ctx.client_count.fetch_add(1, Ordering::AcqRel) + 1;
                        info!("Client connected (total: {})", count);

                        let _guard = ClientCountGuard {
                            client_count: ctx.client_count.clone(),
                            shutdown_tx: ctx.shutdown_tx.clone(),
                        };

                        let handler_ctx = ClientHandlerContext {
                            event_rx: ctx.event_tx.subscribe(),
                            port,
                            shared: SharedDaemonState {
                                channels: ctx.channels.clone(),
                                breakpoint_manager: ctx.breakpoint_manager.clone(),
                                event_tx: ctx.event_tx.clone(),
                                ws_registry: ctx.ws_registry.clone(),
                                script_handle: ctx.script_handle.clone(),
                                quick_settings: ctx.quick_settings.clone(),
                                proxy_auth: ctx.proxy_auth.clone(),
                                metrics: ctx.metrics.clone(),
                                client_count: ctx.client_count.clone(),
                                tls_passthrough: ctx.tls_passthrough.clone(),
                                connection_strategy: ctx.connection_strategy.clone(),
                            },
                        };

                        tokio::spawn(async move {
                            // guard가 이 태스크 스코프에 소유되므로, 태스크가 어떤 방식으로
                            // 종료되든 (정상, 패닉, abort) Drop이 호출되어 카운트가 감소됩니다.
                            let _guard = _guard;
                            handle_client(stream, handler_ctx).await;
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

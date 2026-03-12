use super::config::Socks5Config;
use super::handler::handle_socks5_client;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_graceful::Shutdown;
use tracing::{debug, error, info};

/// SOCKS5 프록시 서버
pub struct Socks5Server {
    listener: TcpListener,
    config: Arc<Socks5Config>,
}

impl Socks5Server {
    /// 지정된 주소에 바인드하여 SOCKS5 서버를 생성합니다.
    pub async fn bind(addr: SocketAddr, config: Socks5Config) -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind(addr).await?;
        Ok(Self {
            listener,
            config: Arc::new(config),
        })
    }

    /// 이미 바인드된 리스너를 사용하여 SOCKS5 서버를 생성합니다.
    pub fn from_listener(listener: TcpListener, config: Socks5Config) -> Self {
        Self {
            listener,
            config: Arc::new(config),
        }
    }

    /// 서버가 바인드된 로컬 주소를 반환합니다.
    pub fn local_addr(&self) -> Result<SocketAddr, std::io::Error> {
        self.listener.local_addr()
    }

    /// SOCKS5 서버를 시작합니다.
    pub async fn start<F>(self, graceful_shutdown: F) -> Result<(), std::io::Error>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let shutdown = Shutdown::new(graceful_shutdown);
        let guard = shutdown.guard_weak();

        info!(
            addr = %self.listener.local_addr()?,
            "SOCKS5 프록시 서버 시작"
        );

        loop {
            tokio::select! {
                res = self.listener.accept() => {
                    let (stream, client_addr) = match res {
                        Ok(v) => v,
                        Err(e) => {
                            error!("SOCKS5 연결 수락 실패: {}", e);
                            continue;
                        }
                    };

                    let config = Arc::clone(&self.config);
                    shutdown.spawn_task_fn(move |_guard| async move {
                        if let Err(e) = handle_socks5_client(stream, client_addr, &config).await {
                            debug!(
                                client = %client_addr,
                                error = %e,
                                "SOCKS5 클라이언트 처리 실패"
                            );
                        }
                    });
                }
                _ = guard.cancelled() => {
                    break;
                }
            }
        }

        shutdown.shutdown().await;
        info!("SOCKS5 프록시 서버 종료");
        Ok(())
    }
}

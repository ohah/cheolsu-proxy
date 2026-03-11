use crate::throttle::ThrottleConfig;
use crate::tls_config::SharedTlsConfig;
use crate::tls_event::TlsEventSender;
use crate::tls_passthrough::TlsPassthrough;
use crate::upstream_proxy::UpstreamProxyConfig;
use crate::websocket_registry::WebSocketRegistry;
use proxy_v2_models::RequestInfo;
use std::sync::Arc;
use tokio::sync::{Semaphore, mpsc, watch};
use tokio_tungstenite::Connector;

/// 서버 연결 전략
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectionStrategy {
    /// 현재 동작: 필요 시 서버 연결 (순차적 스니핑)
    #[default]
    Lazy,
    /// ClientHello 직후 서버 연결을 백그라운드에서 시작
    Eager,
    /// Eager 시도 → 실패 시 Lazy 폴백
    EagerWithFallback,
}

/// 프록시의 공유 상태를 담는 컨텍스트 구조체.
/// 선택적 기능(TLS passthrough, WebSocket 레지스트리 등)을 하나의 구조체로 통합하여
/// InternalProxy, Proxy, Builder 전반에서 필드 전파를 단순화합니다.
#[derive(Clone, Default)]
pub struct ProxyContext {
    pub websocket_connector: Option<Connector>,
    pub tunnel_event_sender: Option<mpsc::Sender<RequestInfo>>,
    pub tls_passthrough: Option<TlsPassthrough>,
    pub websocket_registry: Option<WebSocketRegistry>,
    pub upstream_proxy: Option<UpstreamProxyConfig>,
    pub throttle_rx: Option<Arc<watch::Receiver<Option<ThrottleConfig>>>>,
    /// 동시 연결 수를 제한하는 세마포어 (None이면 제한 없음)
    pub connection_semaphore: Option<Arc<Semaphore>>,
    /// TLS 이벤트 채널 (None이면 이벤트 미발송)
    pub tls_event_sender: Option<TlsEventSender>,
    /// 도메인별 TLS 버전/암호화 스위트 세분화 설정 (None이면 기존 하드코딩 동작)
    pub tls_config: Option<SharedTlsConfig>,
    /// 서버 연결 전략 (Lazy: 순차적, Eager: 백그라운드 선행 연결)
    pub connection_strategy: ConnectionStrategy,
}

impl ProxyContext {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_strategy_default_is_lazy() {
        assert_eq!(ConnectionStrategy::default(), ConnectionStrategy::Lazy);
    }

    #[test]
    fn test_connection_strategy_equality() {
        assert_eq!(ConnectionStrategy::Eager, ConnectionStrategy::Eager);
        assert_ne!(ConnectionStrategy::Eager, ConnectionStrategy::Lazy);
        assert_ne!(
            ConnectionStrategy::EagerWithFallback,
            ConnectionStrategy::Eager
        );
    }

    #[test]
    fn test_connection_strategy_clone_copy() {
        let strategy = ConnectionStrategy::Eager;
        let cloned = strategy;
        assert_eq!(cloned, ConnectionStrategy::Eager);
    }

    #[test]
    fn test_proxy_context_default_strategy_is_lazy() {
        let ctx = ProxyContext::new();
        assert_eq!(ctx.connection_strategy, ConnectionStrategy::Lazy);
    }
}

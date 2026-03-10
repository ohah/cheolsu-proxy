use crate::throttle::ThrottleConfig;
use crate::tls_passthrough::TlsPassthrough;
use crate::upstream_proxy::UpstreamProxyConfig;
use crate::websocket_registry::WebSocketRegistry;
use proxy_v2_models::RequestInfo;
use std::sync::Arc;
use tokio::sync::{Semaphore, mpsc, watch};
use tokio_tungstenite::Connector;

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
}

impl ProxyContext {
    pub fn new() -> Self {
        Self::default()
    }
}

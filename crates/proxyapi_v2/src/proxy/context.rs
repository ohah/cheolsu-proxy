use crate::hybrid_tls_handler::TlsStrategyCache;
use crate::metrics::MetricsCollector;
use crate::throttle::ThrottleConfig;
use crate::tls_config::SharedTlsConfig;
use crate::tls_event::TlsEventSender;
use crate::tls_passthrough::TlsPassthrough;
use crate::upstream_proxy::UpstreamProxyConfig;
use crate::websocket_registry::WebSocketRegistry;
use proxy_v2_models::RequestInfo;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::AtomicU8;
use tokio::sync::{Semaphore, mpsc, watch};
use tokio_tungstenite::Connector;

/// 서버 연결 전략
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ConnectionStrategy {
    /// 현재 동작: 필요 시 서버 연결 (순차적 스니핑)
    #[default]
    #[serde(rename = "lazy")]
    Lazy,
    /// ClientHello 직후 서버 연결을 백그라운드에서 시작
    #[serde(rename = "eager")]
    Eager,
    /// Eager 시도 → 실패 시 Lazy 폴백
    #[serde(rename = "eager_with_fallback")]
    EagerWithFallback,
}

impl ConnectionStrategy {
    /// u8 값에서 ConnectionStrategy로 변환
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => ConnectionStrategy::Eager,
            2 => ConnectionStrategy::EagerWithFallback,
            _ => ConnectionStrategy::Lazy,
        }
    }

    /// ConnectionStrategy를 u8 값으로 변환
    pub fn as_u8(&self) -> u8 {
        match self {
            ConnectionStrategy::Lazy => 0,
            ConnectionStrategy::Eager => 1,
            ConnectionStrategy::EagerWithFallback => 2,
        }
    }
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
    /// Arc<AtomicU8>로 런타임 변경 지원
    pub connection_strategy: Option<Arc<AtomicU8>>,
    /// 메트릭 수집기 (None이면 메트릭 미수집)
    pub metrics: Option<Arc<MetricsCollector>>,
    /// Graceful shutdown 신호 수신기 (detached 태스크 종료용)
    pub shutdown_rx: Option<watch::Receiver<bool>>,
    /// 도메인별 학습된 TLS 전략 캐시 (폴백 학습용)
    pub tls_strategy_cache: Option<TlsStrategyCache>,
}

impl ProxyContext {
    pub fn new() -> Self {
        Self::default()
    }

    /// 현재 연결 전략을 반환합니다. None이면 기본값(Lazy)을 반환합니다.
    pub fn connection_strategy(&self) -> ConnectionStrategy {
        self.connection_strategy
            .as_ref()
            .map(|s| ConnectionStrategy::from_u8(s.load(std::sync::atomic::Ordering::Acquire)))
            .unwrap_or_default()
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
        assert_eq!(ctx.connection_strategy(), ConnectionStrategy::Lazy);
    }

    #[test]
    fn test_connection_strategy_from_u8() {
        assert_eq!(ConnectionStrategy::from_u8(0), ConnectionStrategy::Lazy);
        assert_eq!(ConnectionStrategy::from_u8(1), ConnectionStrategy::Eager);
        assert_eq!(
            ConnectionStrategy::from_u8(2),
            ConnectionStrategy::EagerWithFallback
        );
        assert_eq!(ConnectionStrategy::from_u8(255), ConnectionStrategy::Lazy);
    }

    #[test]
    fn test_connection_strategy_as_u8() {
        assert_eq!(ConnectionStrategy::Lazy.as_u8(), 0);
        assert_eq!(ConnectionStrategy::Eager.as_u8(), 1);
        assert_eq!(ConnectionStrategy::EagerWithFallback.as_u8(), 2);
    }

    #[test]
    fn test_connection_strategy_runtime_update() {
        use std::sync::atomic::AtomicU8;
        let strategy = Arc::new(AtomicU8::new(0));
        let ctx = ProxyContext {
            connection_strategy: Some(strategy.clone()),
            ..ProxyContext::new()
        };
        assert_eq!(ctx.connection_strategy(), ConnectionStrategy::Lazy);
        strategy.store(1, std::sync::atomic::Ordering::Release);
        assert_eq!(ctx.connection_strategy(), ConnectionStrategy::Eager);
        strategy.store(2, std::sync::atomic::Ordering::Release);
        assert_eq!(
            ctx.connection_strategy(),
            ConnectionStrategy::EagerWithFallback
        );
    }

    #[test]
    fn test_connection_strategy_serde() {
        let json = serde_json::to_string(&ConnectionStrategy::Lazy).unwrap();
        assert_eq!(json, r#""lazy""#);
        let json = serde_json::to_string(&ConnectionStrategy::Eager).unwrap();
        assert_eq!(json, r#""eager""#);
        let json = serde_json::to_string(&ConnectionStrategy::EagerWithFallback).unwrap();
        assert_eq!(json, r#""eager_with_fallback""#);

        let strategy: ConnectionStrategy = serde_json::from_str(r#""lazy""#).unwrap();
        assert_eq!(strategy, ConnectionStrategy::Lazy);
        let strategy: ConnectionStrategy = serde_json::from_str(r#""eager""#).unwrap();
        assert_eq!(strategy, ConnectionStrategy::Eager);
        let strategy: ConnectionStrategy =
            serde_json::from_str(r#""eager_with_fallback""#).unwrap();
        assert_eq!(strategy, ConnectionStrategy::EagerWithFallback);
    }
}

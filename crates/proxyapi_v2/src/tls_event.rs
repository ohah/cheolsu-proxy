//! TLS 이벤트 시스템 — TLS 수명주기의 각 단계에서 이벤트를 발행합니다.
//!
//! channel 기반으로 구현하여 기존 `tunnel_event_sender` 패턴과 일관성을 유지하며,
//! `try_send`를 사용하여 non-blocking을 보장합니다.

use crate::hybrid_tls_handler::{TlsConnectionInfo, TlsStrategy};
use crate::upstream_cert::UpstreamCertInfo;
use http::uri::Authority;
use std::time::Duration;

/// TLS 수명주기 이벤트
#[derive(Debug, Clone)]
pub enum TlsEvent {
    /// ClientHello 분석 완료
    ClientHelloAnalyzed {
        authority: Authority,
        tls_info: TlsConnectionInfo,
    },
    /// TLS 전략 결정 완료 (rustls vs OpenSSL)
    StrategySelected {
        authority: Authority,
        strategy: TlsStrategy,
        tls_info: TlsConnectionInfo,
    },
    /// 서버 TLS 연결 시작 전 (upstream cert sniffing)
    ServerConnectionStarting { authority: Authority },
    /// 상류 인증서 스니핑 완료
    UpstreamCertSniffed {
        authority: Authority,
        cert_info: Option<UpstreamCertInfo>,
    },
    /// 위조 인증서 생성 시작
    FakeCertGenerating {
        authority: Authority,
        has_upstream_cert: bool,
    },
    /// 클라이언트 TLS 핸드셰이크 성공
    HandshakeCompleted {
        authority: Authority,
        strategy: TlsStrategy,
        duration: Duration,
    },
    /// TLS 핸드셰이크 실패
    HandshakeFailed {
        authority: Authority,
        strategy: TlsStrategy,
        error: String,
        duration: Duration,
    },
}

/// TLS 이벤트 송신자 타입 alias
pub type TlsEventSender = tokio::sync::mpsc::Sender<TlsEvent>;

/// TLS 이벤트 수신자 타입 alias
#[allow(dead_code)]
pub type TlsEventReceiver = tokio::sync::mpsc::Receiver<TlsEvent>;

/// TLS 이벤트 채널을 생성합니다.
#[allow(dead_code)]
pub fn tls_event_channel(buffer: usize) -> (TlsEventSender, TlsEventReceiver) {
    tokio::sync::mpsc::channel(buffer)
}

/// non-blocking으로 TLS 이벤트를 발행합니다.
/// 채널이 None이거나 가득 찬 경우 조용히 무시합니다.
#[inline]
pub fn emit_tls_event(sender: &Option<TlsEventSender>, event: TlsEvent) {
    if let Some(s) = sender {
        let _ = s.try_send(event);
    }
}

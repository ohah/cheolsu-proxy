//! 프록시 메트릭 수집 시스템 — 연결 상태 및 트래픽 통계를 수집합니다.
//!
//! channel 기반으로 구현하여 기존 `tls_event` 패턴과 일관성을 유지하며,
//! `try_send`를 사용하여 non-blocking을 보장합니다.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use tokio::sync::mpsc;

/// 프록시 메트릭 수집기
///
/// Atomic 카운터로 실시간 글로벌 통계를 관리하고,
/// mpsc 채널로 상세 이벤트를 비동기 전송합니다.
#[derive(Debug)]
pub struct MetricsCollector {
    /// 현재 활성 연결 수 (증가/감소 가능하므로 i64)
    pub active_connections: Arc<AtomicI64>,
    /// 총 처리된 요청 수
    pub total_requests: Arc<AtomicU64>,
    /// 총 전송 바이트 수
    pub total_bytes_sent: Arc<AtomicU64>,
    /// 총 수신 바이트 수
    pub total_bytes_received: Arc<AtomicU64>,
    /// 총 TLS 핸드셰이크 수
    pub total_tls_handshakes: Arc<AtomicU64>,
    /// TLS 핸드셰이크 실패 수
    pub total_tls_failures: Arc<AtomicU64>,
    /// 연결 실패 수
    pub total_connection_failures: Arc<AtomicU64>,
    /// 타임아웃 수
    pub total_timeouts: Arc<AtomicU64>,
    /// 상세 메트릭 이벤트 송신 채널
    pub event_tx: mpsc::Sender<MetricEvent>,
}

/// 메트릭 이벤트 — 도메인별 상세 통계 수집용
#[derive(Debug, Clone)]
pub enum MetricEvent {
    /// HTTP 요청 완료
    RequestCompleted {
        domain: String,
        status: u16,
        duration_ms: u64,
        bytes_sent: u64,
        bytes_received: u64,
    },
    /// TLS 핸드셰이크 완료/실패
    TlsHandshake {
        domain: String,
        duration_ms: u64,
        success: bool,
        error: Option<String>,
    },
    /// 연결 실패
    ConnectionFailed { domain: String, error: String },
    /// 연결 열림
    ConnectionOpened,
    /// 연결 닫힘
    ConnectionClosed,
}

/// 메트릭 이벤트 송신자 타입 alias
pub type MetricEventSender = mpsc::Sender<MetricEvent>;

/// 메트릭 이벤트 수신자 타입 alias
pub type MetricEventReceiver = mpsc::Receiver<MetricEvent>;

/// 메트릭 이벤트 채널을 생성합니다.
pub fn metric_event_channel(buffer: usize) -> (MetricEventSender, MetricEventReceiver) {
    mpsc::channel(buffer)
}

impl MetricsCollector {
    /// 새로운 MetricsCollector를 생성합니다.
    pub fn new(event_tx: mpsc::Sender<MetricEvent>) -> Self {
        Self {
            active_connections: Arc::new(AtomicI64::new(0)),
            total_requests: Arc::new(AtomicU64::new(0)),
            total_bytes_sent: Arc::new(AtomicU64::new(0)),
            total_bytes_received: Arc::new(AtomicU64::new(0)),
            total_tls_handshakes: Arc::new(AtomicU64::new(0)),
            total_tls_failures: Arc::new(AtomicU64::new(0)),
            total_connection_failures: Arc::new(AtomicU64::new(0)),
            total_timeouts: Arc::new(AtomicU64::new(0)),
            event_tx,
        }
    }

    /// 활성 연결 수를 1 증가시킵니다.
    pub fn connection_opened(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
        let _ = self.event_tx.try_send(MetricEvent::ConnectionOpened);
    }

    /// 활성 연결 수를 1 감소시킵니다.
    pub fn connection_closed(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
        let _ = self.event_tx.try_send(MetricEvent::ConnectionClosed);
    }

    /// 요청 완료를 기록합니다.
    pub fn request_completed(
        &self,
        domain: String,
        status: u16,
        duration_ms: u64,
        bytes_sent: u64,
        bytes_received: u64,
    ) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.total_bytes_sent
            .fetch_add(bytes_sent, Ordering::Relaxed);
        self.total_bytes_received
            .fetch_add(bytes_received, Ordering::Relaxed);
        let _ = self.event_tx.try_send(MetricEvent::RequestCompleted {
            domain,
            status,
            duration_ms,
            bytes_sent,
            bytes_received,
        });
    }

    /// 연결 실패를 기록합니다.
    pub fn connection_failed(&self, domain: String, error: String) {
        self.total_connection_failures
            .fetch_add(1, Ordering::Relaxed);
        let _ = self
            .event_tx
            .try_send(MetricEvent::ConnectionFailed { domain, error });
    }

    /// 현재 스냅샷을 반환합니다.
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            active_connections: self.active_connections.load(Ordering::Relaxed),
            total_requests: self.total_requests.load(Ordering::Relaxed),
            total_bytes_sent: self.total_bytes_sent.load(Ordering::Relaxed),
            total_bytes_received: self.total_bytes_received.load(Ordering::Relaxed),
            total_tls_handshakes: self.total_tls_handshakes.load(Ordering::Relaxed),
            total_tls_failures: self.total_tls_failures.load(Ordering::Relaxed),
            total_connection_failures: self.total_connection_failures.load(Ordering::Relaxed),
            total_timeouts: self.total_timeouts.load(Ordering::Relaxed),
        }
    }
}

/// 메트릭 스냅샷 — 특정 시점의 메트릭 값을 담는 구조체
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MetricsSnapshot {
    pub active_connections: i64,
    pub total_requests: u64,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub total_tls_handshakes: u64,
    pub total_tls_failures: u64,
    pub total_connection_failures: u64,
    pub total_timeouts: u64,
}

/// non-blocking으로 메트릭 이벤트를 발행합니다.
/// collector가 None이거나 채널이 가득 찬 경우 조용히 무시합니다.
#[inline]
pub fn emit_metric(collector: &Option<Arc<MetricsCollector>>, event: MetricEvent) {
    if let Some(c) = collector {
        let _ = c.event_tx.try_send(event);
    }
}

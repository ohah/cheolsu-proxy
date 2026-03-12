//! 메트릭 집계기 — MetricsCollector의 이벤트를 수신하여 도메인별 통계를 집계합니다.

use proxyapi_v2::metrics::{MetricEvent, MetricEventReceiver, MetricsCollector, MetricsSnapshot};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, RwLock};

/// 도메인별 트래픽 통계
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DomainStats {
    pub request_count: u64,
    pub error_count: u64,
    pub total_response_time_ms: u64,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
}

/// 최근 에러 항목
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEntry {
    pub domain: String,
    pub error: String,
    pub timestamp_ms: u64,
}

/// 도메인별 통계 엔트리 (프로토콜 전송용)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainStatsEntry {
    pub domain: String,
    pub stats: DomainStats,
}

/// 메트릭 집계기
///
/// MetricsCollector에서 발행된 이벤트를 백그라운드로 수신하여
/// 도메인별 통계와 최근 에러 목록을 유지합니다.
pub struct MetricsAggregator {
    collector: Arc<MetricsCollector>,
    domain_stats: Arc<RwLock<HashMap<String, DomainStats>>>,
    recent_errors: Arc<Mutex<VecDeque<ErrorEntry>>>,
    start_time: Instant,
}

/// 최근 에러 최대 보관 수
const MAX_RECENT_ERRORS: usize = 100;

/// 도메인 통계 최대 보관 수
const MAX_DOMAIN_STATS: usize = 1000;

impl MetricsAggregator {
    /// 새로운 MetricsAggregator를 생성합니다.
    pub fn new(collector: Arc<MetricsCollector>) -> Self {
        Self {
            collector,
            domain_stats: Arc::new(RwLock::new(HashMap::new())),
            recent_errors: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_RECENT_ERRORS))),
            start_time: Instant::now(),
        }
    }

    /// 백그라운드 집계 루프를 시작합니다.
    /// 반환된 JoinHandle을 보관하여 종료 시 정리할 수 있습니다.
    pub fn spawn_aggregation_loop(
        &self,
        mut event_rx: MetricEventReceiver,
    ) -> tokio::task::JoinHandle<()> {
        let domain_stats = self.domain_stats.clone();
        let recent_errors = self.recent_errors.clone();
        let start_time = self.start_time;

        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                match event {
                    MetricEvent::RequestCompleted {
                        domain,
                        status,
                        duration_ms,
                        bytes_sent,
                        bytes_received,
                    } => {
                        let mut stats = domain_stats.write().await;
                        // 도메인 수 제한: 최대 수를 초과하면 가장 적은 요청 수의 도메인 제거
                        if !stats.contains_key(&domain) && stats.len() >= MAX_DOMAIN_STATS {
                            if let Some(min_domain) = stats
                                .iter()
                                .min_by_key(|(_, s)| s.request_count)
                                .map(|(d, _)| d.clone())
                            {
                                stats.remove(&min_domain);
                            }
                        }
                        let entry = stats.entry(domain).or_default();
                        entry.request_count += 1;
                        entry.total_response_time_ms += duration_ms;
                        entry.total_bytes_sent += bytes_sent;
                        entry.total_bytes_received += bytes_received;
                        if status >= 400 {
                            entry.error_count += 1;
                        }
                    }
                    MetricEvent::ConnectionFailed { domain, error } => {
                        {
                            let mut stats = domain_stats.write().await;
                            let entry = stats.entry(domain.clone()).or_default();
                            entry.error_count += 1;
                        }
                        {
                            let elapsed_ms = start_time.elapsed().as_millis() as u64;
                            let mut errors = recent_errors.lock().await;
                            if errors.len() >= MAX_RECENT_ERRORS {
                                errors.pop_front();
                            }
                            errors.push_back(ErrorEntry {
                                domain,
                                error,
                                timestamp_ms: elapsed_ms,
                            });
                        }
                    }
                    MetricEvent::TlsHandshake {
                        domain,
                        success,
                        error,
                        ..
                    } => {
                        if !success {
                            let elapsed_ms = start_time.elapsed().as_millis() as u64;
                            let mut errors = recent_errors.lock().await;
                            if errors.len() >= MAX_RECENT_ERRORS {
                                errors.pop_front();
                            }
                            errors.push_back(ErrorEntry {
                                domain,
                                error: error.unwrap_or_else(|| "TLS handshake failed".to_string()),
                                timestamp_ms: elapsed_ms,
                            });
                        }
                    }
                    MetricEvent::RequestStarted | MetricEvent::RequestFinished => {
                        // Atomic 카운터는 MetricsCollector에서 직접 관리
                    }
                }
            }
        })
    }

    /// 글로벌 메트릭 스냅샷을 반환합니다.
    pub fn get_metrics_snapshot(&self) -> MetricsSnapshot {
        self.collector.snapshot()
    }

    /// 데몬 시작 이후 경과 시간(초)을 반환합니다.
    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// 도메인별 통계를 반환합니다.
    /// domain이 Some이면 해당 도메인만, None이면 전체 반환.
    pub async fn get_domain_stats(&self, domain: Option<&str>) -> Vec<DomainStatsEntry> {
        let stats = self.domain_stats.read().await;
        match domain {
            Some(d) => stats
                .get(d)
                .map(|s| {
                    vec![DomainStatsEntry {
                        domain: d.to_string(),
                        stats: s.clone(),
                    }]
                })
                .unwrap_or_default(),
            None => stats
                .iter()
                .map(|(domain, s)| DomainStatsEntry {
                    domain: domain.clone(),
                    stats: s.clone(),
                })
                .collect(),
        }
    }

    /// 최근 에러 목록을 반환합니다.
    pub async fn get_recent_errors(&self, limit: Option<usize>) -> Vec<ErrorEntry> {
        let errors = self.recent_errors.lock().await;
        let limit = limit.unwrap_or(errors.len());
        errors.iter().rev().take(limit).cloned().collect()
    }
}

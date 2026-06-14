//! 성능 분석 — 느린 요청 감지 및 시간대별 트래픽 분포

use std::collections::HashMap;

use proxy_v2_models::RequestInfo;

use super::{
    get_duration_ms, is_error_status, percentile, SlowRequest, SlowRequestReport, TimelineBucket,
    TrafficAnalytics,
};

impl TrafficAnalytics {
    /// 느린 요청 분석
    pub fn slow_requests(
        entries: &[RequestInfo],
        threshold_ms: u64,
        limit: usize,
    ) -> SlowRequestReport {
        let mut durations: Vec<u64> = Vec::new();
        let mut slow: Vec<SlowRequest> = Vec::new();

        for info in entries {
            let Some(req) = &info.request else {
                continue;
            };
            let Some(dur) = get_duration_ms(info) else {
                continue;
            };
            durations.push(dur);

            if dur >= threshold_ms {
                slow.push(SlowRequest {
                    url: req.uri().to_string(),
                    method: req.method().to_string(),
                    duration_ms: dur,
                    status: info.response.as_ref().map(|r| r.status().as_u16()),
                    timestamp: req.time(),
                });
            }
        }

        // duration 내림차순 정렬
        slow.sort_by(|a, b| b.duration_ms.cmp(&a.duration_ms));
        let total_slow = slow.len();
        slow.truncate(limit);

        durations.sort();

        SlowRequestReport {
            threshold_ms,
            total_slow,
            requests: slow,
            p50_ms: percentile(&durations, 50.0),
            p95_ms: percentile(&durations, 95.0),
            p99_ms: percentile(&durations, 99.0),
        }
    }

    /// 시간대별 트래픽 분포
    pub fn traffic_timeline(entries: &[RequestInfo], bucket_seconds: u64) -> Vec<TimelineBucket> {
        if entries.is_empty() {
            return Vec::new();
        }

        // bucket_seconds가 0이면 time / bucket_ms에서 0으로 나눗셈 패닉이 발생하므로 최소 1로 클램프
        let bucket_ms = (bucket_seconds.max(1) * 1000) as i64;

        // 타임스탬프 기반으로 버킷에 넣기
        let mut buckets: HashMap<i64, (usize, usize, Vec<u64>)> = HashMap::new();

        for info in entries {
            let Some(req) = &info.request else {
                continue;
            };
            let time = req.time();
            let bucket_key = (time / bucket_ms) * bucket_ms;
            let dur = get_duration_ms(info).unwrap_or(0);
            let is_err = info
                .response
                .as_ref()
                .map(|r| is_error_status(r.status().as_u16()))
                .unwrap_or(false);

            let entry = buckets.entry(bucket_key).or_insert((0, 0, Vec::new()));
            entry.0 += 1;
            if is_err {
                entry.1 += 1;
            }
            entry.2.push(dur);
        }

        let mut timeline: Vec<TimelineBucket> = buckets
            .into_iter()
            .map(|(ts, (req_count, err_count, durs))| {
                let avg_dur = if durs.is_empty() {
                    0.0
                } else {
                    durs.iter().sum::<u64>() as f64 / durs.len() as f64
                };
                TimelineBucket {
                    timestamp: ts,
                    request_count: req_count,
                    error_count: err_count,
                    avg_duration_ms: avg_dur,
                }
            })
            .collect();

        timeline.sort_by_key(|b| b.timestamp);
        timeline
    }
}

//! 에러 분석 — 에러율, 에러 분포, 에러 엔드포인트 분석

use std::collections::HashMap;

use proxy_v2_models::RequestInfo;

use super::{
    extract_path, is_error_status, EndpointErrorStat, ErrorEntry, ErrorReport, TrafficAnalytics,
};

impl TrafficAnalytics {
    /// 에러율 분석
    pub fn error_analysis(entries: &[RequestInfo]) -> ErrorReport {
        let mut total = 0usize;
        let mut error_count = 0usize;
        let mut by_status: HashMap<u16, usize> = HashMap::new();
        let mut endpoint_map: HashMap<(String, String), (usize, usize)> = HashMap::new();
        let mut recent_errors: Vec<ErrorEntry> = Vec::new();

        for info in entries {
            let Some(req) = &info.request else {
                continue;
            };
            let Some(res) = &info.response else {
                continue;
            };
            total += 1;
            let status = res.status().as_u16();
            let path = extract_path(&req.uri().to_string());
            let method = req.method().to_string();

            let entry = endpoint_map.entry((method.clone(), path)).or_insert((0, 0));
            entry.0 += 1;

            if is_error_status(status) {
                error_count += 1;
                *by_status.entry(status).or_insert(0) += 1;
                entry.1 += 1;

                recent_errors.push(ErrorEntry {
                    url: req.uri().to_string(),
                    method,
                    status,
                    timestamp: req.time(),
                });
            }
        }

        // 최근 에러 20개
        recent_errors.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        recent_errors.truncate(20);

        // 에러율이 높은 엔드포인트 순으로
        let mut by_endpoint: Vec<EndpointErrorStat> = endpoint_map
            .into_iter()
            .filter(|(_, (_, errors))| *errors > 0)
            .map(|((method, path), (total, errors))| EndpointErrorStat {
                method,
                path,
                total,
                errors,
                error_rate: if total > 0 {
                    errors as f64 / total as f64 * 100.0
                } else {
                    0.0
                },
            })
            .collect();
        by_endpoint.sort_by(|a, b| {
            b.error_rate
                .partial_cmp(&a.error_rate)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        by_endpoint.truncate(20);

        ErrorReport {
            total_requests: total,
            error_count,
            error_rate: if total > 0 {
                error_count as f64 / total as f64 * 100.0
            } else {
                0.0
            },
            by_status,
            by_endpoint,
            recent_errors,
        }
    }
}

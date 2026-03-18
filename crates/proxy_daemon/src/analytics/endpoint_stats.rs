//! 엔드포인트 통계 — 엔드포인트별 성능, 도메인별 통계, 페이로드 크기 분석

use std::collections::HashMap;

use proxy_v2_models::RequestInfo;

use super::{
    extract_domain, extract_path, get_duration_ms, is_error_status, normalize_path_pattern,
    percentile, DomainStat, EndpointStat, PayloadEntry, PayloadReport, TrafficAnalytics,
};

impl TrafficAnalytics {
    /// 엔드포인트별 통계
    pub fn endpoint_stats(entries: &[RequestInfo]) -> Vec<EndpointStat> {
        let mut map: HashMap<(String, String), Vec<(u64, bool, u64)>> = HashMap::new();

        for info in entries {
            let Some(req) = &info.request else {
                continue;
            };
            let Some(res) = &info.response else {
                continue;
            };
            let dur = get_duration_ms(info).unwrap_or(0);
            let method = req.method().to_string();
            let path = normalize_path_pattern(&extract_path(&req.uri().to_string()));
            let is_err = is_error_status(res.status().as_u16());
            let resp_size = res.body_size() as u64;

            map.entry((method, path))
                .or_default()
                .push((dur, is_err, resp_size));
        }

        let mut stats: Vec<EndpointStat> = map
            .into_iter()
            .map(|((method, path), values)| {
                let count = values.len();
                let total_dur: u64 = values.iter().map(|(d, _, _)| d).sum();
                let error_count = values.iter().filter(|(_, e, _)| *e).count();
                let total_size: u64 = values.iter().map(|(_, _, s)| s).sum();

                let mut sorted_dur: Vec<u64> = values.iter().map(|(d, _, _)| *d).collect();
                sorted_dur.sort();

                EndpointStat {
                    method,
                    path,
                    count,
                    avg_duration_ms: if count > 0 {
                        total_dur as f64 / count as f64
                    } else {
                        0.0
                    },
                    p95_duration_ms: percentile(&sorted_dur, 95.0),
                    error_rate: if count > 0 {
                        error_count as f64 / count as f64 * 100.0
                    } else {
                        0.0
                    },
                    avg_response_size: if count > 0 {
                        total_size / count as u64
                    } else {
                        0
                    },
                }
            })
            .collect();

        stats.sort_by(|a, b| b.count.cmp(&a.count));
        stats
    }

    /// 도메인별 통계
    pub fn domain_breakdown(entries: &[RequestInfo]) -> Vec<DomainStat> {
        let mut map: HashMap<String, (usize, usize, Vec<u64>, u64, u64)> = HashMap::new();

        for info in entries {
            let Some(req) = &info.request else {
                continue;
            };
            let domain = extract_domain(&req.uri().to_string());
            let dur = get_duration_ms(info).unwrap_or(0);
            let is_err = info
                .response
                .as_ref()
                .map(|r| is_error_status(r.status().as_u16()))
                .unwrap_or(false);
            let req_size = req.body_size() as u64;
            let res_size = info
                .response
                .as_ref()
                .map(|r| r.body_size() as u64)
                .unwrap_or(0);

            let entry = map.entry(domain).or_insert((0, 0, Vec::new(), 0, 0));
            entry.0 += 1;
            if is_err {
                entry.1 += 1;
            }
            entry.2.push(dur);
            entry.3 += req_size;
            entry.4 += res_size;
        }

        let mut stats: Vec<DomainStat> = map
            .into_iter()
            .map(|(domain, (count, errors, durs, sent, recv))| {
                let avg_dur = if durs.is_empty() {
                    0.0
                } else {
                    durs.iter().sum::<u64>() as f64 / durs.len() as f64
                };
                DomainStat {
                    domain,
                    request_count: count,
                    error_count: errors,
                    avg_duration_ms: avg_dur,
                    total_bytes_sent: sent,
                    total_bytes_received: recv,
                }
            })
            .collect();

        stats.sort_by(|a, b| b.request_count.cmp(&a.request_count));
        stats
    }

    /// 페이로드 크기 분석
    pub fn payload_size_analysis(entries: &[RequestInfo]) -> PayloadReport {
        let mut total = 0usize;
        let mut total_req_size = 0u64;
        let mut total_res_size = 0u64;
        let mut max_req_size = 0u64;
        let mut max_res_size = 0u64;
        let mut req_entries: Vec<PayloadEntry> = Vec::new();
        let mut res_entries: Vec<PayloadEntry> = Vec::new();

        for info in entries {
            let Some(req) = &info.request else {
                continue;
            };
            total += 1;
            let req_size = req.body_size() as u64;
            let res_size = info
                .response
                .as_ref()
                .map(|r| r.body_size() as u64)
                .unwrap_or(0);

            total_req_size += req_size;
            total_res_size += res_size;

            if req_size > max_req_size {
                max_req_size = req_size;
            }
            if res_size > max_res_size {
                max_res_size = res_size;
            }

            req_entries.push(PayloadEntry {
                url: req.uri().to_string(),
                method: req.method().to_string(),
                size: req_size,
            });
            res_entries.push(PayloadEntry {
                url: req.uri().to_string(),
                method: req.method().to_string(),
                size: res_size,
            });
        }

        req_entries.sort_by(|a, b| b.size.cmp(&a.size));
        res_entries.sort_by(|a, b| b.size.cmp(&a.size));
        req_entries.truncate(10);
        res_entries.truncate(10);

        PayloadReport {
            total_requests: total,
            avg_request_size: if total > 0 {
                total_req_size / total as u64
            } else {
                0
            },
            avg_response_size: if total > 0 {
                total_res_size / total as u64
            } else {
                0
            },
            max_request_size: max_req_size,
            max_response_size: max_res_size,
            largest_requests: req_entries,
            largest_responses: res_entries,
        }
    }
}

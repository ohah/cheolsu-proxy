//! 트래픽 분석 엔진 — 캡처된 HTTP 트래픽을 분석하여 구조화된 리포트를 생성합니다.

use proxy_v2_models::RequestInfo;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Report 구조체들 ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlowRequest {
    pub url: String,
    pub method: String,
    pub duration_ms: u64,
    pub status: Option<u16>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlowRequestReport {
    pub threshold_ms: u64,
    pub total_slow: usize,
    pub requests: Vec<SlowRequest>,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointErrorStat {
    pub method: String,
    pub path: String,
    pub total: usize,
    pub errors: usize,
    pub error_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEntry {
    pub url: String,
    pub method: String,
    pub status: u16,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorReport {
    pub total_requests: usize,
    pub error_count: usize,
    pub error_rate: f64,
    pub by_status: HashMap<u16, usize>,
    pub by_endpoint: Vec<EndpointErrorStat>,
    pub recent_errors: Vec<ErrorEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointStat {
    pub method: String,
    pub path: String,
    pub count: usize,
    pub avg_duration_ms: f64,
    pub p95_duration_ms: f64,
    pub error_rate: f64,
    pub avg_response_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub method: String,
    pub url: String,
    pub count: usize,
    pub window_ms: u64,
    pub first_seen: i64,
    pub last_seen: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NPlus1Pattern {
    pub base_url: String,
    pub pattern: String,
    pub count: usize,
    pub window_ms: u64,
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineBucket {
    pub timestamp: i64,
    pub request_count: usize,
    pub error_count: usize,
    pub avg_duration_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainStat {
    pub domain: String,
    pub request_count: usize,
    pub error_count: usize,
    pub avg_duration_ms: f64,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadReport {
    pub total_requests: usize,
    pub avg_request_size: u64,
    pub avg_response_size: u64,
    pub max_request_size: u64,
    pub max_response_size: u64,
    pub largest_requests: Vec<PayloadEntry>,
    pub largest_responses: Vec<PayloadEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadEntry {
    pub url: String,
    pub method: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsIssue {
    pub url: String,
    pub issue_type: String,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixedContentWarning {
    pub secure_page: String,
    pub insecure_resource: String,
    pub resource_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullReport {
    pub slow_requests: SlowRequestReport,
    pub errors: ErrorReport,
    pub top_endpoints: Vec<EndpointStat>,
    pub duplicates: Vec<DuplicateGroup>,
    pub n_plus_one: Vec<NPlus1Pattern>,
    pub domain_breakdown: Vec<DomainStat>,
    pub payload: PayloadReport,
    pub cors_issues: Vec<CorsIssue>,
    pub mixed_content: Vec<MixedContentWarning>,
}

// ─── 헬퍼 함수들 ─────────────────────────────────────────

/// RequestInfo에서 duration_ms를 추출 (response.time - request.time)
fn duration_ms(info: &RequestInfo) -> Option<u64> {
    let req_time = info.request.as_ref()?.time();
    let res_time = info.response.as_ref()?.time();
    let diff = res_time - req_time;
    if diff >= 0 {
        Some(diff as u64)
    } else {
        None
    }
}

/// RequestInfo에서 timing.total_ms를 추출 (더 정확)
fn timing_total_ms(info: &RequestInfo) -> Option<u64> {
    info.response
        .as_ref()?
        .timing()
        .as_ref()
        .map(|t| t.total_ms)
}

/// 요청의 duration을 가져옴 (timing이 있으면 우선, 아니면 time 차이 사용)
fn get_duration_ms(info: &RequestInfo) -> Option<u64> {
    timing_total_ms(info).or_else(|| duration_ms(info))
}

/// URI에서 path 부분만 추출 (문자열 파싱)
fn extract_path(uri: &str) -> String {
    // scheme://host[:port]/path?query 형태에서 path 추출
    if let Some(after_scheme) = uri.find("://") {
        let rest = &uri[after_scheme + 3..];
        if let Some(slash_pos) = rest.find('/') {
            let path_and_query = &rest[slash_pos..];
            // query 제거
            if let Some(q) = path_and_query.find('?') {
                path_and_query[..q].to_string()
            } else {
                path_and_query.to_string()
            }
        } else {
            "/".to_string()
        }
    } else {
        // scheme이 없으면 전체가 path
        if let Some(q) = uri.find('?') {
            uri[..q].to_string()
        } else {
            uri.to_string()
        }
    }
}

/// URI에서 도메인 추출 (문자열 파싱)
fn extract_domain(uri: &str) -> String {
    if let Some(after_scheme) = uri.find("://") {
        let rest = &uri[after_scheme + 3..];
        let host_port = if let Some(slash_pos) = rest.find('/') {
            &rest[..slash_pos]
        } else {
            rest
        };
        // port 제거
        if let Some(colon) = host_port.rfind(':') {
            // IPv6 주소 대응: [::1]:8080
            if host_port.starts_with('[') {
                if let Some(bracket) = host_port.find(']') {
                    host_port[1..bracket].to_string()
                } else {
                    host_port.to_string()
                }
            } else {
                host_port[..colon].to_string()
            }
        } else {
            host_port.to_string()
        }
    } else {
        "unknown".to_string()
    }
}

/// URI에서 scheme 추출 (문자열 파싱)
fn extract_scheme(uri: &str) -> String {
    if let Some(pos) = uri.find("://") {
        uri[..pos].to_string()
    } else {
        "unknown".to_string()
    }
}

/// 상태코드가 에러인지 확인 (4xx, 5xx)
fn is_error_status(status: u16) -> bool {
    status >= 400
}

/// 백분위수 계산
fn percentile(sorted_values: &[u64], p: f64) -> f64 {
    if sorted_values.is_empty() {
        return 0.0;
    }
    if sorted_values.len() == 1 {
        return sorted_values[0] as f64;
    }
    let idx = (p / 100.0 * (sorted_values.len() - 1) as f64).round() as usize;
    let idx = idx.min(sorted_values.len() - 1);
    sorted_values[idx] as f64
}

/// path에서 ID 패턴을 추출 (숫자 부분을 {id}로 변환)
fn normalize_path_pattern(path: &str) -> String {
    let segments: Vec<&str> = path.split('/').collect();
    let normalized: Vec<String> = segments
        .iter()
        .map(|seg| {
            // 순수 숫자이거나, UUID 패턴이면 {id}로 변환
            if seg.chars().all(|c| c.is_ascii_digit()) && !seg.is_empty() {
                "{id}".to_string()
            } else if seg.len() >= 32 && seg.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
                "{id}".to_string()
            } else {
                seg.to_string()
            }
        })
        .collect();
    normalized.join("/")
}

// ─── TrafficAnalytics ────────────────────────────────────

pub struct TrafficAnalytics;

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
        slow.truncate(limit);

        durations.sort();

        SlowRequestReport {
            threshold_ms,
            total_slow: slow.len(),
            requests: slow,
            p50_ms: percentile(&durations, 50.0),
            p95_ms: percentile(&durations, 95.0),
            p99_ms: percentile(&durations, 99.0),
        }
    }

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

    /// 중복 요청 감지
    pub fn duplicate_requests(entries: &[RequestInfo], window_ms: u64) -> Vec<DuplicateGroup> {
        // (method, url) -> 타임스탬프 목록
        let mut groups: HashMap<(String, String), Vec<i64>> = HashMap::new();

        for info in entries {
            let Some(req) = &info.request else {
                continue;
            };
            let key = (req.method().to_string(), req.uri().to_string());
            groups.entry(key).or_default().push(req.time());
        }

        let mut result: Vec<DuplicateGroup> = Vec::new();
        let window = window_ms as i64;

        for ((method, url), times) in &groups {
            if times.len() < 2 {
                continue;
            }
            let mut sorted = times.clone();
            sorted.sort();

            // 슬라이딩 윈도우로 같은 window 내의 요청 그룹 찾기
            let mut i = 0;
            while i < sorted.len() {
                let start = sorted[i];
                let mut j = i + 1;
                while j < sorted.len() && (sorted[j] - start) <= window {
                    j += 1;
                }
                let count = j - i;
                if count >= 2 {
                    result.push(DuplicateGroup {
                        method: method.clone(),
                        url: url.clone(),
                        count,
                        window_ms,
                        first_seen: sorted[i],
                        last_seen: sorted[j - 1],
                    });
                }
                i = j;
            }
        }

        result.sort_by(|a, b| b.count.cmp(&a.count));
        result
    }

    /// N+1 쿼리 패턴 감지
    pub fn n_plus_one_detection(entries: &[RequestInfo]) -> Vec<NPlus1Pattern> {
        // 정규화된 path 패턴별로 연속 호출 감지
        let mut pattern_groups: HashMap<(String, String), Vec<i64>> = HashMap::new();

        for info in entries {
            let Some(req) = &info.request else {
                continue;
            };
            let uri = req.uri().to_string();
            let path = extract_path(&uri);
            let normalized = normalize_path_pattern(&path);

            // {id} 포함 패턴만 대상 (리소스 개별 조회 패턴)
            if normalized.contains("{id}") {
                let base_url = extract_domain(&uri);
                let key = (base_url, normalized);
                pattern_groups.entry(key).or_default().push(req.time());
            }
        }

        let mut result: Vec<NPlus1Pattern> = Vec::new();

        for ((base_url, pattern), times) in &pattern_groups {
            if times.len() < 3 {
                continue;
            }
            let mut sorted = times.clone();
            sorted.sort();

            // 5초 윈도우 내에 3개 이상 연속 호출이면 N+1 의심
            let window = 5000i64;
            let mut i = 0;
            while i < sorted.len() {
                let start = sorted[i];
                let mut j = i + 1;
                while j < sorted.len() && (sorted[j] - start) <= window {
                    j += 1;
                }
                let count = j - i;
                if count >= 3 {
                    let elapsed = if j > i + 1 {
                        (sorted[j - 1] - sorted[i]) as u64
                    } else {
                        0
                    };
                    result.push(NPlus1Pattern {
                        base_url: base_url.clone(),
                        pattern: pattern.clone(),
                        count,
                        window_ms: elapsed,
                        suggestion: format!(
                            "Consider using a batch/list endpoint instead of {} individual requests to {}",
                            count, pattern
                        ),
                    });
                }
                i = j;
            }
        }

        result.sort_by(|a, b| b.count.cmp(&a.count));
        result
    }

    /// 시간대별 트래픽 분포
    pub fn traffic_timeline(entries: &[RequestInfo], bucket_seconds: u64) -> Vec<TimelineBucket> {
        if entries.is_empty() {
            return Vec::new();
        }

        let bucket_ms = (bucket_seconds * 1000) as i64;

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

    /// CORS 문제 감지
    pub fn cors_issues(entries: &[RequestInfo]) -> Vec<CorsIssue> {
        let mut issues: Vec<CorsIssue> = Vec::new();

        for info in entries {
            let Some(req) = &info.request else {
                continue;
            };
            let Some(res) = &info.response else {
                continue;
            };

            let url = req.uri().to_string();
            let has_origin = req.headers().get("origin").is_some();

            if !has_origin {
                continue;
            }

            // OPTIONS 프리플라이트 체크
            if req.method() == "OPTIONS" {
                let has_allow_origin = res.headers().get("access-control-allow-origin").is_some();
                let has_allow_methods = res.headers().get("access-control-allow-methods").is_some();

                if !has_allow_origin {
                    issues.push(CorsIssue {
                        url: url.clone(),
                        issue_type: "preflight_missing_allow_origin".to_string(),
                        details: "OPTIONS preflight response is missing Access-Control-Allow-Origin header".to_string(),
                    });
                }
                if !has_allow_methods {
                    issues.push(CorsIssue {
                        url: url.clone(),
                        issue_type: "preflight_missing_allow_methods".to_string(),
                        details: "OPTIONS preflight response is missing Access-Control-Allow-Methods header".to_string(),
                    });
                }
                if is_error_status(res.status().as_u16()) {
                    issues.push(CorsIssue {
                        url: url.clone(),
                        issue_type: "preflight_failed".to_string(),
                        details: format!(
                            "OPTIONS preflight returned error status {}",
                            res.status().as_u16()
                        ),
                    });
                }
            } else {
                // 일반 CORS 요청
                let has_allow_origin = res.headers().get("access-control-allow-origin").is_some();
                if !has_allow_origin {
                    issues.push(CorsIssue {
                        url,
                        issue_type: "missing_allow_origin".to_string(),
                        details: "Cross-origin request response is missing Access-Control-Allow-Origin header".to_string(),
                    });
                }
            }
        }

        issues
    }

    /// Mixed Content 경고
    pub fn mixed_content_warnings(entries: &[RequestInfo]) -> Vec<MixedContentWarning> {
        let mut warnings: Vec<MixedContentWarning> = Vec::new();

        // HTTPS 페이지에서 HTTP 리소스를 로드하는 패턴 감지
        // Referer 헤더로 부모 페이지 추적
        for info in entries {
            let Some(req) = &info.request else {
                continue;
            };
            let url = req.uri().to_string();
            let scheme = extract_scheme(&url);

            if scheme == "http" {
                // Referer가 HTTPS인지 확인
                if let Some(referer) = req.headers().get("referer") {
                    if let Ok(referer_str) = referer.to_str() {
                        if referer_str.starts_with("https://") {
                            // Content-Type으로 리소스 타입 추론
                            let resource_type = info
                                .response
                                .as_ref()
                                .and_then(|r| r.headers().get("content-type"))
                                .and_then(|ct| ct.to_str().ok())
                                .map(|ct| {
                                    if ct.starts_with("image/") {
                                        "image"
                                    } else if ct.starts_with("text/css") {
                                        "stylesheet"
                                    } else if ct.contains("javascript") {
                                        "script"
                                    } else if ct.starts_with("font/") {
                                        "font"
                                    } else {
                                        "other"
                                    }
                                })
                                .unwrap_or("unknown")
                                .to_string();

                            warnings.push(MixedContentWarning {
                                secure_page: referer_str.to_string(),
                                insecure_resource: url,
                                resource_type,
                            });
                        }
                    }
                }
            }
        }

        warnings
    }

    /// 전체 분석 보고서
    pub fn full_report(entries: &[RequestInfo]) -> FullReport {
        let mut endpoint_stats = Self::endpoint_stats(entries);
        endpoint_stats.truncate(20);

        FullReport {
            slow_requests: Self::slow_requests(entries, 1000, 10),
            errors: Self::error_analysis(entries),
            top_endpoints: endpoint_stats,
            duplicates: Self::duplicate_requests(entries, 3000),
            n_plus_one: Self::n_plus_one_detection(entries),
            domain_breakdown: Self::domain_breakdown(entries),
            payload: Self::payload_size_analysis(entries),
            cors_issues: Self::cors_issues(entries),
            mixed_content: Self::mixed_content_warnings(entries),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use http::{HeaderMap, HeaderValue, Method, StatusCode, Uri, Version};
    use proxy_v2_models::{ClientRequest, ClientResponse, RequestInfo};

    fn make_client_request(method: &str, uri: &str, time: i64) -> ClientRequest {
        // ClientRequest를 ProxiedRequest::for_client로 만들어야 하지만
        // 테스트에서는 직접 생성
        let req = proxy_v2_models::ProxiedRequest::new(
            method.parse::<Method>().unwrap(),
            uri.parse::<Uri>().unwrap(),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
            time,
        );
        req.for_client(None)
    }

    fn make_client_response(status: u16, time: i64) -> ClientResponse {
        let res = proxy_v2_models::ProxiedResponse::new(
            StatusCode::from_u16(status).unwrap(),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
            time,
        );
        res.for_client("test", None)
    }

    fn make_info(
        method: &str,
        uri: &str,
        status: u16,
        req_time: i64,
        res_time: i64,
    ) -> RequestInfo {
        RequestInfo {
            request: Some(make_client_request(method, uri, req_time)),
            response: Some(make_client_response(status, res_time)),
            validations: None,
        }
    }

    #[test]
    fn test_slow_requests_empty() {
        let report = TrafficAnalytics::slow_requests(&[], 1000, 10);
        assert_eq!(report.total_slow, 0);
        assert_eq!(report.p50_ms, 0.0);
    }

    #[test]
    fn test_slow_requests_filters_by_threshold() {
        let entries = vec![
            make_info("GET", "http://example.com/fast", 200, 1000, 1100),
            make_info("GET", "http://example.com/slow", 200, 2000, 4000),
        ];
        let report = TrafficAnalytics::slow_requests(&entries, 500, 10);
        assert_eq!(report.total_slow, 1);
        assert_eq!(report.requests[0].url, "http://example.com/slow");
    }

    #[test]
    fn test_error_analysis() {
        let entries = vec![
            make_info("GET", "http://example.com/ok", 200, 1000, 1100),
            make_info("GET", "http://example.com/err", 500, 2000, 2100),
            make_info("POST", "http://example.com/err", 404, 3000, 3100),
        ];
        let report = TrafficAnalytics::error_analysis(&entries);
        assert_eq!(report.total_requests, 3);
        assert_eq!(report.error_count, 2);
        assert!(report.error_rate > 60.0);
        assert_eq!(*report.by_status.get(&500).unwrap(), 1);
        assert_eq!(*report.by_status.get(&404).unwrap(), 1);
    }

    #[test]
    fn test_endpoint_stats() {
        let entries = vec![
            make_info("GET", "http://example.com/api/users", 200, 1000, 1100),
            make_info("GET", "http://example.com/api/users", 200, 2000, 2200),
            make_info("POST", "http://example.com/api/users", 201, 3000, 3050),
        ];
        let stats = TrafficAnalytics::endpoint_stats(&entries);
        assert!(!stats.is_empty());
        // GET /api/users 가 2개로 가장 많아야 함
        assert_eq!(stats[0].count, 2);
    }

    #[test]
    fn test_duplicate_requests() {
        let entries = vec![
            make_info("GET", "http://example.com/api/data", 200, 1000, 1100),
            make_info("GET", "http://example.com/api/data", 200, 1500, 1600),
            make_info("GET", "http://example.com/api/data", 200, 2000, 2100),
            make_info("GET", "http://example.com/other", 200, 1000, 1100),
        ];
        let dupes = TrafficAnalytics::duplicate_requests(&entries, 3000);
        assert!(!dupes.is_empty());
        assert_eq!(dupes[0].count, 3);
    }

    #[test]
    fn test_n_plus_one_detection() {
        let entries = vec![
            make_info("GET", "http://example.com/api/users/1", 200, 1000, 1100),
            make_info("GET", "http://example.com/api/users/2", 200, 1200, 1300),
            make_info("GET", "http://example.com/api/users/3", 200, 1400, 1500),
            make_info("GET", "http://example.com/api/users/4", 200, 1600, 1700),
        ];
        let patterns = TrafficAnalytics::n_plus_one_detection(&entries);
        assert!(!patterns.is_empty());
        assert!(patterns[0].pattern.contains("{id}"));
        assert!(patterns[0].count >= 3);
    }

    #[test]
    fn test_traffic_timeline() {
        let entries = vec![
            make_info("GET", "http://example.com/a", 200, 0, 100),
            make_info("GET", "http://example.com/b", 500, 500, 600),
            make_info("GET", "http://example.com/c", 200, 60000, 60100),
        ];
        let timeline = TrafficAnalytics::traffic_timeline(&entries, 60);
        assert!(!timeline.is_empty());
    }

    #[test]
    fn test_domain_breakdown() {
        let entries = vec![
            make_info("GET", "http://api.example.com/a", 200, 1000, 1100),
            make_info("GET", "http://api.example.com/b", 200, 2000, 2100),
            make_info("GET", "http://cdn.example.com/img", 200, 3000, 3100),
        ];
        let stats = TrafficAnalytics::domain_breakdown(&entries);
        assert_eq!(stats.len(), 2);
        assert_eq!(stats[0].request_count, 2);
    }

    #[test]
    fn test_payload_size_analysis() {
        let entries = vec![make_info("GET", "http://example.com/a", 200, 1000, 1100)];
        let report = TrafficAnalytics::payload_size_analysis(&entries);
        assert_eq!(report.total_requests, 1);
    }

    #[test]
    fn test_cors_issues_with_origin() {
        let req = {
            let mut headers = HeaderMap::new();
            headers.insert("origin", HeaderValue::from_static("http://localhost:3000"));
            let r = proxy_v2_models::ProxiedRequest::new(
                Method::GET,
                "http://api.example.com/data".parse().unwrap(),
                Version::HTTP_11,
                headers,
                Bytes::new(),
                1000,
            );
            r.for_client(None)
        };
        let res = make_client_response(200, 1100);

        let entries = vec![RequestInfo {
            request: Some(req),
            response: Some(res),
            validations: None,
        }];

        let issues = TrafficAnalytics::cors_issues(&entries);
        // 응답에 ACAO 헤더가 없으므로 이슈 감지
        assert!(!issues.is_empty());
        assert_eq!(issues[0].issue_type, "missing_allow_origin");
    }

    #[test]
    fn test_mixed_content_detection() {
        let req = {
            let mut headers = HeaderMap::new();
            headers.insert(
                "referer",
                HeaderValue::from_static("https://secure.example.com/page"),
            );
            let r = proxy_v2_models::ProxiedRequest::new(
                Method::GET,
                "http://insecure.example.com/image.png".parse().unwrap(),
                Version::HTTP_11,
                headers,
                Bytes::new(),
                1000,
            );
            r.for_client(None)
        };

        let res = {
            let mut headers = HeaderMap::new();
            headers.insert("content-type", HeaderValue::from_static("image/png"));
            let r = proxy_v2_models::ProxiedResponse::new(
                StatusCode::OK,
                Version::HTTP_11,
                headers,
                Bytes::new(),
                1100,
            );
            r.for_client("test", None)
        };

        let entries = vec![RequestInfo {
            request: Some(req),
            response: Some(res),
            validations: None,
        }];

        let warnings = TrafficAnalytics::mixed_content_warnings(&entries);
        assert!(!warnings.is_empty());
        assert_eq!(warnings[0].resource_type, "image");
    }

    #[test]
    fn test_full_report() {
        let entries = vec![
            make_info("GET", "http://example.com/api/users", 200, 1000, 1100),
            make_info("GET", "http://example.com/api/users/1", 200, 2000, 2100),
            make_info("GET", "http://example.com/api/users/2", 500, 3000, 3100),
        ];
        let report = TrafficAnalytics::full_report(&entries);
        assert_eq!(report.errors.total_requests, 3);
        assert_eq!(report.errors.error_count, 1);
    }

    #[test]
    fn test_normalize_path_pattern() {
        assert_eq!(normalize_path_pattern("/api/users/123"), "/api/users/{id}");
        assert_eq!(normalize_path_pattern("/api/users/abc"), "/api/users/abc");
        assert_eq!(
            normalize_path_pattern("/api/users/550e8400-e29b-41d4-a716-446655440000"),
            "/api/users/{id}"
        );
    }

    #[test]
    fn test_percentile() {
        let values = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        // p50 of 10 elements: index = round(0.5 * 9) = round(4.5) = 5 -> values[5] = 6
        assert_eq!(percentile(&values, 50.0), 6.0);
        assert_eq!(percentile(&values, 100.0), 10.0);
        assert_eq!(percentile(&[], 50.0), 0.0);
        assert_eq!(percentile(&[42], 50.0), 42.0);
    }
}

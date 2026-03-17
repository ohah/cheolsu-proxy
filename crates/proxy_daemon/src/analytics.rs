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
            server_cert: None,
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
            server_cert: None,
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
            server_cert: None,
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

    // ─── 헬퍼: 커스텀 헤더가 있는 요청/응답 생성 ─────────────

    fn make_info_with_headers(
        method: &str,
        uri: &str,
        status: u16,
        req_time: i64,
        res_time: i64,
        req_headers: HeaderMap,
        res_headers: HeaderMap,
    ) -> RequestInfo {
        let req = proxy_v2_models::ProxiedRequest::new(
            method.parse::<Method>().unwrap(),
            uri.parse::<Uri>().unwrap(),
            Version::HTTP_11,
            req_headers,
            Bytes::new(),
            req_time,
        );
        let res = proxy_v2_models::ProxiedResponse::new(
            StatusCode::from_u16(status).unwrap(),
            Version::HTTP_11,
            res_headers,
            Bytes::new(),
            res_time,
        );
        RequestInfo {
            request: Some(req.for_client(None)),
            response: Some(res.for_client("test", None)),
            validations: None,
            server_cert: None,
        }
    }

    fn make_info_with_body(
        method: &str,
        uri: &str,
        status: u16,
        req_time: i64,
        res_time: i64,
        req_body: &[u8],
        res_body: &[u8],
    ) -> RequestInfo {
        let req = proxy_v2_models::ProxiedRequest::new(
            method.parse::<Method>().unwrap(),
            uri.parse::<Uri>().unwrap(),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::from(req_body.to_vec()),
            req_time,
        );
        let res = proxy_v2_models::ProxiedResponse::new(
            StatusCode::from_u16(status).unwrap(),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::from(res_body.to_vec()),
            res_time,
        );
        RequestInfo {
            request: Some(req.for_client(None)),
            response: Some(res.for_client("test", None)),
            validations: None,
            server_cert: None,
        }
    }

    // ─── 엣지 케이스: 빈 데이터에 대한 모든 분석 함수 ────────

    #[test]
    fn test_error_analysis_empty() {
        let report = TrafficAnalytics::error_analysis(&[]);
        assert_eq!(report.total_requests, 0);
        assert_eq!(report.error_count, 0);
        assert_eq!(report.error_rate, 0.0);
        assert!(report.by_status.is_empty());
        assert!(report.by_endpoint.is_empty());
        assert!(report.recent_errors.is_empty());
    }

    #[test]
    fn test_endpoint_stats_empty() {
        let stats = TrafficAnalytics::endpoint_stats(&[]);
        assert!(stats.is_empty());
    }

    #[test]
    fn test_duplicate_requests_empty() {
        let dupes = TrafficAnalytics::duplicate_requests(&[], 3000);
        assert!(dupes.is_empty());
    }

    #[test]
    fn test_n_plus_one_detection_empty() {
        let patterns = TrafficAnalytics::n_plus_one_detection(&[]);
        assert!(patterns.is_empty());
    }

    #[test]
    fn test_traffic_timeline_empty() {
        let timeline = TrafficAnalytics::traffic_timeline(&[], 60);
        assert!(timeline.is_empty());
    }

    #[test]
    fn test_domain_breakdown_empty() {
        let stats = TrafficAnalytics::domain_breakdown(&[]);
        assert!(stats.is_empty());
    }

    #[test]
    fn test_payload_size_analysis_empty() {
        let report = TrafficAnalytics::payload_size_analysis(&[]);
        assert_eq!(report.total_requests, 0);
        assert_eq!(report.avg_request_size, 0);
        assert_eq!(report.avg_response_size, 0);
        assert_eq!(report.max_request_size, 0);
        assert_eq!(report.max_response_size, 0);
        assert!(report.largest_requests.is_empty());
        assert!(report.largest_responses.is_empty());
    }

    #[test]
    fn test_cors_issues_empty() {
        let issues = TrafficAnalytics::cors_issues(&[]);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_mixed_content_warnings_empty() {
        let warnings = TrafficAnalytics::mixed_content_warnings(&[]);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_full_report_empty() {
        let report = TrafficAnalytics::full_report(&[]);
        assert_eq!(report.slow_requests.total_slow, 0);
        assert_eq!(report.errors.total_requests, 0);
        assert!(report.top_endpoints.is_empty());
        assert!(report.duplicates.is_empty());
        assert!(report.n_plus_one.is_empty());
        assert!(report.domain_breakdown.is_empty());
        assert_eq!(report.payload.total_requests, 0);
        assert!(report.cors_issues.is_empty());
        assert!(report.mixed_content.is_empty());
    }

    // ─── 엣지 케이스: 단일 요청에 대한 퍼센타일 계산 ─────────

    #[test]
    fn test_single_request_percentiles() {
        let entries = vec![make_info("GET", "http://example.com/a", 200, 1000, 1150)];
        let report = TrafficAnalytics::slow_requests(&entries, 0, 10);
        // 단일 값이면 P50/P95/P99 모두 같아야 함
        assert_eq!(report.p50_ms, report.p95_ms);
        assert_eq!(report.p95_ms, report.p99_ms);
        assert_eq!(report.total_slow, 1);
    }

    // ─── 엣지 케이스: 0ms 응답 시간 ─────────────────────────

    #[test]
    fn test_zero_duration_request() {
        let entries = vec![make_info(
            "GET",
            "http://example.com/instant",
            200,
            1000,
            1000,
        )];
        let report = TrafficAnalytics::slow_requests(&entries, 1000, 10);
        assert_eq!(report.total_slow, 0);
        assert_eq!(report.p50_ms, 0.0);

        let stats = TrafficAnalytics::endpoint_stats(&entries);
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].avg_duration_ms, 0.0);
    }

    // ─── 엣지 케이스: 매우 큰 응답 시간 (u64 범위) ───────────

    #[test]
    fn test_very_large_duration() {
        // res_time - req_time = i64::MAX에 가까운 값
        let entries = vec![make_info(
            "GET",
            "http://example.com/slow",
            200,
            0,
            1_000_000_000, // 10^9 ms = ~11.5일
        )];
        let report = TrafficAnalytics::slow_requests(&entries, 1000, 10);
        assert_eq!(report.total_slow, 1);
        assert_eq!(report.requests[0].duration_ms, 1_000_000_000);
    }

    // ─── 엣지 케이스: 모든 요청이 에러인 경우 ───────────────

    #[test]
    fn test_all_requests_are_errors() {
        let entries = vec![
            make_info("GET", "http://example.com/a", 500, 1000, 1100),
            make_info("GET", "http://example.com/b", 502, 2000, 2100),
            make_info("POST", "http://example.com/c", 404, 3000, 3100),
        ];
        let report = TrafficAnalytics::error_analysis(&entries);
        assert_eq!(report.total_requests, 3);
        assert_eq!(report.error_count, 3);
        assert!((report.error_rate - 100.0).abs() < f64::EPSILON);
        assert_eq!(report.by_status.len(), 3);
    }

    // ─── 엣지 케이스: 에러가 하나도 없는 경우 ───────────────

    #[test]
    fn test_no_errors() {
        let entries = vec![
            make_info("GET", "http://example.com/a", 200, 1000, 1100),
            make_info("GET", "http://example.com/b", 201, 2000, 2100),
            make_info("GET", "http://example.com/c", 304, 3000, 3100),
        ];
        let report = TrafficAnalytics::error_analysis(&entries);
        assert_eq!(report.total_requests, 3);
        assert_eq!(report.error_count, 0);
        assert_eq!(report.error_rate, 0.0);
        assert!(report.by_status.is_empty());
        assert!(report.by_endpoint.is_empty());
        assert!(report.recent_errors.is_empty());
    }

    // ─── 엣지 케이스: 동일한 타임스탬프의 요청들 ─────────────

    #[test]
    fn test_same_timestamp_requests() {
        let entries = vec![
            make_info("GET", "http://example.com/a", 200, 1000, 1100),
            make_info("GET", "http://example.com/b", 200, 1000, 1100),
            make_info("GET", "http://example.com/c", 200, 1000, 1100),
        ];
        let timeline = TrafficAnalytics::traffic_timeline(&entries, 60);
        // 모두 같은 버킷에 들어가야 함
        assert_eq!(timeline.len(), 1);
        assert_eq!(timeline[0].request_count, 3);
    }

    // ─── 엣지 케이스: URL에 쿼리 파라미터가 있는 중복 감지 ──

    #[test]
    fn test_duplicate_with_query_params() {
        // 쿼리 파라미터가 다르면 다른 URL로 취급됨 (현재 구현 기준)
        let entries = vec![
            make_info("GET", "http://example.com/api?page=1", 200, 1000, 1100),
            make_info("GET", "http://example.com/api?page=1", 200, 1500, 1600),
            make_info("GET", "http://example.com/api?page=2", 200, 2000, 2100),
        ];
        let dupes = TrafficAnalytics::duplicate_requests(&entries, 3000);
        // ?page=1 이 2번이므로 1개 그룹
        assert_eq!(dupes.len(), 1);
        assert_eq!(dupes[0].count, 2);
        assert!(dupes[0].url.contains("page=1"));
    }

    // ─── 엣지 케이스: N+1 패턴에서 UUID ID ──────────────────

    #[test]
    fn test_n_plus_one_with_uuid_ids() {
        let entries = vec![
            make_info(
                "GET",
                "http://example.com/api/users/550e8400-e29b-41d4-a716-446655440000",
                200,
                1000,
                1100,
            ),
            make_info(
                "GET",
                "http://example.com/api/users/550e8400-e29b-41d4-a716-446655440001",
                200,
                1200,
                1300,
            ),
            make_info(
                "GET",
                "http://example.com/api/users/550e8400-e29b-41d4-a716-446655440002",
                200,
                1400,
                1500,
            ),
        ];
        let patterns = TrafficAnalytics::n_plus_one_detection(&entries);
        assert!(!patterns.is_empty());
        assert!(patterns[0].pattern.contains("{id}"));
        assert_eq!(patterns[0].count, 3);
    }

    // ─── 엣지 케이스: N+1 패턴에서 중첩 경로 ────────────────

    #[test]
    fn test_n_plus_one_nested_paths() {
        let entries = vec![
            make_info(
                "GET",
                "http://example.com/api/users/1/posts/100",
                200,
                1000,
                1100,
            ),
            make_info(
                "GET",
                "http://example.com/api/users/2/posts/200",
                200,
                1200,
                1300,
            ),
            make_info(
                "GET",
                "http://example.com/api/users/3/posts/300",
                200,
                1400,
                1500,
            ),
        ];
        let patterns = TrafficAnalytics::n_plus_one_detection(&entries);
        assert!(!patterns.is_empty());
        // /api/users/{id}/posts/{id} 패턴이 감지되어야 함
        assert!(patterns[0].pattern.contains("{id}"));
        assert_eq!(patterns[0].count, 3);
    }

    // ─── 엣지 케이스: request/response가 None인 항목 ─────────

    #[test]
    fn test_entries_with_none_request() {
        let entries = vec![RequestInfo {
            request: None,
            response: Some(make_client_response(200, 1100)),
            validations: None,
            server_cert: None,
        }];
        // 모든 분석 함수가 패닉 없이 빈 결과를 반환해야 함
        let slow = TrafficAnalytics::slow_requests(&entries, 1000, 10);
        assert_eq!(slow.total_slow, 0);
        let errors = TrafficAnalytics::error_analysis(&entries);
        assert_eq!(errors.total_requests, 0);
        let stats = TrafficAnalytics::endpoint_stats(&entries);
        assert!(stats.is_empty());
        let dupes = TrafficAnalytics::duplicate_requests(&entries, 3000);
        assert!(dupes.is_empty());
        let patterns = TrafficAnalytics::n_plus_one_detection(&entries);
        assert!(patterns.is_empty());
        let domain = TrafficAnalytics::domain_breakdown(&entries);
        assert!(domain.is_empty());
        let payload = TrafficAnalytics::payload_size_analysis(&entries);
        assert_eq!(payload.total_requests, 0);
    }

    #[test]
    fn test_entries_with_none_response() {
        let entries = vec![RequestInfo {
            request: Some(make_client_request("GET", "http://example.com/a", 1000)),
            response: None,
            validations: None,
            server_cert: None,
        }];
        // slow_requests: response가 없으면 duration 계산 불가 -> 스킵
        let slow = TrafficAnalytics::slow_requests(&entries, 0, 10);
        assert_eq!(slow.total_slow, 0);
        // error_analysis: response가 없으면 스킵
        let errors = TrafficAnalytics::error_analysis(&entries);
        assert_eq!(errors.total_requests, 0);
        // endpoint_stats: response가 없으면 스킵
        let stats = TrafficAnalytics::endpoint_stats(&entries);
        assert!(stats.is_empty());
    }

    // ─── 엣지 케이스: total_slow가 truncate 전 값인지 확인 ──

    #[test]
    fn test_slow_requests_total_slow_before_truncate() {
        let mut entries = Vec::new();
        for i in 0..20 {
            entries.push(make_info(
                "GET",
                "http://example.com/slow",
                200,
                i * 1000,
                i * 1000 + 2000, // 2000ms duration > 500ms threshold
            ));
        }
        let report = TrafficAnalytics::slow_requests(&entries, 500, 5);
        // total_slow는 limit 적용 전의 개수여야 함
        assert_eq!(report.total_slow, 20);
        // requests는 limit으로 잘려야 함
        assert_eq!(report.requests.len(), 5);
    }

    // ─── 엣지 케이스: 퍼센타일 경계값 ──────────────────────

    #[test]
    fn test_percentile_two_values() {
        let values = vec![10, 20];
        // P50: round(0.5 * 1) = round(0.5) = 1 -> values[1] = 20
        // (round rounds .5 to nearest even = 0, but Rust's f64::round rounds .5 away from zero = 1)
        let p50 = percentile(&values, 50.0);
        assert!(p50 == 10.0 || p50 == 20.0); // 구현에 따라 다를 수 있음
        assert_eq!(percentile(&values, 0.0), 10.0);
        assert_eq!(percentile(&values, 100.0), 20.0);
    }

    #[test]
    fn test_percentile_p99_large_dataset() {
        let values: Vec<u64> = (1..=100).collect();
        let p99 = percentile(&values, 99.0);
        // idx = round(0.99 * 99) = round(98.01) = 98 -> values[98] = 99
        assert_eq!(p99, 99.0);
    }

    // ─── 헬퍼 함수 테스트 ───────────────────────────────────

    #[test]
    fn test_extract_path_with_query_params() {
        assert_eq!(
            extract_path("http://example.com/api/users?page=1&limit=10"),
            "/api/users"
        );
        assert_eq!(extract_path("http://example.com/"), "/");
        assert_eq!(extract_path("http://example.com"), "/");
        assert_eq!(extract_path("/api/users?q=test"), "/api/users");
        assert_eq!(extract_path("/api/users"), "/api/users");
    }

    #[test]
    fn test_extract_domain_various() {
        assert_eq!(extract_domain("http://example.com/path"), "example.com");
        assert_eq!(
            extract_domain("https://example.com:8443/path"),
            "example.com"
        );
        assert_eq!(extract_domain("http://example.com"), "example.com");
        assert_eq!(extract_domain("no-scheme"), "unknown");
    }

    #[test]
    fn test_extract_scheme() {
        assert_eq!(extract_scheme("http://example.com"), "http");
        assert_eq!(extract_scheme("https://example.com"), "https");
        assert_eq!(extract_scheme("no-scheme"), "unknown");
    }

    #[test]
    fn test_is_error_status_boundary() {
        assert!(!is_error_status(399));
        assert!(is_error_status(400));
        assert!(is_error_status(401));
        assert!(is_error_status(500));
        assert!(is_error_status(503));
    }

    #[test]
    fn test_normalize_path_pattern_various() {
        // 순수 숫자
        assert_eq!(normalize_path_pattern("/api/users/42"), "/api/users/{id}");
        // 짧은 UUID (32자 미만)
        assert_eq!(
            normalize_path_pattern("/api/users/abc123"),
            "/api/users/abc123"
        );
        // 경로 없음
        assert_eq!(normalize_path_pattern("/"), "/");
        // 여러 ID 세그먼트
        assert_eq!(
            normalize_path_pattern("/api/users/1/posts/2"),
            "/api/users/{id}/posts/{id}"
        );
        // 빈 세그먼트는 건드리지 않음
        assert_eq!(normalize_path_pattern("/api//users"), "/api//users");
    }

    // ─── 통합 시나리오: 실제적인 API 트래픽 시뮬레이션 ──────

    #[test]
    fn test_realistic_api_traffic_simulation() {
        let mut entries = Vec::new();
        let base_time = 1_700_000_000_000i64; // 임의의 Unix ms 타임스탬프

        // 정상 GET 요청 (빠름)
        for i in 0..50 {
            entries.push(make_info(
                "GET",
                "http://api.example.com/api/users",
                200,
                base_time + i * 100,
                base_time + i * 100 + 50, // 50ms
            ));
        }
        // 느린 GET 요청
        for i in 0..5 {
            entries.push(make_info(
                "GET",
                "http://api.example.com/api/reports",
                200,
                base_time + 5000 + i * 200,
                base_time + 5000 + i * 200 + 3000, // 3000ms
            ));
        }
        // 에러 요청
        for i in 0..10 {
            entries.push(make_info(
                "POST",
                "http://api.example.com/api/orders",
                500,
                base_time + 10000 + i * 100,
                base_time + 10000 + i * 100 + 100,
            ));
        }
        // 404 요청
        for i in 0..3 {
            entries.push(make_info(
                "GET",
                "http://api.example.com/api/not-found",
                404,
                base_time + 15000 + i * 100,
                base_time + 15000 + i * 100 + 20,
            ));
        }

        let report = TrafficAnalytics::full_report(&entries);

        // 검증
        assert_eq!(report.errors.total_requests, 68); // 50 + 5 + 10 + 3
        assert_eq!(report.errors.error_count, 13); // 10 + 3
        assert!(report.errors.error_rate > 15.0 && report.errors.error_rate < 25.0);

        // 느린 요청 검출
        assert!(report.slow_requests.total_slow > 0);

        // 도메인은 1개
        assert_eq!(report.domain_breakdown.len(), 1);
        assert_eq!(report.domain_breakdown[0].domain, "api.example.com");
    }

    // ─── 통합 시나리오: 대량 요청 정확성 ─────────────────────

    #[test]
    fn test_large_volume_analysis_accuracy() {
        let mut entries = Vec::new();
        for i in 0..1000 {
            let status = if i % 10 == 0 { 500 } else { 200 };
            entries.push(make_info(
                "GET",
                "http://example.com/api/data",
                status,
                i * 10,
                i * 10 + (i % 50) + 1, // 1~50ms 다양한 duration
            ));
        }

        let report = TrafficAnalytics::error_analysis(&entries);
        assert_eq!(report.total_requests, 1000);
        assert_eq!(report.error_count, 100); // 1000 / 10
        assert!((report.error_rate - 10.0).abs() < 0.01);

        let slow = TrafficAnalytics::slow_requests(&entries, 0, 1000);
        assert_eq!(slow.total_slow, 1000); // 모든 요청이 0ms 이상
        assert!(slow.p50_ms >= 0.0);
        assert!(slow.p95_ms >= slow.p50_ms);
        assert!(slow.p99_ms >= slow.p95_ms);
    }

    // ─── 통합 시나리오: endpoint_stats 정렬 옵션별 검증 ──────

    #[test]
    fn test_endpoint_stats_sorting() {
        let entries = vec![
            // /fast: 빠르지만 많이 호출
            make_info("GET", "http://example.com/fast", 200, 1000, 1010),
            make_info("GET", "http://example.com/fast", 200, 2000, 2010),
            make_info("GET", "http://example.com/fast", 200, 3000, 3010),
            // /slow: 느리지만 적게 호출
            make_info("GET", "http://example.com/slow", 200, 4000, 5000),
            // /error: 에러율 높음
            make_info("GET", "http://example.com/error", 500, 6000, 6100),
            make_info("GET", "http://example.com/error", 200, 7000, 7100),
        ];

        let mut stats = TrafficAnalytics::endpoint_stats(&entries);
        // 기본 정렬: count 내림차순
        assert_eq!(stats[0].path, "/fast");
        assert_eq!(stats[0].count, 3);

        // duration 정렬
        stats.sort_by(|a, b| {
            b.avg_duration_ms
                .partial_cmp(&a.avg_duration_ms)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        assert_eq!(stats[0].path, "/slow");

        // error_rate 정렬
        stats.sort_by(|a, b| {
            b.error_rate
                .partial_cmp(&a.error_rate)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        assert_eq!(stats[0].path, "/error");
        assert!((stats[0].error_rate - 50.0).abs() < 0.01);
    }

    // ─── 통합 시나리오: traffic_timeline 버킷 경계값 ─────────

    #[test]
    fn test_traffic_timeline_bucket_boundaries() {
        let bucket_seconds = 10u64; // 10초 = 10000ms 버킷
        let bucket_ms = (bucket_seconds * 1000) as i64;

        let entries = vec![
            // 버킷 0: [0, 10000)
            make_info("GET", "http://example.com/a", 200, 0, 100),
            make_info("GET", "http://example.com/b", 200, 9999, 10050),
            // 버킷 1: [10000, 20000)
            make_info("GET", "http://example.com/c", 500, 10000, 10100),
            make_info("GET", "http://example.com/d", 200, 15000, 15100),
            // 버킷 2: [20000, 30000)
            make_info("GET", "http://example.com/e", 200, 20000, 20100),
        ];

        let timeline = TrafficAnalytics::traffic_timeline(&entries, bucket_seconds);
        assert_eq!(timeline.len(), 3);

        // 첫 번째 버킷
        assert_eq!(timeline[0].timestamp, 0);
        assert_eq!(timeline[0].request_count, 2);
        assert_eq!(timeline[0].error_count, 0);

        // 두 번째 버킷
        assert_eq!(timeline[1].timestamp, bucket_ms);
        assert_eq!(timeline[1].request_count, 2);
        assert_eq!(timeline[1].error_count, 1);

        // 세 번째 버킷
        assert_eq!(timeline[2].timestamp, bucket_ms * 2);
        assert_eq!(timeline[2].request_count, 1);
    }

    // ─── 통합 시나리오: CORS preflight + 실제 요청 쌍 감지 ──

    #[test]
    fn test_cors_preflight_and_actual_request_pair() {
        // preflight OPTIONS (CORS 헤더 없는 응답)
        let preflight_req_headers = {
            let mut h = HeaderMap::new();
            h.insert("origin", HeaderValue::from_static("http://localhost:3000"));
            h
        };
        let preflight = make_info_with_headers(
            "OPTIONS",
            "http://api.example.com/data",
            200,
            1000,
            1050,
            preflight_req_headers,
            HeaderMap::new(), // CORS 헤더 없음
        );

        // 실제 요청 (CORS 헤더 없는 응답)
        let actual_req_headers = {
            let mut h = HeaderMap::new();
            h.insert("origin", HeaderValue::from_static("http://localhost:3000"));
            h
        };
        let actual = make_info_with_headers(
            "POST",
            "http://api.example.com/data",
            200,
            1100,
            1200,
            actual_req_headers,
            HeaderMap::new(), // CORS 헤더 없음
        );

        let entries = vec![preflight, actual];
        let issues = TrafficAnalytics::cors_issues(&entries);

        // preflight: missing_allow_origin + missing_allow_methods
        // actual: missing_allow_origin
        assert!(issues.len() >= 3);

        let preflight_issues: Vec<_> = issues
            .iter()
            .filter(|i| {
                i.issue_type == "preflight_missing_allow_origin"
                    || i.issue_type == "preflight_missing_allow_methods"
            })
            .collect();
        assert_eq!(preflight_issues.len(), 2);

        let actual_issues: Vec<_> = issues
            .iter()
            .filter(|i| i.issue_type == "missing_allow_origin")
            .collect();
        assert_eq!(actual_issues.len(), 1);
    }

    // ─── 통합 시나리오: Mixed Content 감지 (Referer 헤더) ────

    #[test]
    fn test_mixed_content_various_resource_types() {
        let entries = vec![
            {
                let req_h = {
                    let mut h = HeaderMap::new();
                    h.insert(
                        "referer",
                        HeaderValue::from_static("https://secure.example.com/page"),
                    );
                    h
                };
                let res_h = {
                    let mut h = HeaderMap::new();
                    h.insert("content-type", HeaderValue::from_static("text/css"));
                    h
                };
                make_info_with_headers(
                    "GET",
                    "http://insecure.example.com/style.css",
                    200,
                    1000,
                    1100,
                    req_h,
                    res_h,
                )
            },
            {
                let req_h = {
                    let mut h = HeaderMap::new();
                    h.insert(
                        "referer",
                        HeaderValue::from_static("https://secure.example.com/page"),
                    );
                    h
                };
                let res_h = {
                    let mut h = HeaderMap::new();
                    h.insert(
                        "content-type",
                        HeaderValue::from_static("application/javascript"),
                    );
                    h
                };
                make_info_with_headers(
                    "GET",
                    "http://insecure.example.com/app.js",
                    200,
                    2000,
                    2100,
                    req_h,
                    res_h,
                )
            },
            {
                let req_h = {
                    let mut h = HeaderMap::new();
                    h.insert(
                        "referer",
                        HeaderValue::from_static("https://secure.example.com/page"),
                    );
                    h
                };
                let res_h = {
                    let mut h = HeaderMap::new();
                    h.insert("content-type", HeaderValue::from_static("font/woff2"));
                    h
                };
                make_info_with_headers(
                    "GET",
                    "http://insecure.example.com/font.woff2",
                    200,
                    3000,
                    3100,
                    req_h,
                    res_h,
                )
            },
        ];

        let warnings = TrafficAnalytics::mixed_content_warnings(&entries);
        assert_eq!(warnings.len(), 3);

        let types: Vec<&str> = warnings.iter().map(|w| w.resource_type.as_str()).collect();
        assert!(types.contains(&"stylesheet"));
        assert!(types.contains(&"script"));
        assert!(types.contains(&"font"));
    }

    // ─── 통합 시나리오: HTTPS 요청은 Mixed Content가 아님 ────

    #[test]
    fn test_no_mixed_content_for_https_resources() {
        let req_headers = {
            let mut h = HeaderMap::new();
            h.insert(
                "referer",
                HeaderValue::from_static("https://secure.example.com/page"),
            );
            h
        };
        let entries = vec![make_info_with_headers(
            "GET",
            "https://also-secure.example.com/data",
            200,
            1000,
            1100,
            req_headers,
            HeaderMap::new(),
        )];
        let warnings = TrafficAnalytics::mixed_content_warnings(&entries);
        assert!(warnings.is_empty());
    }

    // ─── 통합 시나리오: 중복 요청 - 윈도우 밖 요청 ───────────

    #[test]
    fn test_duplicate_requests_outside_window() {
        let entries = vec![
            make_info("GET", "http://example.com/api/data", 200, 1000, 1100),
            make_info("GET", "http://example.com/api/data", 200, 5000, 5100),
        ];
        // window 1000ms -> 두 요청 사이 간격 4000ms이므로 중복 아님
        let dupes = TrafficAnalytics::duplicate_requests(&entries, 1000);
        assert!(dupes.is_empty());
    }

    // ─── 통합 시나리오: 단일 요청은 중복이 아님 ──────────────

    #[test]
    fn test_single_request_not_duplicate() {
        let entries = vec![make_info(
            "GET",
            "http://example.com/api/data",
            200,
            1000,
            1100,
        )];
        let dupes = TrafficAnalytics::duplicate_requests(&entries, 3000);
        assert!(dupes.is_empty());
    }

    // ─── 통합 시나리오: N+1 패턴 - 2개는 감지 안됨 ──────────

    #[test]
    fn test_n_plus_one_two_requests_not_detected() {
        let entries = vec![
            make_info("GET", "http://example.com/api/users/1", 200, 1000, 1100),
            make_info("GET", "http://example.com/api/users/2", 200, 1200, 1300),
        ];
        let patterns = TrafficAnalytics::n_plus_one_detection(&entries);
        // 3개 미만이므로 감지되지 않음
        assert!(patterns.is_empty());
    }

    // ─── 통합 시나리오: 페이로드 크기 분석 with body ─────────

    #[test]
    fn test_payload_size_analysis_with_bodies() {
        let entries = vec![
            make_info_with_body(
                "POST",
                "http://example.com/a",
                200,
                1000,
                1100,
                b"req1",
                b"response1",
            ),
            make_info_with_body(
                "POST",
                "http://example.com/b",
                200,
                2000,
                2100,
                b"request2",
                b"r2",
            ),
        ];
        let report = TrafficAnalytics::payload_size_analysis(&entries);
        assert_eq!(report.total_requests, 2);
        // 최대 요청 크기: "request2" = 8 bytes
        assert_eq!(report.max_request_size, 8);
        // 최대 응답 크기: "response1" = 9 bytes
        assert_eq!(report.max_response_size, 9);
    }

    // ─── 통합 시나리오: CORS - origin 없는 요청은 무시 ───────

    #[test]
    fn test_cors_no_origin_header_ignored() {
        let entries = vec![make_info(
            "GET",
            "http://api.example.com/data",
            200,
            1000,
            1100,
        )];
        let issues = TrafficAnalytics::cors_issues(&entries);
        // origin 헤더 없으면 CORS 검사 안 함
        assert!(issues.is_empty());
    }

    // ─── 통합 시나리오: CORS preflight 에러 상태 감지 ────────

    #[test]
    fn test_cors_preflight_error_status() {
        let req_headers = {
            let mut h = HeaderMap::new();
            h.insert("origin", HeaderValue::from_static("http://localhost:3000"));
            h
        };
        let entries = vec![make_info_with_headers(
            "OPTIONS",
            "http://api.example.com/data",
            403,
            1000,
            1050,
            req_headers,
            HeaderMap::new(),
        )];
        let issues = TrafficAnalytics::cors_issues(&entries);
        let failed: Vec<_> = issues
            .iter()
            .filter(|i| i.issue_type == "preflight_failed")
            .collect();
        assert_eq!(failed.len(), 1);
        assert!(failed[0].details.contains("403"));
    }

    // ─── 통합 시나리오: 도메인 통계 - 여러 포트 ──────────────

    #[test]
    fn test_domain_breakdown_with_ports() {
        let entries = vec![
            make_info("GET", "http://example.com:8080/a", 200, 1000, 1100),
            make_info("GET", "http://example.com:8080/b", 200, 2000, 2100),
            make_info("GET", "http://example.com:9090/c", 200, 3000, 3100),
        ];
        let stats = TrafficAnalytics::domain_breakdown(&entries);
        // 포트가 다르면 같은 도메인으로 추출됨 (extract_domain은 포트 제거)
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].domain, "example.com");
        assert_eq!(stats[0].request_count, 3);
    }

    // ─── 통합 시나리오: 대량 요청에 대한 타임라인 ────────────

    #[test]
    fn test_traffic_timeline_large_volume() {
        let mut entries = Vec::new();
        // 100초 동안 매 100ms마다 요청 (1000건)
        for i in 0..1000 {
            let status = if i % 20 == 0 { 500 } else { 200 };
            entries.push(make_info(
                "GET",
                "http://example.com/api/data",
                status,
                i * 100,      // 0, 100, 200, ...
                i * 100 + 50, // 50ms duration
            ));
        }

        let timeline = TrafficAnalytics::traffic_timeline(&entries, 10); // 10초 버킷
                                                                         // 100초 / 10초 = 10개 버킷
        assert_eq!(timeline.len(), 10);

        // 각 버킷에 100개 요청
        for bucket in &timeline {
            assert_eq!(bucket.request_count, 100);
            // 5% 에러율 (20건 중 1건)
            assert_eq!(bucket.error_count, 5);
        }
    }

    // ─── 통합 시나리오: extract_path 엣지케이스 ──────────────

    #[test]
    fn test_extract_path_edge_cases() {
        // fragment
        assert_eq!(extract_path("http://example.com/path?q=1"), "/path");
        // 빈 경로
        assert_eq!(extract_path("http://example.com"), "/");
        // 여러 쿼리 파라미터
        assert_eq!(
            extract_path("http://example.com/api/v1/users?page=1&size=20&sort=name"),
            "/api/v1/users"
        );
    }
}

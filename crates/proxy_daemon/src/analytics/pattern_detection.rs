//! 패턴 감지 — 중복 요청 감지 및 N+1 쿼리 패턴 감지

use std::collections::HashMap;

use proxy_v2_models::RequestInfo;

use super::{
    extract_domain, extract_path, normalize_path_pattern, DuplicateGroup, NPlus1Pattern,
    TrafficAnalytics,
};

impl TrafficAnalytics {
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
}

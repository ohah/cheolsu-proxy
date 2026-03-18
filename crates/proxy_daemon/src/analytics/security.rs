//! 보안 분석 — CORS 문제 감지 및 Mixed Content 경고

use proxy_v2_models::RequestInfo;

use super::{extract_scheme, is_error_status, CorsIssue, MixedContentWarning, TrafficAnalytics};

impl TrafficAnalytics {
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
}

/// 인터셉트 액션 구현 (Block, ModifyRequest, ModifyResponse, Rewrite, MapLocal, MapRemote, Throttle)
use super::helpers::{apply_header_modifications, apply_rewrite_action};
use crate::handler::response_helpers;
use crate::handler::LoggingHandler;
use crate::protocol::{InterceptAction, InterceptRule, RewriteTarget, ServerReplayEntry};
use bytes::Bytes;
use proxyapi_v2::hyper::Request;
use proxyapi_v2::throttle::ThrottleConfig;
use proxyapi_v2::{
    hyper::{
        http::{HeaderValue, StatusCode},
        Response,
    },
    Body, RequestOrResponse,
};
use tracing::{error, info};

impl LoggingHandler {
    /// 서버 리플레이 매칭: method + URL이 일치하는 엔트리 검색
    pub(crate) async fn find_server_replay_match(
        &self,
        url: &str,
        method: &str,
    ) -> Option<ServerReplayEntry> {
        let entries = self.intercept.server_replay_entries.read().await;
        entries
            .iter()
            .find(|entry| entry.method.eq_ignore_ascii_case(method) && entry.url == url)
            .cloned()
    }

    /// URL과 메서드가 인터셉트 규칙에 매칭되는지 확인
    pub(crate) fn rule_matches(rule: &InterceptRule, url: &str, method: &str) -> bool {
        if !rule.enabled {
            return false;
        }
        if let Some(rule_method) = &rule.method {
            if rule_method.to_uppercase() != method.to_uppercase() {
                return false;
            }
        }
        crate::pattern_utils::wildcard_matches(&rule.pattern, url)
    }

    /// 파일 확장자로 Content-Type 추론
    pub(crate) fn guess_content_type(file_path: &str) -> String {
        let ext = std::path::Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        match ext.as_str() {
            "json" => "application/json",
            "html" | "htm" => "text/html; charset=utf-8",
            "xml" => "application/xml",
            "js" | "mjs" => "application/javascript",
            "css" => "text/css",
            "txt" => "text/plain; charset=utf-8",
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "svg" => "image/svg+xml",
            "webp" => "image/webp",
            "woff" | "woff2" => "font/woff2",
            "pdf" => "application/pdf",
            _ => "application/octet-stream",
        }
        .to_string()
    }

    /// 매칭되는 인터셉트 규칙 검색
    pub(crate) async fn find_matching_intercept_rules(
        &self,
        url: &str,
        method: &str,
    ) -> Vec<InterceptRule> {
        let rules_guard = self.intercept.intercept_rules.read().await;
        rules_guard
            .iter()
            .filter(|rule| Self::rule_matches(rule, url, method))
            .cloned()
            .collect()
    }

    /// Block 액션 처리 → 즉시 응답 반환
    pub(crate) fn apply_block(
        status_code: u16,
        body: &str,
        method: &str,
        url: &str,
        rule_name: &str,
    ) -> Response<Body> {
        info!(
            "[Intercept] 요청 차단: {} {} -> {} (규칙: {})",
            method, url, status_code, rule_name
        );
        let mut response = response_helpers::error_response(
            StatusCode::from_u16(status_code).unwrap_or(StatusCode::FORBIDDEN),
            Body::from(body.to_string()),
        );
        if !body.is_empty() {
            let content_type = if body.starts_with('{') || body.starts_with('[') {
                "application/json"
            } else {
                "text/plain; charset=utf-8"
            };
            response
                .headers_mut()
                .insert("content-type", HeaderValue::from_static(content_type));
        }
        response
    }

    /// MapLocal 액션 처리 → 로컬 파일 응답 반환
    pub(crate) fn apply_map_local(
        file_path: &str,
        status_code: u16,
        headers: &std::collections::HashMap<String, String>,
        method: &str,
        url: &str,
        rule_name: &str,
    ) -> Response<Body> {
        info!(
            "[Intercept] Map Local: {} {} -> {} (규칙: {})",
            method, url, file_path, rule_name
        );
        let file_bytes = match std::fs::read(file_path) {
            Ok(bytes) => Bytes::from(bytes),
            Err(e) => {
                error!(
                    "[Intercept] Map Local 파일 읽기 실패: {} - {}",
                    file_path, e
                );
                return response_helpers::build_response(
                    Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .header("x-cheolsu-map-local-error", e.to_string()),
                    Body::from(format!("Map Local Error: file not found - {}", file_path)),
                );
            }
        };

        let mut response = Response::builder()
            .status(StatusCode::from_u16(status_code).unwrap_or(StatusCode::OK))
            .header("x-cheolsu-map-local", file_path)
            .header("content-length", file_bytes.len().to_string());

        let content_type = headers
            .get("content-type")
            .cloned()
            .unwrap_or_else(|| Self::guess_content_type(file_path));
        response = response.header("content-type", content_type);

        for (name, value) in headers {
            if name.to_lowercase() != "content-type" {
                response = response.header(name.as_str(), value.as_str());
            }
        }

        response_helpers::build_response(
            response,
            Body::from(http_body_util::Full::new(file_bytes)),
        )
    }

    /// MapRemote 액션 처리 → 요청 URI를 변경
    pub(crate) fn apply_map_remote(
        req: &mut Request<Body>,
        target_url: &str,
        preserve_path: bool,
        url: &str,
        method: &str,
        rule_name: &str,
    ) {
        info!(
            "[Intercept] Map Remote: {} {} -> {} (규칙: {}, preserve_path={})",
            method, url, target_url, rule_name, preserve_path
        );

        let new_url = if preserve_path {
            url.parse::<proxyapi_v2::hyper::Uri>()
                .ok()
                .and_then(|original| {
                    let path_and_query = original
                        .path_and_query()
                        .map(|pq| pq.as_str())
                        .unwrap_or("/");
                    Some(format!(
                        "{}{}",
                        target_url.trim_end_matches('/'),
                        path_and_query
                    ))
                })
                .unwrap_or_else(|| target_url.to_string())
        } else {
            target_url.to_string()
        };

        let Ok(new_uri) = new_url.parse::<proxyapi_v2::hyper::Uri>() else {
            error!("[Intercept] Map Remote URL 파싱 실패: {}", new_url);
            return;
        };

        // 파싱된 URI에서 host를 먼저 추출한 뒤 uri_mut에 설정 (이중 파싱 방지)
        let host_str = new_uri.host().map(|h| h.to_string());
        *req.uri_mut() = new_uri;

        if let Some(host) = host_str {
            if let Ok(host_value) = host.parse::<HeaderValue>() {
                req.headers_mut()
                    .insert(proxyapi_v2::hyper::header::HOST, host_value);
            }
        }

        req.headers_mut().insert(
            "x-cheolsu-map-remote-original",
            url.parse()
                .unwrap_or_else(|_| HeaderValue::from_static("unknown")),
        );
    }

    /// 인터셉트 규칙에 따라 요청을 차단하거나 수정
    pub(crate) async fn apply_request_intercept(
        &self,
        req: Request<Body>,
        url: &str,
        method: &str,
    ) -> RequestOrResponse {
        let rules = self.find_matching_intercept_rules(url, method).await;

        let mut current_req = req;

        for rule in &rules {
            match &rule.action {
                InterceptAction::Block { status_code, body } => {
                    return Self::apply_block(*status_code, body, method, url, &rule.name).into();
                }
                InterceptAction::ModifyRequest {
                    add_headers,
                    remove_headers,
                    set_body,
                } => {
                    info!(
                        "[Intercept] 요청 수정: {} {} (규칙: {})",
                        method, url, rule.name
                    );
                    apply_header_modifications(
                        current_req.headers_mut(),
                        remove_headers,
                        add_headers,
                    );
                    if let Some(new_body) = set_body {
                        use http_body_util::Full;
                        *current_req.body_mut() =
                            Body::from(Full::new(bytes::Bytes::from(new_body.clone())));
                    }
                }
                InterceptAction::ModifyResponse { .. } => {
                    // 응답 수정 규칙은 handle_response에서 처리
                }
                InterceptAction::Rewrite {
                    target,
                    match_pattern,
                    replace_with,
                } if *target == RewriteTarget::RequestHeader
                    || *target == RewriteTarget::RequestBody =>
                {
                    apply_rewrite_action!(
                        current_req,
                        match_pattern,
                        replace_with,
                        *target == RewriteTarget::RequestHeader,
                        "요청",
                        method,
                        url,
                        &rule.name
                    );
                }
                InterceptAction::Rewrite { .. } => {
                    // response 대상 rewrite는 apply_response_intercept에서 처리
                }
                InterceptAction::MapLocal {
                    file_path,
                    status_code,
                    headers,
                } => {
                    return Self::apply_map_local(
                        file_path,
                        *status_code,
                        headers,
                        method,
                        url,
                        &rule.name,
                    )
                    .into();
                }
                InterceptAction::MapRemote {
                    target_url,
                    preserve_path,
                } => {
                    Self::apply_map_remote(
                        &mut current_req,
                        target_url,
                        *preserve_path,
                        url,
                        method,
                        &rule.name,
                    );
                }
                InterceptAction::Throttle {
                    latency_ms,
                    download_rate,
                    upload_rate,
                } => {
                    info!(
                        "[Intercept] Throttle 적용: {} {} (규칙: {}, latency={}ms, dl={:?}, ul={:?})",
                        method, url, rule.name, latency_ms, download_rate, upload_rate
                    );
                    if *latency_ms > 0 {
                        tokio::time::sleep(std::time::Duration::from_millis(*latency_ms)).await;
                    }
                }
            }
        }

        current_req.into()
    }

    /// 인터셉트 규칙에 따라 응답을 수정
    pub(crate) async fn apply_response_intercept(
        &self,
        mut res: Response<Body>,
        url: &str,
        method: &str,
    ) -> Response<Body> {
        let rules = self.find_matching_intercept_rules(url, method).await;

        for rule in &rules {
            if let InterceptAction::ModifyResponse {
                set_status,
                add_headers,
                remove_headers,
                set_body,
            } = &rule.action
            {
                info!(
                    "[Intercept] 응답 수정: {} {} (규칙: {})",
                    method, url, rule.name
                );

                // 상태 코드 변경
                if let Some(status) = set_status {
                    if let Ok(status_code) = StatusCode::from_u16(*status) {
                        *res.status_mut() = status_code;
                    }
                }

                apply_header_modifications(res.headers_mut(), remove_headers, add_headers);

                // 바디 변경
                if let Some(new_body) = set_body {
                    use http_body_util::Full;
                    // Content-Length 업데이트
                    let body_bytes = bytes::Bytes::from(new_body.clone());
                    crate::header_utils::clear_content_encoding_headers(res.headers_mut());
                    *res.body_mut() = Body::from(Full::new(body_bytes));
                }
            }

            if let InterceptAction::Rewrite {
                target,
                match_pattern,
                replace_with,
            } = &rule.action
            {
                if *target == RewriteTarget::ResponseHeader
                    || *target == RewriteTarget::ResponseBody
                {
                    apply_rewrite_action!(
                        res,
                        match_pattern,
                        replace_with,
                        *target == RewriteTarget::ResponseHeader,
                        "응답",
                        method,
                        url,
                        &rule.name
                    );
                }
            }
        }

        res
    }

    /// Throttle 규칙에 매칭되는 경우 ThrottleConfig를 반환합니다.
    pub(crate) async fn find_response_throttle_config(
        &self,
        url: &str,
        method: &str,
    ) -> Option<ThrottleConfig> {
        let rules = self.find_matching_intercept_rules(url, method).await;
        for rule in &rules {
            if let InterceptAction::Throttle {
                download_rate,
                upload_rate,
                latency_ms,
            } = &rule.action
            {
                if download_rate.is_some() || upload_rate.is_some() {
                    return Some(ThrottleConfig {
                        enabled: true,
                        download_rate: *download_rate,
                        upload_rate: *upload_rate,
                        latency_ms: *latency_ms,
                    });
                }
            }
        }
        None
    }

    /// MapRemote 규칙의 대상 포트가 CONNECT authority와 매칭되는지 확인합니다.
    ///
    /// MapRemote 규칙이 `*app.example.dev*` → `http://localhost:8083` 형태일 때,
    /// dev server의 HMR 스크립트가 `app.example.dev:8083`으로 CONNECT를 시도할 수 있습니다.
    /// 이 경우 SSL 인터셉트를 활성화하여 MapRemote가 정상 동작하도록 합니다.
    pub(crate) async fn should_intercept_for_map_remote(
        &self,
        host: &str,
        port: Option<u16>,
    ) -> bool {
        let rules = self.intercept.intercept_rules.read().await;
        map_remote_needs_ssl_intercept(&rules, host, port)
    }

    /// 응답 body에 속도 제한을 적용합니다.
    pub(crate) fn apply_response_throttle(
        res: Response<Body>,
        config: &ThrottleConfig,
    ) -> Response<Body> {
        let (parts, body) = res.into_parts();

        let rate = match config.download_rate {
            Some(r) if r > 0 => r,
            _ => return Response::from_parts(parts, body),
        };

        // body를 Frame 스트림으로 변환하고, 각 청크마다 rate에 맞춰 지연
        let throttled_stream = async_stream::stream! {
            let mut body = body;
            let mut tokens: f64 = rate as f64; // 시작 시 1초분 토큰
            let mut last_refill = std::time::Instant::now();

            loop {
                use http_body_util::BodyExt;
                match body.frame().await {
                    Some(Ok(frame)) => {
                        if let Some(data) = frame.data_ref() {
                            let chunk_size = data.len() as f64;

                            // 토큰 리필
                            let now = std::time::Instant::now();
                            let elapsed = now.duration_since(last_refill).as_secs_f64();
                            last_refill = now;
                            tokens = (tokens + elapsed * rate as f64).min(rate as f64);

                            // 토큰이 부족하면 대기
                            if tokens < chunk_size {
                                let wait_secs = (chunk_size - tokens) / rate as f64;
                                tokio::time::sleep(std::time::Duration::from_secs_f64(wait_secs)).await;
                                tokens = 0.0;
                            } else {
                                tokens -= chunk_size;
                            }

                            yield Ok::<Bytes, proxyapi_v2::Error>(data.clone());
                        }
                    }
                    Some(Err(e)) => {
                        yield Err(e);
                        break;
                    }
                    None => break,
                }
            }
        };

        let throttled_body = Body::from_stream(throttled_stream);
        Response::from_parts(parts, throttled_body)
    }
}

/// MapRemote 규칙 목록에서 CONNECT host:port 조합이 인터셉트 대상인지 판단합니다.
///
/// MapRemote의 target_url 포트가 CONNECT 포트와 일치하고,
/// CONNECT 호스트가 해당 규칙의 소스 패턴과 매칭되면 `true`를 반환합니다.
pub(crate) fn map_remote_needs_ssl_intercept(
    rules: &[InterceptRule],
    host: &str,
    port: Option<u16>,
) -> bool {
    for rule in rules {
        if !rule.enabled {
            continue;
        }
        if let InterceptAction::MapRemote { target_url, .. } = &rule.action {
            // target_url에서 포트 추출 (예: "http://localhost:8083" → 8083)
            let target_port = target_url
                .parse::<proxyapi_v2::hyper::Uri>()
                .ok()
                .and_then(|uri| uri.port_u16());

            // target_url에 비표준 포트가 있고, CONNECT 포트와 일치하며,
            // CONNECT 호스트가 규칙의 소스 패턴과 매칭되는 경우
            if let (Some(t_port), Some(c_port)) = (target_port, port) {
                if t_port == c_port {
                    // 호스트가 소스 패턴의 도메인과 매칭되는지 확인
                    // CONNECT URI는 "host:port" 형태이므로 https:// scheme을 붙여서 전체 URL 형태로 매칭
                    let synthetic_url = format!("https://{}:{}/", host, c_port);
                    if crate::pattern_utils::wildcard_matches(&rule.pattern, &synthetic_url) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

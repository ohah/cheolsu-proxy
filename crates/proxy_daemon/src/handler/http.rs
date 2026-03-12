use bytes::Bytes;
use proxy_v2_models::{ProxiedRequest, ProxiedResponse, RequestInfo};
use proxyapi_v2::{
    hyper::http::{Method, StatusCode},
    hyper::{Request, Response},
    Body, HttpContext, HttpHandler, RequestOrResponse,
};
use regex::Regex;
use std::error::Error;
use tracing::{debug, error, info};

use crate::cert_distribution;

use super::LoggingHandler;

impl LoggingHandler {
    pub(super) fn serve_ca_cert_download(&self, req: &Request<Body>) -> Response<Body> {
        cert_distribution::handle_cert_request(req, self.config.ca_cert_der.as_ref())
    }

    /// No Caching / Block Cookies / No Gzip 설정을 요청에 적용
    pub(crate) async fn apply_quick_settings_on_request(
        &self,
        mut req: Request<Body>,
    ) -> Request<Body> {
        use proxyapi_v2::hyper::header::{
            ACCEPT_ENCODING, CACHE_CONTROL, COOKIE, IF_MODIFIED_SINCE, IF_NONE_MATCH, PRAGMA,
        };

        let settings = { *self.config.quick_settings.read().await };

        if settings.no_caching {
            req.headers_mut().remove(IF_MODIFIED_SINCE);
            req.headers_mut().remove(IF_NONE_MATCH);
            req.headers_mut().insert(
                CACHE_CONTROL,
                "no-cache, no-store, must-revalidate".parse().unwrap(),
            );
            req.headers_mut()
                .insert(PRAGMA, "no-cache".parse().unwrap());
        }

        if settings.block_cookies {
            req.headers_mut().remove(COOKIE);
        }

        if settings.no_gzip {
            req.headers_mut().remove(ACCEPT_ENCODING);
        }

        req
    }

    /// Block Cookies 설정을 응답에 적용 (Set-Cookie 제거)
    pub(crate) async fn apply_quick_settings_on_response(
        &self,
        mut res: Response<Body>,
    ) -> Response<Body> {
        use proxyapi_v2::hyper::header::SET_COOKIE;

        let settings = { *self.config.quick_settings.read().await };

        if settings.block_cookies {
            res.headers_mut().remove(SET_COOKIE);
        }

        res
    }

    /// 요청과 응답을 묶어서 전송
    pub(crate) async fn send_output(&self) {
        let client_request = self
            .request
            .req
            .as_ref()
            .map(|req| req.clone().for_client(self.config.cache_dir.as_deref()));
        let client_response = self.request.res.as_ref().map(|res| {
            let request_id = self
                .request
                .req
                .as_ref()
                .map(|r| r.id().clone())
                .unwrap_or_default();
            res.clone()
                .for_client(&request_id, self.config.cache_dir.as_deref())
        });
        let request_info = RequestInfo(client_request, client_response);
        if let Err(e) = self.sender.send(request_info).await {
            error!("[LoggingHandler] 이벤트 전송 실패: {}", e);
        }
    }

    /// Request를 ProxiedRequest로 변환하고 원본 요청을 복원
    async fn request_to_proxied_request(
        &self,
        mut req: Request<Body>,
    ) -> (ProxiedRequest, Request<Body>) {
        let mut body_mut = req.body_mut();
        let body_bytes = match Self::body_to_bytes_from_mut(&mut body_mut).await {
            Ok(bytes) => bytes,
            Err(_) => Bytes::new(),
        };

        use http_body_util::Full;
        *body_mut = Body::from(Full::new(body_bytes.clone()));

        let proxied_request = ProxiedRequest::new(
            req.method().clone(),
            req.uri().clone(),
            req.version(),
            req.headers().clone(),
            body_bytes.clone(),
            chrono::Local::now()
                .timestamp_nanos_opt()
                .unwrap_or_default(),
        );

        (proxied_request, req)
    }

    /// Response를 ProxiedResponse로 변환하고 원본 응답을 복원
    async fn response_to_proxied_response(
        &self,
        mut res: Response<Body>,
    ) -> (ProxiedResponse, Response<Body>) {
        let mut body_mut = res.body_mut();
        let body_bytes = match Self::body_to_bytes_from_mut(&mut body_mut).await {
            Ok(bytes) => bytes,
            Err(_) => Bytes::new(),
        };

        use http_body_util::Full;
        *body_mut = Body::from(Full::new(body_bytes.clone()));

        let proxied_response = ProxiedResponse::new(
            res.status(),
            res.version(),
            res.headers().clone(),
            body_bytes.clone(),
            chrono::Local::now()
                .timestamp_nanos_opt()
                .unwrap_or_default(),
        );

        (proxied_response, res)
    }

    /// BodyMut를 Bytes로 변환하는 헬퍼 함수
    async fn body_to_bytes_from_mut(
        body_mut: &mut Body,
    ) -> Result<Bytes, Box<dyn std::error::Error + Send + Sync>> {
        use http_body_util::BodyExt;
        let body_bytes = body_mut.collect().await?.to_bytes();
        Ok(body_bytes)
    }

    fn create_response_from_cached_data(&self) -> Response<Body> {
        if let Some(cached_response) = &self.request.res {
            let mut response = Response::builder()
                .status(*cached_response.status())
                .version(*cached_response.version());

            for (key, value) in cached_response.headers() {
                response = response.header(key, value);
            }

            use http_body_util::Full;
            response
                .body(Body::from(Full::new(cached_response.body().clone())))
                .unwrap_or_else(|_| Response::new(Body::empty()))
        } else {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("No cached response data available"))
                .unwrap_or_else(|_| Response::new(Body::empty()))
        }
    }

    fn extract_tls_info_from_error(
        &self,
        err: &hyper_util::client::legacy::Error,
    ) -> Option<(String, String)> {
        let err_str = err.to_string();

        let tls_version = if err_str.contains("TLS 1.0") || err_str.contains("TLS/1.0") {
            "TLS 1.0"
        } else if err_str.contains("TLS 1.1") || err_str.contains("TLS/1.1") {
            "TLS 1.1"
        } else if err_str.contains("TLS 1.2") || err_str.contains("TLS/1.2") {
            "TLS 1.2"
        } else if err_str.contains("TLS 1.3") || err_str.contains("TLS/1.3") {
            "TLS 1.3"
        } else if err_str.contains("SSL 3.0") || err_str.contains("SSL/3.0") {
            "SSL 3.0"
        } else if err_str.contains("handshake") || err_str.contains("TLS") {
            "TLS (버전 미확인)"
        } else {
            "알 수 없음"
        };

        let tls_backend = if err_str.contains("[RUSTLS]") || err_str.contains("rustls handshake") {
            "RUSTLS"
        } else if err_str.contains("[NATIVE-TLS]")
            || err_str.contains("native-tls handshake")
            || err_str.contains("PKCS12")
        {
            "NATIVE-TLS"
        } else if err_str.contains("rustls") || err_str.contains("RUSTLS") {
            "RUSTLS"
        } else if err_str.contains("native-tls")
            || err_str.contains("NATIVE-TLS")
            || err_str.contains("OpenSSL")
        {
            "NATIVE-TLS"
        } else if err_str.contains("handshake") || err_str.contains("TLS") {
            "TLS (백엔드 미확인)"
        } else {
            "알 수 없음"
        };

        Some((tls_version.to_string(), tls_backend.to_string()))
    }

    fn extract_target_server_from_error(
        &self,
        err: &hyper_util::client::legacy::Error,
    ) -> Option<String> {
        let err_str = err.to_string();

        if let Some(start) = err_str.find(" - ") {
            if let Some(end) = err_str[start + 3..].find(" - 오류:") {
                let server_info = &err_str[start + 3..start + 3 + end];
                if !server_info.is_empty() && server_info.contains(':') {
                    return Some(server_info.trim().to_string());
                }
            }
        }

        let patterns = [
            (r"([a-zA-Z0-9.-]+\.[a-zA-Z]{2,}:\d+)", "도메인:포트"),
            (r"(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}:\d+)", "IP:포트"),
            (r"([a-zA-Z0-9.-]+\.[a-zA-Z]{2,})", "도메인"),
        ];

        for (pattern, _description) in &patterns {
            if let Some(server_info) = Self::extract_pattern(&err_str, pattern) {
                if !server_info.is_empty() {
                    return Some(server_info);
                }
            }
        }

        if let Some(host) = Self::extract_host_from_url(&err_str) {
            return Some(host);
        }

        None
    }

    fn extract_pattern(text: &str, pattern: &str) -> Option<String> {
        if let Ok(regex) = Regex::new(pattern) {
            if let Some(captures) = regex.captures(text) {
                if let Some(matched) = captures.get(1) {
                    return Some(matched.as_str().to_string());
                }
            }
        }
        None
    }

    fn extract_host_from_url(text: &str) -> Option<String> {
        let url_patterns = ["https://", "http://"];

        for pattern in &url_patterns {
            if let Some(start) = text.find(pattern) {
                let after_protocol = &text[start + pattern.len()..];
                if let Some(end) = after_protocol.find('/') {
                    let host_part = &after_protocol[..end];
                    if !host_part.is_empty() && (host_part.contains('.') || host_part.contains(':'))
                    {
                        return Some(host_part.to_string());
                    }
                } else if !after_protocol.is_empty()
                    && (after_protocol.contains('.') || after_protocol.contains(':'))
                {
                    return Some(after_protocol.to_string());
                }
            }
        }

        None
    }

    /// Apply host mapping to the request if a matching rule exists.
    /// Rewrites the URI to point to the mapped target host/port,
    /// while preserving the original Host header for correct virtual host routing.
    async fn apply_host_mapping_if_needed(&self, mut req: Request<Body>) -> Request<Body> {
        let (host, port) = Self::extract_host_port(req.uri());
        let Some(host) = host else {
            return req;
        };

        if let Some((target_host, target_port)) = self.resolve_host_mapping(&host, port).await {
            info!(
                "[HostMapping] {}:{} -> {}:{}",
                host,
                port.map(|p| p.to_string())
                    .unwrap_or_else(|| "default".to_string()),
                target_host,
                target_port
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "default".to_string()),
            );

            if let Some(new_uri) =
                Self::apply_host_mapping_to_uri(req.uri(), &target_host, target_port)
            {
                *req.uri_mut() = new_uri;
                // Keep the original Host header intact so the server
                // can route to the correct virtual host.
                //
                // x-cheolsu-host-mapped: 요청 디버깅/로깅 전용 마커 헤더.
                // 호스트 매핑이 적용되었음을 프록시 내부에서 추적하기 위한 용도이며,
                // 실제 서버로 전송됩니다. 서버 측에서 이 헤더가 문제가 될 경우
                // 향후 요청 전송 직전에 제거하는 옵션을 추가할 수 있습니다.
                req.headers_mut().insert(
                    "x-cheolsu-host-mapped",
                    proxyapi_v2::hyper::http::HeaderValue::from_static("true"),
                );
            }
        }

        req
    }

    pub(super) fn check_cert_download_intercept(
        &self,
        req: &Request<Body>,
    ) -> Option<Response<Body>> {
        if cert_distribution::is_cert_download_request(req) {
            Some(self.serve_ca_cert_download(req))
        } else {
            None
        }
    }

    /// 서버 리플레이 매칭을 확인하고, 매칭되면 응답을 생성합니다.
    async fn check_server_replay(&self, url: &str, method: &str) -> Option<Response<Body>> {
        let entry = self.find_server_replay_match(url, method).await?;
        info!(
            "[ServerReplay] 매칭: {} {} -> status {} (id: {})",
            method, url, entry.status, entry.id
        );
        let mut response = Response::builder()
            .status(StatusCode::from_u16(entry.status).unwrap_or(StatusCode::OK))
            .header("x-cheolsu-server-replay", "true")
            .header("x-cheolsu-server-replay-id", &entry.id);

        for (name, value) in &entry.headers {
            response = response.header(name.as_str(), value.as_str());
        }

        let body_bytes = entry.body.unwrap_or_default();
        Some(
            response
                .body(Body::from(body_bytes))
                .unwrap_or_else(|_| Response::new(Body::empty())),
        )
    }

    /// 스크립트 on_request 훅을 적용합니다.
    /// Respond이면 Err(Response) 반환, Forward/Modify이면 Ok(Request) 반환.
    async fn apply_script_on_request(
        &self,
        req: Request<Body>,
        proxied_request: &ProxiedRequest,
        method: &str,
        url: &str,
    ) -> Result<Request<Body>, Response<Body>> {
        let script_req = Self::to_script_request(proxied_request);
        match self
            .intercept
            .script_handle
            .invoke_on_request(&script_req)
            .await
        {
            Ok(scripting::RequestAction::Forward) => Ok(req),
            Ok(scripting::RequestAction::ModifyRequest { request: modified }) => {
                info!("[Script] 요청 수정: {} {}", method, url);
                Ok(Self::apply_script_request_modify(req, &modified))
            }
            Ok(scripting::RequestAction::Respond { response }) => {
                info!(
                    "[Script] 요청 차단: {} {} -> {}",
                    method, url, response.status
                );
                Err(Self::build_script_response(&response))
            }
            Err(e) => {
                error!("[Script] onRequest 오류: {}", e);
                Ok(req)
            }
        }
    }

    /// 인터셉트 규칙으로 응답을 수정합니다.
    async fn apply_response_intercept_if_needed(&self, res: Response<Body>) -> Response<Body> {
        if let Some(req) = &self.request.req {
            let url = req.uri().to_string();
            let method = req.method().to_string();
            self.apply_response_intercept(res, &url, &method).await
        } else {
            res
        }
    }

    /// 스크립트 on_response 훅을 적용합니다.
    async fn apply_script_on_response(&self, res: Response<Body>) -> Response<Body> {
        let Some(req) = &self.request.req else {
            return res;
        };
        let script_req = Self::to_script_request(req);
        let script_res = Self::to_script_response_from_hyper(&res);
        match self
            .intercept
            .script_handle
            .invoke_on_response(&script_req, &script_res)
            .await
        {
            Ok(scripting::ResponseAction::Forward) => res,
            Ok(scripting::ResponseAction::ModifyResponse { response: modified }) => {
                info!("[Script] 응답 수정: {}", req.uri());
                Self::apply_script_response_modify(res, &modified)
            }
            Err(e) => {
                error!("[Script] onResponse 오류: {}", e);
                res
            }
        }
    }
}

impl HttpHandler for LoggingHandler {
    async fn should_intercept(&mut self, _ctx: &HttpContext, req: &Request<Body>) -> bool {
        // 프록시 인증 체크: 인증 실패 여부를 먼저 판정
        let auth_failed = {
            let auth_config = self.config.proxy_auth.read().await;
            if let Some(config) = auth_config.as_ref() {
                if config.enabled && !config.username.is_empty() {
                    let auth_header = req
                        .headers()
                        .get("proxy-authorization")
                        .and_then(|v| v.to_str().ok());
                    !config.validate_proxy_auth(auth_header)
                } else {
                    false
                }
            } else {
                false
            }
        };

        // 인증 실패 시 반드시 인터셉트하여 handle_request에서 407 응답 반환
        // TLS Passthrough 경로로 빠지면 인증 없이 터널이 수립되므로 여기서 차단 필수
        if auth_failed {
            info!(
                "[ProxyAuth] CONNECT 인증 실패, 터널 수립 거부: {:?}",
                req.uri().authority().map(|a| a.to_string())
            );
            return true;
        }

        // CONNECT 요청의 URI에서 authority(host:port)를 추출
        if let Some(authority) = req.uri().authority() {
            let host = authority.host();
            let port = authority.port_u16();

            let mode = self.intercept.ssl_proxying_mode.read().await;
            let entries = self.intercept.ssl_proxying_entries.read().await;
            let result = crate::ssl_proxying::should_intercept_ssl(&mode, &entries, host, port);

            if !result {
                debug!("[SSLProxying] TLS Passthrough 적용: {}", authority);
            }

            result
        } else {
            true
        }
    }

    async fn handle_request(
        &mut self,
        _ctx: &HttpContext,
        mut req: Request<Body>,
    ) -> RequestOrResponse {
        // 프록시 인증 확인
        if let Some(auth_response) = self.check_proxy_auth(&req).await {
            return auth_response.into();
        }
        // 인증 통과 후 Proxy-Authorization 헤더 제거 (upstream에 전달 방지)
        req.headers_mut().remove("proxy-authorization");

        // 요청 바디 크기 제한 확인 (Content-Length 기반)
        // NOTE: Content-Length 헤더 기반 검사만 수행하므로, chunked transfer-encoding을 사용하는
        // 요청은 Content-Length가 없어 이 검사를 우회할 수 있습니다.
        // 완전한 제한이 필요하면 바디 스트림을 소비하며 누적 크기를 체크하는 방식이 필요합니다.
        if let Some(max_size) = self.config.max_body_size {
            if let Some(content_length) = req
                .headers()
                .get(proxyapi_v2::hyper::header::CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<usize>().ok())
            {
                if content_length > max_size {
                    info!(
                        "[BodyLimit] 요청 바디 크기 초과: {} > {} ({})",
                        content_length,
                        max_size,
                        req.uri()
                    );
                    let response = Response::builder()
                        .status(StatusCode::PAYLOAD_TOO_LARGE)
                        .body(Body::from(format!(
                            "Request body too large: {} bytes (max: {} bytes)",
                            content_length, max_size
                        )))
                        .unwrap_or_else(|_| Response::new(Body::empty()));
                    return response.into();
                }
            }
        }

        if let Some(cert_response) = self.check_cert_download_intercept(&req) {
            return cert_response.into();
        }

        if req
            .headers()
            .get(proxyapi_v2::hyper::header::UPGRADE)
            .and_then(|v| v.to_str().ok())
            .map_or(false, |s| s.to_lowercase() == "websocket")
        {
            req.headers_mut()
                .remove(proxyapi_v2::hyper::header::SEC_WEBSOCKET_EXTENSIONS);
        }

        let (proxied_request, restored_req) = self.request_to_proxied_request(req).await;

        if restored_req.method() == Method::CONNECT || proxied_request.method() == "CONNECT" {
            // CONNECT 터널 요청을 UI에서 볼 수 있도록 로깅
            self.request.req = Some(proxied_request.clone());
            self.send_output().await;
            return restored_req.into();
        }

        self.request.req = Some(proxied_request.clone());

        let url = proxied_request.uri().to_string();
        let method = proxied_request.method().to_string();

        if let Some(replay_response) = self.check_server_replay(&url, &method).await {
            self.send_output().await;
            return replay_response.into();
        }

        let restored_req = match self
            .apply_script_on_request(restored_req, &proxied_request, &method, &url)
            .await
        {
            Ok(req) => req,
            Err(response) => {
                self.send_output().await;
                return response.into();
            }
        };

        let transaction_id = self
            .request
            .req
            .as_ref()
            .map(|r| r.id().clone())
            .unwrap_or_default();

        let restored_req = match self
            .apply_request_breakpoint(restored_req, &url, &method, &transaction_id)
            .await
        {
            Ok(req) => req,
            Err(response) => {
                self.send_output().await;
                return response.into();
            }
        };

        let restored_req = self.apply_host_mapping_if_needed(restored_req).await;

        let restored_req = self.apply_quick_settings_on_request(restored_req).await;

        let result = self
            .apply_request_intercept(restored_req, &url, &method)
            .await;

        if let RequestOrResponse::Response(_) = &result {
            self.send_output().await;
        }

        result
    }

    async fn handle_response(&mut self, _ctx: &HttpContext, res: Response<Body>) -> Response<Body> {
        if res.status() == StatusCode::SWITCHING_PROTOCOLS {
            return res;
        }

        let res = self.apply_response_intercept_if_needed(res).await;
        let res = self.apply_quick_settings_on_response(res).await;
        let res = self.apply_script_on_response(res).await;

        let res = if let Some(req) = &self.request.req {
            let url = req.uri().to_string();
            let method = req.method().to_string();
            let transaction_id = req.id().clone();
            self.apply_response_breakpoint(res, &url, &method, &transaction_id)
                .await
        } else {
            res
        };

        let is_sse = res
            .headers()
            .get(proxyapi_v2::hyper::header::CONTENT_TYPE)
            .map_or(false, |v| {
                v.to_str().unwrap_or("").contains("text/event-stream")
            });

        if is_sse {
            return self.handle_sse_streaming(res);
        }

        let (proxied_response, restored_res) = self.response_to_proxied_response(res).await;
        self.request.res = Some(proxied_response);
        self.send_output().await;
        restored_res
    }

    async fn handle_error(
        &mut self,
        _ctx: &HttpContext,
        err: hyper_util::client::legacy::Error,
    ) -> Response<Body> {
        let tls_info = self.extract_tls_info_from_error(&err);
        let target_server = self.extract_target_server_from_error(&err);

        if let Some(source) = err.source() {
            let source_str = source.to_string();
            if source_str.contains("UnexpectedEof") || source_str.contains("unexpected EOF") {
                debug!(
                    error = %err,
                    target = ?target_server,
                    "TLS close_notify 없이 연결 종료됨 - 정상 종료로 처리"
                );

                if self.request.res.is_some() {
                    return self.create_response_from_cached_data();
                } else {
                    return Response::builder()
                        .status(StatusCode::OK)
                        .body(Body::empty())
                        .unwrap_or_else(|_| Response::new(Body::empty()));
                }
            }
        }

        error!(
            error = %err,
            target = ?target_server,
            tls_info = ?tls_info,
            source = ?err.source().map(|s| s.to_string()),
            "프록시 요청 오류"
        );

        let should_use_curl = err
            .source()
            .map(|s| s.to_string().contains("HandshakeFailure"))
            .unwrap_or(false);

        if should_use_curl {
            if let Some(req) = &self.request.req {
                error!("TLS 핸드셰이크 실패 - curl 폴백 시도");
                match crate::curl_fallback::fallback_with_curl(req).await {
                    Ok(response) => {
                        info!("curl 폴백 성공");
                        return response;
                    }
                    Err(curl_err) => {
                        error!(error = %curl_err, "curl 폴백도 실패");
                    }
                }
            }
        }

        Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .body(Body::from(format!("Proxy Error: {}", err)))
            .unwrap_or_else(|_| Response::new(Body::empty()))
    }
}

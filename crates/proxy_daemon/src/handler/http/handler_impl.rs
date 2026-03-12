use proxyapi_v2::{
    hyper::http::{Method, StatusCode},
    hyper::{Request, Response},
    Body, HttpContext, HttpHandler, RequestOrResponse,
};
use std::error::Error;
use tracing::{debug, error, info};

use super::super::LoggingHandler;

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

        // 요청 바디 크기 제한 확인 (Content-Length 헤더 + body size_hint 기반)
        if let Some(max_size) = self.config.max_body_size {
            // 1) Content-Length 헤더 기반 검사
            let content_length = req
                .headers()
                .get(proxyapi_v2::hyper::header::CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<usize>().ok());

            // 2) body size_hint의 lower bound도 확인 (chunked 전송 시 Content-Length 없어도 감지 가능)
            let body_lower_bound = {
                use proxyapi_v2::hyper::body::Body as HttpBody;
                req.body().size_hint().lower() as usize
            };

            let effective_size = content_length.unwrap_or(body_lower_bound);
            if effective_size > max_size {
                info!(
                    "[BodyLimit] 요청 바디 크기 초과: {} > {} ({})",
                    effective_size,
                    max_size,
                    req.uri()
                );
                let response = Response::builder()
                    .status(StatusCode::PAYLOAD_TOO_LARGE)
                    .body(Body::from(format!(
                        "Request body too large: {} bytes (max: {} bytes)",
                        effective_size, max_size
                    )))
                    .unwrap_or_else(|_| Response::new(Body::empty()));
                return response.into();
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

        // 응답 바디 크기 제한: 업스트림이 과도하게 큰 응답을 보낼 때 OOM 방지
        if let Some(max_size) = self.config.max_body_size {
            let response_size = res
                .headers()
                .get(proxyapi_v2::hyper::header::CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or_else(|| {
                    use proxyapi_v2::hyper::body::Body as HttpBody;
                    res.body().size_hint().lower() as usize
                });
            if response_size > max_size {
                info!(
                    "[BodyLimit] 응답 바디 크기 초과: {} > {} — 바디를 잘라 반환",
                    response_size, max_size
                );
                return Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .body(Body::from(format!(
                        "Response body too large: {} bytes (max: {} bytes)",
                        response_size, max_size
                    )))
                    .unwrap_or_else(|_| Response::new(Body::empty()));
            }
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

use proxyapi_v2::{
    hyper::http::StatusCode,
    hyper::{Request, Response},
    Body, HttpContext, HttpHandler, RequestOrResponse,
};
use std::error::Error;
use tracing::{debug, error, info};

use crate::handler::response_helpers;

use super::super::LoggingHandler;

impl HttpHandler for LoggingHandler {
    async fn should_intercept(&mut self, _ctx: &HttpContext, req: &Request<Body>) -> bool {
        // 프록시 인증 체크: 인증 실패 여부를 먼저 판정
        let auth_failed = {
            let auth_config = self.config.proxy_auth.read().await;
            if let Some(config) = auth_config.as_ref() {
                if !config.enabled {
                    false
                } else if config.method == crate::protocol::AuthMethod::Basic
                    && config.username.is_empty()
                {
                    false
                } else {
                    let auth_value = req
                        .headers()
                        .get("proxy-authorization")
                        .and_then(|v| v.to_str().ok());
                    !config.validate_proxy_auth(auth_value)
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

            let ssl = self.intercept.ssl_proxying.read().await;
            let result = crate::ssl_proxying::should_intercept_ssl(
                &ssl.mode,
                &ssl.entries,
                &ssl.default_passthrough,
                host,
                port,
            );

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
        ctx: &HttpContext,
        mut req: Request<Body>,
    ) -> RequestOrResponse {
        use super::request_pipeline::PipelineAction;

        // 프록시 인증 사용자명 추출 (Basic 인증 시)
        if let Some(auth_header) = req
            .headers()
            .get("proxy-authorization")
            .and_then(|v| v.to_str().ok())
        {
            if let Some(encoded) = auth_header.strip_prefix("Basic ") {
                if let Ok(decoded) =
                    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded)
                {
                    if let Ok(credentials) = std::str::from_utf8(&decoded) {
                        if let Some((username, _)) = credentials.split_once(':') {
                            self.request.proxy_auth_user = Some(username.to_string());
                        }
                    }
                }
            }
        }

        // 초기 파이프라인: 인증, 바디 크기 제한, 인증서 다운로드, WebSocket 확장 제거
        if let Some(action) = self.run_early_pipeline(&mut req).await {
            return match action {
                PipelineAction::Respond(response) | PipelineAction::RespondWithOutput(response) => {
                    response.into()
                }
                PipelineAction::Continue(_) => unreachable!(),
            };
        }

        // Waterfall 타이밍 시작
        self.request.request_start = Some(std::time::Instant::now());

        let (mut proxied_request, restored_req) = self.request_to_proxied_request(req).await;
        proxied_request.set_client_addr(ctx.client_addr.to_string());

        // CONNECT 터널 요청: UI에서 볼 수 있도록 로깅 후 원본 요청 반환
        // clone 대신 move: 이후 proxied_request를 사용하지 않으므로 소유권 이전
        if Self::is_connect_tunnel(&proxied_request, &restored_req) {
            self.request.req = Some(proxied_request);
            self.send_output().await;
            return restored_req.into();
        }

        // url, method를 먼저 추출한 뒤 proxied_request를 move하여 불필요한 clone 제거
        let url = proxied_request.uri().to_string();
        let method = proxied_request.method().to_string();
        self.request.req = Some(proxied_request);

        if let Some(replay_response) = self.check_server_replay(&url, &method).await {
            self.send_output().await;
            return replay_response.into();
        }

        // self.request.req에서 참조로 접근하여 clone 제거
        let restored_req = match self
            .apply_script_on_request(
                restored_req,
                self.request.req.as_ref().unwrap(),
                &method,
                &url,
            )
            .await
        {
            Ok(req) => req,
            Err(response) => {
                self.send_output().await;
                return response.into();
            }
        };

        // id()는 &String을 반환 — self에 대한 공유 참조로 직접 전달하여 clone 제거
        let transaction_id = self
            .request
            .req
            .as_ref()
            .map(|r| r.id().as_str())
            .unwrap_or_default();

        let restored_req = match self
            .apply_request_breakpoint(restored_req, &url, &method, transaction_id)
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

        // 응답 헤더 수신 시각 기록 (TTFB 계산용)
        self.request.response_header_time = Some(std::time::Instant::now());

        // 응답 바디 크기 제한 확인
        if let Some(error_response) = self.check_response_body_size_limit(&res) {
            return error_response;
        }

        let res = self.apply_response_intercept_if_needed(res).await;
        let res = self.apply_quick_settings_on_response(res).await;
        let res = self.apply_script_on_response(res).await;

        let res = if let Some(req) = &self.request.req {
            let url = req.uri().to_string();
            let method = req.method().to_string();
            // id()는 &String을 반환 — 공유 참조로 직접 전달하여 clone 제거
            self.apply_response_breakpoint(res, &url, &method, req.id())
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

        let (proxied_response, mut restored_res) = self.response_to_proxied_response(res).await;
        self.request.res = Some(proxied_response);
        self.send_output().await;

        // 조건부 Throttle: 응답 body 스트리밍에 속도 제한 적용
        if let Some(req) = &self.request.req {
            let url = req.uri().to_string();
            let method = req.method().to_string();
            if let Some(config) = self.find_response_throttle_config(&url, &method).await {
                tracing::info!(
                    "[Intercept] 응답 Throttle 적용: {} {} (dl={:?})",
                    method,
                    url,
                    config.download_rate
                );
                restored_res = Self::apply_response_throttle(restored_res, &config);
            }
        }

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
                    return response_helpers::empty_response(StatusCode::OK);
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

        response_helpers::error_response(
            StatusCode::BAD_GATEWAY,
            Body::from(format!("Proxy Error: {}", err)),
        )
    }
}

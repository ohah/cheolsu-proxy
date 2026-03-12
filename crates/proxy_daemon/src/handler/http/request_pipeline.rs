use proxy_v2_models::ProxiedRequest;
use proxyapi_v2::{
    hyper::http::{Method, StatusCode},
    hyper::{Request, Response},
    Body,
};
use tracing::{error, info};

use crate::cert_distribution;
use crate::handler::response_helpers;

use super::super::LoggingHandler;

impl LoggingHandler {
    pub(in crate::handler) fn serve_ca_cert_download(&self, req: &Request<Body>) -> Response<Body> {
        cert_distribution::handle_cert_request(req, self.config.ca_cert_der.as_ref())
    }

    /// 요청 바디 크기 제한 확인 (Content-Length 헤더 + body size_hint 기반).
    /// 크기 초과 시 413 Payload Too Large 응답을 반환합니다.
    pub(super) fn check_request_body_size_limit(
        &self,
        req: &Request<Body>,
    ) -> Option<Response<Body>> {
        let max_size = self.config.max_body_size?;

        // 1) Content-Length 헤더 기반 검사
        let content_length = req
            .headers()
            .get(proxyapi_v2::hyper::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<usize>().ok());

        // 2) body size_hint의 lower bound 확인
        // NOTE: chunked 전송 시 hyper의 Incoming body는 lower=0, upper=None을 반환하므로
        // Content-Length 없는 chunked 요청은 이 검사를 우회할 수 있음.
        // 완전한 스트리밍 제한이 필요하면 body wrapper로 누적 크기를 체크해야 함.
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
            return Some(response);
        }

        None
    }

    /// 응답 바디 크기 제한 확인.
    /// 업스트림이 과도하게 큰 응답을 보낼 때 OOM 방지를 위해 사용합니다.
    /// NOTE: Content-Length가 없는 chunked 응답은 size_hint lower=0이므로 이 검사를 우회함.
    /// Content-Length가 있는 대용량 응답(파일 다운로드 등)만 차단됨.
    pub(super) fn check_response_body_size_limit(
        &self,
        res: &Response<Body>,
    ) -> Option<Response<Body>> {
        let max_size = self.config.max_body_size?;

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
            return Some(
                Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .body(Body::from(format!(
                        "Response body too large: {} bytes (max: {} bytes)",
                        response_size, max_size
                    )))
                    .unwrap_or_else(|_| Response::new(Body::empty())),
            );
        }

        None
    }

    /// WebSocket 업그레이드 요청일 경우 SEC_WEBSOCKET_EXTENSIONS 헤더를 제거합니다.
    /// 프록시 환경에서 압축 확장(permessage-deflate 등)이 호환성 문제를 일으킬 수 있으므로
    /// 확장 협상을 방지합니다.
    pub(super) fn strip_websocket_extensions(req: &mut Request<Body>) {
        let is_websocket = req
            .headers()
            .get(proxyapi_v2::hyper::header::UPGRADE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|s| s.to_lowercase() == "websocket");

        if is_websocket {
            req.headers_mut()
                .remove(proxyapi_v2::hyper::header::SEC_WEBSOCKET_EXTENSIONS);
        }
    }

    /// CONNECT 터널 요청인지 확인합니다.
    pub(super) fn is_connect_tunnel(
        proxied_request: &ProxiedRequest,
        restored_req: &Request<Body>,
    ) -> bool {
        // 두 소스 모두 확인: restored_req는 복원된 hyper 요청, proxied_request는
        // 원본 프록시 요청 모델. 둘 다 Method 타입이므로 Method::CONNECT로 통일.
        restored_req.method() == Method::CONNECT || proxied_request.method() == Method::CONNECT
    }

    /// Apply host mapping to the request if a matching rule exists.
    /// Rewrites the URI to point to the mapped target host/port,
    /// while preserving the original Host header for correct virtual host routing.
    pub(super) async fn apply_host_mapping_if_needed(
        &self,
        mut req: Request<Body>,
    ) -> Request<Body> {
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

    pub(in crate::handler) fn check_cert_download_intercept(
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
    pub(super) async fn check_server_replay(
        &self,
        url: &str,
        method: &str,
    ) -> Option<Response<Body>> {
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
        Some(response_helpers::build_response(
            response,
            Body::from(body_bytes),
        ))
    }

    /// 스크립트 on_request 훅을 적용합니다.
    /// Respond이면 Err(Response) 반환, Forward/Modify이면 Ok(Request) 반환.
    pub(super) async fn apply_script_on_request(
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
    pub(super) async fn apply_response_intercept_if_needed(
        &self,
        res: Response<Body>,
    ) -> Response<Body> {
        if let Some(req) = &self.request.req {
            let url = req.uri().to_string();
            let method = req.method().to_string();
            self.apply_response_intercept(res, &url, &method).await
        } else {
            res
        }
    }

    /// 스크립트 on_response 훅을 적용합니다.
    pub(super) async fn apply_script_on_response(&self, res: Response<Body>) -> Response<Body> {
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

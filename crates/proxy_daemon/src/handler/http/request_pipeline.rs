use proxy_v2_models::ProxiedRequest;
use proxyapi_v2::{
    hyper::http::StatusCode,
    hyper::{Request, Response},
    Body,
};
use tracing::{error, info};

use crate::cert_distribution;

use super::super::LoggingHandler;

impl LoggingHandler {
    pub(in crate::handler) fn serve_ca_cert_download(&self, req: &Request<Body>) -> Response<Body> {
        cert_distribution::handle_cert_request(req, self.config.ca_cert_der.as_ref())
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
        Some(
            response
                .body(Body::from(body_bytes))
                .unwrap_or_else(|_| Response::new(Body::empty())),
        )
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

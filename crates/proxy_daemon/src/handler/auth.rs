use proxyapi_v2::{
    hyper::http::StatusCode,
    hyper::{Request, Response},
    Body,
};
use tracing::info;

use super::response_helpers;
use super::LoggingHandler;
use crate::protocol::AuthMethod;

impl LoggingHandler {
    /// 프록시 인증 설정 업데이트
    pub async fn update_proxy_auth(&self, config: crate::protocol::ProxyAuthConfig) {
        let mut auth = self.config.proxy_auth.write().await;
        info!(
            "[ProxyAuth] 설정 업데이트: enabled={}, method={:?}, username={}",
            config.enabled, config.method, config.username
        );
        *auth = Some(config);
    }

    /// 인증 방식에 따라 요청에서 적절한 헤더 값을 추출합니다.
    fn extract_auth_value<'a>(
        req: &'a Request<Body>,
        config: &crate::protocol::ProxyAuthConfig,
    ) -> Option<&'a str> {
        match config.method {
            AuthMethod::Basic | AuthMethod::Bearer => req
                .headers()
                .get("proxy-authorization")
                .and_then(|v| v.to_str().ok()),
            AuthMethod::ApiKey => {
                let header_name = config.api_key_header_name();
                req.headers().get(header_name).and_then(|v| v.to_str().ok())
            }
        }
    }

    /// Proxy-Authenticate 응답 헤더를 인증 방식에 맞게 생성합니다.
    fn proxy_authenticate_header(config: &crate::protocol::ProxyAuthConfig) -> &'static str {
        match config.method {
            AuthMethod::Basic => "Basic realm=\"Cheolsu Proxy\"",
            AuthMethod::Bearer => "Bearer realm=\"Cheolsu Proxy\"",
            AuthMethod::ApiKey => "ApiKey",
        }
    }

    /// 프록시 인증을 확인합니다. 인증 실패 시 407 응답을 반환합니다.
    pub(crate) async fn check_proxy_auth(&self, req: &Request<Body>) -> Option<Response<Body>> {
        let auth_config = self.config.proxy_auth.read().await;
        let config = match auth_config.as_ref() {
            Some(c) if c.enabled => c,
            _ => return None,
        };

        // Basic의 경우 username이 비어있으면 인증 불필요
        if config.method == AuthMethod::Basic && config.username.is_empty() {
            return None;
        }

        let auth_value = Self::extract_auth_value(req, config);

        if config.validate_proxy_auth(auth_value) {
            None
        } else {
            info!(
                "[ProxyAuth] 인증 실패 ({:?}): {:?}",
                config.method,
                req.uri().authority().map(|a| a.to_string())
            );
            Some(response_helpers::build_response(
                Response::builder()
                    .status(StatusCode::PROXY_AUTHENTICATION_REQUIRED)
                    .header(
                        "Proxy-Authenticate",
                        Self::proxy_authenticate_header(config),
                    ),
                Body::from("Proxy Authentication Required"),
            ))
        }
    }
}

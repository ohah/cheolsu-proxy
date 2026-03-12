use proxyapi_v2::{
    hyper::http::StatusCode,
    hyper::{Request, Response},
    Body,
};
use tracing::info;

use super::LoggingHandler;

impl LoggingHandler {
    /// 프록시 인증 설정 업데이트
    pub async fn update_proxy_auth(&self, config: crate::protocol::ProxyAuthConfig) {
        let mut auth = self.config.proxy_auth.write().await;
        info!(
            "[ProxyAuth] 설정 업데이트: enabled={}, username={}",
            config.enabled, config.username
        );
        *auth = Some(config);
    }

    /// 프록시 인증을 확인합니다. 인증 실패 시 407 응답을 반환합니다.
    pub(crate) async fn check_proxy_auth(&self, req: &Request<Body>) -> Option<Response<Body>> {
        let auth_config = self.config.proxy_auth.read().await;
        let config = match auth_config.as_ref() {
            Some(c) if c.enabled && !c.username.is_empty() => c,
            _ => return None,
        };

        let auth_header = req
            .headers()
            .get("proxy-authorization")
            .and_then(|v| v.to_str().ok());

        if config.validate_proxy_auth(auth_header) {
            None
        } else {
            info!(
                "[ProxyAuth] 인증 실패: {:?}",
                req.uri().authority().map(|a| a.to_string())
            );
            Some(crate::handler::response_helpers::build_response(
                Response::builder()
                    .status(StatusCode::PROXY_AUTHENTICATION_REQUIRED)
                    .header("Proxy-Authenticate", "Basic realm=\"Cheolsu Proxy\""),
                Body::from("Proxy Authentication Required"),
            ))
        }
    }
}

use proxyapi_v2::{
    hyper::{Request, Response},
    Body,
};

use super::super::LoggingHandler;

impl LoggingHandler {
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
}

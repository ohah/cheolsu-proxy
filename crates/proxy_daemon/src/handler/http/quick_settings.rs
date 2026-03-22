use proxyapi_v2::{
    hyper::{Request, Response},
    Body,
};

use super::super::config::QuickSettings;
use super::super::LoggingHandler;

impl LoggingHandler {
    /// 현재 QuickSettings를 lock-free로 읽기
    fn quick_settings(&self) -> QuickSettings {
        QuickSettings::from_bits(
            self.config
                .quick_settings
                .load(std::sync::atomic::Ordering::Acquire),
        )
    }

    /// No Caching / Block Cookies / No Gzip 설정을 요청에 적용
    pub(crate) fn apply_quick_settings_on_request(&self, mut req: Request<Body>) -> Request<Body> {
        use proxyapi_v2::hyper::header::{
            ACCEPT_ENCODING, CACHE_CONTROL, COOKIE, IF_MODIFIED_SINCE, IF_NONE_MATCH, PRAGMA,
        };

        let settings = self.quick_settings();

        if settings.no_caching {
            req.headers_mut().remove(IF_MODIFIED_SINCE);
            req.headers_mut().remove(IF_NONE_MATCH);
            req.headers_mut().insert(
                CACHE_CONTROL,
                "no-cache, no-store, must-revalidate"
                    .parse()
                    .expect("static header value"),
            );
            req.headers_mut()
                .insert(PRAGMA, "no-cache".parse().expect("static header value"));
        }

        if settings.block_cookies {
            req.headers_mut().remove(COOKIE);
        }

        if settings.no_gzip {
            req.headers_mut().remove(ACCEPT_ENCODING);
        }

        req
    }

    /// Block Cookies / Block QUIC 설정을 응답에 적용
    pub(crate) fn apply_quick_settings_on_response(
        &self,
        mut res: Response<Body>,
    ) -> Response<Body> {
        use proxyapi_v2::hyper::header::{ALT_SVC, SET_COOKIE};

        let settings = self.quick_settings();

        if settings.block_cookies {
            res.headers_mut().remove(SET_COOKIE);
        }

        if settings.block_quic {
            res.headers_mut().remove(ALT_SVC);
        }

        res
    }
}

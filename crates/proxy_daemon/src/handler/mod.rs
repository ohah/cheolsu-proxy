mod auth;
mod config;
mod http;
pub(crate) mod response_helpers;

// Re-export for lib.rs and daemon.rs
pub use crate::tls_client::create_hybrid_client;
pub use crate::tls_client::{create_hybrid_client_with_cert, validate_client_cert_config};

// Re-export types from sub-modules
pub use crate::sse_handler::SseEvent;
pub(crate) use crate::sse_handler::SseState;
pub(crate) use crate::ws_handler::WebSocketState;
pub use crate::ws_handler::WsEvent;

// 설정 및 핸들러 구조체 re-export
pub use config::{LoggingHandler, QuickSettings};
// 외부 모듈의 테스트 코드에서 직접 LoggingHandler를 구성할 때 사용
#[allow(unused_imports)]
pub(crate) use config::{InterceptEngine, ProxyConfig, RequestState, SslProxyingConfig};

#[cfg(test)]
mod tests {
    use super::*;
    use proxyapi_v2::{
        hyper::http::StatusCode,
        hyper::{Request, Response},
        Body,
    };
    use std::sync::Arc;

    use crate::cert_distribution;

    /// 테스트용 LoggingHandler를 생성합니다.
    fn make_test_handler(ca_cert_der: Option<Vec<u8>>) -> LoggingHandler {
        let (sender, _rx) = tokio::sync::mpsc::channel(1);
        let mut handler = LoggingHandler::new(sender, std::path::PathBuf::from("/tmp"));
        if let Some(der) = ca_cert_der {
            handler = handler.with_ca_cert_der(der);
        }
        handler
    }

    #[test]
    fn cert_distribution_module_exists() {
        let req = Request::builder()
            .uri("/ssl")
            .header("host", "cheolsu.proxy")
            .body(Body::from(""))
            .unwrap();
        assert!(cert_distribution::is_cert_download_request(&req));
    }

    #[test]
    fn serve_cert_download_ssl_path_with_cert() {
        let handler = make_test_handler(Some(vec![0xDE, 0xAD, 0xBE, 0xEF]));
        let req = Request::builder()
            .uri("/ssl")
            .header("host", "cheolsu.proxy")
            .body(Body::from(""))
            .unwrap();
        let resp = handler.serve_ca_cert_download(&req);
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get("Content-Type").unwrap(),
            "application/x-x509-ca-cert"
        );
        // /ssl without User-Agent returns PEM (.crt) format, not raw DER
        // 4 DER bytes -> PEM base64 wrapping = 63 bytes
        assert_eq!(resp.headers().get("Content-Length").unwrap(), "63");
        assert!(resp
            .headers()
            .get("Content-Disposition")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("cheolsu-proxy-ca.crt"));
    }

    #[test]
    fn serve_cert_download_cert_path_with_cert() {
        let handler = make_test_handler(Some(vec![1, 2, 3]));
        let req = Request::builder()
            .uri("/cert")
            .body(Body::from(""))
            .unwrap();
        let resp = handler.serve_ca_cert_download(&req);
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get("Content-Type").unwrap(),
            "application/x-x509-ca-cert"
        );
    }

    #[test]
    fn serve_cert_download_root_path_shows_landing_page() {
        let handler = make_test_handler(Some(vec![1]));
        let req = Request::builder().uri("/").body(Body::from("")).unwrap();
        let resp = handler.serve_ca_cert_download(&req);
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(resp
            .headers()
            .get("Content-Type")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("text/html"));
    }

    #[test]
    fn serve_cert_download_ssl_path_without_cert() {
        let handler = make_test_handler(None);
        let req = Request::builder().uri("/ssl").body(Body::from("")).unwrap();
        let resp = handler.serve_ca_cert_download(&req);
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn serve_cert_download_other_path_returns_html() {
        let handler = make_test_handler(Some(vec![1]));
        let req = Request::builder()
            .uri("/about")
            .body(Body::from(""))
            .unwrap();
        let resp = handler.serve_ca_cert_download(&req);
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(resp
            .headers()
            .get("Content-Type")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("text/html"));
    }

    #[test]
    fn with_ca_cert_der_sets_bytes() {
        let handler = make_test_handler(None);
        assert!(handler.config.ca_cert_der.is_none());

        let handler = make_test_handler(Some(vec![0xFF, 0x00]));
        assert!(handler.config.ca_cert_der.is_some());
        assert_eq!(handler.config.ca_cert_der.unwrap().len(), 2);
    }

    #[test]
    fn host_matching_via_cert_distribution() {
        let matching = Request::builder()
            .uri("/ssl")
            .header("host", "cheolsu.proxy:8080")
            .body(Body::from(""))
            .unwrap();
        assert!(cert_distribution::is_cert_download_request(&matching));

        let non_matching = Request::builder()
            .uri("/api")
            .header("host", "other.proxy:8080")
            .body(Body::from(""))
            .unwrap();
        assert!(!cert_distribution::is_cert_download_request(&non_matching));

        let evil = Request::builder()
            .uri("/api")
            .header("host", "cheolsu.proxy.evil.com")
            .body(Body::from(""))
            .unwrap();
        assert!(!cert_distribution::is_cert_download_request(&evil));
    }

    // --- check_cert_download_intercept 테스트 ---

    #[test]
    fn cert_intercept_host_header_exact() {
        let handler = make_test_handler(Some(vec![1]));
        let req = Request::builder()
            .uri("/anything")
            .header("host", "cheolsu.proxy")
            .body(Body::from(""))
            .unwrap();
        assert!(handler.check_cert_download_intercept(&req).is_some());
    }

    #[test]
    fn cert_intercept_host_header_with_port() {
        let handler = make_test_handler(Some(vec![1]));
        let req = Request::builder()
            .uri("/anything")
            .header("host", "cheolsu.proxy:8100")
            .body(Body::from(""))
            .unwrap();
        assert!(handler.check_cert_download_intercept(&req).is_some());
    }

    #[test]
    fn cert_intercept_absolute_uri() {
        let handler = make_test_handler(Some(vec![1]));
        let req = Request::builder()
            .uri("http://cheolsu.proxy/ssl")
            .body(Body::from(""))
            .unwrap();
        assert!(handler.check_cert_download_intercept(&req).is_some());
    }

    #[test]
    fn cert_intercept_direct_ip_ssl_path() {
        let handler = make_test_handler(Some(vec![1]));
        let req = Request::builder().uri("/ssl").body(Body::from("")).unwrap();
        assert!(handler.check_cert_download_intercept(&req).is_some());
    }

    #[test]
    fn cert_intercept_direct_ip_cert_path() {
        let handler = make_test_handler(Some(vec![1]));
        let req = Request::builder()
            .uri("/cert")
            .body(Body::from(""))
            .unwrap();
        assert!(handler.check_cert_download_intercept(&req).is_some());
    }

    #[test]
    fn cert_intercept_non_matching_host() {
        let handler = make_test_handler(Some(vec![1]));
        let req = Request::builder()
            .uri("/api/data")
            .header("host", "example.com")
            .body(Body::from(""))
            .unwrap();
        assert!(handler.check_cert_download_intercept(&req).is_none());
    }

    #[test]
    fn cert_intercept_non_matching_path_without_host() {
        let handler = make_test_handler(Some(vec![1]));
        let req = Request::builder()
            .uri("/api/data")
            .body(Body::from(""))
            .unwrap();
        assert!(handler.check_cert_download_intercept(&req).is_none());
    }

    // --- Quick Settings (No Caching / Block Cookies) 테스트 ---

    /// quick_settings를 지정하여 테스트용 핸들러를 생성하는 헬퍼
    fn make_handler_with_quick_settings(settings: QuickSettings) -> LoggingHandler {
        let qs = Arc::new(std::sync::atomic::AtomicU8::new(settings.to_bits()));
        let handler = make_test_handler(None).with_quick_settings(qs);
        handler
    }

    #[tokio::test]
    async fn no_caching_adds_cache_control_headers() {
        let handler = make_handler_with_quick_settings(QuickSettings {
            no_caching: true,
            ..QuickSettings::default()
        });

        let req = Request::builder()
            .uri("http://example.com/")
            .body(Body::from(""))
            .unwrap();

        let req = handler.apply_quick_settings_on_request(req);

        assert_eq!(
            req.headers().get("cache-control").unwrap(),
            "no-cache, no-store, must-revalidate"
        );
        assert_eq!(req.headers().get("pragma").unwrap(), "no-cache");
    }

    #[tokio::test]
    async fn no_caching_removes_conditional_headers() {
        let handler = make_handler_with_quick_settings(QuickSettings {
            no_caching: true,
            ..QuickSettings::default()
        });

        let req = Request::builder()
            .uri("http://example.com/")
            .header("If-Modified-Since", "Thu, 01 Jan 2026 00:00:00 GMT")
            .header("If-None-Match", "\"etag123\"")
            .body(Body::from(""))
            .unwrap();

        let req = handler.apply_quick_settings_on_request(req);

        assert!(req.headers().get("if-modified-since").is_none());
        assert!(req.headers().get("if-none-match").is_none());
    }

    #[tokio::test]
    async fn block_cookies_removes_cookie_from_request() {
        let handler = make_handler_with_quick_settings(QuickSettings {
            block_cookies: true,
            ..QuickSettings::default()
        });

        let req = Request::builder()
            .uri("http://example.com/")
            .header("Cookie", "session=abc123; user=test")
            .body(Body::from(""))
            .unwrap();

        let req = handler.apply_quick_settings_on_request(req);

        assert!(req.headers().get("cookie").is_none());
    }

    #[tokio::test]
    async fn block_cookies_removes_set_cookie_from_response() {
        let handler = make_handler_with_quick_settings(QuickSettings {
            block_cookies: true,
            ..QuickSettings::default()
        });

        let res = Response::builder()
            .status(200)
            .header("Set-Cookie", "session=abc123; Path=/")
            .header("Content-Type", "text/html")
            .body(Body::from(""))
            .unwrap();

        let res = handler.apply_quick_settings_on_response(res);

        assert!(res.headers().get("set-cookie").is_none());
        // 다른 헤더는 영향받지 않아야 함
        assert!(res.headers().get("content-type").is_some());
    }

    #[tokio::test]
    async fn disabled_quick_settings_preserves_all_headers() {
        let handler = make_handler_with_quick_settings(QuickSettings {
            ..QuickSettings::default()
        });

        let req = Request::builder()
            .uri("http://example.com/")
            .header("If-Modified-Since", "Thu, 01 Jan 2026 00:00:00 GMT")
            .header("If-None-Match", "\"etag123\"")
            .header("Cookie", "session=abc123")
            .body(Body::from(""))
            .unwrap();

        let req = handler.apply_quick_settings_on_request(req);

        assert!(req.headers().get("if-modified-since").is_some());
        assert!(req.headers().get("if-none-match").is_some());
        assert!(req.headers().get("cookie").is_some());
        assert!(req.headers().get("cache-control").is_none());

        let res = Response::builder()
            .status(200)
            .header("Set-Cookie", "session=abc123; Path=/")
            .body(Body::from(""))
            .unwrap();

        let res = handler.apply_quick_settings_on_response(res);

        assert!(res.headers().get("set-cookie").is_some());
    }

    #[tokio::test]
    async fn both_settings_enabled_applies_all_modifications() {
        let handler = make_handler_with_quick_settings(QuickSettings {
            no_caching: true,
            block_cookies: true,
            ..QuickSettings::default()
        });

        let req = Request::builder()
            .uri("http://example.com/")
            .header("If-Modified-Since", "Thu, 01 Jan 2026 00:00:00 GMT")
            .header("If-None-Match", "\"etag123\"")
            .header("Cookie", "session=abc123")
            .body(Body::from(""))
            .unwrap();

        let req = handler.apply_quick_settings_on_request(req);

        // No Caching 적용 확인
        assert!(req.headers().get("if-modified-since").is_none());
        assert!(req.headers().get("if-none-match").is_none());
        assert_eq!(
            req.headers().get("cache-control").unwrap(),
            "no-cache, no-store, must-revalidate"
        );
        assert_eq!(req.headers().get("pragma").unwrap(), "no-cache");
        // Block Cookies 적용 확인
        assert!(req.headers().get("cookie").is_none());

        let res = Response::builder()
            .status(200)
            .header("Set-Cookie", "session=abc123; Path=/")
            .body(Body::from(""))
            .unwrap();

        let res = handler.apply_quick_settings_on_response(res);

        assert!(res.headers().get("set-cookie").is_none());
    }

    /// 동시 읽기/쓰기 시 데드락이 발생하지 않는지 검증
    #[tokio::test]
    async fn concurrent_quick_settings_read_write_no_deadlock() {
        let qs = Arc::new(std::sync::atomic::AtomicU8::new(0));

        let mut handles = Vec::new();

        // 여러 읽기 태스크 동시 실행 (요청 처리 시뮬레이션)
        for _ in 0..10 {
            let qs_clone = qs.clone();
            handles.push(tokio::spawn(async move {
                for _ in 0..100 {
                    let settings = QuickSettings::from_bits(
                        qs_clone.load(std::sync::atomic::Ordering::Relaxed),
                    );
                    let _ = settings.no_caching;
                    let _ = settings.block_cookies;
                    tokio::task::yield_now().await;
                }
            }));
        }

        // 동시에 쓰기 태스크 실행 (설정 변경 시뮬레이션)
        for i in 0..5 {
            let qs_clone = qs.clone();
            handles.push(tokio::spawn(async move {
                for _ in 0..100 {
                    let settings = QuickSettings {
                        no_caching: i % 2 == 0,
                        block_cookies: i % 2 == 1,
                        ..QuickSettings::default()
                    };
                    qs_clone.store(settings.to_bits(), std::sync::atomic::Ordering::Relaxed);
                    tokio::task::yield_now().await;
                }
            }));
        }

        // 3초 타임아웃 - 데드락 시 실패
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            futures_util::future::join_all(handles),
        )
        .await;

        assert!(result.is_ok(), "데드락 감지: 3초 타임아웃 초과");
        for r in result.unwrap() {
            r.unwrap();
        }
    }

    /// apply_quick_settings 메서드의 동시 호출이 데드락 없이 완료되는지 검증
    #[tokio::test]
    async fn concurrent_apply_quick_settings_no_deadlock() {
        let handler = make_handler_with_quick_settings(QuickSettings {
            no_caching: true,
            block_cookies: true,
            ..QuickSettings::default()
        });

        let mut handles = Vec::new();

        for _ in 0..10 {
            let h = handler.clone();
            handles.push(tokio::spawn(async move {
                for _ in 0..50 {
                    let req = Request::builder()
                        .uri("http://example.com/")
                        .header("Cookie", "session=abc")
                        .header("If-None-Match", "\"etag\"")
                        .body(Body::from(""))
                        .unwrap();
                    let req = h.apply_quick_settings_on_request(req);
                    assert!(req.headers().get("cookie").is_none());

                    let res = Response::builder()
                        .status(200)
                        .header("Set-Cookie", "session=abc; Path=/")
                        .body(Body::from(""))
                        .unwrap();
                    let res = h.apply_quick_settings_on_response(res);
                    assert!(res.headers().get("set-cookie").is_none());

                    tokio::task::yield_now().await;
                }
            }));
        }

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            futures_util::future::join_all(handles),
        )
        .await;

        assert!(result.is_ok(), "데드락 감지: 3초 타임아웃 초과");
        for r in result.unwrap() {
            r.unwrap();
        }
    }

    #[tokio::test]
    async fn no_gzip_removes_accept_encoding_header() {
        let handler = make_handler_with_quick_settings(QuickSettings {
            no_gzip: true,
            ..QuickSettings::default()
        });

        let req = Request::builder()
            .uri("http://example.com/")
            .header("Accept-Encoding", "gzip, deflate, br")
            .body(Body::from(""))
            .unwrap();

        let req = handler.apply_quick_settings_on_request(req);

        assert!(req.headers().get("accept-encoding").is_none());
    }

    #[tokio::test]
    async fn no_gzip_disabled_preserves_accept_encoding() {
        let handler = make_handler_with_quick_settings(QuickSettings {
            ..QuickSettings::default()
        });

        let req = Request::builder()
            .uri("http://example.com/")
            .header("Accept-Encoding", "gzip, deflate, br")
            .body(Body::from(""))
            .unwrap();

        let req = handler.apply_quick_settings_on_request(req);

        assert!(req.headers().get("accept-encoding").is_some());
    }

    #[tokio::test]
    async fn all_quick_settings_enabled_applies_all() {
        let handler = make_handler_with_quick_settings(QuickSettings {
            no_caching: true,
            block_cookies: true,
            no_gzip: true,
            ..QuickSettings::default()
        });

        let req = Request::builder()
            .uri("http://example.com/")
            .header("If-Modified-Since", "Thu, 01 Jan 2026 00:00:00 GMT")
            .header("Cookie", "session=abc123")
            .header("Accept-Encoding", "gzip, deflate, br")
            .body(Body::from(""))
            .unwrap();

        let req = handler.apply_quick_settings_on_request(req);

        // No Caching
        assert!(req.headers().get("if-modified-since").is_none());
        assert_eq!(
            req.headers().get("cache-control").unwrap(),
            "no-cache, no-store, must-revalidate"
        );
        // Block Cookies
        assert!(req.headers().get("cookie").is_none());
        // No Gzip
        assert!(req.headers().get("accept-encoding").is_none());
    }
}

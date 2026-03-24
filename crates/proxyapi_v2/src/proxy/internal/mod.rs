mod connect;
mod serve;

use super::context::ProxyContext;
use super::middleware::optimize_streaming_response;
use crate::{
    HttpContext, HttpHandler, RequestOrResponse, WebSocketHandler,
    body::Body,
    certificate_authority::CertificateAuthority,
    tls_event::{TlsEvent, TlsEventSender, emit_tls_event},
    upstream_cert::{ClientHelloMirrorInfo, UpstreamCertInfo, sniff_upstream_cert_with_mirror},
};
use http::uri::Authority;
use hyper::{Method, Request, Response, body::Incoming, header::Entry};
use hyper_util::{
    client::legacy::{Client, connect::Connect},
    rt::TokioExecutor,
    server,
};
use std::{convert::Infallible, net::SocketAddr, pin::Pin, sync::Arc};
use tracing::{Instrument, error, info_span, instrument, warn};

/// ClientHello 미러링 정보를 포함한 Lazy upstream cert 스니핑
pub(super) async fn lazy_sniff_upstream_with_mirror(
    authority: &Authority,
    upstream_proxy: Option<&crate::upstream_proxy::UpstreamProxyConfig>,
    tls_event_sender: &Option<TlsEventSender>,
    mirror_info: Option<&ClientHelloMirrorInfo>,
) -> Option<UpstreamCertInfo> {
    emit_tls_event(
        tls_event_sender,
        TlsEvent::ServerConnectionStarting {
            authority: authority.clone(),
        },
    );
    let cert = sniff_upstream_cert_with_mirror(authority, upstream_proxy, mirror_info).await;
    emit_tls_event(
        tls_event_sender,
        TlsEvent::UpstreamCertSniffed {
            authority: authority.clone(),
            cert_info: cert.clone(),
        },
    );
    cert
}

pub(crate) struct InternalProxy<C, CA, H, W> {
    pub(crate) ca: Arc<CA>,
    pub(crate) client: Client<C, Body>,
    pub(crate) server: server::conn::auto::Builder<TokioExecutor>,
    pub(crate) http_handler: H,
    pub(crate) websocket_handler: W,
    pub(crate) client_addr: SocketAddr,
    pub(crate) ctx: ProxyContext,
    /// 상류 서버 인증서 정보 (TLS 인터셉트 시 캡처)
    pub(crate) upstream_cert_info: Option<UpstreamCertInfo>,
    /// TLS 학습 기반 폴백 사용 여부
    pub(crate) tls_fallback_used: Option<bool>,
}

impl<C, CA, H, W> Clone for InternalProxy<C, CA, H, W>
where
    C: Clone,
    H: Clone,
    W: Clone,
{
    fn clone(&self) -> Self {
        InternalProxy {
            ca: Arc::clone(&self.ca),
            client: self.client.clone(),
            server: self.server.clone(),
            http_handler: self.http_handler.clone(),
            websocket_handler: self.websocket_handler.clone(),
            client_addr: self.client_addr,
            ctx: self.ctx.clone(),
            upstream_cert_info: self.upstream_cert_info.clone(),
            tls_fallback_used: self.tls_fallback_used,
        }
    }
}

impl<C, CA, H, W> hyper::service::Service<Request<Incoming>> for InternalProxy<C, CA, H, W>
where
    C: Connect + Clone + Send + Sync + 'static,
    CA: CertificateAuthority,
    H: HttpHandler,
    W: WebSocketHandler,
{
    type Response = Response<Body>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        let proxy = self.clone();
        Box::pin(proxy.proxy(req))
    }
}

impl<C, CA, H, W> InternalProxy<C, CA, H, W>
where
    C: Connect + Clone + Send + Sync + 'static,
    CA: CertificateAuthority,
    H: HttpHandler,
    W: WebSocketHandler,
{
    pub(crate) fn context(&self) -> HttpContext {
        HttpContext {
            client_addr: self.client_addr,
            server_cert: self
                .upstream_cert_info
                .as_ref()
                .map(|info| info.to_server_cert_info()),
            tls_fallback_used: self.tls_fallback_used,
        }
    }

    #[instrument(
        skip_all,
        fields(
            version = ?req.version(),
            method = %req.method(),
            uri=%req.uri(),
            client_addr = %self.client_addr,
        )
    )]
    pub(crate) async fn proxy(
        mut self,
        req: Request<Incoming>,
    ) -> Result<Response<Body>, Infallible> {
        let ctx = self.context();

        // 메트릭: 활성 요청 증가
        if let Some(ref metrics) = self.ctx.metrics {
            metrics.request_started();
        }

        let request_start = std::time::Instant::now();

        let req = match self
            .http_handler
            .handle_request(&ctx, req.map(Body::from))
            .instrument(info_span!("handle_request"))
            .await
        {
            RequestOrResponse::Request(req) => req,
            RequestOrResponse::Response(res) => {
                // 메트릭: 활성 요청 감소
                if let Some(ref metrics) = self.ctx.metrics {
                    metrics.request_finished();
                }
                return Ok(res);
            }
        };

        if req.method() == Method::CONNECT {
            // CONNECT 요청은 터널링이므로 여기서 활성 요청 감소
            if let Some(ref metrics) = self.ctx.metrics {
                metrics.request_finished();
            }
            Ok(self.process_connect(req))
        } else if hyper_tungstenite::is_upgrade_request(&req) {
            if let Some(ref metrics) = self.ctx.metrics {
                metrics.request_finished();
            }
            Ok(self.upgrade_websocket(req))
        } else {
            let normalized_req = normalize_request(req);

            // 요청 정보 미리 추출 (에러 로깅용 + 메트릭용)
            let req_uri = normalized_req.uri().clone();
            let req_method = normalized_req.method().clone();
            let req_host = normalized_req.headers().get("host").cloned();
            let req_user_agent = normalized_req.headers().get("user-agent").cloned();
            let domain = req_uri.host().unwrap_or("unknown").to_string();
            let req_content_length = normalized_req
                .headers()
                .get("content-length")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0);

            let res = self
                .client
                .request(normalized_req)
                .instrument(info_span!("proxy_request"))
                .await;

            let result = match res {
                Ok(res) => {
                    // 메트릭: 요청 완료 기록
                    if let Some(ref metrics) = self.ctx.metrics {
                        let duration_ms = request_start.elapsed().as_millis() as u64;
                        let status = res.status().as_u16();
                        let res_content_length = res
                            .headers()
                            .get("content-length")
                            .and_then(|v| v.to_str().ok())
                            .and_then(|v| v.parse::<u64>().ok())
                            .unwrap_or(0);
                        metrics.request_completed(
                            domain,
                            status,
                            duration_ms,
                            req_content_length,
                            res_content_length,
                        );
                    }

                    let response = optimize_streaming_response(res.map(Body::from), &req_uri);

                    Ok(self
                        .http_handler
                        .handle_response(&ctx, response)
                        .instrument(info_span!("handle_response"))
                        .await)
                }
                Err(err) => {
                    // 메트릭: 연결 실패 기록
                    if let Some(ref metrics) = self.ctx.metrics {
                        metrics.connection_failed(domain, err.to_string());
                    }

                    error!(
                        url = %req_uri,
                        method = %req_method,
                        host = ?req_host,
                        user_agent = ?req_user_agent,
                        error = %err,
                        error_type = ?err,
                        "프록시 요청 실패"
                    );

                    Ok(self
                        .http_handler
                        .handle_error(&ctx, err)
                        .instrument(info_span!("handle_error"))
                        .await)
                }
            };

            // 메트릭: 활성 요청 감소
            if let Some(ref metrics) = self.ctx.metrics {
                metrics.request_finished();
            }

            result
        }
    }
}

#[instrument(skip_all)]
pub(crate) fn normalize_request<T>(mut req: Request<T>) -> Request<T> {
    // Hyper will automatically add a Host header if needed.
    req.headers_mut().remove(hyper::header::HOST);

    // HTTP/2 supports multiple cookie headers, but HTTP/1.x only supports one.
    if let Entry::Occupied(mut cookies) = req.headers_mut().entry(hyper::header::COOKIE) {
        let joined_cookies = bstr::join(b"; ", cookies.iter());
        match joined_cookies.try_into() {
            Ok(value) => {
                cookies.insert(value);
            }
            Err(e) => {
                warn!("쿠키 결합 실패: {}", e);
            }
        }
    }

    *req.version_mut() = hyper::Version::HTTP_11;
    req
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::StatusCode;
    use hyper_util::client::legacy::connect::HttpConnector;
    use tokio_rustls::rustls::ServerConfig;

    struct CA;

    impl CertificateAuthority for CA {
        async fn gen_server_config(
            &self,
            _authority: &Authority,
            _upstream_cert: Option<&crate::upstream_cert::UpstreamCertInfo>,
        ) -> Result<Arc<ServerConfig>, Box<dyn std::error::Error + Send + Sync>> {
            unimplemented!();
        }

        fn get_ca_cert_der(&self) -> Option<Vec<u8>> {
            None
        }

        async fn is_config_cached(&self, _authority: &Authority) -> bool {
            false
        }

        #[cfg(feature = "openssl-ca")]
        async fn gen_openssl_context(
            &self,
            _authority: &Authority,
            _upstream_cert: Option<&crate::upstream_cert::UpstreamCertInfo>,
        ) -> Result<openssl::ssl::SslContext, Box<dyn std::error::Error + Send + Sync>> {
            unimplemented!();
        }
    }

    fn build_proxy() -> InternalProxy<HttpConnector, CA, crate::NoopHandler, crate::NoopHandler> {
        InternalProxy {
            ca: Arc::new(CA),
            client: Client::builder(TokioExecutor::new()).build(HttpConnector::new()),
            server: server::conn::auto::Builder::new(TokioExecutor::new()),
            http_handler: crate::NoopHandler::new(),
            websocket_handler: crate::NoopHandler::new(),
            client_addr: "127.0.0.1:8080".parse().unwrap(),
            ctx: ProxyContext::new(),
            upstream_cert_info: None,
            tls_fallback_used: None,
        }
    }

    mod bad_request {
        use super::*;

        #[test]
        fn correct_status() {
            let res = crate::proxy::helpers::bad_request();
            assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        }
    }

    mod normalize_request {
        use super::*;

        #[test]
        fn removes_host_header() {
            let req = Request::builder()
                .uri("http://example.com/")
                .header(hyper::header::HOST, "example.com")
                .body(())
                .unwrap();

            let req = super::normalize_request(req);

            assert_eq!(req.headers().get(hyper::header::HOST), None);
        }

        #[test]
        fn joins_cookies() {
            let req = Request::builder()
                .uri("http://example.com/")
                .header(hyper::header::COOKIE, "foo=bar")
                .header(hyper::header::COOKIE, "baz=qux")
                .body(())
                .unwrap();

            let req = super::normalize_request(req);

            assert_eq!(
                req.headers().get_all(hyper::header::COOKIE).iter().count(),
                1
            );

            assert_eq!(
                req.headers().get(hyper::header::COOKIE),
                Some(&"foo=bar; baz=qux".parse().unwrap())
            );
        }
    }

    mod tls_error_hint {
        use super::super::connect::{detect_tls_backend, tls_error_hint};

        #[test]
        fn signature_algorithms_extension() {
            let hint = tls_error_hint("SignatureAlgorithmsExtensionRequired error occurred");
            assert!(hint.contains("SignatureAlgorithmsExtension"));
        }

        #[test]
        fn peer_incompatible() {
            let hint = tls_error_hint("peer is incompatible with TLS version");
            assert!(hint.contains("호환성"));
        }

        #[test]
        fn certificate_error() {
            let hint = tls_error_hint("certificate verification failed");
            assert!(hint.contains("인증서"));
        }

        #[test]
        fn handshake_error() {
            let hint = tls_error_hint("handshake protocol error");
            assert!(hint.contains("핸드셰이크"));
        }

        #[test]
        fn timeout_error() {
            let hint = tls_error_hint("connection timeout during TLS");
            assert!(hint.contains("타임아웃"));
        }

        #[test]
        fn unknown_error_returns_empty() {
            let hint = tls_error_hint("some random error");
            assert!(hint.is_empty());
        }

        #[test]
        fn detect_rustls_backend() {
            assert_eq!(detect_tls_backend("rustls error: invalid cert"), "RUSTLS");
        }

        #[test]
        fn detect_native_tls_backend() {
            assert_eq!(
                detect_tls_backend("native-tls error: handshake failed"),
                "NATIVE-TLS"
            );
        }

        #[test]
        fn detect_openssl_backend() {
            assert_eq!(
                detect_tls_backend("openssl SSL_ERROR_SYSCALL"),
                "NATIVE-TLS"
            );
        }

        #[test]
        fn detect_unknown_backend() {
            assert_eq!(detect_tls_backend("generic TLS failure"), "UNKNOWN");
        }
    }

    mod process_connect {
        use super::*;

        #[test]
        fn returns_bad_request_if_missing_authority() {
            let proxy = build_proxy();

            let req = Request::builder()
                .uri("/foo/bar?baz")
                .body(Body::empty())
                .unwrap();

            let res = proxy.process_connect(req);

            assert_eq!(res.status(), StatusCode::BAD_REQUEST)
        }
    }

    mod upgrade_websocket {
        use super::*;

        #[test]
        fn returns_bad_request_if_missing_authority() {
            let proxy = build_proxy();

            let req = Request::builder()
                .uri("/foo/bar?baz")
                .body(Body::empty())
                .unwrap();

            let res = proxy.upgrade_websocket(req);

            assert_eq!(res.status(), StatusCode::BAD_REQUEST)
        }

        #[test]
        fn returns_bad_request_if_missing_headers() {
            let proxy = build_proxy();

            let req = Request::builder()
                .uri("http://example.com/foo/bar?baz")
                .body(Body::empty())
                .unwrap();

            let res = proxy.upgrade_websocket(req);

            assert_eq!(res.status(), StatusCode::BAD_REQUEST)
        }
    }
}

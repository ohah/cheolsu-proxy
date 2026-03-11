use super::context::ProxyContext;
use super::helpers::{bad_request, spawn_with_trace};
use super::middleware::optimize_streaming_response;
use crate::{
    HttpContext, HttpHandler, RequestOrResponse, WebSocketHandler,
    body::Body,
    certificate_authority::CertificateAuthority,
    hybrid_tls_handler::HybridTlsHandler,
    rewind::Rewind,
    throttle,
    tls_event::{TlsEvent, emit_tls_event},
    tls_version_detector::TlsVersionDetector,
    upstream_cert::sniff_upstream_cert,
    upstream_proxy::connect_to_target,
};
use http::uri::{Authority, Scheme};
use hyper::{
    Method, Request, Response, Uri,
    body::{Bytes, Incoming},
    header::Entry,
    service::service_fn,
};
use hyper_util::{
    client::legacy::{Client, connect::Connect},
    rt::{TokioExecutor, TokioIo},
    server,
};
use std::{convert::Infallible, net::SocketAddr, pin::Pin, sync::Arc};
use tokio::io::AsyncReadExt;
use tokio_rustls::TlsAcceptor;
use tracing::{Instrument, debug, error, info, info_span, instrument, warn};

pub(crate) struct InternalProxy<C, CA, H, W> {
    pub(crate) ca: Arc<CA>,
    pub(crate) client: Client<C, Body>,
    pub(crate) server: server::conn::auto::Builder<TokioExecutor>,
    pub(crate) http_handler: H,
    pub(crate) websocket_handler: W,
    pub(crate) client_addr: SocketAddr,
    pub(crate) ctx: ProxyContext,
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

        let req = match self
            .http_handler
            .handle_request(&ctx, req.map(Body::from))
            .instrument(info_span!("handle_request"))
            .await
        {
            RequestOrResponse::Request(req) => req,
            RequestOrResponse::Response(res) => return Ok(res),
        };

        if req.method() == Method::CONNECT {
            Ok(self.process_connect(req))
        } else if hyper_tungstenite::is_upgrade_request(&req) {
            Ok(self.upgrade_websocket(req))
        } else {
            let normalized_req = normalize_request(req);

            // 요청 정보 미리 추출 (에러 로깅용)
            let req_uri = normalized_req.uri().clone();
            let req_method = normalized_req.method().clone();
            let req_host = normalized_req.headers().get("host").cloned();
            let req_user_agent = normalized_req.headers().get("user-agent").cloned();

            let res = self
                .client
                .request(normalized_req)
                .instrument(info_span!("proxy_request"))
                .await;

            match res {
                Ok(res) => {
                    let response = optimize_streaming_response(res.map(Body::from), &req_uri);

                    Ok(self
                        .http_handler
                        .handle_response(&ctx, response)
                        .instrument(info_span!("handle_response"))
                        .await)
                }
                Err(err) => {
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
            }
        }
    }

    pub(crate) fn process_connect(mut self, mut req: Request<Body>) -> Response<Body> {
        match req.uri().authority().cloned() {
            Some(authority) => {
                // 자동 학습 바이패스 체크 (이전에 TLS 핸드셰이크 실패한 도메인)
                let should_bypass = if let Some(ref passthrough) = self.ctx.tls_passthrough {
                    match passthrough.failures_ref().try_read() {
                        Ok(failures) => failures.contains_key(authority.host()),
                        Err(_) => false,
                    }
                } else {
                    false
                };

                if should_bypass {
                    debug!(
                        "[TLS-PASSTHROUGH] 자동 바이패스 적용 (이전 실패 기록): {}",
                        authority
                    );
                    let response = match Response::builder()
                        .status(200)
                        .header("Connection", "keep-alive")
                        .body(Body::empty())
                    {
                        Ok(resp) => resp,
                        Err(e) => {
                            error!("바이패스 응답 생성 실패: {}", e);
                            return bad_request();
                        }
                    };

                    let authority_clone = authority.clone();
                    let _tunnel_sender = self.ctx.tunnel_event_sender.clone();
                    let _client_addr = self.client_addr;
                    tokio::spawn(async move {
                        match hyper::upgrade::on(&mut req).await {
                            Ok(upgraded) => {
                                let mut client_stream = TokioIo::new(upgraded);
                                let target_addr = format!(
                                    "{}:{}",
                                    authority_clone.host(),
                                    authority_clone
                                        .port()
                                        .map(|p| p.to_string())
                                        .unwrap_or_else(|| "443".to_string())
                                );
                                match connect_to_target(
                                    &target_addr,
                                    self.ctx.upstream_proxy.as_ref(),
                                )
                                .await
                                {
                                    Ok(mut server_stream) => {
                                        let throttle_config = self
                                            .ctx
                                            .throttle_rx
                                            .as_ref()
                                            .and_then(|rx| rx.borrow().clone());
                                        let _ = if let Some(ref config) = throttle_config {
                                            if config.enabled {
                                                throttle::copy_bidirectional_throttled(
                                                    &mut client_stream,
                                                    &mut server_stream,
                                                    config,
                                                )
                                                .await
                                            } else {
                                                tokio::io::copy_bidirectional(
                                                    &mut client_stream,
                                                    &mut server_stream,
                                                )
                                                .await
                                            }
                                        } else {
                                            tokio::io::copy_bidirectional(
                                                &mut client_stream,
                                                &mut server_stream,
                                            )
                                            .await
                                        };
                                    }
                                    Err(e) => {
                                        error!(
                                            "[TLS-PASSTHROUGH] 서버 연결 실패: {} - {}",
                                            target_addr, e
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                error!("[TLS-PASSTHROUGH] 업그레이드 실패: {}", e);
                            }
                        }
                    });

                    return response;
                }

                let span = info_span!("process_connect");
                let fut = async move {
                    match hyper::upgrade::on(&mut req).await {
                        Ok(upgraded) => {
                            let mut upgraded = TokioIo::new(upgraded);

                            // 먼저 TLS 레코드 헤더를 읽어서 전체 길이를 파악
                            let mut header_buffer = [0; 5]; // TLS 레코드 헤더 (5 bytes)
                            let header_bytes_read = match upgraded.read(&mut header_buffer).await {
                                Ok(bytes_read) => bytes_read,
                                Err(e) => {
                                    error!(
                                        "Failed to read TLS header from upgraded connection: {}",
                                        e
                                    );
                                    return;
                                }
                            };

                            if header_bytes_read < 5 {
                                error!("TLS header too short: {} bytes", header_bytes_read);
                                return;
                            }

                            // 레코드 길이 계산 (bytes 3-4)
                            let record_length =
                                u16::from_be_bytes([header_buffer[3], header_buffer[4]]) as usize;
                            let total_expected_length = 5 + record_length; // 헤더(5) + 레코드 길이

                            debug!(
                                record_type = format_args!("0x{:02x}", header_buffer[0]),
                                record_version = format_args!(
                                    "0x{:02x}{:02x}",
                                    header_buffer[1], header_buffer[2]
                                ),
                                record_length,
                                total_expected_length,
                                "[BUFFER-READ] TLS 레코드 분석"
                            );

                            // 전체 ClientHello 메시지를 읽기 위한 버퍼 생성
                            let mut full_buffer = vec![0; total_expected_length];
                            full_buffer[..5].copy_from_slice(&header_buffer);

                            // 나머지 데이터 읽기
                            let remaining_bytes = total_expected_length - 5;
                            if remaining_bytes > 0 {
                                let remaining_bytes_read =
                                    match upgraded.read(&mut full_buffer[5..]).await {
                                        Ok(bytes_read) => bytes_read,
                                        Err(e) => {
                                            error!("Failed to read remaining TLS data: {}", e);
                                            return;
                                        }
                                    };

                                debug!(
                                    header_bytes = 5,
                                    remaining_bytes_read,
                                    remaining_bytes_expected = remaining_bytes,
                                    total_bytes_read = 5 + remaining_bytes_read,
                                    "[BUFFER-READ] 데이터 읽기 완료"
                                );

                                if remaining_bytes_read < remaining_bytes {
                                    warn!(
                                        remaining_bytes_read,
                                        remaining_bytes_expected = remaining_bytes,
                                        "전체 ClientHello가 완전히 읽히지 않음"
                                    );
                                }
                            }

                            let mut upgraded =
                                Rewind::new(upgraded, Bytes::copy_from_slice(&full_buffer));

                            if self
                                .http_handler
                                .should_intercept(&self.context(), &req)
                                .await
                            {
                                if full_buffer.len() >= 4 && full_buffer[..4] == *b"GET " {
                                    if let Err(e) = self
                                        .serve_stream(
                                            TokioIo::new(upgraded),
                                            Scheme::HTTP,
                                            authority,
                                        )
                                        .await
                                    {
                                        error!("WebSocket connect error: {}", e);
                                    }

                                    return;
                                } else if full_buffer.len() >= 2 && full_buffer[..2] == *b"\x16\x03"
                                {
                                    // TLS 버전 감지
                                    let tls_version =
                                        TlsVersionDetector::detect_tls_version(&full_buffer);

                                    match tls_version {
                                        Some(version) => {
                                            debug!(
                                                %version,
                                                "TLS 버전 감지 - 하이브리드 핸들러 사용"
                                            );

                                            // 캐시 히트 시 스니핑 스킵 (불필요한 upstream 연결 방지)
                                            let upstream_cert = if self
                                                .ca
                                                .is_config_cached(&authority)
                                                .await
                                            {
                                                debug!(
                                                    "[UPSTREAM-CERT] 캐시 히트 - 스니핑 스킵: {}",
                                                    authority
                                                );
                                                None
                                            } else {
                                                emit_tls_event(
                                                    &self.ctx.tls_event_sender,
                                                    TlsEvent::ServerConnectionStarting {
                                                        authority: authority.clone(),
                                                    },
                                                );
                                                let cert = sniff_upstream_cert(
                                                    &authority,
                                                    self.ctx.upstream_proxy.as_ref(),
                                                )
                                                .await;
                                                emit_tls_event(
                                                    &self.ctx.tls_event_sender,
                                                    TlsEvent::UpstreamCertSniffed {
                                                        authority: authority.clone(),
                                                        cert_info: cert.clone(),
                                                    },
                                                );
                                                cert
                                            };

                                            // HybridTlsHandler 생성
                                            let hybrid_handler = match HybridTlsHandler::new(
                                                Arc::clone(&self.ca),
                                                self.ctx.tls_event_sender.clone(),
                                                self.ctx.tls_config.clone(),
                                            )
                                            .await
                                            {
                                                Ok(handler) => handler,
                                                Err(e) => {
                                                    error!(
                                                        error = %e,
                                                        "HybridTlsHandler 생성 실패"
                                                    );
                                                    return;
                                                }
                                            };

                                            // 하이브리드 TLS 연결 처리
                                            match hybrid_handler
                                                .handle_tls_connection_upgraded(
                                                    &authority,
                                                    upgraded,
                                                    &full_buffer,
                                                    upstream_cert.as_ref(),
                                                )
                                                .await
                                            {
                                                Ok(hybrid_stream) => {
                                                    info!(
                                                        %version,
                                                        "하이브리드 TLS 연결 성공"
                                                    );
                                                    let stream = TokioIo::new(hybrid_stream);

                                                    if let Err(e) = self
                                                        .serve_stream(
                                                            stream,
                                                            Scheme::HTTPS,
                                                            authority.clone(),
                                                        )
                                                        .await
                                                    {
                                                        error!("HTTPS connect error: {}", e);
                                                    }
                                                }
                                                Err(e) => {
                                                    // 오류 메시지에서 TLS 백엔드 확인
                                                    let error_str = e.to_string();
                                                    let tls_backend =
                                                        if error_str.contains("rustls") {
                                                            "RUSTLS"
                                                        } else if error_str.contains("native-tls")
                                                            || error_str.contains("openssl")
                                                        {
                                                            "NATIVE-TLS"
                                                        } else {
                                                            "UNKNOWN"
                                                        };

                                                    let tls_hint = if error_str.contains(
                                                        "SignatureAlgorithmsExtensionRequired",
                                                    ) {
                                                        "서버가 SignatureAlgorithmsExtension을 요구함 - TLS 1.2+ 클라이언트 사용 또는 서버 설정 확인"
                                                    } else if error_str
                                                        .contains("peer is incompatible")
                                                    {
                                                        "클라이언트-서버 호환성 문제 - 지원하지 않는 TLS 버전, 암호화 스위트, 또는 확장"
                                                    } else if error_str.contains("certificate") {
                                                        "인증서 관련 오류 - 인증서 검증 실패, 만료된 인증서, 또는 CA 신뢰 문제"
                                                    } else {
                                                        ""
                                                    };

                                                    error!(
                                                        authority = %authority,
                                                        tls_version = %version,
                                                        tls_backend,
                                                        error = %e,
                                                        error_type = ?e,
                                                        tls_hint,
                                                        "하이브리드 TLS 연결 실패"
                                                    );

                                                    // 자동 학습: 실패한 도메인 기록
                                                    if let Some(ref passthrough) =
                                                        self.ctx.tls_passthrough
                                                    {
                                                        passthrough
                                                            .record_failure(&authority)
                                                            .await;
                                                    }

                                                    return;
                                                }
                                            }
                                        }
                                        None => {
                                            warn!("TLS 버전을 감지할 수 없음, 기존 rustls로 시도");

                                            // 기존 rustls 로직 사용
                                            let server_config = self
                                                .ca
                                                .gen_server_config(&authority, None)
                                                .instrument(info_span!("gen_server_config"))
                                                .await;

                                            let stream = match TlsAcceptor::from(server_config)
                                                .accept(upgraded)
                                                .await
                                            {
                                                Ok(stream) => TokioIo::new(stream),
                                                Err(e) => {
                                                    let error_str = e.to_string();
                                                    let tls_hint = if error_str.contains(
                                                        "SignatureAlgorithmsExtensionRequired",
                                                    ) {
                                                        "서버가 SignatureAlgorithmsExtension을 요구함 - TLS 1.2+ 클라이언트 사용 또는 서버 설정 확인"
                                                    } else if error_str
                                                        .contains("peer is incompatible")
                                                    {
                                                        "클라이언트-서버 호환성 문제 - 지원하지 않는 TLS 버전, 암호화 스위트, 또는 확장"
                                                    } else if error_str.contains("certificate") {
                                                        "인증서 관련 오류 - 인증서 검증 실패, 만료된 인증서, 또는 CA 신뢰 문제"
                                                    } else if error_str.contains("handshake") {
                                                        "핸드셰이크 프로토콜 오류 - 프로토콜 버전 불일치, 암호화 스위트 협상 실패"
                                                    } else if error_str.contains("timeout") {
                                                        "핸드셰이크 타임아웃 - 네트워크 지연, 서버 과부하, 또는 방화벽 차단"
                                                    } else {
                                                        ""
                                                    };

                                                    error!(
                                                        authority = %authority,
                                                        error = %e,
                                                        error_type = ?e,
                                                        tls_hint,
                                                        "TLS 핸드셰이크 실패"
                                                    );

                                                    // 자동 학습: 실패한 도메인 기록
                                                    if let Some(ref passthrough) =
                                                        self.ctx.tls_passthrough
                                                    {
                                                        passthrough
                                                            .record_failure(&authority)
                                                            .await;
                                                    }

                                                    return;
                                                }
                                            };

                                            if let Err(e) = self
                                                .serve_stream(
                                                    stream,
                                                    Scheme::HTTPS,
                                                    authority.clone(),
                                                )
                                                .await
                                            {
                                                error!("HTTPS connect error: {}", e);
                                            }
                                        }
                                    }

                                    return;
                                } else {
                                    warn!(
                                        "Unknown protocol, read '{:02X?}' from upgraded connection",
                                        &full_buffer[..full_buffer.len().min(16)]
                                    );
                                }
                            }

                            let mut server = match connect_to_target(
                                authority.as_ref(),
                                self.ctx.upstream_proxy.as_ref(),
                            )
                            .await
                            {
                                Ok(server) => server,
                                Err(e) => {
                                    error!(
                                        authority = %authority,
                                        error = %e,
                                        "업스트림 서버 연결 실패"
                                    );
                                    return;
                                }
                            };

                            let throttle_config = self
                                .ctx
                                .throttle_rx
                                .as_ref()
                                .and_then(|rx| rx.borrow().clone());
                            let tunnel_result = if let Some(ref config) = throttle_config {
                                if config.enabled {
                                    throttle::copy_bidirectional_throttled(
                                        &mut upgraded,
                                        &mut server,
                                        config,
                                    )
                                    .await
                                } else {
                                    tokio::io::copy_bidirectional(&mut upgraded, &mut server).await
                                }
                            } else {
                                tokio::io::copy_bidirectional(&mut upgraded, &mut server).await
                            };
                            if let Err(e) = tunnel_result {
                                error!(
                                    authority = %authority,
                                    error = %e,
                                    "터널링 실패"
                                );
                            }
                        }
                        Err(e) => {
                            error!(error = %e, "연결 업그레이드 실패");
                        }
                    };
                };

                spawn_with_trace(fut, span);
                Response::new(Body::empty())
            }
            None => bad_request(),
        }
    }

    pub(crate) async fn serve_stream<I>(
        self,
        stream: I,
        scheme: Scheme,
        authority: Authority,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        I: hyper::rt::Read + hyper::rt::Write + Unpin + Send + 'static,
    {
        debug!(
            %authority,
            ?scheme,
            "[SERVE-STREAM] 스트림 서빙 시작"
        );

        let proxy_clone = self.clone();
        let service = service_fn({
            let authority = authority.clone();
            let scheme = scheme.clone();
            move |mut req| {
                debug!(
                    method = %req.method(),
                    uri = %req.uri(),
                    version = ?req.version(),
                    "[SERVE-STREAM] HTTP 요청 수신"
                );

                if req.version() == hyper::Version::HTTP_10
                    || req.version() == hyper::Version::HTTP_11
                {
                    let (mut parts, body) = req.into_parts();

                    parts.uri = {
                        let mut uri_parts = parts.uri.into_parts();
                        uri_parts.scheme = Some(scheme.clone());
                        uri_parts.authority = Some(authority.clone());
                        match Uri::from_parts(uri_parts) {
                            Ok(uri) => uri,
                            Err(e) => {
                                warn!("URI 재구성 실패: {}", e);
                                match Uri::builder()
                                    .scheme(scheme.clone())
                                    .authority(authority.clone())
                                    .path_and_query("/")
                                    .build()
                                {
                                    Ok(fallback) => fallback,
                                    Err(e2) => {
                                        error!("URI fallback 생성도 실패: {}", e2);
                                        Uri::builder()
                                            .path_and_query("/")
                                            .build()
                                            .unwrap_or_default()
                                    }
                                }
                            }
                        }
                    };

                    req = Request::from_parts(parts, body);
                    debug!(uri = %req.uri(), "[SERVE-STREAM] URI 재구성 완료");
                };

                debug!("[SERVE-STREAM] 프록시 요청 전달 시작");
                proxy_clone.clone().proxy(req)
            }
        });

        debug!("[SERVE-STREAM] 서버 연결 시작 - serve_connection_with_upgrades 호출");
        let result = self
            .server
            .serve_connection_with_upgrades(stream, service)
            .await;

        match result {
            Ok(_) => {
                debug!("[SERVE-STREAM] 스트림 서빙 완료: {}", authority);
                Ok(())
            }
            Err(e) => {
                // 에러 소스 체인을 모두 수집
                let mut error_chain = format!("{}", e);
                let mut source: Option<&dyn std::error::Error> = e.source();
                while let Some(s) = source {
                    error_chain.push_str(&format!(" → {}", s));
                    source = s.source();
                }

                let is_benign = error_chain.contains("error shutting down connection")
                    || error_chain.contains("close_notify")
                    || error_chain.contains("connection reset")
                    || error_chain.contains("broken pipe");

                if is_benign {
                    debug!(
                        %authority,
                        error_chain,
                        "[SERVE-STREAM] 연결 종료 (정상)"
                    );
                    Ok(())
                } else {
                    error!(
                        %authority,
                        error_chain,
                        "[SERVE-STREAM] 스트림 서빙 실패"
                    );
                    Err(e)
                }
            }
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
        ) -> Arc<ServerConfig> {
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

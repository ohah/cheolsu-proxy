use bytes::Bytes;
use futures_util::StreamExt;
use http_body_util::BodyExt;
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client},
    rt::TokioExecutor,
};
use proxyapi_v2::{
    builder::ProxyBuilder,
    certificate_authority::build_ca,
    hyper::{Request, Response},
    tokio_tungstenite::tungstenite::Message,
    Body, HttpContext, HttpHandler, RequestOrResponse, WebSocketContext, WebSocketHandler,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tauri::Emitter;
use tokio::net::TcpListener;
use tokio::sync::oneshot::Sender;
use tokio::sync::Mutex;
use tokio_rustls::rustls::{crypto::aws_lc_rs, ClientConfig};

/// HTTP와 HTTPS를 모두 처리할 수 있는 커스텀 클라이언트 생성
fn create_http_https_client(
) -> Result<Client<hyper_rustls::HttpsConnector<HttpConnector>, Body>, Box<dyn std::error::Error>> {
    // 모든 인증서를 허용하는 Rustls 설정 생성
    let rustls_config =
        ClientConfig::builder_with_provider(std::sync::Arc::new(aws_lc_rs::default_provider()))
            .with_safe_default_protocol_versions()?
            .dangerous()
            .with_custom_certificate_verifier(std::sync::Arc::new(DangerousCertificateVerifier))
            .with_no_client_auth();

    // HTTP와 HTTPS를 모두 처리할 수 있는 커넥터 생성
    let https = HttpsConnectorBuilder::new()
        .with_tls_config(rustls_config)
        .https_or_http() // HTTP와 HTTPS 모두 지원
        .enable_http1() // HTTP/1.1 지원
        .build();

    Ok(Client::builder(TokioExecutor::new())
        .http1_title_case_headers(true)
        .http1_preserve_header_case(true)
        .build(https))
}

/// 모든 인증서를 허용하는 위험한 인증서 검증기
#[derive(Debug)]
struct DangerousCertificateVerifier;

impl tokio_rustls::rustls::client::danger::ServerCertVerifier for DangerousCertificateVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &tokio_rustls::rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[tokio_rustls::rustls::pki_types::CertificateDer<'_>],
        _server_name: &tokio_rustls::rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: tokio_rustls::rustls::pki_types::UnixTime,
    ) -> Result<tokio_rustls::rustls::client::danger::ServerCertVerified, tokio_rustls::rustls::Error>
    {
        // 모든 인증서를 허용
        Ok(tokio_rustls::rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &tokio_rustls::rustls::pki_types::CertificateDer<'_>,
        _dss: &tokio_rustls::rustls::DigitallySignedStruct,
    ) -> Result<
        tokio_rustls::rustls::client::danger::HandshakeSignatureValid,
        tokio_rustls::rustls::Error,
    > {
        // 모든 서명을 허용
        Ok(tokio_rustls::rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &tokio_rustls::rustls::pki_types::CertificateDer<'_>,
        _dss: &tokio_rustls::rustls::DigitallySignedStruct,
    ) -> Result<
        tokio_rustls::rustls::client::danger::HandshakeSignatureValid,
        tokio_rustls::rustls::Error,
    > {
        // 모든 서명을 허용
        Ok(tokio_rustls::rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<tokio_rustls::rustls::SignatureScheme> {
        // 모든 서명 스키마 지원
        vec![
            tokio_rustls::rustls::SignatureScheme::RSA_PKCS1_SHA1,
            tokio_rustls::rustls::SignatureScheme::ECDSA_SHA1_Legacy,
            tokio_rustls::rustls::SignatureScheme::RSA_PKCS1_SHA256,
            tokio_rustls::rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            tokio_rustls::rustls::SignatureScheme::RSA_PKCS1_SHA384,
            tokio_rustls::rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            tokio_rustls::rustls::SignatureScheme::RSA_PKCS1_SHA512,
            tokio_rustls::rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            tokio_rustls::rustls::SignatureScheme::RSA_PSS_SHA256,
            tokio_rustls::rustls::SignatureScheme::RSA_PSS_SHA384,
            tokio_rustls::rustls::SignatureScheme::RSA_PSS_SHA512,
            tokio_rustls::rustls::SignatureScheme::ED25519,
            tokio_rustls::rustls::SignatureScheme::ED448,
        ]
    }
}

/// HTTP 및 WebSocket 요청/응답을 로깅하는 핸들러
#[derive(Clone)]
pub struct LoggingHandler {
    app_handle: tauri::AppHandle,
}

impl LoggingHandler {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self { app_handle }
    }

    /// 에러 응답을 프론트엔드로 전송
    fn emit_error(&self, error_type: &str, details: &str) {
        let error_info = format!("{}: {}", error_type, details);
        let _ = self.app_handle.emit("proxy_error", &error_info);
        eprintln!("{}", error_info);
    }

    /// Body를 Bytes로 변환하는 헬퍼 함수
    async fn body_to_bytes(body: Body) -> Result<Bytes, Box<dyn std::error::Error + Send + Sync>> {
        let mut body_stream = body.into_data_stream();
        let mut bytes = Vec::new();

        while let Some(chunk_result) = body_stream.next().await {
            let chunk: Bytes = chunk_result?;
            bytes.extend_from_slice(&chunk);
        }

        Ok(Bytes::from(bytes))
    }
}

impl HttpHandler for LoggingHandler {
    async fn handle_request(
        &mut self,
        _ctx: &HttpContext,
        req: Request<Body>,
    ) -> RequestOrResponse {
        // 특정 URL 요청을 무조건 실패시키기
        if let Some(authority) = req.uri().authority() {
            if authority.host().contains("img.battlepage.com")
                && req.uri().path().contains("/icon/3/3765.png")
            {
                println!("🚫 [BLOCKED] 특정 이미지 요청 차단: {}", req.uri());

                // 404 Not Found 응답 생성
                let error_response = Response::builder()
                    .status(404)
                    .header("content-type", "text/plain")
                    .body(Body::from("Image not found - blocked by proxy"))
                    .unwrap();

                // 에러 정보를 프론트엔드로 전송
                let error_msg = format!("🚫 차단된 요청: {}", req.uri());
                let _ = self.app_handle.emit("proxy_error", error_msg);

                return error_response.into();
            }
        }

        // 요청 정보를 프론트엔드로 전송
        let _ = self.app_handle.emit("proxy_request", format!("{:?}", req));

        // img.battlepage.com 관련 요청만 로깅
        if let Some(authority) = req.uri().authority() {
            if authority.host().contains("battlepage.com") {
                println!("=== HTTP 요청 상세 (battlepage.com) ===");
                println!("Method: {}", req.method());
                println!("URI: {}", req.uri());
                println!("Headers: {:?}", req.headers());

                // 요청 타입별 추가 정보
                match req.method().as_str() {
                    "CONNECT" => {
                        println!("🔗 CONNECT 요청 - 터널 연결 시도");
                        println!("   - 대상 서버: {}", authority);
                    }
                    "GET" | "POST" | "PUT" | "DELETE" => {
                        println!("📡 HTTP 요청 - 프록시 처리");
                        println!("   - 대상 서버: {}", authority);
                        println!("   - 요청 경로: {}", req.uri().path());
                    }
                    _ => {
                        println!("❓ 기타 HTTP 메서드: {}", req.method());
                    }
                }
            }
        }

        req.into()
    }

    async fn handle_response(&mut self, _ctx: &HttpContext, res: Response<Body>) -> Response<Body> {
        // 응답 정보를 미리 저장
        let status = res.status();
        let version = res.version();
        let headers = res.headers().clone();

        // 응답 body를 읽어서 Bytes로 변환
        let (parts, body) = res.into_parts();
        let body_bytes = match Self::body_to_bytes(body).await {
            Ok(bytes) => bytes,
            Err(e) => {
                eprintln!("❌ 응답 body 읽기 실패: {}", e);
                Bytes::new()
            }
        };

        // 응답 정보를 구조화된 형태로 프론트엔드에 전송
        let response_info = serde_json::json!({
            "status": parts.status.as_u16(),
            "status_text": parts.status.canonical_reason().unwrap_or("Unknown"),
            "headers": parts.headers.iter().map(|(k, v)| {
                (k.as_str(), v.to_str().unwrap_or(""))
            }).collect::<std::collections::HashMap<_, _>>(),
            "version": format!("{:?}", parts.version),
            "body": {
                "size": body_bytes.len(),
                "content": body_bytes.to_vec()
            }
        });

        let _ = self.app_handle.emit("proxy_response", &response_info);

        // battlepage.com 관련 응답만 로깅 (URI 정보가 없으므로 항상 로깅)
        println!("=== HTTP 응답 상세 (battlepage.com) ===");
        println!(
            "Status: {} ({})",
            status,
            status.canonical_reason().unwrap_or("Unknown")
        );
        println!("Headers: {:?}", headers);

        // 응답 버전 정보 추가
        println!("Response Version: {:?}", version);

        // 응답 본문 크기 확인
        if let Some(content_length) = headers.get("content-length") {
            if let Ok(len) = content_length.to_str() {
                if let Ok(len_num) = len.parse::<usize>() {
                    println!("Response Content-Length: {} bytes", len_num);
                }
            }
        }

        // 응답 본문 타입 정보 로깅
        if let Some(content_type) = headers.get("content-type") {
            if let Ok(ct) = content_type.to_str() {
                println!("Content-Type: {}", ct);

                // 특정 타입의 응답에 대한 추가 정보
                if ct.contains("text/html") {
                    println!("📄 HTML 응답");
                } else if ct.contains("application/json") {
                    println!("📊 JSON 응답");
                } else if ct.contains("image/") {
                    println!("🖼️ 이미지 응답");
                } else if ct.contains("text/css") {
                    println!("🎨 CSS 응답");
                } else if ct.contains("application/javascript") {
                    println!("⚡ JavaScript 응답");
                }
            }
        }

        // 특정 에러 상태 코드 상세 분석
        match status.as_u16() {
            502 => {
                let error_msg = "🚨 502 Bad Gateway: 프록시가 업스트림 서버에 연결할 수 없음";
                eprintln!("{}", error_msg);
                let _ = self.app_handle.emit("proxy_error", error_msg);

                // 502 에러 추가 정보
                println!("   - 가능한 원인:");
                println!("     * CA 인증서 문제");
                println!("     * 대상 서버 연결 실패");
                println!("     * 네트워크 타임아웃");
                println!("     * 프록시 설정 오류");
                println!("     * 도메인별 인증서 생성 실패");
                println!("     * 응답 스트림 처리 문제");

                // 현재 요청 정보 출력
                println!("   - 현재 요청 도메인: {}", _ctx.client_addr);
            }
            503 => {
                let error_msg = "⚠️ 503 Service Unavailable: 서비스 일시적 사용 불가";
                eprintln!("{}", error_msg);
                let _ = self.app_handle.emit("proxy_error", error_msg);
            }
            504 => {
                let error_msg = "⏰ 504 Gateway Timeout: 프록시 연결 타임아웃";
                eprintln!("{}", error_msg);
                let _ = self.app_handle.emit("proxy_error", error_msg);
            }
            _ => {
                if status.is_client_error() || status.is_server_error() {
                    let error_msg = format!(
                        "❌ HTTP 에러 {}: {}",
                        status,
                        status.canonical_reason().unwrap_or("Unknown")
                    );
                    eprintln!("{}", error_msg);
                    let _ = self.app_handle.emit("proxy_error", error_msg);
                } else {
                    println!("✅ 정상 응답: {}", status);

                    // 정상 응답의 경우 추가 정보 로깅
                    if let Some(content_type) = headers.get("content-type") {
                        if let Ok(ct) = content_type.to_str() {
                            if ct.contains("image/") {
                                println!("🖼️ 이미지 응답 - 브라우저에서 표시 가능해야 함");
                            }
                        }
                    }
                }
            }
        }

        // 응답 처리 완료 로깅
        println!("📤 응답을 클라이언트에게 전달 중...");
        println!("   - 응답 상태: {}", parts.status);
        println!("   - 응답 헤더 수: {}", parts.headers.len());
        println!("   - 응답 버전: {:?}", parts.version);
        println!("   - 응답 body 크기: {} bytes", body_bytes.len());
        println!("==========================================");

        // 원본 응답을 body와 함께 재구성하여 반환
        use http_body_util::Full;
        Response::from_parts(parts, Body::from(Full::new(body_bytes)))
    }
}

impl WebSocketHandler for LoggingHandler {
    async fn handle_message(&mut self, _ctx: &WebSocketContext, msg: Message) -> Option<Message> {
        // WebSocket 메시지를 프론트엔드로 전송
        let _ = self
            .app_handle
            .emit("proxy_websocket", format!("{:?}", msg));
        Some(msg)
    }
}

/// hudsucker를 사용하는 프록시 상태 (proxy.rs와 유사한 구조)
pub type ProxyV2State = Arc<Mutex<Option<(Sender<()>, tauri::async_runtime::JoinHandle<()>)>>>;

/// hudsucker 프록시 시작 (실제 프록시 서버 실행)
#[tauri::command]
pub async fn start_proxy_v2(
    app: tauri::AppHandle,
    proxy: tauri::State<'_, ProxyV2State>,
    addr: SocketAddr,
) -> Result<(), String> {
    // CA 인증서 생성 (proxyapi_v2의 build_ca 함수 사용)
    let ca = match build_ca() {
        Ok(ca) => {
            println!("✅ 기존 CA 인증서 로드 완료");
            ca
        }
        Err(e) => {
            let error_msg = format!("❌ CA 인증서 생성 실패: {}", e);
            eprintln!("{}", error_msg);
            return Err(error_msg);
        }
    };

    // 로깅 핸들러 생성
    let handler = LoggingHandler::new(app.clone());

    // TCP 리스너 생성
    let listener = match TcpListener::bind(addr).await {
        Ok(listener) => {
            println!("✅ 포트 {}에서 TCP 리스너 시작됨", addr.port());
            listener
        }
        Err(e) => {
            let error_msg = format!("❌ 포트 {} 바인딩 실패: {}", addr.port(), e);
            eprintln!("{}", error_msg);
            return Err(error_msg);
        }
    };

    // HTTP와 HTTPS를 모두 처리할 수 있는 커스텀 클라이언트 생성
    let custom_client = match create_http_https_client() {
        Ok(client) => {
            println!("✅ HTTP/HTTPS 모두 지원하는 커스텀 클라이언트 생성 완료");
            client
        }
        Err(e) => {
            let error_msg = format!("❌ 커스텀 클라이언트 생성 실패: {}", e);
            eprintln!("{}", error_msg);
            return Err(error_msg);
        }
    };

    // 프록시 빌더로 프록시 구성
    let proxy_builder = match ProxyBuilder::new()
        .with_listener(listener)
        .with_ca(ca)
        .with_client(custom_client) // 커스텀 클라이언트 사용 (모든 인증서 허용)
        .with_http_handler(handler.clone())
        .with_websocket_handler(handler.clone())
        .build()
    {
        Ok(builder) => {
            println!("✅ 프록시 빌더 구성 완료");
            println!("   - CA 인증서: 로드됨");
            println!("   - TLS 클라이언트: 커스텀 클라이언트 (모든 인증서 허용)");
            println!("   - HTTP 핸들러: 로깅 핸들러");
            println!("   - WebSocket 핸들러: 로깅 핸들러");
            builder
        }
        Err(e) => {
            let error_msg = format!("❌ 프록시 빌드 실패: {}", e);
            eprintln!("{}", error_msg);
            return Err(error_msg);
        }
    };

    // 종료 신호를 위한 채널 생성
    let (close_tx, _close_rx) = tokio::sync::oneshot::channel();

    // 프록시를 백그라운드에서 실행
    let app_handle = app.clone();
    let thread = tauri::async_runtime::spawn(async move {
        println!("🚀 프록시 서버 시작 중...");
        match proxy_builder.start().await {
            Ok(_) => println!("✅ 프록시 서버가 정상적으로 종료되었습니다"),
            Err(e) => {
                let error_msg = format!("❌ 프록시 실행 오류: {}", e);
                eprintln!("{}", error_msg);
                // 에러를 프론트엔드로 전송
                let _ = app_handle.emit("proxy_error", error_msg);
            }
        }
    });

    // 프록시 상태 업데이트
    let mut proxy_guard = proxy.lock().await;
    proxy_guard.replace((close_tx, thread));

    println!(
        "🎉 프록시 V2가 포트 {}에서 성공적으로 시작되었습니다",
        addr.port()
    );
    println!(
        "📋 시스템 프록시 설정을 127.0.0.1:{}로 변경하세요",
        addr.port()
    );

    Ok(())
}

/// hudsucker 프록시 중지 (실제 프록시 서버 중지)
#[tauri::command]
pub async fn stop_proxy_v2(proxy: tauri::State<'_, ProxyV2State>) -> Result<(), String> {
    let mut proxy_guard = proxy.lock().await;

    if let Some((close_tx, thread)) = proxy_guard.take() {
        // 종료 신호 전송 (oneshot 채널은 한 번만 사용 가능)
        match close_tx.send(()) {
            Ok(_) => {
                println!("✅ 프록시 종료 신호 전송 성공");
            }
            Err(_) => {
                // 이미 사용된 채널이거나 수신자가 이미 종료된 경우
                println!("⚠️ 프록시 종료 신호 전송 실패 (이미 종료 중이거나 완료됨)");
            }
        }

        // 스레드 종료 대기 (타임아웃 설정)
        match tokio::time::timeout(std::time::Duration::from_secs(5), thread).await {
            Ok(result) => match result {
                Ok(_) => println!("✅ 프록시 V2가 정상적으로 중지되었습니다"),
                Err(e) => {
                    let error_msg = format!("❌ 프록시 스레드 종료 실패: {}", e);
                    eprintln!("{}", error_msg);
                    return Err(error_msg);
                }
            },
            Err(_) => {
                println!("⏰ 프록시 종료 타임아웃 (5초), 강제 종료");
            }
        }

        println!("📋 시스템 프록시 설정을 해제하세요");
    } else {
        return Err("프록시가 실행 중이 아닙니다".to_string());
    }

    Ok(())
}

/// hudsucker 프록시 상태 확인 (실제 프록시 상태 확인)
#[tauri::command]
pub async fn proxy_v2_status(proxy: tauri::State<'_, ProxyV2State>) -> Result<bool, String> {
    Ok(proxy.lock().await.is_some())
}

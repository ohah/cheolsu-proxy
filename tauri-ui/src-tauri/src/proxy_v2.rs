use bytes::Bytes;
use futures_util::StreamExt;
use http_body_util::BodyExt;
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client},
    rt::TokioExecutor,
};
use proxy_v2_models::{ProxiedRequest, ProxiedResponse, RequestInfo};
use proxyapi_v2::{
    builder::ProxyBuilder,
    certificate_authority::build_ca,
    hyper::{Request, Response},
    tokio_tungstenite::tungstenite::Message,
    Body, HttpContext, HttpHandler, RequestOrResponse, WebSocketContext, WebSocketHandler,
};
use std::net::SocketAddr;
use std::sync::mpsc;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime, State};
use tauri_plugin_store::{JsonValue, StoreExt};
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
            tokio_rustls::rustls::SignatureScheme::ML_DSA_44,
            tokio_rustls::rustls::SignatureScheme::ML_DSA_65,
            tokio_rustls::rustls::SignatureScheme::ML_DSA_87,
        ]
    }
}

/// HTTP 및 WebSocket 요청/응답을 로깅하는 핸들러
#[derive(Clone)]
pub struct LoggingHandler {
    sender: mpsc::SyncSender<RequestInfo>,
    req: Option<ProxiedRequest>,
    res: Option<ProxiedResponse>,
    sessions: Arc<Mutex<JsonValue>>,
}

impl LoggingHandler {
    pub fn new(sender: mpsc::SyncSender<RequestInfo>) -> Self {
        Self {
            sender,
            req: None,
            res: None,
            sessions: Arc::new(Mutex::new(JsonValue::Array(Vec::new()))),
        }
    }

    /// 세션 데이터 업데이트
    pub async fn update_sessions(&self, sessions: JsonValue) {
        let mut sessions_guard = self.sessions.lock().await;
        *sessions_guard = sessions;
    }

    /// 요청 URL이 세션에 있는지 확인하고 더미 데이터 반환 여부 결정
    async fn should_return_dummy_data(&self, url: &str) -> bool {
        let sessions_guard = self.sessions.lock().await;
        if let JsonValue::Array(sessions) = &*sessions_guard {
            sessions.iter().any(|session| {
                if let Some(session_url) = session.get("url").and_then(|v| v.as_str()) {
                    url.contains(session_url) || session_url.contains(url)
                } else {
                    false
                }
            })
        } else {
            false
        }
    }

    /// 더미 응답 생성
    fn create_dummy_response(&self, req: &Request<Body>) -> Response<Body> {
        let mut response = Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .header("Access-Control-Allow-Origin", "*")
            .header(
                "Access-Control-Allow-Methods",
                "GET, POST, PUT, DELETE, OPTIONS",
            )
            .header(
                "Access-Control-Allow-Headers",
                "Content-Type, Authorization",
            );

        // 요청의 헤더를 응답에 복사 (CORS 등)
        for (key, value) in req.headers() {
            if key.as_str().starts_with("access-control-") {
                response = response.header(key, value);
            }
        }

        let dummy_data = serde_json::json!({
            "message": "더미 데이터입니다 (proxy_v2)",
            "timestamp": chrono::Local::now().to_rfc3339(),
            "url": req.uri().to_string(),
            "method": req.method().to_string(),
            "proxy_version": "v2"
        });

        response
            .body(Body::from(dummy_data.to_string()))
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(500)
                    .body(Body::from("Internal Server Error"))
                    .unwrap()
            })
    }

    /// 요청과 응답을 묶어서 전송
    fn send_output(&self) {
        let request_info = RequestInfo(self.req.clone(), self.res.clone());
        if let Err(e) = self.sender.send(request_info) {
            eprintln!("Error on sending RequestInfo to main thread: {}", e);
        }
    }

    /// Request를 ProxiedRequest로 변환
    fn request_to_proxied_request(&self, req: &Request<Body>) -> ProxiedRequest {
        // 요청 body를 읽어서 Bytes로 변환 (비동기이므로 여기서는 빈 body로 설정)
        ProxiedRequest::new(
            req.method().clone(),
            req.uri().clone(),
            req.version(),
            req.headers().clone(),
            Bytes::new(), // TODO: 실제 body 읽기
            chrono::Local::now()
                .timestamp_nanos_opt()
                .unwrap_or_default(),
        )
    }

    /// Response를 ProxiedResponse로 변환하고 원본 응답을 복원
    async fn response_to_proxied_response(
        &self,
        mut res: Response<Body>,
    ) -> (ProxiedResponse, Response<Body>) {
        // 응답 body를 읽어서 Bytes로 변환
        let mut body_mut = res.body_mut();
        let body_bytes = match Self::body_to_bytes_from_mut(&mut body_mut).await {
            Ok(bytes) => bytes,
            Err(e) => {
                eprintln!("❌ 응답 body 읽기 실패: {}", e);
                Bytes::new()
            }
        };

        // 원본 body 복원
        use http_body_util::Full;
        *body_mut = Body::from(Full::new(body_bytes.clone()));

        let proxied_response = ProxiedResponse::new(
            res.status(),
            res.version(),
            res.headers().clone(),
            body_bytes,
            chrono::Local::now()
                .timestamp_nanos_opt()
                .unwrap_or_default(),
        );

        (proxied_response, res)
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

    /// BodyMut를 Bytes로 변환하는 헬퍼 함수 (기존 proxyapi 방식)
    async fn body_to_bytes_from_mut(
        body_mut: &mut Body,
    ) -> Result<Bytes, Box<dyn std::error::Error + Send + Sync>> {
        use http_body_util::BodyExt;
        let body_bytes = body_mut.collect().await?.to_bytes();
        Ok(body_bytes)
    }
}

impl HttpHandler for LoggingHandler {
    async fn handle_request(
        &mut self,
        _ctx: &HttpContext,
        req: Request<Body>,
    ) -> RequestOrResponse {
        // 요청 정보를 ProxiedRequest로 변환하여 저장 (전송하지 않음)
        let proxied_request = self.request_to_proxied_request(&req);
        self.req = Some(proxied_request);

        // 세션에 있는 URL인지 확인하고 더미 데이터 반환
        let url = req.uri().to_string();
        if self.should_return_dummy_data(&url).await {
            println!("🎭 더미 데이터 반환: {}", url);
            let dummy_response = self.create_dummy_response(&req);

            // 더미 응답을 ProxiedResponse로 변환하여 저장
            let dummy_proxied_response = ProxiedResponse::new(
                dummy_response.status(),
                dummy_response.version(),
                dummy_response.headers().clone(),
                Bytes::from("더미 데이터입니다 (proxy_v2)"),
                chrono::Local::now()
                    .timestamp_nanos_opt()
                    .unwrap_or_default(),
            );
            self.res = Some(dummy_proxied_response);

            // 요청과 응답을 묶어서 전송
            self.send_output();

            return dummy_response.into();
        }

        req.into()
    }

    async fn handle_response(&mut self, _ctx: &HttpContext, res: Response<Body>) -> Response<Body> {
        // 응답 정보를 ProxiedResponse로 변환하고 원본 응답을 복원
        let (proxied_response, restored_res) = self.response_to_proxied_response(res).await;
        self.res = Some(proxied_response);

        // 요청과 응답을 묶어서 전송
        self.send_output();

        // 원본 응답을 그대로 반환 (기존 proxyapi 방식)
        restored_res
    }
}

impl WebSocketHandler for LoggingHandler {
    async fn handle_message(&mut self, _ctx: &WebSocketContext, msg: Message) -> Option<Message> {
        // WebSocket 메시지는 현재 RequestInfo 구조에 맞지 않으므로 로깅만 수행
        println!("WebSocket Message: {:?}", msg);
        Some(msg)
    }
}

/// hudsucker를 사용하는 프록시 상태 (proxy.rs와 유사한 구조)
pub type ProxyV2State = Arc<
    Mutex<
        Option<(
            Sender<()>,
            tauri::async_runtime::JoinHandle<()>,
            LoggingHandler,
        )>,
    >,
>;

/// hudsucker 프록시 시작 (실제 프록시 서버 실행)
#[tauri::command]
pub async fn start_proxy_v2<R: Runtime>(
    app: AppHandle<R>,
    proxy: State<'_, ProxyV2State>,
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

    // 이벤트 전송을 위한 채널 생성 (proxy.rs와 동일한 구조)
    let (tx, rx) = std::sync::mpsc::sync_channel(1);

    // 세션 데이터 로드 (proxy.rs와 동일한 방식)
    let store = app.store("session.json").map_err(|e| e.to_string())?;
    let sessions = store.get("sessions").unwrap_or_default();

    // 로깅 핸들러 생성
    let handler = LoggingHandler::new(tx.clone());

    // 세션 데이터를 핸들러에 전달
    handler.update_sessions(sessions).await;

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
    proxy_guard.replace((close_tx, thread, handler.clone()));

    // 이벤트 전송을 위한 백그라운드 태스크 (proxy.rs와 동일한 구조)
    tauri::async_runtime::spawn(async move {
        for event in rx.iter() {
            let _ = app.emit("proxy_event", event);
        }
    });

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

    if let Some((close_tx, thread, _handler)) = proxy_guard.take() {
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

/// 세션 데이터 변경 시 호출되는 명령어 (proxy.rs와 동일한 기능)
#[tauri::command]
pub async fn store_changed_v2<R: Runtime>(
    app: AppHandle<R>,
    proxy: State<'_, ProxyV2State>,
) -> Result<(), String> {
    let mut proxy_guard = proxy.lock().await;

    if proxy_guard.is_none() {
        println!("store_changed_v2: Proxy V2가 실행 중이 아니므로 세션 업데이트를 무시합니다");
        return Ok(());
    }

    // 세션 데이터 로드
    let store = app.store("session.json").map_err(|e| e.to_string())?;
    let sessions = store.get("sessions").unwrap_or_default();

    let session_count = if let JsonValue::Array(arr) = &sessions {
        arr.len()
    } else {
        0
    };

    println!(
        "🔄 Proxy V2 세션 데이터 업데이트: {} 개의 세션",
        session_count
    );

    // 핸들러에 세션 데이터 전달
    if let Some((_close_tx, _thread, handler)) = proxy_guard.as_mut() {
        handler.update_sessions(sessions).await;
        println!("✅ Proxy V2 핸들러에 세션 데이터 업데이트 완료");
    }

    Ok(())
}

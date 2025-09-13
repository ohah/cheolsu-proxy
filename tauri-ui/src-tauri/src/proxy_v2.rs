use bytes::Bytes;
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client},
    rt::TokioExecutor,
};
use proxy_v2_models::{ProxiedRequest, ProxiedResponse, RequestInfo};
use proxyapi_v2::{
    builder::ProxyBuilder,
    certificate_authority::build_ca,
    hyper::http::{HeaderMap, StatusCode},
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

    /// 요청 URL이 세션에 있는지 확인하고 매칭되는 세션 반환
    async fn find_matching_session(&self, url: &str, method: &str) -> Option<JsonValue> {
        let sessions_guard = self.sessions.lock().await;
        if let JsonValue::Array(sessions) = &*sessions_guard {
            sessions
                .iter()
                .find(|session| {
                    let session_url = session.get("url").and_then(|v| v.as_str()).unwrap_or("");
                    let session_method = session
                        .get("method")
                        .and_then(|v| v.as_str())
                        .unwrap_or("GET");

                    // URL과 메서드가 모두 매칭되는지 확인
                    (url.contains(session_url) || session_url.contains(url))
                        && session_method.to_uppercase() == method.to_uppercase()
                })
                .cloned()
        } else {
            None
        }
    }

    /// 세션 데이터로부터 HTTP 응답을 생성하는 메서드 (proxyapi와 동일한 로직)
    fn create_response_from_session(&self, response_data: &JsonValue) -> Response<Body> {
        // 상태 코드 추출
        let status_code = response_data
            .get("status")
            .and_then(|v| v.as_u64())
            .unwrap_or(200) as u16;

        // 헤더 추출 (content-length 제외)
        let mut headers: HeaderMap = response_data
            .get("headers")
            .and_then(JsonValue::as_object)
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| {
                        // content-length 헤더는 제외 (실제 본문 길이에 맞게 자동 설정됨)
                        if k.to_lowercase() == "content-length" {
                            return None;
                        }
                        Some((k.parse().ok()?, v.as_str()?.parse().ok()?))
                    })
                    .collect()
            })
            .unwrap_or_default();

        // 기본 Content-Type 헤더 설정 (없는 경우)
        if !headers.contains_key("content-type") {
            headers.insert("content-type", "application/json".parse().unwrap());
        }

        // 세션 응답임을 나타내는 특별한 헤더 추가
        headers.insert("x-cheolsu-proxy-session", "true".parse().unwrap());
        headers.insert("x-cheolsu-proxy-version", "v2".parse().unwrap());

        // 응답 본문 생성
        let body = if let Some(data) = response_data.get("data") {
            println!("🎭 응답 본문 데이터 발견: {:?}", data);
            match data {
                JsonValue::String(s) => {
                    println!("🎭 문자열 데이터: {}", s);
                    Body::from(s.clone())
                }
                JsonValue::Object(_) | JsonValue::Array(_) => {
                    let json_string = serde_json::to_string(data).unwrap_or_default();
                    println!("🎭 JSON 데이터: {}", json_string);
                    Body::from(json_string)
                }
                _ => {
                    let string_data = data.to_string();
                    println!("🎭 기타 데이터: {}", string_data);
                    Body::from(string_data)
                }
            }
        } else {
            println!("🎭 응답 본문 데이터 없음 - 빈 응답 생성");
            Body::empty()
        };

        // 응답 생성
        let mut response = Response::new(body);
        *response.status_mut() = StatusCode::from_u16(status_code).unwrap_or(StatusCode::OK);
        *response.headers_mut() = headers;

        response
    }

    /// 요청과 응답을 묶어서 전송
    fn send_output(&self) {
        let request_info = RequestInfo(self.req.clone(), self.res.clone());
        if let Err(e) = self.sender.send(request_info) {
            eprintln!("Error on sending RequestInfo to main thread: {}", e);
        }
    }

    /// Request를 ProxiedRequest로 변환하고 원본 요청을 복원 (비동기)
    async fn request_to_proxied_request(
        &self,
        mut req: Request<Body>,
    ) -> (ProxiedRequest, Request<Body>) {
        // 요청 body를 읽어서 Bytes로 변환
        let mut body_mut = req.body_mut();
        let body_bytes = match Self::body_to_bytes_from_mut(&mut body_mut).await {
            Ok(bytes) => bytes,
            Err(e) => {
                eprintln!("❌ 요청 body 읽기 실패: {}", e);
                Bytes::new()
            }
        };

        // 원본 body 복원
        use http_body_util::Full;
        *body_mut = Body::from(Full::new(body_bytes.clone()));

        let proxied_request = ProxiedRequest::new(
            req.method().clone(),
            req.uri().clone(),
            req.version(),
            req.headers().clone(),
            body_bytes,
            chrono::Local::now()
                .timestamp_nanos_opt()
                .unwrap_or_default(),
        );

        (proxied_request, req)
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
        // 요청 정보를 ProxiedRequest로 변환하고 원본 요청을 복원
        let (proxied_request, restored_req) = self.request_to_proxied_request(req).await;
        self.req = Some(proxied_request);

        restored_req.into()
    }

    async fn handle_response(&mut self, _ctx: &HttpContext, res: Response<Body>) -> Response<Body> {
        // 세션 응답인지 확인 (x-cheolsu-proxy-session 헤더 체크)
        let is_session_response = res
            .headers()
            .get("x-cheolsu-proxy-session")
            .and_then(|h| h.to_str().ok())
            .map(|s| s == "true")
            .unwrap_or(false);

        if is_session_response {
            println!("🎭 세션 응답 감지됨 - 로깅만 수행");
            // 세션 응답의 경우 로깅만 수행하고 원본 응답을 그대로 반환
            let (proxied_response, restored_res) = self.response_to_proxied_response(res).await;
            self.res = Some(proxied_response);
            self.send_output();
            return restored_res;
        }

        // 일반 응답 처리 - 세션 매칭 확인
        if let Some(req) = &self.req {
            let url = req.uri().to_string();
            let method = req.method().to_string();

            if let Some(session) = self.find_matching_session(&url, &method).await {
                // 세션의 response 데이터 추출
                let default_response = JsonValue::Object(serde_json::Map::new());
                let response_data = session.get("response").unwrap_or(&default_response);
                println!("🎭 응답 데이터: {:?}", response_data);
                let mut session_response = self.create_response_from_session(response_data);

                // 세션 응답의 실제 본문을 가져와서 Bytes로 변환
                let session_body_bytes =
                    match Self::body_to_bytes_from_mut(&mut session_response.body_mut()).await {
                        Ok(bytes) => bytes,
                        Err(e) => {
                            eprintln!("❌ 세션 응답 body 읽기 실패: {}", e);
                            Bytes::from("세션 응답 읽기 실패")
                        }
                    };

                // 세션 응답을 ProxiedResponse로 변환하여 저장
                let session_proxied_response = ProxiedResponse::new(
                    session_response.status(),
                    session_response.version(),
                    session_response.headers().clone(),
                    session_body_bytes.clone(),
                    chrono::Local::now()
                        .timestamp_nanos_opt()
                        .unwrap_or_default(),
                );
                self.res = Some(session_proxied_response);

                // 요청과 응답을 묶어서 전송
                self.send_output();

                // body를 다시 복원하여 반환
                use http_body_util::Full;
                *session_response.body_mut() = Body::from(Full::new(session_body_bytes));
                return session_response;
            }
        }

        // 일반 응답 처리
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

/// 프록시 시작 결과를 나타내는 구조체
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProxyStartResult {
    pub status: bool,
    pub message: String,
}

/// hudsucker 프록시 시작 (실제 프록시 서버 실행)
#[tauri::command]
pub async fn start_proxy_v2<R: Runtime>(
    app: AppHandle<R>,
    proxy: State<'_, ProxyV2State>,
    addr: SocketAddr,
) -> Result<ProxyStartResult, ProxyStartResult> {
    // 이미 프록시가 실행 중인지 확인
    let proxy_guard = proxy.lock().await;
    if proxy_guard.is_some() {
        let already_running_message = format!(
            "프록시 V2가 이미 포트 {}에서 실행 중입니다. 시스템 프록시 설정을 127.0.0.1:{}로 변경하세요",
            addr.port(),
            addr.port()
        );
        println!("ℹ️ {}", already_running_message);
        return Ok(ProxyStartResult {
            status: true,
            message: already_running_message,
        });
    }
    drop(proxy_guard); // 락 해제

    // CA 인증서 생성 (proxyapi_v2의 build_ca 함수 사용)
    let ca = match build_ca() {
        Ok(ca) => {
            println!("✅ 기존 CA 인증서 로드 완료");
            ca
        }
        Err(e) => {
            let error_msg = format!("CA 인증서 생성 실패: {}", e);
            eprintln!("❌ {}", error_msg);
            return Err(ProxyStartResult {
                status: false,
                message: error_msg,
            });
        }
    };

    // 이벤트 전송을 위한 채널 생성 (proxy.rs와 동일한 구조)
    let (tx, rx) = std::sync::mpsc::sync_channel(1);

    // 세션 데이터 로드 (proxy.rs와 동일한 방식)
    let store = match app.store("session.json") {
        Ok(store) => store,
        Err(e) => {
            let error_msg = format!("세션 스토어 로드 실패: {}", e);
            eprintln!("❌ {}", error_msg);
            return Err(ProxyStartResult {
                status: false,
                message: error_msg,
            });
        }
    };
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
            let error_msg = format!("포트 {} 바인딩 실패: {}", addr.port(), e);
            eprintln!("❌ {}", error_msg);
            return Err(ProxyStartResult {
                status: false,
                message: error_msg,
            });
        }
    };

    // HTTP와 HTTPS를 모두 처리할 수 있는 커스텀 클라이언트 생성
    let custom_client = match create_http_https_client() {
        Ok(client) => {
            println!("✅ HTTP/HTTPS 모두 지원하는 커스텀 클라이언트 생성 완료");
            client
        }
        Err(e) => {
            let error_msg = format!("커스텀 클라이언트 생성 실패: {}", e);
            eprintln!("❌ {}", error_msg);
            return Err(ProxyStartResult {
                status: false,
                message: error_msg,
            });
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
            let error_msg = format!("프록시 빌드 실패: {}", e);
            eprintln!("❌ {}", error_msg);
            return Err(ProxyStartResult {
                status: false,
                message: error_msg,
            });
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

    let success_message = format!(
        "프록시 V2가 포트 {}에서 성공적으로 시작되었습니다. 시스템 프록시 설정을 127.0.0.1:{}로 변경하세요",
        addr.port(),
        addr.port()
    );

    println!("🎉 {}", success_message);
    Ok(ProxyStartResult {
        status: true,
        message: success_message,
    })
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

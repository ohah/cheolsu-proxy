use proxyapi_v2::{
    builder::ProxyBuilder,
    certificate_authority::RcgenAuthority,
    hyper::{Request, Response},
    rcgen::{CertificateParams, KeyPair},
    rustls::crypto::aws_lc_rs,
    tokio_tungstenite::tungstenite::Message,
    Body, HttpContext, HttpHandler, RequestOrResponse, WebSocketContext, WebSocketHandler,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

/// HTTP 및 WebSocket 요청/응답을 로깅하는 핸들러
#[derive(Clone)]
pub struct LoggingHandler {
    app_handle: tauri::AppHandle,
}

impl LoggingHandler {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self { app_handle }
    }
}

impl HttpHandler for LoggingHandler {
    async fn handle_request(
        &mut self,
        _ctx: &HttpContext,
        req: Request<Body>,
    ) -> RequestOrResponse {
        // 요청 정보를 프론트엔드로 전송
        let _ = self.app_handle.emit("proxy_request", format!("{:?}", req));
        req.into()
    }

    async fn handle_response(&mut self, _ctx: &HttpContext, res: Response<Body>) -> Response<Body> {
        // 응답 정보를 프론트엔드로 전송
        let _ = self.app_handle.emit("proxy_response", format!("{:?}", res));
        res
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

/// hudsucker를 사용하는 프록시 상태
pub struct ProxyV2State(Arc<Mutex<bool>>);

impl Default for ProxyV2State {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(false)))
    }
}

impl ProxyV2State {
    pub async fn set_running(&self, running: bool) {
        let mut guard = self.0.lock().await;
        *guard = running;
    }

    pub async fn get_running(&self) -> bool {
        let guard = self.0.lock().await;
        *guard
    }
}

/// CA 인증서 생성
fn build_ca() -> Result<RcgenAuthority, String> {
    // 키 페어 생성
    let key_pair = KeyPair::generate().map_err(|e| format!("키 페어 생성 실패: {}", e))?;

    // 인증서 파라미터 설정
    let mut cert_params = CertificateParams::new(vec!["localhost".to_string()])
        .map_err(|e| format!("인증서 파라미터 생성 실패: {}", e))?;

    cert_params.is_ca =
        proxyapi_v2::rcgen::IsCa::Ca(proxyapi_v2::rcgen::BasicConstraints::Unconstrained);
    cert_params.key_usages = vec![
        proxyapi_v2::rcgen::KeyUsagePurpose::DigitalSignature,
        proxyapi_v2::rcgen::KeyUsagePurpose::KeyCertSign,
        proxyapi_v2::rcgen::KeyUsagePurpose::CrlSign,
    ];

    // 자체 서명 인증서 생성
    let ca_cert = cert_params
        .self_signed(&key_pair)
        .map_err(|e| format!("CA 인증서 생성 실패: {}", e))?;

    // RcgenAuthority 생성
    let ca = RcgenAuthority::new(key_pair, ca_cert, 1_000, aws_lc_rs::default_provider());

    Ok(ca)
}

/// hudsucker 프록시 시작
#[tauri::command]
pub async fn start_proxy_v2(app: tauri::AppHandle, addr: SocketAddr) -> Result<(), String> {
    // 프록시 상태를 true로 설정
    let proxy_state: tauri::State<ProxyV2State> = app.state();
    proxy_state.set_running(true).await;

    // CA 인증서 생성
    let ca = build_ca()?;
    println!("CA 인증서 생성 완료");

    // 로깅 핸들러 생성
    let handler = LoggingHandler::new(app.clone());

    // TCP 리스너 생성
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| format!("포트 {} 바인딩 실패: {}", addr.port(), e))?;

    println!("포트 {}에서 TCP 리스너 시작됨", addr.port());

    // 프록시 빌더로 프록시 구성
    let proxy = ProxyBuilder::new()
        .with_listener(listener)
        .with_ca(ca)
        .with_rustls_client(aws_lc_rs::default_provider())
        .with_http_handler(handler.clone())
        .with_websocket_handler(handler)
        .build()
        .map_err(|e| format!("프록시 빌드 실패: {}", e))?;

    println!("프록시 빌더 구성 완료");

    // 프록시를 백그라운드에서 실행
    tauri::async_runtime::spawn(async move {
        println!("프록시 서버 시작 중...");
        if let Err(e) = proxy.start().await {
            eprintln!("프록시 실행 오류: {}", e);
        }
    });

    println!(
        "프록시 V2가 포트 {}에서 성공적으로 시작되었습니다",
        addr.port()
    );
    println!(
        "시스템 프록시 설정을 127.0.0.1:{}로 변경하세요",
        addr.port()
    );

    Ok(())
}

/// hudsucker 프록시 중지
#[tauri::command]
pub async fn stop_proxy_v2(app: tauri::AppHandle) -> Result<(), String> {
    let proxy_state: tauri::State<ProxyV2State> = app.state();
    proxy_state.set_running(false).await;

    println!("프록시 V2가 중지되었습니다");
    println!("시스템 프록시 설정을 해제하세요");
    Ok(())
}

/// hudsucker 프록시 상태 확인
#[tauri::command]
pub async fn proxy_v2_status(app: tauri::AppHandle) -> Result<bool, String> {
    let proxy_state: tauri::State<ProxyV2State> = app.state();
    Ok(proxy_state.get_running().await)
}

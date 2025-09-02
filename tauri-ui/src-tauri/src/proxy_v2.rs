use proxyapi_v2::{
    builder::ProxyBuilder,
    certificate_authority::build_ca,
    hyper::{Request, Response},
    rustls::crypto::aws_lc_rs,
    tokio_tungstenite::tungstenite::Message,
    Body, HttpContext, HttpHandler, RequestOrResponse, WebSocketContext, WebSocketHandler,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::net::TcpListener;
use tokio::sync::oneshot::Sender;
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
    let ca = build_ca()?;
    println!("기존 CA 인증서 로드 완료");

    // 로깅 핸들러 생성
    let handler = LoggingHandler::new(app.clone());

    // TCP 리스너 생성
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| format!("포트 {} 바인딩 실패: {}", addr.port(), e))?;

    println!("포트 {}에서 TCP 리스너 시작됨", addr.port());

    // 프록시 빌더로 프록시 구성
    let proxy_builder = ProxyBuilder::new()
        .with_listener(listener)
        .with_ca(ca)
        .with_rustls_client(aws_lc_rs::default_provider())
        .with_http_handler(handler.clone())
        .with_websocket_handler(handler)
        .build()
        .map_err(|e| format!("프록시 빌드 실패: {}", e))?;

    println!("프록시 빌더 구성 완료");

    // 종료 신호를 위한 채널 생성
    let (close_tx, _close_rx) = tokio::sync::oneshot::channel();

    // 프록시를 백그라운드에서 실행
    let thread = tauri::async_runtime::spawn(async move {
        println!("프록시 서버 시작 중...");
        if let Err(e) = proxy_builder.start().await {
            eprintln!("프록시 실행 오류: {}", e);
        }
    });

    // 프록시 상태 업데이트
    let mut proxy_guard = proxy.lock().await;
    proxy_guard.replace((close_tx, thread));

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

/// hudsucker 프록시 중지 (실제 프록시 서버 중지)
#[tauri::command]
pub async fn stop_proxy_v2(proxy: tauri::State<'_, ProxyV2State>) -> Result<(), String> {
    let mut proxy_guard = proxy.lock().await;

    if let Some((close_tx, thread)) = proxy_guard.take() {
        // 종료 신호 전송
        let _ = close_tx.send(());

        // 스레드 종료 대기
        let _ = thread.await;

        println!("프록시 V2가 중지되었습니다");
        println!("시스템 프록시 설정을 해제하세요");
    }

    Ok(())
}

/// hudsucker 프록시 상태 확인 (실제 프록시 상태 확인)
#[tauri::command]
pub async fn proxy_v2_status(proxy: tauri::State<'_, ProxyV2State>) -> Result<bool, String> {
    Ok(proxy.lock().await.is_some())
}

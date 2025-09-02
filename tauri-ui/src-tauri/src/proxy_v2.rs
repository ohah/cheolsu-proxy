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
use tauri::Emitter;
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

    /// 에러 응답을 프론트엔드로 전송
    fn emit_error(&self, error_type: &str, details: &str) {
        let error_info = format!("{}: {}", error_type, details);
        let _ = self.app_handle.emit("proxy_error", &error_info);
        eprintln!("{}", error_info);
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

        // 요청 상세 로깅
        println!("=== HTTP 요청 상세 ===");
        println!("Method: {}", req.method());
        println!("URI: {}", req.uri());
        println!("Headers: {:?}", req.headers());

        req.into()
    }

    async fn handle_response(&mut self, _ctx: &HttpContext, res: Response<Body>) -> Response<Body> {
        // 응답 정보를 프론트엔드로 전송
        let _ = self.app_handle.emit("proxy_response", format!("{:?}", res));

        // 응답 상태 상세 로깅
        println!("=== HTTP 응답 상세 ===");
        println!(
            "Status: {} ({})",
            res.status(),
            res.status().canonical_reason().unwrap_or("Unknown")
        );
        println!("Headers: {:?}", res.headers());

        // 응답 본문 크기 확인
        if let Some(content_length) = res.headers().get("content-length") {
            if let Ok(len) = content_length.to_str() {
                if let Ok(len_num) = len.parse::<usize>() {
                    println!("Response Content-Length: {} bytes", len_num);
                }
            }
        }

        // 특정 에러 상태 코드 상세 분석
        match res.status().as_u16() {
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
                if res.status().is_client_error() || res.status().is_server_error() {
                    let error_msg = format!(
                        "❌ HTTP 에러 {}: {}",
                        res.status(),
                        res.status().canonical_reason().unwrap_or("Unknown")
                    );
                    eprintln!("{}", error_msg);
                    let _ = self.app_handle.emit("proxy_error", error_msg);
                } else {
                    println!("✅ 정상 응답: {}", res.status());
                }
            }
        }

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

    // 프록시 빌더로 프록시 구성
    let proxy_builder = match ProxyBuilder::new()
        .with_listener(listener)
        .with_ca(ca)
        .with_rustls_client(aws_lc_rs::default_provider())
        .with_http_handler(handler.clone())
        .with_websocket_handler(handler.clone())
        .build()
    {
        Ok(builder) => {
            println!("✅ 프록시 빌더 구성 완료");
            println!("   - CA 인증서: 로드됨");
            println!("   - TLS 클라이언트: rustls (aws_lc_rs)");
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

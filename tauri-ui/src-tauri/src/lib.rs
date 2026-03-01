// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod proxy_v2;
mod system_proxy;
use proxy_v2::{
    clean_old_proxy_cache, proxy_v2_status, read_body_file, start_proxy_v2, stop_proxy_v2,
    store_changed_v2, ProxyV2State,
};
use system_proxy::{get_proxy_status_command, set_proxy};
use tauri::Manager;
use tauri_plugin_cli::CliExt;

/// headless (CLI) 모드인지 확인하고, headless일 경우 프록시를 자동 시작합니다.
fn handle_cli_mode(app: &tauri::App) -> bool {
    let cli_matches = match app.cli().matches() {
        Ok(m) => m,
        Err(_) => return false,
    };

    let headless = cli_matches
        .args
        .get("headless")
        .map(|v| v.occurrences > 0)
        .unwrap_or(false);

    if !headless {
        return false;
    }

    // verbose 모드
    let verbose = cli_matches
        .args
        .get("verbose")
        .map(|v| v.occurrences > 0)
        .unwrap_or(false);

    // 포트 파싱 (기본값 8080)
    let port: u16 = cli_matches
        .args
        .get("port")
        .and_then(|v| v.value.as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);

    // 호스트 파싱 (기본값 127.0.0.1)
    let host = cli_matches
        .args
        .get("host")
        .and_then(|v| v.value.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "127.0.0.1".to_string());

    println!("========================================");
    println!(" Cheolsu Proxy — Headless Mode");
    println!("========================================");
    println!("  Listen : {}:{}", host, port);
    println!("  Verbose: {}", verbose);
    println!("========================================");

    // GUI 윈도우 닫기
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.close();
    }

    let addr: std::net::SocketAddr = format!("{}:{}", host, port)
        .parse()
        .expect("Invalid host:port");

    // headless 모드에서 프록시 자동 시작
    tauri::async_runtime::spawn(async move {
        use proxyapi_v2::builder::ProxyBuilder;
        use proxyapi_v2::certificate_authority::{
            build_ca, generate_session_hash, get_cache_storage_dir,
        };
        use tokio::net::TcpListener;

        // CA 인증서 생성
        let ca = match build_ca() {
            Ok(ca) => ca,
            Err(e) => {
                eprintln!("CA 인증서 생성 실패: {}", e);
                std::process::exit(1);
            }
        };

        // 세션 해시 및 캐시 디렉토리
        let session_hash = generate_session_hash();
        let cache_dir = match get_cache_storage_dir(&session_hash) {
            Ok(dir) => dir,
            Err(e) => {
                eprintln!("캐시 디렉토리 생성 실패: {}", e);
                std::process::exit(1);
            }
        };

        // 이벤트 채널 (stdout 출력용)
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let (tunnel_tx, mut tunnel_rx) =
            tokio::sync::mpsc::channel::<proxy_v2_models::RequestInfo>(100);

        // 로깅 핸들러
        let handler = proxy_v2::LoggingHandler::new(tx.clone(), cache_dir);

        // 하이브리드 클라이언트
        let hybrid_client = match proxy_v2::create_hybrid_client() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("클라이언트 생성 실패: {}", e);
                std::process::exit(1);
            }
        };

        // TCP 리스너
        let listener = match TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("포트 {} 바인딩 실패: {}", addr.port(), e);
                std::process::exit(1);
            }
        };

        // 프록시 빌더
        let proxy_builder = match ProxyBuilder::new()
            .with_listener(listener)
            .with_ca(ca)
            .with_client(hybrid_client)
            .with_http_handler(handler.clone())
            .with_websocket_handler(handler.clone())
            .with_tunnel_event_sender(tunnel_tx)
            .build()
        {
            Ok(b) => b,
            Err(e) => {
                eprintln!("프록시 빌드 실패: {}", e);
                std::process::exit(1);
            }
        };

        println!("Proxy listening on {}:{}", host, port);
        println!("Press Ctrl+C to stop.");

        // stdout으로 이벤트 출력 (HTTP 요청/응답)
        tauri::async_runtime::spawn(async move {
            for event in rx.iter() {
                let json = serde_json::to_string(&event).unwrap_or_default();
                println!("[HTTP] {}", json);
            }
        });

        // stdout으로 터널 이벤트 출력
        tauri::async_runtime::spawn(async move {
            while let Some(tunnel_event) = tunnel_rx.recv().await {
                let json = serde_json::to_string(&tunnel_event).unwrap_or_default();
                println!("[TUNNEL] {}", json);
            }
        });

        // 프록시 실행
        if let Err(e) = proxy_builder.start().await {
            eprintln!("프록시 실행 오류: {}", e);
            std::process::exit(1);
        }
    });

    // Ctrl+C graceful shutdown
    let app_handle2 = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        if let Ok(()) = tokio::signal::ctrl_c().await {
            println!("\nShutting down...");
            if let Err(e) = set_proxy(false) {
                eprintln!("프록시 설정 해제 실패: {}", e);
            }
            app_handle2.exit(0);
        }
    });

    true
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(debug_assertions)]
    // let devtools = tauri_plugin_devtools::init();
    {
        let mut builder = tauri::Builder::default()
            .plugin(tauri_plugin_cli::init())
            .plugin(tauri_plugin_http::init())
            .plugin(tauri_plugin_opener::init())
            .plugin(tauri_plugin_fs::init())
            .plugin(tauri_plugin_clipboard_manager::init())
            .plugin(tauri_plugin_store::Builder::default().build());

        // DevTools 플러그인 추가 (개발 빌드에서만)
        // #[cfg(debug_assertions)]
        // {
        //     builder = builder.plugin(devtools);
        // }

        builder
            .setup(|app_handle| {
                // proxyapi_v2 프록시 상태
                app_handle.manage(ProxyV2State::default());

                // CLI 모드 확인 — headless이면 GUI 셋업 건너뜀
                if handle_cli_mode(app_handle) {
                    return Ok(());
                }

                // 앱 시작 시 자동 캐시 정리 (1일 이상 된 캐시)
                tauri::async_runtime::spawn(async {
                    match clean_old_proxy_cache(1).await {
                        Ok(message) => println!("🧹 {}", message),
                        Err(e) => eprintln!("⚠️ 캐시 정리 실패: {}", e),
                    }
                });

                tauri::async_runtime::spawn(async {
                    if let Err(e) = set_proxy(true) {
                        eprintln!("프록시 설정 실패: {}", e);
                    }
                });
                Ok(())
            })
            .on_window_event(|_window, event| {
                // 앱 종료 시 프록시 해제
                if let tauri::WindowEvent::CloseRequested { .. } = event {
                    println!("CloseRequested");
                    if let Err(e) = set_proxy(false) {
                        eprintln!("프록시 설정 실패: {}", e);
                    }
                }
            })
            .invoke_handler(tauri::generate_handler![
                start_proxy_v2,
                stop_proxy_v2,
                proxy_v2_status,
                store_changed_v2,
                get_proxy_status_command,
                read_body_file,
                clean_old_proxy_cache
            ])
            .run(tauri::generate_context!())
            .expect("error while running tauri application");
    }
}

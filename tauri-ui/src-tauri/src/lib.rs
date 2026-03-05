// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod proxy_v2;
mod system_proxy;
use proxy_v2::{
    clean_old_proxy_cache, proxy_v2_status, read_body_file, start_proxy_v2, stop_proxy_v2,
    store_changed_v2, ProxyV2State,
};
use system_proxy::get_proxy_status_command;
use tauri::Manager;
use tauri_plugin_cli::CliExt;

/// headless (CLI) 모드인지 확인하고, headless일 경우 daemon 클라이언트로 동작합니다.
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

    // GUI 윈도우 숨기기 (close하면 Tauri가 앱을 종료함)
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }

    // headless 모드: daemon에 연결하여 이벤트를 stdout에 출력
    let app_handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        match proxy_daemon::ensure_daemon(port, &host, move |event| {
            let json = serde_json::to_string(&event).unwrap_or_default();
            println!("{}", json);
        })
        .await
        {
            Ok(conn) => {
                println!("Connected to daemon (port {})", conn.port);

                // Keep connection alive until Ctrl+C
                match tokio::signal::ctrl_c().await {
                    Ok(()) => {
                        println!("\nShutting down...");
                        conn.disconnect().await;
                        app_handle.exit(0);
                    }
                    Err(e) => {
                        eprintln!("Ctrl+C handler error: {}", e);
                        conn.disconnect().await;
                        app_handle.exit(1);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to connect to daemon: {}", e);
                app_handle.exit(1);
            }
        }
    });

    true
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(debug_assertions)]
    // let devtools = tauri_plugin_devtools::init();
    {
        let builder = tauri::Builder::default()
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
                        Ok(message) => println!("{}", message),
                        Err(e) => eprintln!("캐시 정리 실패: {}", e),
                    }
                });

                // GUI 모드에서는 시스템 프록시 설정을 daemon이 담당하므로 여기서는 하지 않음

                Ok(())
            })
            .on_window_event(|_window, event| {
                // GUI 종료 시 daemon과의 연결만 해제 (프록시 설정 해제는 daemon이 담당)
                if let tauri::WindowEvent::CloseRequested { .. } = event {
                    println!("CloseRequested");
                    // daemon과의 연결은 ProxyV2State drop 시 자동 해제
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

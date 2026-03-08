// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod proxy_v2;
mod system_proxy;
use proxy_v2::{
    check_ca_installed, check_cli_installed, clean_old_proxy_cache, export_har_file,
    get_ca_cert_path, get_mcp_server_path, install_ca_cert, install_cli, load_script,
    proxy_v2_status, read_body_file, replay_request, replay_sequence, start_proxy_v2,
    stop_proxy_v2, uninstall_ca_cert, uninstall_cli, unload_script, update_intercept_rules_v2,
    update_server_replay, update_upstream_proxy, ws_inject_message, ProxyV2State,
};
use system_proxy::get_proxy_status_command;
use tauri::menu::SubmenuBuilder;
use tauri::Manager;

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // #[cfg(debug_assertions)]
    // let devtools = tauri_plugin_devtools::init();
    {
        let mut builder = tauri::Builder::default()
            .plugin(tauri_plugin_http::init())
            .plugin(tauri_plugin_opener::init())
            .plugin(tauri_plugin_fs::init())
            .plugin(tauri_plugin_dialog::init())
            .plugin(tauri_plugin_clipboard_manager::init())
            .plugin(tauri_plugin_store::Builder::default().build())
            .plugin(tauri_plugin_updater::Builder::new().build())
            .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
                // 이미 실행 중인 인스턴스의 메인 윈도우를 포커스
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            }));

        #[cfg(debug_assertions)]
        {
            builder = builder.plugin(tauri_plugin_mcp_bridge::init());
        }

        // DevTools 플러그인 추가 (개발 빌드에서만)
        // #[cfg(debug_assertions)]
        // {
        //     builder = builder.plugin(devtools);
        // }

        builder
            .setup(|app_handle| {
                // proxyapi_v2 프록시 상태
                app_handle.manage(ProxyV2State::default());

                // 네이티브 메뉴 설정
                let app_menu = SubmenuBuilder::new(app_handle, "Cheolsu Proxy")
                    .about(None)
                    .separator()
                    .hide()
                    .hide_others()
                    .show_all()
                    .separator()
                    .quit()
                    .build()?;

                let edit_menu = SubmenuBuilder::new(app_handle, "Edit")
                    .undo()
                    .redo()
                    .separator()
                    .cut()
                    .copy()
                    .paste()
                    .select_all()
                    .build()?;

                let view_menu = SubmenuBuilder::new(app_handle, "View")
                    .item(
                        &tauri::menu::MenuItemBuilder::with_id("refresh", "Reload")
                            .accelerator("CmdOrCtrl+R")
                            .build(app_handle)?,
                    )
                    .separator()
                    .fullscreen()
                    .build()?;

                let window_menu = SubmenuBuilder::new(app_handle, "Window")
                    .minimize()
                    .maximize()
                    .close_window()
                    .build()?;

                let menu = tauri::menu::MenuBuilder::new(app_handle)
                    .items(&[&app_menu, &edit_menu, &view_menu, &window_menu])
                    .build()?;

                app_handle.set_menu(menu)?;

                // 앱 시작 시 자동 캐시 정리 (1일 이상 된 캐시)
                tauri::async_runtime::spawn(async {
                    match clean_old_proxy_cache(1).await {
                        Ok(message) => println!("{}", message),
                        Err(e) => eprintln!("캐시 정리 실패: {}", e),
                    }
                });

                Ok(())
            })
            .on_menu_event(|app_handle, event| {
                static LAST_RELOAD: AtomicU64 = AtomicU64::new(0);
                const DEBOUNCE_MS: u64 = 500;

                if event.id().as_ref() == "refresh" {
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    let last = LAST_RELOAD.load(Ordering::Relaxed);
                    if now.saturating_sub(last) < DEBOUNCE_MS {
                        return;
                    }
                    LAST_RELOAD.store(now, Ordering::Relaxed);

                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.eval("location.reload()");
                    }
                }
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
                update_intercept_rules_v2,
                get_proxy_status_command,
                read_body_file,
                clean_old_proxy_cache,
                replay_request,
                replay_sequence,
                ws_inject_message,
                update_upstream_proxy,
                update_server_replay,
                get_mcp_server_path,
                install_cli,
                uninstall_cli,
                check_cli_installed,
                get_ca_cert_path,
                check_ca_installed,
                install_ca_cert,
                uninstall_ca_cert,
                load_script,
                unload_script,
                export_har_file
            ])
            .run(tauri::generate_context!())
            .expect("error while running tauri application");
    }
}

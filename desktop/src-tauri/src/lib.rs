// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod proxy_v2;
mod system_proxy;
mod tray;
use proxy_v2::{
    advanced_repeat, autoload_session, autosave_session, check_ca_installed, check_cli_installed,
    clean_old_proxy_cache, clear_log_file, diff_transaction_pairs, diff_transactions,
    export_har_file, get_ca_cert_path, get_cert_download_info, get_log_dir, get_log_files,
    get_mcp_server_path, import_har_file_cmd, install_ca_cert, install_cli, load_script,
    load_session, proxy_v2_status, read_body_file, read_log_file, replay_request, replay_sequence,
    resolve_breakpoint, save_session, start_proxy_v2, stop_proxy_v2, uninstall_ca_cert,
    uninstall_cli, unload_script, update_breakpoint_rules, update_client_certificate,
    update_host_mappings, update_intercept_rules_v2, update_proxy_auth, update_quick_settings,
    update_server_replay, update_ssl_proxying_list, update_throttle, update_upstream_proxy,
    ws_inject_message, ProxyV2State,
};
use system_proxy::get_proxy_status_command;
use tauri::menu::{MenuItemBuilder, SubmenuBuilder};
use tauri::{Emitter, Manager};
use tray::setup_tray;

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
            .plugin(tauri_plugin_updater::Builder::new().build())
            .plugin(tauri_plugin_global_shortcut::Builder::new().build())
            .plugin(tauri_plugin_os::init())
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

                // 시스템 트레이 설정
                setup_tray(app_handle)?;

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
                    .fullscreen()
                    .build()?;

                // Proxy 메뉴 (찰스 프록시 스타일)
                let proxy_menu = SubmenuBuilder::new(app_handle, "Proxy")
                    .item(
                        &MenuItemBuilder::with_id("nav_settings", "Proxy Settings...")
                            .build(app_handle)?,
                    )
                    .item(
                        &MenuItemBuilder::with_id("nav_ssl_proxying", "SSL Proxying Settings...")
                            .build(app_handle)?,
                    )
                    .separator()
                    .item(
                        &MenuItemBuilder::with_id("nav_breakpoint", "Breakpoint Settings...")
                            .build(app_handle)?,
                    )
                    .separator()
                    .item(
                        &MenuItemBuilder::with_id("nav_intercept_rules", "Intercept Rules...")
                            .build(app_handle)?,
                    )
                    .build()?;

                // Tools 메뉴 (찰스 프록시 스타일)
                let tools_menu = SubmenuBuilder::new(app_handle, "Tools")
                    .item(
                        &MenuItemBuilder::with_id("nav_map_rules", "Map Remote / Map Local...")
                            .build(app_handle)?,
                    )
                    .item(
                        &MenuItemBuilder::with_id("nav_host_mapping", "DNS Spoofing...")
                            .build(app_handle)?,
                    )
                    .separator()
                    .item(
                        &MenuItemBuilder::with_id("nav_server_replay", "Server Replay...")
                            .build(app_handle)?,
                    )
                    .item(
                        &MenuItemBuilder::with_id("nav_script", "Scripting...")
                            .build(app_handle)?,
                    )
                    .build()?;

                let window_menu = SubmenuBuilder::new(app_handle, "Window")
                    .minimize()
                    .maximize()
                    .close_window()
                    .build()?;

                let menu = tauri::menu::MenuBuilder::new(app_handle)
                    .items(&[
                        &app_menu,
                        &edit_menu,
                        &view_menu,
                        &proxy_menu,
                        &tools_menu,
                        &window_menu,
                    ])
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
                let id = event.id().as_ref();
                if id.starts_with("nav_") {
                    let path = match id {
                        "nav_settings" => "/settings",
                        "nav_ssl_proxying" => "/settings?section=ssl-proxying",
                        "nav_breakpoint" => "/breakpoint",
                        "nav_intercept_rules" => "/intercept-rules",
                        "nav_map_rules" => "/map-rules",
                        "nav_host_mapping" => "/host-mapping",
                        "nav_server_replay" => "/server-replay",
                        "nav_script" => "/script",
                        _ => return,
                    };
                    let _ = app_handle.emit("menu_navigate", path);
                }
            })
            .on_window_event(|window, event| {
                // 메인 윈도우 닫기 시 트레이로 최소화 (완전 종료 대신)
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    if window.label() == "main" {
                        // 닫기를 막고 윈도우를 숨김
                        api.prevent_close();
                        let _ = window.hide();
                    }
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
                advanced_repeat,
                ws_inject_message,
                update_upstream_proxy,
                update_proxy_auth,
                update_throttle,
                update_server_replay,
                update_host_mappings,
                update_quick_settings,
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
                export_har_file,
                save_session,
                load_session,
                autosave_session,
                autoload_session,
                import_har_file_cmd,
                get_cert_download_info,
                diff_transactions,
                diff_transaction_pairs,
                update_breakpoint_rules,
                update_ssl_proxying_list,
                update_client_certificate,
                resolve_breakpoint,
                tray::tray_get_info,
                tray::tray_show_main_window,
                tray::tray_quit_app,
                get_log_files,
                read_log_file,
                clear_log_file,
                get_log_dir,
            ])
            .run(tauri::generate_context!())
            .expect("error while running tauri application");
    }
}

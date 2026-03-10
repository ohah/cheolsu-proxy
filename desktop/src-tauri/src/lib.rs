// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod proxy_v2;
mod system_proxy;
mod tray;
use proxy_v2::{
    advanced_repeat, autoload_session, autosave_session, check_ca_installed, check_cli_installed,
    clean_old_proxy_cache, diff_transaction_pairs, diff_transactions, export_har_file,
    get_ca_cert_path, get_cert_download_info, get_mcp_server_path, import_har_file_cmd,
    install_ca_cert, install_cli, load_script, load_session, proxy_v2_status, read_body_file,
    replay_request, replay_sequence, resolve_breakpoint, save_session, start_proxy_v2,
    stop_proxy_v2, uninstall_ca_cert, uninstall_cli, unload_script, update_breakpoint_rules,
    update_client_certificate, update_host_mappings, update_intercept_rules_v2, update_proxy_auth,
    update_quick_settings, update_server_replay, update_ssl_proxying_list, update_throttle,
    update_upstream_proxy, ws_inject_message, ProxyV2State,
};
use system_proxy::get_proxy_status_command;
use tauri::menu::SubmenuBuilder;
use tauri::Manager;
use tray::setup_tray;

// ============================================================
// 데드락 진단: DIAG_MINIMAL_MODE = true → 최소 구성으로 실행
// 플러그인, 트레이, 메뉴, 시작 작업 모두 제거
// ============================================================
const DIAG_MINIMAL_MODE: bool = true;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    if DIAG_MINIMAL_MODE {
        // ── 최소 구성: opener 플러그인만 ──
        builder = builder
            .plugin(tauri_plugin_opener::init())
            .setup(|app_handle| {
                app_handle.manage(ProxyV2State::default());
                Ok(())
            });
    } else {
        // ── 원래 전체 구성 ──
        builder = builder
            .plugin(tauri_plugin_http::init())
            .plugin(tauri_plugin_opener::init())
            .plugin(tauri_plugin_fs::init())
            .plugin(tauri_plugin_dialog::init())
            .plugin(tauri_plugin_clipboard_manager::init())
            .plugin(tauri_plugin_store::Builder::default().build())
            .plugin(tauri_plugin_updater::Builder::new().build())
            .plugin(tauri_plugin_global_shortcut::Builder::new().build())
            .plugin(tauri_plugin_os::init())
            .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            }));

        #[cfg(debug_assertions)]
        {
            builder = builder.plugin(tauri_plugin_mcp_bridge::init());
        }

        builder = builder
            .setup(|app_handle| {
                app_handle.manage(ProxyV2State::default());

                setup_tray(app_handle)?;

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

                let window_menu = SubmenuBuilder::new(app_handle, "Window")
                    .minimize()
                    .maximize()
                    .close_window()
                    .build()?;

                let menu = tauri::menu::MenuBuilder::new(app_handle)
                    .items(&[&app_menu, &edit_menu, &view_menu, &window_menu])
                    .build()?;

                app_handle.set_menu(menu)?;

                tauri::async_runtime::spawn(async {
                    match clean_old_proxy_cache(1).await {
                        Ok(message) => println!("{}", message),
                        Err(e) => eprintln!("캐시 정리 실패: {}", e),
                    }
                });

                Ok(())
            })
            .on_window_event(|window, event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    if window.label() == "main" {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                }
            });
    }

    builder
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

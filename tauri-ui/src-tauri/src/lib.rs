// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod proxy;
mod proxy_v2;
use proxy::{
    get_proxy_status_command, proxy_status, set_proxy, start_proxy, stop_proxy, store_changed,
    ProxyState,
};
use proxy_v2::{proxy_v2_status, start_proxy_v2, stop_proxy_v2, store_changed_v2, ProxyV2State};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(debug_assertions)]
    let devtools = tauri_plugin_devtools::init();
    {
        let mut builder = tauri::Builder::default()
            .plugin(tauri_plugin_http::init())
            .plugin(tauri_plugin_opener::init())
            .plugin(tauri_plugin_fs::init())
            .plugin(tauri_plugin_store::Builder::default().build());

        // DevTools 플러그인 추가 (개발 빌드에서만)
        #[cfg(debug_assertions)]
        {
            builder = builder.plugin(devtools);
        }

        builder
            .setup(|app_handle| {
                use tauri::async_runtime::Mutex;
                // 기존 프록시 상태
                app_handle.manage(Mutex::new(None) as ProxyState);
                // 새로운 proxyapi_v2 프록시 상태
                app_handle.manage(ProxyV2State::default());

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
                start_proxy,
                stop_proxy,
                store_changed,
                proxy_status,
                start_proxy_v2,
                stop_proxy_v2,
                proxy_v2_status,
                store_changed_v2,
                get_proxy_status_command
            ])
            .run(tauri::generate_context!())
            .expect("error while running tauri application");
    }
}

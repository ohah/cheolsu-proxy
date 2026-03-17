// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use chrono::FixedOffset;
use chrono::Utc;
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

fn main() {
    // Initialize logging only when the LOG environment variable is "true"
    if std::env::var("LOG").unwrap_or_default() == "true" {
        let kst_offset = FixedOffset::east_opt(9 * 3600).expect("KST offset (9h) is always valid");
        let now = Utc::now().with_timezone(&kst_offset);
        let date_str = now.format("%Y-%m-%d").to_string();

        let file_appender =
            tracing_appender::rolling::never("logs", &format!("proxy.log.{}", date_str));
        let (non_blocking_writer, _guard) = tracing_appender::non_blocking(file_appender);

        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

        tracing_subscriber::registry()
            .with(
                fmt::layer()
                    .with_writer(non_blocking_writer)
                    .with_ansi(false), // When writing to a file, it's best to disable ANSI color codes.
            )
            .with(filter)
            .init();
    }

    // Check for --daemon flag BEFORE Tauri initialization
    let args: Vec<String> = std::env::args().collect();
    if args.contains(&"--daemon".to_string()) {
        // Daemon 모드에서는 LOG 설정 여부와 관계없이 tracing을 stderr로 초기화
        // (stdout/stderr가 터미널+로그 파일로 tee됨)
        if std::env::var("LOG").unwrap_or_default() != "true" {
            let filter =
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));
            tracing_subscriber::registry()
                .with(fmt::layer().with_writer(std::io::stderr).with_ansi(false))
                .with(filter)
                .init();
        }

        // Parse port (default 8100)
        let port: u16 = args
            .windows(2)
            .find(|w| w[0] == "--port")
            .and_then(|w| w[1].parse().ok())
            .unwrap_or(8100);

        // Parse host (default 0.0.0.0)
        let host = args
            .windows(2)
            .find(|w| w[0] == "--host")
            .map(|w| w[1].clone())
            .unwrap_or_else(|| "0.0.0.0".to_string());

        // Run daemon without Tauri runtime
        proxy_daemon::run_daemon(port, host);
    }

    cheolsu_proxy::run()
}

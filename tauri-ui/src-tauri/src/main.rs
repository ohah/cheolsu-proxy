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
        let kst_offset = FixedOffset::east_opt(9 * 3600).unwrap();
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

    cheolsu_proxy::run()
}

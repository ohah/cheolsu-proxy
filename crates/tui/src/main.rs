mod app;
mod event;
mod tabs;
mod ui;

use app::App;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "cheolsu", about = "Cheolsu Proxy TUI")]
struct Cli {
    /// 프록시 포트
    #[arg(short, long, default_value_t = 8100)]
    port: u16,

    /// 프록시 호스트
    #[arg(short = 'b', long, default_value = "127.0.0.1")]
    host: String,
}

#[tokio::main]
async fn main() -> color_eyre_stub::Result<()> {
    let cli = Cli::parse();

    let mut app = App::new(cli.port, cli.host);
    app.run().await
}

/// color_eyre 없이 간단한 에러 처리
mod color_eyre_stub {
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
}

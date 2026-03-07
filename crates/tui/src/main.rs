mod app;
mod event;
mod tabs;
mod ui;

use app::App;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "cheolsu", about = "Cheolsu Proxy TUI")]
struct Cli {
    /// Proxy port
    #[arg(short, long, default_value_t = 8100)]
    port: u16,

    /// Proxy host
    #[arg(short = 'b', long, default_value = "127.0.0.1")]
    host: String,

    /// Run as background daemon (internal use)
    #[arg(long, hide = true)]
    daemon: bool,
}

#[tokio::main]
async fn main() -> color_eyre_stub::Result<()> {
    let cli = Cli::parse();

    if cli.daemon {
        proxy_daemon::run_daemon(cli.port, cli.host);
    }

    let mut app = App::new(cli.port, cli.host);
    app.run().await
}

/// Simple error handling without color_eyre
mod color_eyre_stub {
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
}

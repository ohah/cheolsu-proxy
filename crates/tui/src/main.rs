/// 순환 next()/prev() 메서드를 자동 생성하는 매크로
/// 사용법: cycle_enum!(EnumType);
/// 전제: 대상 enum에 `const ALL: [Self; N]` 배열이 정의되어 있어야 함
macro_rules! cycle_enum {
    ($t:ty) => {
        impl $t {
            pub fn next(&self) -> Self {
                let idx = Self::ALL.iter().position(|v| v == self).unwrap_or(0);
                Self::ALL[(idx + 1) % Self::ALL.len()]
            }

            pub fn prev(&self) -> Self {
                let idx = Self::ALL.iter().position(|v| v == self).unwrap_or(0);
                if idx == 0 {
                    Self::ALL[Self::ALL.len() - 1]
                } else {
                    Self::ALL[idx - 1]
                }
            }
        }
    };
}

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
    #[arg(short = 'b', long, default_value = "0.0.0.0")]
    host: String,

    /// Run as background daemon (internal use)
    #[arg(long, hide = true)]
    daemon: bool,
}

fn main() -> color_eyre_stub::Result<()> {
    let cli = Cli::parse();

    // Daemon mode: run_daemon creates its own tokio runtime and never returns.
    // Must be called before #[tokio::main] to avoid nested runtime panic.
    if cli.daemon {
        proxy_daemon::run_daemon(cli.port, cli.host);
    }

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let mut app = App::new(cli.port, cli.host);
            app.run().await
        })
}

/// Simple error handling without color_eyre
mod color_eyre_stub {
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
}

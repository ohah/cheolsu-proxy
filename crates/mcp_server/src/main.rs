mod connection;
mod helpers;
mod params;
mod server;
mod store;
mod tools;

#[cfg(test)]
mod tests;

use anyhow::Result;
use rmcp::{transport::stdio, ServiceExt};
use tracing_subscriber::EnvFilter;

use connection::try_connect_daemon;
use server::CheolsuMcpServer;
use store::Store;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting Cheolsu Proxy MCP server");

    let store = Store::new();
    let conn = try_connect_daemon(&store).await;
    let server = CheolsuMcpServer::new(store, conn);

    let service = server.serve(stdio()).await.inspect_err(|e| {
        tracing::error!("Server error: {:?}", e);
    })?;

    service.waiting().await?;
    Ok(())
}

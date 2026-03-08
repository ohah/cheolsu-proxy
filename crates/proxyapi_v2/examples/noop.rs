use proxyapi_v2::{certificate_authority::build_ca, *};
use std::net::SocketAddr;
use tokio_rustls::rustls::crypto::aws_lc_rs;
use tracing::*;

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install CTRL+C signal handler");
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let ca = build_ca().expect("Failed to build CA");

    let proxy = Proxy::builder()
        .with_addr(SocketAddr::from(([127, 0, 0, 1], 3000)))
        .with_ca(ca)
        .with_rustls_client(aws_lc_rs::default_provider())
        .with_graceful_shutdown(shutdown_signal())
        .build()
        .expect("Failed to create proxy");

    if let Err(e) = proxy.start().await {
        error!("{}", e);
    }
}

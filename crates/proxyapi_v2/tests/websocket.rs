use async_http_proxy::http_connect_tokio;
use futures::{SinkExt, StreamExt};
use proxyapi_v2::{
    certificate_authority::RcgenAuthority,
    rustls::crypto::aws_lc_rs,
    tokio_tungstenite::tungstenite::{Message, Utf8Bytes},
};
use std::sync::atomic::Ordering;
use tokio::net::TcpStream;

#[allow(unused)]
mod common;

const HELLO: Utf8Bytes = Utf8Bytes::from_static("hello");

fn build_ca() -> RcgenAuthority {
    let key_pem = include_str!("../src/certificate_authority/cheolsu-proxy.key");
    let ca_cert_pem = include_str!("../src/certificate_authority/cheolsu-proxy.cer");

    RcgenAuthority::from_pem(ca_cert_pem, key_pem, 1000, aws_lc_rs::default_provider())
        .expect("Failed to create RcgenAuthority from PEM")
}

#[tokio::test]
async fn http() {
    let (proxy_addr, handler, stop_proxy) = common::start_proxy(
        build_ca(),
        common::native_tls_client(),
        common::native_tls_websocket_connector(),
    )
    .await
    .unwrap();

    let (server_addr, stop_server) = common::start_http_server().await.unwrap();

    let mut stream = TcpStream::connect(proxy_addr).await.unwrap();
    http_connect_tokio(
        &mut stream,
        &server_addr.ip().to_string(),
        server_addr.port(),
    )
    .await
    .unwrap();

    let (mut ws, _) = tokio_tungstenite::client_async(format!("ws://{}", server_addr), stream)
        .await
        .unwrap();

    ws.send(Message::Text(HELLO)).await.unwrap();
    let msg = ws.next().await.unwrap().unwrap();
    assert_eq!(msg.into_text().unwrap(), common::WORLD);
    assert_eq!(handler.message_counter.load(Ordering::Relaxed), 2);

    stop_server.send(()).unwrap();
    stop_proxy.send(()).unwrap();
}

#[tokio::test]
async fn https_rustls() {
    let (proxy_addr, handler, stop_proxy) = common::start_proxy(
        build_ca(),
        common::rustls_client(),
        common::rustls_websocket_connector(),
    )
    .await
    .unwrap();

    let (server_addr, stop_server) = common::start_https_server(build_ca()).await.unwrap();

    let mut stream = TcpStream::connect(proxy_addr).await.unwrap();
    http_connect_tokio(&mut stream, "localhost", server_addr.port())
        .await
        .unwrap();

    let (mut ws, _) = tokio_tungstenite::client_async_tls_with_config(
        format!("wss://localhost:{}", server_addr.port()),
        stream,
        None,
        Some(common::rustls_websocket_connector()),
    )
    .await
    .unwrap();

    ws.send(Message::Text(HELLO)).await.unwrap();

    let msg = ws.next().await.unwrap().unwrap();

    assert_eq!(msg.into_text().unwrap(), common::WORLD);
    assert_eq!(handler.message_counter.load(Ordering::Relaxed), 2);

    stop_server.send(()).unwrap();
    stop_proxy.send(()).unwrap();
}

#[tokio::test]
async fn https_native_tls() {
    let (proxy_addr, handler, stop_proxy) = common::start_proxy(
        build_ca(),
        common::native_tls_client(),
        common::native_tls_websocket_connector(),
    )
    .await
    .unwrap();

    let (server_addr, stop_server) = common::start_https_server(build_ca()).await.unwrap();

    let mut stream = TcpStream::connect(proxy_addr).await.unwrap();
    http_connect_tokio(&mut stream, "localhost", server_addr.port())
        .await
        .unwrap();

    let (mut ws, _) = tokio_tungstenite::client_async_tls_with_config(
        format!("wss://localhost:{}", server_addr.port()),
        stream,
        None,
        Some(common::native_tls_websocket_connector()),
    )
    .await
    .unwrap();

    ws.send(Message::Text(HELLO)).await.unwrap();

    let msg = ws.next().await.unwrap().unwrap();

    assert_eq!(msg.into_text().unwrap(), common::WORLD);
    assert_eq!(handler.message_counter.load(Ordering::Relaxed), 2);

    stop_server.send(()).unwrap();
    stop_proxy.send(()).unwrap();
}

#[tokio::test]
async fn without_intercept() {
    let (proxy_addr, handler, stop_proxy) = common::start_proxy_without_intercept(
        build_ca(),
        common::http_client(),
        common::plain_websocket_connector(),
    )
    .await
    .unwrap();

    let (server_addr, stop_server) = common::start_http_server().await.unwrap();

    let mut stream = TcpStream::connect(proxy_addr).await.unwrap();
    http_connect_tokio(
        &mut stream,
        &server_addr.ip().to_string(),
        server_addr.port(),
    )
    .await
    .unwrap();

    let (mut ws, _) = tokio_tungstenite::client_async(format!("ws://{}", server_addr), stream)
        .await
        .unwrap();

    ws.send(Message::Text(HELLO)).await.unwrap();

    let msg = ws.next().await.unwrap().unwrap();

    assert_eq!(msg.into_text().unwrap(), common::WORLD);
    assert_eq!(handler.message_counter.load(Ordering::Relaxed), 0);

    stop_server.send(()).unwrap();
    stop_proxy.send(()).unwrap();
}

#[tokio::test]
async fn noop() {
    let (proxy_addr, stop_proxy) = common::start_noop_proxy(build_ca()).await.unwrap();
    let (server_addr, stop_server) = common::start_http_server().await.unwrap();

    let mut stream = TcpStream::connect(proxy_addr).await.unwrap();
    http_connect_tokio(
        &mut stream,
        &server_addr.ip().to_string(),
        server_addr.port(),
    )
    .await
    .unwrap();

    let (mut ws, _) = tokio_tungstenite::client_async(format!("ws://{}", server_addr), stream)
        .await
        .unwrap();

    ws.send(Message::Text(HELLO)).await.unwrap();
    let msg = ws.next().await.unwrap().unwrap();

    assert_eq!(msg.into_text().unwrap(), common::WORLD);

    stop_server.send(()).unwrap();
    stop_proxy.send(()).unwrap();
}

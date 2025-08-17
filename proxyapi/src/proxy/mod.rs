mod internal;
//pub mod builder;

use serde_json::Value;
use std::{
    convert::Infallible,
    future::Future,
    net::SocketAddr,
    path::PathBuf,
    sync::{mpsc::SyncSender, Arc},
};

use internal::InternalProxy;

use crate::{ca::Ssl, error::Error, proxy_handler};

//use builder::{AddrListenerServer, WantsAddr};

use hyper::{
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Client, Server,
};

use hyper_rustls::HttpsConnectorBuilder;

pub struct Proxy {
    addr: SocketAddr,
    tx: Option<SyncSender<proxy_handler::ProxyHandler>>,
    sessions: Value,
}

impl Proxy {
    pub fn new(
        addr: SocketAddr,
        tx: Option<SyncSender<proxy_handler::ProxyHandler>>,
        sessions: Value,
    ) -> Self {
        Self { addr, tx, sessions }
    }

    pub async fn start<F: Future<Output = ()>>(self, signal: F) -> Result<(), Error> {
        let addr = self.addr;
        let sessions = self.sessions;

        let https = HttpsConnectorBuilder::new()
            .with_webpki_roots()
            .https_or_http()
            .enable_http1()
            .build();

        let client = Client::builder()
            .http1_preserve_header_case(true)
            .http1_title_case_headers(true)
            .build(https);

        let server_builder = Server::try_bind(&addr)?
            .http1_preserve_header_case(true)
            .http1_title_case_headers(true);

        let ssl = Arc::new(Ssl::default());

        let make_service = make_service_fn(move |conn: &AddrStream| {
            let client = client.clone();
            let ca = Arc::clone(&ssl);
            let http_handler = proxy_handler::ProxyHandler::new(self.tx.clone().unwrap());
            let websocket_connector = None;
            let remote_addr = conn.remote_addr();
            let sessions = sessions.clone();
            async move {
                Ok::<_, Infallible>(service_fn(move |req| {
                    InternalProxy {
                        ca: Arc::clone(&ca),
                        client: client.clone(),
                        http_handler: http_handler.clone(),
                        remote_addr,
                        websocket_connector: websocket_connector.clone(),
                        sessions: sessions.clone(),
                    }
                    .proxy(req)
                }))
            }
        });

        server_builder
            .serve(make_service)
            .with_graceful_shutdown(signal)
            .await
            .map_err(Into::into)
    }
}

mod internal;
//pub mod builder;

use serde_json::Value;
use std::{
    convert::Infallible,
    future::Future,
    net::SocketAddr,
    sync::{mpsc::SyncSender, Arc, Mutex},
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

#[derive(Clone)]
pub struct Proxy {
    addr: SocketAddr,
    tx: Option<SyncSender<proxy_handler::ProxyHandler>>,
    sessions: Arc<Mutex<Value>>,
}

impl Proxy {
    pub fn new(
        addr: SocketAddr,
        tx: Option<SyncSender<proxy_handler::ProxyHandler>>,
        sessions: Value,
    ) -> Self {
        Self {
            addr,
            tx,
            sessions: Arc::new(Mutex::new(sessions)),
        }
    }

    // 새로운 sessions로 교체
    pub fn update_sessions(&mut self, new_sessions: Value) {
        if let Ok(mut sessions) = self.sessions.lock() {
            *sessions = new_sessions;
        }
    }

    // 현재 sessions 가져오기
    pub fn get_sessions(&self) -> Value {
        if let Ok(sessions) = self.sessions.lock() {
            sessions.clone()
        } else {
            serde_json::json!([])
        }
    }

    pub async fn start<F: Future<Output = ()>>(self, signal: F) -> Result<(), Error> {
        let addr = self.addr;
        let sessions = Arc::clone(&self.sessions);

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
            let sessions = Arc::clone(&sessions);
            async move {
                Ok::<_, Infallible>(service_fn(move |req| {
                    let sessions_clone = Arc::clone(&sessions);
                    let ca = Arc::clone(&ca);
                    let client = client.clone();
                    let http_handler = http_handler.clone();
                    let remote_addr = remote_addr;
                    let websocket_connector = websocket_connector.clone();

                    async move {
                        let current_sessions = if let Ok(sessions) = sessions_clone.lock() {
                            sessions.clone()
                        } else {
                            serde_json::json!([])
                        };

                        let proxy = InternalProxy {
                            ca,
                            client,
                            http_handler,
                            remote_addr,
                            websocket_connector,
                            sessions: current_sessions,
                        };
                        proxy.proxy(req).await
                    }
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

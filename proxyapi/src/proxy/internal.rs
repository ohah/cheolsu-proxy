// This code was derived from the hudsucker repository:
// https://github.com/omjadas/hudsucker

use crate::{ca::CertificateAuthority, rewind::Rewind, HttpContext, HttpHandler, RequestResponse};
use http::uri::{Authority, Scheme};
use hyper::{
    client::connect::Connect, header::Entry, server::conn::Http, service::service_fn,
    upgrade::Upgraded, Body, Client, Method, Request, Response, Uri,
};
use serde_json::Value;
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite},
    net::TcpStream,
};
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::{tungstenite, Connector};

pub struct InternalProxy<C, CA, H> {
    pub ca: Arc<CA>,
    pub client: Client<C>,
    pub http_handler: H,
    pub websocket_connector: Option<Connector>,
    pub remote_addr: SocketAddr,
    pub sessions: Value,
}

impl<C, CA, H> Clone for InternalProxy<C, CA, H>
where
    C: Clone,
    H: Clone,
{
    fn clone(&self) -> Self {
        InternalProxy {
            ca: Arc::clone(&self.ca),
            client: self.client.clone(),
            http_handler: self.http_handler.clone(),
            websocket_connector: self.websocket_connector.clone(),
            remote_addr: self.remote_addr,
            sessions: self.sessions.clone(),
        }
    }
}

impl<C, CA, H> InternalProxy<C, CA, H>
where
    C: Connect + Clone + Send + Sync + 'static,
    CA: CertificateAuthority,
    H: HttpHandler,
{
    pub(crate) async fn proxy(
        mut self,
        req: Request<Body>,
    ) -> Result<Response<Body>, hyper::Error> {
        let ctx = HttpContext {
            remote_addr: self.remote_addr,
        };

        let req = match self.http_handler.handle_request(&ctx, req).await {
            RequestResponse::Request(req) => req,
            RequestResponse::Response(res) => return Ok(res),
        };

        if req.method() == Method::CONNECT {
            self.process_connect(req)
        } else if hyper_tungstenite::is_upgrade_request(&req) {
            Ok(self.upgrade_websocket(req))
        } else {
            // 세션에서 매칭되는 응답이 있는지 확인 (안전하게 처리)
            match self.check_session_response(&req).await {
                Some(session_response) => {
                    return Ok(self
                        .http_handler
                        .handle_response(&ctx, session_response)
                        .await);
                }
                None => {
                    let res = self.client.request(normalize_request(req)).await?;
                    Ok(self.http_handler.handle_response(&ctx, res).await)
                }
            }
        }
    }

    // 세션에서 매칭되는 응답을 확인하는 새로운 메서드 (더 안전하게)
    async fn check_session_response(&self, req: &Request<Body>) -> Option<Response<Body>> {
        // 세션 데이터가 없으면 즉시 반환
        if self.sessions.is_null() {
            return None;
        }

        // 세션 데이터를 안전하게 파싱
        let sessions = match self.sessions.as_array() {
            Some(sessions) => sessions,
            None => {
                return None;
            }
        };

        let req_uri = req.uri().to_string();
        let req_method = req.method().as_str();

        for (index, session) in sessions.iter().enumerate() {
            // 세션 데이터를 안전하게 추출
            let session_url = match session.get("url").and_then(|v| v.as_str()) {
                Some(url) => url,
                None => {
                    continue;
                }
            };

            let session_method = match session.get("method").and_then(|v| v.as_str()) {
                Some(method) => method,
                None => {
                    continue;
                }
            };

            if session_url == req_uri && session_method == req_method {
                // 응답 데이터를 안전하게 추출
                match session.get("response") {
                    Some(response_data) => {
                        return self.create_response_from_session(response_data);
                    }
                    None => {
                        return None;
                    }
                }
            }
        }

        None
    }

    // 세션 데이터로부터 HTTP 응답을 생성하는 메서드
    fn create_response_from_session(&self, response_data: &Value) -> Option<Response<Body>> {
        // 상태 코드 추출
        let status_code = response_data
            .get("status")
            .and_then(|v| v.as_u64())
            .unwrap_or(200) as u16;

        // 헤더 추출
        let mut headers: http::HeaderMap = response_data
            .get("headers")
            .and_then(Value::as_object)
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| Some((k.parse().ok()?, v.as_str()?.parse().ok()?)))
                    .collect()
            })
            .unwrap_or_default();

        // 기본 Content-Type 헤더 설정 (없는 경우)
        if !headers.contains_key("content-type") {
            headers.insert("content-type", "application/json".parse().unwrap());
        }

        // 세션 응답임을 나타내는 특별한 헤더 추가
        headers.insert("x-proxelar-session", "true".parse().unwrap());

        // 응답 본문 생성
        let body = if let Some(data) = response_data.get("data") {
            match data {
                Value::String(s) => Body::from(s.clone()),
                Value::Object(_) | Value::Array(_) => {
                    let json_string = serde_json::to_string(data).unwrap_or_default();

                    Body::from(json_string)
                }
                _ => {
                    let string_data = data.to_string();

                    Body::from(string_data)
                }
            }
        } else {
            Body::empty()
        };

        // 응답 생성
        let mut response = Response::new(body);
        *response.status_mut() =
            http::StatusCode::from_u16(status_code).unwrap_or(http::StatusCode::OK);
        *response.headers_mut() = headers;

        Some(response)
    }

    fn process_connect(self, mut req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
        let fut = async move {
            match hyper::upgrade::on(&mut req).await {
                Ok(mut upgraded) => {
                    let mut buffer = [0; 4];
                    let bytes_read = match upgraded.read(&mut buffer).await {
                        Ok(bytes_read) => bytes_read,
                        Err(e) => {
                            eprintln!("Failed to read from upgraded connection: {e}");
                            return;
                        }
                    };

                    //TEST: 데이터를 읽지 못한 경우 (빈 연결) 처리
                    if bytes_read == 0 {
                        eprintln!("No data received from upgraded connection");
                        return;
                    }

                    let mut upgraded = Rewind::new_buffered(
                        upgraded,
                        bytes::Bytes::copy_from_slice(buffer[..bytes_read].as_ref()),
                    );

                    if buffer == *b"GET " {
                        if let Err(e) = self.serve_stream(upgraded, Scheme::HTTP).await {
                            eprintln!("Websocket connect error: {e}");
                        }
                    } else if buffer[..2] == *b"\x16\x03" {
                        let authority = req
                            .uri()
                            .authority()
                            .expect("Uri doesn't contain authority");

                        let server_config = self.ca.gen_server_config(authority).await;

                        let stream = match TlsAcceptor::from(server_config).accept(upgraded).await {
                            Ok(stream) => stream,
                            Err(e) => {
                                eprintln!("Failed to establish TLS for '{}': {:?}", authority, e);
                                return;
                            }
                        };

                        if let Err(e) = self.serve_stream(stream, Scheme::HTTPS).await {
                            if !e.to_string().starts_with("error shutting down connection") {
                                eprintln!("HTTPS connect error: {e}");
                            }
                        }
                    } else {
                        eprintln!(
                            "Unknown protocol, read '{:02X?}' from upgraded connection",
                            &buffer[..bytes_read]
                        );

                        let authority = req
                            .uri()
                            .authority()
                            .expect("Uri doesn't contain authority")
                            .as_ref();

                        let mut server = match TcpStream::connect(authority).await {
                            Ok(server) => server,
                            Err(e) => {
                                eprintln! {"failed to connect to {authority}: {e}"};
                                return;
                            }
                        };

                        if let Err(e) =
                            tokio::io::copy_bidirectional(&mut upgraded, &mut server).await
                        {
                            eprintln!("Failed to tunnel unknown protocol to {}: {}", authority, e);
                        }
                    }
                }
                Err(e) => eprintln!("Upgrade error {e}"),
            };
        };

        tokio::spawn(fut);
        Ok(Response::new(Body::empty()))
    }

    fn upgrade_websocket(self, req: Request<Body>) -> Response<Body> {
        let mut req = {
            let (mut parts, _) = req.into_parts();

            parts.uri = {
                let mut parts = parts.uri.into_parts();

                parts.scheme = if parts.scheme.unwrap_or(Scheme::HTTP) == Scheme::HTTP {
                    Some("ws".try_into().expect("Failed to convert scheme"))
                } else {
                    Some("wss".try_into().expect("Failed to convert scheme"))
                };

                Uri::from_parts(parts).expect("Failed to build URI")
            };

            Request::from_parts(parts, ())
        };

        let (res, websocket) =
            hyper_tungstenite::upgrade(&mut req, None).expect("Request missing headers");

        let fut = async move {
            match websocket.await {
                Ok(ws) => {
                    if let Err(e) = self.handle_websocket(ws, req).await {
                        eprintln!("Failed to handle websocket: {e}");
                    }
                }
                Err(e) => {
                    eprintln!("Failed to upgrade to websocket: {e}");
                }
            }
        };

        tokio::spawn(fut);
        res
    }

    async fn handle_websocket(
        self,
        _server_socket: hyper_tungstenite::WebSocketStream<Upgraded>,
        _req: Request<()>,
    ) -> Result<(), tungstenite::Error> {
        Ok(())
    }

    async fn serve_stream<I>(self, stream: I, scheme: Scheme) -> Result<(), hyper::Error>
    where
        I: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let service = service_fn(|mut req| {
            if req.version() == hyper::Version::HTTP_10 || req.version() == hyper::Version::HTTP_11
            {
                let (mut parts, body) = req.into_parts();

                let authority = parts
                    .headers
                    .get(hyper::header::HOST)
                    .expect("Host is a required header")
                    .as_bytes();
                parts.uri = {
                    let mut parts = parts.uri.into_parts();
                    parts.scheme = Some(scheme.clone());
                    parts.authority =
                        Some(Authority::try_from(authority).expect("Failed to parse authority"));
                    Uri::from_parts(parts).expect("Failed to build URI")
                };

                req = Request::from_parts(parts, body);
            };

            self.clone().proxy(req)
        });

        Http::new()
            .serve_connection(stream, service)
            .with_upgrades()
            .await
    }
}

fn normalize_request<T>(mut req: Request<T>) -> Request<T> {
    req.headers_mut().remove(hyper::header::HOST);

    if let Entry::Occupied(mut cookies) = req.headers_mut().entry(hyper::header::COOKIE) {
        let joined_cookies = bstr::join(b"; ", cookies.iter());
        cookies.insert(joined_cookies.try_into().expect("Failed to join cookies"));
    }

    *req.version_mut() = hyper::Version::HTTP_11;
    req
}

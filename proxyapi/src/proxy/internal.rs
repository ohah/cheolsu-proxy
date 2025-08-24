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
            // 세션에서 매칭되는 응답이 있는지 확인
            if let Some(session_response) = self.check_session_response(&req).await {
                // 세션 응답을 http_handler를 통해 처리하여 이벤트 발생
                return Ok(self.http_handler.handle_response(&ctx, session_response).await);
            }

            let res = self.client.request(normalize_request(req)).await?;

            Ok(self.http_handler.handle_response(&ctx, res).await)
        }
    }

    // 세션에서 매칭되는 응답을 확인하는 새로운 메서드
    async fn check_session_response(&self, req: &Request<Body>) -> Option<Response<Body>> {
        println!("🔍 세션 응답 확인 시작");
        println!("📡 요청 URI: {}", req.uri());
        println!("📡 요청 메서드: {}", req.method());

        // 세션 데이터를 파싱
        let sessions = match self.sessions.as_array() {
            Some(sessions) => {
                println!("📋 등록된 세션 수: {}", sessions.len());
                sessions
            }
            None => {
                println!("❌ 세션 데이터가 배열 형태가 아님");
                return None;
            }
        };

        let req_uri = req.uri().to_string();
        let req_method = req.method().as_str();

        for (index, session) in sessions.iter().enumerate() {
            println!(" 세션 {} 확인 중", index + 1);

            // 세션의 URL과 메서드가 요청과 일치하는지 확인
            if let (Some(session_url), Some(session_method)) = (
                session.get("url").and_then(|v| v.as_str()),
                session.get("method").and_then(|v| v.as_str()),
            ) {
                println!("  📋 세션 URL: {}", session_url);
                println!("  📋 세션 메서드: {}", session_method);
                println!(
                    "   매칭 확인: URL={}, 메서드={}",
                    session_url == req_uri,
                    session_method == req_method
                );

                if session_url == req_uri && session_method == req_method {
                    println!("✅ 세션 매칭 성공!");

                    // 응답 데이터가 있는지 확인
                    if let Some(response_data) = session.get("response") {
                        println!("📤 응답 데이터 발견: {:?}", response_data);
                        return self.create_response_from_session(response_data);
                    } else {
                        println!("❌ 세션에 응답 데이터가 없음");
                    }
                }
            } else {
                println!("❌ 세션의 URL 또는 메서드 정보 누락");
            }
        }

        println!("❌ 매칭되는 세션을 찾지 못함");
        None
    }

    // 세션 데이터로부터 HTTP 응답을 생성하는 메서드
    fn create_response_from_session(&self, response_data: &Value) -> Option<Response<Body>> {
        println!("🔧 세션 응답 생성 시작");

        // 상태 코드 추출
        let status_code = response_data
            .get("status")
            .and_then(|v| v.as_u64())
            .unwrap_or(200) as u16;
        println!(" 상태 코드: {}", status_code);

        // 헤더 추출
        let mut headers = http::HeaderMap::new();
        if let Some(headers_data) = response_data.get("headers") {
            println!("📋 헤더 데이터: {:?}", headers_data);
            if let Some(headers_obj) = headers_data.as_object() {
                for (key, value) in headers_obj {
                    if let Some(value_str) = value.as_str() {
                        if let Ok(header_name) = key.parse::<http::HeaderName>() {
                            if let Ok(header_value) = value_str.parse::<http::HeaderValue>() {
                                headers.insert(header_name, header_value);
                                println!("  📋 헤더 추가: {} = {}", key, value_str);
                            } else {
                                println!("  ❌ 헤더 값 파싱 실패: {}", value_str);
                            }
                        } else {
                            println!("  ❌ 헤더 이름 파싱 실패: {}", key);
                        }
                    }
                }
            }
        }

        // 기본 Content-Type 헤더 설정 (없는 경우)
        if !headers.contains_key("content-type") {
            headers.insert("content-type", "application/json".parse().unwrap());
            println!(" 기본 Content-Type 헤더 추가: application/json");
        }

        // 응답 본문 생성
        let body = if let Some(data) = response_data.get("data") {
            println!("📦 응답 데이터: {:?}", data);
            match data {
                Value::String(s) => {
                    println!("📝 문자열 데이터로 응답 생성: {}", s);
                    Body::from(s.clone())
                }
                Value::Object(_) | Value::Array(_) => {
                    let json_string = serde_json::to_string(data).unwrap_or_default();
                    println!("📝 JSON 데이터로 응답 생성: {}", json_string);
                    Body::from(json_string)
                }
                _ => {
                    let string_data = data.to_string();
                    println!("📝 기타 데이터로 응답 생성: {}", string_data);
                    Body::from(string_data)
                }
            }
        } else {
            println!("📝 빈 응답 본문 생성");
            Body::empty()
        };

        // 응답 생성
        let mut response = Response::new(body);
        *response.status_mut() =
            http::StatusCode::from_u16(status_code).unwrap_or(http::StatusCode::OK);
        *response.headers_mut() = headers;

        println!("✅ 세션 응답 생성 완료 - 상태: {}", response.status());
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
                                eprintln!("Failed to establish TLS Connection:{e}");
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

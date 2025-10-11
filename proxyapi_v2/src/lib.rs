#![cfg_attr(docsrs, feature(doc_auto_cfg))]

//! Hudsucker is a MITM HTTP/S proxy that allows you to:
//!
//! - Modify HTTP/S requests
//! - Modify HTTP/S responses
//! - Modify WebSocket messages
//!
//! ## Features
//!
//! - `decoder`: Enables [`decode_request`] and [`decode_response`] helpers (enabled by default).
//! - `full`: Enables all features.
//! - `http2`: Enables HTTP/2 support.
//! - `native-tls-client`: Enables [`ProxyBuilder::with_native_tls_client`](builder::ProxyBuilder::with_native_tls_client).
//! - `openssl-ca`: Enables [`OpensslAuthority`](certificate_authority::OpensslAuthority).
//! - `rcgen-ca`: Enables [`RcgenAuthority`](certificate_authority::RcgenAuthority) (enabled by default).
//! - `rustls-client`: Enables [`ProxyBuilder::with_rustls_client`](builder::ProxyBuilder::with_rustls_client) (enabled by default).

mod body;
#[cfg(feature = "decoder")]
mod decoder;
mod error;
mod noop;
mod proxy;
mod rewind;

pub mod certificate_authority;
pub mod hybrid_tls_handler;
pub mod tls_version_detector;

use futures::{Sink, SinkExt, Stream, StreamExt};
use hyper::{Request, Response, StatusCode, Uri};
use std::net::SocketAddr;
use tokio_tungstenite::tungstenite::{self, Message};
use tracing::error;

pub use futures;
pub use hyper;
pub use hyper_util;
#[cfg(feature = "openssl-ca")]
pub use openssl;
#[cfg(feature = "rcgen-ca")]
pub use rcgen;
pub use tokio_rustls::rustls;
pub use tokio_tungstenite;

pub use body::Body;
#[cfg(feature = "decoder")]
pub use decoder::{decode_request, decode_response};
pub use error::Error;
pub use hybrid_tls_handler::*;
pub use noop::*;
pub use proxy::*;
pub use tls_version_detector::*;

/// Enum representing either an HTTP request or response.
#[derive(Debug)]
pub enum RequestOrResponse {
    /// HTTP Request
    Request(Request<Body>),
    /// HTTP Response
    Response(Response<Body>),
}

impl From<Request<Body>> for RequestOrResponse {
    fn from(req: Request<Body>) -> Self {
        Self::Request(req)
    }
}

impl From<Response<Body>> for RequestOrResponse {
    fn from(res: Response<Body>) -> Self {
        Self::Response(res)
    }
}

/// Context for HTTP requests and responses.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub struct HttpContext {
    /// Address of the client that is sending the request.
    pub client_addr: SocketAddr,
}

/// SSE 전용 핸들러 - chunk 단위로 실시간 전달
pub struct SseHandler;

impl SseHandler {
    /// SSE 응답을 chunk 단위로 실시간 전달
    pub async fn handle_sse_response(&self, res: Response<Body>) -> Response<Body> {
        let sse_processing_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
            
        println!("🚀 SSE 전용 핸들러 활성화 - chunk 단위 전달 모드");
        println!("⏰ SSE 핸들러 처리 시작 시간: {}ms", sse_processing_time);

        let (mut parts, body) = res.into_parts();

        // SSE 최적화된 헤더 설정
        parts
            .headers
            .insert("Cache-Control", "no-cache".parse().unwrap());
        parts
            .headers
            .insert("Connection", "keep-alive".parse().unwrap());
        parts.headers.remove("content-length");

        // Transfer-Encoding: chunked 명시적 설정
        parts
            .headers
            .insert("Transfer-Encoding", "chunked".parse().unwrap());

        println!("🔧 SSE 헤더 최적화 완료 - 실시간 스트리밍 활성화");
        println!("   - Cache-Control: no-cache");
        println!("   - Connection: keep-alive");
        println!("   - Transfer-Encoding: chunked");
        println!("   - Content-Length: 제거됨");

        // Body를 그대로 전달 (proxy/internal.rs에서 이미 chunk 단위로 처리됨)
        Response::from_parts(parts, body)
    }
}

/// Context for websocket messages.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum WebSocketContext {
    #[non_exhaustive]
    ClientToServer {
        /// Address of the client.
        src: SocketAddr,
        /// URI of the server.
        dst: Uri,
    },
    #[non_exhaustive]
    ServerToClient {
        /// URI of the server.
        src: Uri,
        /// Address of the client.
        dst: SocketAddr,
    },
}

/// Handler for HTTP requests and responses.
///
/// Each request/response pair is passed to the same instance of the handler.
pub trait HttpHandler: Clone + Send + Sync + 'static {
    /// This handler will be called for each HTTP request. It can either return a modified request,
    /// or a response. If a request is returned, it will be sent to the upstream server. If a
    /// response is returned, it will be sent to the client.
    fn handle_request(
        &mut self,
        _ctx: &HttpContext,
        req: Request<Body>,
    ) -> impl Future<Output = RequestOrResponse> + Send {
        async { req.into() }
    }

    /// This handler will be called for each HTTP response. It can modify a response before it is
    /// forwarded to the client.
    fn handle_response(
        &mut self,
        _ctx: &HttpContext,
        res: Response<Body>,
    ) -> impl Future<Output = Response<Body>> + Send {
        async {
            // SSE 스트리밍 응답 감지 및 로깅
            let content_type = res
                .headers()
                .get("content-type")
                .and_then(|ct| ct.to_str().ok())
                .unwrap_or("");

            let transfer_encoding = res
                .headers()
                .get("transfer-encoding")
                .and_then(|te| te.to_str().ok())
                .unwrap_or("");

            let is_sse = content_type.contains("text/event-stream")
                || content_type.contains("application/x-ndjson")
                || transfer_encoding.contains("chunked");

            if is_sse {
                let sse_handler_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis();
                    
                println!("🌊 SSE 응답 핸들러에서 감지!");
                println!("   - Content-Type: {}", content_type);
                println!("   - Transfer-Encoding: {}", transfer_encoding);
                println!(
                    "   - Content-Length: {:?}",
                    res.headers().get("content-length")
                );
                println!("🔧 SSE 전용 핸들러로 chunk 단위 전달 시작");
                println!("⏰ SSE 핸들러 처리 시간: {}ms", sse_handler_time);

                // SSE 응답인 경우 헤더 최적화
                let (mut parts, body) = res.into_parts();

                // 스트리밍을 위한 헤더 설정
                parts
                    .headers
                    .insert("Cache-Control", "no-cache".parse().unwrap());
                parts
                    .headers
                    .insert("Connection", "keep-alive".parse().unwrap());

                // Content-Length가 있다면 제거 (스트리밍에서는 불필요)
                parts.headers.remove("content-length");

                println!("🔧 SSE 헤더 최적화 완료 - chunk 단위 전달 활성화");

                // SSE 전용 핸들러로 chunk 단위 전달
                println!("🔧 SSE 응답을 원본 스트림으로 전달");
                return SseHandler
                    .handle_sse_response(Response::from_parts(parts, body))
                    .await;
            }

            res
        }
    }

    /// This handler will be called if a proxy request fails. Default response is a 502 Bad Gateway.
    fn handle_error(
        &mut self,
        _ctx: &HttpContext,
        err: hyper_util::client::legacy::Error,
    ) -> impl Future<Output = Response<Body>> + Send {
        async move {
            error!("Failed to forward request: {}", err);
            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Body::empty())
                .expect("Failed to build response")
        }
    }

    /// Whether a CONNECT request should be intercepted. Defaults to `true` for all requests.
    fn should_intercept(
        &mut self,
        _ctx: &HttpContext,
        _req: &Request<Body>,
    ) -> impl Future<Output = bool> + Send {
        async { true }
    }
}

/// Handler for WebSocket messages.
///
/// Messages sent over the same WebSocket Stream are passed to the same instance of the handler.
pub trait WebSocketHandler: Clone + Send + Sync + 'static {
    /// This handler is responsible for forwarding WebSocket messages from a Stream to a Sink and
    /// recovering from any potential errors.
    fn handle_websocket(
        mut self,
        ctx: WebSocketContext,
        mut stream: impl Stream<Item = Result<Message, tungstenite::Error>> + Unpin + Send + 'static,
        mut sink: impl Sink<Message, Error = tungstenite::Error> + Unpin + Send + 'static,
    ) -> impl Future<Output = ()> + Send {
        async move {
            println!("🔄 WebSocket 메시지 전달 루프 시작: {:?}", ctx);
            let mut message_count = 0;

            loop {
                match stream.next().await {
                    Some(message) => {
                        message_count += 1;
                        match message {
                            Ok(message) => {
                                let Some(message) = self.handle_message(&ctx, message).await else {
                                    println!("⚠️ 메시지가 필터링되어 전달되지 않음: {:?}", ctx);
                                    continue;
                                };

                                match sink.send(message).await {
                                    Err(tungstenite::Error::ConnectionClosed) => {
                                        println!("🔚 WebSocket 연결이 닫혔음: {:?}", ctx);
                                        break;
                                    }
                                    Err(e) => {
                                        println!("❌ WebSocket 전송 에러: {} - {:?}", e, ctx);
                                        break;
                                    }
                                    Ok(_) => {
                                        if message_count % 10 == 0 {
                                            println!(
                                                "✅ {}개 메시지 전달 완료: {:?}",
                                                message_count, ctx
                                            );
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                println!("❌ WebSocket 메시지 에러: {} - {:?}", e, ctx);

                                // Reserved bits 에러인 경우 연결을 끊지 않고 계속 진행
                                if e.to_string().contains("Reserved bits are non-zero") {
                                    println!(
                                        "⚠️ Reserved bits 에러 감지 - 메시지를 건너뛰고 계속 대기: {:?}",
                                        ctx
                                    );
                                    // 이 메시지만 건너뛰고 다음 메시지 계속 처리
                                    continue;
                                }

                                match sink.send(Message::Close(None)).await {
                                    Err(tungstenite::Error::ConnectionClosed) => {
                                        println!("🔚 연결이 이미 닫혀있음: {:?}", ctx);
                                    }
                                    Err(e) => {
                                        println!("❌ WebSocket Close 전송 에러: {} - {:?}", e, ctx);
                                    }
                                    Ok(_) => {
                                        println!("🔚 WebSocket Close 메시지 전송 완료: {:?}", ctx);
                                    }
                                };

                                break;
                            }
                        }
                    }
                    None => {
                        println!("🔚 WebSocket 스트림이 종료됨: {:?}", ctx);

                        // Pusher 연결인 경우 재연결 시도
                        let is_pusher_connection = match &ctx {
                            WebSocketContext::ClientToServer { dst, .. } => dst
                                .host()
                                .map(|h| h.contains("pusher.com"))
                                .unwrap_or(false),
                            WebSocketContext::ServerToClient { src, .. } => src
                                .host()
                                .map(|h| h.contains("pusher.com"))
                                .unwrap_or(false),
                        };

                        if is_pusher_connection {
                            println!(
                                "🔄 Pusher WebSocket 연결 종료 감지 - 재연결을 위해 잠시 대기"
                            );
                            // 잠시 대기 후 재연결 시도
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        }
                        break;
                    }
                }
            }
            println!(
                "🏁 WebSocket 메시지 전달 루프 종료 (총 {}개 메시지): {:?}",
                message_count, ctx
            );
        }
    }

    /// This handler will be called for each WebSocket message. It can return an optional modified
    /// message. If None is returned the message will not be forwarded.
    fn handle_message(
        &mut self,
        ctx: &WebSocketContext,
        message: Message,
    ) -> impl Future<Output = Option<Message>> + Send {
        async move {
            match &message {
                Message::Text(text) => {
                    println!(
                        "📨 WebSocket 텍스트 메시지 [{}]: {} bytes",
                        match ctx {
                            WebSocketContext::ClientToServer { src, dst } =>
                                format!("{} → {}", src, dst.host().unwrap_or("unknown")),
                            WebSocketContext::ServerToClient { src, dst } =>
                                format!("{} → {}", src.host().unwrap_or("unknown"), dst),
                        },
                        text.len()
                    );

                    // SockJS 프레이밍 제거 (다양한 형태 지원)
                    let cleaned_text = if text.starts_with("a[\"") && text.ends_with("\"]") {
                        let inner = &text[3..text.len() - 2]; // a[" 와 "] 제거
                        println!(
                            "🧹 SockJS 프레이밍 제거 (a[\"...\"]): {} -> {}",
                            text, inner
                        );
                        inner.to_string()
                    } else if text.starts_with("a[") && text.ends_with("]") {
                        // a[...] 형태 (따옴표 없음)
                        let inner = &text[2..text.len() - 1]; // a[ 와 ] 제거
                        println!("🧹 SockJS 프레이밍 제거 (a[...]): {} -> {}", text, inner);
                        inner.to_string()
                    } else if text.starts_with("a\"") && text.ends_with("\"") {
                        // a"..." 형태
                        let inner = &text[2..text.len() - 1]; // a" 와 " 제거
                        println!("🧹 SockJS 프레이밍 제거 (a\"...\"): {} -> {}", text, inner);
                        inner.to_string()
                    } else {
                        text.to_string()
                    };

                    // Pusher 메시지인 경우 내용도 로그
                    if cleaned_text.contains("pusher")
                        || cleaned_text.contains("subscribe")
                        || cleaned_text.contains("channel")
                    {
                        println!("🎯 Pusher 메시지 내용: {}", cleaned_text);
                    }

                    // 정리된 메시지로 교체
                    return Some(Message::Text(cleaned_text.into()));
                }
                Message::Binary(data) => {
                    println!(
                        "📦 WebSocket 바이너리 메시지 [{}]: {} bytes",
                        match ctx {
                            WebSocketContext::ClientToServer { src, dst } =>
                                format!("{} → {}", src, dst.host().unwrap_or("unknown")),
                            WebSocketContext::ServerToClient { src, dst } =>
                                format!("{} → {}", src.host().unwrap_or("unknown"), dst),
                        },
                        data.len()
                    );
                }
                Message::Ping(data) => {
                    println!(
                        "🏓 WebSocket Ping [{}]: {} bytes",
                        match ctx {
                            WebSocketContext::ClientToServer { src, dst } =>
                                format!("{} → {}", src, dst.host().unwrap_or("unknown")),
                            WebSocketContext::ServerToClient { src, dst } =>
                                format!("{} → {}", src.host().unwrap_or("unknown"), dst),
                        },
                        data.len()
                    );
                }
                Message::Pong(data) => {
                    println!(
                        "🏓 WebSocket Pong [{}]: {} bytes",
                        match ctx {
                            WebSocketContext::ClientToServer { src, dst } =>
                                format!("{} → {}", src, dst.host().unwrap_or("unknown")),
                            WebSocketContext::ServerToClient { src, dst } =>
                                format!("{} → {}", src.host().unwrap_or("unknown"), dst),
                        },
                        data.len()
                    );
                }
                Message::Close(frame) => {
                    println!(
                        "🔚 WebSocket Close [{}]: {:?}",
                        match ctx {
                            WebSocketContext::ClientToServer { src, dst } =>
                                format!("{} → {}", src, dst.host().unwrap_or("unknown")),
                            WebSocketContext::ServerToClient { src, dst } =>
                                format!("{} → {}", src.host().unwrap_or("unknown"), dst),
                        },
                        frame
                    );
                }
                Message::Frame(_) => {
                    println!(
                        "🖼️ WebSocket Frame [{}]",
                        match ctx {
                            WebSocketContext::ClientToServer { src, dst } =>
                                format!("{} → {}", src, dst.host().unwrap_or("unknown")),
                            WebSocketContext::ServerToClient { src, dst } =>
                                format!("{} → {}", src.host().unwrap_or("unknown"), dst),
                        }
                    );
                }
            }
            Some(message)
        }
    }
}

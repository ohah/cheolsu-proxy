use crate::protocol::{InterceptRule, ServerReplayEntry};
use bytes::Bytes;
use futures_util::stream::StreamExt;
use http_body_util::{BodyExt, StreamBody};
use proxy_v2_models::{
    ProxiedRequest, ProxiedResponse, RequestInfo, WsConnectionEvent, WsDirection, WsMessageInfo,
    WsMessageType,
};
use proxyapi_v2::{
    hyper::http::{Method, StatusCode},
    hyper::{Request, Response},
    tokio_tungstenite::tungstenite::Message,
    Body, HttpContext, HttpHandler, RequestOrResponse, WebSocketContext, WebSocketHandler,
};
use regex::Regex;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, error, info};

// Re-export for lib.rs and daemon.rs
pub use crate::tls_client::create_hybrid_client;

/// WebSocket 이벤트 (메시지 또는 연결 상태)
#[derive(Clone, Debug)]
pub enum WsEvent {
    Message(WsMessageInfo),
    Connection(WsConnectionEvent),
}

/// 인증서 다운로드를 위한 내부 호스트명
const CERT_DOWNLOAD_HOST: &str = "cheolsu.proxy";
const CERT_DOWNLOAD_HOST_COLON: &str = "cheolsu.proxy:";

/// HTTP 및 WebSocket 요청/응답을 로깅하는 핸들러
#[derive(Clone)]
pub struct LoggingHandler {
    pub(crate) sender: tokio::sync::mpsc::Sender<RequestInfo>,
    pub(crate) ws_sender: Option<tokio::sync::mpsc::Sender<WsEvent>>,
    pub(crate) ws_sequence: Arc<std::sync::atomic::AtomicU64>,
    pub(crate) mqtt_versions: Arc<std::sync::Mutex<std::collections::HashMap<String, u8>>>,
    pub(crate) req: Option<ProxiedRequest>,
    pub(crate) res: Option<ProxiedResponse>,
    pub(crate) intercept_rules: Arc<Mutex<Vec<InterceptRule>>>,
    pub(crate) server_replay_entries: Arc<Mutex<Vec<ServerReplayEntry>>>,
    pub(crate) cache_dir: Option<std::path::PathBuf>,
    pub(crate) script_handle: scripting::ScriptHandle,
    /// CA 인증서 DER 바이트 (외부 기기 인증서 다운로드용, zero-copy)
    pub(crate) ca_cert_der: Option<Bytes>,
}

impl LoggingHandler {
    pub fn new(
        sender: tokio::sync::mpsc::Sender<RequestInfo>,
        cache_dir: std::path::PathBuf,
    ) -> Self {
        Self {
            sender,
            ws_sender: None,
            ws_sequence: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            mqtt_versions: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            req: None,
            res: None,
            intercept_rules: Arc::new(Mutex::new(Vec::new())),
            server_replay_entries: Arc::new(Mutex::new(Vec::new())),
            cache_dir: Some(cache_dir),
            script_handle: scripting::ScriptHandle::new(),
            ca_cert_der: None,
        }
    }

    /// CA 인증서 DER 바이트를 설정합니다 (외부 기기 인증서 다운로드용)
    pub fn with_ca_cert_der(mut self, der: Vec<u8>) -> Self {
        self.ca_cert_der = Some(Bytes::from(der));
        self
    }

    pub fn with_ws_sender(mut self, ws_sender: tokio::sync::mpsc::Sender<WsEvent>) -> Self {
        self.ws_sender = Some(ws_sender);
        self
    }

    pub fn with_script_handle(mut self, handle: scripting::ScriptHandle) -> Self {
        self.script_handle = handle;
        self
    }

    /// 인터셉트 규칙 업데이트
    pub async fn update_intercept_rules(&self, rules: Vec<InterceptRule>) {
        let mut rules_guard = self.intercept_rules.lock().await;
        info!("[Intercept] 규칙 업데이트: {} 개", rules.len());
        *rules_guard = rules;
    }

    /// 서버 리플레이 엔트리 업데이트
    pub async fn update_server_replay_entries(&self, entries: Vec<ServerReplayEntry>) {
        let mut entries_guard = self.server_replay_entries.lock().await;
        info!("[ServerReplay] 엔트리 업데이트: {} 개", entries.len());
        *entries_guard = entries;
    }

    /// 스크립트 핸들 반환
    pub fn script_handle(&self) -> &scripting::ScriptHandle {
        &self.script_handle
    }

    /// CA 인증서 다운로드 응답을 생성합니다.
    /// `http://cheolsu.proxy/ssl` 또는 `/cert` 경로로 접근 시 .cer 파일 다운로드
    fn serve_ca_cert_download(&self, req: &Request<Body>) -> Response<Body> {
        let path = req.uri().path();

        // /ssl 또는 /cert 경로: 인증서 다운로드
        if path == "/ssl" || path == "/cert" || path == "/" {
            if let Some(der) = &self.ca_cert_der {
                info!(
                    "[CertDownload] CA 인증서 다운로드 제공 ({} bytes)",
                    der.len()
                );
                let body_bytes = der.clone(); // Bytes::clone은 참조카운트만 증가 (zero-copy)
                return Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "application/x-x509-ca-cert")
                    .header(
                        "Content-Disposition",
                        "attachment; filename=\"cheolsu-proxy-ca.cer\"",
                    )
                    .header("Content-Length", der.len().to_string())
                    .body(Body::from(http_body_util::Full::new(body_bytes)))
                    .unwrap_or_else(|_| {
                        Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Body::from("인증서 응답 생성 실패"))
                            .unwrap_or_else(|_| Response::new(Body::empty()))
                    });
            }
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header("Content-Type", "text/plain; charset=utf-8")
                .body(Body::from(
                    "CA 인증서가 아직 생성되지 않았습니다. 프록시를 먼저 실행해주세요.",
                ))
                .unwrap_or_else(|_| Response::new(Body::empty()));
        }

        // 그 외 경로: 안내 페이지
        let html = r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Cheolsu Proxy - CA Certificate</title>
<style>body{font-family:-apple-system,system-ui,sans-serif;display:flex;justify-content:center;align-items:center;min-height:100vh;margin:0;background:#f5f5f5}
.card{background:white;border-radius:12px;padding:40px;text-align:center;box-shadow:0 2px 8px rgba(0,0,0,0.1);max-width:400px}
h1{margin:0 0 8px;font-size:24px}p{color:#666;margin:8px 0}
a{display:inline-block;margin-top:20px;padding:12px 32px;background:#2563eb;color:white;border-radius:8px;text-decoration:none;font-weight:600}
a:hover{background:#1d4ed8}</style></head>
<body><div class="card"><h1>Cheolsu Proxy</h1><p>CA 인증서를 다운로드하여 이 기기에 설치하세요.</p>
<a href="/ssl">Download CA Certificate</a></div></body></html>"#;

        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/html; charset=utf-8")
            .body(Body::from(html))
            .unwrap_or_else(|_| Response::new(Body::empty()))
    }

    /// 요청과 응답을 묶어서 전송
    pub(crate) async fn send_output(&self) {
        let client_request = self
            .req
            .as_ref()
            .map(|req| req.clone().for_client(self.cache_dir.as_deref()));
        let client_response = self.res.as_ref().map(|res| {
            let request_id = self
                .req
                .as_ref()
                .map(|r| r.id().clone())
                .unwrap_or_default();
            res.clone()
                .for_client(&request_id, self.cache_dir.as_deref())
        });
        let request_info = RequestInfo(client_request, client_response);
        if let Err(e) = self.sender.send(request_info).await {
            error!("[LoggingHandler] 이벤트 전송 실패: {}", e);
        }
    }

    /// Request를 ProxiedRequest로 변환하고 원본 요청을 복원
    async fn request_to_proxied_request(
        &self,
        mut req: Request<Body>,
    ) -> (ProxiedRequest, Request<Body>) {
        let mut body_mut = req.body_mut();
        let body_bytes = match Self::body_to_bytes_from_mut(&mut body_mut).await {
            Ok(bytes) => bytes,
            Err(_) => Bytes::new(),
        };

        use http_body_util::Full;
        *body_mut = Body::from(Full::new(body_bytes.clone()));

        let proxied_request = ProxiedRequest::new(
            req.method().clone(),
            req.uri().clone(),
            req.version(),
            req.headers().clone(),
            body_bytes.clone(),
            chrono::Local::now()
                .timestamp_nanos_opt()
                .unwrap_or_default(),
        );

        (proxied_request, req)
    }

    /// Response를 ProxiedResponse로 변환하고 원본 응답을 복원
    async fn response_to_proxied_response(
        &self,
        mut res: Response<Body>,
    ) -> (ProxiedResponse, Response<Body>) {
        let mut body_mut = res.body_mut();
        let body_bytes = match Self::body_to_bytes_from_mut(&mut body_mut).await {
            Ok(bytes) => bytes,
            Err(_) => Bytes::new(),
        };

        use http_body_util::Full;
        *body_mut = Body::from(Full::new(body_bytes.clone()));

        let proxied_response = ProxiedResponse::new(
            res.status(),
            res.version(),
            res.headers().clone(),
            body_bytes.clone(),
            chrono::Local::now()
                .timestamp_nanos_opt()
                .unwrap_or_default(),
        );

        (proxied_response, res)
    }

    /// BodyMut를 Bytes로 변환하는 헬퍼 함수
    async fn body_to_bytes_from_mut(
        body_mut: &mut Body,
    ) -> Result<Bytes, Box<dyn std::error::Error + Send + Sync>> {
        use http_body_util::BodyExt;
        let body_bytes = body_mut.collect().await?.to_bytes();
        Ok(body_bytes)
    }

    fn create_response_from_cached_data(&self) -> Response<Body> {
        if let Some(cached_response) = &self.res {
            let mut response = Response::builder()
                .status(*cached_response.status())
                .version(*cached_response.version());

            for (key, value) in cached_response.headers() {
                response = response.header(key, value);
            }

            use http_body_util::Full;
            response
                .body(Body::from(Full::new(cached_response.body().clone())))
                .unwrap_or_else(|_| {
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::from("Failed to create response from cached data"))
                        .unwrap_or_else(|_| Response::new(Body::empty()))
                })
        } else {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("No cached response data available"))
                .unwrap_or_else(|_| Response::new(Body::empty()))
        }
    }

    fn extract_tls_info_from_error(
        &self,
        err: &hyper_util::client::legacy::Error,
    ) -> Option<(String, String)> {
        let err_str = err.to_string();

        let tls_version = if err_str.contains("TLS 1.0") || err_str.contains("TLS/1.0") {
            "TLS 1.0"
        } else if err_str.contains("TLS 1.1") || err_str.contains("TLS/1.1") {
            "TLS 1.1"
        } else if err_str.contains("TLS 1.2") || err_str.contains("TLS/1.2") {
            "TLS 1.2"
        } else if err_str.contains("TLS 1.3") || err_str.contains("TLS/1.3") {
            "TLS 1.3"
        } else if err_str.contains("SSL 3.0") || err_str.contains("SSL/3.0") {
            "SSL 3.0"
        } else if err_str.contains("handshake") || err_str.contains("TLS") {
            "TLS (버전 미확인)"
        } else {
            "알 수 없음"
        };

        let tls_backend = if err_str.contains("[RUSTLS]") || err_str.contains("rustls handshake") {
            "RUSTLS"
        } else if err_str.contains("[NATIVE-TLS]")
            || err_str.contains("native-tls handshake")
            || err_str.contains("PKCS12")
        {
            "NATIVE-TLS"
        } else if err_str.contains("rustls") || err_str.contains("RUSTLS") {
            "RUSTLS"
        } else if err_str.contains("native-tls")
            || err_str.contains("NATIVE-TLS")
            || err_str.contains("OpenSSL")
        {
            "NATIVE-TLS"
        } else if err_str.contains("handshake") || err_str.contains("TLS") {
            "TLS (백엔드 미확인)"
        } else {
            "알 수 없음"
        };

        Some((tls_version.to_string(), tls_backend.to_string()))
    }

    fn extract_target_server_from_error(
        &self,
        err: &hyper_util::client::legacy::Error,
    ) -> Option<String> {
        let err_str = err.to_string();

        if let Some(start) = err_str.find(" - ") {
            if let Some(end) = err_str[start + 3..].find(" - 오류:") {
                let server_info = &err_str[start + 3..start + 3 + end];
                if !server_info.is_empty() && server_info.contains(':') {
                    return Some(server_info.trim().to_string());
                }
            }
        }

        let patterns = [
            (r"([a-zA-Z0-9.-]+\.[a-zA-Z]{2,}:\d+)", "도메인:포트"),
            (r"(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}:\d+)", "IP:포트"),
            (r"([a-zA-Z0-9.-]+\.[a-zA-Z]{2,})", "도메인"),
        ];

        for (pattern, _description) in &patterns {
            if let Some(server_info) = Self::extract_pattern(&err_str, pattern) {
                if !server_info.is_empty() {
                    return Some(server_info);
                }
            }
        }

        if let Some(host) = Self::extract_host_from_url(&err_str) {
            return Some(host);
        }

        None
    }

    fn extract_pattern(text: &str, pattern: &str) -> Option<String> {
        if let Ok(regex) = Regex::new(pattern) {
            if let Some(captures) = regex.captures(text) {
                if let Some(matched) = captures.get(1) {
                    return Some(matched.as_str().to_string());
                }
            }
        }
        None
    }

    fn extract_host_from_url(text: &str) -> Option<String> {
        let url_patterns = ["https://", "http://"];

        for pattern in &url_patterns {
            if let Some(start) = text.find(pattern) {
                let after_protocol = &text[start + pattern.len()..];
                if let Some(end) = after_protocol.find('/') {
                    let host_part = &after_protocol[..end];
                    if !host_part.is_empty() && (host_part.contains('.') || host_part.contains(':'))
                    {
                        return Some(host_part.to_string());
                    }
                } else if !after_protocol.is_empty()
                    && (after_protocol.contains('.') || after_protocol.contains(':'))
                {
                    return Some(after_protocol.to_string());
                }
            }
        }

        None
    }
}

impl HttpHandler for LoggingHandler {
    async fn handle_request(
        &mut self,
        _ctx: &HttpContext,
        mut req: Request<Body>,
    ) -> RequestOrResponse {
        // cheolsu.proxy 호스트 요청 인터셉트: CA 인증서 다운로드 제공
        if let Some(host) = req.headers().get("host").and_then(|v| v.to_str().ok()) {
            if host == CERT_DOWNLOAD_HOST || host.starts_with(CERT_DOWNLOAD_HOST_COLON) {
                return self.serve_ca_cert_download(&req).into();
            }
        }
        // URI에서도 호스트 확인 (절대 URI 형식)
        if let Some(host) = req.uri().host() {
            if host == CERT_DOWNLOAD_HOST {
                return self.serve_ca_cert_download(&req).into();
            }
        }
        // 직접 IP 접속: URI가 상대 경로이고 /ssl 또는 /cert 경로인 경우 인증서 제공
        if req.uri().host().is_none() {
            let path = req.uri().path();
            if path == "/ssl" || path == "/cert" {
                return self.serve_ca_cert_download(&req).into();
            }
        }

        if req
            .headers()
            .get(proxyapi_v2::hyper::header::UPGRADE)
            .and_then(|v| v.to_str().ok())
            .map_or(false, |s| s.to_lowercase() == "websocket")
        {
            req.headers_mut()
                .remove(proxyapi_v2::hyper::header::SEC_WEBSOCKET_EXTENSIONS);
        }

        let (proxied_request, restored_req) = self.request_to_proxied_request(req).await;

        if restored_req.method() == Method::CONNECT || proxied_request.method() == "CONNECT" {
            return restored_req.into();
        }

        self.req = Some(proxied_request.clone());

        let url = proxied_request.uri().to_string();
        let method = proxied_request.method().to_string();

        // 서버 리플레이 매칭 확인 (인터셉트보다 우선)
        if let Some(entry) = self.find_server_replay_match(&url, &method).await {
            info!(
                "[ServerReplay] 매칭: {} {} -> status {} (id: {})",
                method, url, entry.status, entry.id
            );
            let mut response = Response::builder()
                .status(StatusCode::from_u16(entry.status).unwrap_or(StatusCode::OK))
                .header("x-cheolsu-server-replay", "true")
                .header("x-cheolsu-server-replay-id", &entry.id);

            for (name, value) in &entry.headers {
                response = response.header(name.as_str(), value.as_str());
            }

            let body_bytes = entry.body.unwrap_or_default();
            let res = response.body(Body::from(body_bytes)).unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("Server replay error"))
                    .unwrap()
            });
            self.send_output().await;
            return res.into();
        }

        // 스크립트 훅 적용 (인터셉트 규칙보다 먼저)
        let script_req = Self::to_script_request(&proxied_request);
        let restored_req = match self.script_handle.invoke_on_request(&script_req).await {
            Ok(scripting::RequestAction::Forward) => restored_req,
            Ok(scripting::RequestAction::ModifyRequest { request: modified }) => {
                info!("[Script] 요청 수정: {} {}", method, url);
                Self::apply_script_request_modify(restored_req, &modified)
            }
            Ok(scripting::RequestAction::Respond { response }) => {
                info!(
                    "[Script] 요청 차단: {} {} -> {}",
                    method, url, response.status
                );
                let res = Self::build_script_response(&response);
                self.send_output().await;
                return res.into();
            }
            Err(e) => {
                error!("[Script] onRequest 오류: {}", e);
                restored_req
            }
        };

        // 인터셉트 규칙 적용 (차단, 요청 수정)
        let result = self
            .apply_request_intercept(restored_req, &url, &method)
            .await;

        // 차단된 경우 로깅 출력
        if let RequestOrResponse::Response(_) = &result {
            self.send_output().await;
        }

        result
    }

    async fn handle_response(&mut self, _ctx: &HttpContext, res: Response<Body>) -> Response<Body> {
        if res.status() == StatusCode::SWITCHING_PROTOCOLS {
            return res;
        }

        // 인터셉트 규칙으로 응답 수정
        let res = if let Some(req) = &self.req {
            let url = req.uri().to_string();
            let method = req.method().to_string();
            self.apply_response_intercept(res, &url, &method).await
        } else {
            res
        };

        // 스크립트 훅으로 응답 수정
        let res = if let Some(req) = &self.req {
            let script_req = Self::to_script_request(req);
            let script_res = Self::to_script_response_from_hyper(&res);
            match self
                .script_handle
                .invoke_on_response(&script_req, &script_res)
                .await
            {
                Ok(scripting::ResponseAction::Forward) => res,
                Ok(scripting::ResponseAction::ModifyResponse { response: modified }) => {
                    info!("[Script] 응답 수정: {}", req.uri());
                    Self::apply_script_response_modify(res, &modified)
                }
                Err(e) => {
                    error!("[Script] onResponse 오류: {}", e);
                    res
                }
            }
        } else {
            res
        };

        let is_sse = res
            .headers()
            .get(proxyapi_v2::hyper::header::CONTENT_TYPE)
            .map_or(false, |v| {
                v.to_str().unwrap_or("").contains("text/event-stream")
            });

        if !is_sse {
            let (proxied_response, restored_res) = self.response_to_proxied_response(res).await;
            self.res = Some(proxied_response);
            self.send_output().await;
            return restored_res;
        }

        // --- SSE 스트리밍 처리 로직 ---
        let (parts, body) = res.into_parts();

        let (tx, rx) = tokio::sync::mpsc::channel(4);

        let stream = ReceiverStream::new(rx).map(Ok::<_, proxyapi_v2::Error>);
        let stream_body = StreamBody::new(stream);

        let response_for_client = Response::from_parts(parts.clone(), Body::from(stream_body));

        let mut handler_clone = self.clone();

        tokio::spawn(async move {
            let mut body_stream = body;
            let mut collected_chunks = Vec::new();

            while let Some(frame_result) = body_stream.frame().await {
                match frame_result {
                    Ok(frame) => {
                        if let Some(data) = frame.data_ref() {
                            collected_chunks.extend_from_slice(data);
                        }

                        if tx.send(frame).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        error!("[SSE Stream] Error reading from upstream: {:?}", e);
                        break;
                    }
                }
            }

            let proxied_response = ProxiedResponse::new(
                parts.status,
                parts.version,
                parts.headers,
                Bytes::from(collected_chunks),
                chrono::Local::now()
                    .timestamp_nanos_opt()
                    .unwrap_or_default(),
            );

            handler_clone.res = Some(proxied_response);
            handler_clone.send_output().await;
        });

        response_for_client
    }

    async fn handle_error(
        &mut self,
        _ctx: &HttpContext,
        err: hyper_util::client::legacy::Error,
    ) -> Response<Body> {
        let tls_info = self.extract_tls_info_from_error(&err);
        let target_server = self.extract_target_server_from_error(&err);

        if let Some(source) = err.source() {
            let source_str = source.to_string();
            if source_str.contains("UnexpectedEof") || source_str.contains("unexpected EOF") {
                debug!(
                    error = %err,
                    target = ?target_server,
                    "TLS close_notify 없이 연결 종료됨 - 정상 종료로 처리"
                );

                if self.res.is_some() {
                    return self.create_response_from_cached_data();
                } else {
                    return Response::builder()
                        .status(StatusCode::OK)
                        .body(Body::empty())
                        .unwrap_or_else(|_| Response::new(Body::empty()));
                }
            }
        }

        error!(
            error = %err,
            target = ?target_server,
            tls_info = ?tls_info,
            source = ?err.source().map(|s| s.to_string()),
            "프록시 요청 오류"
        );

        let should_use_curl = err
            .source()
            .map(|s| s.to_string().contains("HandshakeFailure"))
            .unwrap_or(false);

        if should_use_curl {
            if let Some(req) = &self.req {
                error!("TLS 핸드셰이크 실패 - curl 폴백 시도");
                match crate::curl_fallback::fallback_with_curl(req).await {
                    Ok(response) => {
                        info!("curl 폴백 성공");
                        return response;
                    }
                    Err(curl_err) => {
                        error!(error = %curl_err, "curl 폴백도 실패");
                    }
                }
            }
        }

        Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .body(Body::from(format!("Proxy Error: {}", err)))
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::empty())
                    .unwrap_or_else(|_| Response::new(Body::empty()))
            })
    }
}

impl WebSocketHandler for LoggingHandler {
    async fn on_connected(&mut self, ctx: &WebSocketContext) {
        if let Some(ws_sender) = &self.ws_sender {
            let (connection_id, uri) = match ctx {
                WebSocketContext::ClientToServer { dst, .. } => (dst.to_string(), dst.to_string()),
                WebSocketContext::ServerToClient { src, .. } => (src.to_string(), src.to_string()),
            };
            let event = WsConnectionEvent::Connected {
                connection_id,
                uri,
                time: chrono::Local::now()
                    .timestamp_nanos_opt()
                    .unwrap_or_default(),
            };
            let _ = ws_sender.try_send(WsEvent::Connection(event));
        }
    }

    async fn on_disconnected(&mut self, ctx: &WebSocketContext) {
        if let Some(ws_sender) = &self.ws_sender {
            let connection_id = match ctx {
                WebSocketContext::ClientToServer { dst, .. } => dst.to_string(),
                WebSocketContext::ServerToClient { src, .. } => src.to_string(),
            };
            let event = WsConnectionEvent::Disconnected {
                connection_id,
                time: chrono::Local::now()
                    .timestamp_nanos_opt()
                    .unwrap_or_default(),
            };
            let _ = ws_sender.try_send(WsEvent::Connection(event));
        }
    }

    async fn handle_message(&mut self, ctx: &WebSocketContext, msg: Message) -> Option<Message> {
        let (direction, connection_id) = match ctx {
            WebSocketContext::ClientToServer { dst, .. } => {
                (WsDirection::ClientToServer, dst.to_string())
            }
            WebSocketContext::ServerToClient { src, .. } => {
                (WsDirection::ServerToClient, src.to_string())
            }
        };

        let (message_type, payload, size, is_binary) = match &msg {
            Message::Text(text) => (WsMessageType::Text, text.to_string(), text.len(), false),
            Message::Binary(data) => {
                use base64::Engine;
                let encoded = base64::engine::general_purpose::STANDARD.encode(data);
                (WsMessageType::Binary, encoded, data.len(), true)
            }
            Message::Ping(data) => (
                WsMessageType::Ping,
                format!("{} bytes", data.len()),
                data.len(),
                true,
            ),
            Message::Pong(data) => (
                WsMessageType::Pong,
                format!("{} bytes", data.len()),
                data.len(),
                true,
            ),
            Message::Close(frame) => {
                let payload = frame
                    .as_ref()
                    .map(|f| format!("{}: {}", f.code, f.reason))
                    .unwrap_or_default();
                let size = payload.len();
                (WsMessageType::Close, payload, size, false)
            }
            Message::Frame(_) => return Some(msg),
        };

        // 스크립트 onWebSocketMessage 훅 적용 (Text/Binary 메시지만)
        let (msg, payload, is_binary) =
            if matches!(message_type, WsMessageType::Text | WsMessageType::Binary) {
                let script_direction = match ctx {
                    WebSocketContext::ClientToServer { .. } => scripting::WsDirection::ToServer,
                    WebSocketContext::ServerToClient { .. } => scripting::WsDirection::ToClient,
                };
                let url = match ctx {
                    WebSocketContext::ClientToServer { dst, .. } => dst.to_string(),
                    WebSocketContext::ServerToClient { src, .. } => src.to_string(),
                };
                let script_msg = scripting::ScriptWsMessage {
                    connection_id: connection_id.clone(),
                    url,
                    direction: script_direction,
                    payload: payload.clone(),
                    is_binary,
                };
                match self.script_handle.invoke_on_ws_message(&script_msg).await {
                    Ok(scripting::WsAction::Forward) => (msg, payload, is_binary),
                    Ok(scripting::WsAction::Modify {
                        payload: new_payload,
                        is_binary: new_is_binary,
                    }) => {
                        let new_msg = if new_is_binary {
                            use base64::Engine;
                            match base64::engine::general_purpose::STANDARD.decode(&new_payload) {
                                Ok(data) => Message::Binary(data.into()),
                                Err(_) => Message::Text(new_payload.clone().into()),
                            }
                        } else {
                            Message::Text(new_payload.clone().into())
                        };
                        (new_msg, new_payload, new_is_binary)
                    }
                    Ok(scripting::WsAction::Drop) => {
                        return None;
                    }
                    Err(e) => {
                        error!("[Script] onWebSocketMessage 오류: {}", e);
                        (msg, payload, is_binary)
                    }
                }
            } else {
                (msg, payload, is_binary)
            };

        if let Some(ws_sender) = &self.ws_sender {
            let sequence = self
                .ws_sequence
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            let content_type = proxy_v2_models::detect_ws_content_type(&payload, is_binary);

            // MQTT 메시지인 경우 버전 추적
            let mqtt_version = if content_type == proxy_v2_models::WsContentType::Mqtt {
                // CONNECT 패킷이면 버전을 추출하여 저장
                if let Some(ver) = proxy_v2_models::extract_mqtt_version_from_connect(&payload) {
                    if let Ok(mut versions) = self.mqtt_versions.lock() {
                        versions.insert(connection_id.clone(), ver);
                    }
                    Some(ver)
                } else {
                    // 다른 MQTT 패킷이면 저장된 버전 참조
                    self.mqtt_versions
                        .lock()
                        .ok()
                        .and_then(|versions| versions.get(&connection_id).copied())
                }
            } else {
                None
            };

            let info = WsMessageInfo {
                connection_id,
                sequence,
                direction,
                message_type,
                payload,
                size,
                time: chrono::Local::now()
                    .timestamp_nanos_opt()
                    .unwrap_or_default(),
                is_binary,
                content_type,
                mqtt_version,
            };

            let _ = ws_sender.try_send(WsEvent::Message(info));
        }

        Some(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 테스트용 LoggingHandler를 생성합니다.
    fn make_test_handler(ca_cert_der: Option<Vec<u8>>) -> LoggingHandler {
        let (sender, _rx) = tokio::sync::mpsc::channel(1);
        let mut handler = LoggingHandler::new(sender, std::path::PathBuf::from("/tmp"));
        if let Some(der) = ca_cert_der {
            handler = handler.with_ca_cert_der(der);
        }
        handler
    }

    #[test]
    fn cert_download_host_constants() {
        assert_eq!(CERT_DOWNLOAD_HOST, "cheolsu.proxy");
        assert!(CERT_DOWNLOAD_HOST_COLON.starts_with(CERT_DOWNLOAD_HOST));
        assert!(CERT_DOWNLOAD_HOST_COLON.ends_with(':'));
    }

    #[test]
    fn serve_cert_download_ssl_path_with_cert() {
        let handler = make_test_handler(Some(vec![0xDE, 0xAD, 0xBE, 0xEF]));
        let req = Request::builder()
            .uri("/ssl")
            .header("host", "cheolsu.proxy")
            .body(Body::from(""))
            .unwrap();
        let resp = handler.serve_ca_cert_download(&req);
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get("Content-Type").unwrap(),
            "application/x-x509-ca-cert"
        );
        assert_eq!(resp.headers().get("Content-Length").unwrap(), "4");
        assert!(resp
            .headers()
            .get("Content-Disposition")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("cheolsu-proxy-ca.cer"));
    }

    #[test]
    fn serve_cert_download_cert_path_with_cert() {
        let handler = make_test_handler(Some(vec![1, 2, 3]));
        let req = Request::builder()
            .uri("/cert")
            .body(Body::from(""))
            .unwrap();
        let resp = handler.serve_ca_cert_download(&req);
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get("Content-Type").unwrap(),
            "application/x-x509-ca-cert"
        );
    }

    #[test]
    fn serve_cert_download_root_path_with_cert() {
        let handler = make_test_handler(Some(vec![1]));
        let req = Request::builder().uri("/").body(Body::from("")).unwrap();
        let resp = handler.serve_ca_cert_download(&req);
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get("Content-Type").unwrap(),
            "application/x-x509-ca-cert"
        );
    }

    #[test]
    fn serve_cert_download_ssl_path_without_cert() {
        let handler = make_test_handler(None);
        let req = Request::builder().uri("/ssl").body(Body::from("")).unwrap();
        let resp = handler.serve_ca_cert_download(&req);
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        assert!(resp
            .headers()
            .get("Content-Type")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("text/plain"));
    }

    #[test]
    fn serve_cert_download_other_path_returns_html() {
        let handler = make_test_handler(Some(vec![1]));
        let req = Request::builder()
            .uri("/about")
            .body(Body::from(""))
            .unwrap();
        let resp = handler.serve_ca_cert_download(&req);
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(resp
            .headers()
            .get("Content-Type")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("text/html"));
    }

    #[test]
    fn with_ca_cert_der_sets_bytes() {
        let handler = make_test_handler(None);
        assert!(handler.ca_cert_der.is_none());

        let handler = make_test_handler(Some(vec![0xFF, 0x00]));
        assert!(handler.ca_cert_der.is_some());
        assert_eq!(handler.ca_cert_der.unwrap().len(), 2);
    }

    #[test]
    fn host_matching_exact() {
        assert_eq!("cheolsu.proxy", CERT_DOWNLOAD_HOST);
        assert!("cheolsu.proxy:8080".starts_with(CERT_DOWNLOAD_HOST_COLON));
        assert!(!"other.proxy:8080".starts_with(CERT_DOWNLOAD_HOST_COLON));
        assert!(!"cheolsu.proxy.evil.com".starts_with(CERT_DOWNLOAD_HOST_COLON));
    }
}

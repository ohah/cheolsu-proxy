use crate::breakpoint::BreakpointManager;
use crate::protocol::{
    BreakpointAction, BreakpointData, BreakpointPhase, HostMapping, InterceptRule,
    ServerReplayEntry,
};
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

use crate::cert_distribution;

/// 요청별 임시 상태 (매 요청마다 초기화)
#[derive(Clone)]
pub(crate) struct RequestState {
    pub(crate) req: Option<ProxiedRequest>,
    pub(crate) res: Option<ProxiedResponse>,
}

/// 프록시 전역 설정 (세션 수명 동안 유지)
#[derive(Clone)]
pub(crate) struct ProxyConfig {
    pub(crate) cache_dir: Option<std::path::PathBuf>,
    /// CA 인증서 DER 바이트 (외부 기기 인증서 다운로드용, zero-copy)
    pub(crate) ca_cert_der: Option<Bytes>,
}

/// 인터셉트 규칙 및 스크립트 엔진
#[derive(Clone)]
pub(crate) struct InterceptEngine {
    pub(crate) intercept_rules: Arc<Mutex<Vec<InterceptRule>>>,
    pub(crate) server_replay_entries: Arc<Mutex<Vec<ServerReplayEntry>>>,
    pub(crate) host_mappings: Arc<Mutex<Vec<HostMapping>>>,
    pub(crate) script_handle: scripting::ScriptHandle,
}

/// WebSocket 상태 관리
#[derive(Clone)]
pub(crate) struct WebSocketState {
    pub(crate) ws_sender: Option<tokio::sync::mpsc::Sender<WsEvent>>,
    pub(crate) ws_sequence: Arc<std::sync::atomic::AtomicU64>,
    pub(crate) mqtt_versions: Arc<parking_lot::Mutex<std::collections::HashMap<String, u8>>>,
}

/// HTTP 및 WebSocket 요청/응답을 로깅하는 핸들러
#[derive(Clone)]
pub struct LoggingHandler {
    pub(crate) sender: tokio::sync::mpsc::Sender<RequestInfo>,
    pub(crate) request: RequestState,
    pub(crate) config: ProxyConfig,
    pub(crate) intercept: InterceptEngine,
    pub(crate) ws: WebSocketState,
    pub(crate) breakpoint_manager: Option<BreakpointManager>,
}

impl LoggingHandler {
    pub fn new(
        sender: tokio::sync::mpsc::Sender<RequestInfo>,
        cache_dir: std::path::PathBuf,
    ) -> Self {
        Self {
            sender,
            request: RequestState {
                req: None,
                res: None,
            },
            config: ProxyConfig {
                cache_dir: Some(cache_dir),
                ca_cert_der: None,
            },
            intercept: InterceptEngine {
                intercept_rules: Arc::new(Mutex::new(Vec::new())),
                server_replay_entries: Arc::new(Mutex::new(Vec::new())),
                host_mappings: Arc::new(Mutex::new(Vec::new())),
                script_handle: scripting::ScriptHandle::new(),
            },
            ws: WebSocketState {
                ws_sender: None,
                ws_sequence: Arc::new(std::sync::atomic::AtomicU64::new(0)),
                mqtt_versions: Arc::new(parking_lot::Mutex::new(std::collections::HashMap::new())),
            },
            breakpoint_manager: None,
        }
    }

    /// CA 인증서 DER 바이트를 설정합니다 (외부 기기 인증서 다운로드용)
    pub fn with_ca_cert_der(mut self, der: Vec<u8>) -> Self {
        self.config.ca_cert_der = Some(Bytes::from(der));
        self
    }

    pub fn with_ws_sender(mut self, ws_sender: tokio::sync::mpsc::Sender<WsEvent>) -> Self {
        self.ws.ws_sender = Some(ws_sender);
        self
    }

    pub fn with_script_handle(mut self, handle: scripting::ScriptHandle) -> Self {
        self.intercept.script_handle = handle;
        self
    }

    pub fn with_breakpoint_manager(mut self, mgr: BreakpointManager) -> Self {
        self.breakpoint_manager = Some(mgr);
        self
    }

    pub fn breakpoint_manager(&self) -> Option<&BreakpointManager> {
        self.breakpoint_manager.as_ref()
    }

    /// 인터셉트 규칙 업데이트
    pub async fn update_intercept_rules(&self, rules: Vec<InterceptRule>) {
        let mut rules_guard = self.intercept.intercept_rules.lock().await;
        info!("[Intercept] 규칙 업데이트: {} 개", rules.len());
        *rules_guard = rules;
    }

    /// 서버 리플레이 엔트리 업데이트
    pub async fn update_server_replay_entries(&self, entries: Vec<ServerReplayEntry>) {
        let mut entries_guard = self.intercept.server_replay_entries.lock().await;
        info!("[ServerReplay] 엔트리 업데이트: {} 개", entries.len());
        *entries_guard = entries;
    }

    /// Update host mappings
    pub async fn update_host_mappings(&self, mappings: Vec<HostMapping>) {
        let mut mappings_guard = self.intercept.host_mappings.lock().await;
        info!("[HostMapping] mappings updated: {} entries", mappings.len());
        *mappings_guard = mappings;
    }

    /// 스크립트 핸들 반환
    pub fn script_handle(&self) -> &scripting::ScriptHandle {
        &self.intercept.script_handle
    }

    fn serve_ca_cert_download(&self, req: &Request<Body>) -> Response<Body> {
        cert_distribution::handle_cert_request(req, self.config.ca_cert_der.as_ref())
    }

    /// 요청과 응답을 묶어서 전송
    pub(crate) async fn send_output(&self) {
        let client_request = self
            .request
            .req
            .as_ref()
            .map(|req| req.clone().for_client(self.config.cache_dir.as_deref()));
        let client_response = self.request.res.as_ref().map(|res| {
            let request_id = self
                .request
                .req
                .as_ref()
                .map(|r| r.id().clone())
                .unwrap_or_default();
            res.clone()
                .for_client(&request_id, self.config.cache_dir.as_deref())
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
        if let Some(cached_response) = &self.request.res {
            let mut response = Response::builder()
                .status(*cached_response.status())
                .version(*cached_response.version());

            for (key, value) in cached_response.headers() {
                response = response.header(key, value);
            }

            use http_body_util::Full;
            response
                .body(Body::from(Full::new(cached_response.body().clone())))
                .unwrap_or_else(|_| Response::new(Body::empty()))
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

    /// WebSocketContext에서 방향, connection_id, URL을 추출합니다.
    fn extract_ws_context(ctx: &WebSocketContext) -> (WsDirection, String, String) {
        match ctx {
            WebSocketContext::ClientToServer { dst, .. } => {
                let url = dst.to_string();
                (WsDirection::ClientToServer, url.clone(), url)
            }
            WebSocketContext::ServerToClient { src, .. } => {
                let url = src.to_string();
                (WsDirection::ServerToClient, url.clone(), url)
            }
        }
    }

    /// WebSocket 메시지를 (message_type, payload, size, is_binary) 튜플로 변환합니다.
    /// Message::Frame은 None을 반환합니다.
    fn convert_ws_message_payload(msg: &Message) -> Option<(WsMessageType, String, usize, bool)> {
        match msg {
            Message::Text(text) => Some((WsMessageType::Text, text.to_string(), text.len(), false)),
            Message::Binary(data) => {
                use base64::Engine;
                let encoded = base64::engine::general_purpose::STANDARD.encode(data);
                Some((WsMessageType::Binary, encoded, data.len(), true))
            }
            Message::Ping(data) => Some((
                WsMessageType::Ping,
                format!("{} bytes", data.len()),
                data.len(),
                true,
            )),
            Message::Pong(data) => Some((
                WsMessageType::Pong,
                format!("{} bytes", data.len()),
                data.len(),
                true,
            )),
            Message::Close(frame) => {
                let payload = frame
                    .as_ref()
                    .map(|f| format!("{}: {}", f.code, f.reason))
                    .unwrap_or_default();
                let size = payload.len();
                Some((WsMessageType::Close, payload, size, false))
            }
            Message::Frame(_) => None,
        }
    }

    /// Text/Binary 메시지에 대해 스크립트 onWebSocketMessage 훅을 적용합니다.
    /// Drop이면 None 반환, Forward/Modify면 (변경된 msg, payload, is_binary) 반환.
    async fn apply_ws_script_hook(
        &self,
        ctx: &WebSocketContext,
        msg: Message,
        connection_id: &str,
        url: &str,
        message_type: WsMessageType,
        payload: String,
        is_binary: bool,
    ) -> Option<(Message, String, bool)> {
        if !matches!(message_type, WsMessageType::Text | WsMessageType::Binary) {
            return Some((msg, payload, is_binary));
        }

        let script_direction = match ctx {
            WebSocketContext::ClientToServer { .. } => scripting::WsDirection::ToServer,
            WebSocketContext::ServerToClient { .. } => scripting::WsDirection::ToClient,
        };
        let script_msg = scripting::ScriptWsMessage {
            connection_id: connection_id.to_string(),
            url: url.to_string(),
            direction: script_direction,
            payload: payload.clone(),
            is_binary,
        };
        match self
            .intercept
            .script_handle
            .invoke_on_ws_message(&script_msg)
            .await
        {
            Ok(scripting::WsAction::Forward) => Some((msg, payload, is_binary)),
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
                Some((new_msg, new_payload, new_is_binary))
            }
            Ok(scripting::WsAction::Drop) => None,
            Err(e) => {
                error!("[Script] onWebSocketMessage 오류: {}", e);
                Some((msg, payload, is_binary))
            }
        }
    }

    /// WebSocket 이벤트를 생성하여 ws_sender로 전송합니다.
    fn emit_ws_event(
        &self,
        connection_id: String,
        direction: WsDirection,
        message_type: WsMessageType,
        payload: String,
        size: usize,
        is_binary: bool,
    ) {
        let Some(ws_sender) = &self.ws.ws_sender else {
            return;
        };

        let sequence = self
            .ws
            .ws_sequence
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let content_type = proxy_v2_models::detect_ws_content_type(&payload, is_binary);

        let mqtt_version = if content_type == proxy_v2_models::WsContentType::Mqtt {
            if let Some(ver) = proxy_v2_models::extract_mqtt_version_from_connect(&payload) {
                self.ws
                    .mqtt_versions
                    .lock()
                    .insert(connection_id.clone(), ver);
                Some(ver)
            } else {
                self.ws.mqtt_versions.lock().get(&connection_id).copied()
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

    /// Apply host mapping to the request if a matching rule exists.
    /// Rewrites the URI to point to the mapped target host/port,
    /// while preserving the original Host header for correct virtual host routing.
    async fn apply_host_mapping_if_needed(&self, mut req: Request<Body>) -> Request<Body> {
        let (host, port) = Self::extract_host_port(req.uri());
        let Some(host) = host else {
            return req;
        };

        if let Some((target_host, target_port)) =
            self.resolve_host_mapping(&host, port).await
        {
            info!(
                "[HostMapping] {}:{} -> {}:{}",
                host,
                port.map(|p| p.to_string()).unwrap_or_else(|| "default".to_string()),
                target_host,
                target_port.map(|p| p.to_string()).unwrap_or_else(|| "default".to_string()),
            );

            if let Some(new_uri) = Self::apply_host_mapping_to_uri(req.uri(), &target_host, target_port) {
                *req.uri_mut() = new_uri;
                // Keep the original Host header intact so the server
                // can route to the correct virtual host.
                req.headers_mut().insert(
                    "x-cheolsu-host-mapped",
                    proxyapi_v2::hyper::http::HeaderValue::from_static("true"),
                );
            }
        }

        req
    }


    fn check_cert_download_intercept(&self, req: &Request<Body>) -> Option<Response<Body>> {
        if cert_distribution::is_cert_download_request(req) {
            Some(self.serve_ca_cert_download(req))
        } else {
            None
        }
    }

    /// 서버 리플레이 매칭을 확인하고, 매칭되면 응답을 생성합니다.
    async fn check_server_replay(&self, url: &str, method: &str) -> Option<Response<Body>> {
        let entry = self.find_server_replay_match(url, method).await?;
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
        Some(
            response
                .body(Body::from(body_bytes))
                .unwrap_or_else(|_| Response::new(Body::empty())),
        )
    }

    /// 스크립트 on_request 훅을 적용합니다.
    /// Respond이면 Err(Response) 반환, Forward/Modify이면 Ok(Request) 반환.
    async fn apply_script_on_request(
        &self,
        req: Request<Body>,
        proxied_request: &ProxiedRequest,
        method: &str,
        url: &str,
    ) -> Result<Request<Body>, Response<Body>> {
        let script_req = Self::to_script_request(proxied_request);
        match self
            .intercept
            .script_handle
            .invoke_on_request(&script_req)
            .await
        {
            Ok(scripting::RequestAction::Forward) => Ok(req),
            Ok(scripting::RequestAction::ModifyRequest { request: modified }) => {
                info!("[Script] 요청 수정: {} {}", method, url);
                Ok(Self::apply_script_request_modify(req, &modified))
            }
            Ok(scripting::RequestAction::Respond { response }) => {
                info!(
                    "[Script] 요청 차단: {} {} -> {}",
                    method, url, response.status
                );
                Err(Self::build_script_response(&response))
            }
            Err(e) => {
                error!("[Script] onRequest 오류: {}", e);
                Ok(req)
            }
        }
    }

    /// 인터셉트 규칙으로 응답을 수정합니다.
    async fn apply_response_intercept_if_needed(&self, res: Response<Body>) -> Response<Body> {
        if let Some(req) = &self.request.req {
            let url = req.uri().to_string();
            let method = req.method().to_string();
            self.apply_response_intercept(res, &url, &method).await
        } else {
            res
        }
    }

    /// Apply breakpoint check on request phase.
    /// If a breakpoint matches, pauses and waits for resolution.
    /// Returns either the (possibly modified) request, or a Response to short-circuit.
    async fn apply_request_breakpoint(
        &self,
        req: Request<Body>,
        url: &str,
        method: &str,
        transaction_id: &str,
    ) -> Result<Request<Body>, Response<Body>> {
        let Some(mgr) = &self.breakpoint_manager else {
            return Ok(req);
        };
        if !mgr.should_break(url, &BreakpointPhase::Request).await {
            return Ok(req);
        }

        info!("[Breakpoint] Request paused: {} {}", method, url);

        let headers: std::collections::HashMap<String, String> = req
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("<binary>").to_string()))
            .collect();

        let data = BreakpointData {
            method: method.to_string(),
            url: url.to_string(),
            headers,
            body: None,
            status: None,
        };

        let action = mgr
            .pause_and_wait(transaction_id, BreakpointPhase::Request, data)
            .await;

        match action {
            BreakpointAction::Forward => Ok(req),
            BreakpointAction::ModifyAndForward {
                headers: new_headers,
                body: new_body,
                ..
            } => {
                let mut req = req;
                if let Some(hdrs) = new_headers {
                    for (name, value) in hdrs {
                        if let (Ok(header_name), Ok(header_value)) = (
                            name.parse::<proxyapi_v2::hyper::http::HeaderName>(),
                            value.parse::<proxyapi_v2::hyper::http::HeaderValue>(),
                        ) {
                            req.headers_mut().insert(header_name, header_value);
                        }
                    }
                }
                if let Some(body) = new_body {
                    use http_body_util::Full;
                    *req.body_mut() = Body::from(Full::new(bytes::Bytes::from(body)));
                }
                Ok(req)
            }
            BreakpointAction::Drop | BreakpointAction::Abort => {
                let status = if matches!(action, BreakpointAction::Abort) {
                    StatusCode::BAD_GATEWAY
                } else {
                    StatusCode::SERVICE_UNAVAILABLE
                };
                let response = Response::builder()
                    .status(status)
                    .header("x-cheolsu-breakpoint", "dropped")
                    .body(Body::from("Request dropped by breakpoint"))
                    .unwrap_or_else(|_| Response::new(Body::empty()));
                Err(response)
            }
        }
    }

    /// Apply breakpoint check on response phase.
    async fn apply_response_breakpoint(
        &self,
        res: Response<Body>,
        url: &str,
        method: &str,
        transaction_id: &str,
    ) -> Response<Body> {
        let Some(mgr) = &self.breakpoint_manager else {
            return res;
        };
        if !mgr.should_break(url, &BreakpointPhase::Response).await {
            return res;
        }

        info!("[Breakpoint] Response paused: {} {}", method, url);

        let headers: std::collections::HashMap<String, String> = res
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("<binary>").to_string()))
            .collect();

        let data = BreakpointData {
            method: method.to_string(),
            url: url.to_string(),
            headers,
            body: None,
            status: Some(res.status().as_u16()),
        };

        let action = mgr
            .pause_and_wait(transaction_id, BreakpointPhase::Response, data)
            .await;

        match action {
            BreakpointAction::Forward => res,
            BreakpointAction::ModifyAndForward {
                headers: new_headers,
                body: new_body,
                status: new_status,
            } => {
                let mut res = res;
                if let Some(status) = new_status {
                    if let Ok(status_code) = StatusCode::from_u16(status) {
                        *res.status_mut() = status_code;
                    }
                }
                if let Some(hdrs) = new_headers {
                    for (name, value) in hdrs {
                        if let (Ok(header_name), Ok(header_value)) = (
                            name.parse::<proxyapi_v2::hyper::http::HeaderName>(),
                            value.parse::<proxyapi_v2::hyper::http::HeaderValue>(),
                        ) {
                            res.headers_mut().insert(header_name, header_value);
                        }
                    }
                }
                if let Some(body) = new_body {
                    use http_body_util::Full;
                    res.headers_mut().remove("content-length");
                    res.headers_mut().remove("content-encoding");
                    res.headers_mut().remove("transfer-encoding");
                    *res.body_mut() = Body::from(Full::new(bytes::Bytes::from(body)));
                }
                res
            }
            BreakpointAction::Drop | BreakpointAction::Abort => {
                let status = if matches!(action, BreakpointAction::Abort) {
                    StatusCode::BAD_GATEWAY
                } else {
                    StatusCode::SERVICE_UNAVAILABLE
                };
                Response::builder()
                    .status(status)
                    .header("x-cheolsu-breakpoint", "dropped")
                    .body(Body::from("Response dropped by breakpoint"))
                    .unwrap_or_else(|_| Response::new(Body::empty()))
            }
        }
    }

    /// 스크립트 on_response 훅을 적용합니다.
    async fn apply_script_on_response(&self, res: Response<Body>) -> Response<Body> {
        let Some(req) = &self.request.req else {
            return res;
        };
        let script_req = Self::to_script_request(req);
        let script_res = Self::to_script_response_from_hyper(&res);
        match self
            .intercept
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
    }

    /// SSE(Server-Sent Events) 응답을 스트리밍 처리합니다.
    fn handle_sse_streaming(&mut self, res: Response<Body>) -> Response<Body> {
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

            handler_clone.request.res = Some(proxied_response);
            handler_clone.send_output().await;
        });

        response_for_client
    }
}

impl HttpHandler for LoggingHandler {
    async fn handle_request(
        &mut self,
        _ctx: &HttpContext,
        mut req: Request<Body>,
    ) -> RequestOrResponse {
        if let Some(cert_response) = self.check_cert_download_intercept(&req) {
            return cert_response.into();
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

        self.request.req = Some(proxied_request.clone());

        let url = proxied_request.uri().to_string();
        let method = proxied_request.method().to_string();

        if let Some(replay_response) = self.check_server_replay(&url, &method).await {
            self.send_output().await;
            return replay_response.into();
        }

        let restored_req = match self
            .apply_script_on_request(restored_req, &proxied_request, &method, &url)
            .await
        {
            Ok(req) => req,
            Err(response) => {
                self.send_output().await;
                return response.into();
            }
        };

        let transaction_id = self
            .request
            .req
            .as_ref()
            .map(|r| r.id().clone())
            .unwrap_or_default();

        let restored_req = match self
            .apply_request_breakpoint(restored_req, &url, &method, &transaction_id)
            .await
        {
            Ok(req) => req,
            Err(response) => {
                self.send_output().await;
                return response.into();
            }
        };

        let restored_req = self.apply_host_mapping_if_needed(restored_req).await;

        let result = self
            .apply_request_intercept(restored_req, &url, &method)
            .await;

        if let RequestOrResponse::Response(_) = &result {
            self.send_output().await;
        }

        result
    }

    async fn handle_response(&mut self, _ctx: &HttpContext, res: Response<Body>) -> Response<Body> {
        if res.status() == StatusCode::SWITCHING_PROTOCOLS {
            return res;
        }

        let res = self.apply_response_intercept_if_needed(res).await;
        let res = self.apply_script_on_response(res).await;

        let res = if let Some(req) = &self.request.req {
            let url = req.uri().to_string();
            let method = req.method().to_string();
            let transaction_id = req.id().clone();
            self.apply_response_breakpoint(res, &url, &method, &transaction_id)
                .await
        } else {
            res
        };

        let is_sse = res
            .headers()
            .get(proxyapi_v2::hyper::header::CONTENT_TYPE)
            .map_or(false, |v| {
                v.to_str().unwrap_or("").contains("text/event-stream")
            });

        if is_sse {
            return self.handle_sse_streaming(res);
        }

        let (proxied_response, restored_res) = self.response_to_proxied_response(res).await;
        self.request.res = Some(proxied_response);
        self.send_output().await;
        restored_res
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

                if self.request.res.is_some() {
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
            if let Some(req) = &self.request.req {
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
            .unwrap_or_else(|_| Response::new(Body::empty()))
    }
}

impl WebSocketHandler for LoggingHandler {
    async fn on_connected(&mut self, ctx: &WebSocketContext) {
        if let Some(ws_sender) = &self.ws.ws_sender {
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
        if let Some(ws_sender) = &self.ws.ws_sender {
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
        let (direction, connection_id, url) = Self::extract_ws_context(ctx);

        let (message_type, payload, size, is_binary) = match Self::convert_ws_message_payload(&msg)
        {
            Some(tuple) => tuple,
            None => return Some(msg),
        };

        let (msg, payload, is_binary) = self
            .apply_ws_script_hook(
                ctx,
                msg,
                &connection_id,
                &url,
                message_type,
                payload,
                is_binary,
            )
            .await?;

        self.emit_ws_event(
            connection_id,
            direction,
            message_type,
            payload,
            size,
            is_binary,
        );
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
    fn cert_distribution_module_exists() {
        let req = Request::builder()
            .uri("/ssl")
            .header("host", "cheolsu.proxy")
            .body(Body::from(""))
            .unwrap();
        assert!(cert_distribution::is_cert_download_request(&req));
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
    fn serve_cert_download_root_path_shows_landing_page() {
        let handler = make_test_handler(Some(vec![1]));
        let req = Request::builder().uri("/").body(Body::from("")).unwrap();
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
    fn serve_cert_download_ssl_path_without_cert() {
        let handler = make_test_handler(None);
        let req = Request::builder().uri("/ssl").body(Body::from("")).unwrap();
        let resp = handler.serve_ca_cert_download(&req);
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
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
        assert!(handler.config.ca_cert_der.is_none());

        let handler = make_test_handler(Some(vec![0xFF, 0x00]));
        assert!(handler.config.ca_cert_der.is_some());
        assert_eq!(handler.config.ca_cert_der.unwrap().len(), 2);
    }

    #[test]
    fn host_matching_via_cert_distribution() {
        let matching = Request::builder()
            .uri("/ssl")
            .header("host", "cheolsu.proxy:8080")
            .body(Body::from(""))
            .unwrap();
        assert!(cert_distribution::is_cert_download_request(&matching));

        let non_matching = Request::builder()
            .uri("/api")
            .header("host", "other.proxy:8080")
            .body(Body::from(""))
            .unwrap();
        assert!(!cert_distribution::is_cert_download_request(&non_matching));

        let evil = Request::builder()
            .uri("/api")
            .header("host", "cheolsu.proxy.evil.com")
            .body(Body::from(""))
            .unwrap();
        assert!(!cert_distribution::is_cert_download_request(&evil));
    }

    // --- convert_ws_message_payload 테스트 ---

    #[test]
    fn convert_text_message() {
        let msg = Message::Text("hello".into());
        let (msg_type, payload, size, is_binary) =
            LoggingHandler::convert_ws_message_payload(&msg).unwrap();
        assert_eq!(msg_type, WsMessageType::Text);
        assert_eq!(payload, "hello");
        assert_eq!(size, 5);
        assert!(!is_binary);
    }

    #[test]
    fn convert_binary_message() {
        let data = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let msg = Message::Binary(data.clone().into());
        let (msg_type, payload, size, is_binary) =
            LoggingHandler::convert_ws_message_payload(&msg).unwrap();
        assert_eq!(msg_type, WsMessageType::Binary);
        use base64::Engine;
        assert_eq!(
            payload,
            base64::engine::general_purpose::STANDARD.encode(&data)
        );
        assert_eq!(size, 4);
        assert!(is_binary);
    }

    #[test]
    fn convert_ping_message() {
        let msg = Message::Ping(vec![1, 2, 3].into());
        let (msg_type, payload, size, is_binary) =
            LoggingHandler::convert_ws_message_payload(&msg).unwrap();
        assert_eq!(msg_type, WsMessageType::Ping);
        assert_eq!(payload, "3 bytes");
        assert_eq!(size, 3);
        assert!(is_binary);
    }

    #[test]
    fn convert_pong_message() {
        let msg = Message::Pong(vec![].into());
        let (msg_type, payload, size, is_binary) =
            LoggingHandler::convert_ws_message_payload(&msg).unwrap();
        assert_eq!(msg_type, WsMessageType::Pong);
        assert_eq!(payload, "0 bytes");
        assert_eq!(size, 0);
        assert!(is_binary);
    }

    #[test]
    fn convert_close_message_with_frame() {
        use proxyapi_v2::tokio_tungstenite::tungstenite::protocol::CloseFrame;
        let frame = CloseFrame {
            code: proxyapi_v2::tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Normal,
            reason: "bye".into(),
        };
        let msg = Message::Close(Some(frame));
        let (msg_type, payload, _size, is_binary) =
            LoggingHandler::convert_ws_message_payload(&msg).unwrap();
        assert_eq!(msg_type, WsMessageType::Close);
        assert!(payload.contains("bye"));
        assert!(!is_binary);
    }

    #[test]
    fn convert_close_message_without_frame() {
        let msg = Message::Close(None);
        let (msg_type, payload, _size, _is_binary) =
            LoggingHandler::convert_ws_message_payload(&msg).unwrap();
        assert_eq!(msg_type, WsMessageType::Close);
        assert!(payload.is_empty());
    }

    #[test]
    fn convert_frame_message_returns_none() {
        use proxyapi_v2::tokio_tungstenite::tungstenite::protocol::frame::Frame;
        let msg = Message::Frame(Frame::ping(Bytes::new()));
        assert!(LoggingHandler::convert_ws_message_payload(&msg).is_none());
    }

    // --- check_cert_download_intercept 테스트 ---

    #[test]
    fn cert_intercept_host_header_exact() {
        let handler = make_test_handler(Some(vec![1]));
        let req = Request::builder()
            .uri("/anything")
            .header("host", "cheolsu.proxy")
            .body(Body::from(""))
            .unwrap();
        assert!(handler.check_cert_download_intercept(&req).is_some());
    }

    #[test]
    fn cert_intercept_host_header_with_port() {
        let handler = make_test_handler(Some(vec![1]));
        let req = Request::builder()
            .uri("/anything")
            .header("host", "cheolsu.proxy:8100")
            .body(Body::from(""))
            .unwrap();
        assert!(handler.check_cert_download_intercept(&req).is_some());
    }

    #[test]
    fn cert_intercept_absolute_uri() {
        let handler = make_test_handler(Some(vec![1]));
        let req = Request::builder()
            .uri("http://cheolsu.proxy/ssl")
            .body(Body::from(""))
            .unwrap();
        assert!(handler.check_cert_download_intercept(&req).is_some());
    }

    #[test]
    fn cert_intercept_direct_ip_ssl_path() {
        let handler = make_test_handler(Some(vec![1]));
        let req = Request::builder().uri("/ssl").body(Body::from("")).unwrap();
        assert!(handler.check_cert_download_intercept(&req).is_some());
    }

    #[test]
    fn cert_intercept_direct_ip_cert_path() {
        let handler = make_test_handler(Some(vec![1]));
        let req = Request::builder()
            .uri("/cert")
            .body(Body::from(""))
            .unwrap();
        assert!(handler.check_cert_download_intercept(&req).is_some());
    }

    #[test]
    fn cert_intercept_non_matching_host() {
        let handler = make_test_handler(Some(vec![1]));
        let req = Request::builder()
            .uri("/api/data")
            .header("host", "example.com")
            .body(Body::from(""))
            .unwrap();
        assert!(handler.check_cert_download_intercept(&req).is_none());
    }

    #[test]
    fn cert_intercept_non_matching_path_without_host() {
        let handler = make_test_handler(Some(vec![1]));
        let req = Request::builder()
            .uri("/api/data")
            .body(Body::from(""))
            .unwrap();
        assert!(handler.check_cert_download_intercept(&req).is_none());
    }

    // --- emit_ws_event 테스트 ---

    #[test]
    fn emit_ws_event_without_sender_does_nothing() {
        let handler = make_test_handler(None);
        // ws_sender가 None이면 패닉 없이 조용히 반환
        handler.emit_ws_event(
            "conn1".to_string(),
            WsDirection::ClientToServer,
            WsMessageType::Text,
            "hello".to_string(),
            5,
            false,
        );
    }

    #[test]
    fn emit_ws_event_sends_to_channel() {
        let (sender, _rx) = tokio::sync::mpsc::channel(1);
        let (ws_sender, mut ws_rx) = tokio::sync::mpsc::channel(8);
        let handler = LoggingHandler {
            sender,
            request: RequestState {
                req: None,
                res: None,
            },
            config: ProxyConfig {
                cache_dir: None,
                ca_cert_der: None,
            },
            intercept: InterceptEngine {
                intercept_rules: Arc::new(Mutex::new(Vec::new())),
                server_replay_entries: Arc::new(Mutex::new(Vec::new())),
                host_mappings: Arc::new(Mutex::new(Vec::new())),
                script_handle: scripting::ScriptHandle::new(),
            },
            ws: WebSocketState {
                ws_sender: Some(ws_sender),
                ws_sequence: Arc::new(std::sync::atomic::AtomicU64::new(0)),
                mqtt_versions: Arc::new(parking_lot::Mutex::new(std::collections::HashMap::new())),
            },
            breakpoint_manager: None,
        };

        handler.emit_ws_event(
            "wss://example.com".to_string(),
            WsDirection::ClientToServer,
            WsMessageType::Text,
            "test payload".to_string(),
            12,
            false,
        );

        let event = ws_rx.try_recv().unwrap();
        match event {
            WsEvent::Message(info) => {
                assert_eq!(info.connection_id, "wss://example.com");
                assert_eq!(info.sequence, 0);
                assert_eq!(info.direction, WsDirection::ClientToServer);
                assert_eq!(info.message_type, WsMessageType::Text);
                assert_eq!(info.payload, "test payload");
                assert_eq!(info.size, 12);
                assert!(!info.is_binary);
            }
            _ => panic!("Expected WsEvent::Message"),
        }
    }

    #[test]
    fn emit_ws_event_increments_sequence() {
        let (sender, _rx) = tokio::sync::mpsc::channel(1);
        let (ws_sender, mut ws_rx) = tokio::sync::mpsc::channel(8);
        let handler = LoggingHandler {
            sender,
            request: RequestState {
                req: None,
                res: None,
            },
            config: ProxyConfig {
                cache_dir: None,
                ca_cert_der: None,
            },
            intercept: InterceptEngine {
                intercept_rules: Arc::new(Mutex::new(Vec::new())),
                server_replay_entries: Arc::new(Mutex::new(Vec::new())),
                host_mappings: Arc::new(Mutex::new(Vec::new())),
                script_handle: scripting::ScriptHandle::new(),
            },
            ws: WebSocketState {
                ws_sender: Some(ws_sender),
                ws_sequence: Arc::new(std::sync::atomic::AtomicU64::new(0)),
                mqtt_versions: Arc::new(parking_lot::Mutex::new(std::collections::HashMap::new())),
            },
            breakpoint_manager: None,
        };

        for i in 0..3 {
            handler.emit_ws_event(
                "conn".to_string(),
                WsDirection::ClientToServer,
                WsMessageType::Text,
                "msg".to_string(),
                3,
                false,
            );
            match ws_rx.try_recv().unwrap() {
                WsEvent::Message(info) => assert_eq!(info.sequence, i),
                _ => panic!("Expected WsEvent::Message"),
            }
        }
    }
}

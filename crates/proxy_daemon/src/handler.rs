use crate::breakpoint::BreakpointManager;
use crate::protocol::{HostMapping, InterceptRule, ServerReplayEntry};
use bytes::Bytes;
use proxy_v2_models::{ProxiedRequest, ProxiedResponse, RequestInfo};
use proxyapi_v2::{
    hyper::http::{Method, StatusCode},
    hyper::{Request, Response},
    Body, HttpContext, HttpHandler, RequestOrResponse,
};
use regex::Regex;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

// Re-export for lib.rs and daemon.rs
pub use crate::tls_client::create_hybrid_client;
pub use crate::tls_client::{create_hybrid_client_with_cert, validate_client_cert_config};

// Re-export types from sub-modules
pub use crate::sse_handler::SseEvent;
pub(crate) use crate::sse_handler::SseState;
pub(crate) use crate::ws_handler::WebSocketState;
pub use crate::ws_handler::WsEvent;

use crate::cert_distribution;

/// 요청별 임시 상태 (매 요청마다 초기화)
#[derive(Clone)]
pub(crate) struct RequestState {
    pub(crate) req: Option<ProxiedRequest>,
    pub(crate) res: Option<ProxiedResponse>,
}

/// 빠른 설정 (No Caching, Block Cookies, No Gzip)
#[derive(Clone, Copy, Debug, Default)]
pub struct QuickSettings {
    pub no_caching: bool,
    pub block_cookies: bool,
    pub no_gzip: bool,
}

/// 프록시 전역 설정 (세션 수명 동안 유지)
#[derive(Clone)]
pub(crate) struct ProxyConfig {
    pub(crate) cache_dir: Option<std::path::PathBuf>,
    /// CA 인증서 DER 바이트 (외부 기기 인증서 다운로드용, zero-copy)
    pub(crate) ca_cert_der: Option<Bytes>,
    /// 빠른 설정 (No Caching, Block Cookies)
    pub(crate) quick_settings: Arc<tokio::sync::RwLock<QuickSettings>>,
    // SAFETY: parking_lot::RwLock - async 컨텍스트에서 사용 중이나,
    // .await를 넘어서 lock을 유지하지 않으므로 안전함.
    // 리팩토링 시 tokio::sync::RwLock으로 교체 검토 필요.
    /// 프록시 인증 설정
    pub(crate) proxy_auth: Arc<parking_lot::RwLock<Option<crate::protocol::ProxyAuthConfig>>>,
    /// 요청 바디 최대 크기 (None이면 제한 없음)
    pub(crate) max_body_size: Option<usize>,
}

/// 인터셉트 규칙 및 스크립트 엔진
#[derive(Clone)]
pub(crate) struct InterceptEngine {
    pub(crate) intercept_rules: Arc<RwLock<Vec<InterceptRule>>>,
    pub(crate) server_replay_entries: Arc<RwLock<Vec<ServerReplayEntry>>>,
    pub(crate) host_mappings: Arc<RwLock<Vec<HostMapping>>>,
    pub(crate) script_handle: scripting::ScriptHandle,
    /// SSL Proxying 모드
    pub(crate) ssl_proxying_mode: Arc<RwLock<crate::protocol::SslProxyingMode>>,
    /// SSL Proxying 엔트리 목록
    pub(crate) ssl_proxying_entries: Arc<RwLock<Vec<crate::protocol::SslProxyingEntry>>>,
}

/// HTTP 및 WebSocket 요청/응답을 로깅하는 핸들러
#[derive(Clone)]
pub struct LoggingHandler {
    pub(crate) sender: tokio::sync::mpsc::Sender<RequestInfo>,
    pub(crate) request: RequestState,
    pub(crate) config: ProxyConfig,
    pub(crate) intercept: InterceptEngine,
    pub(crate) ws: WebSocketState,
    pub(crate) sse: SseState,
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
                quick_settings: Arc::new(tokio::sync::RwLock::new(QuickSettings::default())),
                proxy_auth: Arc::new(parking_lot::RwLock::new(None)),
                max_body_size: None,
            },
            intercept: InterceptEngine {
                intercept_rules: Arc::new(RwLock::new(Vec::new())),
                server_replay_entries: Arc::new(RwLock::new(Vec::new())),
                host_mappings: Arc::new(RwLock::new(Vec::new())),
                script_handle: scripting::ScriptHandle::new(),
                ssl_proxying_mode: Arc::new(RwLock::new(
                    crate::protocol::SslProxyingMode::default(),
                )),
                ssl_proxying_entries: Arc::new(RwLock::new(Vec::new())),
            },
            ws: WebSocketState {
                ws_sender: None,
                ws_sequence: Arc::new(std::sync::atomic::AtomicU64::new(0)),
                mqtt_versions: Arc::new(parking_lot::Mutex::new(std::collections::HashMap::new())),
            },
            sse: SseState {
                sse_sender: None,
                sse_sequence: Arc::new(std::sync::atomic::AtomicU64::new(0)),
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

    pub fn with_sse_sender(mut self, sse_sender: tokio::sync::mpsc::Sender<SseEvent>) -> Self {
        self.sse.sse_sender = Some(sse_sender);
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

    pub fn with_quick_settings(
        mut self,
        quick_settings: Arc<tokio::sync::RwLock<QuickSettings>>,
    ) -> Self {
        self.config.quick_settings = quick_settings;
        self
    }

    pub fn breakpoint_manager(&self) -> Option<&BreakpointManager> {
        self.breakpoint_manager.as_ref()
    }

    /// 인터셉트 규칙 업데이트
    pub async fn update_intercept_rules(&self, rules: Vec<InterceptRule>) {
        let mut rules_guard = self.intercept.intercept_rules.write().await;
        info!("[Intercept] 규칙 업데이트: {} 개", rules.len());
        *rules_guard = rules;
    }

    /// 서버 리플레이 엔트리 업데이트
    pub async fn update_server_replay_entries(&self, entries: Vec<ServerReplayEntry>) {
        let mut entries_guard = self.intercept.server_replay_entries.write().await;
        info!("[ServerReplay] 엔트리 업데이트: {} 개", entries.len());
        *entries_guard = entries;
    }

    /// Update host mappings
    pub async fn update_host_mappings(&self, mappings: Vec<HostMapping>) {
        let mut mappings_guard = self.intercept.host_mappings.write().await;
        info!("[HostMapping] mappings updated: {} entries", mappings.len());
        *mappings_guard = mappings;
    }

    /// SSL Proxying 모드 및 목록 업데이트
    pub async fn update_ssl_proxying(
        &self,
        mode: crate::protocol::SslProxyingMode,
        entries: Vec<crate::protocol::SslProxyingEntry>,
    ) {
        let mut mode_guard = self.intercept.ssl_proxying_mode.write().await;
        *mode_guard = mode.clone();
        drop(mode_guard);
        let mut entries_guard = self.intercept.ssl_proxying_entries.write().await;
        info!(
            "[SSLProxying] 업데이트: mode={:?}, {} 개",
            mode,
            entries.len()
        );
        *entries_guard = entries;
    }

    /// 프록시 인증 설정 업데이트
    pub fn update_proxy_auth(&self, config: crate::protocol::ProxyAuthConfig) {
        // SAFETY: parking_lot write lock - .await 없이 즉시 해제되므로 안전함.
        let mut auth = self.config.proxy_auth.write();
        info!(
            "[ProxyAuth] 설정 업데이트: enabled={}, username={}",
            config.enabled, config.username
        );
        *auth = Some(config);
    }

    pub fn with_proxy_auth(
        mut self,
        proxy_auth: Arc<parking_lot::RwLock<Option<crate::protocol::ProxyAuthConfig>>>,
    ) -> Self {
        self.config.proxy_auth = proxy_auth;
        self
    }

    /// 요청 바디 최대 크기를 설정합니다 (None이면 제한 없음)
    pub fn with_max_body_size(mut self, max_body_size: Option<usize>) -> Self {
        self.config.max_body_size = max_body_size;
        self
    }

    /// 프록시 인증을 확인합니다. 인증 실패 시 407 응답을 반환합니다.
    fn check_proxy_auth(&self, req: &Request<Body>) -> Option<Response<Body>> {
        // SAFETY: parking_lot read lock - .await 없이 즉시 해제되므로 안전함.
        let auth_config = self.config.proxy_auth.read();
        let config = match auth_config.as_ref() {
            Some(c) if c.enabled && !c.username.is_empty() => c,
            _ => return None,
        };

        let auth_header = req
            .headers()
            .get("proxy-authorization")
            .and_then(|v| v.to_str().ok());

        if config.validate_proxy_auth(auth_header) {
            None
        } else {
            info!(
                "[ProxyAuth] 인증 실패: {:?}",
                req.uri().authority().map(|a| a.to_string())
            );
            Some(
                Response::builder()
                    .status(StatusCode::PROXY_AUTHENTICATION_REQUIRED)
                    .header("Proxy-Authenticate", "Basic realm=\"Cheolsu Proxy\"")
                    .body(Body::from("Proxy Authentication Required"))
                    .unwrap_or_else(|_| Response::new(Body::empty())),
            )
        }
    }

    /// 스크립트 핸들 반환
    pub fn script_handle(&self) -> &scripting::ScriptHandle {
        &self.intercept.script_handle
    }

    fn serve_ca_cert_download(&self, req: &Request<Body>) -> Response<Body> {
        cert_distribution::handle_cert_request(req, self.config.ca_cert_der.as_ref())
    }

    /// No Caching / Block Cookies / No Gzip 설정을 요청에 적용
    async fn apply_quick_settings_on_request(&self, mut req: Request<Body>) -> Request<Body> {
        use proxyapi_v2::hyper::header::{
            ACCEPT_ENCODING, CACHE_CONTROL, COOKIE, IF_MODIFIED_SINCE, IF_NONE_MATCH, PRAGMA,
        };

        let settings = { *self.config.quick_settings.read().await };

        if settings.no_caching {
            req.headers_mut().remove(IF_MODIFIED_SINCE);
            req.headers_mut().remove(IF_NONE_MATCH);
            req.headers_mut().insert(
                CACHE_CONTROL,
                "no-cache, no-store, must-revalidate".parse().unwrap(),
            );
            req.headers_mut()
                .insert(PRAGMA, "no-cache".parse().unwrap());
        }

        if settings.block_cookies {
            req.headers_mut().remove(COOKIE);
        }

        if settings.no_gzip {
            req.headers_mut().remove(ACCEPT_ENCODING);
        }

        req
    }

    /// Block Cookies 설정을 응답에 적용 (Set-Cookie 제거)
    async fn apply_quick_settings_on_response(&self, mut res: Response<Body>) -> Response<Body> {
        use proxyapi_v2::hyper::header::SET_COOKIE;

        let settings = { *self.config.quick_settings.read().await };

        if settings.block_cookies {
            res.headers_mut().remove(SET_COOKIE);
        }

        res
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

    /// Apply host mapping to the request if a matching rule exists.
    /// Rewrites the URI to point to the mapped target host/port,
    /// while preserving the original Host header for correct virtual host routing.
    async fn apply_host_mapping_if_needed(&self, mut req: Request<Body>) -> Request<Body> {
        let (host, port) = Self::extract_host_port(req.uri());
        let Some(host) = host else {
            return req;
        };

        if let Some((target_host, target_port)) = self.resolve_host_mapping(&host, port).await {
            info!(
                "[HostMapping] {}:{} -> {}:{}",
                host,
                port.map(|p| p.to_string())
                    .unwrap_or_else(|| "default".to_string()),
                target_host,
                target_port
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "default".to_string()),
            );

            if let Some(new_uri) =
                Self::apply_host_mapping_to_uri(req.uri(), &target_host, target_port)
            {
                *req.uri_mut() = new_uri;
                // Keep the original Host header intact so the server
                // can route to the correct virtual host.
                //
                // x-cheolsu-host-mapped: 요청 디버깅/로깅 전용 마커 헤더.
                // 호스트 매핑이 적용되었음을 프록시 내부에서 추적하기 위한 용도이며,
                // 실제 서버로 전송됩니다. 서버 측에서 이 헤더가 문제가 될 경우
                // 향후 요청 전송 직전에 제거하는 옵션을 추가할 수 있습니다.
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
}

impl HttpHandler for LoggingHandler {
    async fn should_intercept(&mut self, _ctx: &HttpContext, req: &Request<Body>) -> bool {
        // 프록시 인증 체크: 인증 실패 여부를 먼저 판정
        let auth_failed = {
            // SAFETY: parking_lot read lock - .await 없이 블록 내에서 즉시 해제되므로 안전함.
            let auth_config = self.config.proxy_auth.read();
            if let Some(config) = auth_config.as_ref() {
                if config.enabled && !config.username.is_empty() {
                    let auth_header = req
                        .headers()
                        .get("proxy-authorization")
                        .and_then(|v| v.to_str().ok());
                    !config.validate_proxy_auth(auth_header)
                } else {
                    false
                }
            } else {
                false
            }
        };

        // 인증 실패 시 반드시 인터셉트하여 handle_request에서 407 응답 반환
        // TLS Passthrough 경로로 빠지면 인증 없이 터널이 수립되므로 여기서 차단 필수
        if auth_failed {
            info!(
                "[ProxyAuth] CONNECT 인증 실패, 터널 수립 거부: {:?}",
                req.uri().authority().map(|a| a.to_string())
            );
            return true;
        }

        // CONNECT 요청의 URI에서 authority(host:port)를 추출
        if let Some(authority) = req.uri().authority() {
            let host = authority.host();
            let port = authority.port_u16();

            let mode = self.intercept.ssl_proxying_mode.read().await;
            let entries = self.intercept.ssl_proxying_entries.read().await;
            let result = crate::ssl_proxying::should_intercept_ssl(&mode, &entries, host, port);

            if !result {
                debug!("[SSLProxying] TLS Passthrough 적용: {}", authority);
            }

            result
        } else {
            true
        }
    }

    async fn handle_request(
        &mut self,
        _ctx: &HttpContext,
        mut req: Request<Body>,
    ) -> RequestOrResponse {
        // 프록시 인증 확인
        if let Some(auth_response) = self.check_proxy_auth(&req) {
            return auth_response.into();
        }
        // 인증 통과 후 Proxy-Authorization 헤더 제거 (upstream에 전달 방지)
        req.headers_mut().remove("proxy-authorization");

        // 요청 바디 크기 제한 확인 (Content-Length 기반)
        // NOTE: Content-Length 헤더 기반 검사만 수행하므로, chunked transfer-encoding을 사용하는
        // 요청은 Content-Length가 없어 이 검사를 우회할 수 있습니다.
        // 완전한 제한이 필요하면 바디 스트림을 소비하며 누적 크기를 체크하는 방식이 필요합니다.
        if let Some(max_size) = self.config.max_body_size {
            if let Some(content_length) = req
                .headers()
                .get(proxyapi_v2::hyper::header::CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<usize>().ok())
            {
                if content_length > max_size {
                    info!(
                        "[BodyLimit] 요청 바디 크기 초과: {} > {} ({})",
                        content_length,
                        max_size,
                        req.uri()
                    );
                    let response = Response::builder()
                        .status(StatusCode::PAYLOAD_TOO_LARGE)
                        .body(Body::from(format!(
                            "Request body too large: {} bytes (max: {} bytes)",
                            content_length, max_size
                        )))
                        .unwrap_or_else(|_| Response::new(Body::empty()));
                    return response.into();
                }
            }
        }

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
            // CONNECT 터널 요청을 UI에서 볼 수 있도록 로깅
            self.request.req = Some(proxied_request.clone());
            self.send_output().await;
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

        let restored_req = self.apply_quick_settings_on_request(restored_req).await;

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
        let res = self.apply_quick_settings_on_response(res).await;
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
        // /ssl without User-Agent returns PEM (.crt) format, not raw DER
        // 4 DER bytes → PEM base64 wrapping = 63 bytes
        assert_eq!(resp.headers().get("Content-Length").unwrap(), "63");
        assert!(resp
            .headers()
            .get("Content-Disposition")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("cheolsu-proxy-ca.crt"));
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

    // --- Quick Settings (No Caching / Block Cookies) 테스트 ---

    /// quick_settings를 지정하여 테스트용 핸들러를 생성하는 헬퍼
    fn make_handler_with_quick_settings(settings: QuickSettings) -> LoggingHandler {
        let qs = Arc::new(tokio::sync::RwLock::new(settings));
        let handler = make_test_handler(None).with_quick_settings(qs);
        handler
    }

    #[tokio::test]
    async fn no_caching_adds_cache_control_headers() {
        let handler = make_handler_with_quick_settings(QuickSettings {
            no_caching: true,
            block_cookies: false,
            no_gzip: false,
        });

        let req = Request::builder()
            .uri("http://example.com/")
            .body(Body::from(""))
            .unwrap();

        let req = handler.apply_quick_settings_on_request(req).await;

        assert_eq!(
            req.headers().get("cache-control").unwrap(),
            "no-cache, no-store, must-revalidate"
        );
        assert_eq!(req.headers().get("pragma").unwrap(), "no-cache");
    }

    #[tokio::test]
    async fn no_caching_removes_conditional_headers() {
        let handler = make_handler_with_quick_settings(QuickSettings {
            no_caching: true,
            block_cookies: false,
            no_gzip: false,
        });

        let req = Request::builder()
            .uri("http://example.com/")
            .header("If-Modified-Since", "Thu, 01 Jan 2026 00:00:00 GMT")
            .header("If-None-Match", "\"etag123\"")
            .body(Body::from(""))
            .unwrap();

        let req = handler.apply_quick_settings_on_request(req).await;

        assert!(req.headers().get("if-modified-since").is_none());
        assert!(req.headers().get("if-none-match").is_none());
    }

    #[tokio::test]
    async fn block_cookies_removes_cookie_from_request() {
        let handler = make_handler_with_quick_settings(QuickSettings {
            no_caching: false,
            block_cookies: true,
            no_gzip: false,
        });

        let req = Request::builder()
            .uri("http://example.com/")
            .header("Cookie", "session=abc123; user=test")
            .body(Body::from(""))
            .unwrap();

        let req = handler.apply_quick_settings_on_request(req).await;

        assert!(req.headers().get("cookie").is_none());
    }

    #[tokio::test]
    async fn block_cookies_removes_set_cookie_from_response() {
        let handler = make_handler_with_quick_settings(QuickSettings {
            no_caching: false,
            block_cookies: true,
            no_gzip: false,
        });

        let res = Response::builder()
            .status(200)
            .header("Set-Cookie", "session=abc123; Path=/")
            .header("Content-Type", "text/html")
            .body(Body::from(""))
            .unwrap();

        let res = handler.apply_quick_settings_on_response(res).await;

        assert!(res.headers().get("set-cookie").is_none());
        // 다른 헤더는 영향받지 않아야 함
        assert!(res.headers().get("content-type").is_some());
    }

    #[tokio::test]
    async fn disabled_quick_settings_preserves_all_headers() {
        let handler = make_handler_with_quick_settings(QuickSettings {
            no_caching: false,
            block_cookies: false,
            no_gzip: false,
        });

        let req = Request::builder()
            .uri("http://example.com/")
            .header("If-Modified-Since", "Thu, 01 Jan 2026 00:00:00 GMT")
            .header("If-None-Match", "\"etag123\"")
            .header("Cookie", "session=abc123")
            .body(Body::from(""))
            .unwrap();

        let req = handler.apply_quick_settings_on_request(req).await;

        assert!(req.headers().get("if-modified-since").is_some());
        assert!(req.headers().get("if-none-match").is_some());
        assert!(req.headers().get("cookie").is_some());
        assert!(req.headers().get("cache-control").is_none());

        let res = Response::builder()
            .status(200)
            .header("Set-Cookie", "session=abc123; Path=/")
            .body(Body::from(""))
            .unwrap();

        let res = handler.apply_quick_settings_on_response(res).await;

        assert!(res.headers().get("set-cookie").is_some());
    }

    #[tokio::test]
    async fn both_settings_enabled_applies_all_modifications() {
        let handler = make_handler_with_quick_settings(QuickSettings {
            no_caching: true,
            block_cookies: true,
            no_gzip: false,
        });

        let req = Request::builder()
            .uri("http://example.com/")
            .header("If-Modified-Since", "Thu, 01 Jan 2026 00:00:00 GMT")
            .header("If-None-Match", "\"etag123\"")
            .header("Cookie", "session=abc123")
            .body(Body::from(""))
            .unwrap();

        let req = handler.apply_quick_settings_on_request(req).await;

        // No Caching 적용 확인
        assert!(req.headers().get("if-modified-since").is_none());
        assert!(req.headers().get("if-none-match").is_none());
        assert_eq!(
            req.headers().get("cache-control").unwrap(),
            "no-cache, no-store, must-revalidate"
        );
        assert_eq!(req.headers().get("pragma").unwrap(), "no-cache");
        // Block Cookies 적용 확인
        assert!(req.headers().get("cookie").is_none());

        let res = Response::builder()
            .status(200)
            .header("Set-Cookie", "session=abc123; Path=/")
            .body(Body::from(""))
            .unwrap();

        let res = handler.apply_quick_settings_on_response(res).await;

        assert!(res.headers().get("set-cookie").is_none());
    }

    /// 동시 읽기/쓰기 시 데드락이 발생하지 않는지 검증
    #[tokio::test]
    async fn concurrent_quick_settings_read_write_no_deadlock() {
        let qs = Arc::new(tokio::sync::RwLock::new(QuickSettings {
            no_caching: false,
            block_cookies: false,
            no_gzip: false,
        }));

        let mut handles = Vec::new();

        // 여러 읽기 태스크 동시 실행 (요청 처리 시뮬레이션)
        for _ in 0..10 {
            let qs_clone = qs.clone();
            handles.push(tokio::spawn(async move {
                for _ in 0..100 {
                    let settings = { *qs_clone.read().await };
                    // 읽은 값이 유효한지 확인 (bool이므로 항상 유효)
                    let _ = settings.no_caching;
                    let _ = settings.block_cookies;
                    tokio::task::yield_now().await;
                }
            }));
        }

        // 동시에 쓰기 태스크 실행 (설정 변경 시뮬레이션)
        for i in 0..5 {
            let qs_clone = qs.clone();
            handles.push(tokio::spawn(async move {
                for _ in 0..100 {
                    let mut settings = qs_clone.write().await;
                    settings.no_caching = i % 2 == 0;
                    settings.block_cookies = i % 2 == 1;
                    drop(settings);
                    tokio::task::yield_now().await;
                }
            }));
        }

        // 3초 타임아웃 - 데드락 시 실패
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            futures_util::future::join_all(handles),
        )
        .await;

        assert!(result.is_ok(), "데드락 감지: 3초 타임아웃 초과");
        for r in result.unwrap() {
            r.unwrap();
        }
    }

    /// apply_quick_settings 메서드의 동시 호출이 데드락 없이 완료되는지 검증
    #[tokio::test]
    async fn concurrent_apply_quick_settings_no_deadlock() {
        let handler = make_handler_with_quick_settings(QuickSettings {
            no_caching: true,
            block_cookies: true,
            no_gzip: false,
        });

        let mut handles = Vec::new();

        for _ in 0..10 {
            let h = handler.clone();
            handles.push(tokio::spawn(async move {
                for _ in 0..50 {
                    let req = Request::builder()
                        .uri("http://example.com/")
                        .header("Cookie", "session=abc")
                        .header("If-None-Match", "\"etag\"")
                        .body(Body::from(""))
                        .unwrap();
                    let req = h.apply_quick_settings_on_request(req).await;
                    assert!(req.headers().get("cookie").is_none());

                    let res = Response::builder()
                        .status(200)
                        .header("Set-Cookie", "session=abc; Path=/")
                        .body(Body::from(""))
                        .unwrap();
                    let res = h.apply_quick_settings_on_response(res).await;
                    assert!(res.headers().get("set-cookie").is_none());

                    tokio::task::yield_now().await;
                }
            }));
        }

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            futures_util::future::join_all(handles),
        )
        .await;

        assert!(result.is_ok(), "데드락 감지: 3초 타임아웃 초과");
        for r in result.unwrap() {
            r.unwrap();
        }
    }

    #[tokio::test]
    async fn no_gzip_removes_accept_encoding_header() {
        let handler = make_handler_with_quick_settings(QuickSettings {
            no_caching: false,
            block_cookies: false,
            no_gzip: true,
        });

        let req = Request::builder()
            .uri("http://example.com/")
            .header("Accept-Encoding", "gzip, deflate, br")
            .body(Body::from(""))
            .unwrap();

        let req = handler.apply_quick_settings_on_request(req).await;

        assert!(req.headers().get("accept-encoding").is_none());
    }

    #[tokio::test]
    async fn no_gzip_disabled_preserves_accept_encoding() {
        let handler = make_handler_with_quick_settings(QuickSettings {
            no_caching: false,
            block_cookies: false,
            no_gzip: false,
        });

        let req = Request::builder()
            .uri("http://example.com/")
            .header("Accept-Encoding", "gzip, deflate, br")
            .body(Body::from(""))
            .unwrap();

        let req = handler.apply_quick_settings_on_request(req).await;

        assert!(req.headers().get("accept-encoding").is_some());
    }

    #[tokio::test]
    async fn all_quick_settings_enabled_applies_all() {
        let handler = make_handler_with_quick_settings(QuickSettings {
            no_caching: true,
            block_cookies: true,
            no_gzip: true,
        });

        let req = Request::builder()
            .uri("http://example.com/")
            .header("If-Modified-Since", "Thu, 01 Jan 2026 00:00:00 GMT")
            .header("Cookie", "session=abc123")
            .header("Accept-Encoding", "gzip, deflate, br")
            .body(Body::from(""))
            .unwrap();

        let req = handler.apply_quick_settings_on_request(req).await;

        // No Caching
        assert!(req.headers().get("if-modified-since").is_none());
        assert_eq!(
            req.headers().get("cache-control").unwrap(),
            "no-cache, no-store, must-revalidate"
        );
        // Block Cookies
        assert!(req.headers().get("cookie").is_none());
        // No Gzip
        assert!(req.headers().get("accept-encoding").is_none());
    }
}

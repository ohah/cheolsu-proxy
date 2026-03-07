use crate::protocol::{InterceptAction, InterceptRule, ServerReplayEntry};
use bytes::Bytes;
use futures_util::stream::StreamExt;
use http_body_util::{BodyExt, StreamBody};
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::{client::legacy::Client, rt::TokioExecutor};
use proxy_v2_models::{
    ProxiedRequest, ProxiedResponse, RequestInfo, WsConnectionEvent, WsDirection, WsMessageInfo,
    WsMessageType,
};
use proxyapi_v2::{
    hyper::http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
    hyper::{Request, Response},
    tokio_tungstenite::tungstenite::Message,
    upstream_proxy::{ProxyHttpConnector, UpstreamProxyConfig},
    Body, HttpContext, HttpHandler, RequestOrResponse, WebSocketContext, WebSocketHandler,
};
use regex::Regex;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_rustls::rustls::{crypto::aws_lc_rs, ClientConfig};
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, error, info};

/// 모든 인증서를 허용하는 위험한 인증서 검증기
#[derive(Debug)]
struct DangerousCertificateVerifier;

impl tokio_rustls::rustls::client::danger::ServerCertVerifier for DangerousCertificateVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &tokio_rustls::rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[tokio_rustls::rustls::pki_types::CertificateDer<'_>],
        _server_name: &tokio_rustls::rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: tokio_rustls::rustls::pki_types::UnixTime,
    ) -> Result<tokio_rustls::rustls::client::danger::ServerCertVerified, tokio_rustls::rustls::Error>
    {
        Ok(tokio_rustls::rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &tokio_rustls::rustls::pki_types::CertificateDer<'_>,
        _dss: &tokio_rustls::rustls::DigitallySignedStruct,
    ) -> Result<
        tokio_rustls::rustls::client::danger::HandshakeSignatureValid,
        tokio_rustls::rustls::Error,
    > {
        Ok(tokio_rustls::rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &tokio_rustls::rustls::pki_types::CertificateDer<'_>,
        _dss: &tokio_rustls::rustls::DigitallySignedStruct,
    ) -> Result<
        tokio_rustls::rustls::client::danger::HandshakeSignatureValid,
        tokio_rustls::rustls::Error,
    > {
        Ok(tokio_rustls::rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<tokio_rustls::rustls::SignatureScheme> {
        vec![
            tokio_rustls::rustls::SignatureScheme::RSA_PKCS1_SHA1,
            tokio_rustls::rustls::SignatureScheme::ECDSA_SHA1_Legacy,
            tokio_rustls::rustls::SignatureScheme::RSA_PKCS1_SHA256,
            tokio_rustls::rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            tokio_rustls::rustls::SignatureScheme::RSA_PKCS1_SHA384,
            tokio_rustls::rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            tokio_rustls::rustls::SignatureScheme::RSA_PKCS1_SHA512,
            tokio_rustls::rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            tokio_rustls::rustls::SignatureScheme::RSA_PSS_SHA256,
            tokio_rustls::rustls::SignatureScheme::RSA_PSS_SHA384,
            tokio_rustls::rustls::SignatureScheme::RSA_PSS_SHA512,
            tokio_rustls::rustls::SignatureScheme::ED25519,
            tokio_rustls::rustls::SignatureScheme::ED448,
            tokio_rustls::rustls::SignatureScheme::ML_DSA_44,
            tokio_rustls::rustls::SignatureScheme::ML_DSA_65,
            tokio_rustls::rustls::SignatureScheme::ML_DSA_87,
        ]
    }
}

/// 하이브리드 클라이언트 생성 (모든 인증서 허용, upstream proxy 지원)
///
/// `upstream_rx`를 통해 런타임에 upstream proxy 설정 변경이 즉시 반영됩니다.
pub fn create_hybrid_client(
    upstream_rx: tokio::sync::watch::Receiver<Option<UpstreamProxyConfig>>,
) -> Result<
    Client<hyper_rustls::HttpsConnector<ProxyHttpConnector>, Body>,
    Box<dyn std::error::Error>,
> {
    let rustls_config =
        ClientConfig::builder_with_provider(std::sync::Arc::new(aws_lc_rs::default_provider()))
            .with_safe_default_protocol_versions()?
            .dangerous()
            .with_custom_certificate_verifier(std::sync::Arc::new(DangerousCertificateVerifier))
            .with_no_client_auth();

    let proxy_connector = ProxyHttpConnector::new(upstream_rx);

    let https = HttpsConnectorBuilder::new()
        .with_tls_config(rustls_config)
        .https_or_http()
        .enable_http1()
        .enable_http2()
        .wrap_connector(proxy_connector);

    Ok(Client::builder(TokioExecutor::new())
        .http1_title_case_headers(true)
        .http1_preserve_header_case(true)
        .build(https))
}

/// WebSocket 이벤트 (메시지 또는 연결 상태)
#[derive(Clone, Debug)]
pub enum WsEvent {
    Message(WsMessageInfo),
    Connection(WsConnectionEvent),
}

/// HTTP 및 WebSocket 요청/응답을 로깅하는 핸들러
#[derive(Clone)]
pub struct LoggingHandler {
    sender: tokio::sync::mpsc::Sender<RequestInfo>,
    ws_sender: Option<tokio::sync::mpsc::Sender<WsEvent>>,
    ws_sequence: Arc<std::sync::atomic::AtomicU64>,
    mqtt_versions: Arc<std::sync::Mutex<std::collections::HashMap<String, u8>>>,
    req: Option<ProxiedRequest>,
    res: Option<ProxiedResponse>,
    intercept_rules: Arc<Mutex<Vec<InterceptRule>>>,
    server_replay_entries: Arc<Mutex<Vec<ServerReplayEntry>>>,
    cache_dir: Option<std::path::PathBuf>,
    script_handle: scripting::ScriptHandle,
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
        }
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

    /// ProxiedRequest → ScriptRequest 변환
    fn to_script_request(req: &ProxiedRequest) -> scripting::ScriptRequest {
        let mut headers = std::collections::HashMap::new();
        for (name, value) in req.headers() {
            if let Ok(v) = value.to_str() {
                headers.insert(name.to_string(), v.to_string());
            }
        }
        scripting::ScriptRequest {
            method: req.method().to_string(),
            url: req.uri().to_string(),
            headers,
            body: std::str::from_utf8(req.body()).ok().map(|s| s.to_string()),
        }
    }

    /// hyper Response → ScriptResponse 변환
    fn to_script_response_from_hyper(res: &Response<Body>) -> scripting::ScriptResponse {
        let mut headers = std::collections::HashMap::new();
        for (name, value) in res.headers() {
            if let Ok(v) = value.to_str() {
                headers.insert(name.to_string(), v.to_string());
            }
        }
        scripting::ScriptResponse {
            status: res.status().as_u16(),
            headers,
            body: None, // 응답 body는 스트리밍이라 읽을 수 없음
        }
    }

    /// ScriptRequest의 수정사항을 hyper Request에 적용
    fn apply_script_request_modify(
        mut req: Request<Body>,
        modified: &scripting::ScriptRequest,
    ) -> Request<Body> {
        // 헤더 수정
        req.headers_mut().clear();
        for (name, value) in &modified.headers {
            if let (Ok(hn), Ok(hv)) = (
                HeaderName::from_bytes(name.as_bytes()),
                HeaderValue::from_str(value),
            ) {
                req.headers_mut().insert(hn, hv);
            }
        }
        // URL 수정
        if let Ok(uri) = modified.url.parse() {
            *req.uri_mut() = uri;
        }
        // Body 수정
        if let Some(body) = &modified.body {
            req = req.map(|_| Body::from(body.clone()));
        }
        req
    }

    /// ScriptResponse에서 hyper Response 생성
    fn build_script_response(script_res: &scripting::ScriptResponse) -> Response<Body> {
        let mut builder = Response::builder()
            .status(StatusCode::from_u16(script_res.status).unwrap_or(StatusCode::OK))
            .header("x-cheolsu-scripted", "true");

        for (name, value) in &script_res.headers {
            builder = builder.header(name.as_str(), value.as_str());
        }

        let body = script_res.body.clone().unwrap_or_default();
        builder.body(Body::from(body)).unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Script response error"))
                .unwrap()
        })
    }

    /// ScriptResponse의 수정사항을 hyper Response에 적용
    fn apply_script_response_modify(
        res: Response<Body>,
        modified: &scripting::ScriptResponse,
    ) -> Response<Body> {
        let (mut parts, body) = res.into_parts();

        // 상태 코드 수정
        if let Ok(status) = StatusCode::from_u16(modified.status) {
            parts.status = status;
        }

        // 헤더 수정
        for (name, value) in &modified.headers {
            if let (Ok(hn), Ok(hv)) = (
                HeaderName::from_bytes(name.as_bytes()),
                HeaderValue::from_str(value),
            ) {
                parts.headers.insert(hn, hv);
            }
        }

        if let Some(new_body) = &modified.body {
            Response::from_parts(parts, Body::from(new_body.clone()))
        } else {
            Response::from_parts(parts, body)
        }
    }

    /// 서버 리플레이 매칭: method + URL이 일치하는 엔트리 검색
    async fn find_server_replay_match(&self, url: &str, method: &str) -> Option<ServerReplayEntry> {
        let entries = self.server_replay_entries.lock().await;
        entries
            .iter()
            .find(|entry| entry.method.eq_ignore_ascii_case(method) && entry.url == url)
            .cloned()
    }

    /// 와일드카드 패턴 매칭 (* = 임의 문자열, ? = 단일 문자)
    fn wildcard_matches(pattern: &str, text: &str) -> bool {
        let regex_pattern = format!(
            "(?i){}",
            regex::escape(pattern)
                .replace("\\*", ".*")
                .replace("\\?", ".")
        );
        Regex::new(&regex_pattern)
            .map(|re| re.is_match(text))
            .unwrap_or(false)
    }

    /// URL과 메서드가 인터셉트 규칙에 매칭되는지 확인
    fn rule_matches(rule: &InterceptRule, url: &str, method: &str) -> bool {
        if !rule.enabled {
            return false;
        }
        if let Some(rule_method) = &rule.method {
            if rule_method.to_uppercase() != method.to_uppercase() {
                return false;
            }
        }
        Self::wildcard_matches(&rule.pattern, url)
    }

    /// 파일 확장자로 Content-Type 추론
    fn guess_content_type(file_path: &str) -> String {
        let ext = std::path::Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        match ext.as_str() {
            "json" => "application/json",
            "html" | "htm" => "text/html; charset=utf-8",
            "xml" => "application/xml",
            "js" | "mjs" => "application/javascript",
            "css" => "text/css",
            "txt" => "text/plain; charset=utf-8",
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "svg" => "image/svg+xml",
            "webp" => "image/webp",
            "woff" | "woff2" => "font/woff2",
            "pdf" => "application/pdf",
            _ => "application/octet-stream",
        }
        .to_string()
    }

    /// 매칭되는 인터셉트 규칙 검색
    async fn find_matching_intercept_rules(&self, url: &str, method: &str) -> Vec<InterceptRule> {
        let rules_guard = self.intercept_rules.lock().await;
        rules_guard
            .iter()
            .filter(|rule| Self::rule_matches(rule, url, method))
            .cloned()
            .collect()
    }

    /// 인터셉트 규칙에 따라 요청을 차단하거나 수정
    async fn apply_request_intercept(
        &self,
        req: Request<Body>,
        url: &str,
        method: &str,
    ) -> RequestOrResponse {
        let rules = self.find_matching_intercept_rules(url, method).await;

        let mut current_req = req;

        for rule in &rules {
            match &rule.action {
                InterceptAction::Block { status_code, body } => {
                    info!(
                        "[Intercept] 요청 차단: {} {} -> {} (규칙: {})",
                        method, url, status_code, rule.name
                    );
                    let mut response = Response::builder()
                        .status(StatusCode::from_u16(*status_code).unwrap_or(StatusCode::FORBIDDEN))
                        .header("x-cheolsu-intercepted", "true")
                        .header("x-cheolsu-intercept-rule", &rule.id)
                        .body(Body::from(body.clone()))
                        .unwrap_or_else(|_| {
                            Response::builder()
                                .status(StatusCode::FORBIDDEN)
                                .body(Body::from("Blocked by intercept rule"))
                                .unwrap()
                        });
                    // Content-Type 설정
                    if !body.is_empty() {
                        if body.starts_with('{') || body.starts_with('[') {
                            response
                                .headers_mut()
                                .insert("content-type", "application/json".parse().unwrap());
                        } else {
                            response.headers_mut().insert(
                                "content-type",
                                "text/plain; charset=utf-8".parse().unwrap(),
                            );
                        }
                    }
                    return response.into();
                }
                InterceptAction::ModifyRequest {
                    add_headers,
                    remove_headers,
                    set_body,
                } => {
                    info!(
                        "[Intercept] 요청 수정: {} {} (규칙: {})",
                        method, url, rule.name
                    );
                    // 헤더 제거
                    for name in remove_headers {
                        if let Ok(header_name) = name.parse::<HeaderName>() {
                            current_req.headers_mut().remove(header_name);
                        }
                    }
                    // 헤더 추가
                    for (name, value) in add_headers {
                        if let (Ok(header_name), Ok(header_value)) =
                            (name.parse::<HeaderName>(), value.parse::<HeaderValue>())
                        {
                            current_req.headers_mut().insert(header_name, header_value);
                        }
                    }
                    // 바디 변경
                    if let Some(new_body) = set_body {
                        use http_body_util::Full;
                        *current_req.body_mut() =
                            Body::from(Full::new(bytes::Bytes::from(new_body.clone())));
                    }
                }
                InterceptAction::ModifyResponse { .. } => {
                    // 응답 수정 규칙은 handle_response에서 처리
                }
                InterceptAction::MapLocal {
                    file_path,
                    status_code,
                    headers,
                } => {
                    info!(
                        "[Intercept] Map Local: {} {} -> {} (규칙: {})",
                        method, url, file_path, rule.name
                    );
                    match std::fs::read(file_path) {
                        Ok(file_bytes) => {
                            let file_bytes = Bytes::from(file_bytes);
                            let mut response = Response::builder()
                                .status(
                                    StatusCode::from_u16(*status_code).unwrap_or(StatusCode::OK),
                                )
                                .header("x-cheolsu-intercepted", "true")
                                .header("x-cheolsu-intercept-rule", &rule.id)
                                .header("x-cheolsu-map-local", file_path.as_str());

                            // Content-Length 설정
                            response =
                                response.header("content-length", file_bytes.len().to_string());

                            // Content-Type 추론
                            let content_type = headers
                                .get("content-type")
                                .cloned()
                                .unwrap_or_else(|| Self::guess_content_type(file_path));
                            response = response.header("content-type", content_type);

                            // 추가 헤더 설정
                            for (name, value) in headers {
                                if name.to_lowercase() != "content-type" {
                                    response = response.header(name.as_str(), value.as_str());
                                }
                            }

                            return response
                                .body(Body::from(http_body_util::Full::new(file_bytes)))
                                .unwrap_or_else(|_| {
                                    Response::builder()
                                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                                        .body(Body::from("Failed to build map local response"))
                                        .unwrap()
                                })
                                .into();
                        }
                        Err(e) => {
                            error!(
                                "[Intercept] Map Local 파일 읽기 실패: {} - {}",
                                file_path, e
                            );
                            return Response::builder()
                                .status(StatusCode::NOT_FOUND)
                                .header("x-cheolsu-intercepted", "true")
                                .header("x-cheolsu-map-local-error", e.to_string())
                                .body(Body::from(format!(
                                    "Map Local Error: file not found - {}",
                                    file_path
                                )))
                                .unwrap()
                                .into();
                        }
                    }
                }
                InterceptAction::MapRemote {
                    target_url,
                    preserve_path,
                } => {
                    info!(
                        "[Intercept] Map Remote: {} {} -> {} (규칙: {}, preserve_path={})",
                        method, url, target_url, rule.name, preserve_path
                    );
                    let new_url = if *preserve_path {
                        // 원본 URL에서 path + query 추출하여 target에 붙임
                        if let Ok(original) = url.parse::<proxyapi_v2::hyper::Uri>() {
                            let path_and_query = original
                                .path_and_query()
                                .map(|pq| pq.as_str())
                                .unwrap_or("/");
                            let base = target_url.trim_end_matches('/');
                            format!("{}{}", base, path_and_query)
                        } else {
                            target_url.clone()
                        }
                    } else {
                        target_url.clone()
                    };

                    if let Ok(new_uri) = new_url.parse::<proxyapi_v2::hyper::Uri>() {
                        *current_req.uri_mut() = new_uri;
                        // Host 헤더도 새 URL에 맞게 변경
                        if let Some(host) = new_url
                            .parse::<proxyapi_v2::hyper::Uri>()
                            .ok()
                            .and_then(|u| u.host().map(|h| h.to_string()))
                        {
                            if let Ok(host_value) = host.parse::<HeaderValue>() {
                                current_req
                                    .headers_mut()
                                    .insert(proxyapi_v2::hyper::header::HOST, host_value);
                            }
                        }
                        current_req
                            .headers_mut()
                            .insert("x-cheolsu-intercepted", "true".parse().unwrap());
                        current_req.headers_mut().insert(
                            "x-cheolsu-map-remote-original",
                            url.parse().unwrap_or_else(|_| "unknown".parse().unwrap()),
                        );
                    } else {
                        error!("[Intercept] Map Remote URL 파싱 실패: {}", new_url);
                    }
                }
            }
        }

        current_req.into()
    }

    /// 인터셉트 규칙에 따라 응답을 수정
    async fn apply_response_intercept(
        &self,
        mut res: Response<Body>,
        url: &str,
        method: &str,
    ) -> Response<Body> {
        let rules = self.find_matching_intercept_rules(url, method).await;

        for rule in &rules {
            if let InterceptAction::ModifyResponse {
                set_status,
                add_headers,
                remove_headers,
                set_body,
            } = &rule.action
            {
                info!(
                    "[Intercept] 응답 수정: {} {} (규칙: {})",
                    method, url, rule.name
                );

                // 상태 코드 변경
                if let Some(status) = set_status {
                    if let Ok(status_code) = StatusCode::from_u16(*status) {
                        *res.status_mut() = status_code;
                    }
                }

                // 헤더 제거
                for name in remove_headers {
                    if let Ok(header_name) = name.parse::<HeaderName>() {
                        res.headers_mut().remove(header_name);
                    }
                }

                // 헤더 추가
                for (name, value) in add_headers {
                    if let (Ok(header_name), Ok(header_value)) =
                        (name.parse::<HeaderName>(), value.parse::<HeaderValue>())
                    {
                        res.headers_mut().insert(header_name, header_value);
                    }
                }

                // 바디 변경
                if let Some(new_body) = set_body {
                    use http_body_util::Full;
                    // Content-Length 업데이트
                    let body_bytes = bytes::Bytes::from(new_body.clone());
                    res.headers_mut().remove("content-length");
                    res.headers_mut().remove("content-encoding");
                    res.headers_mut().remove("transfer-encoding");
                    *res.body_mut() = Body::from(Full::new(body_bytes));
                }

                res.headers_mut()
                    .insert("x-cheolsu-intercepted", "true".parse().unwrap());
                res.headers_mut().insert(
                    "x-cheolsu-intercept-rule",
                    rule.id
                        .parse()
                        .unwrap_or_else(|_| "unknown".parse().unwrap()),
                );
            }
        }

        res
    }

    /// 요청과 응답을 묶어서 전송
    async fn send_output(&self) {
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
}

impl LoggingHandler {
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
                        .unwrap()
                })
        } else {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("No cached response data available"))
                .unwrap()
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
        let restored_req = if self.script_handle.is_active() {
            let script_req = Self::to_script_request(&proxied_request);
            match self.script_handle.invoke_on_request(&script_req).await {
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
            }
        } else {
            restored_req
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
        let res = if self.script_handle.is_active() {
            if let Some(req) = &self.req {
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

        // Changed from tauri::async_runtime::spawn to tokio::spawn
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
                        .unwrap_or_else(|_| {
                            Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .body(Body::from("Internal Server Error"))
                                .unwrap()
                        });
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
                match fallback_with_curl(req).await {
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
            .expect("Failed to build error response")
    }
}

/// curl을 사용해서 직접 요청을 보내고 응답을 받는 함수
pub async fn fallback_with_curl(
    req: &ProxiedRequest,
) -> Result<Response<Body>, Box<dyn std::error::Error>> {
    use std::process::Command;
    use std::str;

    let url = req.uri().to_string();
    let method = req.method().to_string();

    let mut curl_cmd = Command::new("curl");
    curl_cmd
        .arg("-s")
        .arg("-i")
        .arg("-X")
        .arg(&method)
        .arg("--max-time")
        .arg("10")
        .arg("--connect-timeout")
        .arg("5")
        .arg("--insecure");

    for (name, value) in req.headers() {
        let name_str = name.as_str();
        if let Ok(value_str) = value.to_str() {
            if name_str.to_lowercase() != "host" {
                curl_cmd
                    .arg("-H")
                    .arg(format!("{}: {}", name_str, value_str));
            }
        }
    }

    curl_cmd.arg(&url);

    debug!("curl 명령어 실행: {:?}", curl_cmd);

    let output = curl_cmd.output()?;

    if !output.status.success() {
        return Err(format!("curl 실행 실패: {}", output.status).into());
    }

    let response_text = str::from_utf8(&output.stdout)?;
    debug!("curl 응답 길이: {} bytes", response_text.len());

    parse_curl_response(response_text)
}

/// curl 응답을 HTTP Response로 파싱하는 함수
pub fn parse_curl_response(
    response_text: &str,
) -> Result<Response<Body>, Box<dyn std::error::Error>> {
    let lines: Vec<&str> = response_text.lines().collect();
    if lines.is_empty() {
        return Err("빈 응답".into());
    }

    let status_line = lines[0];
    let parts: Vec<&str> = status_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err("잘못된 상태 라인".into());
    }

    let status_code = parts[1].parse::<u16>()?;
    let status = StatusCode::from_u16(status_code)?;

    let mut header_end = 0;
    for (i, line) in lines.iter().enumerate() {
        if line.is_empty() {
            header_end = i;
            break;
        }
    }

    let mut headers = HeaderMap::new();
    for line in &lines[1..header_end] {
        if let Some(colon_pos) = line.find(':') {
            let name = &line[..colon_pos].trim();
            let value = &line[colon_pos + 1..].trim();

            if name.to_lowercase() == "content-length" {
                continue;
            }

            if let (Ok(header_name), Ok(header_value)) =
                (name.parse::<HeaderName>(), value.parse::<HeaderValue>())
            {
                headers.insert(header_name, header_value);
            }
        }
    }

    let body_text = if header_end + 1 < lines.len() {
        lines[header_end + 1..].join("\n")
    } else {
        String::new()
    };

    let mut response = Response::builder()
        .status(status)
        .body(Body::from(body_text))?;

    *response.headers_mut() = headers;

    Ok(response)
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
        if let Some(ws_sender) = &self.ws_sender {
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

            let sequence = self
                .ws_sequence
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            let content_type = proxy_v2_models::detect_ws_content_type(&payload, is_binary);

            // MQTT 메시지인 경우 버전 추적
            let mqtt_version = if content_type == proxy_v2_models::WsContentType::Mqtt {
                // CONNECT 패킷이면 버전을 추출하여 저장
                if let Some(ver) = proxy_v2_models::extract_mqtt_version_from_connect(&payload) {
                    self.mqtt_versions
                        .lock()
                        .unwrap()
                        .insert(connection_id.clone(), ver);
                    Some(ver)
                } else {
                    // 다른 MQTT 패킷이면 저장된 버전 참조
                    self.mqtt_versions
                        .lock()
                        .unwrap()
                        .get(&connection_id)
                        .copied()
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
    use crate::protocol::{InterceptAction, InterceptRule};

    // --- wildcard_matches 테스트 ---

    #[test]
    fn test_wildcard_star_prefix() {
        // *패턴 → URL 뒷부분 매칭
        assert!(LoggingHandler::wildcard_matches(
            "*ads.example.com*",
            "https://ads.example.com/banner"
        ));
    }

    #[test]
    fn test_wildcard_subdomain_and_path() {
        assert!(LoggingHandler::wildcard_matches(
            "*.example.com/api/*",
            "https://sub.example.com/api/v1/users"
        ));
    }

    #[test]
    fn test_wildcard_no_match() {
        assert!(!LoggingHandler::wildcard_matches(
            "*ads.example.com*",
            "https://other.com/page"
        ));
    }

    #[test]
    fn test_wildcard_exact_domain() {
        assert!(LoggingHandler::wildcard_matches(
            "*example.com*",
            "https://example.com"
        ));
    }

    #[test]
    fn test_wildcard_path_only() {
        assert!(LoggingHandler::wildcard_matches(
            "*/api/v1/*",
            "https://any.com/api/v1/users"
        ));
        assert!(!LoggingHandler::wildcard_matches(
            "*/api/v1/*",
            "https://any.com/api/v2/users"
        ));
    }

    #[test]
    fn test_wildcard_question_mark() {
        assert!(LoggingHandler::wildcard_matches(
            "*api/v?/users*",
            "https://example.com/api/v1/users"
        ));
        assert!(LoggingHandler::wildcard_matches(
            "*api/v?/users*",
            "https://example.com/api/v2/users"
        ));
        assert!(!LoggingHandler::wildcard_matches(
            "*api/v?/users*",
            "https://example.com/api/v10/users"
        ));
    }

    #[test]
    fn test_wildcard_case_insensitive() {
        assert!(LoggingHandler::wildcard_matches(
            "*Example.COM*",
            "https://example.com/page"
        ));
    }

    #[test]
    fn test_wildcard_catch_all() {
        assert!(LoggingHandler::wildcard_matches(
            "*",
            "https://anything.com/any/path"
        ));
    }

    #[test]
    fn test_wildcard_no_wildcards_partial() {
        // 와일드카드 없으면 부분 매칭 (regex is_match)
        assert!(LoggingHandler::wildcard_matches(
            "example.com",
            "https://example.com/api"
        ));
    }

    #[test]
    fn test_wildcard_special_chars_escaped() {
        // 정규식 특수문자가 이스케이프 되는지
        assert!(LoggingHandler::wildcard_matches(
            "*example.com/api?key=*",
            "https://example.com/api?key=value"
        ));
    }

    // --- rule_matches 테스트 ---

    fn make_rule(pattern: &str, method: Option<&str>, enabled: bool) -> InterceptRule {
        InterceptRule {
            id: "test".to_string(),
            name: "Test".to_string(),
            enabled,
            pattern: pattern.to_string(),
            method: method.map(|m| m.to_string()),
            action: InterceptAction::Block {
                status_code: 403,
                body: String::new(),
            },
        }
    }

    #[test]
    fn test_rule_matches_basic() {
        let rule = make_rule("*example.com*", None, true);
        assert!(LoggingHandler::rule_matches(
            &rule,
            "https://example.com/api",
            "GET"
        ));
    }

    #[test]
    fn test_rule_matches_with_method() {
        let rule = make_rule("*api.com*", Some("POST"), true);
        assert!(LoggingHandler::rule_matches(
            &rule,
            "https://api.com/v1",
            "POST"
        ));
        assert!(!LoggingHandler::rule_matches(
            &rule,
            "https://api.com/v1",
            "GET"
        ));
    }

    #[test]
    fn test_rule_matches_method_case_insensitive() {
        let rule = make_rule("*api.com*", Some("post"), true);
        assert!(LoggingHandler::rule_matches(
            &rule,
            "https://api.com/v1",
            "POST"
        ));
    }

    #[test]
    fn test_rule_matches_disabled() {
        let rule = make_rule("*example.com*", None, false);
        assert!(!LoggingHandler::rule_matches(
            &rule,
            "https://example.com/api",
            "GET"
        ));
    }

    #[test]
    fn test_rule_matches_no_method_filter_matches_all() {
        let rule = make_rule("*example.com*", None, true);
        assert!(LoggingHandler::rule_matches(
            &rule,
            "https://example.com",
            "GET"
        ));
        assert!(LoggingHandler::rule_matches(
            &rule,
            "https://example.com",
            "POST"
        ));
        assert!(LoggingHandler::rule_matches(
            &rule,
            "https://example.com",
            "DELETE"
        ));
    }

    #[test]
    fn test_rule_matches_complex_pattern() {
        let rule = make_rule("*.example.com/api/*/users", Some("GET"), true);
        assert!(LoggingHandler::rule_matches(
            &rule,
            "https://sub.example.com/api/v1/users",
            "GET"
        ));
        assert!(!LoggingHandler::rule_matches(
            &rule,
            "https://sub.example.com/api/v1/posts",
            "GET"
        ));
        assert!(!LoggingHandler::rule_matches(
            &rule,
            "https://sub.example.com/api/v1/users",
            "POST"
        ));
    }
}

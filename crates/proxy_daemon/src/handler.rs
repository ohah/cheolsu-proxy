use bytes::Bytes;
use futures_util::stream::StreamExt;
use http_body_util::{BodyExt, StreamBody};
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client},
    rt::TokioExecutor,
};
use proxy_v2_models::{ProxiedRequest, ProxiedResponse, RequestInfo};
use proxyapi_v2::{
    hyper::http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
    hyper::{Request, Response},
    tokio_tungstenite::tungstenite::Message,
    Body, HttpContext, HttpHandler, RequestOrResponse, WebSocketContext, WebSocketHandler,
};
use regex::Regex;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info};
use tokio_rustls::rustls::{crypto::aws_lc_rs, ClientConfig};
use tokio_stream::wrappers::ReceiverStream;

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

/// 하이브리드 클라이언트 생성 (모든 인증서 허용)
pub fn create_hybrid_client(
) -> Result<Client<hyper_rustls::HttpsConnector<HttpConnector>, Body>, Box<dyn std::error::Error>> {
    let rustls_config =
        ClientConfig::builder_with_provider(std::sync::Arc::new(aws_lc_rs::default_provider()))
            .with_safe_default_protocol_versions()?
            .dangerous()
            .with_custom_certificate_verifier(std::sync::Arc::new(DangerousCertificateVerifier))
            .with_no_client_auth();

    let https = HttpsConnectorBuilder::new()
        .with_tls_config(rustls_config)
        .https_or_http()
        .enable_http1()
        .build();

    Ok(Client::builder(TokioExecutor::new())
        .http1_title_case_headers(true)
        .http1_preserve_header_case(true)
        .build(https))
}

/// HTTP 및 WebSocket 요청/응답을 로깅하는 핸들러
#[derive(Clone)]
pub struct LoggingHandler {
    sender: tokio::sync::mpsc::Sender<RequestInfo>,
    req: Option<ProxiedRequest>,
    res: Option<ProxiedResponse>,
    sessions: Arc<Mutex<serde_json::Value>>,
    cache_dir: Option<std::path::PathBuf>,
}

impl LoggingHandler {
    pub fn new(sender: tokio::sync::mpsc::Sender<RequestInfo>, cache_dir: std::path::PathBuf) -> Self {
        Self {
            sender,
            req: None,
            res: None,
            sessions: Arc::new(Mutex::new(serde_json::Value::Array(Vec::new()))),
            cache_dir: Some(cache_dir),
        }
    }

    /// 세션 데이터 업데이트
    pub async fn update_sessions(&self, sessions: serde_json::Value) {
        let mut sessions_guard = self.sessions.lock().await;
        *sessions_guard = sessions;
    }

    /// 요청 URL이 세션에 있는지 확인하고 매칭되는 세션 반환
    async fn find_matching_session(&self, url: &str, method: &str) -> Option<serde_json::Value> {
        let sessions_guard = self.sessions.lock().await;
        if let serde_json::Value::Array(sessions) = &*sessions_guard {
            sessions
                .iter()
                .find(|session| {
                    let session_url = session.get("url").and_then(|v| v.as_str()).unwrap_or("");
                    let session_method = session
                        .get("method")
                        .and_then(|v| v.as_str())
                        .unwrap_or("GET");

                    (url.contains(session_url) || session_url.contains(url))
                        && session_method.to_uppercase() == method.to_uppercase()
                })
                .cloned()
        } else {
            None
        }
    }

    /// 세션 데이터로부터 HTTP 응답을 생성하는 메서드
    fn create_response_from_session(&self, response_data: &serde_json::Value) -> Response<Body> {
        let status_code = response_data
            .get("status")
            .and_then(|v| v.as_u64())
            .unwrap_or(200) as u16;

        let mut headers: HeaderMap = response_data
            .get("headers")
            .and_then(serde_json::Value::as_object)
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| {
                        if k.to_lowercase() == "content-length" {
                            return None;
                        }
                        Some((k.parse().ok()?, v.as_str()?.parse().ok()?))
                    })
                    .collect()
            })
            .unwrap_or_default();

        if !headers.contains_key("content-type") {
            headers.insert("content-type", "application/json".parse().unwrap());
        }

        headers.insert("x-cheolsu-proxy-session", "true".parse().unwrap());
        headers.insert("x-cheolsu-proxy-version", "v2".parse().unwrap());

        let body = if let Some(data) = response_data.get("data") {
            match data {
                serde_json::Value::String(s) => Body::from(s.clone()),
                serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
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

        let mut response = Response::new(body);
        *response.status_mut() = StatusCode::from_u16(status_code).unwrap_or(StatusCode::OK);
        *response.headers_mut() = headers;

        response
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

        self.req = Some(proxied_request);

        restored_req.into()
    }

    async fn handle_response(&mut self, _ctx: &HttpContext, res: Response<Body>) -> Response<Body> {
        if res.status() == StatusCode::SWITCHING_PROTOCOLS {
            return res;
        }

        if let Some(req) = &self.req {
            let url = req.uri().to_string();
            let method = req.method().to_string();

            if let Some(session) = self.find_matching_session(&url, &method).await {
                let default_response = serde_json::Value::Object(serde_json::Map::new());
                let response_data = session.get("response").unwrap_or(&default_response);
                let mut session_response = self.create_response_from_session(response_data);

                let session_body_bytes =
                    match Self::body_to_bytes_from_mut(&mut session_response.body_mut()).await {
                        Ok(bytes) => bytes,
                        Err(_) => Bytes::from("세션 응답 읽기 실패"),
                    };

                let session_proxied_response = ProxiedResponse::new(
                    session_response.status(),
                    session_response.version(),
                    session_response.headers().clone(),
                    session_body_bytes.clone(),
                    chrono::Local::now()
                        .timestamp_nanos_opt()
                        .unwrap_or_default(),
                );

                self.res = Some(session_proxied_response);

                self.send_output().await;

                use http_body_util::Full;
                *session_response.body_mut() = Body::from(Full::new(session_body_bytes));
                return session_response;
            }
        }

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
    async fn handle_message(&mut self, _ctx: &WebSocketContext, msg: Message) -> Option<Message> {
        Some(msg)
    }
}

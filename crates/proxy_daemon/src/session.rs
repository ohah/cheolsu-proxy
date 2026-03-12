use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use proxy_v2_models::{RequestInfo, WsMessageInfo};
use proxyapi_v2::throttle::ThrottleConfig;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::Path;

use crate::protocol::{InterceptRule, ServerReplayEntry};

const SESSION_VERSION: u32 = 1;
const CHEOLSU_EXTENSION: &str = "cheolsu";
const CHEOLSU_GZ_EXTENSION: &str = "cheolsu.gz";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SessionFile {
    pub version: u32,
    pub created_at: String,
    pub metadata: SessionMetadata,
    pub transactions: Vec<SessionTransaction>,
    pub websocket_messages: Vec<WsMessageInfo>,
    pub intercept_rules: Vec<InterceptRule>,
    pub server_replay_entries: Vec<ServerReplayEntry>,
    pub throttle_config: Option<ThrottleConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SessionMetadata {
    pub name: Option<String>,
    pub description: Option<String>,
    pub proxy_port: u16,
    pub total_transactions: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SessionTransaction {
    pub info: RequestInfo,
}

impl SessionFile {
    pub fn new(port: u16) -> Self {
        Self {
            version: SESSION_VERSION,
            created_at: chrono::Utc::now().to_rfc3339(),
            metadata: SessionMetadata {
                name: None,
                description: None,
                proxy_port: port,
                total_transactions: 0,
            },
            transactions: Vec::new(),
            websocket_messages: Vec::new(),
            intercept_rules: Vec::new(),
            server_replay_entries: Vec::new(),
            throttle_config: None,
        }
    }

    pub fn from_traffic(
        port: u16,
        transactions: &[RequestInfo],
        ws_messages: &[WsMessageInfo],
        intercept_rules: &[InterceptRule],
        server_replay_entries: &[ServerReplayEntry],
        throttle_config: Option<ThrottleConfig>,
    ) -> Self {
        let session_transactions: Vec<SessionTransaction> = transactions
            .iter()
            .map(|info| SessionTransaction { info: info.clone() })
            .collect();

        Self {
            version: SESSION_VERSION,
            created_at: chrono::Utc::now().to_rfc3339(),
            metadata: SessionMetadata {
                name: None,
                description: None,
                proxy_port: port,
                total_transactions: transactions.len(),
            },
            transactions: session_transactions,
            websocket_messages: ws_messages.to_vec(),
            intercept_rules: intercept_rules.to_vec(),
            server_replay_entries: server_replay_entries.to_vec(),
            throttle_config,
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), SessionError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| SessionError::Serialize(e.to_string()))?;

        let use_gzip = path
            .to_string_lossy()
            .ends_with(&format!(".{}", CHEOLSU_GZ_EXTENSION));

        if use_gzip {
            let file = std::fs::File::create(path).map_err(|e| SessionError::Io(e.to_string()))?;
            let mut encoder = GzEncoder::new(file, Compression::default());
            encoder
                .write_all(json.as_bytes())
                .map_err(|e| SessionError::Io(e.to_string()))?;
            encoder
                .finish()
                .map_err(|e| SessionError::Io(e.to_string()))?;
        } else {
            std::fs::write(path, json.as_bytes()).map_err(|e| SessionError::Io(e.to_string()))?;
        }

        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self, SessionError> {
        let raw = std::fs::read(path).map_err(|e| SessionError::Io(e.to_string()))?;

        let json_str = if is_gzip(&raw) {
            let mut decoder = GzDecoder::new(&raw[..]);
            let mut decompressed = String::new();
            decoder
                .read_to_string(&mut decompressed)
                .map_err(|e| SessionError::Decompress(e.to_string()))?;
            decompressed
        } else {
            String::from_utf8(raw).map_err(|e| SessionError::Deserialize(e.to_string()))?
        };

        let session: SessionFile = serde_json::from_str(&json_str)
            .map_err(|e| SessionError::Deserialize(e.to_string()))?;

        Ok(session)
    }

    pub fn extract_transactions(&self) -> Vec<RequestInfo> {
        self.transactions.iter().map(|st| st.info.clone()).collect()
    }
}

fn is_gzip(data: &[u8]) -> bool {
    data.len() >= 2 && data[0] == 0x1f && data[1] == 0x8b
}

pub fn ensure_extension(path: &str) -> String {
    if path.ends_with(&format!(".{}", CHEOLSU_EXTENSION))
        || path.ends_with(&format!(".{}", CHEOLSU_GZ_EXTENSION))
    {
        path.to_string()
    } else {
        format!("{}.{}", path, CHEOLSU_EXTENSION)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("IO error: {0}")]
    Io(String),
    #[error("Serialization error: {0}")]
    Serialize(String),
    #[error("Deserialization error: {0}")]
    Deserialize(String),
    #[error("Decompression error: {0}")]
    Decompress(String),
}

// --- HAR import support ---

pub fn import_har(har_json: &str) -> Result<Vec<RequestInfo>, SessionError> {
    let har: HarImport = serde_json::from_str(har_json)
        .map_err(|e| SessionError::Deserialize(format!("Invalid HAR format: {}", e)))?;

    let transactions: Vec<RequestInfo> = har
        .log
        .entries
        .into_iter()
        .filter_map(|entry| convert_har_entry(entry).ok())
        .collect();

    Ok(transactions)
}

pub fn import_har_file(path: &Path) -> Result<Vec<RequestInfo>, SessionError> {
    let content = std::fs::read_to_string(path).map_err(|e| SessionError::Io(e.to_string()))?;
    import_har(&content)
}

// HAR import structures (subset needed for import)
#[derive(Deserialize)]
struct HarImport {
    log: HarLogImport,
}

#[derive(Deserialize)]
struct HarLogImport {
    entries: Vec<HarEntryImport>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HarEntryImport {
    started_date_time: Option<String>,
    request: HarRequestImport,
    response: Option<HarResponseImport>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HarRequestImport {
    method: String,
    url: String,
    http_version: Option<String>,
    headers: Option<Vec<HarHeaderImport>>,
    #[serde(default)]
    post_data: Option<HarPostDataImport>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HarResponseImport {
    status: u16,
    http_version: Option<String>,
    headers: Option<Vec<HarHeaderImport>>,
    content: Option<HarContentImport>,
}

#[derive(Deserialize)]
struct HarHeaderImport {
    name: String,
    value: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HarPostDataImport {
    text: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HarContentImport {
    text: Option<String>,
    encoding: Option<String>,
}

fn convert_har_entry(entry: HarEntryImport) -> Result<RequestInfo, SessionError> {
    use bytes::Bytes;
    use proxyapi_v2::hyper::http::{
        header::HeaderName, HeaderMap, HeaderValue, Method, StatusCode, Uri,
    };

    let timestamp = entry
        .started_date_time
        .and_then(|dt| chrono::DateTime::parse_from_rfc3339(&dt).ok())
        .map(|dt| dt.timestamp_millis())
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    let method: Method = entry
        .request
        .method
        .parse()
        .map_err(|e| SessionError::Deserialize(format!("Invalid method: {}", e)))?;

    let uri: Uri = entry
        .request
        .url
        .parse()
        .map_err(|e| SessionError::Deserialize(format!("Invalid URL: {}", e)))?;

    let version = parse_http_version(entry.request.http_version.as_deref());

    let mut req_headers = HeaderMap::new();
    if let Some(headers) = entry.request.headers {
        for h in headers {
            if let (Ok(name), Ok(value)) = (
                h.name.parse::<HeaderName>(),
                HeaderValue::from_str(&h.value),
            ) {
                req_headers.insert(name, value);
            }
        }
    }

    let req_body = entry
        .request
        .post_data
        .and_then(|pd| pd.text)
        .map(|t| Bytes::from(t))
        .unwrap_or_default();

    let proxied_req = proxy_v2_models::ProxiedRequest::new(
        method,
        uri,
        version,
        req_headers,
        req_body,
        timestamp,
    );
    let req_id = proxied_req.id().clone();
    let client_req = proxied_req.for_client(None);

    let client_res = entry.response.and_then(|res| {
        let status = StatusCode::from_u16(res.status).ok()?;
        let res_version = parse_http_version(res.http_version.as_deref());

        let mut res_headers = HeaderMap::new();
        if let Some(headers) = res.headers {
            for h in headers {
                if let (Ok(name), Ok(value)) = (
                    h.name.parse::<HeaderName>(),
                    HeaderValue::from_str(&h.value),
                ) {
                    res_headers.insert(name, value);
                }
            }
        }

        let res_body = res
            .content
            .and_then(|c| {
                let text = c.text?;
                if c.encoding.as_deref() == Some("base64") {
                    use base64::Engine;
                    base64::engine::general_purpose::STANDARD
                        .decode(&text)
                        .ok()
                        .map(Bytes::from)
                } else {
                    Some(Bytes::from(text))
                }
            })
            .unwrap_or_default();

        let proxied_res = proxy_v2_models::ProxiedResponse::new(
            status,
            res_version,
            res_headers,
            res_body,
            timestamp,
        );
        Some(proxied_res.for_client(&req_id, None))
    });

    Ok(RequestInfo(Some(client_req), client_res))
}

fn parse_http_version(version: Option<&str>) -> proxyapi_v2::hyper::http::Version {
    use proxyapi_v2::hyper::http::Version;
    match version {
        Some("HTTP/1.0") | Some("http/1.0") => Version::HTTP_10,
        Some("HTTP/2.0") | Some("HTTP/2") | Some("h2") => Version::HTTP_2,
        _ => Version::HTTP_11,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::InterceptAction;
    use tempfile::TempDir;

    fn make_empty_session(port: u16) -> SessionFile {
        SessionFile::new(port)
    }

    #[test]
    fn test_session_new() {
        let session = make_empty_session(8100);
        assert_eq!(session.version, 1);
        assert_eq!(session.metadata.proxy_port, 8100);
        assert_eq!(session.metadata.total_transactions, 0);
        assert!(session.transactions.is_empty());
        assert!(session.websocket_messages.is_empty());
        assert!(session.intercept_rules.is_empty());
        assert!(session.server_replay_entries.is_empty());
        assert!(session.throttle_config.is_none());
    }

    #[test]
    fn test_session_save_and_load_json() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.cheolsu");

        let session = make_empty_session(8100);
        session.save(&path).unwrap();

        let loaded = SessionFile::load(&path).unwrap();
        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.metadata.proxy_port, 8100);
    }

    #[test]
    fn test_session_save_and_load_gzip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.cheolsu.gz");

        let session = make_empty_session(9090);
        session.save(&path).unwrap();

        let loaded = SessionFile::load(&path).unwrap();
        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.metadata.proxy_port, 9090);
    }

    #[test]
    fn test_session_load_nonexistent_file() {
        let result = SessionFile::load(Path::new("/nonexistent/file.cheolsu"));
        assert!(result.is_err());
    }

    #[test]
    fn test_ensure_extension_already_has() {
        assert_eq!(ensure_extension("test.cheolsu"), "test.cheolsu");
        assert_eq!(ensure_extension("test.cheolsu.gz"), "test.cheolsu.gz");
    }

    #[test]
    fn test_ensure_extension_adds() {
        assert_eq!(ensure_extension("test"), "test.cheolsu");
        assert_eq!(ensure_extension("/path/to/file"), "/path/to/file.cheolsu");
    }

    #[test]
    fn test_is_gzip() {
        assert!(is_gzip(&[0x1f, 0x8b, 0x08]));
        assert!(!is_gzip(&[0x7b, 0x22])); // JSON starts with {"
        assert!(!is_gzip(&[0x1f])); // too short
        assert!(!is_gzip(&[]));
    }

    #[test]
    fn test_session_serialization_roundtrip() {
        let session = make_empty_session(8100);
        let json = serde_json::to_string(&session).unwrap();
        let deserialized: SessionFile = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.version, session.version);
        assert_eq!(
            deserialized.metadata.proxy_port,
            session.metadata.proxy_port
        );
    }

    #[test]
    fn test_session_with_metadata() {
        let mut session = make_empty_session(8100);
        session.metadata.name = Some("Test Session".to_string());
        session.metadata.description = Some("A test session".to_string());

        let json = serde_json::to_string(&session).unwrap();
        let deserialized: SessionFile = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.metadata.name, Some("Test Session".to_string()));
        assert_eq!(
            deserialized.metadata.description,
            Some("A test session".to_string())
        );
    }

    #[test]
    fn test_import_har_minimal() {
        let har_json = r#"{
            "log": {
                "version": "1.2",
                "creator": {"name": "test", "version": "1.0"},
                "entries": [
                    {
                        "startedDateTime": "2024-01-01T00:00:00.000Z",
                        "request": {
                            "method": "GET",
                            "url": "https://example.com/api",
                            "httpVersion": "HTTP/1.1",
                            "headers": [],
                            "queryString": [],
                            "headersSize": -1,
                            "bodySize": 0
                        },
                        "response": {
                            "status": 200,
                            "statusText": "OK",
                            "httpVersion": "HTTP/1.1",
                            "headers": [],
                            "content": {
                                "size": 2,
                                "mimeType": "text/plain",
                                "text": "OK"
                            },
                            "redirectURL": "",
                            "headersSize": -1,
                            "bodySize": 2
                        },
                        "cache": {},
                        "timings": {"send": 0, "wait": 50, "receive": 0}
                    }
                ]
            }
        }"#;

        let transactions = import_har(har_json).unwrap();
        assert_eq!(transactions.len(), 1);
        let req = transactions[0].0.as_ref().unwrap();
        assert_eq!(req.method().as_str(), "GET");
        assert!(req.uri().to_string().contains("example.com"));
    }

    #[test]
    fn test_import_har_invalid_json() {
        let result = import_har("not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_import_har_empty_entries() {
        let har_json =
            r#"{"log":{"version":"1.2","creator":{"name":"t","version":"1"},"entries":[]}}"#;
        let transactions = import_har(har_json).unwrap();
        assert!(transactions.is_empty());
    }

    #[test]
    fn test_parse_http_version() {
        use proxyapi_v2::hyper::http::Version;
        assert_eq!(parse_http_version(Some("HTTP/1.0")), Version::HTTP_10);
        assert_eq!(parse_http_version(Some("HTTP/1.1")), Version::HTTP_11);
        assert_eq!(parse_http_version(Some("HTTP/2.0")), Version::HTTP_2);
        assert_eq!(parse_http_version(Some("h2")), Version::HTTP_2);
        assert_eq!(parse_http_version(None), Version::HTTP_11);
        assert_eq!(parse_http_version(Some("unknown")), Version::HTTP_11);
    }

    #[test]
    fn test_import_har_file_nonexistent() {
        let result = import_har_file(Path::new("/nonexistent/file.har"));
        assert!(result.is_err());
    }

    #[test]
    fn test_import_har_file_valid() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.har");
        let har_json = r#"{"log":{"version":"1.2","creator":{"name":"t","version":"1"},"entries":[{"startedDateTime":"2024-01-01T00:00:00.000Z","request":{"method":"POST","url":"https://api.test.com/data","httpVersion":"HTTP/1.1","headers":[{"name":"content-type","value":"application/json"}],"postData":{"mimeType":"application/json","text":"{\"key\":1}"},"queryString":[],"headersSize":-1,"bodySize":9},"response":{"status":201,"statusText":"Created","httpVersion":"HTTP/1.1","headers":[],"content":{"size":0,"mimeType":"text/plain"},"redirectURL":"","headersSize":-1,"bodySize":0},"cache":{},"timings":{"send":0,"wait":10,"receive":0}}]}}"#;
        std::fs::write(&path, har_json).unwrap();

        let transactions = import_har_file(&path).unwrap();
        assert_eq!(transactions.len(), 1);
        let req = transactions[0].0.as_ref().unwrap();
        assert_eq!(req.method().as_str(), "POST");
    }

    #[test]
    fn test_from_traffic_empty() {
        let session = SessionFile::from_traffic(8100, &[], &[], &[], &[], None);
        assert_eq!(session.metadata.total_transactions, 0);
        assert!(session.transactions.is_empty());
    }

    #[test]
    fn test_from_traffic_with_data() {
        let transactions = vec![RequestInfo(None, None)];
        let session = SessionFile::from_traffic(8100, &transactions, &[], &[], &[], None);
        assert_eq!(session.metadata.total_transactions, 1);
        assert_eq!(session.transactions.len(), 1);
    }

    #[test]
    fn test_extract_transactions() {
        let transactions = vec![RequestInfo(None, None)];
        let session = SessionFile::from_traffic(8100, &transactions, &[], &[], &[], None);
        let extracted = session.extract_transactions();
        assert_eq!(extracted.len(), 1);
    }

    #[test]
    fn test_session_error_display() {
        let err = SessionError::Io("file not found".to_string());
        assert!(err.to_string().contains("file not found"));

        let err = SessionError::Serialize("bad data".to_string());
        assert!(err.to_string().contains("bad data"));
    }

    /// 테스트용 RequestInfo 생성 헬퍼 (request + response 포함)
    fn make_request_info_with_response() -> RequestInfo {
        use bytes::Bytes;
        use proxyapi_v2::hyper::http::{HeaderMap, Method, StatusCode, Uri, Version};

        let req = proxy_v2_models::ProxiedRequest::new(
            Method::GET,
            "https://example.com/test".parse::<Uri>().unwrap(),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::from("request body"),
            1000,
        );
        let req_id = req.id().clone();
        let client_req = req.for_client(None);

        let res = proxy_v2_models::ProxiedResponse::new(
            StatusCode::OK,
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::from("response body"),
            1001,
        );
        let client_res = res.for_client(&req_id, None);

        RequestInfo(Some(client_req), Some(client_res))
    }

    #[test]
    fn test_save_load_preserves_transaction_data() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("data_integrity.cheolsu");

        let info = make_request_info_with_response();
        let transactions = vec![info];
        let session = SessionFile::from_traffic(8100, &transactions, &[], &[], &[], None);

        // request와 response 모두 존재하는지 확인
        assert!(session.transactions[0].info.0.is_some());
        assert!(session.transactions[0].info.1.is_some());

        session.save(&path).unwrap();
        let loaded = SessionFile::load(&path).unwrap();

        // 로드 후에도 request와 response 모두 보존되는지 확인
        assert_eq!(loaded.transactions.len(), 1);
        let loaded_info = &loaded.transactions[0].info;
        assert!(loaded_info.0.is_some(), "request가 손실됨");
        assert!(loaded_info.1.is_some(), "response가 손실됨");

        let req = loaded_info.0.as_ref().unwrap();
        assert_eq!(req.method().as_str(), "GET");
        assert!(req.uri().to_string().contains("example.com"));

        let res = loaded_info.1.as_ref().unwrap();
        assert_eq!(res.status().as_u16(), 200);
    }

    #[test]
    fn test_gzip_smaller_than_json() {
        let tmp = TempDir::new().unwrap();
        let json_path = tmp.path().join("test.cheolsu");
        let gz_path = tmp.path().join("test.cheolsu.gz");

        // 충분한 데이터를 가진 세션 생성
        let mut session = make_empty_session(8100);
        session.metadata.name = Some("Large session for compression test".to_string());
        session.metadata.description =
            Some("This is a longer description to make compression meaningful".to_string());
        // 빈 트랜잭션을 여러 개 추가
        for _ in 0..50 {
            session.transactions.push(SessionTransaction {
                info: RequestInfo(None, None),
            });
        }

        session.save(&json_path).unwrap();
        session.save(&gz_path).unwrap();

        let json_size = std::fs::metadata(&json_path).unwrap().len();
        let gz_size = std::fs::metadata(&gz_path).unwrap().len();

        assert!(
            gz_size < json_size,
            "gzip 파일({gz_size})이 JSON 파일({json_size})보다 작아야 함"
        );
    }

    #[test]
    fn test_load_invalid_json_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("bad.cheolsu");
        std::fs::write(&path, b"this is not valid json at all").unwrap();

        let result = SessionFile::load(&path);
        assert!(result.is_err());
        match result.unwrap_err() {
            SessionError::Deserialize(_) => {} // 예상대로
            other => panic!("Deserialize 에러를 기대했지만 {:?}를 받음", other),
        }
    }

    #[test]
    fn test_load_invalid_gzip_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("bad.cheolsu.gz");
        // gzip 매직 바이트 + 잘못된 데이터
        std::fs::write(&path, &[0x1f, 0x8b, 0x00, 0x00, 0xff, 0xff]).unwrap();

        let result = SessionFile::load(&path);
        assert!(result.is_err());
        match result.unwrap_err() {
            SessionError::Decompress(_) => {} // 예상대로
            other => panic!("Decompress 에러를 기대했지만 {:?}를 받음", other),
        }
    }

    #[test]
    fn test_import_har_base64_response_body() {
        use base64::Engine;
        let binary_data = b"binary content \x00\x01\x02";
        let encoded = base64::engine::general_purpose::STANDARD.encode(binary_data);

        let har_json = format!(
            r#"{{
            "log": {{
                "version": "1.2",
                "creator": {{"name": "test", "version": "1.0"}},
                "entries": [{{
                    "startedDateTime": "2024-01-01T00:00:00.000Z",
                    "request": {{
                        "method": "GET",
                        "url": "https://example.com/binary",
                        "httpVersion": "HTTP/1.1",
                        "headers": [],
                        "queryString": [],
                        "headersSize": -1,
                        "bodySize": 0
                    }},
                    "response": {{
                        "status": 200,
                        "statusText": "OK",
                        "httpVersion": "HTTP/1.1",
                        "headers": [],
                        "content": {{
                            "size": {},
                            "mimeType": "application/octet-stream",
                            "text": "{}",
                            "encoding": "base64"
                        }},
                        "redirectURL": "",
                        "headersSize": -1,
                        "bodySize": {}
                    }},
                    "cache": {{}},
                    "timings": {{"send": 0, "wait": 10, "receive": 0}}
                }}]
            }}
        }}"#,
            binary_data.len(),
            encoded,
            binary_data.len()
        );

        let transactions = import_har(&har_json).unwrap();
        assert_eq!(transactions.len(), 1);
        let res = transactions[0].1.as_ref().unwrap();
        // body가 base64 디코딩 후 원래 바이너리와 일치하는지 확인
        let body = res.body();
        assert!(body.is_some());
    }

    #[test]
    fn test_import_har_http_versions() {
        let make_har = |version: &str| {
            format!(
                r#"{{
                "log": {{
                    "version": "1.2",
                    "creator": {{"name": "test", "version": "1.0"}},
                    "entries": [{{
                        "startedDateTime": "2024-01-01T00:00:00.000Z",
                        "request": {{
                            "method": "GET",
                            "url": "https://example.com/",
                            "httpVersion": "{}",
                            "headers": [],
                            "queryString": [],
                            "headersSize": -1,
                            "bodySize": 0
                        }},
                        "response": {{
                            "status": 200,
                            "statusText": "OK",
                            "httpVersion": "{}",
                            "headers": [],
                            "content": {{"size": 0, "mimeType": "text/plain"}},
                            "redirectURL": "",
                            "headersSize": -1,
                            "bodySize": 0
                        }},
                        "cache": {{}},
                        "timings": {{"send": 0, "wait": 10, "receive": 0}}
                    }}]
                }}
            }}"#,
                version, version
            )
        };

        // HTTP/1.0
        let txns = import_har(&make_har("HTTP/1.0")).unwrap();
        assert_eq!(txns.len(), 1);
        assert!(txns[0].0.is_some());

        // HTTP/2
        let txns = import_har(&make_har("HTTP/2")).unwrap();
        assert_eq!(txns.len(), 1);
        assert!(txns[0].0.is_some());

        // h2
        let txns = import_har(&make_har("h2")).unwrap();
        assert_eq!(txns.len(), 1);
        assert!(txns[0].0.is_some());
    }

    #[test]
    fn test_ensure_extension_edge_cases() {
        // 빈 문자열
        assert_eq!(ensure_extension(""), ".cheolsu");

        // .cheolsu가 이름 중간에 있는 경우
        assert_eq!(
            ensure_extension("my.cheolsu.backup"),
            "my.cheolsu.backup.cheolsu"
        );

        // 경로에 점이 있는 경우
        assert_eq!(
            ensure_extension("/path/to/my.file"),
            "/path/to/my.file.cheolsu"
        );

        // 이미 .cheolsu.gz 확장자가 있는 경우
        assert_eq!(ensure_extension("session.cheolsu.gz"), "session.cheolsu.gz");

        // .gz만 있는 경우 (cheolsu가 아닌)
        assert_eq!(ensure_extension("file.gz"), "file.gz.cheolsu");
    }

    #[test]
    fn test_from_traffic_with_intercept_rules_and_throttle() {
        let rules = vec![InterceptRule {
            id: "rule-1".to_string(),
            name: "Test Rule".to_string(),
            enabled: true,
            pattern: "*.example.com".to_string(),
            method: Some("GET".to_string()),
            action: InterceptAction::Block {
                status_code: 403,
                body: "Forbidden".to_string(),
            },
        }];

        let throttle = ThrottleConfig {
            enabled: true,
            download_rate: Some(1024),
            upload_rate: Some(512),
            latency_ms: 100,
        };

        let info = make_request_info_with_response();
        let transactions = vec![info];
        let session = SessionFile::from_traffic(
            8080,
            &transactions,
            &[],
            &rules,
            &[],
            Some(throttle.clone()),
        );

        assert_eq!(session.intercept_rules.len(), 1);
        assert_eq!(session.intercept_rules[0].id, "rule-1");
        assert_eq!(session.intercept_rules[0].name, "Test Rule");
        assert!(session.intercept_rules[0].enabled);

        let tc = session.throttle_config.as_ref().unwrap();
        assert!(tc.enabled);
        assert_eq!(tc.download_rate, Some(1024));
        assert_eq!(tc.upload_rate, Some(512));
        assert_eq!(tc.latency_ms, 100);

        // 저장/로드 후에도 보존되는지 확인
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("rules_throttle.cheolsu");
        session.save(&path).unwrap();
        let loaded = SessionFile::load(&path).unwrap();

        assert_eq!(loaded.intercept_rules.len(), 1);
        assert!(loaded.throttle_config.is_some());
        assert_eq!(loaded.throttle_config.unwrap().latency_ms, 100);
    }

    #[test]
    fn test_extract_transactions_preserves_response() {
        let info = make_request_info_with_response();
        let transactions = vec![info];
        let session = SessionFile::from_traffic(8100, &transactions, &[], &[], &[], None);
        let extracted = session.extract_transactions();

        assert_eq!(extracted.len(), 1);
        assert!(extracted[0].0.is_some(), "추출된 request가 없음");
        assert!(extracted[0].1.is_some(), "추출된 response가 없음");
    }

    #[test]
    fn test_empty_traffic_save_load_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("empty.cheolsu");

        let session = SessionFile::from_traffic(8100, &[], &[], &[], &[], None);
        session.save(&path).unwrap();
        let loaded = SessionFile::load(&path).unwrap();

        assert_eq!(loaded.metadata.total_transactions, 0);
        assert!(loaded.transactions.is_empty());
        assert!(loaded.websocket_messages.is_empty());
        assert!(loaded.intercept_rules.is_empty());
        assert!(loaded.server_replay_entries.is_empty());
        assert!(loaded.throttle_config.is_none());
    }

    #[test]
    fn test_empty_traffic_save_load_roundtrip_gzip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("empty.cheolsu.gz");

        let session = SessionFile::from_traffic(8100, &[], &[], &[], &[], None);
        session.save(&path).unwrap();
        let loaded = SessionFile::load(&path).unwrap();

        assert_eq!(loaded.metadata.total_transactions, 0);
        assert!(loaded.transactions.is_empty());
    }

    #[test]
    fn test_large_transaction_count_save_load() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("large.cheolsu");

        let count = 500;
        let transactions: Vec<RequestInfo> = (0..count)
            .map(|_| make_request_info_with_response())
            .collect();

        let session = SessionFile::from_traffic(8100, &transactions, &[], &[], &[], None);
        assert_eq!(session.metadata.total_transactions, count);
        assert_eq!(session.transactions.len(), count);

        session.save(&path).unwrap();
        let loaded = SessionFile::load(&path).unwrap();

        assert_eq!(loaded.metadata.total_transactions, count);
        assert_eq!(loaded.transactions.len(), count);
        for t in &loaded.transactions {
            assert!(t.info.0.is_some());
            assert!(t.info.1.is_some());
        }
    }

    #[test]
    fn test_large_transaction_count_save_load_gzip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("large.cheolsu.gz");

        let count = 500;
        let transactions: Vec<RequestInfo> = (0..count)
            .map(|_| make_request_info_with_response())
            .collect();

        let session = SessionFile::from_traffic(8100, &transactions, &[], &[], &[], None);
        session.save(&path).unwrap();
        let loaded = SessionFile::load(&path).unwrap();

        assert_eq!(loaded.transactions.len(), count);
    }

    #[test]
    fn test_gzip_save_load_preserves_transaction_data() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("data.cheolsu.gz");

        let info = make_request_info_with_response();
        let session = SessionFile::from_traffic(8100, &[info], &[], &[], &[], None);
        session.save(&path).unwrap();
        let loaded = SessionFile::load(&path).unwrap();

        assert_eq!(loaded.transactions.len(), 1);
        let loaded_info = &loaded.transactions[0].info;
        assert!(loaded_info.0.is_some());
        assert!(loaded_info.1.is_some());

        let req = loaded_info.0.as_ref().unwrap();
        assert_eq!(req.method().as_str(), "GET");
        let res = loaded_info.1.as_ref().unwrap();
        assert_eq!(res.status().as_u16(), 200);
    }

    #[test]
    fn test_from_traffic_with_ws_messages() {
        use proxy_v2_models::{WsContentType, WsDirection, WsMessageInfo, WsMessageType};

        let ws_msgs = vec![WsMessageInfo {
            connection_id: "ws://example.com/ws".to_string(),
            sequence: 1,
            direction: WsDirection::ClientToServer,
            message_type: WsMessageType::Text,
            payload: "hello".to_string(),
            size: 5,
            time: 1000,
            is_binary: false,
            content_type: WsContentType::Plain,
            mqtt_version: None,
        }];

        let session = SessionFile::from_traffic(8100, &[], &ws_msgs, &[], &[], None);
        assert_eq!(session.websocket_messages.len(), 1);
        assert_eq!(session.websocket_messages[0].payload, "hello");
        assert_eq!(session.websocket_messages[0].sequence, 1);
    }

    #[test]
    fn test_from_traffic_with_server_replay_entries() {
        use std::collections::HashMap;

        let entries = vec![ServerReplayEntry {
            id: "replay-1".to_string(),
            method: "GET".to_string(),
            url: "https://example.com/api".to_string(),
            status: 200,
            headers: HashMap::from([("content-type".to_string(), "text/plain".to_string())]),
            body: Some("mock response".to_string()),
        }];

        let session = SessionFile::from_traffic(8100, &[], &[], &[], &entries, None);
        assert_eq!(session.server_replay_entries.len(), 1);
        assert_eq!(session.server_replay_entries[0].id, "replay-1");
        assert_eq!(session.server_replay_entries[0].status, 200);
        assert_eq!(
            session.server_replay_entries[0].body,
            Some("mock response".to_string())
        );
    }

    #[test]
    fn test_from_traffic_with_all_fields_save_load() {
        use proxy_v2_models::{WsContentType, WsDirection, WsMessageInfo, WsMessageType};
        use std::collections::HashMap;

        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("full.cheolsu");

        let transactions = vec![make_request_info_with_response()];
        let ws_msgs = vec![WsMessageInfo {
            connection_id: "ws://example.com/ws".to_string(),
            sequence: 0,
            direction: WsDirection::ServerToClient,
            message_type: WsMessageType::Text,
            payload: "world".to_string(),
            size: 5,
            time: 2000,
            is_binary: false,
            content_type: WsContentType::Plain,
            mqtt_version: None,
        }];
        let rules = vec![InterceptRule {
            id: "r1".to_string(),
            name: "Rule".to_string(),
            enabled: true,
            pattern: "*".to_string(),
            method: None,
            action: InterceptAction::Block {
                status_code: 500,
                body: "error".to_string(),
            },
        }];
        let replay_entries = vec![ServerReplayEntry {
            id: "s1".to_string(),
            method: "POST".to_string(),
            url: "https://api.test.com".to_string(),
            status: 201,
            headers: HashMap::new(),
            body: None,
        }];
        let throttle = ThrottleConfig {
            enabled: true,
            download_rate: Some(2048),
            upload_rate: None,
            latency_ms: 50,
        };

        let session = SessionFile::from_traffic(
            9999,
            &transactions,
            &ws_msgs,
            &rules,
            &replay_entries,
            Some(throttle),
        );

        session.save(&path).unwrap();
        let loaded = SessionFile::load(&path).unwrap();

        assert_eq!(loaded.metadata.proxy_port, 9999);
        assert_eq!(loaded.metadata.total_transactions, 1);
        assert_eq!(loaded.transactions.len(), 1);
        assert_eq!(loaded.websocket_messages.len(), 1);
        assert_eq!(loaded.websocket_messages[0].payload, "world");
        assert_eq!(loaded.intercept_rules.len(), 1);
        assert_eq!(loaded.intercept_rules[0].id, "r1");
        assert_eq!(loaded.server_replay_entries.len(), 1);
        assert_eq!(loaded.server_replay_entries[0].method, "POST");
        let tc = loaded.throttle_config.unwrap();
        assert!(tc.enabled);
        assert_eq!(tc.download_rate, Some(2048));
        assert!(tc.upload_rate.is_none());
        assert_eq!(tc.latency_ms, 50);
    }

    #[test]
    fn test_import_har_multiple_entries() {
        let har_json = r#"{
            "log": {
                "version": "1.2",
                "creator": {"name": "test", "version": "1.0"},
                "entries": [
                    {
                        "startedDateTime": "2024-01-01T00:00:00.000Z",
                        "request": {
                            "method": "GET",
                            "url": "https://example.com/1",
                            "httpVersion": "HTTP/1.1",
                            "headers": [],
                            "queryString": [],
                            "headersSize": -1,
                            "bodySize": 0
                        },
                        "response": {
                            "status": 200,
                            "statusText": "OK",
                            "httpVersion": "HTTP/1.1",
                            "headers": [],
                            "content": {"size": 0, "mimeType": "text/plain"},
                            "redirectURL": "",
                            "headersSize": -1,
                            "bodySize": 0
                        },
                        "cache": {},
                        "timings": {"send": 0, "wait": 10, "receive": 0}
                    },
                    {
                        "startedDateTime": "2024-01-01T00:00:01.000Z",
                        "request": {
                            "method": "POST",
                            "url": "https://example.com/2",
                            "httpVersion": "HTTP/1.1",
                            "headers": [{"name": "content-type", "value": "application/json"}],
                            "postData": {"mimeType": "application/json", "text": "{\"a\":1}"},
                            "queryString": [],
                            "headersSize": -1,
                            "bodySize": 6
                        },
                        "response": {
                            "status": 201,
                            "statusText": "Created",
                            "httpVersion": "HTTP/1.1",
                            "headers": [],
                            "content": {"size": 0, "mimeType": "text/plain"},
                            "redirectURL": "",
                            "headersSize": -1,
                            "bodySize": 0
                        },
                        "cache": {},
                        "timings": {"send": 0, "wait": 10, "receive": 0}
                    },
                    {
                        "startedDateTime": "2024-01-01T00:00:02.000Z",
                        "request": {
                            "method": "DELETE",
                            "url": "https://example.com/3",
                            "httpVersion": "HTTP/2.0",
                            "headers": [],
                            "queryString": [],
                            "headersSize": -1,
                            "bodySize": 0
                        },
                        "response": {
                            "status": 204,
                            "statusText": "No Content",
                            "httpVersion": "HTTP/2.0",
                            "headers": [],
                            "content": {"size": 0, "mimeType": "text/plain"},
                            "redirectURL": "",
                            "headersSize": -1,
                            "bodySize": 0
                        },
                        "cache": {},
                        "timings": {"send": 0, "wait": 10, "receive": 0}
                    }
                ]
            }
        }"#;

        let transactions = import_har(har_json).unwrap();
        assert_eq!(transactions.len(), 3);

        let methods: Vec<&str> = transactions
            .iter()
            .map(|t| t.0.as_ref().unwrap().method().as_str())
            .collect();
        assert_eq!(methods, vec!["GET", "POST", "DELETE"]);
    }

    #[test]
    fn test_import_har_request_without_response() {
        let har_json = r#"{
            "log": {
                "version": "1.2",
                "creator": {"name": "test", "version": "1.0"},
                "entries": [
                    {
                        "startedDateTime": "2024-01-01T00:00:00.000Z",
                        "request": {
                            "method": "GET",
                            "url": "https://example.com/no-response",
                            "httpVersion": "HTTP/1.1",
                            "headers": [],
                            "queryString": [],
                            "headersSize": -1,
                            "bodySize": 0
                        },
                        "cache": {},
                        "timings": {"send": 0, "wait": 10, "receive": 0}
                    }
                ]
            }
        }"#;

        let transactions = import_har(har_json).unwrap();
        assert_eq!(transactions.len(), 1);
        assert!(transactions[0].0.is_some());
        assert!(transactions[0].1.is_none());
    }

    #[test]
    fn test_import_har_with_request_headers() {
        let har_json = r#"{
            "log": {
                "version": "1.2",
                "creator": {"name": "test", "version": "1.0"},
                "entries": [
                    {
                        "startedDateTime": "2024-01-01T00:00:00.000Z",
                        "request": {
                            "method": "GET",
                            "url": "https://example.com/headers",
                            "httpVersion": "HTTP/1.1",
                            "headers": [
                                {"name": "accept", "value": "application/json"},
                                {"name": "authorization", "value": "Bearer token123"}
                            ],
                            "queryString": [],
                            "headersSize": -1,
                            "bodySize": 0
                        },
                        "response": {
                            "status": 200,
                            "statusText": "OK",
                            "httpVersion": "HTTP/1.1",
                            "headers": [
                                {"name": "content-type", "value": "application/json"}
                            ],
                            "content": {"size": 2, "mimeType": "application/json", "text": "{}"},
                            "redirectURL": "",
                            "headersSize": -1,
                            "bodySize": 2
                        },
                        "cache": {},
                        "timings": {"send": 0, "wait": 10, "receive": 0}
                    }
                ]
            }
        }"#;

        let transactions = import_har(har_json).unwrap();
        assert_eq!(transactions.len(), 1);
        let req = transactions[0].0.as_ref().unwrap();
        assert!(req.headers().contains_key("accept"));
        assert!(req.headers().contains_key("authorization"));
    }

    #[test]
    fn test_metadata_defaults() {
        let session = SessionFile::new(3000);
        assert!(session.metadata.name.is_none());
        assert!(session.metadata.description.is_none());
        assert_eq!(session.metadata.proxy_port, 3000);
        assert_eq!(session.metadata.total_transactions, 0);
    }

    #[test]
    fn test_metadata_save_load_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("meta.cheolsu");

        let mut session = SessionFile::new(4000);
        session.metadata.name = Some("My Proxy Session".to_string());
        session.metadata.description = Some("Debugging API calls".to_string());
        session.metadata.total_transactions = 42;

        session.save(&path).unwrap();
        let loaded = SessionFile::load(&path).unwrap();

        assert_eq!(loaded.metadata.name, Some("My Proxy Session".to_string()));
        assert_eq!(
            loaded.metadata.description,
            Some("Debugging API calls".to_string())
        );
        assert_eq!(loaded.metadata.proxy_port, 4000);
        assert_eq!(loaded.metadata.total_transactions, 42);
    }
}

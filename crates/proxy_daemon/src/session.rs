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
    pub request: RequestInfo,
    pub response: Option<RequestInfo>,
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
            .map(|info| SessionTransaction {
                request: info.clone(),
                response: None,
            })
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
            let file = std::fs::File::create(path)
                .map_err(|e| SessionError::Io(e.to_string()))?;
            let mut encoder = GzEncoder::new(file, Compression::default());
            encoder
                .write_all(json.as_bytes())
                .map_err(|e| SessionError::Io(e.to_string()))?;
            encoder
                .finish()
                .map_err(|e| SessionError::Io(e.to_string()))?;
        } else {
            std::fs::write(path, json.as_bytes())
                .map_err(|e| SessionError::Io(e.to_string()))?;
        }

        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self, SessionError> {
        let raw = std::fs::read(path)
            .map_err(|e| SessionError::Io(e.to_string()))?;

        let json_str = if is_gzip(&raw) {
            let mut decoder = GzDecoder::new(&raw[..]);
            let mut decompressed = String::new();
            decoder
                .read_to_string(&mut decompressed)
                .map_err(|e| SessionError::Decompress(e.to_string()))?;
            decompressed
        } else {
            String::from_utf8(raw)
                .map_err(|e| SessionError::Deserialize(e.to_string()))?
        };

        let session: SessionFile = serde_json::from_str(&json_str)
            .map_err(|e| SessionError::Deserialize(e.to_string()))?;

        Ok(session)
    }

    pub fn extract_transactions(&self) -> Vec<RequestInfo> {
        self.transactions
            .iter()
            .map(|st| st.request.clone())
            .collect()
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
    let content = std::fs::read_to_string(path)
        .map_err(|e| SessionError::Io(e.to_string()))?;
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
        method, uri, version, req_headers, req_body, timestamp,
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

        let res_body = res.content.and_then(|c| {
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
        }).unwrap_or_default();

        let proxied_res = proxy_v2_models::ProxiedResponse::new(
            status, res_version, res_headers, res_body, timestamp,
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
        assert!(!is_gzip(&[0x1f]));        // too short
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
        let har_json = r#"{"log":{"version":"1.2","creator":{"name":"t","version":"1"},"entries":[]}}"#;
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
}

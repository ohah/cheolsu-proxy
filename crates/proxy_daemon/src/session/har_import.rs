//! HAR(HTTP Archive) 파일 가져오기 기능

use proxy_v2_models::RequestInfo;
use serde::Deserialize;
use std::path::Path;

use super::SessionError;

/// HAR JSON 문자열에서 트랜잭션 목록을 가져옴
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

/// HAR 파일 경로에서 트랜잭션 목록을 가져옴
pub fn import_har_file(path: &Path) -> Result<Vec<RequestInfo>, SessionError> {
    let content = std::fs::read_to_string(path).map_err(|e| SessionError::Io(e.to_string()))?;
    import_har(&content)
}

// --- HAR import 구조체 (가져오기에 필요한 최소 구조) ---

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

/// HAR 엔트리를 RequestInfo로 변환
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

    Ok(RequestInfo(Some(client_req), client_res, None))
}

/// HTTP 버전 문자열을 hyper Version으로 파싱
pub(crate) fn parse_http_version(version: Option<&str>) -> proxyapi_v2::hyper::http::Version {
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

    #[test]
    fn test_parse_http_version_1_0() {
        use proxyapi_v2::hyper::http::Version;
        assert_eq!(parse_http_version(Some("HTTP/1.0")), Version::HTTP_10);
        assert_eq!(parse_http_version(Some("http/1.0")), Version::HTTP_10);
    }

    #[test]
    fn test_parse_http_version_1_1() {
        use proxyapi_v2::hyper::http::Version;
        assert_eq!(parse_http_version(Some("HTTP/1.1")), Version::HTTP_11);
    }

    #[test]
    fn test_parse_http_version_2() {
        use proxyapi_v2::hyper::http::Version;
        assert_eq!(parse_http_version(Some("HTTP/2.0")), Version::HTTP_2);
        assert_eq!(parse_http_version(Some("HTTP/2")), Version::HTTP_2);
        assert_eq!(parse_http_version(Some("h2")), Version::HTTP_2);
    }

    #[test]
    fn test_parse_http_version_default() {
        use proxyapi_v2::hyper::http::Version;
        assert_eq!(parse_http_version(None), Version::HTTP_11);
        assert_eq!(parse_http_version(Some("unknown")), Version::HTTP_11);
        assert_eq!(parse_http_version(Some("")), Version::HTTP_11);
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
        match result.unwrap_err() {
            SessionError::Deserialize(msg) => {
                assert!(msg.contains("Invalid HAR format"));
            }
            other => panic!("Deserialize 에러를 기대했지만 {:?}를 받음", other),
        }
    }

    #[test]
    fn test_import_har_empty_entries() {
        let har_json =
            r#"{"log":{"version":"1.2","creator":{"name":"t","version":"1"},"entries":[]}}"#;
        let transactions = import_har(har_json).unwrap();
        assert!(transactions.is_empty());
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
                    }
                ]
            }
        }"#;

        let transactions = import_har(har_json).unwrap();
        assert_eq!(transactions.len(), 2);

        let methods: Vec<&str> = transactions
            .iter()
            .map(|t| t.0.as_ref().unwrap().method().as_str())
            .collect();
        assert_eq!(methods, vec!["GET", "POST"]);
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
        let body = res.body();
        assert!(body.is_some());
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
    fn test_convert_har_entry_various_http_versions() {
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

        for version in &["HTTP/1.0", "HTTP/2", "h2", "HTTP/1.1"] {
            let txns = import_har(&make_har(version)).unwrap();
            assert_eq!(txns.len(), 1);
            assert!(txns[0].0.is_some());
        }
    }
}

use base64::Engine;
use serde::Serialize;

use crate::{ClientRequest, ClientResponse, DataType, RequestInfo};

#[derive(Serialize)]
pub struct Har {
    pub log: HarLog,
}

#[derive(Serialize)]
pub struct HarLog {
    pub version: String,
    pub creator: HarCreator,
    pub entries: Vec<HarEntry>,
}

#[derive(Serialize)]
pub struct HarCreator {
    pub name: String,
    pub version: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HarEntry {
    pub started_date_time: String,
    pub time: i64,
    pub request: HarRequest,
    pub response: HarResponse,
    pub cache: serde_json::Value,
    pub timings: HarTimings,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HarRequest {
    pub method: String,
    pub url: String,
    pub http_version: String,
    pub cookies: Vec<HarCookie>,
    pub headers: Vec<HarHeader>,
    pub query_string: Vec<HarQueryParam>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_data: Option<HarPostData>,
    pub headers_size: i64,
    pub body_size: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HarResponse {
    pub status: u16,
    pub status_text: String,
    pub http_version: String,
    pub cookies: Vec<HarCookie>,
    pub headers: Vec<HarHeader>,
    pub content: HarContent,
    #[serde(rename = "redirectURL")]
    pub redirect_url: String,
    pub headers_size: i64,
    pub body_size: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HarContent {
    pub size: i64,
    pub mime_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HarPostData {
    pub mime_type: String,
    pub text: String,
}

#[derive(Serialize)]
pub struct HarHeader {
    pub name: String,
    pub value: String,
}

#[derive(Serialize)]
pub struct HarCookie {
    pub name: String,
    pub value: String,
}

#[derive(Serialize)]
pub struct HarQueryParam {
    pub name: String,
    pub value: String,
}

#[derive(Serialize)]
pub struct HarTimings {
    pub send: i64,
    pub wait: i64,
    pub receive: i64,
}

fn headers_to_har(headers: &http::HeaderMap) -> Vec<HarHeader> {
    headers
        .iter()
        .map(|(name, value)| HarHeader {
            name: name.to_string(),
            value: value.to_str().unwrap_or("").to_string(),
        })
        .collect()
}

fn compute_headers_size(headers: &http::HeaderMap) -> i64 {
    let mut size: i64 = 0;
    for (name, value) in headers.iter() {
        // "name: value\r\n"
        size += name.as_str().len() as i64 + 2 + value.len() as i64 + 2;
    }
    if size > 0 {
        size + 2 // final \r\n
    } else {
        -1
    }
}

fn parse_cookies(headers: &http::HeaderMap, header_name: &str) -> Vec<HarCookie> {
    let Some(value) = headers.get(header_name) else {
        return Vec::new();
    };
    let Ok(cookie_str) = value.to_str() else {
        return Vec::new();
    };

    cookie_str
        .split(';')
        .filter_map(|pair| {
            let pair = pair.trim();
            let (name, val) = pair.split_once('=')?;
            Some(HarCookie {
                name: name.trim().to_string(),
                value: val.trim().to_string(),
            })
        })
        .collect()
}

fn parse_query_string(uri: &str) -> Vec<HarQueryParam> {
    let Some(query) = uri.split('?').nth(1) else {
        return Vec::new();
    };
    // fragment 제거
    let query = query.split('#').next().unwrap_or(query);

    query
        .split('&')
        .filter_map(|pair| {
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            if name.is_empty() {
                return None;
            }
            Some(HarQueryParam {
                name: name.to_string(),
                value: value.to_string(),
            })
        })
        .collect()
}

fn http_version_string(version: &http::Version) -> String {
    match *version {
        http::Version::HTTP_10 => "HTTP/1.0".to_string(),
        http::Version::HTTP_2 => "HTTP/2.0".to_string(),
        _ => "HTTP/1.1".to_string(),
    }
}

fn status_text(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "",
    }
}

fn data_type_mime(data_type: &DataType) -> &'static str {
    match data_type {
        DataType::Json | DataType::GraphQL => "application/json",
        DataType::Xml => "application/xml",
        DataType::Html => "text/html",
        DataType::Css => "text/css",
        DataType::Javascript => "application/javascript",
        DataType::Text => "text/plain",
        DataType::Image => "image/*",
        DataType::Video => "video/*",
        DataType::Audio => "audio/*",
        DataType::Document => "application/pdf",
        DataType::Archive => "application/zip",
        _ => "application/octet-stream",
    }
}

fn encode_body(body: &[u8]) -> (String, Option<String>) {
    match std::str::from_utf8(body) {
        Ok(text) => (text.to_string(), None),
        Err(_) => {
            let encoded = base64::engine::general_purpose::STANDARD.encode(body);
            (encoded, Some("base64".to_string()))
        }
    }
}

fn timestamp_to_iso(timestamp_ms: i64) -> String {
    let secs = timestamp_ms / 1000;
    let nanos = ((timestamp_ms % 1000) * 1_000_000) as u32;
    match chrono::DateTime::from_timestamp(secs, nanos) {
        Some(dt) => dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        None => chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    }
}

fn build_har_request(req: &ClientRequest) -> HarRequest {
    let headers = req.headers();
    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or(data_type_mime(req.data_type()));

    let post_data = req.body().and_then(|body| {
        if body.is_empty() {
            return None;
        }
        let (text, _) = encode_body(body);
        Some(HarPostData {
            mime_type: content_type.to_string(),
            text,
        })
    });

    HarRequest {
        method: req.method().to_string(),
        url: req.uri().to_string(),
        http_version: http_version_string(req.version()),
        cookies: parse_cookies(headers, "cookie"),
        headers: headers_to_har(headers),
        query_string: parse_query_string(&req.uri().to_string()),
        post_data,
        headers_size: compute_headers_size(headers),
        body_size: req.body_size() as i64,
    }
}

fn build_har_response(res: &ClientResponse) -> HarResponse {
    let headers = res.headers();
    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or(data_type_mime(res.data_type()));

    let (text, encoding) = if let Some(body) = res.body() {
        if body.is_empty() {
            (None, None)
        } else {
            let (t, e) = encode_body(body);
            (Some(t), e)
        }
    } else {
        (None, None)
    };

    let redirect_url = headers
        .get("location")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    HarResponse {
        status: res.status().as_u16(),
        status_text: status_text(res.status().as_u16()).to_string(),
        http_version: http_version_string(res.version()),
        cookies: parse_cookies(headers, "set-cookie"),
        headers: headers_to_har(headers),
        content: HarContent {
            size: res.body_size() as i64,
            mime_type: content_type.to_string(),
            text,
            encoding,
        },
        redirect_url,
        headers_size: compute_headers_size(headers),
        body_size: res.body_size() as i64,
    }
}

fn empty_har_response() -> HarResponse {
    HarResponse {
        status: 0,
        status_text: String::new(),
        http_version: "HTTP/1.1".to_string(),
        cookies: Vec::new(),
        headers: Vec::new(),
        content: HarContent {
            size: 0,
            mime_type: "x-unknown".to_string(),
            text: None,
            encoding: None,
        },
        redirect_url: String::new(),
        headers_size: -1,
        body_size: -1,
    }
}

/// `RequestInfo` 슬라이스를 HAR 1.2 JSON으로 변환
pub fn build_har(transactions: &[RequestInfo]) -> Har {
    let entries = transactions
        .iter()
        .filter_map(|info| {
            let req = info.0.as_ref()?;

            let har_request = build_har_request(req);
            let har_response = info
                .1
                .as_ref()
                .map(build_har_response)
                .unwrap_or_else(empty_har_response);

            let req_time = req.time();
            let res_time = info.1.as_ref().map(|r| r.time()).unwrap_or(req_time);
            let elapsed = (res_time - req_time).max(0);

            Some(HarEntry {
                started_date_time: timestamp_to_iso(req_time),
                time: elapsed,
                request: har_request,
                response: har_response,
                cache: serde_json::json!({}),
                timings: HarTimings {
                    send: 0,
                    wait: elapsed,
                    receive: 0,
                },
            })
        })
        .collect();

    Har {
        log: HarLog {
            version: "1.2".to_string(),
            creator: HarCreator {
                name: "Cheolsu Proxy".to_string(),
                version: "0.1.0".to_string(),
            },
            entries,
        },
    }
}

/// `RequestInfo` 슬라이스를 HAR JSON 문자열로 변환
pub fn build_har_json(transactions: &[RequestInfo]) -> Result<String, serde_json::Error> {
    let har = build_har(transactions);
    serde_json::to_string_pretty(&har)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use http::{HeaderMap, Method, StatusCode, Uri, Version};

    use crate::{ProxiedRequest, ProxiedResponse};

    fn make_request(method: &str, uri: &str, body: &[u8], time: i64) -> crate::ClientRequest {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());
        headers.insert("host", "example.com".parse().unwrap());

        let req = ProxiedRequest::new(
            method.parse::<Method>().unwrap(),
            uri.parse::<Uri>().unwrap(),
            Version::HTTP_11,
            headers,
            Bytes::from(body.to_vec()),
            time,
        );
        req.for_client(None)
    }

    fn make_response(status: u16, body: &[u8], time: i64, req_id: &str) -> crate::ClientResponse {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());

        let res = ProxiedResponse::new(
            StatusCode::from_u16(status).unwrap(),
            Version::HTTP_11,
            headers,
            Bytes::from(body.to_vec()),
            time,
        );
        res.for_client(req_id, None)
    }

    fn make_transaction(
        method: &str,
        uri: &str,
        req_body: &[u8],
        status: u16,
        res_body: &[u8],
        req_time: i64,
        res_time: i64,
    ) -> RequestInfo {
        let req = make_request(method, uri, req_body, req_time);
        let id = req.id().to_string();
        let res = make_response(status, res_body, res_time, &id);
        RequestInfo(Some(req), Some(res))
    }

    #[test]
    fn build_har_empty_transactions() {
        let har = build_har(&[]);
        assert_eq!(har.log.version, "1.2");
        assert_eq!(har.log.creator.name, "Cheolsu Proxy");
        assert!(har.log.entries.is_empty());
    }

    #[test]
    fn build_har_single_transaction() {
        let tx = make_transaction(
            "GET",
            "https://example.com/api?q=test&page=1",
            b"",
            200,
            b"{\"ok\":true}",
            1700000000000,
            1700000000100,
        );

        let har = build_har(&[tx]);
        assert_eq!(har.log.entries.len(), 1);

        let entry = &har.log.entries[0];
        assert_eq!(entry.request.method, "GET");
        assert_eq!(entry.request.url, "https://example.com/api?q=test&page=1");
        assert_eq!(entry.request.http_version, "HTTP/1.1");
        assert_eq!(entry.response.status, 200);
        assert_eq!(entry.response.status_text, "OK");
        assert_eq!(entry.time, 100);
        assert_eq!(entry.timings.wait, 100);
    }

    #[test]
    fn build_har_query_string_parsed() {
        let tx = make_transaction(
            "GET",
            "https://example.com/search?q=hello&lang=ko",
            b"",
            200,
            b"",
            1700000000000,
            1700000000050,
        );

        let har = build_har(&[tx]);
        let qs = &har.log.entries[0].request.query_string;
        assert_eq!(qs.len(), 2);
        assert_eq!(qs[0].name, "q");
        assert_eq!(qs[0].value, "hello");
        assert_eq!(qs[1].name, "lang");
        assert_eq!(qs[1].value, "ko");
    }

    #[test]
    fn build_har_post_data_included() {
        let body = b"{\"name\":\"test\"}";
        let tx = make_transaction(
            "POST",
            "https://example.com/api",
            body,
            201,
            b"",
            1700000000000,
            1700000000050,
        );

        let har = build_har(&[tx]);
        let post_data = har.log.entries[0].request.post_data.as_ref().unwrap();
        assert!(post_data.mime_type.contains("json"));
        assert_eq!(post_data.text, "{\"name\":\"test\"}");
    }

    #[test]
    fn build_har_binary_body_base64_encoded() {
        let binary_body: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0xFF, 0xFE, 0x00, 0x01];
        let tx = make_transaction(
            "GET",
            "https://example.com/image",
            b"",
            200,
            &binary_body,
            1700000000000,
            1700000000050,
        );

        let har = build_har(&[tx]);
        let content = &har.log.entries[0].response.content;
        assert_eq!(content.encoding.as_deref(), Some("base64"));
        assert!(content.text.is_some());
    }

    #[test]
    fn build_har_text_body_no_encoding() {
        let tx = make_transaction(
            "GET",
            "https://example.com/api",
            b"",
            200,
            b"{\"result\":42}",
            1700000000000,
            1700000000050,
        );

        let har = build_har(&[tx]);
        let content = &har.log.entries[0].response.content;
        assert!(content.encoding.is_none());
        assert_eq!(content.text.as_deref(), Some("{\"result\":42}"));
    }

    #[test]
    fn build_har_request_only_no_response() {
        let req = make_request("GET", "https://example.com/pending", b"", 1700000000000);
        let tx = RequestInfo(Some(req), None);

        let har = build_har(&[tx]);
        assert_eq!(har.log.entries.len(), 1);
        let entry = &har.log.entries[0];
        assert_eq!(entry.response.status, 0);
        assert_eq!(entry.response.body_size, -1);
        assert_eq!(entry.time, 0);
    }

    #[test]
    fn build_har_no_request_skipped() {
        let tx = RequestInfo(None, None);
        let har = build_har(&[tx]);
        assert!(har.log.entries.is_empty());
    }

    #[test]
    fn build_har_headers_serialized() {
        let tx = make_transaction(
            "GET",
            "https://example.com/",
            b"",
            200,
            b"ok",
            1700000000000,
            1700000000050,
        );

        let har = build_har(&[tx]);
        let req_headers = &har.log.entries[0].request.headers;
        assert!(req_headers.iter().any(|h| h.name == "content-type"));
        assert!(req_headers.iter().any(|h| h.name == "host"));
    }

    #[test]
    fn build_har_cookies_parsed() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "text/html".parse().unwrap());
        headers.insert("cookie", "sid=abc123; lang=ko".parse().unwrap());

        let req = ProxiedRequest::new(
            Method::GET,
            "https://example.com/".parse().unwrap(),
            Version::HTTP_11,
            headers,
            Bytes::new(),
            1700000000000,
        );
        let client_req = req.for_client(None);
        let tx = RequestInfo(Some(client_req), None);

        let har = build_har(&[tx]);
        let cookies = &har.log.entries[0].request.cookies;
        assert_eq!(cookies.len(), 2);
        assert_eq!(cookies[0].name, "sid");
        assert_eq!(cookies[0].value, "abc123");
        assert_eq!(cookies[1].name, "lang");
        assert_eq!(cookies[1].value, "ko");
    }

    #[test]
    fn build_har_redirect_url() {
        let mut res_headers = HeaderMap::new();
        res_headers.insert("content-type", "text/html".parse().unwrap());
        res_headers.insert("location", "https://example.com/new".parse().unwrap());

        let req = make_request("GET", "https://example.com/old", b"", 1700000000000);
        let id = req.id().to_string();
        let res = ProxiedResponse::new(
            StatusCode::MOVED_PERMANENTLY,
            Version::HTTP_11,
            res_headers,
            Bytes::new(),
            1700000000050,
        );
        let client_res = res.for_client(&id, None);
        let tx = RequestInfo(Some(req), Some(client_res));

        let har = build_har(&[tx]);
        assert_eq!(
            har.log.entries[0].response.redirect_url,
            "https://example.com/new"
        );
        assert_eq!(har.log.entries[0].response.status, 301);
    }

    #[test]
    fn build_har_multiple_transactions() {
        let txs = vec![
            make_transaction(
                "GET",
                "https://example.com/1",
                b"",
                200,
                b"ok",
                1700000000000,
                1700000000050,
            ),
            make_transaction(
                "POST",
                "https://example.com/2",
                b"data",
                201,
                b"created",
                1700000001000,
                1700000001200,
            ),
            make_transaction(
                "DELETE",
                "https://example.com/3",
                b"",
                404,
                b"",
                1700000002000,
                1700000002010,
            ),
        ];

        let har = build_har(&txs);
        assert_eq!(har.log.entries.len(), 3);
        assert_eq!(har.log.entries[0].request.method, "GET");
        assert_eq!(har.log.entries[1].request.method, "POST");
        assert_eq!(har.log.entries[2].request.method, "DELETE");
        assert_eq!(har.log.entries[2].response.status, 404);
    }

    #[test]
    fn build_har_json_valid() {
        let tx = make_transaction(
            "GET",
            "https://example.com/",
            b"",
            200,
            b"ok",
            1700000000000,
            1700000000050,
        );

        let json = build_har_json(&[tx]).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["log"]["version"], "1.2");
        assert_eq!(parsed["log"]["entries"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn build_har_started_date_time_iso_format() {
        let tx = make_transaction(
            "GET",
            "https://example.com/",
            b"",
            200,
            b"",
            1700000000000,
            1700000000000,
        );

        let har = build_har(&[tx]);
        let dt = &har.log.entries[0].started_date_time;
        // ISO 8601 형식 검증
        assert!(dt.contains("T"));
        assert!(dt.ends_with("Z") || dt.contains("+"));
    }

    // -- 내부 함수 테스트 --

    #[test]
    fn parse_query_string_empty() {
        assert!(parse_query_string("https://example.com/path").is_empty());
    }

    #[test]
    fn parse_query_string_with_fragment() {
        let qs = parse_query_string("https://example.com/path?a=1&b=2#section");
        assert_eq!(qs.len(), 2);
        assert_eq!(qs[1].value, "2");
    }

    #[test]
    fn parse_query_string_no_value() {
        let qs = parse_query_string("https://example.com/?flag");
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].name, "flag");
        assert_eq!(qs[0].value, "");
    }

    #[test]
    fn encode_body_text() {
        let (text, encoding) = encode_body(b"hello world");
        assert_eq!(text, "hello world");
        assert!(encoding.is_none());
    }

    #[test]
    fn encode_body_binary() {
        let (text, encoding) = encode_body(&[0xFF, 0xFE, 0x00]);
        assert_eq!(encoding.as_deref(), Some("base64"));
        // base64 디코딩 검증
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&text)
            .unwrap();
        assert_eq!(decoded, vec![0xFF, 0xFE, 0x00]);
    }

    #[test]
    fn status_text_known_codes() {
        assert_eq!(status_text(200), "OK");
        assert_eq!(status_text(404), "Not Found");
        assert_eq!(status_text(500), "Internal Server Error");
    }

    #[test]
    fn status_text_unknown_code() {
        assert_eq!(status_text(418), "");
    }

    #[test]
    fn http_version_string_variants() {
        assert_eq!(http_version_string(&Version::HTTP_10), "HTTP/1.0");
        assert_eq!(http_version_string(&Version::HTTP_11), "HTTP/1.1");
        assert_eq!(http_version_string(&Version::HTTP_2), "HTTP/2.0");
    }

    #[test]
    fn compute_headers_size_empty() {
        let headers = HeaderMap::new();
        assert_eq!(compute_headers_size(&headers), -1);
    }

    #[test]
    fn compute_headers_size_with_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("host", "example.com".parse().unwrap());
        let size = compute_headers_size(&headers);
        // "host: example.com\r\n" = 4 + 2 + 11 + 2 = 19, + final \r\n = 21
        assert_eq!(size, 21);
    }

    #[test]
    fn parse_cookies_empty() {
        let headers = HeaderMap::new();
        assert!(parse_cookies(&headers, "cookie").is_empty());
    }

    #[test]
    fn parse_cookies_multiple() {
        let mut headers = HeaderMap::new();
        headers.insert("cookie", "a=1; b=2; c=3".parse().unwrap());
        let cookies = parse_cookies(&headers, "cookie");
        assert_eq!(cookies.len(), 3);
        assert_eq!(cookies[0].name, "a");
        assert_eq!(cookies[2].value, "3");
    }
}

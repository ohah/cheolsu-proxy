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

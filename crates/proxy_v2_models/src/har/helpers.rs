use base64::Engine;

use super::types::{HarCookie, HarHeader, HarQueryParam};
use crate::DataType;

pub(super) fn headers_to_har(headers: &http::HeaderMap) -> Vec<HarHeader> {
    headers
        .iter()
        .map(|(name, value)| HarHeader {
            name: name.to_string(),
            value: value.to_str().unwrap_or("").to_string(),
        })
        .collect()
}

pub(super) fn compute_headers_size(headers: &http::HeaderMap) -> i64 {
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

pub(super) fn parse_cookies(headers: &http::HeaderMap, header_name: &str) -> Vec<HarCookie> {
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

pub(super) fn parse_query_string(uri: &str) -> Vec<HarQueryParam> {
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

pub(super) fn http_version_string(version: &http::Version) -> String {
    match *version {
        http::Version::HTTP_10 => "HTTP/1.0".to_string(),
        http::Version::HTTP_2 => "HTTP/2.0".to_string(),
        _ => "HTTP/1.1".to_string(),
    }
}

pub(super) fn status_text(status: u16) -> &'static str {
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

pub(super) fn data_type_mime(data_type: &DataType) -> &'static str {
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

pub(super) fn encode_body(body: &[u8]) -> (String, Option<String>) {
    match std::str::from_utf8(body) {
        Ok(text) => (text.to_string(), None),
        Err(_) => {
            let encoded = base64::engine::general_purpose::STANDARD.encode(body);
            (encoded, Some("base64".to_string()))
        }
    }
}

pub(super) fn timestamp_to_iso(timestamp_ms: i64) -> String {
    let secs = timestamp_ms / 1000;
    let nanos = ((timestamp_ms % 1000) * 1_000_000) as u32;
    match chrono::DateTime::from_timestamp(secs, nanos) {
        Some(dt) => dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        None => chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    }
}

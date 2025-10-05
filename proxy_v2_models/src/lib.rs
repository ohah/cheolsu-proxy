use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Re-export commonly used types
pub use bytes::Bytes;
pub use http::{HeaderMap, Method, StatusCode, Uri, Version};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProxiedRequest {
    #[serde(with = "http_serde::method")]
    method: Method,
    #[serde(with = "http_serde::uri")]
    uri: Uri,
    #[serde(with = "http_serde::version")]
    version: Version,
    #[serde(with = "http_serde::header_map")]
    headers: HeaderMap,
    body: Bytes,
    time: i64,
    id: String,           // 고유 ID 추가
    content_type: String, // Content-Type 정보 추가
}

impl ProxiedRequest {
    pub fn new(
        method: Method,
        uri: Uri,
        version: Version,
        headers: HeaderMap,
        body: Bytes,
        time: i64,
    ) -> Self {
        // 고유 ID 생성: 타임스탬프 + 랜덤 문자열
        let id = format!(
            "{}-{}",
            time,
            uuid::Uuid::new_v4().to_string().replace('-', "")
        );

        let content_type = Self::detect_content_type(&headers, &body);

        Self {
            method,
            uri,
            version,
            headers,
            body,
            time,
            id,
            content_type,
        }
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn uri(&self) -> &Uri {
        &self.uri
    }

    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn body(&self) -> &Bytes {
        &self.body
    }

    pub fn time(&self) -> i64 {
        self.time
    }

    pub fn id(&self) -> &String {
        &self.id
    }

    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    /// Content-Type을 감지하는 함수
    fn detect_content_type(headers: &HeaderMap, body: &Bytes) -> String {
        // 1. Content-Type 헤더에서 먼저 확인
        if let Some(content_type_header) = headers.get("content-type") {
            if let Ok(content_type_str) = content_type_header.to_str() {
                return content_type_str.to_string();
            }
        }

        // 2. body 내용을 분석해서 타입 추론
        if body.is_empty() {
            return "empty".to_string();
        }

        // JSON 감지
        if let Ok(body_str) = std::str::from_utf8(body) {
            if body_str.trim().starts_with('{') || body_str.trim().starts_with('[') {
                if serde_json::from_str::<serde_json::Value>(body_str).is_ok() {
                    return "application/json".to_string();
                }
            }
        }

        // XML 감지
        if let Ok(body_str) = std::str::from_utf8(body) {
            if body_str.trim().starts_with('<') {
                return "application/xml".to_string();
            }
        }

        // HTML 감지
        if let Ok(body_str) = std::str::from_utf8(body) {
            if body_str.trim().to_lowercase().starts_with("<!doctype html")
                || body_str.trim().to_lowercase().starts_with("<html")
            {
                return "text/html".to_string();
            }
        }

        // 바이너리 데이터 감지 (이미지, 파일 등)
        if body.len() > 0 {
            // PNG 시그니처
            if body.len() >= 8 && &body[0..8] == b"\x89PNG\r\n\x1a\n" {
                return "image/png".to_string();
            }
            // JPEG 시그니처
            if body.len() >= 2 && &body[0..2] == b"\xff\xd8" {
                return "image/jpeg".to_string();
            }
            // GIF 시그니처
            if body.len() >= 6 && &body[0..6] == b"GIF87a" || &body[0..6] == b"GIF89a" {
                return "image/gif".to_string();
            }
            // PDF 시그니처
            if body.len() >= 4 && &body[0..4] == b"%PDF" {
                return "application/pdf".to_string();
            }
        }

        // 기본값: 바이너리 데이터로 추정
        "application/octet-stream".to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProxiedResponse {
    #[serde(with = "http_serde::status_code")]
    status: StatusCode,
    #[serde(with = "http_serde::version")]
    version: Version,
    #[serde(with = "http_serde::header_map")]
    headers: HeaderMap,
    body: Bytes,
    time: i64,
    content_type: String, // Content-Type 정보 추가
}

impl ProxiedResponse {
    pub fn new(
        status: StatusCode,
        version: Version,
        headers: HeaderMap,
        body: Bytes,
        time: i64,
    ) -> Self {
        let content_type = Self::detect_content_type(&headers, &body);
        Self {
            status,
            version,
            headers,
            body,
            time,
            content_type,
        }
    }

    pub fn status(&self) -> &StatusCode {
        &self.status
    }

    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn body(&self) -> &Bytes {
        &self.body
    }

    pub fn time(&self) -> i64 {
        self.time
    }

    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    /// Content-Type을 감지하는 함수
    fn detect_content_type(headers: &HeaderMap, body: &Bytes) -> String {
        // 1. Content-Type 헤더에서 먼저 확인
        if let Some(content_type_header) = headers.get("content-type") {
            if let Ok(content_type_str) = content_type_header.to_str() {
                return content_type_str.to_string();
            }
        }

        // 2. body 내용을 분석해서 타입 추론
        if body.is_empty() {
            return "empty".to_string();
        }

        // JSON 감지
        if let Ok(body_str) = std::str::from_utf8(body) {
            if body_str.trim().starts_with('{') || body_str.trim().starts_with('[') {
                if serde_json::from_str::<serde_json::Value>(body_str).is_ok() {
                    return "application/json".to_string();
                }
            }
        }

        // XML 감지
        if let Ok(body_str) = std::str::from_utf8(body) {
            if body_str.trim().starts_with('<') {
                return "application/xml".to_string();
            }
        }

        // HTML 감지
        if let Ok(body_str) = std::str::from_utf8(body) {
            if body_str.trim().to_lowercase().starts_with("<!doctype html")
                || body_str.trim().to_lowercase().starts_with("<html")
            {
                return "text/html".to_string();
            }
        }

        // 바이너리 데이터 감지 (이미지, 파일 등)
        if body.len() > 0 {
            // PNG 시그니처
            if body.len() >= 8 && &body[0..8] == b"\x89PNG\r\n\x1a\n" {
                return "image/png".to_string();
            }
            // JPEG 시그니처
            if body.len() >= 2 && &body[0..2] == b"\xff\xd8" {
                return "image/jpeg".to_string();
            }
            // GIF 시그니처
            if body.len() >= 6 && &body[0..6] == b"GIF87a" || &body[0..6] == b"GIF89a" {
                return "image/gif".to_string();
            }
            // PDF 시그니처
            if body.len() >= 4 && &body[0..4] == b"%PDF" {
                return "application/pdf".to_string();
            }
        }

        // 기본값: 바이너리 데이터로 추정
        "application/octet-stream".to_string()
    }
}

trait ToString {
    fn to_string(&self) -> String;
}

trait ToHashString {
    fn to_hash_string(&self) -> HashMap<String, String>;
}

impl ToHashString for HeaderMap {
    fn to_hash_string(&self) -> HashMap<String, String> {
        let mut headers: HashMap<String, String> = HashMap::new();

        for (k, v) in self.iter() {
            headers
                .insert(k.as_str().to_string(), v.to_str().unwrap().to_string())
                .unwrap_or("NO header".to_string());
        }
        headers
    }
}

impl ToString for Version {
    fn to_string(&self) -> String {
        match *self {
            Version::HTTP_09 => "HTTP_09".to_string(),
            Version::HTTP_10 => "HTTP_10".to_string(),
            Version::HTTP_11 => "HTTP_11".to_string(),
            Version::HTTP_2 => "HTTP_2".to_string(),
            Version::HTTP_3 => "HTTP_3".to_string(),
            _ => "__NonExhaustive".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RequestInfo(pub Option<ProxiedRequest>, pub Option<ProxiedResponse>);

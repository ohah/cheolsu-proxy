use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Re-export commonly used types
pub use bytes::Bytes;
pub use http::{HeaderMap, Method, StatusCode, Uri, Version};

// Re-export data type module
pub mod data_type;
pub use data_type::{decompress_brotli, decompress_gzip, detect_data_type, DataType};

/// 압축된 body를 해제하는 헬퍼 함수
fn decompress_body_if_needed(headers: &HeaderMap, body: &Bytes) -> Vec<u8> {
    // Content-Encoding 헤더 확인
    if let Some(content_encoding) = headers.get("content-encoding") {
        if let Ok(encoding) = content_encoding.to_str() {
            let encoding_lower = encoding.to_lowercase();
            // Brotli 압축 해제
            if encoding_lower.contains("br") {
                if let Ok(decompressed) = decompress_brotli(body) {
                    return decompressed;
                }
            }
            // GZIP 압축 해제
            if encoding_lower.contains("gzip") {
                if let Ok(decompressed) = decompress_gzip(body) {
                    return decompressed;
                }
            }
        }
    }

    // GZIP magic number로 확인 (헤더가 없는 경우)
    if body.len() >= 2 && body[0] == 0x1f && body[1] == 0x8b {
        if let Ok(decompressed) = decompress_gzip(body) {
            return decompressed;
        }
    }

    // 압축되지 않은 경우 원본 반환
    body.to_vec()
}

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
    id: String,                           // 고유 ID 추가
    data_type: DataType,                  // 데이터 타입 정보 추가
    body_json: Option<serde_json::Value>, // JSON 파싱된 데이터 (JSON 타입인 경우)
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

        let data_type = detect_data_type(&headers, &body);

        // JSON 타입인 경우 파싱 시도
        let body_json = if data_type == data_type::DataType::Json {
            // 압축 해제 (필요한 경우)
            let body_to_parse = decompress_body_if_needed(&headers, &body);

            if let Ok(body_str) = std::str::from_utf8(&body_to_parse) {
                serde_json::from_str(body_str).ok()
            } else {
                None
            }
        } else {
            None
        };

        Self {
            method,
            uri,
            version,
            headers,
            body,
            time,
            id,
            data_type,
            body_json,
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

    pub fn data_type(&self) -> &DataType {
        &self.data_type
    }

    /// MIME 타입 문자열 반환
    pub fn mime_type(&self) -> &'static str {
        self.data_type.to_mime_type()
    }

    /// Monaco Editor 언어 모드 반환
    pub fn monaco_language(&self) -> &'static str {
        self.data_type.to_monaco_language()
    }

    /// JSON 파싱된 데이터 반환 (JSON 타입인 경우)
    pub fn body_json(&self) -> &Option<serde_json::Value> {
        &self.body_json
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
    data_type: DataType,                  // 데이터 타입 정보 추가
    body_json: Option<serde_json::Value>, // JSON 파싱된 데이터 (JSON 타입인 경우)
}

impl ProxiedResponse {
    pub fn new(
        status: StatusCode,
        version: Version,
        headers: HeaderMap,
        body: Bytes,
        time: i64,
    ) -> Self {
        let data_type = detect_data_type(&headers, &body);

        // JSON 타입인 경우 파싱 시도
        let body_json = if data_type == data_type::DataType::Json {
            // 압축 해제 (필요한 경우)
            let body_to_parse = decompress_body_if_needed(&headers, &body);

            if let Ok(body_str) = std::str::from_utf8(&body_to_parse) {
                serde_json::from_str(body_str).ok()
            } else {
                None
            }
        } else {
            None
        };

        Self {
            status,
            version,
            headers,
            body,
            time,
            data_type,
            body_json,
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

    pub fn data_type(&self) -> &DataType {
        &self.data_type
    }

    /// MIME 타입 문자열 반환
    pub fn mime_type(&self) -> &'static str {
        self.data_type.to_mime_type()
    }

    /// Monaco Editor 언어 모드 반환
    pub fn monaco_language(&self) -> &'static str {
        self.data_type.to_monaco_language()
    }

    /// JSON 파싱된 데이터 반환 (JSON 타입인 경우)
    pub fn body_json(&self) -> &Option<serde_json::Value> {
        &self.body_json
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

use bytes::Bytes;
use http::{HeaderMap, Method, Uri, Version};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::data_type::{detect_data_type, DataType};
use crate::file_storage::{decompress_body_if_needed, save_body_to_file};
use crate::mime_utils::is_media_data_type;
use crate::BODY_FILE_THRESHOLD;

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
    id: String, // 고유 ID 추가
    // 내부 처리용 필드들 (직렬화되지 않음)
    #[serde(skip)]
    data_type: DataType,
    #[serde(skip)]
    body_json: Option<serde_json::Value>,
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

        // JSON 타입인 경우 파싱 시도 (GraphQL도 JSON 기반)
        let body_json = if data_type == DataType::Json || data_type == DataType::GraphQL {
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

    /// 클라이언트(타우리 UI)용으로 변환
    pub fn for_client(self, cache_dir: Option<&Path>) -> ClientRequest {
        let original_body_size = self.body.len();
        #[allow(clippy::absurd_extreme_comparisons)]
        let (body, file_path) =
            if original_body_size >= BODY_FILE_THRESHOLD || is_media_data_type(&self.data_type) {
                // 큰 body이거나 미디어 파일은 파일로 저장
                if let Some(cache_dir) = cache_dir {
                    match save_body_to_file(
                        &self.id,
                        &self.body,
                        cache_dir,
                        "request",
                        &self.data_type,
                        &self.headers,
                    ) {
                        Ok(path) => {
                            (None, Some(path)) // 파일로 저장했으므로 body는 None
                        }
                        Err(_e) => (Some(self.body), None),
                    }
                } else {
                    (Some(self.body), None)
                }
            } else {
                // 작은 body는 메모리에 유지
                (Some(self.body), None)
            };

        ClientRequest {
            method: self.method,
            uri: self.uri,
            version: self.version,
            headers: self.headers,
            body,
            time: self.time,
            id: self.id,
            data_type: self.data_type,
            body_json: self.body_json,
            file_path,
            body_size: original_body_size, // 원본 크기 유지
        }
    }
}

/// 클라이언트(타우리 UI)용 요청 구조체
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClientRequest {
    #[serde(with = "http_serde::method")]
    method: Method,
    #[serde(with = "http_serde::uri")]
    uri: Uri,
    #[serde(with = "http_serde::version")]
    version: Version,
    #[serde(with = "http_serde::header_map")]
    headers: HeaderMap,
    body: Option<Bytes>, // 파일로 저장된 경우 None
    time: i64,
    id: String,
    data_type: DataType,
    body_json: Option<serde_json::Value>,
    file_path: Option<String>, // body가 저장된 파일 경로
    body_size: usize,          // 실제 body 크기 (파일 저장 시에도 원본 크기 유지)
}

impl ClientRequest {
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

    pub fn body(&self) -> Option<&Bytes> {
        self.body.as_ref()
    }

    pub fn time(&self) -> i64 {
        self.time
    }

    pub fn id(&self) -> &str {
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

    /// body가 저장된 파일 경로 반환
    pub fn file_path(&self) -> &Option<String> {
        &self.file_path
    }

    /// 실제 body 크기 반환
    pub fn body_size(&self) -> usize {
        self.body_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use http::{HeaderMap, HeaderValue, Method, Uri, Version};

    fn make_request(method: Method, uri: &str, body: &[u8]) -> ProxiedRequest {
        ProxiedRequest::new(
            method,
            uri.parse::<Uri>().unwrap(),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::from(body.to_vec()),
            1000,
        )
    }

    #[test]
    fn test_new_generates_unique_id() {
        let r1 = make_request(Method::GET, "http://example.com", b"");
        let r2 = make_request(Method::GET, "http://example.com", b"");
        assert_ne!(r1.id(), r2.id());
        assert!(r1.id().starts_with("1000-"));
    }

    #[test]
    fn test_accessor_methods() {
        let req = make_request(Method::POST, "http://example.com/api", b"hello");
        assert_eq!(req.method(), Method::POST);
        assert_eq!(req.uri().to_string(), "http://example.com/api");
        assert_eq!(req.version(), &Version::HTTP_11);
        assert_eq!(req.body().as_ref(), b"hello");
        assert_eq!(req.time(), 1000);
    }

    #[test]
    fn test_json_body_parsing() {
        let json_body = br#"{"key":"value"}"#;
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        let req = ProxiedRequest::new(
            Method::POST,
            "http://example.com".parse().unwrap(),
            Version::HTTP_11,
            headers,
            Bytes::from(json_body.to_vec()),
            1000,
        );
        assert!(req.body_json().is_some());
        let json = req.body_json().as_ref().unwrap();
        assert_eq!(json["key"], "value");
    }

    #[test]
    fn test_non_json_body_has_no_json() {
        let req = make_request(Method::GET, "http://example.com", b"plain text");
        assert!(req.body_json().is_none());
    }

    #[test]
    fn test_data_type_detection() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        let req = ProxiedRequest::new(
            Method::POST,
            "http://example.com".parse().unwrap(),
            Version::HTTP_11,
            headers,
            Bytes::from(br#"{"a":1}"#.to_vec()),
            1000,
        );
        assert_eq!(*req.data_type(), DataType::Json);
        assert_eq!(req.mime_type(), "application/json");
        assert_eq!(req.monaco_language(), "json");
    }

    #[test]
    fn test_for_client_preserves_fields() {
        let req = make_request(Method::GET, "http://example.com", b"body");
        let id = req.id().clone();
        let client = req.for_client(None);
        assert_eq!(client.method(), Method::GET);
        assert_eq!(client.uri().host(), Some("example.com"));
        assert_eq!(client.id(), id);
        assert_eq!(client.body_size(), 4);
        assert_eq!(client.body().unwrap().as_ref(), b"body");
        assert!(client.file_path().is_none());
    }

    #[test]
    fn test_for_client_with_cache_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let req = make_request(Method::GET, "http://example.com", b"data");
        let client = req.for_client(Some(tmp.path()));
        // BODY_FILE_THRESHOLD=0 이므로 항상 파일로 저장
        assert!(client.file_path().is_some());
        assert!(client.body().is_none());
        assert_eq!(client.body_size(), 4);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let req = make_request(Method::GET, "http://example.com/test", b"hello");
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: ProxiedRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.method(), req.method());
        assert_eq!(deserialized.uri().to_string(), req.uri().to_string());
        assert_eq!(deserialized.body(), req.body());
        // data_type과 body_json은 serde(skip)이므로 기본값
    }
}

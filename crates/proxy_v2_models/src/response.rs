use bytes::Bytes;
use http::{HeaderMap, StatusCode, Version};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::data_type::{detect_data_type, DataType};
use crate::file_storage::{decompress_body_if_needed, save_body_to_file};
use crate::grpc::{
    extract_grpc_content_subtype, extract_grpc_status, parse_grpc_frames, GrpcMetadata,
};
use crate::mime_utils::is_media_data_type;
use crate::request::ClientRequest;
use crate::timing::TimingWaterfall;
use crate::BODY_FILE_THRESHOLD;

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
    // 내부 처리용 필드들 (직렬화되지 않음)
    #[serde(skip)]
    data_type: DataType,
    #[serde(skip)]
    body_json: Option<serde_json::Value>,
    #[serde(skip)]
    decompressed_body: Option<Bytes>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    grpc_metadata: Option<GrpcMetadata>,
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

        // 압축 해제된 데이터 생성 (타우리 UI용)
        let decompressed_body = if data_type == DataType::Json
            || data_type == DataType::GraphQL
            || headers
                .get("content-encoding")
                .map(|h| h.to_str().unwrap_or("").to_lowercase().contains("gzip"))
                .unwrap_or(false)
            || headers
                .get("content-encoding")
                .map(|h| h.to_str().unwrap_or("").to_lowercase().contains("br"))
                .unwrap_or(false)
        {
            let decompressed = decompress_body_if_needed(&headers, &body);
            if decompressed != body.to_vec() {
                Some(Bytes::from(decompressed))
            } else {
                None
            }
        } else {
            None
        };

        // gRPC 메타데이터 추출
        let grpc_metadata = if data_type == DataType::Grpc {
            let (status_code, status_message) = extract_grpc_status(&headers);
            let content_subtype = headers
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .and_then(extract_grpc_content_subtype);
            let frames = parse_grpc_frames(&body);
            let is_compressed = frames.iter().any(|f| f.compressed);
            let frame_count = frames.len();

            Some(GrpcMetadata {
                service: None, // 응답에는 URI 정보 없음
                method: None,  // 응답에는 URI 정보 없음
                status_code,
                status_message,
                content_subtype,
                streaming_type: Default::default(),
                frame_count,
                is_compressed,
            })
        } else {
            None
        };

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
            status,
            version,
            headers,
            body,
            time,
            data_type,
            body_json,
            decompressed_body,
            grpc_metadata,
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

    /// 압축 해제된 데이터 반환 (타우리 UI용)
    pub fn decompressed_body(&self) -> &Option<Bytes> {
        &self.decompressed_body
    }

    /// gRPC 메타데이터 반환
    pub fn grpc_metadata(&self) -> &Option<GrpcMetadata> {
        &self.grpc_metadata
    }

    /// 클라이언트(타우리 UI)용으로 변환
    pub fn for_client(self, request_id: &str, cache_dir: Option<&Path>) -> ClientResponse {
        self.for_client_with_timing(request_id, cache_dir, None)
    }

    /// 타이밍 정보와 함께 클라이언트용으로 변환
    pub fn for_client_with_timing(
        self,
        request_id: &str,
        cache_dir: Option<&Path>,
        timing: Option<TimingWaterfall>,
    ) -> ClientResponse {
        let body_to_save = self.decompressed_body.unwrap_or(self.body);
        let original_body_size = body_to_save.len();
        #[allow(clippy::absurd_extreme_comparisons)]
        let (body, file_path) =
            if original_body_size >= BODY_FILE_THRESHOLD || is_media_data_type(&self.data_type) {
                // 큰 body이거나 미디어 파일은 파일로 저장
                if let Some(cache_dir) = cache_dir {
                    match save_body_to_file(
                        request_id,
                        &body_to_save,
                        cache_dir,
                        "response",
                        &self.data_type,
                        &self.headers,
                    ) {
                        Ok(path) => {
                            (None, Some(path)) // 파일로 저장했으므로 body는 None
                        }
                        Err(_e) => (Some(body_to_save), None),
                    }
                } else {
                    (Some(body_to_save), None)
                }
            } else {
                // 작은 body는 메모리에 유지
                (Some(body_to_save), None)
            };

        ClientResponse {
            status: self.status,
            version: self.version,
            headers: self.headers,
            body,
            time: self.time,
            id: request_id.to_string(),
            data_type: self.data_type,
            body_json: self.body_json,
            grpc_metadata: self.grpc_metadata,
            file_path,
            body_size: original_body_size, // 원본 크기 유지
            timing,
        }
    }
}

/// 클라이언트(타우리 UI)용 응답 구조체
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClientResponse {
    #[serde(with = "http_serde::status_code")]
    status: StatusCode,
    #[serde(with = "http_serde::version")]
    version: Version,
    #[serde(with = "http_serde::header_map")]
    headers: HeaderMap,
    body: Option<Bytes>, // 파일로 저장된 경우 None
    time: i64,
    id: String, // ClientRequest의 id와 동일
    data_type: DataType,
    body_json: Option<serde_json::Value>,
    grpc_metadata: Option<GrpcMetadata>,
    file_path: Option<String>, // body가 저장된 파일 경로
    body_size: usize,          // 실제 body 크기 (파일 저장 시에도 원본 크기 유지)
    /// 요청/응답 각 단계별 타이밍 정보
    #[serde(skip_serializing_if = "Option::is_none", default)]
    timing: Option<TimingWaterfall>,
}

impl ClientResponse {
    pub fn status(&self) -> &StatusCode {
        &self.status
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

    /// gRPC 메타데이터 반환
    pub fn grpc_metadata(&self) -> &Option<GrpcMetadata> {
        &self.grpc_metadata
    }

    /// ID 반환
    pub fn id(&self) -> &str {
        &self.id
    }

    /// body가 저장된 파일 경로 반환
    pub fn file_path(&self) -> &Option<String> {
        &self.file_path
    }

    /// 실제 body 크기 반환
    pub fn body_size(&self) -> usize {
        self.body_size
    }

    /// 타이밍 정보 반환
    pub fn timing(&self) -> &Option<TimingWaterfall> {
        &self.timing
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RequestInfo(pub Option<ClientRequest>, pub Option<ClientResponse>);

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use http::{HeaderMap, HeaderValue, StatusCode, Version};

    fn make_response(status: u16, body: &[u8]) -> ProxiedResponse {
        ProxiedResponse::new(
            StatusCode::from_u16(status).unwrap(),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::from(body.to_vec()),
            2000,
        )
    }

    #[test]
    fn test_accessor_methods() {
        let res = make_response(200, b"OK");
        assert_eq!(res.status(), &StatusCode::OK);
        assert_eq!(res.version(), &Version::HTTP_11);
        assert_eq!(res.body().as_ref(), b"OK");
        assert_eq!(res.time(), 2000);
    }

    #[test]
    fn test_json_body_parsing() {
        let json_body = br#"{"result":"ok"}"#;
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        let res = ProxiedResponse::new(
            StatusCode::OK,
            Version::HTTP_11,
            headers,
            Bytes::from(json_body.to_vec()),
            2000,
        );
        assert!(res.body_json().is_some());
        assert_eq!(res.body_json().as_ref().unwrap()["result"], "ok");
    }

    #[test]
    fn test_non_json_body_has_no_json() {
        let res = make_response(200, b"plain text");
        assert!(res.body_json().is_none());
    }

    #[test]
    fn test_no_decompression_for_plain_body() {
        let res = make_response(200, b"plain");
        assert!(res.decompressed_body().is_none());
    }

    #[test]
    fn test_gzip_decompression() {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let original = br#"{"compressed":true}"#;
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(original).unwrap();
        let compressed = encoder.finish().unwrap();

        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        headers.insert("content-encoding", HeaderValue::from_static("gzip"));

        let res = ProxiedResponse::new(
            StatusCode::OK,
            Version::HTTP_11,
            headers,
            Bytes::from(compressed),
            2000,
        );
        assert!(res.decompressed_body().is_some());
        let decompressed = res.decompressed_body().as_ref().unwrap();
        assert_eq!(decompressed.as_ref(), original);
        assert!(res.body_json().is_some());
        assert_eq!(res.body_json().as_ref().unwrap()["compressed"], true);
    }

    #[test]
    fn test_data_type_detection() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("text/html"));
        let res = ProxiedResponse::new(
            StatusCode::OK,
            Version::HTTP_11,
            headers,
            Bytes::from("<html></html>"),
            2000,
        );
        assert_eq!(*res.data_type(), DataType::Html);
        assert_eq!(res.mime_type(), "text/html");
        assert_eq!(res.monaco_language(), "html");
    }

    #[test]
    fn test_for_client_preserves_fields() {
        let res = make_response(404, b"Not Found");
        let client = res.for_client("req-123", None);
        assert_eq!(client.status(), &StatusCode::NOT_FOUND);
        assert_eq!(client.id(), "req-123");
        assert_eq!(client.body_size(), 9);
        assert_eq!(client.body().unwrap().as_ref(), b"Not Found");
        assert!(client.file_path().is_none());
    }

    #[test]
    fn test_for_client_with_cache_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let res = make_response(200, b"response body");
        let client = res.for_client("req-456", Some(tmp.path()));
        // BODY_FILE_THRESHOLD=0 이므로 파일로 저장
        assert!(client.file_path().is_some());
        assert!(client.body().is_none());
        assert_eq!(client.body_size(), 13);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let res = make_response(200, b"body");
        let json = serde_json::to_string(&res).unwrap();
        let deserialized: ProxiedResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.status(), res.status());
        assert_eq!(deserialized.body(), res.body());
    }

    #[test]
    fn test_request_info_structure() {
        let info = RequestInfo(None, None);
        assert!(info.0.is_none());
        assert!(info.1.is_none());

        let json = serde_json::to_string(&info).unwrap();
        let parsed: RequestInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, info);
    }
}

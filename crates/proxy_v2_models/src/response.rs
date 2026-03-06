use bytes::Bytes;
use http::{HeaderMap, StatusCode, Version};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::data_type::{detect_data_type, DataType};
use crate::file_storage::{decompress_body_if_needed, save_body_to_file};
use crate::mime_utils::is_media_data_type;
use crate::request::ClientRequest;
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

        // JSON 타입인 경우 파싱 시도
        let body_json = if data_type == DataType::Json {
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

    /// 클라이언트(타우리 UI)용으로 변환
    pub fn for_client(self, request_id: &str, cache_dir: Option<&Path>) -> ClientResponse {
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
            file_path,
            body_size: original_body_size, // 원본 크기 유지
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
    file_path: Option<String>, // body가 저장된 파일 경로
    body_size: usize,          // 실제 body 크기 (파일 저장 시에도 원본 크기 유지)
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
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RequestInfo(pub Option<ClientRequest>, pub Option<ClientResponse>);

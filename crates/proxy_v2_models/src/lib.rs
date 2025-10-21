use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

// Re-export commonly used types
pub use bytes::Bytes;
pub use http::{HeaderMap, Method, StatusCode, Uri, Version};

// Re-export data type module
pub mod data_type;
pub use data_type::{decompress_brotli, decompress_gzip, detect_data_type, DataType};

/// Body 크기 임계값 (테스트용: 0 초과)
/// 이 크기 이상의 body는 파일시스템에 저장됩니다
pub const BODY_FILE_THRESHOLD: usize = 0; // 테스트용: 모든 body를 파일로 저장, 1_048_576 1MB

/// 미디어 파일 타입인지 확인하는 함수
/// 이미지, 비디오, 오디오는 무조건 파일로 저장
pub fn is_media_data_type(data_type: &DataType) -> bool {
    matches!(
        data_type,
        DataType::Image | DataType::Video | DataType::Audio
    )
}

/// 파일 헤더를 읽어서 파일 확장자를 감지하는 함수
pub fn detect_file_extension_from_header(body: &Bytes) -> Option<&'static str> {
    if body.len() < 4 {
        return None;
    }

    let header = &body[0..4.min(body.len())];

    match header {
        // 이미지 파일 시그니처
        [0xFF, 0xD8, 0xFF, _] => Some("jpg"),    // JPEG
        [0x89, 0x50, 0x4E, 0x47] => Some("png"), // PNG
        [0x47, 0x49, 0x46, 0x38] => Some("gif"), // GIF
        [0x52, 0x49, 0x46, 0x46] if body.len() >= 12 && &body[8..12] == b"WEBP" => Some("webp"), // WebP
        [0x42, 0x4D, _, _] => Some("bmp"), // BMP
        [0x49, 0x49, 0x2A, 0x00] | [0x4D, 0x4D, 0x00, 0x2A] => Some("tiff"), // TIFF
        [0x00, 0x00, 0x01, 0x00] => Some("ico"), // ICO

        // 비디오 파일 시그니처
        [0x00, 0x00, 0x00, 0x18] if body.len() >= 8 && &body[4..8] == b"ftyp" => Some("mp4"), // MP4
        [0x1A, 0x45, 0xDF, 0xA3] => Some("mkv"),                                              // MKV
        [0x52, 0x49, 0x46, 0x46] if body.len() >= 12 && &body[8..12] == b"AVI " => Some("avi"), // AVI

        // 오디오 파일 시그니처
        [0x49, 0x44, 0x33, _] | [0xFF, 0xFB, _, _] | [0xFF, 0xF3, _, _] | [0xFF, 0xF2, _, _] => {
            Some("mp3")
        } // MP3
        [0x52, 0x49, 0x46, 0x46] if body.len() >= 12 && &body[8..12] == b"WAVE" => Some("wav"), // WAV
        [0x4F, 0x67, 0x67, 0x53] => Some("ogg"), // OGG

        // 문서 파일 시그니처
        [0x25, 0x50, 0x44, 0x46] => Some("pdf"), // PDF

        // 압축 파일 시그니처
        [0x50, 0x4B, 0x03, 0x04] | [0x50, 0x4B, 0x05, 0x06] | [0x50, 0x4B, 0x07, 0x08] => {
            Some("zip")
        } // ZIP
        [0x52, 0x61, 0x72, 0x21] => Some("rar"), // RAR
        [0x37, 0x7A, 0xBC, 0xAF] => Some("7z"),  // 7Z

        _ => None,
    }
}

/// 파일 확장자에서 MIME 타입을 추출하는 함수
pub fn get_mime_type_from_extension(extension: &str) -> &'static str {
    match extension {
        // 이미지
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "tiff" => "image/tiff",
        "ico" => "image/x-icon",

        // 비디오
        "mp4" => "video/mp4",
        "avi" => "video/avi",
        "mov" => "video/quicktime",
        "wmv" => "video/x-ms-wmv",
        "flv" => "video/x-flv",
        "webm" => "video/webm",
        "mkv" => "video/x-matroska",
        "3gp" => "video/3gpp",

        // 오디오
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "aac" => "audio/aac",
        "flac" => "audio/flac",
        "m4a" => "audio/mp4",
        "wma" => "audio/x-ms-wma",

        // 문서
        "pdf" => "application/pdf",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",

        // 압축
        "zip" => "application/zip",
        "rar" => "application/x-rar-compressed",
        "7z" => "application/x-7z-compressed",
        "gz" => "application/gzip",
        "tar" => "application/x-tar",

        // 기타
        "txt" => "text/plain",
        "html" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "xml" => "application/xml",

        _ => "application/octet-stream",
    }
}

/// MIME 타입에서 파일 확장자를 추출하는 함수
pub fn get_extension_from_mime_type(mime_type: &str) -> &'static str {
    match mime_type {
        // 이미지
        "image/jpeg" | "image/jpg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/svg+xml" => "svg",
        "image/bmp" => "bmp",
        "image/tiff" => "tiff",
        "image/ico" | "image/x-icon" => "ico",

        // 비디오
        "video/mp4" => "mp4",
        "video/avi" => "avi",
        "video/mov" => "mov",
        "video/wmv" => "wmv",
        "video/flv" => "flv",
        "video/webm" => "webm",
        "video/mkv" => "mkv",
        "video/3gp" => "3gp",

        // 오디오
        "audio/mpeg" | "audio/mp3" => "mp3",
        "audio/wav" => "wav",
        "audio/ogg" => "ogg",
        "audio/aac" => "aac",
        "audio/flac" => "flac",
        "audio/m4a" => "m4a",
        "audio/wma" => "wma",

        // 문서
        "application/pdf" => "pdf",
        "application/msword" => "doc",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => "docx",
        "application/vnd.ms-excel" => "xls",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => "xlsx",
        "application/vnd.ms-powerpoint" => "ppt",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation" => "pptx",

        // 압축
        "application/zip" => "zip",
        "application/x-rar-compressed" => "rar",
        "application/x-7z-compressed" => "7z",
        "application/gzip" => "gz",
        "application/x-tar" => "tar",

        // 기타
        "text/plain" => "txt",
        "text/html" => "html",
        "text/css" => "css",
        "application/javascript" | "text/javascript" => "js",
        "application/json" => "json",
        "application/xml" | "text/xml" => "xml",

        _ => "", // 기본값
    }
}

/// Body를 파일로 저장하는 함수
///
/// # Arguments
/// * `id` - 요청/응답의 고유 ID
/// * `body` - 저장할 body 데이터
/// * `cache_dir` - 캐시 디렉토리 경로
/// * `prefix` - 파일명 접두사 ("request" 또는 "response")
/// * `data_type` - 데이터 타입
/// * `headers` - HTTP 헤더 (MIME 타입 추출용)
///
/// # Returns
/// * `Ok(String)` - 저장된 파일의 전체 경로
/// * `Err(String)` - 저장 실패 시 에러 메시지
pub fn save_body_to_file(
    id: &str,
    body: &Bytes,
    cache_dir: &Path,
    prefix: &str,
    data_type: &DataType,
    headers: &HeaderMap,
) -> Result<String, String> {
    use std::fs::File;
    use std::io::Write;

    // 확장자 결정: MIME 타입 -> 파일 헤더 -> 데이터 타입 순으로 시도
    let extension = if let Some(content_type) = headers.get("content-type") {
        if let Ok(mime_type) = content_type.to_str() {
            let mime_ext = get_extension_from_mime_type(mime_type);
            if !mime_ext.is_empty() {
                mime_ext
            } else {
                // MIME 타입에서 확장자를 찾을 수 없으면 파일 헤더 확인
                detect_file_extension_from_header(body).unwrap_or_else(|| match data_type {
                    DataType::Image => "img",
                    DataType::Video => "video",
                    DataType::Audio => "audio",
                    _ => "body",
                })
            }
        } else {
            // MIME 타입 파싱 실패 시 파일 헤더 확인
            detect_file_extension_from_header(body).unwrap_or_else(|| match data_type {
                DataType::Image => "img",
                DataType::Video => "video",
                DataType::Audio => "audio",
                _ => "body",
            })
        }
    } else {
        // Content-Type 헤더가 없는 경우 파일 헤더 확인
        detect_file_extension_from_header(body).unwrap_or_else(|| match data_type {
            DataType::Image => "img",
            DataType::Video => "video",
            DataType::Audio => "audio",
            _ => "body",
        })
    };

    // 파일명 생성: 확장자가 있으면 점 포함, 없으면 점 없음
    let filename = if extension.is_empty() {
        format!("{}_{}", id, prefix)
    } else {
        format!("{}_{}.{}", id, prefix, extension)
    };
    let file_path = cache_dir.join(filename);

    // 파일 생성 및 쓰기
    let mut file =
        File::create(&file_path).map_err(|e| format!("Failed to create body file: {}", e))?;

    file.write_all(body)
        .map_err(|e| format!("Failed to write body to file: {}", e))?;

    // 전체 경로를 문자열로 변환
    Ok(file_path.to_string_lossy().to_string())
}

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

    /// 클라이언트(타우리 UI)용으로 변환
    pub fn for_client(self, cache_dir: Option<&Path>) -> ClientRequest {
        let original_body_size = self.body.len();
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
                        Err(e) => (Some(self.body), None),
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
        let decompressed_body = if data_type == data_type::DataType::Json
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
                        Err(e) => (Some(body_to_save), None),
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
pub struct RequestInfo(pub Option<ClientRequest>, pub Option<ClientResponse>);

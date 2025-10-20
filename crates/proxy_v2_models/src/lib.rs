use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

// Re-export commonly used types
pub use bytes::Bytes;
pub use http::{HeaderMap, Method, StatusCode, Uri, Version};

// Re-export data type module
pub mod data_type;
pub use data_type::{decompress_brotli, decompress_gzip, detect_data_type, DataType};

/// Body 크기 임계값 (1MB)
/// 이 크기 이상의 body는 파일시스템에 저장됩니다
pub const BODY_FILE_THRESHOLD: usize = 1_048_576; // 1MB

/// Body를 파일로 저장하는 함수
///
/// # Arguments
/// * `id` - 요청/응답의 고유 ID
/// * `body` - 저장할 body 데이터
/// * `cache_dir` - 캐시 디렉토리 경로
/// * `prefix` - 파일명 접두사 ("request" 또는 "response")
///
/// # Returns
/// * `Ok(String)` - 저장된 파일의 전체 경로
/// * `Err(String)` - 저장 실패 시 에러 메시지
pub fn save_body_to_file(
    id: &str,
    body: &Bytes,
    cache_dir: &Path,
    prefix: &str,
) -> Result<String, String> {
    use std::fs::File;
    use std::io::Write;

    // 파일명 생성: {id}_{prefix}.body
    let filename = format!("{}_{}.body", id, prefix);
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
        let (body, file_path) = if original_body_size >= BODY_FILE_THRESHOLD {
            // 큰 body는 파일로 저장
            if let Some(cache_dir) = cache_dir {
                match save_body_to_file(&self.id, &self.body, cache_dir, "request") {
                    Ok(path) => {
                        println!(
                            "📁 Request body 저장됨: {} ({} bytes)",
                            path,
                            original_body_size
                        );
                        (Bytes::new(), Some(path))
                    }
                    Err(e) => {
                        eprintln!("⚠️ Request body 파일 저장 실패: {}", e);
                        (self.body, None)
                    }
                }
            } else {
                (self.body, None)
            }
        } else {
            // 작은 body는 메모리에 유지
            (self.body, None)
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
    body: Bytes,
    time: i64,
    id: String,
    data_type: DataType,
    body_json: Option<serde_json::Value>,
    file_path: Option<String>, // body가 저장된 파일 경로
    body_size: usize, // 실제 body 크기 (파일 저장 시에도 원본 크기 유지)
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

    pub fn body(&self) -> &Bytes {
        &self.body
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
        let (body, file_path) = if original_body_size >= BODY_FILE_THRESHOLD {
            // 큰 body는 파일로 저장
            if let Some(cache_dir) = cache_dir {
                match save_body_to_file(request_id, &body_to_save, cache_dir, "response") {
                    Ok(path) => {
                        println!(
                            "📁 Response body 저장됨: {} ({} bytes)",
                            path,
                            original_body_size
                        );
                        (Bytes::new(), Some(path))
                    }
                    Err(e) => {
                        eprintln!("⚠️ Response body 파일 저장 실패: {}", e);
                        (body_to_save, None)
                    }
                }
            } else {
                (body_to_save, None)
            }
        } else {
            // 작은 body는 메모리에 유지
            (body_to_save, None)
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
    body: Bytes,
    time: i64,
    id: String, // ClientRequest의 id와 동일
    data_type: DataType,
    body_json: Option<serde_json::Value>,
    file_path: Option<String>, // body가 저장된 파일 경로
    body_size: usize, // 실제 body 크기 (파일 저장 시에도 원본 크기 유지)
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

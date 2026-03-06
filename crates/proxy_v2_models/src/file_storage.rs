use bytes::Bytes;
use http::HeaderMap;
use std::path::Path;

use crate::data_type::{decompress_brotli, decompress_gzip, DataType};
use crate::mime_utils::{detect_file_extension_from_header, get_extension_from_mime_type};

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
    let fallback_ext = detect_file_extension_from_header(body).unwrap_or(match data_type {
        DataType::Image => "img",
        DataType::Video => "video",
        DataType::Audio => "audio",
        _ => "body",
    });

    let extension = if let Some(content_type) = headers.get("content-type") {
        if let Ok(mime_type) = content_type.to_str() {
            let mime_ext = get_extension_from_mime_type(mime_type);
            if !mime_ext.is_empty() {
                mime_ext
            } else {
                fallback_ext
            }
        } else {
            fallback_ext
        }
    } else {
        fallback_ext
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

    // 상대 경로만 반환 (BaseDirectory.Cache 기준)
    // cache_dir에서 세션 해시 부분을 제외한 기본 캐시 디렉토리 기준으로 상대 경로 생성
    let base_cache_dir = cache_dir.parent().unwrap(); // /Users/[username]/Library/Caches/com.cheolsu-proxy/data/
    let relative_path: String = file_path
        .strip_prefix(base_cache_dir)
        .map_err(|e| format!("Failed to create relative path: {}", e))?
        .to_string_lossy()
        .to_string();

    // 앱별 캐시 디렉토리 경로 추가 (com.cheolsu-proxy/data/...)
    let app_cache_path = format!("com.cheolsu-proxy/data/{}", relative_path);
    Ok(app_cache_path)
}

/// 압축된 body를 해제하는 헬퍼 함수
pub fn decompress_body_if_needed(headers: &HeaderMap, body: &Bytes) -> Vec<u8> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use http::HeaderMap;

    #[test]
    fn decompress_plain_body_unchanged() {
        let body = Bytes::from("hello world");
        let headers = HeaderMap::new();
        let result = decompress_body_if_needed(&headers, &body);
        assert_eq!(result, b"hello world");
    }

    #[test]
    fn decompress_gzip_body_with_header() {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(b"compressed data").unwrap();
        let compressed = encoder.finish().unwrap();

        let body = Bytes::from(compressed);
        let mut headers = HeaderMap::new();
        headers.insert("content-encoding", "gzip".parse().unwrap());

        let result = decompress_body_if_needed(&headers, &body);
        assert_eq!(result, b"compressed data");
    }

    #[test]
    fn decompress_gzip_body_without_header_by_magic_number() {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(b"magic gzip").unwrap();
        let compressed = encoder.finish().unwrap();

        let body = Bytes::from(compressed);
        let headers = HeaderMap::new(); // no content-encoding header

        let result = decompress_body_if_needed(&headers, &body);
        assert_eq!(result, b"magic gzip");
    }

    #[test]
    fn decompress_invalid_gzip_returns_original() {
        let body = Bytes::from_static(&[0x1f, 0x8b, 0x00, 0x00, 0xFF, 0xFF]);
        let headers = HeaderMap::new();
        let result = decompress_body_if_needed(&headers, &body);
        // Should return original since decompression fails
        assert_eq!(result, body.to_vec());
    }

    #[test]
    fn save_body_to_file_with_content_type() {
        let dir = tempfile::tempdir().unwrap();
        let cache_dir = dir.path().join("session");
        std::fs::create_dir_all(&cache_dir).unwrap();

        let body = Bytes::from("test body content");
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());

        let result = save_body_to_file(
            "req-001",
            &body,
            &cache_dir,
            "request",
            &DataType::Json,
            &headers,
        );

        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.contains("req-001_request.json"));
    }

    #[test]
    fn save_body_to_file_with_binary_header_detection() {
        let dir = tempfile::tempdir().unwrap();
        let cache_dir = dir.path().join("session");
        std::fs::create_dir_all(&cache_dir).unwrap();

        // PNG magic bytes
        let mut png_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        png_data.extend_from_slice(&[0; 100]);
        let body = Bytes::from(png_data);
        let headers = HeaderMap::new(); // no content-type

        let result = save_body_to_file(
            "req-002",
            &body,
            &cache_dir,
            "response",
            &DataType::Image,
            &headers,
        );

        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.contains("req-002_response.png"));
    }
}

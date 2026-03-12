mod binary_signature;
mod content_type_header;

use bytes::Bytes;
use http::HeaderMap;

use super::decompression::{decompress_brotli, decompress_gzip};
use super::DataType;

use binary_signature::{
    detect_audio_signature, detect_document_archive_signature, detect_ftyp_signature,
    detect_image_signature, detect_video_signature,
};
use content_type_header::detect_by_content_type_header;

/// GZIP 압축된 데이터의 실제 내용 타입 감지
fn detect_gzip_content_type(data: &[u8]) -> DataType {
    match decompress_gzip(data) {
        Ok(decompressed) => {
            let headers = HeaderMap::new();
            detect_data_type(&headers, &Bytes::from(decompressed))
        }
        Err(_) => DataType::Archive,
    }
}

/// Brotli 압축된 데이터의 실제 내용 타입 감지
fn detect_brotli_content_type(data: &[u8]) -> DataType {
    match decompress_brotli(data) {
        Ok(decompressed) => {
            let headers = HeaderMap::new();
            detect_data_type(&headers, &Bytes::from(decompressed))
        }
        Err(_) => DataType::Binary,
    }
}

/// 데이터 타입 감지 유틸리티 함수 (MITM 프록시에 최적화)
pub fn detect_data_type(headers: &HeaderMap, body: &Bytes) -> DataType {
    // 0. Content-Encoding 헤더 확인 (가장 우선순위 높음)
    if let Some(content_encoding) = headers.get("content-encoding") {
        if let Ok(encoding) = content_encoding.to_str() {
            let encoding_lower = encoding.to_lowercase();
            // Brotli 압축 감지
            if encoding_lower.contains("br") {
                return detect_brotli_content_type(body);
            }
            // GZIP 압축 감지 (헤더로 확인)
            if encoding_lower.contains("gzip") {
                return detect_gzip_content_type(body);
            }
        }
    }

    // 1. 내용 분석 (JSON 감지 포함)
    if !body.is_empty() {
        // JSON 감지 (가장 정확한 방법)
        if let Ok(body_str) = std::str::from_utf8(body) {
            let trimmed = body_str.trim();
            if (trimmed.starts_with('{') || trimmed.starts_with('['))
                && serde_json::from_str::<serde_json::Value>(trimmed).is_ok()
            {
                // GraphQL 감지: JSON object에 "query" 문자열 필드가 있으면 GraphQL
                if trimmed.starts_with('{') {
                    if let Ok(serde_json::Value::Object(map)) =
                        serde_json::from_str::<serde_json::Value>(trimmed)
                    {
                        if map.get("query").is_some_and(|v| v.is_string()) {
                            return DataType::GraphQL;
                        }
                    }
                }
                return DataType::Json;
            }
        }

        // GZIP 압축 파일 감지 및 내용 분석 (magic number로 확인)
        if body.len() >= 2 && body[0] == 0x1f && body[1] == 0x8b {
            return detect_gzip_content_type(body);
        }

        // SVG 감지 (XML보다 우선)
        if let Ok(body_str) = std::str::from_utf8(body) {
            let trimmed = body_str.trim();
            if trimmed.starts_with("<svg") || trimmed.contains("<svg") {
                return DataType::Image;
            }
        }

        // 바이너리 파일 내용 분석 (이미지, 동영상, 오디오, 문서, 아카이브만)

        // 이미지 파일 감지 (구체적인 형식) - 우선순위 높음
        if let Some(data_type) = detect_image_signature(body) {
            return data_type;
        }

        // ftyp 기반 컨테이너 감지 (이미지/오디오/비디오 통합)
        if let Some(data_type) = detect_ftyp_signature(body) {
            return data_type;
        }

        // 비디오 파일 감지 (ftyp 이외)
        if let Some(data_type) = detect_video_signature(body) {
            return data_type;
        }

        // 오디오 파일 감지 (ftyp 이외)
        if let Some(data_type) = detect_audio_signature(body) {
            return data_type;
        }

        // 문서/아카이브 파일 감지
        if let Some(data_type) = detect_document_archive_signature(body) {
            return data_type;
        }
    }

    // 2. Content-Type 헤더 확인 (내용 분석 다음)
    if let Some(data_type) = detect_by_content_type_header(headers) {
        return data_type;
    }

    // 3. 기본값 (내용 분석과 Content-Type 헤더로 구분할 수 없는 경우)
    if body.is_empty() {
        DataType::Empty
    } else {
        // 간단한 텍스트/바이너리 구분만 수행
        if std::str::from_utf8(body).is_ok() {
            DataType::Text
        } else {
            DataType::Binary
        }
    }
}

#[cfg(test)]
mod tests;

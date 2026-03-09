use bytes::Bytes;
use http::HeaderMap;

use super::decompression::{decompress_brotli, decompress_gzip};
use super::DataType;

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
        if body.len() >= 2 {
            // PNG 시그니처 (매우 명확)
            if body.len() >= 8 && &body[0..8] == b"\x89PNG\r\n\x1a\n" {
                return DataType::Image;
            }

            // JPEG 시그니처 (매우 명확)
            if &body[0..2] == b"\xff\xd8" {
                return DataType::Image;
            }

            // GIF 시그니처 (매우 명확)
            if body.len() >= 6 && (&body[0..6] == b"GIF87a" || &body[0..6] == b"GIF89a") {
                return DataType::Image;
            }

            // WebP 시그니처 (RIFF 컨테이너 확인)
            if body.len() >= 12 && &body[0..4] == b"RIFF" && &body[8..12] == b"WEBP" {
                return DataType::Image;
            }

            // BMP 시그니처
            if body.len() >= 2 && &body[0..2] == b"BM" {
                return DataType::Image;
            }

            // ICO 시그니처
            if body.len() >= 4 && &body[0..4] == b"\x00\x00\x01\x00" {
                return DataType::Image;
            }

            // TIFF 시그니처 (Little Endian)
            if body.len() >= 4 && &body[0..4] == b"II*\x00" {
                return DataType::Image;
            }

            // TIFF 시그니처 (Big Endian)
            if body.len() >= 4 && &body[0..4] == b"MM\x00*" {
                return DataType::Image;
            }
        }

        // ftyp 기반 컨테이너 감지 (이미지/오디오/비디오 통합)
        if body.len() >= 12 && &body[4..8] == b"ftyp" {
            let brand = &body[8..12];
            // 이미지: AVIF/HEIF/HEIC
            if brand == b"avif"
                || brand == b"avis"
                || brand == b"heic"
                || brand == b"heix"
                || brand == b"mif1"
            {
                return DataType::Image;
            }
            // 오디오: M4A/AAC
            if brand == b"M4A " || brand == b"M4B " || brand == b"mp4a" {
                return DataType::Audio;
            }
            // 비디오: MP4/M4V/3GP 등
            if brand == b"mp41"
                || brand == b"mp42"
                || brand == b"isom"
                || brand == b"avc1"
                || brand == b"iso2"
                || brand == b"iso3"
                || brand == b"iso4"
                || brand == b"iso5"
                || brand == b"iso6"
                || brand == b"M4V "
                || brand == b"M4VP"
                || brand == b"3gp4"
                || brand == b"3gp5"
                || brand == b"3gp6"
                || brand == b"3g2a"
                || brand == b"dash"
                || brand == b"mmp4"
            {
                return DataType::Video;
            }
            // ftyp가 있지만 알 수 없는 브랜드 → 비디오로 추정 (안전한 폴백)
            return DataType::Video;
        }

        // 비디오 파일 감지 (ftyp 이외)
        if body.len() >= 8 {
            // MOV 파일 시그니처 (QuickTime)
            if &body[4..8] == b"moov"
                || &body[4..8] == b"mdat"
                || &body[4..8] == b"free"
                || &body[4..8] == b"wide"
            {
                return DataType::Video;
            }
        }

        if body.len() >= 4 {
            // WebM/MKV 시그니처 (EBML 매직 넘버)
            if &body[0..4] == b"\x1a\x45\xdf\xa3" {
                return DataType::Video;
            }
            // AVI 시그니처 (RIFF 컨테이너)
            if body.len() >= 12 && &body[0..4] == b"RIFF" && &body[8..12] == b"AVI " {
                return DataType::Video;
            }
            // FLV 시그니처
            if &body[0..3] == b"FLV" && body[3] == 0x01 {
                return DataType::Video;
            }
            // MPEG-TS 시그니처 (동기화 바이트 0x47, 188바이트 패킷)
            if body[0] == 0x47 && body.len() >= 188 + 4 && body[188] == 0x47 {
                return DataType::Video;
            }
        }

        // 오디오 파일 감지 (ftyp 이외)
        if body.len() >= 2 {
            // MP3 시그니처 (ID3 태그)
            if body.len() >= 3 && &body[0..3] == b"ID3" {
                return DataType::Audio;
            }
            // MP3 프레임 동기 (0xFF + 상위 3비트 = 111)
            if body[0] == 0xFF && (body[1] & 0xE0) == 0xE0 {
                return DataType::Audio;
            }
            // WAV 시그니처 (RIFF 컨테이너)
            if body.len() >= 12 && &body[0..4] == b"RIFF" && &body[8..12] == b"WAVE" {
                return DataType::Audio;
            }
            // OGG 시그니처 (Vorbis, Opus 등)
            if body.len() >= 4 && &body[0..4] == b"OggS" {
                return DataType::Audio;
            }
            // FLAC 시그니처
            if body.len() >= 4 && &body[0..4] == b"fLaC" {
                return DataType::Audio;
            }
            // AIFF 시그니처
            if body.len() >= 12 && &body[0..4] == b"FORM" && &body[8..12] == b"AIFF" {
                return DataType::Audio;
            }
        }

        // 문서 파일 감지
        if body.len() >= 4 && &body[0..4] == b"%PDF" {
            return DataType::Document;
        }

        // ZIP 아카이브 감지 (GZIP이 아닌 경우)
        if body.len() >= 4 && &body[0..4] == b"PK\x03\x04" {
            return DataType::Archive;
        }
    }

    // 2. Content-Type 헤더 확인 (내용 분석 다음)
    if let Some(content_type_header) = headers.get("content-type") {
        if let Ok(content_type_str) = content_type_header.to_str() {
            let content_type = content_type_str.to_lowercase();
            if content_type.contains("application/grpc")
                || content_type.contains("application/protobuf")
                || content_type.contains("application/x-protobuf")
            {
                return DataType::Protobuf;
            } else if content_type.contains("graphql") {
                return DataType::GraphQL;
            } else if content_type.contains("json") {
                return DataType::Json;
            } else if content_type.contains("xml") {
                return DataType::Xml;
            } else if content_type.contains("html") {
                return DataType::Html;
            } else if content_type.contains("css") {
                return DataType::Css;
            } else if content_type.contains("javascript") || content_type.contains("typescript") {
                return DataType::Javascript;
            } else if content_type.contains("image/") {
                return DataType::Image;
            } else if content_type.contains("video/") {
                return DataType::Video;
            } else if content_type.contains("audio/") {
                return DataType::Audio;
            } else if content_type.contains("pdf") {
                return DataType::Document;
            } else if content_type.contains("zip") || content_type.contains("gzip") {
                return DataType::Archive;
            } else if content_type.contains("text") {
                return DataType::Text;
            }
        }
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
mod tests {
    use super::*;
    use http::HeaderMap;

    #[test]
    fn test_json_detection() {
        use http::HeaderValue;

        // 내용 분석으로 JSON 감지 (Content-Type 헤더 없이)
        let headers = HeaderMap::new();
        let body = Bytes::from(r#"{"key": "value"}"#);
        assert_eq!(detect_data_type(&headers, &body), DataType::Json);

        // Content-Type 헤더로도 JSON 감지
        let mut headers_with_json = HeaderMap::new();
        headers_with_json.insert("content-type", HeaderValue::from_static("application/json"));
        let body = Bytes::from(r#"{"key": "value"}"#);
        assert_eq!(detect_data_type(&headers_with_json, &body), DataType::Json);

        // 유효하지 않은 JSON은 텍스트로 분류
        let invalid_json = Bytes::from("{ invalid json }");
        assert_eq!(detect_data_type(&headers, &invalid_json), DataType::Text);
    }

    #[test]
    fn test_xml_detection() {
        use http::HeaderValue;

        // Content-Type 헤더로 XML 감지
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/xml"));
        let body = Bytes::from("<root><item>test</item></root>");
        assert_eq!(detect_data_type(&headers, &body), DataType::Xml);

        // Content-Type이 없는 경우 텍스트로 분류
        headers.clear();
        let xml_without_header = Bytes::from("<root><item>test</item></root>");
        assert_eq!(
            detect_data_type(&headers, &xml_without_header),
            DataType::Text
        );
    }

    #[test]
    fn test_html_detection() {
        use http::HeaderValue;

        // Content-Type 헤더로 HTML 감지
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("text/html"));
        let body = Bytes::from("<!DOCTYPE html><html><body>test</body></html>");
        assert_eq!(detect_data_type(&headers, &body), DataType::Html);

        // Content-Type이 없는 경우 텍스트로 분류
        headers.clear();
        let html_without_header = Bytes::from("<!DOCTYPE html><html><body>test</body></html>");
        assert_eq!(
            detect_data_type(&headers, &html_without_header),
            DataType::Text
        );
    }

    #[test]
    fn test_empty_body() {
        let headers = HeaderMap::new();
        let body = Bytes::new();
        assert_eq!(detect_data_type(&headers, &body), DataType::Empty);
    }

    #[test]
    fn test_css_detection() {
        use http::HeaderValue;

        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("text/css"));

        let css_content = Bytes::from("@import url('style.css'); body { color: red; }");
        assert_eq!(detect_data_type(&headers, &css_content), DataType::Css);

        headers.clear();
        let css_without_header = Bytes::from(".my-class { background: blue; }");
        assert_eq!(
            detect_data_type(&headers, &css_without_header),
            DataType::Text
        );
    }

    #[test]
    fn test_javascript_detection() {
        use http::HeaderValue;

        let mut headers = HeaderMap::new();
        headers.insert(
            "content-type",
            HeaderValue::from_static("application/javascript"),
        );

        let js_code = Bytes::from("function hello() { console.log('Hello World'); }");
        assert_eq!(detect_data_type(&headers, &js_code), DataType::Javascript);

        headers.clear();
        headers.insert(
            "content-type",
            HeaderValue::from_static("application/typescript"),
        );
        let ts_interface = Bytes::from("interface User { name: string; age: number; }");
        assert_eq!(
            detect_data_type(&headers, &ts_interface),
            DataType::Javascript
        );

        headers.clear();
        let js_without_header = Bytes::from("const add = (a, b) => a + b;");
        assert_eq!(
            detect_data_type(&headers, &js_without_header),
            DataType::Text
        );
    }

    #[test]
    fn test_image_detection() {
        let headers = HeaderMap::new();

        // PNG
        let png_data = Bytes::from(vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
        assert_eq!(detect_data_type(&headers, &png_data), DataType::Image);

        // JPEG
        let jpeg_data = Bytes::from(vec![0xFF, 0xD8, 0xFF]);
        assert_eq!(detect_data_type(&headers, &jpeg_data), DataType::Image);

        // SVG
        let svg_data = Bytes::from(
            "<svg width=\"100\" height=\"100\"><circle cx=\"50\" cy=\"50\" r=\"40\"/></svg>",
        );
        assert_eq!(detect_data_type(&headers, &svg_data), DataType::Image);

        // GIF
        let gif_data = Bytes::from(b"GIF89a\x01\x00".as_slice());
        assert_eq!(detect_data_type(&headers, &gif_data), DataType::Image);

        // WebP
        let webp_data = Bytes::from(vec![
            b'R', b'I', b'F', b'F', 0x00, 0x00, 0x00, 0x00, b'W', b'E', b'B', b'P',
        ]);
        assert_eq!(detect_data_type(&headers, &webp_data), DataType::Image);

        // BMP
        let bmp_data = Bytes::from(vec![b'B', b'M', 0x00, 0x00]);
        assert_eq!(detect_data_type(&headers, &bmp_data), DataType::Image);

        // ICO
        let ico_data = Bytes::from(vec![0x00, 0x00, 0x01, 0x00]);
        assert_eq!(detect_data_type(&headers, &ico_data), DataType::Image);

        // TIFF (Little Endian)
        let tiff_le_data = Bytes::from(b"II*\x00".as_slice());
        assert_eq!(detect_data_type(&headers, &tiff_le_data), DataType::Image);

        // TIFF (Big Endian)
        let tiff_be_data = Bytes::from(b"MM\x00*".as_slice());
        assert_eq!(detect_data_type(&headers, &tiff_be_data), DataType::Image);

        // AVIF
        let avif_data = Bytes::from(vec![
            0x00, 0x00, 0x00, 0x1C, b'f', b't', b'y', b'p', b'a', b'v', b'i', b'f',
        ]);
        assert_eq!(detect_data_type(&headers, &avif_data), DataType::Image);

        // HEIC
        let heic_data = Bytes::from(vec![
            0x00, 0x00, 0x00, 0x18, b'f', b't', b'y', b'p', b'h', b'e', b'i', b'c',
        ]);
        assert_eq!(detect_data_type(&headers, &heic_data), DataType::Image);
    }

    #[test]
    fn test_video_detection() {
        let headers = HeaderMap::new();

        // MP4
        let mp4_data = Bytes::from(vec![
            0x00, 0x00, 0x00, 0x20, 0x66, 0x74, 0x79, 0x70, 0x69, 0x73, 0x6F, 0x6D,
        ]);
        assert_eq!(detect_data_type(&headers, &mp4_data), DataType::Video);

        // WebM
        let webm_data = Bytes::from(vec![0x1A, 0x45, 0xDF, 0xA3]);
        assert_eq!(detect_data_type(&headers, &webm_data), DataType::Video);

        // MOV (moov)
        let mov_data = Bytes::from(vec![0x00, 0x00, 0x00, 0x08, b'm', b'o', b'o', b'v']);
        assert_eq!(detect_data_type(&headers, &mov_data), DataType::Video);

        // AVI
        let avi_data = Bytes::from(vec![
            b'R', b'I', b'F', b'F', 0x00, 0x00, 0x00, 0x00, b'A', b'V', b'I', b' ',
        ]);
        assert_eq!(detect_data_type(&headers, &avi_data), DataType::Video);

        // FLV
        let flv_data = Bytes::from(vec![b'F', b'L', b'V', 0x01]);
        assert_eq!(detect_data_type(&headers, &flv_data), DataType::Video);

        // 3GP
        let gp3_data = Bytes::from(vec![
            0x00, 0x00, 0x00, 0x14, b'f', b't', b'y', b'p', b'3', b'g', b'p', b'5',
        ]);
        assert_eq!(detect_data_type(&headers, &gp3_data), DataType::Video);

        // 알 수 없는 ftyp 브랜드도 비디오로 감지
        let unknown_ftyp = Bytes::from(vec![
            0x00, 0x00, 0x00, 0x14, b'f', b't', b'y', b'p', b'x', b'x', b'x', b'x',
        ]);
        assert_eq!(detect_data_type(&headers, &unknown_ftyp), DataType::Video);
    }

    #[test]
    fn test_audio_detection() {
        let headers = HeaderMap::new();

        // MP3 (ID3)
        let mp3_data = Bytes::from(vec![0x49, 0x44, 0x33]);
        assert_eq!(detect_data_type(&headers, &mp3_data), DataType::Audio);

        // MP3 프레임 동기
        let mp3_sync = Bytes::from(vec![0xFF, 0xFB, 0x90, 0x00]);
        assert_eq!(detect_data_type(&headers, &mp3_sync), DataType::Audio);

        // WAV
        let wav_data = Bytes::from(vec![
            0x52, 0x49, 0x46, 0x46, 0x00, 0x00, 0x00, 0x00, 0x57, 0x41, 0x56, 0x45,
        ]);
        assert_eq!(detect_data_type(&headers, &wav_data), DataType::Audio);

        // OGG
        let ogg_data = Bytes::from(b"OggS\x00\x02".as_slice());
        assert_eq!(detect_data_type(&headers, &ogg_data), DataType::Audio);

        // FLAC
        let flac_data = Bytes::from(b"fLaC\x00\x00".as_slice());
        assert_eq!(detect_data_type(&headers, &flac_data), DataType::Audio);

        // AIFF
        let aiff_data = Bytes::from(vec![
            b'F', b'O', b'R', b'M', 0x00, 0x00, 0x00, 0x00, b'A', b'I', b'F', b'F',
        ]);
        assert_eq!(detect_data_type(&headers, &aiff_data), DataType::Audio);

        // M4A
        let m4a_data = Bytes::from(vec![
            0x00, 0x00, 0x00, 0x20, b'f', b't', b'y', b'p', b'M', b'4', b'A', b' ',
        ]);
        assert_eq!(detect_data_type(&headers, &m4a_data), DataType::Audio);
    }

    #[test]
    fn test_document_detection() {
        let headers = HeaderMap::new();
        let pdf_data = Bytes::from(vec![0x25, 0x50, 0x44, 0x46]);
        assert_eq!(detect_data_type(&headers, &pdf_data), DataType::Document);
    }

    #[test]
    fn test_archive_detection() {
        let headers = HeaderMap::new();

        // ZIP
        let zip_data = Bytes::from(vec![0x50, 0x4B, 0x03, 0x04]);
        assert_eq!(detect_data_type(&headers, &zip_data), DataType::Archive);

        // GZIP (압축 해제 실패 시 Archive 반환)
        let gzip_data = Bytes::from(vec![0x1F, 0x8B]);
        assert_eq!(detect_data_type(&headers, &gzip_data), DataType::Archive);
    }

    #[test]
    fn test_gzip_decompression() {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let headers = HeaderMap::new();

        // JSON 데이터를 GZIP으로 압축
        let json_data = r#"{"name": "test", "value": 123}"#;
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(json_data.as_bytes()).unwrap();
        let compressed = encoder.finish().unwrap();

        assert_eq!(
            detect_data_type(&headers, &Bytes::from(compressed)),
            DataType::Json
        );

        // HTML 데이터를 GZIP으로 압축
        let html_data =
            "<!DOCTYPE html><html><head><title>Test</title></head><body>Content</body></html>";
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(html_data.as_bytes()).unwrap();
        let compressed = encoder.finish().unwrap();

        assert_eq!(
            detect_data_type(&headers, &Bytes::from(compressed)),
            DataType::Text
        );
    }

    #[test]
    fn test_content_type_header_priority() {
        use http::HeaderValue;

        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));

        let non_json_body = Bytes::from("this is not json");
        assert_eq!(detect_data_type(&headers, &non_json_body), DataType::Json);

        headers.clear();
        headers.insert("content-type", HeaderValue::from_static("text/css"));
        let non_css_body = Bytes::from("this is not css");
        assert_eq!(detect_data_type(&headers, &non_css_body), DataType::Css);
    }

    #[test]
    fn test_fallback_to_text_or_binary() {
        let headers = HeaderMap::new();

        let unknown_text = Bytes::from("some random text that doesn't match any pattern");
        assert_eq!(detect_data_type(&headers, &unknown_text), DataType::Text);

        let binary_data = Bytes::from(vec![0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]);
        assert_eq!(detect_data_type(&headers, &binary_data), DataType::Binary);
    }

    #[test]
    fn test_graphql_detection() {
        use http::HeaderValue;

        let headers = HeaderMap::new();

        let graphql_body =
            Bytes::from(r#"{"query":"query { user(id: 1) { name } }","variables":{}}"#);
        assert_eq!(detect_data_type(&headers, &graphql_body), DataType::GraphQL);

        let json_body = Bytes::from(r#"{"name":"test","value":123}"#);
        assert_eq!(detect_data_type(&headers, &json_body), DataType::Json);

        let array_body = Bytes::from(r#"[{"query":"test"}]"#);
        assert_eq!(detect_data_type(&headers, &array_body), DataType::Json);

        let mut graphql_headers = HeaderMap::new();
        graphql_headers.insert(
            "content-type",
            HeaderValue::from_static("application/graphql+json"),
        );
        let body = Bytes::from("some text");
        assert_eq!(detect_data_type(&graphql_headers, &body), DataType::GraphQL);
    }

    #[test]
    fn test_protobuf_detection_by_content_type() {
        use http::HeaderValue;

        let body = Bytes::from(vec![0x08, 0x96, 0x01]);

        let cases = vec![
            "application/protobuf",
            "application/x-protobuf",
            "application/grpc",
            "application/grpc+proto",
            "application/grpc-web",
            "application/grpc-web+proto",
        ];

        for ct in cases {
            let mut headers = HeaderMap::new();
            headers.insert("content-type", HeaderValue::from_str(ct).unwrap());
            assert_eq!(
                detect_data_type(&headers, &body),
                DataType::Protobuf,
                "Content-Type: {} should be detected as Protobuf",
                ct
            );
        }
    }

    #[test]
    fn test_protobuf_not_detected_without_content_type() {
        let headers = HeaderMap::new();
        let body = Bytes::from(vec![0x08, 0x96, 0x01]);
        assert_eq!(detect_data_type(&headers, &body), DataType::Binary);
    }
}

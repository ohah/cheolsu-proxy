use bytes::Bytes;
use flate2::read::GzDecoder;
use http::HeaderMap;
use serde::{Deserialize, Serialize};
use std::io::Read;

/// 데이터 타입을 나타내는 열거형 (MITM 프록시에 최적화)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DataType {
    /// JSON 데이터
    Json,
    /// XML 데이터
    Xml,
    /// HTML 데이터
    Html,
    /// 일반 텍스트
    Text,
    /// CSS 스타일시트
    Css,
    /// JavaScript/TypeScript 코드
    Javascript,
    /// 이미지 파일 (PNG, JPEG, GIF, WebP, SVG 등)
    Image,
    /// 비디오 파일 (MP4, WebM 등)
    Video,
    /// 오디오 파일 (MP3, WAV 등)
    Audio,
    /// 문서 파일 (PDF 등)
    Document,
    /// 압축 파일 (ZIP, GZIP 등)
    Archive,
    /// 바이너리 데이터 (알 수 없는 형식)
    Binary,
    /// 빈 데이터
    Empty,
    /// 알 수 없는 타입
    Unknown,
}

impl DataType {
    /// DataType을 MIME 타입 문자열로 변환
    pub fn to_mime_type(&self) -> &'static str {
        match self {
            DataType::Json => "application/json",
            DataType::Xml => "application/xml",
            DataType::Html => "text/html",
            DataType::Text => "text/plain",
            DataType::Css => "text/css",
            DataType::Javascript => "application/javascript",
            DataType::Image => "image/*",
            DataType::Video => "video/*",
            DataType::Audio => "audio/*",
            DataType::Document => "application/pdf",
            DataType::Archive => "application/zip",
            DataType::Binary => "application/octet-stream",
            DataType::Empty => "empty",
            DataType::Unknown => "application/octet-stream",
        }
    }

    /// DataType을 Monaco Editor 언어 모드로 변환
    pub fn to_monaco_language(&self) -> &'static str {
        match self {
            DataType::Json => "json",
            DataType::Xml => "xml",
            DataType::Html => "html",
            DataType::Css => "css",
            DataType::Javascript => "javascript",
            DataType::Text => "plaintext",
            DataType::Image
            | DataType::Video
            | DataType::Audio
            | DataType::Document
            | DataType::Archive
            | DataType::Binary
            | DataType::Empty
            | DataType::Unknown => "plaintext",
        }
    }

    /// 데이터 타입이 텍스트 기반인지 확인
    pub fn is_text_based(&self) -> bool {
        matches!(
            self,
            DataType::Json
                | DataType::Xml
                | DataType::Html
                | DataType::Css
                | DataType::Javascript
                | DataType::Text
        )
    }

    /// 데이터 타입이 이미지인지 확인
    pub fn is_image(&self) -> bool {
        matches!(self, DataType::Image)
    }

    /// 데이터 타입이 비디오인지 확인
    pub fn is_video(&self) -> bool {
        matches!(self, DataType::Video)
    }

    /// 데이터 타입이 오디오인지 확인
    pub fn is_audio(&self) -> bool {
        matches!(self, DataType::Audio)
    }

    /// 데이터 타입이 문서인지 확인
    pub fn is_document(&self) -> bool {
        matches!(self, DataType::Document)
    }

    /// 데이터 타입이 압축 파일인지 확인
    pub fn is_archive(&self) -> bool {
        matches!(self, DataType::Archive)
    }

    /// 데이터 타입이 바이너리인지 확인
    pub fn is_binary(&self) -> bool {
        matches!(
            self,
            DataType::Image
                | DataType::Video
                | DataType::Audio
                | DataType::Document
                | DataType::Archive
                | DataType::Binary
        )
    }
}

/// GZIP 압축 해제 함수
pub fn decompress_gzip(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut decoder = GzDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}

/// GZIP 압축된 데이터의 실제 내용 타입 감지
fn detect_gzip_content_type(data: &[u8]) -> DataType {
    match decompress_gzip(data) {
        Ok(decompressed) => {
            // 압축 해제된 데이터로 타입 감지
            let headers = HeaderMap::new();
            detect_data_type(&headers, &Bytes::from(decompressed))
        }
        Err(_) => {
            // 압축 해제 실패 시 Archive로 반환
            DataType::Archive
        }
    }
}

/// 데이터 타입 감지 유틸리티 함수 (MITM 프록시에 최적화)
pub fn detect_data_type(headers: &HeaderMap, body: &Bytes) -> DataType {
    // 1. body 내용을 먼저 분석해서 타입 추론 (우선순위 높음)
    if !body.is_empty() {
        // GZIP 압축 파일 감지 및 내용 분석
        if body.len() >= 2 && body[0] == 0x1f && body[1] == 0x8b {
            // GZIP 압축 파일 - 압축 해제 후 실제 내용 타입 감지
            return detect_gzip_content_type(body);
        }

        // JSON 감지 (가장 정확한 방법)
        if let Ok(body_str) = std::str::from_utf8(body) {
            let trimmed = body_str.trim();
            if trimmed.starts_with('{') || trimmed.starts_with('[') {
                if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
                    return DataType::Json;
                }
            }
        }

        // SVG 감지 (XML보다 우선)
        if let Ok(body_str) = std::str::from_utf8(body) {
            let trimmed = body_str.trim();
            if trimmed.starts_with("<svg") || trimmed.contains("<svg") {
                return DataType::Image;
            }
        }

        // HTML 감지 (XML보다 우선)
        if let Ok(body_str) = std::str::from_utf8(body) {
            let lower_content = body_str.trim().to_lowercase();
            if lower_content.starts_with("<!doctype html")
                || lower_content.starts_with("<html")
                || (lower_content.contains("<head>") && lower_content.contains("<body>"))
                || (lower_content.contains("<title>") && lower_content.contains("<head>"))
            {
                return DataType::Html;
            }
        }

        // XML 감지 (HTML이 아닌 경우)
        if let Ok(body_str) = std::str::from_utf8(body) {
            let trimmed = body_str.trim();
            if trimmed.starts_with('<') {
                if trimmed.starts_with("<?xml")
                    || (trimmed.contains('<')
                        && trimmed.contains('>')
                        && !trimmed.contains("<!doctype"))
                {
                    return DataType::Xml;
                }
            }
        }

        // CSS 감지
        if let Ok(body_str) = std::str::from_utf8(body) {
            let trimmed = body_str.trim();
            let has_css_at_rules = trimmed.contains("@import")
                || trimmed.contains("@media")
                || trimmed.contains("@keyframes")
                || trimmed.contains("@font-face")
                || trimmed.contains("@supports");

            let has_css_selectors = trimmed.contains("body {")
                || trimmed.contains("div {")
                || trimmed.contains(".class")
                || trimmed.contains("#id")
                || trimmed.contains(":hover")
                || trimmed.contains(":focus");

            let has_css_properties = trimmed.contains("color:")
                || trimmed.contains("background:")
                || trimmed.contains("margin:")
                || trimmed.contains("padding:")
                || trimmed.contains("font-size:")
                || trimmed.contains("display:");

            if (has_css_at_rules || has_css_selectors || has_css_properties)
                && trimmed.contains("{")
                && trimmed.contains("}")
                && !trimmed.starts_with('<')
                && !trimmed.starts_with('{')
                && !trimmed.starts_with('[')
            {
                return DataType::Css;
            }
        }

        // JavaScript/TypeScript 감지
        if let Ok(body_str) = std::str::from_utf8(body) {
            let trimmed = body_str.trim();

            let has_js_patterns = trimmed.contains("function ")
                || trimmed.contains("const ")
                || trimmed.contains("let ")
                || trimmed.contains("var ")
                || trimmed.contains("=>")
                || trimmed.contains("() =>")
                || trimmed.contains("async ")
                || trimmed.contains("await ")
                || trimmed.contains("import ")
                || trimmed.contains("export ")
                || trimmed.contains("require(")
                || trimmed.contains("module.exports")
                || trimmed.contains("class ")
                || trimmed.contains("new ")
                || trimmed.contains("this.")
                || trimmed.contains("console.log")
                || trimmed.contains("setTimeout")
                || trimmed.contains("setInterval")
                || trimmed.contains("addEventListener")
                || trimmed.contains("document.")
                || trimmed.contains("window.")
                || trimmed.contains("JSON.")
                || trimmed.contains("Array.")
                || trimmed.contains("Object.")
                || trimmed.contains("Math.")
                || trimmed.contains("Date.")
                || trimmed.contains("Promise.")
                || trimmed.contains("fetch(")
                || trimmed.contains("XMLHttpRequest")
                || trimmed.contains("interface ")
                || trimmed.contains("type ")
                || trimmed.contains(": string")
                || trimmed.contains(": number")
                || trimmed.contains(": boolean")
                || trimmed.contains(": any")
                || trimmed.contains(": void")
                || trimmed.contains(": object")
                || trimmed.contains("enum ")
                || trimmed.contains("namespace ")
                || trimmed.contains("declare ");

            if has_js_patterns
                && !trimmed.starts_with('<')
                && !trimmed.starts_with('{')
                && !trimmed.starts_with('[')
            {
                return DataType::Javascript;
            }
        }

        // 이미지 파일 감지 (구체적인 형식)
        if body.len() >= 2 {
            // PNG 시그니처
            if body.len() >= 8 && &body[0..8] == b"\x89PNG\r\n\x1a\n" {
                return DataType::Image;
            }
            // JPEG 시그니처
            if &body[0..2] == b"\xff\xd8" {
                return DataType::Image;
            }
            // GIF 시그니처
            if body.len() >= 6 && (&body[0..6] == b"GIF87a" || &body[0..6] == b"GIF89a") {
                return DataType::Image;
            }
            // WebP 시그니처
            if body.len() >= 12 && &body[0..4] == b"RIFF" && &body[8..12] == b"WEBP" {
                return DataType::Image;
            }
        }

        // 비디오 파일 감지 (통합)
        if body.len() >= 4 {
            // MP4 시그니처
            if body.len() >= 8 && (&body[4..8] == b"ftyp" || &body[4..8] == b"moov") {
                return DataType::Video;
            }
            // WebM 시그니처
            if &body[0..4] == b"\x1a\x45\xdf\xa3" {
                return DataType::Video;
            }
        }

        // 오디오 파일 감지 (통합)
        if body.len() >= 2 {
            // MP3 시그니처
            if body.len() >= 3 && (&body[0..3] == b"ID3" || &body[0..2] == b"\xff\xfb") {
                return DataType::Audio;
            }
            // WAV 시그니처
            if body.len() >= 12 && &body[0..4] == b"RIFF" && &body[8..12] == b"WAVE" {
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

    // 2. body가 비어있거나 분석 실패 시 Content-Type 헤더 확인
    if let Some(content_type_header) = headers.get("content-type") {
        if let Ok(content_type_str) = content_type_header.to_str() {
            let content_type = content_type_str.to_lowercase();
            if content_type.contains("json") {
                return DataType::Json;
            } else if content_type.contains("xml") {
                return DataType::Xml;
            } else if content_type.contains("html") {
                return DataType::Html;
            } else if content_type.contains("css") {
                return DataType::Css;
            } else if content_type.contains("javascript") {
                return DataType::Javascript;
            } else if content_type.contains("typescript") {
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

    // 3. 기본값
    if body.is_empty() {
        DataType::Empty
    } else {
        DataType::Binary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderMap;

    #[test]
    fn test_json_detection() {
        let headers = HeaderMap::new();
        let body = Bytes::from(r#"{"key": "value"}"#);
        assert_eq!(detect_data_type(&headers, &body), DataType::Json);
    }

    #[test]
    fn test_xml_detection() {
        let headers = HeaderMap::new();
        let body = Bytes::from("<root><item>test</item></root>");
        assert_eq!(detect_data_type(&headers, &body), DataType::Xml);
    }

    #[test]
    fn test_html_detection() {
        let headers = HeaderMap::new();
        let body = Bytes::from("<!DOCTYPE html><html><body>test</body></html>");
        assert_eq!(detect_data_type(&headers, &body), DataType::Html);
    }

    #[test]
    fn test_empty_body() {
        let headers = HeaderMap::new();
        let body = Bytes::new();
        assert_eq!(detect_data_type(&headers, &body), DataType::Empty);
    }

    #[test]
    fn test_data_type_methods() {
        let json_type = DataType::Json;
        assert_eq!(json_type.to_mime_type(), "application/json");
        assert_eq!(json_type.to_monaco_language(), "json");
        assert!(json_type.is_text_based());
        assert!(!json_type.is_binary());

        let image_type = DataType::Image;
        assert_eq!(image_type.to_mime_type(), "image/*");
        assert_eq!(image_type.to_monaco_language(), "plaintext");
        assert!(!image_type.is_text_based());
        assert!(image_type.is_binary());
        assert!(image_type.is_image());
    }

    #[test]
    fn test_css_detection() {
        let headers = HeaderMap::new();

        // CSS @import 테스트
        let css_import = Bytes::from("@import url('style.css'); body { color: red; }");
        assert_eq!(detect_data_type(&headers, &css_import), DataType::Css);

        // CSS 클래스 선택자 테스트
        let css_class = Bytes::from(".my-class { background: blue; }");
        assert_eq!(detect_data_type(&headers, &css_class), DataType::Css);

        // CSS ID 선택자 테스트
        let css_id = Bytes::from("#my-id { margin: 10px; }");
        assert_eq!(detect_data_type(&headers, &css_id), DataType::Css);

        // CSS 속성 테스트
        let css_property = Bytes::from("div { font-size: 16px; padding: 5px; }");
        assert_eq!(detect_data_type(&headers, &css_property), DataType::Css);

        // CSS가 아닌 경우 (JSON과 유사하지만 CSS가 아님)
        let not_css = Bytes::from("{ \"key\": \"value\" }");
        assert_eq!(detect_data_type(&headers, &not_css), DataType::Json);
    }

    #[test]
    fn test_html_vs_xml_detection() {
        let headers = HeaderMap::new();

        // HTML 감지 테스트
        let html = Bytes::from(
            "<!DOCTYPE html><html><head><title>Test</title></head><body>Content</body></html>",
        );
        assert_eq!(detect_data_type(&headers, &html), DataType::Html);

        // XML 감지 테스트
        let xml = Bytes::from("<?xml version=\"1.0\"?><root><item>test</item></root>");
        assert_eq!(detect_data_type(&headers, &xml), DataType::Xml);

        // HTML 태그가 있는 XML (하지만 HTML 구조가 아님)
        let xml_with_html_tags =
            Bytes::from("<data><title>Test</title><content>Data</content></data>");
        assert_eq!(
            detect_data_type(&headers, &xml_with_html_tags),
            DataType::Xml
        );
    }

    #[test]
    fn test_invalid_json_detection() {
        let headers = HeaderMap::new();

        // 유효하지 않은 JSON
        let invalid_json = Bytes::from("{ invalid json }");
        assert_ne!(detect_data_type(&headers, &invalid_json), DataType::Json);

        // JSON 배열이 아닌 배열 형태
        let not_json_array = Bytes::from("[not, a, json, array]");
        assert_ne!(detect_data_type(&headers, &not_json_array), DataType::Json);

        // 유효한 JSON
        let valid_json = Bytes::from(r#"{"name": "test", "value": 123}"#);
        assert_eq!(detect_data_type(&headers, &valid_json), DataType::Json);
    }

    #[test]
    fn test_javascript_detection() {
        let headers = HeaderMap::new();

        // JavaScript 감지 테스트
        let js_code = Bytes::from("function hello() { console.log('Hello World'); }");
        assert_eq!(detect_data_type(&headers, &js_code), DataType::Javascript);

        let js_arrow = Bytes::from("const add = (a, b) => a + b;");
        assert_eq!(detect_data_type(&headers, &js_arrow), DataType::Javascript);

        let js_async = Bytes::from("async function fetchData() { await fetch('/api'); }");
        assert_eq!(detect_data_type(&headers, &js_async), DataType::Javascript);

        // TypeScript도 JavaScript로 감지
        let ts_interface = Bytes::from("interface User { name: string; age: number; }");
        assert_eq!(
            detect_data_type(&headers, &ts_interface),
            DataType::Javascript
        );

        let ts_type = Bytes::from("type Status = 'loading' | 'success' | 'error';");
        assert_eq!(detect_data_type(&headers, &ts_type), DataType::Javascript);

        let ts_class = Bytes::from("class MyClass { private value: string; }");
        assert_eq!(detect_data_type(&headers, &ts_class), DataType::Javascript);

        let ts_generic = Bytes::from(
            "function process<T>(data: T): Promise<T> { return Promise.resolve(data); }",
        );
        assert_eq!(
            detect_data_type(&headers, &ts_generic),
            DataType::Javascript
        );
    }

    #[test]
    fn test_image_detection() {
        let headers = HeaderMap::new();

        // PNG 시그니처 테스트
        let png_data = Bytes::from(vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
        assert_eq!(detect_data_type(&headers, &png_data), DataType::Image);

        // JPEG 시그니처 테스트
        let jpeg_data = Bytes::from(vec![0xFF, 0xD8, 0xFF]);
        assert_eq!(detect_data_type(&headers, &jpeg_data), DataType::Image);

        // SVG 테스트
        let svg_data = Bytes::from(
            "<svg width=\"100\" height=\"100\"><circle cx=\"50\" cy=\"50\" r=\"40\"/></svg>",
        );
        assert_eq!(detect_data_type(&headers, &svg_data), DataType::Image);
    }

    #[test]
    fn test_video_detection() {
        let headers = HeaderMap::new();

        // MP4 시그니처 테스트
        let mp4_data = Bytes::from(vec![0x00, 0x00, 0x00, 0x20, 0x66, 0x74, 0x79, 0x70]);
        assert_eq!(detect_data_type(&headers, &mp4_data), DataType::Video);

        // WebM 시그니처 테스트
        let webm_data = Bytes::from(vec![0x1A, 0x45, 0xDF, 0xA3]);
        assert_eq!(detect_data_type(&headers, &webm_data), DataType::Video);
    }

    #[test]
    fn test_audio_detection() {
        let headers = HeaderMap::new();

        // MP3 시그니처 테스트
        let mp3_data = Bytes::from(vec![0x49, 0x44, 0x33]);
        assert_eq!(detect_data_type(&headers, &mp3_data), DataType::Audio);

        // WAV 시그니처 테스트
        let wav_data = Bytes::from(vec![
            0x52, 0x49, 0x46, 0x46, 0x00, 0x00, 0x00, 0x00, 0x57, 0x41, 0x56, 0x45,
        ]);
        assert_eq!(detect_data_type(&headers, &wav_data), DataType::Audio);
    }

    #[test]
    fn test_document_detection() {
        let headers = HeaderMap::new();

        // PDF 시그니처 테스트
        let pdf_data = Bytes::from(vec![0x25, 0x50, 0x44, 0x46]);
        assert_eq!(detect_data_type(&headers, &pdf_data), DataType::Document);
    }

    #[test]
    fn test_archive_detection() {
        let headers = HeaderMap::new();

        // ZIP 시그니처 테스트
        let zip_data = Bytes::from(vec![0x50, 0x4B, 0x03, 0x04]);
        assert_eq!(detect_data_type(&headers, &zip_data), DataType::Archive);

        // GZIP 시그니처 테스트 (압축 해제 실패 시 Archive 반환)
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

        // 압축된 데이터가 JSON으로 감지되는지 확인
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

        // 압축된 데이터가 HTML로 감지되는지 확인
        assert_eq!(
            detect_data_type(&headers, &Bytes::from(compressed)),
            DataType::Html
        );
    }
}

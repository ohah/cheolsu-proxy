use super::*;
use bytes::Bytes;
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

    let graphql_body = Bytes::from(r#"{"query":"query { user(id: 1) { name } }","variables":{}}"#);
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

    let protobuf_cases = vec!["application/protobuf", "application/x-protobuf"];

    for ct in protobuf_cases {
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
fn test_grpc_detection_by_content_type() {
    use http::HeaderValue;

    let body = Bytes::from(vec![0x08, 0x96, 0x01]);

    let grpc_cases = vec![
        "application/grpc",
        "application/grpc+proto",
        "application/grpc-web",
        "application/grpc-web+proto",
    ];

    for ct in grpc_cases {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_str(ct).unwrap());
        assert_eq!(
            detect_data_type(&headers, &body),
            DataType::Grpc,
            "Content-Type: {} should be detected as Grpc",
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

#[test]
fn test_html_with_inline_svg_not_detected_as_image() {
    use http::HeaderValue;

    // Content-Type: text/html인 HTML에 인라인 SVG가 포함된 경우 → Html이어야 함
    let mut headers = HeaderMap::new();
    headers.insert("content-type", HeaderValue::from_static("text/html"));
    let html_with_svg = Bytes::from(
        r#"<!DOCTYPE html><html><body><svg width="100" height="100"><circle cx="50" cy="50" r="40"/></svg></body></html>"#,
    );
    assert_eq!(detect_data_type(&headers, &html_with_svg), DataType::Html);

    // Content-Type 없이 인라인 SVG가 포함된 HTML → Text (Image가 아님)
    let headers_empty = HeaderMap::new();
    assert_eq!(
        detect_data_type(&headers_empty, &html_with_svg),
        DataType::Text
    );
}

#[test]
fn test_js_with_svg_string_not_detected_as_image() {
    use http::HeaderValue;

    // Content-Type: application/javascript인 JS에 SVG 문자열이 포함된 경우
    let mut headers = HeaderMap::new();
    headers.insert(
        "content-type",
        HeaderValue::from_static("application/javascript"),
    );
    let js_with_svg =
        Bytes::from(r#"const icon = '<svg viewBox="0 0 24 24"><path d="M12 2L2 22h20z"/></svg>';"#);
    assert_eq!(
        detect_data_type(&headers, &js_with_svg),
        DataType::Javascript
    );

    // Content-Type 없이 → Text (Image가 아님)
    let headers_empty = HeaderMap::new();
    assert_eq!(
        detect_data_type(&headers_empty, &js_with_svg),
        DataType::Text
    );
}

#[test]
fn test_pure_svg_detected_as_image() {
    let headers = HeaderMap::new();

    // <svg>로 시작하는 순수 SVG 파일
    let svg = Bytes::from(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><circle cx="50" cy="50" r="40"/></svg>"#,
    );
    assert_eq!(detect_data_type(&headers, &svg), DataType::Image);

    // <?xml> 선언 + SVG (HTML 아님)
    let xml_svg = Bytes::from(
        r#"<?xml version="1.0" encoding="UTF-8"?><svg xmlns="http://www.w3.org/2000/svg"><rect width="100" height="100"/></svg>"#,
    );
    assert_eq!(detect_data_type(&headers, &xml_svg), DataType::Image);

    // <?xml> 선언 + HTML 내 SVG → Image가 아님
    let xml_html_svg =
        Bytes::from(r#"<?xml version="1.0"?><html><body><svg><circle/></svg></body></html>"#);
    assert_ne!(detect_data_type(&headers, &xml_html_svg), DataType::Image);
}

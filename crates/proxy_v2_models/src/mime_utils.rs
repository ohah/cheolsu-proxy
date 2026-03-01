use bytes::Bytes;

use crate::DataType;

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

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn is_media_data_type_returns_true_for_media() {
        assert!(is_media_data_type(&DataType::Image));
        assert!(is_media_data_type(&DataType::Video));
        assert!(is_media_data_type(&DataType::Audio));
    }

    #[test]
    fn is_media_data_type_returns_false_for_non_media() {
        assert!(!is_media_data_type(&DataType::Json));
        assert!(!is_media_data_type(&DataType::Html));
        assert!(!is_media_data_type(&DataType::Css));
        assert!(!is_media_data_type(&DataType::Text));
        assert!(!is_media_data_type(&DataType::Binary));
        assert!(!is_media_data_type(&DataType::Empty));
    }

    #[test]
    fn detect_extension_jpeg() {
        let body = Bytes::from_static(&[0xFF, 0xD8, 0xFF, 0xE0]);
        assert_eq!(detect_file_extension_from_header(&body), Some("jpg"));
    }

    #[test]
    fn detect_extension_png() {
        let body = Bytes::from_static(&[0x89, 0x50, 0x4E, 0x47]);
        assert_eq!(detect_file_extension_from_header(&body), Some("png"));
    }

    #[test]
    fn detect_extension_gif() {
        let body = Bytes::from_static(&[0x47, 0x49, 0x46, 0x38]);
        assert_eq!(detect_file_extension_from_header(&body), Some("gif"));
    }

    #[test]
    fn detect_extension_pdf() {
        let body = Bytes::from_static(&[0x25, 0x50, 0x44, 0x46]);
        assert_eq!(detect_file_extension_from_header(&body), Some("pdf"));
    }

    #[test]
    fn detect_extension_zip() {
        let body = Bytes::from_static(&[0x50, 0x4B, 0x03, 0x04]);
        assert_eq!(detect_file_extension_from_header(&body), Some("zip"));
    }

    #[test]
    fn detect_extension_too_short() {
        let body = Bytes::from_static(&[0xFF, 0xD8]);
        assert_eq!(detect_file_extension_from_header(&body), None);
    }

    #[test]
    fn detect_extension_unknown() {
        let body = Bytes::from_static(&[0x00, 0x01, 0x02, 0x03]);
        assert_eq!(detect_file_extension_from_header(&body), None);
    }

    #[test]
    fn detect_extension_webp() {
        let mut body = vec![0x52, 0x49, 0x46, 0x46, 0x00, 0x00, 0x00, 0x00];
        body.extend_from_slice(b"WEBP");
        let body = Bytes::from(body);
        assert_eq!(detect_file_extension_from_header(&body), Some("webp"));
    }

    #[test]
    fn detect_extension_mp3_id3() {
        let body = Bytes::from_static(&[0x49, 0x44, 0x33, 0x04]);
        assert_eq!(detect_file_extension_from_header(&body), Some("mp3"));
    }

    #[test]
    fn mime_to_extension_roundtrip() {
        let cases = vec![
            ("image/jpeg", "jpg"),
            ("image/png", "png"),
            ("application/json", "json"),
            ("text/html", "html"),
            ("application/pdf", "pdf"),
            ("video/mp4", "mp4"),
            ("audio/mpeg", "mp3"),
        ];
        for (mime, expected_ext) in cases {
            assert_eq!(
                get_extension_from_mime_type(mime),
                expected_ext,
                "mime: {}",
                mime
            );
        }
    }

    #[test]
    fn extension_to_mime_roundtrip() {
        let cases = vec![
            ("jpg", "image/jpeg"),
            ("png", "image/png"),
            ("json", "application/json"),
            ("html", "text/html"),
            ("pdf", "application/pdf"),
            ("mp4", "video/mp4"),
            ("mp3", "audio/mpeg"),
        ];
        for (ext, expected_mime) in cases {
            assert_eq!(
                get_mime_type_from_extension(ext),
                expected_mime,
                "ext: {}",
                ext
            );
        }
    }

    #[test]
    fn unknown_mime_returns_empty_extension() {
        assert_eq!(get_extension_from_mime_type("application/x-custom"), "");
    }

    #[test]
    fn unknown_extension_returns_octet_stream() {
        assert_eq!(
            get_mime_type_from_extension("xyz"),
            "application/octet-stream"
        );
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

mod decompression;
mod detection;

pub use decompression::{decompress_brotli, decompress_gzip};
pub use detection::detect_data_type;

use serde::{Deserialize, Serialize};

/// 데이터 타입을 나타내는 열거형 (MITM 프록시에 최적화)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
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
    /// GraphQL 쿼리/뮤테이션
    GraphQL,
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
    /// Protobuf 바이너리 데이터
    Protobuf,
    /// gRPC 요청/응답
    Grpc,
    /// 바이너리 데이터 (알 수 없는 형식)
    Binary,
    /// 빈 데이터
    Empty,
    /// 알 수 없는 타입
    #[default]
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
            DataType::GraphQL => "application/json",
            DataType::Image => "image/*",
            DataType::Video => "video/*",
            DataType::Audio => "audio/*",
            DataType::Document => "application/pdf",
            DataType::Archive => "application/zip",
            DataType::Protobuf => "application/x-protobuf",
            DataType::Grpc => "application/grpc",
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
            DataType::GraphQL => "graphql",
            DataType::Text => "plaintext",
            DataType::Image
            | DataType::Video
            | DataType::Audio
            | DataType::Document
            | DataType::Archive
            | DataType::Protobuf
            | DataType::Grpc
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
                | DataType::GraphQL
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
                | DataType::Protobuf
                | DataType::Grpc
                | DataType::Binary
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_graphql_type_properties() {
        let gql = DataType::GraphQL;
        assert_eq!(gql.to_monaco_language(), "graphql");
        assert_eq!(gql.to_mime_type(), "application/json");
        assert!(gql.is_text_based());
        assert!(!gql.is_binary());
    }

    #[test]
    fn test_protobuf_type_properties() {
        let pb = DataType::Protobuf;
        assert_eq!(pb.to_mime_type(), "application/x-protobuf");
        assert_eq!(pb.to_monaco_language(), "plaintext");
        assert!(!pb.is_text_based());
        assert!(pb.is_binary());
    }
}

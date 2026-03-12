use http::HeaderMap;

use super::super::DataType;

/// Content-Type 헤더 기반 데이터 타입 감지
pub(crate) fn detect_by_content_type_header(headers: &HeaderMap) -> Option<DataType> {
    let content_type_header = headers.get("content-type")?;
    let content_type_str = content_type_header.to_str().ok()?;
    let content_type = content_type_str.to_lowercase();

    if content_type.contains("application/grpc") {
        Some(DataType::Grpc)
    } else if content_type.contains("application/protobuf")
        || content_type.contains("application/x-protobuf")
    {
        Some(DataType::Protobuf)
    } else if content_type.contains("graphql") {
        Some(DataType::GraphQL)
    } else if content_type.contains("json") {
        Some(DataType::Json)
    } else if content_type.contains("xml") {
        Some(DataType::Xml)
    } else if content_type.contains("html") {
        Some(DataType::Html)
    } else if content_type.contains("css") {
        Some(DataType::Css)
    } else if content_type.contains("javascript") || content_type.contains("typescript") {
        Some(DataType::Javascript)
    } else if content_type.contains("image/") {
        Some(DataType::Image)
    } else if content_type.contains("video/") {
        Some(DataType::Video)
    } else if content_type.contains("audio/") {
        Some(DataType::Audio)
    } else if content_type.contains("pdf") {
        Some(DataType::Document)
    } else if content_type.contains("zip") || content_type.contains("gzip") {
        Some(DataType::Archive)
    } else if content_type.contains("text") {
        Some(DataType::Text)
    } else {
        None
    }
}

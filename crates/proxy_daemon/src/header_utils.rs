use proxyapi_v2::hyper::http::HeaderMap;

/// 바디를 교체할 때 기존 인코딩/길이 관련 헤더를 제거하는 헬퍼 함수.
///
/// `content-length`, `content-encoding`, `transfer-encoding` 헤더를 제거한다.
/// 바디를 새로운 내용으로 교체하면 기존 값이 무효화되므로 반드시 제거해야 한다.
pub(crate) fn clear_content_encoding_headers(headers: &mut HeaderMap) {
    headers.remove("content-length");
    headers.remove("content-encoding");
    headers.remove("transfer-encoding");
}

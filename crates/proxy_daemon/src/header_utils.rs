use proxyapi_v2::hyper::http::HeaderMap;

/// 바디를 교체할 때 기존 인코딩/길이 관련 헤더를 제거하는 헬퍼 함수.
///
/// `content-length`, `content-encoding`, `transfer-encoding` 헤더를 제거한다.
/// 바디를 새로운 내용으로 교체하면 기존 값이 무효화되므로 반드시 제거해야 한다.
///
/// # 참고: 동작 변경 사항
///
/// 기존 `intercept.rs`의 요청 바디 rewrite 로직에서는 `content-length`와
/// `content-encoding` 2개만 제거했으나, 이 헬퍼 도입 시 `transfer-encoding`
/// 제거도 추가되었다. 바디를 통째로 교체하면 chunked 등 기존 transfer-encoding도
/// 무효화되므로, 이는 의도적인 버그 수정에 해당한다.
pub(crate) fn clear_content_encoding_headers(headers: &mut HeaderMap) {
    headers.remove("content-length");
    headers.remove("content-encoding");
    headers.remove("transfer-encoding");
}

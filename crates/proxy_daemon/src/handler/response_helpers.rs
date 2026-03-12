//! Response 빌더 헬퍼 함수 모음
//!
//! `Response::builder()...unwrap_or_else(|_| Response::new(Body::empty()))` 패턴의
//! 반복을 줄이기 위한 유틸리티 함수들을 제공합니다.

use proxyapi_v2::{hyper::http::StatusCode, hyper::Response, Body};
use tracing::warn;

/// 지정된 상태 코드와 문자열 바디로 응답을 생성합니다.
///
/// `Response::builder()` 실패 시 빈 바디의 기본 응답을 반환합니다.
pub(crate) fn error_response(status: StatusCode, body: impl Into<Body>) -> Response<Body> {
    Response::builder()
        .status(status)
        .body(body.into())
        .unwrap_or_else(|e| {
            warn!("error_response 빌더 실패 (status={status}): {e}");
            Response::new(Body::empty())
        })
}

/// 지정된 상태 코드와 빈 바디로 응답을 생성합니다.
pub(crate) fn empty_response(status: StatusCode) -> Response<Body> {
    Response::builder()
        .status(status)
        .body(Body::empty())
        .unwrap_or_else(|e| {
            warn!("empty_response 빌더 실패 (status={status}): {e}");
            Response::new(Body::empty())
        })
}

/// 사전 구성된 `http::response::Builder`를 최종 `Response<Body>`로 변환합니다.
///
/// 빌더에 헤더를 동적으로 추가한 뒤 최종 빌드할 때 사용합니다.
/// 빌드 실패 시 빈 바디의 기본 응답을 반환합니다.
pub(crate) fn build_response(
    builder: proxyapi_v2::hyper::http::response::Builder,
    body: impl Into<Body>,
) -> Response<Body> {
    builder.body(body.into()).unwrap_or_else(|e| {
        warn!("build_response 빌더 실패: {e}");
        Response::new(Body::empty())
    })
}

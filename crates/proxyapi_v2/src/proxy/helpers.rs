use crate::Body;
use hyper::{Response, StatusCode};
use tokio::task::JoinHandle;
use tracing::{Instrument, Span};

pub(super) fn bad_request() -> Response<Body> {
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(Body::empty())
        .expect("Failed to build response")
}

pub(super) fn spawn_with_trace<T: Send + Sync + 'static>(
    fut: impl Future<Output = T> + Send + 'static,
    span: Span,
) -> JoinHandle<T> {
    tokio::spawn(fut.instrument(span))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bad_request_has_400_status() {
        let resp = bad_request();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn bad_request_body_reports_no_size() {
        use hyper::body::Body as HttpBody;
        let resp = bad_request();
        // Empty body should have exact size hint of 0
        let hint = resp.body().size_hint();
        assert_eq!(hint.exact(), Some(0));
    }

    #[tokio::test]
    async fn spawn_with_trace_completes() {
        let span = Span::none();
        let handle = spawn_with_trace(async { 42 }, span);
        let result = handle.await.unwrap();
        assert_eq!(result, 42);
    }
}

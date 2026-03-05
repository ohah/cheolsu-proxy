use crate::Body;
use hyper::{Request, Response, body::Incoming};
use std::{convert::Infallible, pin::Pin, task::Poll};
use tower_service::Service as TowerService;

/// 스트리밍 응답(SSE, chunked)을 감지하고 헤더를 최적화하는 Tower 미들웨어.
///
/// 이 Layer는 Tower의 Service/Layer 패턴을 따르며,
/// `TowerToHyperService`를 통해 hyper에 연결됩니다.
#[derive(Clone)]
pub struct StreamingResponseLayer;

impl<S> tower::Layer<S> for StreamingResponseLayer {
    type Service = StreamingResponseService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        StreamingResponseService { inner }
    }
}

#[derive(Clone)]
pub struct StreamingResponseService<S> {
    inner: S,
}

impl<S> TowerService<Request<Incoming>> for StreamingResponseService<S>
where
    S: TowerService<Request<Incoming>, Response = Response<Body>, Error = Infallible>
        + Clone
        + Send
        + 'static,
    S::Future: Send,
{
    type Response = Response<Body>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Incoming>) -> Self::Future {
        let req_uri = req.uri().clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let res = inner.call(req).await?;
            Ok(optimize_streaming_response(res, &req_uri))
        })
    }
}

/// 스트리밍 응답 헤더를 최적화합니다.
pub(crate) fn optimize_streaming_response(
    res: Response<Body>,
    req_uri: &hyper::Uri,
) -> Response<Body> {
    let content_type = res
        .headers()
        .get("content-type")
        .and_then(|ct| ct.to_str().ok())
        .unwrap_or("");

    let transfer_encoding = res
        .headers()
        .get("transfer-encoding")
        .and_then(|te| te.to_str().ok())
        .unwrap_or("");

    let is_streaming =
        content_type.contains("text/event-stream") || content_type.contains("application/x-ndjson");
    let is_chunked = transfer_encoding.contains("chunked");
    let is_ces_v1_t = req_uri.path().contains("/ces/v1/t");

    if is_streaming || is_chunked || is_ces_v1_t {
        let (mut parts, body) = res.into_parts();

        parts.headers.insert(
            "Cache-Control",
            "no-cache, no-store, must-revalidate".parse().unwrap(),
        );
        parts
            .headers
            .insert("Connection", "keep-alive".parse().unwrap());
        parts
            .headers
            .insert("Transfer-Encoding", "chunked".parse().unwrap());
        parts.headers.remove("content-length");
        parts
            .headers
            .insert("X-Accel-Buffering", "no".parse().unwrap());
        parts
            .headers
            .insert("X-Content-Type-Options", "nosniff".parse().unwrap());

        Response::from_parts(parts, body)
    } else {
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::Uri;

    #[test]
    fn test_non_streaming_response_unchanged() {
        let res = Response::builder()
            .header("content-type", "application/json")
            .body(Body::empty())
            .unwrap();

        let uri: Uri = "http://example.com/api".parse().unwrap();
        let result = optimize_streaming_response(res, &uri);

        assert!(result.headers().get("X-Accel-Buffering").is_none());
    }

    #[test]
    fn test_sse_response_optimized() {
        let res = Response::builder()
            .header("content-type", "text/event-stream")
            .body(Body::empty())
            .unwrap();

        let uri: Uri = "http://example.com/events".parse().unwrap();
        let result = optimize_streaming_response(res, &uri);

        assert_eq!(
            result.headers().get("X-Accel-Buffering").unwrap(),
            "no"
        );
        assert_eq!(
            result.headers().get("Transfer-Encoding").unwrap(),
            "chunked"
        );
        assert!(result.headers().get("content-length").is_none());
    }

    #[test]
    fn test_chunked_response_optimized() {
        let res = Response::builder()
            .header("transfer-encoding", "chunked")
            .body(Body::empty())
            .unwrap();

        let uri: Uri = "http://example.com/data".parse().unwrap();
        let result = optimize_streaming_response(res, &uri);

        assert_eq!(
            result.headers().get("Cache-Control").unwrap(),
            "no-cache, no-store, must-revalidate"
        );
    }

    #[test]
    fn test_ces_v1_t_path_optimized() {
        let res = Response::builder()
            .header("content-type", "application/json")
            .body(Body::empty())
            .unwrap();

        let uri: Uri = "http://example.com/ces/v1/t".parse().unwrap();
        let result = optimize_streaming_response(res, &uri);

        assert_eq!(
            result.headers().get("X-Accel-Buffering").unwrap(),
            "no"
        );
    }
}

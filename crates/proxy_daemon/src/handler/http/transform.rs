use bytes::Bytes;
use proxy_v2_models::{ProxiedRequest, ProxiedResponse, RequestInfo, TimingWaterfall};
use proxyapi_v2::{hyper::http::StatusCode, hyper::Response, Body};
use tracing::error;

use proxyapi_v2::hyper::Request;

use crate::handler::response_helpers;

use super::super::LoggingHandler;

impl LoggingHandler {
    /// 요청과 응답을 묶어서 전송
    pub(crate) async fn send_output(&self) {
        let client_request = self
            .request
            .req
            .as_ref()
            .map(|req| req.clone().for_client(self.config.cache_dir.as_deref()));

        // Waterfall 타이밍 계산: TTFB와 Content Download 분리
        let timing = self.request.request_start.map(|start| {
            let total_ms = start.elapsed().as_millis() as u64;
            let ttfb_ms = self
                .request
                .response_header_time
                .map(|header_time| header_time.duration_since(start).as_millis() as u64);
            let content_download_ms = match (self.request.response_header_time, ttfb_ms) {
                (Some(_), Some(ttfb)) if total_ms > ttfb => Some(total_ms - ttfb),
                _ => None,
            };
            TimingWaterfall {
                total_ms,
                ttfb_ms,
                content_download_ms,
                ..Default::default()
            }
        });

        let client_response = self.request.res.as_ref().map(|res| {
            // id()는 &String을 반환 — 공유 참조로 직접 전달하여 clone 제거
            let request_id = self
                .request
                .req
                .as_ref()
                .map(|r| r.id().as_str())
                .unwrap_or_default();
            res.clone()
                .for_client_with_timing(request_id, self.config.cache_dir.as_deref(), timing)
        });
        let request_info = RequestInfo(client_request, client_response);
        if let Err(e) = self.sender.send(request_info).await {
            error!("[LoggingHandler] 이벤트 전송 실패: {}", e);
        }
    }

    /// Request를 ProxiedRequest로 변환하고 원본 요청을 복원
    pub(super) async fn request_to_proxied_request(
        &self,
        mut req: Request<Body>,
    ) -> (ProxiedRequest, Request<Body>) {
        let body_bytes = {
            let body_mut = req.body_mut();
            match Self::body_to_bytes_from_mut(body_mut).await {
                Ok(bytes) => bytes,
                Err(_) => Bytes::new(),
            }
        };
        // body_mut 가변 참조를 먼저 해제한 뒤 req 필드에 접근하여
        // body_bytes의 마지막 사용처에서 clone 대신 move 가능
        use http_body_util::Full;
        *req.body_mut() = Body::from(Full::new(body_bytes.clone()));

        let proxied_request = ProxiedRequest::new(
            req.method().clone(),
            req.uri().clone(),
            req.version(),
            req.headers().clone(),
            body_bytes,
            chrono::Local::now()
                .timestamp_nanos_opt()
                .unwrap_or_default(),
        );

        (proxied_request, req)
    }

    /// Response를 ProxiedResponse로 변환하고 원본 응답을 복원
    pub(super) async fn response_to_proxied_response(
        &self,
        mut res: Response<Body>,
    ) -> (ProxiedResponse, Response<Body>) {
        let body_bytes = {
            let body_mut = res.body_mut();
            match Self::body_to_bytes_from_mut(body_mut).await {
                Ok(bytes) => bytes,
                Err(_) => Bytes::new(),
            }
        };
        // body_mut 가변 참조를 먼저 해제한 뒤 res 필드에 접근하여
        // body_bytes의 마지막 사용처에서 clone 대신 move 가능
        use http_body_util::Full;
        *res.body_mut() = Body::from(Full::new(body_bytes.clone()));

        let proxied_response = ProxiedResponse::new(
            res.status(),
            res.version(),
            res.headers().clone(),
            body_bytes,
            chrono::Local::now()
                .timestamp_nanos_opt()
                .unwrap_or_default(),
        );

        (proxied_response, res)
    }

    /// BodyMut를 Bytes로 변환하는 헬퍼 함수
    pub(super) async fn body_to_bytes_from_mut(
        body_mut: &mut Body,
    ) -> Result<Bytes, Box<dyn std::error::Error + Send + Sync>> {
        use http_body_util::BodyExt;
        let body_bytes = body_mut.collect().await?.to_bytes();
        Ok(body_bytes)
    }

    pub(super) fn create_response_from_cached_data(&self) -> Response<Body> {
        if let Some(cached_response) = &self.request.res {
            let mut response = Response::builder()
                .status(*cached_response.status())
                .version(*cached_response.version());

            for (key, value) in cached_response.headers() {
                response = response.header(key, value);
            }

            use http_body_util::Full;
            response_helpers::build_response(
                response,
                Body::from(Full::new(cached_response.body().clone())),
            )
        } else {
            response_helpers::error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                Body::from("No cached response data available"),
            )
        }
    }
}

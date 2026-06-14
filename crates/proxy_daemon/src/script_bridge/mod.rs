use proxy_v2_models::ProxiedRequest;
use proxyapi_v2::{
    hyper::http::{HeaderName, HeaderValue, StatusCode},
    hyper::{Request, Response},
    Body,
};

use super::handler::LoggingHandler;

impl LoggingHandler {
    /// ProxiedRequest → ScriptRequest 변환
    pub(crate) fn to_script_request(req: &ProxiedRequest) -> scripting::ScriptRequest {
        let mut headers = std::collections::HashMap::new();
        for (name, value) in req.headers() {
            if let Ok(v) = value.to_str() {
                headers.insert(name.to_string(), v.to_string());
            }
        }
        scripting::ScriptRequest {
            method: req.method().to_string(),
            url: req.uri().to_string(),
            headers,
            body: std::str::from_utf8(req.body()).ok().map(|s| s.to_string()),
        }
    }

    /// hyper Response → ScriptResponse 변환
    pub(crate) fn to_script_response_from_hyper(res: &Response<Body>) -> scripting::ScriptResponse {
        let mut headers = std::collections::HashMap::new();
        for (name, value) in res.headers() {
            if let Ok(v) = value.to_str() {
                headers.insert(name.to_string(), v.to_string());
            }
        }
        scripting::ScriptResponse {
            status: res.status().as_u16(),
            headers,
            body: None, // 응답 body는 스트리밍이라 읽을 수 없음
        }
    }

    /// ScriptRequest의 수정사항을 hyper Request에 적용
    pub(crate) fn apply_script_request_modify(
        mut req: Request<Body>,
        modified: &scripting::ScriptRequest,
    ) -> Request<Body> {
        // 헤더 수정
        req.headers_mut().clear();
        for (name, value) in &modified.headers {
            if let (Ok(hn), Ok(hv)) = (
                HeaderName::from_bytes(name.as_bytes()),
                HeaderValue::from_str(value),
            ) {
                req.headers_mut().insert(hn, hv);
            }
        }
        // URL 수정
        if let Ok(uri) = modified.url.parse() {
            *req.uri_mut() = uri;
        }
        // Body 수정
        if let Some(body) = &modified.body {
            req = req.map(|_| Body::from(body.clone()));
            // 바디를 교체하면 원본 content-length/encoding이 무효화된다. 스크립트가 받은
            // 헤더 맵에 원본 값이 그대로 남아 있을 수 있으므로 제거한다(intercept/breakpoint
            // 경로와 동일하게: 길이 mismatch 및 잘못된 디코딩으로 인한 손상 방지).
            crate::header_utils::clear_content_encoding_headers(req.headers_mut());
        }
        req
    }

    /// ScriptResponse에서 hyper Response 생성
    pub(crate) fn build_script_response(script_res: &scripting::ScriptResponse) -> Response<Body> {
        let mut builder = Response::builder()
            .status(StatusCode::from_u16(script_res.status).unwrap_or(StatusCode::OK))
            .header("x-cheolsu-scripted", "true");

        for (name, value) in &script_res.headers {
            builder = builder.header(name.as_str(), value.as_str());
        }

        let body = script_res.body.clone().unwrap_or_default();
        builder.body(Body::from(body)).unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Script response error"))
                .unwrap_or_else(|_| Response::new(Body::empty()))
        })
    }

    /// ScriptResponse의 수정사항을 hyper Response에 적용
    pub(crate) fn apply_script_response_modify(
        res: Response<Body>,
        modified: &scripting::ScriptResponse,
    ) -> Response<Body> {
        let (mut parts, body) = res.into_parts();

        // 상태 코드 수정
        if let Ok(status) = StatusCode::from_u16(modified.status) {
            parts.status = status;
        }

        // 헤더 수정
        for (name, value) in &modified.headers {
            if let (Ok(hn), Ok(hv)) = (
                HeaderName::from_bytes(name.as_bytes()),
                HeaderValue::from_str(value),
            ) {
                parts.headers.insert(hn, hv);
            }
        }

        if let Some(new_body) = &modified.body {
            // 바디를 교체하면 원본 content-length/content-encoding(예: gzip)이 무효화된다.
            // 제거하지 않으면 클라이언트가 평문을 gzip으로 디코딩하려다 손상되거나 길이가
            // 어긋난다(intercept/breakpoint 경로와 동일).
            crate::header_utils::clear_content_encoding_headers(&mut parts.headers);
            Response::from_parts(parts, Body::from(new_body.clone()))
        } else {
            Response::from_parts(parts, body)
        }
    }
}

#[cfg(test)]
mod tests;

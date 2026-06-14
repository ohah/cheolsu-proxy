use crate::protocol::{BreakpointAction, BreakpointData, BreakpointPhase};
use proxyapi_v2::{hyper::http::StatusCode, hyper::Response, Body};
use tracing::info;

use crate::handler::LoggingHandler;

impl LoggingHandler {
    /// Apply breakpoint check on request phase.
    /// If a breakpoint matches, pauses and waits for resolution.
    /// Returns either the (possibly modified) request, or a Response to short-circuit.
    pub(crate) async fn apply_request_breakpoint(
        &self,
        req: proxyapi_v2::hyper::Request<Body>,
        url: &str,
        method: &str,
        transaction_id: &str,
    ) -> Result<proxyapi_v2::hyper::Request<Body>, Response<Body>> {
        let Some(mgr) = &self.breakpoint_manager else {
            return Ok(req);
        };
        if !mgr.should_break(url, &BreakpointPhase::Request).await {
            return Ok(req);
        }

        info!("[Breakpoint] Request paused: {} {}", method, url);

        let headers: std::collections::HashMap<String, String> = req
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("<binary>").to_string()))
            .collect();

        let data = BreakpointData {
            method: method.to_string(),
            url: url.to_string(),
            headers,
            body: None,
            status: None,
        };

        let action = mgr
            .pause_and_wait(transaction_id, BreakpointPhase::Request, data)
            .await;

        match action {
            BreakpointAction::Forward => Ok(req),
            BreakpointAction::ModifyAndForward {
                headers: new_headers,
                body: new_body,
                ..
            } => {
                let mut req = req;
                if let Some(hdrs) = new_headers {
                    for (name, value) in hdrs {
                        if let (Ok(header_name), Ok(header_value)) = (
                            name.parse::<proxyapi_v2::hyper::http::HeaderName>(),
                            value.parse::<proxyapi_v2::hyper::http::HeaderValue>(),
                        ) {
                            req.headers_mut().insert(header_name, header_value);
                        }
                    }
                }
                if let Some(body) = new_body {
                    use http_body_util::Full;
                    // 바디 교체 시 stale content-length/encoding 제거(요청 행/손상 방지)
                    crate::header_utils::clear_content_encoding_headers(req.headers_mut());
                    *req.body_mut() = Body::from(Full::new(bytes::Bytes::from(body)));
                }
                Ok(req)
            }
            BreakpointAction::Drop | BreakpointAction::Abort => {
                let status = if matches!(action, BreakpointAction::Abort) {
                    StatusCode::BAD_GATEWAY
                } else {
                    StatusCode::SERVICE_UNAVAILABLE
                };
                let response = Response::builder()
                    .status(status)
                    .header("x-cheolsu-breakpoint", "dropped")
                    .body(Body::from("Request dropped by breakpoint"))
                    .unwrap_or_else(|_| Response::new(Body::empty()));
                Err(response)
            }
        }
    }

    /// Apply breakpoint check on response phase.
    pub(crate) async fn apply_response_breakpoint(
        &self,
        res: Response<Body>,
        url: &str,
        method: &str,
        transaction_id: &str,
    ) -> Response<Body> {
        let Some(mgr) = &self.breakpoint_manager else {
            return res;
        };
        if !mgr.should_break(url, &BreakpointPhase::Response).await {
            return res;
        }

        info!("[Breakpoint] Response paused: {} {}", method, url);

        let headers: std::collections::HashMap<String, String> = res
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("<binary>").to_string()))
            .collect();

        let data = BreakpointData {
            method: method.to_string(),
            url: url.to_string(),
            headers,
            body: None,
            status: Some(res.status().as_u16()),
        };

        let action = mgr
            .pause_and_wait(transaction_id, BreakpointPhase::Response, data)
            .await;

        match action {
            BreakpointAction::Forward => res,
            BreakpointAction::ModifyAndForward {
                headers: new_headers,
                body: new_body,
                status: new_status,
            } => {
                let mut res = res;
                if let Some(status) = new_status {
                    if let Ok(status_code) = StatusCode::from_u16(status) {
                        *res.status_mut() = status_code;
                    }
                }
                if let Some(hdrs) = new_headers {
                    for (name, value) in hdrs {
                        if let (Ok(header_name), Ok(header_value)) = (
                            name.parse::<proxyapi_v2::hyper::http::HeaderName>(),
                            value.parse::<proxyapi_v2::hyper::http::HeaderValue>(),
                        ) {
                            res.headers_mut().insert(header_name, header_value);
                        }
                    }
                }
                if let Some(body) = new_body {
                    use http_body_util::Full;
                    res.headers_mut().remove("content-length");
                    res.headers_mut().remove("content-encoding");
                    res.headers_mut().remove("transfer-encoding");
                    *res.body_mut() = Body::from(Full::new(bytes::Bytes::from(body)));
                }
                res
            }
            BreakpointAction::Drop | BreakpointAction::Abort => {
                let status = if matches!(action, BreakpointAction::Abort) {
                    StatusCode::BAD_GATEWAY
                } else {
                    StatusCode::SERVICE_UNAVAILABLE
                };
                Response::builder()
                    .status(status)
                    .header("x-cheolsu-breakpoint", "dropped")
                    .body(Body::from("Response dropped by breakpoint"))
                    .unwrap_or_else(|_| Response::new(Body::empty()))
            }
        }
    }
}

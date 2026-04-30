use super::helpers::{
    compute_headers_size, data_type_mime, encode_body, headers_to_har, http_version_string,
    parse_cookies, parse_query_string, status_text, timestamp_to_iso,
};
use super::types::{
    Har, HarContent, HarCreator, HarEntry, HarLog, HarPostData, HarRequest, HarResponse, HarTimings,
};
use crate::{ClientRequest, ClientResponse, RequestInfo};

fn build_har_request(req: &ClientRequest) -> HarRequest {
    let headers = req.headers();
    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or(data_type_mime(req.data_type()));

    let post_data = req.body().and_then(|body| {
        if body.is_empty() {
            return None;
        }
        let (text, _) = encode_body(body);
        Some(HarPostData {
            mime_type: content_type.to_string(),
            text,
        })
    });

    HarRequest {
        method: req.method().to_string(),
        url: req.uri().to_string(),
        http_version: http_version_string(req.version()),
        cookies: parse_cookies(headers, "cookie"),
        headers: headers_to_har(headers),
        query_string: parse_query_string(&req.uri().to_string()),
        post_data,
        headers_size: compute_headers_size(headers),
        body_size: req.body_size() as i64,
    }
}

fn build_har_response(res: &ClientResponse) -> HarResponse {
    let headers = res.headers();
    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or(data_type_mime(res.data_type()));

    let (text, encoding) = if let Some(body) = res.body() {
        if body.is_empty() {
            (None, None)
        } else {
            let (t, e) = encode_body(body);
            (Some(t), e)
        }
    } else {
        (None, None)
    };

    let redirect_url = headers
        .get("location")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    HarResponse {
        status: res.status().as_u16(),
        status_text: status_text(res.status().as_u16()).to_string(),
        http_version: http_version_string(res.version()),
        cookies: parse_cookies(headers, "set-cookie"),
        headers: headers_to_har(headers),
        content: HarContent {
            size: res.body_size() as i64,
            mime_type: content_type.to_string(),
            text,
            encoding,
        },
        redirect_url,
        headers_size: compute_headers_size(headers),
        body_size: res.body_size() as i64,
    }
}

fn empty_har_response() -> HarResponse {
    HarResponse {
        status: 0,
        status_text: String::new(),
        http_version: "HTTP/1.1".to_string(),
        cookies: Vec::new(),
        headers: Vec::new(),
        content: HarContent {
            size: 0,
            mime_type: "x-unknown".to_string(),
            text: None,
            encoding: None,
        },
        redirect_url: String::new(),
        headers_size: -1,
        body_size: -1,
    }
}

/// `RequestInfo` 슬라이스를 HAR 1.2 JSON으로 변환
pub fn build_har(transactions: &[RequestInfo]) -> Har {
    let entries = transactions
        .iter()
        .filter_map(|info| {
            let req = info.request.as_ref()?;

            let har_request = build_har_request(req);
            let har_response = info
                .response
                .as_ref()
                .map(build_har_response)
                .unwrap_or_else(empty_har_response);

            let req_time = req.time();
            let res_time = info.response.as_ref().map(|r| r.time()).unwrap_or(req_time);
            let elapsed = (res_time - req_time).max(0);

            Some(HarEntry {
                started_date_time: timestamp_to_iso(req_time),
                time: elapsed,
                request: har_request,
                response: har_response,
                cache: serde_json::json!({}),
                timings: HarTimings {
                    send: 0,
                    wait: elapsed,
                    receive: 0,
                },
            })
        })
        .collect();

    Har {
        log: HarLog {
            version: "1.2".to_string(),
            creator: HarCreator {
                name: "Cheolsu Proxy".to_string(),
                version: "0.1.1".to_string(),
            },
            entries,
        },
    }
}

/// `RequestInfo` 슬라이스를 HAR JSON 문자열로 변환
pub fn build_har_json(transactions: &[RequestInfo]) -> Result<String, serde_json::Error> {
    let har = build_har(transactions);
    serde_json::to_string_pretty(&har)
}

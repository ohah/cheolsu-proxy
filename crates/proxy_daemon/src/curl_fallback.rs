use proxy_v2_models::ProxiedRequest;
use proxyapi_v2::{
    hyper::http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    hyper::Response,
    Body,
};
use tracing::debug;

/// curl을 사용해서 직접 요청을 보내고 응답을 받는 함수
pub async fn fallback_with_curl(
    req: &ProxiedRequest,
) -> Result<Response<Body>, Box<dyn std::error::Error>> {
    use std::process::Command;
    use std::str;

    let url = req.uri().to_string();
    let method = req.method().to_string();

    let mut curl_cmd = Command::new("curl");
    curl_cmd
        .arg("-s")
        .arg("-i")
        .arg("-X")
        .arg(&method)
        .arg("--max-time")
        .arg("10")
        .arg("--connect-timeout")
        .arg("5")
        .arg("--insecure");

    for (name, value) in req.headers() {
        let name_str = name.as_str();
        if let Ok(value_str) = value.to_str() {
            if name_str.to_lowercase() != "host" {
                curl_cmd
                    .arg("-H")
                    .arg(format!("{}: {}", name_str, value_str));
            }
        }
    }

    curl_cmd.arg(&url);

    debug!("curl 명령어 실행: {:?}", curl_cmd);

    let output = curl_cmd.output()?;

    if !output.status.success() {
        return Err(format!("curl 실행 실패: {}", output.status).into());
    }

    let response_text = str::from_utf8(&output.stdout)?;
    debug!("curl 응답 길이: {} bytes", response_text.len());

    parse_curl_response(response_text)
}

/// curl 응답을 HTTP Response로 파싱하는 함수
pub fn parse_curl_response(
    response_text: &str,
) -> Result<Response<Body>, Box<dyn std::error::Error>> {
    let lines: Vec<&str> = response_text.lines().collect();
    if lines.is_empty() {
        return Err("빈 응답".into());
    }

    let status_line = lines[0];
    let parts: Vec<&str> = status_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err("잘못된 상태 라인".into());
    }

    let status_code = parts[1].parse::<u16>()?;
    let status = StatusCode::from_u16(status_code)?;

    let mut header_end = 0;
    for (i, line) in lines.iter().enumerate() {
        if line.is_empty() {
            header_end = i;
            break;
        }
    }

    let mut headers = HeaderMap::new();
    for line in &lines[1..header_end] {
        if let Some(colon_pos) = line.find(':') {
            let name = &line[..colon_pos].trim();
            let value = &line[colon_pos + 1..].trim();

            if name.to_lowercase() == "content-length" {
                continue;
            }

            if let (Ok(header_name), Ok(header_value)) =
                (name.parse::<HeaderName>(), value.parse::<HeaderValue>())
            {
                headers.insert(header_name, header_value);
            }
        }
    }

    let body_text = if header_end + 1 < lines.len() {
        lines[header_end + 1..].join("\n")
    } else {
        String::new()
    };

    let mut response = Response::builder()
        .status(status)
        .body(Body::from(body_text))?;

    *response.headers_mut() = headers;

    Ok(response)
}

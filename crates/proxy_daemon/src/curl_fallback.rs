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

    debug!("curl 응답 길이: {} bytes", output.stdout.len());

    parse_curl_response(&output.stdout)
}

/// raw 응답을 헤더 블록과 본문 바이트로 분리한다(\r\n\r\n 우선, 없으면 \n\n).
fn split_head_body(raw: &[u8]) -> (&[u8], &[u8]) {
    let find = |needle: &[u8]| raw.windows(needle.len()).position(|w| w == needle);
    if let Some(pos) = find(b"\r\n\r\n") {
        (&raw[..pos], &raw[pos + 4..])
    } else if let Some(pos) = find(b"\n\n") {
        (&raw[..pos], &raw[pos + 2..])
    } else {
        (raw, &[])
    }
}

/// curl 응답(raw 바이트)을 HTTP Response로 파싱하는 함수.
/// 본문은 원본 바이트를 그대로 보존한다(과거엔 `.lines().join("\n")`으로 \r/바이너리가 손상됐다).
pub fn parse_curl_response(raw: &[u8]) -> Result<Response<Body>, Box<dyn std::error::Error>> {
    use http_body_util::Full;

    let (header_bytes, body_bytes) = split_head_body(raw);
    let header_text = String::from_utf8_lossy(header_bytes);
    let mut lines = header_text.lines();

    let status_line = lines.next().ok_or("빈 응답")?;
    let parts: Vec<&str> = status_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err("잘못된 상태 라인".into());
    }
    let status = StatusCode::from_u16(parts[1].parse::<u16>()?)?;

    let mut headers = HeaderMap::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        if let Some(colon_pos) = line.find(':') {
            let name = line[..colon_pos].trim();
            let value = line[colon_pos + 1..].trim();

            // 본문을 원본 바이트 그대로 전달하므로 content-length는 무효(hyper가 재계산),
            // transfer-encoding(chunked)도 curl이 이미 디코드했으므로 제거한다.
            // content-encoding은 본문이 아직 인코딩된 상태일 수 있으므로 보존한다.
            let lower = name.to_ascii_lowercase();
            if lower == "content-length" || lower == "transfer-encoding" {
                continue;
            }

            if let (Ok(header_name), Ok(header_value)) =
                (name.parse::<HeaderName>(), value.parse::<HeaderValue>())
            {
                headers.insert(header_name, header_value);
            }
        }
    }

    let mut response = Response::builder()
        .status(status)
        .body(Body::from(Full::new(bytes::Bytes::from(
            body_bytes.to_vec(),
        ))))?;

    *response.headers_mut() = headers;

    Ok(response)
}

use proxy_daemon::{diff_headers, diff_json, diff_text, BodyDiff, HeaderDiff};

pub(crate) fn diff_part(
    headers_a: &http::HeaderMap,
    headers_b: &http::HeaderMap,
    body_a: Option<&[u8]>,
    body_b: Option<&[u8]>,
    size_a: usize,
    size_b: usize,
    file_path_a: &Option<String>,
    file_path_b: &Option<String>,
    data_type_a: &proxy_v2_models::DataType,
    data_type_b: &proxy_v2_models::DataType,
) -> (Vec<HeaderDiff>, Option<BodyDiff>) {
    let extract = |h: &http::HeaderMap| -> Vec<(String, String)> {
        h.iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("<binary>").to_string()))
            .collect()
    };
    let header_diffs = diff_headers(&extract(headers_a), &extract(headers_b));
    let body_diff = compute_body_diff(
        body_a,
        body_b,
        size_a,
        size_b,
        file_path_a,
        file_path_b,
        data_type_a,
        data_type_b,
    );
    (header_diffs, body_diff)
}

pub(crate) fn compute_body_diff(
    body_a: Option<&[u8]>,
    body_b: Option<&[u8]>,
    size_a: usize,
    size_b: usize,
    file_path_a: &Option<String>,
    file_path_b: &Option<String>,
    data_type_a: &proxy_v2_models::DataType,
    data_type_b: &proxy_v2_models::DataType,
) -> Option<BodyDiff> {
    let bytes_a = body_a
        .map(|b| b.to_vec())
        .or_else(|| {
            file_path_a.as_ref().and_then(|p| {
                std::fs::read(p)
                    .map_err(|e| {
                        tracing::warn!("Failed to read body file {}: {}", p, e);
                        e
                    })
                    .ok()
            })
        })
        .unwrap_or_default();
    let bytes_b = body_b
        .map(|b| b.to_vec())
        .or_else(|| {
            file_path_b.as_ref().and_then(|p| {
                std::fs::read(p)
                    .map_err(|e| {
                        tracing::warn!("Failed to read body file {}: {}", p, e);
                        e
                    })
                    .ok()
            })
        })
        .unwrap_or_default();

    if bytes_a == bytes_b {
        return None;
    }

    let is_json = matches!(
        data_type_a,
        proxy_v2_models::DataType::Json | proxy_v2_models::DataType::GraphQL
    ) && matches!(
        data_type_b,
        proxy_v2_models::DataType::Json | proxy_v2_models::DataType::GraphQL
    );

    if is_json {
        if let (Ok(text_a), Ok(text_b)) =
            (std::str::from_utf8(&bytes_a), std::str::from_utf8(&bytes_b))
        {
            if let (Ok(json_a), Ok(json_b)) = (
                serde_json::from_str::<serde_json::Value>(text_a),
                serde_json::from_str::<serde_json::Value>(text_b),
            ) {
                return Some(diff_json(&json_a, &json_b));
            }
        }
    }

    let is_text = data_type_a.is_text_based() && data_type_b.is_text_based();
    if is_text {
        if let (Ok(text_a), Ok(text_b)) =
            (std::str::from_utf8(&bytes_a), std::str::from_utf8(&bytes_b))
        {
            return Some(diff_text(text_a, text_b));
        }
    }

    Some(BodyDiff::Binary {
        old_size: size_a,
        new_size: size_b,
    })
}

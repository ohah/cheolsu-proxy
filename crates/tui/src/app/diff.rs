/// 트래픽 비교(Diff) 로직
use proxy_daemon::{
    diff_headers, diff_json, diff_text, format_diff_text, BodyDiff, TrafficDiff,
    TransactionPartDiff,
};

use super::App;

impl App {
    pub(crate) fn run_diff(&mut self, idx_a: usize, idx_b: usize) {
        let (Some(txn_a), Some(txn_b)) =
            (self.transactions.get(idx_a), self.transactions.get(idx_b))
        else {
            self.set_status("Diff failed: transaction not found");
            return;
        };

        let request_diff = match (&txn_a.0, &txn_b.0) {
            (Some(req_a), Some(req_b)) => {
                let method_diff = if req_a.method() != req_b.method() {
                    Some((req_a.method().to_string(), req_b.method().to_string()))
                } else {
                    None
                };

                let url_diff = if req_a.uri().to_string() != req_b.uri().to_string() {
                    Some((req_a.uri().to_string(), req_b.uri().to_string()))
                } else {
                    None
                };

                let headers_a: Vec<(String, String)> = req_a
                    .headers()
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("<binary>").to_string()))
                    .collect();
                let headers_b: Vec<(String, String)> = req_b
                    .headers()
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("<binary>").to_string()))
                    .collect();
                let header_diffs = diff_headers(&headers_a, &headers_b);

                let body_diff = Self::compute_body_diff(
                    req_a.body().map(|b| b.as_ref()),
                    req_b.body().map(|b| b.as_ref()),
                    req_a.body_size(),
                    req_b.body_size(),
                    req_a.data_type(),
                    req_b.data_type(),
                );

                if method_diff.is_none()
                    && url_diff.is_none()
                    && header_diffs.is_empty()
                    && body_diff.is_none()
                {
                    None
                } else {
                    Some(TransactionPartDiff {
                        method_diff,
                        url_diff,
                        status_diff: None,
                        header_diffs,
                        body_diff,
                    })
                }
            }
            _ => None,
        };

        let response_diff = match (&txn_a.1, &txn_b.1) {
            (Some(res_a), Some(res_b)) => {
                let status_diff = if res_a.status() != res_b.status() {
                    Some((res_a.status().as_u16(), res_b.status().as_u16()))
                } else {
                    None
                };

                let headers_a: Vec<(String, String)> = res_a
                    .headers()
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("<binary>").to_string()))
                    .collect();
                let headers_b: Vec<(String, String)> = res_b
                    .headers()
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("<binary>").to_string()))
                    .collect();
                let header_diffs = diff_headers(&headers_a, &headers_b);

                let body_diff = Self::compute_body_diff(
                    res_a.body().map(|b| b.as_ref()),
                    res_b.body().map(|b| b.as_ref()),
                    res_a.body_size(),
                    res_b.body_size(),
                    res_a.data_type(),
                    res_b.data_type(),
                );

                if status_diff.is_none() && header_diffs.is_empty() && body_diff.is_none() {
                    None
                } else {
                    Some(TransactionPartDiff {
                        method_diff: None,
                        url_diff: None,
                        status_diff,
                        header_diffs,
                        body_diff,
                    })
                }
            }
            _ => None,
        };

        let diff = TrafficDiff {
            request_diff,
            response_diff,
        };

        self.diff_result = Some(format_diff_text(&diff));
        self.show_diff = true;
        self.diff_scroll = 0;
        self.diff_mark = None;
    }

    fn compute_body_diff(
        body_a: Option<&[u8]>,
        body_b: Option<&[u8]>,
        size_a: usize,
        size_b: usize,
        data_type_a: &proxy_v2_models::DataType,
        data_type_b: &proxy_v2_models::DataType,
    ) -> Option<BodyDiff> {
        let bytes_a = body_a.map(|b| b.to_vec()).unwrap_or_default();
        let bytes_b = body_b.map(|b| b.to_vec()).unwrap_or_default();

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
}

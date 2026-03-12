use super::state::{base64_encode, base64_engine, is_hop_by_hop_header};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};
use tokio::sync::Mutex;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct ReplayRequestParams {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct ReplayResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub body_size: usize,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct SequenceReplayResult {
    pub index: usize,
    pub url: String,
    pub method: String,
    pub response: Option<ReplayResponse>,
    pub error: Option<String>,
}

#[tauri::command]
pub(crate) async fn replay_request(params: ReplayRequestParams) -> Result<ReplayResponse, String> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| format!("HTTP 클라이언트 생성 실패: {}", e))?;

    tracing::debug!("replay 요청: SSL 인증서 검증 비활성화 (프록시 테스트 목적)");

    let method: reqwest::Method = params
        .method
        .parse()
        .map_err(|e| format!("잘못된 HTTP 메서드: {}", e))?;

    let mut request_builder = client.request(method, &params.url);

    for (key, value) in &params.headers {
        if is_hop_by_hop_header(key) {
            continue;
        }
        request_builder = request_builder.header(key.as_str(), value.as_str());
    }

    if let Some(body) = params.body {
        request_builder = request_builder.body(body);
    }

    let start = std::time::Instant::now();
    let response = request_builder
        .send()
        .await
        .map_err(|e| format!("요청 전송 실패: {}", e))?;
    let elapsed_ms = start.elapsed().as_millis() as u64;

    let status = response.status().as_u16();
    let headers: HashMap<String, String> = response
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let body_bytes = response
        .bytes()
        .await
        .map_err(|e| format!("응답 본문 읽기 실패: {}", e))?;
    let body_size = body_bytes.len();

    let body = if body_size == 0 {
        None
    } else {
        Some(String::from_utf8(body_bytes.to_vec()).unwrap_or_else(|_| {
            let engine = base64_engine();
            format!("base64:{}", base64_encode(&engine, &body_bytes))
        }))
    };

    Ok(ReplayResponse {
        status,
        headers,
        body,
        body_size,
        elapsed_ms,
    })
}

#[tauri::command]
pub(crate) async fn replay_sequence(
    requests: Vec<ReplayRequestParams>,
) -> Result<Vec<SequenceReplayResult>, String> {
    let mut results = Vec::new();

    for (index, params) in requests.into_iter().enumerate() {
        let url = params.url.clone();
        let method = params.method.clone();

        match replay_request(params).await {
            Ok(response) => {
                results.push(SequenceReplayResult {
                    index,
                    url,
                    method,
                    response: Some(response),
                    error: None,
                });
            }
            Err(e) => {
                results.push(SequenceReplayResult {
                    index,
                    url,
                    method,
                    response: None,
                    error: Some(e),
                });
            }
        }
    }

    Ok(results)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct AdvancedRepeatParams {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub iterations: usize,
    pub concurrency: usize,
    pub delay_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct AdvancedRepeatProgress {
    pub completed: usize,
    pub total: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub last_status: Option<u16>,
    pub last_elapsed_ms: Option<u64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct AdvancedRepeatResult {
    pub total: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub min_time_ms: u64,
    pub max_time_ms: u64,
    pub avg_time_ms: f64,
    pub total_time_ms: u64,
    pub requests_per_second: f64,
    pub status_codes: HashMap<u16, usize>,
}

#[tauri::command]
pub(crate) async fn advanced_repeat<R: Runtime>(
    app: AppHandle<R>,
    params: AdvancedRepeatParams,
) -> Result<AdvancedRepeatResult, String> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::Semaphore;

    let iterations = params.iterations.max(1).min(10000);
    let concurrency = params.concurrency.max(1).min(100);
    let delay_ms = params.delay_ms;

    let client = Arc::new(
        reqwest::Client::builder()
            .no_proxy()
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(|e| format!("HTTP 클라이언트 생성 실패: {}", e))?,
    );

    tracing::warn!("replay 요청: SSL 인증서 검증 비활성화 (프록시 테스트 목적)");

    let method: reqwest::Method = params
        .method
        .parse()
        .map_err(|e| format!("잘못된 HTTP 메서드: {}", e))?;

    let filtered_headers: Arc<HashMap<String, String>> = Arc::new(
        params
            .headers
            .into_iter()
            .filter(|(key, _)| !is_hop_by_hop_header(key))
            .collect(),
    );
    let url: Arc<str> = Arc::from(params.url.as_str());
    let body: Arc<Option<String>> = Arc::new(params.body);

    let semaphore = Arc::new(Semaphore::new(concurrency));
    let success_count = Arc::new(AtomicUsize::new(0));
    let failure_count = Arc::new(AtomicUsize::new(0));
    let completed = Arc::new(AtomicUsize::new(0));
    let elapsed_times = Arc::new(Mutex::new(Vec::with_capacity(iterations)));
    let status_codes = Arc::new(Mutex::new(HashMap::<u16, usize>::new()));

    let total_start = std::time::Instant::now();

    let mut handles = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| format!("세마포어 획득 실패: {}", e))?;

        let client = client.clone();
        let method = method.clone();
        let url = Arc::clone(&url);
        let headers = Arc::clone(&filtered_headers);
        let body = Arc::clone(&body);
        let success_count = success_count.clone();
        let failure_count = failure_count.clone();
        let completed = completed.clone();
        let elapsed_times = elapsed_times.clone();
        let status_codes = status_codes.clone();
        let app = app.clone();
        let total = iterations;

        let handle = tokio::spawn(async move {
            let mut request_builder = client.request(method, &*url);

            for (key, value) in &*headers {
                request_builder = request_builder.header(key.as_str(), value.as_str());
            }

            if let Some(body) = body.as_ref() {
                request_builder = request_builder.body(body.clone());
            }

            let start = std::time::Instant::now();
            let result = request_builder.send().await;
            let elapsed_ms = start.elapsed().as_millis() as u64;

            let (_is_success, status) = match result {
                Ok(response) => {
                    let status = response.status().as_u16();
                    let _ = response.bytes().await;
                    let ok = (200..300).contains(&status);
                    if ok {
                        success_count.fetch_add(1, Ordering::Relaxed);
                    } else {
                        failure_count.fetch_add(1, Ordering::Relaxed);
                    }
                    {
                        let mut codes = status_codes.lock().await;
                        *codes.entry(status).or_insert(0) += 1;
                    }
                    (ok, Some(status))
                }
                Err(_) => {
                    failure_count.fetch_add(1, Ordering::Relaxed);
                    (false, None)
                }
            };

            {
                let mut times = elapsed_times.lock().await;
                times.push(elapsed_ms);
            }

            let done = completed.fetch_add(1, Ordering::Relaxed) + 1;

            let _ = app.emit(
                "advanced_repeat_progress",
                AdvancedRepeatProgress {
                    completed: done,
                    total,
                    success_count: success_count.load(Ordering::Relaxed),
                    failure_count: failure_count.load(Ordering::Relaxed),
                    last_status: status,
                    last_elapsed_ms: Some(elapsed_ms),
                },
            );

            drop(permit);

            if delay_ms > 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        if handle.await.is_err() {
            failure_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    let total_time_ms = total_start.elapsed().as_millis() as u64;
    let times = elapsed_times.lock().await;
    let codes = status_codes.lock().await;

    let min_time_ms = times.iter().copied().min().unwrap_or(0);
    let max_time_ms = times.iter().copied().max().unwrap_or(0);
    let avg_time_ms = if times.is_empty() {
        0.0
    } else {
        times.iter().sum::<u64>() as f64 / times.len() as f64
    };
    let requests_per_second = if total_time_ms > 0 {
        iterations as f64 / (total_time_ms as f64 / 1000.0)
    } else {
        0.0
    };

    Ok(AdvancedRepeatResult {
        total: iterations,
        success_count: success_count.load(Ordering::Relaxed),
        failure_count: failure_count.load(Ordering::Relaxed),
        min_time_ms,
        max_time_ms,
        avg_time_ms,
        total_time_ms,
        requests_per_second,
        status_codes: codes.clone(),
    })
}

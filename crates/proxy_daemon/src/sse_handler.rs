use bytes::Bytes;
use futures_util::stream::StreamExt;
use http_body_util::{BodyExt, StreamBody};
use proxy_v2_models::{ProxiedResponse, SseConnectionEvent, SseEventInfo, SseParser};
use proxyapi_v2::{hyper::Response, Body};
use tokio_stream::wrappers::ReceiverStream;
use tracing::error;

use crate::handler::LoggingHandler;

/// SSE 이벤트 (이벤트 또는 연결 상태)
#[derive(Clone, Debug)]
pub enum SseEvent {
    Event(SseEventInfo),
    Connection(SseConnectionEvent),
}

/// SSE 상태 관리
#[derive(Clone)]
pub(crate) struct SseState {
    pub(crate) sse_sender: Option<tokio::sync::mpsc::Sender<SseEvent>>,
    pub(crate) sse_sequence: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl LoggingHandler {
    /// SSE(Server-Sent Events) 응답을 스트리밍 처리합니다.
    pub(crate) fn handle_sse_streaming(&mut self, res: Response<Body>) -> Response<Body> {
        let (parts, body) = res.into_parts();

        let (tx, rx) = tokio::sync::mpsc::channel(4);

        let stream = ReceiverStream::new(rx).map(Ok::<_, proxyapi_v2::Error>);
        let stream_body = StreamBody::new(stream);

        let response_for_client = Response::from_parts(parts.clone(), Body::from(stream_body));

        let mut handler_clone = self.clone();

        // SSE 연결 ID (타임스탬프 + 시퀀스) 및 URI 추출
        let connection_id = format!(
            "sse-{}-{}",
            chrono::Local::now().timestamp_millis(),
            self.sse
                .sse_sequence
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );
        let connection_uri = self
            .request
            .req
            .as_ref()
            .map(|r| r.uri().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let sse_sender = self.sse.sse_sender.clone();
        let sse_sequence = self.sse.sse_sequence.clone();
        let script_handle = self.intercept.script_handle.clone();

        // SSE Connected 이벤트 전송
        if let Some(ref sender) = sse_sender {
            let event = SseConnectionEvent::Connected {
                connection_id: connection_id.clone(),
                uri: connection_uri,
                time: chrono::Local::now()
                    .timestamp_nanos_opt()
                    .unwrap_or_default(),
            };
            let _ = sender.try_send(SseEvent::Connection(event));
        }

        let connection_id_clone = connection_id.clone();
        tokio::spawn(async move {
            let mut body_stream = body;
            let mut collected_chunks = Vec::new();
            let mut sse_parser = SseParser::new();

            while let Some(frame_result) = body_stream.frame().await {
                match frame_result {
                    Ok(frame) => {
                        if let Some(data) = frame.data_ref() {
                            collected_chunks.extend_from_slice(data);

                            // SSE 파서에 청크 전달
                            if let Ok(chunk_str) = std::str::from_utf8(data) {
                                let parsed_events = sse_parser.feed(chunk_str);
                                for parsed in parsed_events {
                                    // 스크립팅 훅 호출
                                    let sse_msg = scripting::ScriptSseEvent {
                                        connection_id: connection_id_clone.clone(),
                                        event_type: parsed.event_type.clone(),
                                        data: parsed.data.clone(),
                                        id: parsed.id.clone(),
                                    };

                                    if let Ok(scripting::SseAction::Drop) =
                                        script_handle.invoke_on_sse_event(&sse_msg).await
                                    {
                                        continue;
                                    }

                                    if let Some(ref sender) = sse_sender {
                                        let seq = sse_sequence
                                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                        let event_info = SseEventInfo {
                                            connection_id: connection_id_clone.clone(),
                                            sequence: seq,
                                            event_type: parsed.event_type,
                                            data: parsed.data.clone(),
                                            id: parsed.id,
                                            retry: parsed.retry,
                                            size: parsed.data.len(),
                                            time: chrono::Local::now()
                                                .timestamp_nanos_opt()
                                                .unwrap_or_default(),
                                        };
                                        let _ = sender.try_send(SseEvent::Event(event_info));
                                    }
                                }
                            }
                        }

                        if tx.send(frame).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        error!("[SSE Stream] Error reading from upstream: {:?}", e);
                        break;
                    }
                }
            }

            // SSE Disconnected 이벤트 전송
            if let Some(ref sender) = sse_sender {
                let event = SseConnectionEvent::Disconnected {
                    connection_id: connection_id_clone.clone(),
                    time: chrono::Local::now()
                        .timestamp_nanos_opt()
                        .unwrap_or_default(),
                };
                let _ = sender.try_send(SseEvent::Connection(event));
            }

            let proxied_response = ProxiedResponse::new(
                parts.status,
                parts.version,
                parts.headers,
                Bytes::from(collected_chunks),
                chrono::Local::now()
                    .timestamp_nanos_opt()
                    .unwrap_or_default(),
            );

            handler_clone.request.res = Some(proxied_response);
            handler_clone.send_output().await;
        });

        response_for_client
    }
}

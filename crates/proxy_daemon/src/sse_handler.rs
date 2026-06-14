use bytes::Bytes;
use futures_util::stream::StreamExt;
use http_body_util::{BodyExt, StreamBody};
use proxy_v2_models::{ProxiedResponse, SseConnectionEvent, SseEventInfo, SseParser};
use proxyapi_v2::{hyper::Response, Body};
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, error};

use crate::handler::LoggingHandler;

/// 누적 버퍼에서 디코딩 가능한 최대 UTF-8 prefix를 String으로 반환하고, 소비한 바이트 수를 돌려준다.
/// 끝의 불완전한 멀티바이트 시퀀스는 소비하지 않고 남겨, 다음 프레임과 이어붙인다.
/// 중간에 실제로 잘못된 바이트가 있으면 lossy로 처리하고 소비해 스톨을 방지한다.
fn decode_sse_prefix(buf: &[u8]) -> (String, usize) {
    match std::str::from_utf8(buf) {
        Ok(s) => (s.to_string(), buf.len()),
        Err(e) => {
            let valid = e.valid_up_to();
            match e.error_len() {
                // 끝에 불완전한 시퀀스 → 유효 prefix만 소비하고 나머지는 다음 프레임 대기
                None => (String::from_utf8_lossy(&buf[..valid]).into_owned(), valid),
                // 중간의 잘못된 바이트 → 잘못된 부분까지 lossy로 소비(파서 스톨 방지)
                Some(bad) => (
                    String::from_utf8_lossy(&buf[..valid + bad]).into_owned(),
                    valid + bad,
                ),
            }
        }
    }
}

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
            if let Err(e) = sender.try_send(SseEvent::Connection(event)) {
                debug!("[SSE] 연결 이벤트 전송 실패 (채널 가득 참): {}", e);
            }
        }

        let connection_id_clone = connection_id.clone();
        tokio::spawn(async move {
            let mut body_stream = body;
            let mut collected_chunks = Vec::new();
            let mut sse_parser = SseParser::new();
            // 프레임 경계에서 분할된 멀티바이트 UTF-8을 이어붙이기 위한 캐리오버 버퍼
            let mut pending: Vec<u8> = Vec::new();

            while let Some(frame_result) = body_stream.frame().await {
                match frame_result {
                    Ok(frame) => {
                        if let Some(data) = frame.data_ref() {
                            collected_chunks.extend_from_slice(data);

                            // SSE 파서에 청크 전달.
                            // 프레임 경계에서 멀티바이트 UTF-8이 쪼개지면 from_utf8가 실패해
                            // 해당 바이트가 영구 유실(파서 desync)되므로, 유효 prefix만 디코딩하고
                            // 불완전한 잔여 바이트는 다음 프레임으로 넘긴다.
                            pending.extend_from_slice(data);
                            let (decoded, consumed) = decode_sse_prefix(&pending);
                            pending.drain(..consumed);
                            if !decoded.is_empty() {
                                let parsed_events = sse_parser.feed(&decoded);
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
                                        if let Err(e) = sender.try_send(SseEvent::Event(event_info))
                                        {
                                            debug!("[SSE] 이벤트 전송 실패 (채널 가득 참): {}", e);
                                        }
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
                if let Err(e) = sender.try_send(SseEvent::Connection(event)) {
                    debug!("[SSE] 연결 이벤트 전송 실패 (채널 가득 참): {}", e);
                }
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

#[cfg(test)]
mod tests {
    use super::decode_sse_prefix;

    #[test]
    fn decode_sse_prefix_full_valid() {
        let (decoded, consumed) = decode_sse_prefix("hello".as_bytes());
        assert_eq!(decoded, "hello");
        assert_eq!(consumed, 5);
    }

    #[test]
    fn decode_sse_prefix_handles_split_multibyte() {
        // "한"(U+D55C) = [0xED, 0x95, 0x9C]
        let full = "ab한".as_bytes().to_vec();
        // 마지막 멀티바이트의 첫 2바이트만 도착한 상태
        let (decoded, consumed) = decode_sse_prefix(&full[..full.len() - 1]);
        assert_eq!(decoded, "ab", "유효 prefix만 디코딩되어야 함");
        assert_eq!(consumed, 2, "불완전 시퀀스는 소비하지 않고 보존해야 함");

        // 나머지 바이트가 도착하면 완성됨
        let remaining = &full[consumed..];
        let (decoded2, consumed2) = decode_sse_prefix(remaining);
        assert_eq!(decoded2, "한");
        assert_eq!(consumed2, remaining.len());
    }
}

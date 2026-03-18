use serde::{Deserialize, Serialize};

/// SSE 이벤트 정보 (UI 전달용)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseEventInfo {
    pub connection_id: String,
    pub sequence: u64,
    pub event_type: Option<String>,
    pub data: String,
    pub id: Option<String>,
    pub retry: Option<u64>,
    pub size: usize,
    pub time: i64,
}

/// SSE 연결 상태 이벤트
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum SseConnectionEvent {
    #[serde(rename = "connected")]
    Connected {
        connection_id: String,
        uri: String,
        time: i64,
    },
    #[serde(rename = "disconnected")]
    Disconnected { connection_id: String, time: i64 },
}

/// 증분 SSE 파서
pub struct SseParser {
    buffer: String,
    event_type: Option<String>,
    data_lines: Vec<String>,
    last_event_id: Option<String>,
    retry: Option<u64>,
}

/// 파싱된 SSE 이벤트
#[derive(Debug, Clone)]
pub struct ParsedSseEvent {
    pub event_type: Option<String>,
    pub data: String,
    pub id: Option<String>,
    pub retry: Option<u64>,
    pub raw: String,
}

impl SseParser {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            event_type: None,
            data_lines: Vec::new(),
            last_event_id: None,
            retry: None,
        }
    }

    /// 새 청크를 추가하고 완성된 이벤트들을 반환
    pub fn feed(&mut self, chunk: &str) -> Vec<ParsedSseEvent> {
        self.buffer.push_str(chunk);
        let mut events = Vec::new();

        // 빈 줄(\n\n)을 기준으로 이벤트 경계를 찾는다
        loop {
            // 이벤트 경계 찾기: \n\n, \r\n\r\n, \r\r
            let boundary = self.find_event_boundary();
            if let Some((end_of_event, boundary_len)) = boundary {
                let event_block = self.buffer[..end_of_event].to_string();
                self.buffer = self.buffer[end_of_event + boundary_len..].to_string();

                // 이벤트 블록의 각 줄을 처리
                for line in event_block.lines() {
                    self.process_line(line);
                }

                // data가 있으면 이벤트를 디스패치
                if !self.data_lines.is_empty() {
                    let data = self.data_lines.join("\n");
                    events.push(ParsedSseEvent {
                        event_type: self.event_type.take(),
                        data,
                        id: self.last_event_id.clone(),
                        retry: self.retry.take(),
                        raw: event_block,
                    });
                    self.data_lines.clear();
                } else {
                    // data가 없어도 필드를 리셋
                    self.event_type = None;
                    self.retry = None;
                    self.data_lines.clear();
                }
            } else {
                break;
            }
        }

        events
    }

    /// 버퍼에서 이벤트 경계(빈 줄)를 찾는다
    fn find_event_boundary(&self) -> Option<(usize, usize)> {
        let bytes = self.buffer.as_bytes();
        let len = bytes.len();

        let mut i = 0;
        while i < len {
            if bytes[i] == b'\n' {
                // \n\n
                if i + 1 < len && bytes[i + 1] == b'\n' {
                    return Some((i, 2));
                }
            } else if bytes[i] == b'\r' {
                if i + 1 < len && bytes[i + 1] == b'\n' {
                    // \r\n\r\n
                    if i + 3 < len && bytes[i + 2] == b'\r' && bytes[i + 3] == b'\n' {
                        return Some((i, 4));
                    }
                } else {
                    // \r\r
                    if i + 1 < len && bytes[i + 1] == b'\r' {
                        return Some((i, 2));
                    }
                }
            }
            i += 1;
        }
        None
    }

    /// SSE 사양에 따라 한 줄을 처리한다
    fn process_line(&mut self, line: &str) {
        // 주석: ':'로 시작하는 줄은 무시
        if line.starts_with(':') {
            return;
        }

        if let Some(colon_pos) = line.find(':') {
            let field = &line[..colon_pos];
            // 콜론 뒤에 스페이스가 있으면 건너뛴다
            let value = if line.len() > colon_pos + 1 && line.as_bytes()[colon_pos + 1] == b' ' {
                &line[colon_pos + 2..]
            } else {
                &line[colon_pos + 1..]
            };

            match field {
                "data" => {
                    self.data_lines.push(value.to_string());
                }
                "event" => {
                    self.event_type = Some(value.to_string());
                }
                // id 필드에 NUL 문자가 있으면 무시 (SSE 사양)
                "id" if !value.contains('\0') => {
                    self.last_event_id = Some(value.to_string());
                }
                "retry" => {
                    if let Ok(ms) = value.parse::<u64>() {
                        self.retry = Some(ms);
                    }
                }
                _ => {
                    // 알 수 없는 필드 - 무시
                }
            }
        } else if !line.is_empty() {
            // 콜론이 없으면 전체 줄을 필드 이름으로 취급, 값은 빈 문자열
            // SSE 사양: "If the line is not empty but does not contain a U+003A COLON character"
            match line {
                "data" => {
                    self.data_lines.push(String::new());
                }
                "event" => {
                    self.event_type = Some(String::new());
                }
                "id" => {
                    self.last_event_id = Some(String::new());
                }
                "retry" => {
                    // retry에 값이 없으면 무시
                }
                _ => {}
            }
        }
    }
}

impl Default for SseParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_sse_event() {
        let mut parser = SseParser::new();
        let events = parser.feed("data: hello\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "hello");
        assert_eq!(events[0].event_type, None);
    }

    #[test]
    fn test_sse_event_with_type() {
        let mut parser = SseParser::new();
        let events = parser.feed("event: message\ndata: hello\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "hello");
        assert_eq!(events[0].event_type, Some("message".to_string()));
    }

    #[test]
    fn test_sse_multiline_data() {
        let mut parser = SseParser::new();
        let events = parser.feed("data: line1\ndata: line2\ndata: line3\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "line1\nline2\nline3");
    }

    #[test]
    fn test_sse_event_with_id() {
        let mut parser = SseParser::new();
        let events = parser.feed("id: 42\ndata: hello\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, Some("42".to_string()));
    }

    #[test]
    fn test_sse_event_with_retry() {
        let mut parser = SseParser::new();
        let events = parser.feed("retry: 5000\ndata: hello\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].retry, Some(5000));
    }

    #[test]
    fn test_sse_comment_ignored() {
        let mut parser = SseParser::new();
        let events = parser.feed(": this is a comment\ndata: hello\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "hello");
    }

    #[test]
    fn test_sse_incremental_feed() {
        let mut parser = SseParser::new();
        let events1 = parser.feed("data: hel");
        assert_eq!(events1.len(), 0);
        let events2 = parser.feed("lo\n\n");
        assert_eq!(events2.len(), 1);
        assert_eq!(events2[0].data, "hello");
    }

    #[test]
    fn test_sse_multiple_events() {
        let mut parser = SseParser::new();
        let events = parser.feed("data: first\n\ndata: second\n\n");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].data, "first");
        assert_eq!(events[1].data, "second");
    }

    #[test]
    fn test_sse_crlf() {
        let mut parser = SseParser::new();
        let events = parser.feed("data: hello\r\n\r\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "hello");
    }

    #[test]
    fn test_sse_no_space_after_colon() {
        let mut parser = SseParser::new();
        let events = parser.feed("data:hello\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "hello");
    }

    #[test]
    fn test_sse_empty_data() {
        let mut parser = SseParser::new();
        let events = parser.feed("data:\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "");
    }

    #[test]
    fn test_sse_id_with_nul_ignored() {
        let mut parser = SseParser::new();
        let events = parser.feed("id: abc\0def\ndata: hello\n\n");
        assert_eq!(events.len(), 1);
        // NUL 문자가 있는 id는 무시됨
        assert_eq!(events[0].id, None);
    }

    #[test]
    fn test_sse_last_event_id_persists() {
        let mut parser = SseParser::new();
        let events = parser.feed("id: 1\ndata: first\n\ndata: second\n\n");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].id, Some("1".to_string()));
        // last_event_id는 유지된다
        assert_eq!(events[1].id, Some("1".to_string()));
    }

    #[test]
    fn test_sse_event_type_resets() {
        let mut parser = SseParser::new();
        let events = parser.feed("event: custom\ndata: first\n\ndata: second\n\n");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, Some("custom".to_string()));
        // event type은 다음 이벤트에서는 리셋
        assert_eq!(events[1].event_type, None);
    }

    #[test]
    fn test_sse_retry_invalid_ignored() {
        let mut parser = SseParser::new();
        let events = parser.feed("retry: abc\ndata: hello\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].retry, None);
    }

    #[test]
    fn test_sse_serialization() {
        let event = SseEventInfo {
            connection_id: "test-conn".to_string(),
            sequence: 1,
            event_type: Some("message".to_string()),
            data: "hello".to_string(),
            id: None,
            retry: None,
            size: 5,
            time: 1234567890,
        };
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: SseEventInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.connection_id, "test-conn");
        assert_eq!(deserialized.data, "hello");
    }

    #[test]
    fn test_sse_connection_event_serialization() {
        let connected = SseConnectionEvent::Connected {
            connection_id: "conn1".to_string(),
            uri: "https://example.com/events".to_string(),
            time: 1234567890,
        };
        let json = serde_json::to_string(&connected).unwrap();
        assert!(json.contains("\"status\":\"connected\""));

        let disconnected = SseConnectionEvent::Disconnected {
            connection_id: "conn1".to_string(),
            time: 1234567890,
        };
        let json = serde_json::to_string(&disconnected).unwrap();
        assert!(json.contains("\"status\":\"disconnected\""));
    }
}

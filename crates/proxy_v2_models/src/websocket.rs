use base64::Engine;
use serde::{Deserialize, Serialize};

/// WebSocket 메시지 콘텐츠 타입 (Content View 확장)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum WsContentType {
    #[default]
    Plain,
    SocketIO,
    Mqtt,
}

/// WebSocket 페이로드에서 콘텐츠 타입을 감지
pub fn detect_ws_content_type(payload: &str, is_binary: bool) -> WsContentType {
    if is_binary {
        // MQTT: 첫 바이트 상위 4비트가 유효한 MQTT 패킷 타입 (1~14)
        if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(payload) {
            if !bytes.is_empty() {
                let packet_type = bytes[0] >> 4;
                if (1..=14).contains(&packet_type) {
                    // 추가 검증: Remaining Length 필드가 유효한지 확인
                    if bytes.len() >= 2 && is_valid_mqtt_remaining_length(&bytes[1..]) {
                        return WsContentType::Mqtt;
                    }
                }
            }
        }
    } else {
        // Socket.IO: Engine.IO 메시지(4) + Socket.IO 타입(0-4) + 데이터
        // 패턴: "4" + digit + optional "[" or "{"
        let bytes = payload.as_bytes();
        if !bytes.is_empty() {
            let first = bytes[0];
            // Engine.IO packet types: 0=open, 1=close, 2=ping, 3=pong, 4=message
            if first == b'4' && bytes.len() >= 2 {
                let second = bytes[1];
                // Socket.IO packet types: 0=CONNECT, 1=DISCONNECT, 2=EVENT, 3=ACK, 4=ERROR
                if second.is_ascii_digit() {
                    return WsContentType::SocketIO;
                }
            }
        }
    }
    WsContentType::Plain
}

/// MQTT Remaining Length 필드가 유효한지 확인
fn is_valid_mqtt_remaining_length(data: &[u8]) -> bool {
    let mut multiplier: usize = 1;
    let mut value: usize = 0;
    for (i, &byte) in data.iter().enumerate() {
        value += (byte as usize & 0x7F) * multiplier;
        if byte & 0x80 == 0 {
            // Remaining Length 이후 데이터 길이가 value와 정확히 일치해야 함
            let remaining_data = data.len() - (i + 1);
            return remaining_data == value;
        }
        multiplier *= 128;
        if i >= 3 {
            return false; // Remaining Length는 최대 4바이트
        }
    }
    false
}

/// WebSocket 메시지 방향
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WsDirection {
    ClientToServer,
    ServerToClient,
}

/// WebSocket 메시지 타입
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WsMessageType {
    Text,
    Binary,
    Ping,
    Pong,
    Close,
}

/// UI로 전달되는 WebSocket 메시지 정보
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct WsMessageInfo {
    /// WebSocket 연결 ID (URI)
    pub connection_id: String,
    /// 메시지 순번
    pub sequence: u64,
    /// 메시지 방향
    pub direction: WsDirection,
    /// 메시지 타입
    pub message_type: WsMessageType,
    /// 메시지 페이로드 (텍스트 또는 base64 인코딩된 바이너리)
    pub payload: String,
    /// 페이로드 크기 (바이트)
    pub size: usize,
    /// 타임스탬프 (나노초)
    pub time: i64,
    /// 바이너리 여부
    pub is_binary: bool,
    /// 콘텐츠 타입 (Content View 확장)
    #[serde(default)]
    pub content_type: WsContentType,
}

/// WebSocket 연결 상태 이벤트
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "status")]
pub enum WsConnectionEvent {
    #[serde(rename = "connected")]
    Connected {
        connection_id: String,
        uri: String,
        time: i64,
    },
    #[serde(rename = "disconnected")]
    Disconnected { connection_id: String, time: i64 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_socketio() {
        // Socket.IO EVENT: 42["chat","hello"]
        assert_eq!(
            detect_ws_content_type(r#"42["chat","hello"]"#, false),
            WsContentType::SocketIO,
        );

        // Socket.IO CONNECT: 40
        assert_eq!(detect_ws_content_type("40", false), WsContentType::SocketIO,);

        // Not Socket.IO: regular text
        assert_eq!(
            detect_ws_content_type("hello world", false),
            WsContentType::Plain,
        );

        // Not Socket.IO: JSON
        assert_eq!(
            detect_ws_content_type(r#"{"type":"ping"}"#, false),
            WsContentType::Plain,
        );
    }

    #[test]
    fn test_detect_mqtt() {
        use base64::Engine;

        // MQTT CONNECT packet (type=1, remaining length=0)
        let connect_packet = vec![0x10, 0x00];
        let encoded = base64::engine::general_purpose::STANDARD.encode(&connect_packet);
        assert_eq!(detect_ws_content_type(&encoded, true), WsContentType::Mqtt,);

        // MQTT PUBLISH packet (type=3)
        let publish_packet = vec![0x30, 0x05, 0x00, 0x01, b'a', b'h', b'i'];
        let encoded = base64::engine::general_purpose::STANDARD.encode(&publish_packet);
        assert_eq!(detect_ws_content_type(&encoded, true), WsContentType::Mqtt,);

        // Not MQTT: invalid packet type (0 or 15)
        let invalid_packet = vec![0x00, 0x00];
        let encoded = base64::engine::general_purpose::STANDARD.encode(&invalid_packet);
        assert_eq!(detect_ws_content_type(&encoded, true), WsContentType::Plain,);

        // Not MQTT: regular base64 text
        let text = base64::engine::general_purpose::STANDARD.encode(b"hello");
        assert_eq!(detect_ws_content_type(&text, true), WsContentType::Plain,);
    }
}

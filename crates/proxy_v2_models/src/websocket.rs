use base64::Engine;
use serde::{Deserialize, Serialize};

/// WebSocket 메시지 콘텐츠 타입 (Content View 확장)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum WsContentType {
    #[default]
    Plain,
    #[serde(rename = "socket_io")]
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

/// MQTT CONNECT 패킷에서 프로토콜 버전 바이트를 추출한다.
/// CONNECT 패킷 구조: Fixed Header + Remaining Length + Protocol Name(UTF-8) + Version(1 byte) + ...
/// 반환값: 4 = MQTT 3.1.1, 5 = MQTT 5.0
pub fn extract_mqtt_version_from_connect(base64_payload: &str) -> Option<u8> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(base64_payload)
        .ok()?;
    if bytes.is_empty() || (bytes[0] >> 4) != 1 {
        return None; // CONNECT가 아님
    }
    // Remaining Length 건너뛰기
    let mut idx = 1;
    while idx < bytes.len() {
        let b = bytes[idx];
        idx += 1;
        if b & 0x80 == 0 {
            break;
        }
    }
    // Protocol Name (UTF-8 string: 2-byte length + string)
    if idx + 2 > bytes.len() {
        return None;
    }
    let name_len = ((bytes[idx] as usize) << 8) | (bytes[idx + 1] as usize);
    idx += 2 + name_len;
    // Version byte
    if idx >= bytes.len() {
        return None;
    }
    Some(bytes[idx])
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
    /// MQTT 프로토콜 버전 (4=3.1.1, 5=5.0). MQTT 메시지인 경우에만 Some.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mqtt_version: Option<u8>,
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
    use base64::Engine;

    fn encode(bytes: &[u8]) -> String {
        base64::engine::general_purpose::STANDARD.encode(bytes)
    }

    /// MQTT UTF-8 String 인코딩 헬퍼
    fn mqtt_string(s: &str) -> Vec<u8> {
        let bytes = s.as_bytes();
        let len = bytes.len() as u16;
        let mut v = vec![(len >> 8) as u8, (len & 0xff) as u8];
        v.extend_from_slice(bytes);
        v
    }

    /// MQTT CONNECT 패킷 생성 헬퍼
    fn build_connect_packet(version: u8) -> Vec<u8> {
        let mut var_header = mqtt_string("MQTT");
        var_header.push(version);
        var_header.push(0x02); // Clean Session
        var_header.extend_from_slice(&[0x00, 0x3c]); // Keep Alive = 60s
        var_header.extend(mqtt_string("test-client"));

        let remaining_len = var_header.len() as u8;
        let mut packet = vec![0x10, remaining_len]; // CONNECT = 1 << 4
        packet.extend(var_header);
        packet
    }

    // ============================================================
    // detect_ws_content_type 테스트
    // ============================================================

    #[test]
    fn test_detect_socketio_event() {
        assert_eq!(
            detect_ws_content_type(r#"42["chat","hello"]"#, false),
            WsContentType::SocketIO,
        );
    }

    #[test]
    fn test_detect_socketio_connect() {
        assert_eq!(
            detect_ws_content_type("40", false),
            WsContentType::SocketIO,
        );
    }

    #[test]
    fn test_detect_socketio_disconnect() {
        assert_eq!(
            detect_ws_content_type("41", false),
            WsContentType::SocketIO,
        );
    }

    #[test]
    fn test_detect_socketio_ack() {
        assert_eq!(
            detect_ws_content_type("43", false),
            WsContentType::SocketIO,
        );
    }

    #[test]
    fn test_detect_plain_text() {
        assert_eq!(
            detect_ws_content_type("hello world", false),
            WsContentType::Plain,
        );
    }

    #[test]
    fn test_detect_plain_json() {
        assert_eq!(
            detect_ws_content_type(r#"{"type":"ping"}"#, false),
            WsContentType::Plain,
        );
    }

    #[test]
    fn test_detect_plain_single_digit() {
        // "4" 단독은 Socket.IO가 아님 (두 번째 digit 필요)
        assert_eq!(detect_ws_content_type("4", false), WsContentType::Plain);
    }

    #[test]
    fn test_detect_plain_non_message_engineio() {
        // Engine.IO ping(2), pong(3) 등은 Socket.IO가 아님
        assert_eq!(
            detect_ws_content_type("2probe", false),
            WsContentType::Plain,
        );
    }

    #[test]
    fn test_detect_mqtt_connect() {
        let packet = vec![0x10, 0x00]; // CONNECT, remaining length=0
        assert_eq!(
            detect_ws_content_type(&encode(&packet), true),
            WsContentType::Mqtt,
        );
    }

    #[test]
    fn test_detect_mqtt_publish() {
        let packet = vec![0x30, 0x05, 0x00, 0x01, b'a', b'h', b'i'];
        assert_eq!(
            detect_ws_content_type(&encode(&packet), true),
            WsContentType::Mqtt,
        );
    }

    #[test]
    fn test_detect_mqtt_pingreq() {
        let packet = vec![0xc0, 0x00]; // PINGREQ
        assert_eq!(
            detect_ws_content_type(&encode(&packet), true),
            WsContentType::Mqtt,
        );
    }

    #[test]
    fn test_detect_mqtt_disconnect() {
        let packet = vec![0xe0, 0x00]; // DISCONNECT
        assert_eq!(
            detect_ws_content_type(&encode(&packet), true),
            WsContentType::Mqtt,
        );
    }

    #[test]
    fn test_detect_not_mqtt_invalid_type() {
        // packet type 0과 15는 유효하지 않음
        let packet = vec![0x00, 0x00];
        assert_eq!(
            detect_ws_content_type(&encode(&packet), true),
            WsContentType::Plain,
        );
        let packet = vec![0xf0, 0x00];
        assert_eq!(
            detect_ws_content_type(&encode(&packet), true),
            WsContentType::Plain,
        );
    }

    #[test]
    fn test_detect_not_mqtt_hello() {
        // "hello"를 base64 인코딩하면 MQTT로 오탐지되지 않아야 함
        let text = encode(b"hello");
        assert_eq!(
            detect_ws_content_type(&text, true),
            WsContentType::Plain,
        );
    }

    #[test]
    fn test_detect_not_mqtt_remaining_length_mismatch() {
        // Remaining Length가 실제 데이터와 불일치
        let packet = vec![0x30, 0x0a, 0x00, 0x01, b'x']; // remaining=10이지만 실제=3
        assert_eq!(
            detect_ws_content_type(&encode(&packet), true),
            WsContentType::Plain,
        );
    }

    // ============================================================
    // is_valid_mqtt_remaining_length 테스트
    // ============================================================

    #[test]
    fn test_remaining_length_zero() {
        assert!(is_valid_mqtt_remaining_length(&[0x00]));
    }

    #[test]
    fn test_remaining_length_exact_match() {
        // remaining length = 3, 뒤에 3바이트
        assert!(is_valid_mqtt_remaining_length(&[0x03, 0xaa, 0xbb, 0xcc]));
    }

    #[test]
    fn test_remaining_length_mismatch() {
        // remaining length = 5, 뒤에 2바이트
        assert!(!is_valid_mqtt_remaining_length(&[0x05, 0xaa, 0xbb]));
    }

    #[test]
    fn test_remaining_length_multibyte() {
        // 128 = 0x80 0x01 (variable byte integer)
        // 뒤에 정확히 128바이트
        let mut data = vec![0x80, 0x01];
        data.extend(vec![0x00; 128]);
        assert!(is_valid_mqtt_remaining_length(&data));
    }

    #[test]
    fn test_remaining_length_empty() {
        assert!(!is_valid_mqtt_remaining_length(&[]));
    }

    // ============================================================
    // extract_mqtt_version_from_connect 테스트
    // ============================================================

    #[test]
    fn test_extract_mqtt_version_311() {
        let packet = build_connect_packet(4);
        let result = extract_mqtt_version_from_connect(&encode(&packet));
        assert_eq!(result, Some(4));
    }

    #[test]
    fn test_extract_mqtt_version_50() {
        let packet = build_connect_packet(5);
        let result = extract_mqtt_version_from_connect(&encode(&packet));
        assert_eq!(result, Some(5));
    }

    #[test]
    fn test_extract_version_not_connect() {
        // PUBLISH 패킷 — CONNECT가 아니므로 None
        let packet = vec![0x30, 0x00];
        assert_eq!(extract_mqtt_version_from_connect(&encode(&packet)), None);
    }

    #[test]
    fn test_extract_version_too_short() {
        // CONNECT 헤더만 있고 데이터 부족
        let packet = vec![0x10, 0x02, 0x00];
        assert_eq!(extract_mqtt_version_from_connect(&encode(&packet)), None);
    }

    #[test]
    fn test_extract_version_invalid_base64() {
        assert_eq!(
            extract_mqtt_version_from_connect("!!!not-base64!!!"),
            None,
        );
    }

    #[test]
    fn test_extract_version_empty() {
        assert_eq!(extract_mqtt_version_from_connect(""), None);
    }

    // ============================================================
    // WsContentType serde 테스트
    // ============================================================

    #[test]
    fn test_ws_content_type_default() {
        assert_eq!(WsContentType::default(), WsContentType::Plain);
    }

    #[test]
    fn test_ws_content_type_serde_roundtrip() {
        let types = vec![
            WsContentType::Plain,
            WsContentType::SocketIO,
            WsContentType::Mqtt,
        ];
        for ct in types {
            let json = serde_json::to_string(&ct).unwrap();
            let deserialized: WsContentType = serde_json::from_str(&json).unwrap();
            assert_eq!(ct, deserialized);
        }
    }

    #[test]
    fn test_ws_content_type_snake_case_serialization() {
        assert_eq!(
            serde_json::to_string(&WsContentType::SocketIO).unwrap(),
            "\"socket_io\"",
        );
        assert_eq!(
            serde_json::to_string(&WsContentType::Mqtt).unwrap(),
            "\"mqtt\"",
        );
        assert_eq!(
            serde_json::to_string(&WsContentType::Plain).unwrap(),
            "\"plain\"",
        );
    }
}

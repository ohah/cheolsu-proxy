# WebSocket

WebSocket 연결을 실시간으로 모니터링하고 메시지를 주입할 수 있습니다.

---

## 프로토콜 감지

WebSocket 메시지의 콘텐츠 타입을 자동으로 감지합니다.

| 타입           | 설명                                     |
| -------------- | ---------------------------------------- |
| **Plain Text** | 일반 텍스트 또는 JSON 메시지             |
| **Socket.IO**  | Engine.IO + Socket.IO 프로토콜 자동 감지 |
| **MQTT**       | MQTT 패킷 감지 (v3.1.1, v5.0 지원)       |

---

## 주요 기능

### 연결 목록

- 활성 WebSocket 연결을 시간순으로 표시
- 연결/해제 상태 표시
- 연결별 URI 표시

### 메시지 보기

- 메시지 방향 표시 (Client -> Server, Server -> Client)
- 메시지 타입 표시 (Text, Binary, Ping, Pong, Close)
- 바이너리/텍스트 구분
- 페이로드 내용 확인

### 메시지 주입

활성 WebSocket 연결에 메시지를 직접 주입할 수 있습니다.

- **방향 선택**: Client -> Server 또는 Server -> Client
- **텍스트/바이너리**: 텍스트 또는 바이너리 메시지 전송

### 메시지 인터셉트

스크립팅 훅(`cheolsu.onWebSocketMessage`)을 사용하여 WebSocket 메시지를 수정하거나 차단할 수 있습니다.

---

## 사용 방법

### Desktop

1. 사이드바에서 **WebSocket** 메뉴 선택
2. 연결 목록에서 연결 선택
3. 메시지 내용 확인

### TUI

1. **WebSocket** 탭으로 이동
2. 연결 및 메시지 조회

### MCP

```
"WebSocket 메시지 중 MQTT 관련 내용을 보여줘"
```

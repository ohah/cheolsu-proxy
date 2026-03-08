# MCP Server

cheolsu-proxy는 [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) 서버를 내장하고 있어, Claude Code, Cursor, Claude Desktop 등 AI 어시스턴트에서 캡처된 트래픽을 직접 조회하고 조작할 수 있습니다.

---

## 설정 방법

### 1. MCP 설정 복사

앱 좌측 사이드바 하단의 **MCP Server** 버튼을 클릭하면 설정 JSON이 표시됩니다. 복사 버튼을 눌러 클립보드에 복사하세요.

### 2. AI 클라이언트에 등록

복사한 설정을 AI 클라이언트의 MCP 설정 파일에 붙여넣습니다.

**Claude Code** (`.claude/settings.json`)

```json
{
  "mcpServers": {
    "cheolsu-proxy": {
      "command": "/path/to/cheolsu-proxy-mcp"
    }
  }
}
```

**Cursor** (`.cursor/mcp.json`)

```json
{
  "mcpServers": {
    "cheolsu-proxy": {
      "command": "/path/to/cheolsu-proxy-mcp"
    }
  }
}
```

### 3. 사용

Cheolsu Proxy 앱을 실행한 상태에서 AI 어시스턴트에게 트래픽 관련 질문을 하면 됩니다.

---

## 제공되는 MCP Tools

### 트래픽 조회

| Tool                     | 설명                                                         |
| ------------------------ | ------------------------------------------------------------ |
| `search_traffic`         | 호스트, HTTP 메서드, 상태코드, URL 경로로 캡처된 트래픽 검색 |
| `get_transaction`        | 특정 트랜잭션의 요청/응답 헤더 및 바디 상세 조회             |
| `get_websocket_messages` | 캡처된 WebSocket 메시지 조회 (연결 URI 필터 지원)            |

### 요청 전송

| Tool             | 설명                                                   |
| ---------------- | ------------------------------------------------------ |
| `replay_request` | HTTP 요청을 직접 전송 (프록시 우회). API 테스트에 유용 |

### 인터셉트 규칙 관리

| Tool          | 설명                                                                                  |
| ------------- | ------------------------------------------------------------------------------------- |
| `list_rules`  | 현재 설정된 인터셉트 규칙 목록 조회                                                   |
| `add_rule`    | 새 인터셉트 규칙 추가 (block, modify_request, modify_response, map_local, map_remote) |
| `remove_rule` | ID로 인터셉트 규칙 삭제                                                               |

### 스크립팅

| Tool            | 설명                                                      |
| --------------- | --------------------------------------------------------- |
| `load_script`   | JavaScript/TypeScript 스크립트 로드 (파일 경로 또는 코드) |
| `unload_script` | 현재 로드된 스크립트 언로드                               |

### 상태 관리

| Tool            | 설명                                 |
| --------------- | ------------------------------------ |
| `proxy_status`  | 프록시 데몬 상태 및 트래픽 통계 확인 |
| `clear_traffic` | 메모리에 캡처된 트래픽 데이터 초기화 |

---

## 사용 예시

AI 어시스턴트에게 다음과 같이 요청할 수 있습니다:

- "최근 500 에러가 난 API 요청을 찾아줘"
- "이 API의 요청/응답을 보고 TypeScript 인터페이스를 만들어줘"
- "example.com 도메인을 차단하는 규칙을 추가해줘"
- "이 요청을 body만 바꿔서 다시 보내줘"

---

## 아키텍처

```
AI Assistant (Claude Code / Cursor)
        │
        │ MCP Protocol (stdio)
        ▼
┌─────────────────────┐
│  cheolsu-proxy-mcp  │  MCP 서버 바이너리
│  (트래픽 수집/노출)  │
└─────────┬───────────┘
          │ Unix Domain Socket
          ▼
┌─────────────────────┐
│   Proxy Daemon      │  프록시 데몬
└─────────────────────┘
```

MCP 서버는 프록시 데몬의 또 다른 클라이언트로 동작합니다. 실시간으로 트래픽을 수집하며, 최근 1,000개의 HTTP 트랜잭션과 5,000개의 WebSocket 메시지를 메모리에 유지합니다.

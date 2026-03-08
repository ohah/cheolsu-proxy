# 스크립팅

Deno Core (V8) 기반의 JavaScript/TypeScript 스크립팅 엔진으로 HTTP 요청/응답 및 WebSocket 메시지를 프로그래밍 방식으로 조작할 수 있습니다.

---

## 지원 파일 형식

- `.js`, `.ts`, `.mjs`, `.mts`
- TypeScript는 swc 기반으로 자동 트랜스파일됩니다.

---

## 훅 API

### cheolsu.onRequest(handler)

HTTP 요청이 서버로 전달되기 전에 호출됩니다.

```javascript
cheolsu.onRequest((request) => {
  // request: { method, url, headers, body }

  // 그대로 전달
  return { action: "forward" };

  // 수정된 요청 전달
  return {
    action: "modify",
    request: {
      ...request,
      headers: { ...request.headers, "X-Custom": "value" },
    },
  };

  // 직접 응답 (서버 요청 생략)
  return {
    action: "respond",
    response: {
      status: 200,
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ mocked: true }),
    },
  };
});
```

### cheolsu.onResponse(handler)

서버 응답이 클라이언트에 전달되기 전에 호출됩니다.

```javascript
cheolsu.onResponse((request, response) => {
  // response: { status, headers, body }

  // 그대로 전달
  return { action: "forward" };

  // 수정된 응답 전달
  return {
    action: "modify",
    response: {
      ...response,
      headers: { ...response.headers, "X-Modified": "true" },
    },
  };
});
```

### cheolsu.onWebSocketMessage(handler)

WebSocket 메시지가 전달되기 전에 호출됩니다.

```javascript
cheolsu.onWebSocketMessage((message) => {
  // message: { direction, payload, is_binary }
  // direction: "to_server" | "to_client"

  // 그대로 전달
  return { action: "forward" };

  // 수정된 메시지 전달
  return { action: "modify", payload: "modified", is_binary: false };

  // 메시지 버림
  return { action: "drop" };
});
```

---

## 콘솔 API

스크립트 내에서 다음 콘솔 함수를 사용할 수 있으며, 로그는 GUI/TUI 콘솔 패널에 실시간으로 출력됩니다.

- `console.log()` - 일반 로그
- `console.warn()` - 경고
- `console.error()` - 에러
- `console.debug()` - 디버그

---

## 사용 방법

### Desktop

1. 사이드바에서 **Script** 메뉴 선택
2. Monaco Editor에서 직접 코드 작성 또는 파일 경로 입력
3. `Cmd/Ctrl + Enter`로 스크립트 실행
4. 하단 콘솔 패널에서 로그 확인
5. API Reference 탭에서 전체 API 문서 확인 가능

### TUI

1. **Script** 탭으로 이동
2. 스크립트 파일 경로 입력
3. 로드/언로드

### MCP

```
"API 응답에서 특정 필드를 마스킹하는 스크립트를 작성해줘"
```

---

## 파일 자동 리로드

파일에서 스크립트를 로드한 경우, 파일이 변경되면 자동으로 리로드됩니다. 500ms 디바운스가 적용됩니다.

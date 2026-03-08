# 스크립팅

Deno Core (V8) 기반의 JavaScript/TypeScript 스크립팅 엔진으로 HTTP 요청/응답 및 WebSocket 메시지를 프로그래밍 방식으로 조작할 수 있습니다.

---

## 지원 파일 형식

- `.js`, `.ts`, `.mjs`, `.mts`
- TypeScript는 oxc 기반으로 자동 트랜스파일됩니다.

---

## 훅 API

훅 함수는 **동기(sync)** 및 **비동기(async)** 모두 지원됩니다.

### cheolsu.onRequest(handler)

HTTP 요청이 서버로 전달되기 전에 호출됩니다.

```javascript
// 동기 훅
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

// async 훅
cheolsu.onRequest(async (request) => {
  await new Promise((resolve) => setTimeout(resolve, 100));
  request.headers["X-Timestamp"] = Date.now().toString();
  return { action: "modify", request };
});
```

### cheolsu.onResponse(handler)

서버 응답이 클라이언트에 전달되기 전에 호출됩니다.

```javascript
cheolsu.onResponse(async (request, response) => {
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

## 타이머 API

```javascript
// 지정된 시간(ms) 후 콜백 실행
const id = setTimeout(callback, delay);
clearTimeout(id);

// 지정된 간격(ms)마다 콜백 반복 실행
const id = setInterval(callback, delay);
clearInterval(id);
```

> **참고:** 타이머는 훅 실행 중(이벤트 루프가 동작하는 동안)에만 동작합니다. 훅 호출 사이에는 이벤트 루프가 유휴 상태이므로 백그라운드 타이머로 사용할 수 없습니다.

---

## 콘솔 API

스크립트 내에서 다음 콘솔 함수를 사용할 수 있으며, 로그는 GUI/TUI 콘솔 패널에 실시간으로 출력됩니다.

- `console.log()` - 일반 로그
- `console.warn()` - 경고
- `console.error()` - 에러
- `console.info()` - 정보
- `console.debug()` - 디버그

---

## API 레퍼런스

### 사용 가능한 기능

| API                                                    | 설명                                  |
| ------------------------------------------------------ | ------------------------------------- |
| `cheolsu.onRequest(handler)`                           | HTTP 요청 훅 등록 (sync/async)        |
| `cheolsu.onResponse(handler)`                          | HTTP 응답 훅 등록 (sync/async)        |
| `cheolsu.onWebSocketMessage(handler)`                  | WebSocket 메시지 훅 등록 (sync/async) |
| `console.log/warn/error/info/debug()`                  | 콘솔 로깅                             |
| `setTimeout(callback, delay)`                          | 지연 실행                             |
| `clearTimeout(id)`                                     | 타이머 취소                           |
| `setInterval(callback, delay)`                         | 반복 실행                             |
| `clearInterval(id)`                                    | 반복 타이머 취소                      |
| `async` / `await`                                      | 비동기 처리                           |
| `Promise`                                              | Promise API                           |
| `JSON.parse()` / `JSON.stringify()`                    | JSON 처리                             |
| `Math.*`                                               | 수학 함수                             |
| `Date`                                                 | 날짜/시간                             |
| `RegExp`                                               | 정규표현식                            |
| `Array` / `Object` / `String` / `Map` / `Set`          | 표준 내장 객체                        |
| `Symbol` / `WeakMap` / `WeakSet` / `Proxy` / `Reflect` | ECMAScript 표준                       |
| `TextEncoder` / `TextDecoder`                          | 텍스트 인코딩 (V8 내장)               |
| `structuredClone()`                                    | 깊은 복사 (V8 내장)                   |
| TypeScript                                             | 자동 트랜스파일 지원                  |

### 사용 불가능한 기능

| API                                  | 이유                                          |
| ------------------------------------ | --------------------------------------------- |
| `fetch()`                            | 네트워크 I/O op 미등록                        |
| `XMLHttpRequest`                     | 브라우저 전용 API                             |
| `require()`                          | CommonJS 모듈 시스템 미지원                   |
| `import` / `export` (ESM)            | ES 모듈 시스템 미지원                         |
| `fs` / `path` / `os`                 | Node.js 내장 모듈 미지원                      |
| `process`                            | Node.js 전용 전역 객체                        |
| `Buffer`                             | Node.js 전용 (대신 `Uint8Array` 사용)         |
| `crypto`                             | Web Crypto API 미등록                         |
| `WebSocket` (클라이언트)             | 네트워크 I/O 미지원                           |
| `Worker` / `SharedWorker`            | 워커 스레드 미지원                            |
| `localStorage` / `sessionStorage`    | 브라우저 전용 API                             |
| `DOM API` (`document`, `window`)     | 브라우저 전용                                 |
| `alert()` / `confirm()` / `prompt()` | 브라우저 전용                                 |
| Top-level `await`                    | 스크립트 모드에서 미지원 (훅 내부에서만 가능) |

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

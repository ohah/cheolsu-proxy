// 사용자 스크립트에서 등록하는 훅 함수를 저장
const _hooks = {
  onRequest: null,
  onResponse: null,
  onWebSocketMessage: null,
};

// 사용자 스크립트에서 호출하는 글로벌 API
globalThis.cheolsu = {
  onRequest(handler) {
    _hooks.onRequest = handler;
  },
  onResponse(handler) {
    _hooks.onResponse = handler;
  },
  onWebSocketMessage(handler) {
    _hooks.onWebSocketMessage = handler;
  },
};

// console.log를 Rust 쪽으로 전달
globalThis.console = {
  log(...args) {
    Deno.core.print(args.map(a => typeof a === 'string' ? a : JSON.stringify(a)).join(' ') + '\n', false);
  },
  error(...args) {
    Deno.core.print(args.map(a => typeof a === 'string' ? a : JSON.stringify(a)).join(' ') + '\n', true);
  },
  warn(...args) {
    Deno.core.print('[WARN] ' + args.map(a => typeof a === 'string' ? a : JSON.stringify(a)).join(' ') + '\n', true);
  },
  info(...args) {
    Deno.core.print(args.map(a => typeof a === 'string' ? a : JSON.stringify(a)).join(' ') + '\n', false);
  },
};

// Rust에서 호출하는 내부 함수들 (동기)
globalThis.__cheolsu_internal = {
  invokeOnRequest(requestJson) {
    if (!_hooks.onRequest) return JSON.stringify({ action: "forward" });
    try {
      const request = JSON.parse(requestJson);
      const result = _hooks.onRequest(request);
      if (!result) return JSON.stringify({ action: "forward" });
      return JSON.stringify(result);
    } catch (e) {
      console.error("onRequest hook error:", e.message || e);
      return JSON.stringify({ action: "forward" });
    }
  },

  invokeOnResponse(requestJson, responseJson) {
    if (!_hooks.onResponse) return JSON.stringify({ action: "forward" });
    try {
      const request = JSON.parse(requestJson);
      const response = JSON.parse(responseJson);
      const result = _hooks.onResponse(request, response);
      if (!result) return JSON.stringify({ action: "forward" });
      return JSON.stringify(result);
    } catch (e) {
      console.error("onResponse hook error:", e.message || e);
      return JSON.stringify({ action: "forward" });
    }
  },

  invokeOnWebSocketMessage(messageJson) {
    if (!_hooks.onWebSocketMessage) return JSON.stringify({ action: "forward" });
    try {
      const message = JSON.parse(messageJson);
      const result = _hooks.onWebSocketMessage(message);
      if (!result) return JSON.stringify({ action: "forward" });
      return JSON.stringify(result);
    } catch (e) {
      console.error("onWebSocketMessage hook error:", e.message || e);
      return JSON.stringify({ action: "forward" });
    }
  },

  hasOnRequest() { return _hooks.onRequest !== null; },
  hasOnResponse() { return _hooks.onResponse !== null; },
  hasOnWebSocketMessage() { return _hooks.onWebSocketMessage !== null; },
};

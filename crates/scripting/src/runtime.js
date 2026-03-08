// IIFE로 내부 상태를 클로저에 감싸서 사용자 스크립트에서 접근 불가하게 보호
((globalThis) => {
  // 사용자 스크립트에서 등록하는 훅 함수를 저장
  const _hooks = {
    onRequest: null,
    onResponse: null,
    onWebSocketMessage: null,
  };

  // 로그 버퍼
  const _logBuffer = [];

  // 최대 로그 버퍼 크기
  const MAX_LOG_BUFFER = 5000;

  function pushLog(level, message) {
    if (_logBuffer.length >= MAX_LOG_BUFFER) {
      _logBuffer.shift();
    }
    _logBuffer.push({ level, message });
  }

  function formatArgs(args) {
    return args.map((a) => (typeof a === "string" ? a : JSON.stringify(a))).join(" ");
  }

  // ── 타이머 구현 ──
  const _timers = {
    nextId: 1,
    active: new Map(),
  };

  globalThis.setTimeout = function (callback, delay, ...args) {
    if (typeof callback !== "function") return 0;
    const id = _timers.nextId;
    _timers.nextId += 1;
    _timers.active.set(id, true);
    const ms = Math.max(0, Math.trunc(Number(delay) || 0));
    Deno.core.ops.op_timer_sleep(ms).then(() => {
      if (_timers.active.has(id)) {
        _timers.active.delete(id);
        try {
          callback(...args);
        } catch (e) {
          pushLog("error", "setTimeout callback error: " + (e.message || e));
          Deno.core.print("[ERROR] setTimeout callback: " + (e.message || e) + "\n", true);
        }
      }
    });
    return id;
  };

  globalThis.clearTimeout = function (id) {
    _timers.active.delete(id);
  };

  globalThis.setInterval = function (callback, delay, ...args) {
    if (typeof callback !== "function") return 0;
    const id = _timers.nextId;
    _timers.nextId += 1;
    const ms = Math.max(10, Math.trunc(Number(delay) || 10));
    _timers.active.set(id, true);
    function tick() {
      Deno.core.ops.op_timer_sleep(ms).then(() => {
        if (_timers.active.has(id)) {
          try {
            callback(...args);
          } catch (e) {
            pushLog("error", "setInterval callback error: " + (e.message || e));
            Deno.core.print("[ERROR] setInterval callback: " + (e.message || e) + "\n", true);
          }
          tick();
        }
      });
    }
    tick();
    return id;
  };

  globalThis.clearInterval = function (id) {
    _timers.active.delete(id);
  };

  // ── 사용자 API ──
  globalThis.cheolsu = Object.freeze({
    onRequest(handler) {
      _hooks.onRequest = handler;
    },
    onResponse(handler) {
      _hooks.onResponse = handler;
    },
    onWebSocketMessage(handler) {
      _hooks.onWebSocketMessage = handler;
    },
  });

  // console.log를 로그 버퍼에 저장 + Rust 쪽으로 전달
  globalThis.console = Object.freeze({
    log(...args) {
      const msg = formatArgs(args);
      pushLog("info", msg);
      Deno.core.print(msg + "\n", false);
    },
    error(...args) {
      const msg = formatArgs(args);
      pushLog("error", msg);
      Deno.core.print(msg + "\n", true);
    },
    warn(...args) {
      const msg = formatArgs(args);
      pushLog("warn", msg);
      Deno.core.print("[WARN] " + msg + "\n", true);
    },
    info(...args) {
      const msg = formatArgs(args);
      pushLog("info", msg);
      Deno.core.print(msg + "\n", false);
    },
    debug(...args) {
      const msg = formatArgs(args);
      pushLog("debug", msg);
      Deno.core.print("[DEBUG] " + msg + "\n", false);
    },
  });

  // async/sync 훅 호출 헬퍼: Promise면 .then으로 JSON.stringify, 아니면 바로 반환
  function invokeHook(hookFn, args, hookName) {
    try {
      const result = hookFn(...args);
      // async 훅 지원: Promise 반환 시 .then으로 처리
      if (result && typeof result.then === "function") {
        return result
          .then((r) => JSON.stringify(r || { action: "forward" }))
          .catch((e) => {
            console.error(hookName + " hook error:", e.message || e);
            return JSON.stringify({ action: "forward" });
          });
      }
      if (!result) return JSON.stringify({ action: "forward" });
      return JSON.stringify(result);
    } catch (e) {
      console.error(hookName + " hook error:", e.message || e);
      return JSON.stringify({ action: "forward" });
    }
  }

  // Rust에서 호출하는 내부 함수들
  // Object.freeze로 사용자 스크립트에서 덮어쓰기 방지
  globalThis.__cheolsu_internal = Object.freeze({
    invokeOnRequest(requestJson) {
      if (!_hooks.onRequest) return JSON.stringify({ action: "forward" });
      const request = JSON.parse(requestJson);
      return invokeHook(_hooks.onRequest, [request], "onRequest");
    },

    invokeOnResponse(requestJson, responseJson) {
      if (!_hooks.onResponse) return JSON.stringify({ action: "forward" });
      const request = JSON.parse(requestJson);
      const response = JSON.parse(responseJson);
      return invokeHook(_hooks.onResponse, [request, response], "onResponse");
    },

    invokeOnWebSocketMessage(messageJson) {
      if (!_hooks.onWebSocketMessage) return JSON.stringify({ action: "forward" });
      const message = JSON.parse(messageJson);
      return invokeHook(_hooks.onWebSocketMessage, [message], "onWebSocketMessage");
    },

    hasOnRequest() {
      return _hooks.onRequest !== null;
    },
    hasOnResponse() {
      return _hooks.onResponse !== null;
    },
    hasOnWebSocketMessage() {
      return _hooks.onWebSocketMessage !== null;
    },

    // 로그 버퍼를 JSON 배열로 반환하고 비움
    drainLogs() {
      const logs = JSON.stringify(_logBuffer);
      _logBuffer.length = 0;
      return logs;
    },

    // 모든 타이머 해제 (스크립트 언로드 시 사용)
    clearAllTimers() {
      _timers.active.clear();
      _timers.nextId = 1;
    },
  });
})(globalThis);

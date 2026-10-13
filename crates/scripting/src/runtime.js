// IIFE로 내부 상태를 클로저에 감싸서 사용자 스크립트에서 접근 불가하게 보호
((globalThis) => {
  // 사용자 스크립트에서 등록하는 훅 함수를 저장
  const _hooks = {
    onRequest: null,
    onResponse: null,
    onWebSocketMessage: null,
    onSSEMessage: null,
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

  // ── 엔진 네이티브 함수 (Rust에서 등록) ──
  // 사용자 스크립트에서 접근하지 못하도록 캡처 후 전역에서 제거한다.
  const _now = globalThis.__cheolsu_now;
  const _print = globalThis.__cheolsu_print;
  delete globalThis.__cheolsu_now;
  delete globalThis.__cheolsu_print;

  // ── 타이머 구현 ──
  // 타이머는 큐에만 등록되고, Rust 쪽 이벤트 루프가 훅의 Promise를 기다리는 동안
  // nextTimerDelay()/runDueTimers()로 만료된 콜백을 실행한다.
  const _timers = {
    nextId: 1,
    // id -> { due, callback, args, interval }
    active: new Map(),
  };

  function addTimer(callback, ms, args, interval) {
    const id = _timers.nextId;
    _timers.nextId += 1;
    _timers.active.set(id, { due: _now() + ms, callback, args, interval });
    return id;
  }

  globalThis.setTimeout = function (callback, delay, ...args) {
    if (typeof callback !== "function") return 0;
    const ms = Math.max(0, Math.trunc(Number(delay) || 0));
    return addTimer(callback, ms, args, 0);
  };

  globalThis.clearTimeout = function (id) {
    _timers.active.delete(id);
  };

  globalThis.setInterval = function (callback, delay, ...args) {
    if (typeof callback !== "function") return 0;
    // 최소 10ms: CPU 과부하 방지 (브라우저 표준 4ms보다 보수적으로 설정)
    const ms = Math.max(10, Math.trunc(Number(delay) || 10));
    return addTimer(callback, ms, args, ms);
  };

  globalThis.clearInterval = function (id) {
    _timers.active.delete(id);
  };

  if (typeof globalThis.queueMicrotask !== "function") {
    globalThis.queueMicrotask = function (callback) {
      Promise.resolve().then(callback);
    };
  }

  function nextTimerDelay() {
    let earliest = -1;
    for (const timer of _timers.active.values()) {
      if (earliest < 0 || timer.due < earliest) earliest = timer.due;
    }
    return earliest < 0 ? -1 : Math.max(0, earliest - _now());
  }

  function runDueTimers() {
    const now = _now();
    const due = [];
    for (const [id, timer] of _timers.active) {
      if (timer.due <= now) due.push([id, timer]);
    }
    // 만료 시각 순, 같으면 등록 순으로 실행
    due.sort((a, b) => a[1].due - b[1].due || a[0] - b[0]);
    for (const [id, timer] of due) {
      // 앞선 콜백이 이 타이머를 해제했을 수 있다
      if (_timers.active.get(id) !== timer) continue;
      if (timer.interval > 0) {
        timer.due = now + timer.interval;
      } else {
        _timers.active.delete(id);
      }
      const kind = timer.interval > 0 ? "setInterval" : "setTimeout";
      try {
        timer.callback(...timer.args);
      } catch (e) {
        pushLog("error", kind + " callback error: " + (e.message || e));
        _print("[ERROR] " + kind + " callback: " + (e.message || e) + "\n", true);
      }
    }
  }

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
    onSSEMessage(handler) {
      _hooks.onSSEMessage = handler;
    },
  });

  // console.log를 로그 버퍼에 저장 + Rust 쪽으로 전달
  globalThis.console = Object.freeze({
    log(...args) {
      const msg = formatArgs(args);
      pushLog("info", msg);
      _print(msg + "\n", false);
    },
    error(...args) {
      const msg = formatArgs(args);
      pushLog("error", msg);
      _print(msg + "\n", true);
    },
    warn(...args) {
      const msg = formatArgs(args);
      pushLog("warn", msg);
      _print("[WARN] " + msg + "\n", true);
    },
    info(...args) {
      const msg = formatArgs(args);
      pushLog("info", msg);
      _print(msg + "\n", false);
    },
    debug(...args) {
      const msg = formatArgs(args);
      pushLog("debug", msg);
      _print("[DEBUG] " + msg + "\n", false);
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

    invokeOnSSEMessage(eventJson) {
      if (!_hooks.onSSEMessage) return JSON.stringify({ action: "forward" });
      const event = JSON.parse(eventJson);
      return invokeHook(_hooks.onSSEMessage, [event], "onSSEMessage");
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
    hasOnSSEMessage() {
      return _hooks.onSSEMessage !== null;
    },

    // 로그 버퍼를 JSON 배열로 반환하고 비움
    drainLogs() {
      const logs = JSON.stringify(_logBuffer);
      _logBuffer.length = 0;
      return logs;
    },

    // 가장 가까운 타이머까지 남은 시간(ms), 타이머가 없으면 -1
    nextTimerDelay,

    // 만료된 타이머 콜백 실행
    runDueTimers,

    // 모든 타이머 해제 (스크립트 언로드 시 사용)
    clearAllTimers() {
      _timers.active.clear();
      _timers.nextId = 1;
    },
  });
})(globalThis);

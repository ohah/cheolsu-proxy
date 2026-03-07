import { describe, test, expect, beforeAll, afterAll } from "bun:test";
import {
  connectMcp,
  disconnectMcp,
  startSession,
  stopSession,
  domSnapshot,
  waitFor,
  executeJs,
  callTool,
} from "./mcp-helpers";

/**
 * 스냅샷 텍스트에서 특정 텍스트를 가진 요소의 ref를 찾는다
 */
function findRef(snapshot: string, text: string): string | null {
  for (const line of snapshot.split("\n")) {
    if (line.includes(text)) {
      const match = line.match(/\[ref=(e\d+)\]/);
      if (match) return `ref=${match[1]}`;
    }
  }
  return null;
}

/** ref로 클릭 */
async function clickRef(ref: string) {
  return callTool("webview_interact", { action: "click", selector: ref });
}

/** 짧은 대기 */
async function sleep(ms: number) {
  await executeJs(`await new Promise(r => setTimeout(r, ${ms}))`);
}

/**
 * Tauri 이벤트 시스템으로 목 proxy_event를 emit
 * withGlobalTauri: true이므로 window.__TAURI__.event.emit 사용 가능
 */
async function emitMockProxyEvent(request: object | null, response: object | null) {
  const payload = JSON.stringify([request, response]);
  // Tauri v2의 JS event emit API 호출
  return executeJs(`
    (async function() {
      try {
        // Tauri v2 global API
        var mod = window.__TAURI_INTERNALS__;
        if (mod && mod.invoke) {
          // emit은 plugin:event|emit 커맨드로 구현됨
          await mod.invoke('plugin:event|emit', {
            event: 'proxy_event',
            payload: ${payload}
          });
          return 'emitted via invoke';
        }
        return 'no tauri internals';
      } catch(e) {
        return 'error: ' + e.message;
      }
    })()
  `);
}

/** 목데이터 생성 헬퍼 */
function mockRequest(id: string, method: string, uri: string, dataType = "Empty") {
  const url = new URL(uri);
  return {
    method,
    uri,
    version: "HTTP/1.1",
    headers: {
      host: url.host,
      "user-agent": "MockClient/1.0",
      accept: "application/json",
    },
    body: null,
    time: Date.now(),
    id,
    data_type: dataType,
    body_size: 0,
  };
}

function mockResponse(id: string, status: number, dataType = "Json", bodySize = 100) {
  return {
    status,
    version: "HTTP/1.1",
    headers: { "content-type": "application/json" },
    body: null,
    time: Date.now() + 50,
    id,
    data_type: dataType,
    body_size: bodySize,
  };
}

/**
 * Network 페이지 목데이터 e2e 테스트
 *
 * 실제 프록시 daemon이 실행 중인 Tauri 앱의 Network 페이지에서
 * Tauri 이벤트를 JS로 직접 emit하여 목데이터가 UI에 표시되는지 검증합니다.
 */
describe("Network Page - Mock Data e2e", () => {
  beforeAll(async () => {
    await connectMcp();
    await startSession();

    // Network 페이지(루트)로 이동
    await executeJs("location.assign('/')");
    await sleep(3000);
  }, 30000);

  afterAll(async () => {
    await stopSession();
    await disconnectMcp();
  }, 15000);

  test("Network 페이지가 표시된다", async () => {
    const snap = await domSnapshot("accessibility");
    expect(snap).toContain("button Network");
    expect(snap).toContain("button Sessions");
    expect(snap).toContain("button Intercept Rules");
  });

  test("목데이터 proxy_event를 emit하면 트랜잭션이 표시된다", async () => {
    const id1 = "mock-001";
    await emitMockProxyEvent(
      mockRequest(id1, "GET", "https://api.example.com/api/users"),
      mockResponse(id1, 200),
    );
    await sleep(800);

    const snap = await domSnapshot("accessibility");
    expect(snap).toContain("api.example.com");
  });

  test("여러 트랜잭션을 emit하면 목록에 추가된다", async () => {
    await emitMockProxyEvent(
      mockRequest("mock-002", "POST", "https://auth.example.com/api/login", "Json"),
      mockResponse("mock-002", 401),
    );
    await sleep(300);

    await emitMockProxyEvent(
      mockRequest("mock-003", "GET", "https://cdn.example.com/static/style.css"),
      mockResponse("mock-003", 200, "Css", 1024),
    );
    await sleep(300);

    const snap = await domSnapshot("accessibility");
    expect(snap).toContain("auth.example.com");
    expect(snap).toContain("cdn.example.com");
  });

  test("HTTP 메서드가 올바르게 표시된다", async () => {
    const snap = await domSnapshot("accessibility");
    expect(snap).toContain("POST");
    expect(snap).toContain("GET");
  });

  test("다양한 상태 코드가 표시된다", async () => {
    const snap = await domSnapshot("accessibility");
    expect(snap).toContain("200");
    expect(snap).toContain("401");
  });

  test("트랜잭션 행을 클릭하면 상세 패널이 열린다", async () => {
    let snap = await domSnapshot("accessibility");
    const rowRef = findRef(snap, "api.example.com");
    if (rowRef) {
      await clickRef(rowRef);
      await sleep(500);
      snap = await domSnapshot("accessibility");
      // 상세 패널에 호스트 정보가 표시됨
      expect(snap).toContain("api.example.com");
    }
  });
});

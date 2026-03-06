import { describe, test, expect, beforeAll, afterAll } from "bun:test";
import {
  connectMcp,
  disconnectMcp,
  startSession,
  stopSession,
  domSnapshot,
  clickElement,
  typeText,
  screenshot,
  waitFor,
  executeJs,
  ipcMonitor,
  ipcGetCaptured,
  getBackendState,
  callTool,
} from "./mcp-helpers";

/**
 * 스냅샷 텍스트에서 특정 텍스트를 가진 요소의 ref를 찾는다 (@eN 형식으로 반환)
 */
function findRef(snapshot: string, text: string): string | null {
  for (const line of snapshot.split("\n")) {
    if (line.includes(text)) {
      const match = line.match(/\[ref=(e\d+)\]/);
      if (match) return `@${match[1]}`;
    }
  }
  return null;
}

/** ref 문자열에서 selector 형식 추출 (e.g. @e39 -> "ref=e39") */
function refSelector(ref: string): string {
  return `ref=${ref.replace("@", "")}`;
}

/** ref로 클릭 — webview_interact 사용 */
async function clickRef(ref: string) {
  return callTool("webview_interact", { action: "click", selector: refSelector(ref) });
}

/** placeholder로 input/textarea를 찾아 React 호환 방식으로 값 설정 */
async function typeByPlaceholder(placeholder: string, text: string) {
  const escaped = text.replace(/\\/g, "\\\\").replace(/'/g, "\\'");
  const escapedPh = placeholder.replace(/\\/g, "\\\\").replace(/'/g, "\\'");
  return executeJs(`
    (function() {
      var els = document.querySelectorAll('input, textarea');
      var el = null;
      for (var i = 0; i < els.length; i++) {
        if (els[i].placeholder === '${escapedPh}') { el = els[i]; break; }
      }
      if (!el) return 'element not found for placeholder: ${escapedPh}';
      el.focus();
      var tracker = el._valueTracker;
      if (tracker) { tracker.setValue(''); }
      var nativeSetter = Object.getOwnPropertyDescriptor(
        window.HTMLInputElement.prototype, 'value'
      )?.set || Object.getOwnPropertyDescriptor(
        window.HTMLTextAreaElement.prototype, 'value'
      )?.set;
      if (nativeSetter) {
        nativeSetter.call(el, '${escaped}');
      } else {
        el.value = '${escaped}';
      }
      el.dispatchEvent(new Event('input', { bubbles: true }));
      el.dispatchEvent(new Event('change', { bubbles: true }));
      return 'typed: ' + el.value;
    })()
  `);
}

/** 짧은 대기 */
async function sleep(ms: number) {
  await executeJs(`await new Promise(r => setTimeout(r, ${ms}))`);
}

/**
 * Intercept Rules 페이지 e2e 테스트 (MCP Server Tauri)
 *
 * 실행 전 Tauri 앱이 dev 모드로 떠 있어야 합니다:
 *   cd tauri-ui && bun run tauri-dev
 */
describe("Intercept Rules Page - MCP e2e", () => {
  beforeAll(async () => {
    await connectMcp();
    await startSession();

    // localStorage 초기화 후 intercept-rules 페이지로 이동
    await executeJs("localStorage.removeItem('cheolsu-intercept-rules')");
    await executeJs("location.assign('/intercept-rules')");
    await waitFor("text", "No intercept rules", 10000);
    // aria-api 로드 대기
    await sleep(1000);
  }, 30000);

  afterAll(async () => {
    // 다이얼로그가 열려있을 수 있으므로 닫기
    await executeJs("document.querySelector('[data-slot=\"dialog-close\"]')?.click()").catch(
      () => {},
    );
    await sleep(300);
    await stopSession();
    await disconnectMcp();
  }, 15000);

  test("페이지 타이틀과 빈 상태 메시지가 표시된다", async () => {
    const snap = await domSnapshot("accessibility");

    expect(snap).toContain("Intercept Rules");
    expect(snap).toContain("No intercept rules");
    expect(snap).toContain("Add your first rule");
  });

  test("사이드바에 네비게이션 항목들이 있다", async () => {
    const snap = await domSnapshot("accessibility");

    expect(snap).toContain("button Network");
    expect(snap).toContain("button Sessions");
    expect(snap).toContain("button Intercept Rules");
  });

  test("Add Rule 버튼 클릭 시 다이얼로그가 열린다", async () => {
    const snap = await domSnapshot("accessibility");
    const addRef = findRef(snap, "Add your first rule");
    expect(addRef).not.toBeNull();

    await clickRef(addRef!);
    await waitFor("text", "Pattern", 5000);

    const dialogSnap = await domSnapshot("accessibility");
    expect(dialogSnap).toContain("Pattern");
    expect(dialogSnap).toContain("Cancel");
  });

  test("Block 규칙을 추가할 수 있다", async () => {
    // 현재 다이얼로그가 열려있는 상태
    let snap = await domSnapshot("accessibility");

    // placeholder 기반으로 input에 값 입력
    await typeByPlaceholder("Rule name (optional)", "Block Test APIs");
    await typeByPlaceholder("*.example.com/api/*", "*.test-block.com/*");

    // 다이얼로그 내 "Add Rule" 버튼 클릭
    snap = await domSnapshot("accessibility");
    const lines = snap.split("\n").filter((l: string) => l.includes("button Add Rule"));
    const lastLine = lines[lines.length - 1];
    const submitRef = lastLine?.match(/\[ref=(e\d+)\]/)?.[1];
    expect(submitRef).toBeDefined();

    await clickRef(`@${submitRef}`);
    await waitFor("text", "Block Test APIs", 5000);

    const result = await domSnapshot("accessibility");
    expect(result).toContain("Block Test APIs");
  });

  test("규칙이 추가된 후 카운트가 업데이트된다", async () => {
    const snap = await domSnapshot("accessibility");
    // "1" 과 "rules" 텍스트가 포함되어야 함
    expect(snap).toContain('"1"');
    expect(snap).toContain("rules");
  });

  test("규칙 토글(비활성화/활성화)이 동작한다", async () => {
    let snap = await domSnapshot("accessibility");
    const switchRef = findRef(snap, "switch");

    if (switchRef) {
      await clickRef(switchRef);
      await sleep(300);
      await clickRef(switchRef);
      await sleep(300);
    }

    snap = await domSnapshot("accessibility");
    expect(snap).toContain("Block Test APIs");
  });

  test("Edit 버튼 클릭 시 수정 다이얼로그가 열린다", async () => {
    let snap = await domSnapshot("accessibility");
    const editRef = findRef(snap, "Edit rule");

    expect(editRef).not.toBeNull();
    await clickRef(editRef!);
    await waitFor("text", "Edit Rule", 5000);

    const dialogSnap = await domSnapshot("accessibility");
    expect(dialogSnap).toContain("Edit Rule");
    expect(dialogSnap).toContain("Update");

    // Cancel 클릭
    const cancelRef = findRef(dialogSnap, "button Cancel");
    expect(cancelRef).not.toBeNull();
    await clickRef(cancelRef!);
    await sleep(500);
  });

  test("두 번째 규칙을 추가할 수 있다", async () => {
    let snap = await domSnapshot("accessibility");
    const addRef = findRef(snap, "button Add Rule");
    expect(addRef).not.toBeNull();

    await clickRef(addRef!);
    await waitFor("text", "Pattern", 5000);

    await typeByPlaceholder("Rule name (optional)", "Second Rule");
    await typeByPlaceholder("*.example.com/api/*", "*.second.com/*");

    snap = await domSnapshot("accessibility");
    const lines = snap.split("\n").filter((l: string) => l.includes("button Add Rule"));
    const submitRef = lines[lines.length - 1]?.match(/\[ref=(e\d+)\]/)?.[1];
    await clickRef(`@${submitRef}`);
    await waitFor("text", "Second Rule", 5000);

    snap = await domSnapshot("accessibility");
    expect(snap).toContain("Second Rule");
    expect(snap).toContain('"2"');
  });

  test("규칙을 삭제할 수 있다", async () => {
    let snap = await domSnapshot("accessibility");
    const deleteRef = findRef(snap, "Delete rule");
    expect(deleteRef).not.toBeNull();

    await clickRef(deleteRef!);
    await sleep(500);

    snap = await domSnapshot("accessibility");
    expect(snap).toContain('"1"');
    expect(snap).toContain("rules");
  });

  test("Clear All로 모든 규칙을 삭제할 수 있다", async () => {
    let snap = await domSnapshot("accessibility");
    const clearRef = findRef(snap, "Clear All");
    expect(clearRef).not.toBeNull();

    await clickRef(clearRef!);
    await waitFor("text", "No intercept rules", 5000);

    snap = await domSnapshot("accessibility");
    expect(snap).toContain("No intercept rules");
  });

  test("백엔드 상태를 조회할 수 있다", async () => {
    const state = await getBackendState();
    expect(state).toBeDefined();
  });

  test("스크린샷을 캡처할 수 있다", async () => {
    const result = await screenshot("/tmp/intercept-rules-e2e.png");
    expect(result).toBeDefined();
  });

  test("페이지 네비게이션이 동작한다", async () => {
    // Network 페이지로 이동
    let snap = await domSnapshot("accessibility");
    const networkRef = findRef(snap, "button Network");
    expect(networkRef).not.toBeNull();

    await clickRef(networkRef!);
    await sleep(1000);

    snap = await domSnapshot("accessibility");
    // Network 페이지에서는 "No intercept rules" 가 없어야 함
    expect(snap).not.toContain("No intercept rules");
    expect(snap).not.toContain("Add your first rule");

    // 다시 Intercept Rules로 이동
    const irRef = findRef(snap, "button Intercept Rules");
    expect(irRef).not.toBeNull();

    await clickRef(irRef!);
    await waitFor("text", "No intercept rules", 5000);

    snap = await domSnapshot("accessibility");
    expect(snap).toContain("No intercept rules");
  });
});

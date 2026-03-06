import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";

let client: Client | null = null;
let transport: StdioClientTransport | null = null;

/**
 * MCP 서버에 연결하여 Tauri 앱 제어 클라이언트를 반환
 */
export async function connectMcp(): Promise<Client> {
  if (client) return client;

  transport = new StdioClientTransport({
    command: "npx",
    args: ["@hypothesi/tauri-mcp-server"],
  });

  client = new Client({ name: "e2e-test", version: "1.0.0" });
  await client.connect(transport);

  return client;
}

/**
 * MCP 연결 종료
 */
export async function disconnectMcp(): Promise<void> {
  if (client) {
    try {
      await client.close();
    } catch {
      // 이미 닫힌 경우 무시
    }
    client = null;
    transport = null;
  }
}

/**
 * MCP 도구 호출 헬퍼
 */
export async function callTool(
  name: string,
  args: Record<string, unknown> = {},
): Promise<any> {
  const c = await connectMcp();
  const result = await c.callTool({ name, arguments: args });

  // 결과에서 텍스트 콘텐츠 추출
  if (result.content && Array.isArray(result.content)) {
    const textContent = result.content.find((c: any) => c.type === "text");
    if (textContent) {
      try {
        return JSON.parse(textContent.text);
      } catch {
        return textContent.text;
      }
    }
    const imageContent = result.content.find((c: any) => c.type === "image");
    if (imageContent) return imageContent;
  }

  return result;
}

// --- 고수준 헬퍼 ---

/** 자동화 세션 시작 */
export async function startSession(port = 9223): Promise<any> {
  return callTool("driver_session", { action: "start", port });
}

/** 자동화 세션 종료 */
export async function stopSession(): Promise<any> {
  return callTool("driver_session", { action: "stop" });
}

/** 세션 상태 확인 */
export async function sessionStatus(): Promise<any> {
  return callTool("driver_session", { action: "status" });
}

/** DOM 스냅샷 조회 */
export async function domSnapshot(
  type: "accessibility" | "structure" = "accessibility",
  selector?: string,
): Promise<any> {
  const args: Record<string, unknown> = { type };
  if (selector) args.selector = selector;
  return callTool("webview_dom_snapshot", args);
}

/** 요소 찾기 */
export async function findElement(
  selector: string,
  strategy: "css" | "xpath" | "text" = "css",
): Promise<any> {
  return callTool("webview_find_element", { selector, strategy });
}

/** 요소 클릭 */
export async function clickElement(selector: string): Promise<any> {
  return callTool("webview_interact", { action: "click", selector });
}

/** 텍스트 입력 */
export async function typeText(
  selector: string,
  text: string,
): Promise<any> {
  return callTool("webview_keyboard", { action: "type", selector, text });
}

/** 스크린샷 캡처 */
export async function screenshot(filePath?: string): Promise<any> {
  const args: Record<string, unknown> = { format: "png" };
  if (filePath) args.filePath = filePath;
  return callTool("webview_screenshot", args);
}

/** 조건 대기 */
export async function waitFor(
  type: "selector" | "text" | "ipc-event",
  value: string,
  timeout = 5000,
): Promise<any> {
  return callTool("webview_wait_for", { type, value, timeout });
}

/** JS 실행 */
export async function executeJs(script: string): Promise<any> {
  return callTool("webview_execute_js", { script });
}

/** IPC 커맨드 실행 */
export async function ipcExecute(
  command: string,
  args?: Record<string, unknown>,
): Promise<any> {
  const toolArgs: Record<string, unknown> = { command };
  if (args) toolArgs.args = args;
  return callTool("ipc_execute_command", toolArgs);
}

/** IPC 모니터링 시작/종료 */
export async function ipcMonitor(action: "start" | "stop"): Promise<any> {
  return callTool("ipc_monitor", { action });
}

/** 캡처된 IPC 트래픽 조회 */
export async function ipcGetCaptured(filter?: string): Promise<any> {
  const args: Record<string, unknown> = {};
  if (filter) args.filter = filter;
  return callTool("ipc_get_captured", args);
}

/** 콘솔 로그 조회 */
export async function readLogs(
  source: "console" | "system" = "console",
  lines = 50,
  filter?: string,
): Promise<any> {
  const args: Record<string, unknown> = { source, lines };
  if (filter) args.filter = filter;
  return callTool("read_logs", args);
}

/** 백엔드 상태 조회 */
export async function getBackendState(): Promise<any> {
  return callTool("ipc_get_backend_state");
}

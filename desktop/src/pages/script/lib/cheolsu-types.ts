export const CHEOLSU_TYPE_DEFS = `
declare namespace cheolsu {
  interface Request {
    method: string;
    url: string;
    headers: Record<string, string>;
    body?: string;
  }

  interface Response {
    status: number;
    headers: Record<string, string>;
    body?: string;
  }

  interface WsMessage {
    connection_id: string;
    url: string;
    direction: "to_server" | "to_client";
    payload: string;
    is_binary: boolean;
  }

  type RequestResult =
    | { action: "forward" }
    | { action: "modify"; request: Request }
    | { action: "respond"; response: Response };

  type ResponseResult =
    | { action: "forward" }
    | { action: "modify"; response: Response };

  type WsResult =
    | { action: "forward" }
    | { action: "modify"; payload: string; is_binary: boolean }
    | { action: "drop" };

  /**
   * HTTP 요청이 프록시를 통과할 때 호출되는 훅을 등록합니다.
   * @example
   * cheolsu.onRequest((req) => {
   *   console.log(\`\${req.method} \${req.url}\`);
   *   if (req.url.includes("blocked.com")) {
   *     return { action: "respond", response: { status: 403, headers: {}, body: "Blocked" } };
   *   }
   *   return { action: "forward" };
   * });
   */
  function onRequest(handler: (request: Request) => RequestResult): void;

  /**
   * HTTP 응답이 프록시를 통과할 때 호출되는 훅을 등록합니다.
   * @example
   * cheolsu.onResponse((req, res) => {
   *   res.headers["X-Proxy"] = "cheolsu";
   *   return { action: "modify", response: res };
   * });
   */
  function onResponse(handler: (request: Request, response: Response) => ResponseResult): void;

  /**
   * WebSocket 메시지가 프록시를 통과할 때 호출되는 훅을 등록합니다.
   * @example
   * cheolsu.onWebSocketMessage((msg) => {
   *   console.log(\`[WS \${msg.direction}] \${msg.payload}\`);
   *   return { action: "forward" };
   * });
   */
  function onWebSocketMessage(handler: (message: WsMessage) => WsResult): void;
}
`;

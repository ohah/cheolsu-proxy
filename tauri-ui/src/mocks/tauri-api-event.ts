import type { ProxyEventTuple } from "@/entities/proxy";
import { MOCK_TRANSACTIONS } from "./mock-data";

type UnlistenFn = () => void;
type EventCallback<T> = (event: { payload: T }) => void;

const listeners = new Map<string, Set<EventCallback<any>>>();

export async function listen<T>(
  event: string,
  handler: EventCallback<T>,
): Promise<UnlistenFn> {
  if (!listeners.has(event)) {
    listeners.set(event, new Set());
  }
  listeners.get(event)!.add(handler);

  // proxy_event: 목 트랜잭션을 순차적으로 전송
  if (event === "proxy_event") {
    startMockStream(handler as EventCallback<ProxyEventTuple>);
  }

  return () => {
    listeners.get(event)?.delete(handler);
  };
}

export async function emit(event: string, payload?: any): Promise<void> {
  listeners.get(event)?.forEach((handler) => handler({ payload }));
}

let streamStarted = false;

function startMockStream(handler: EventCallback<ProxyEventTuple>) {
  if (streamStarted) return;
  streamStarted = true;

  let index = 0;

  // 초기 데이터를 빠르게 전송
  const sendNext = () => {
    if (index >= MOCK_TRANSACTIONS.length) {
      // 모든 초기 데이터 전송 완료 후, 랜덤으로 새 트래픽 생성
      streamStarted = false;
      startRandomStream();
      return;
    }

    const tx = MOCK_TRANSACTIONS[index++];
    handler({ payload: [tx.request, tx.response] });

    setTimeout(sendNext, 80 + Math.random() * 200);
  };

  setTimeout(sendNext, 500);
}

function startRandomStream() {
  const methods = ["GET", "POST", "PUT", "DELETE", "PATCH"];
  const statuses = [200, 201, 204, 301, 400, 401, 403, 404, 500, 502];
  const hosts = [
    "api.example.com",
    "cdn.example.com",
    "auth.example.com",
    "ws.example.com",
    "www.example.com",
  ];
  const paths = [
    "/v1/users",
    "/v1/products",
    "/v1/orders",
    "/v1/auth/token",
    "/v1/files",
    "/v1/search",
    "/v1/notifications",
    "/v1/analytics/events",
    "/v2/graphql",
    "/health",
  ];

  let counter = 1000;

  const generate = () => {
    const id = `mock-rand-${++counter}-${Date.now()}`;
    const method = methods[Math.floor(Math.random() * methods.length)];
    const host = hosts[Math.floor(Math.random() * hosts.length)];
    const path = paths[Math.floor(Math.random() * paths.length)];
    const status = statuses[Math.floor(Math.random() * statuses.length)];

    const payload: ProxyEventTuple = [
      {
        method,
        uri: `https://${host}${path}`,
        version: "HTTP/2.0",
        headers: { "content-type": "application/json", host },
        body: null,
        time: Date.now(),
        id,
        data_type: "Json",
        body_json: method !== "GET" ? { mock: true } : undefined,
        body_size: method !== "GET" ? 15 : 0,
      },
      {
        status,
        version: "HTTP/2.0",
        headers: { "content-type": "application/json" },
        body: null,
        time: Date.now() + Math.round(Math.random() * 500),
        id,
        data_type: "Json",
        body_json: { ok: status < 400 },
        body_size: 15,
      },
    ];

    listeners.get("proxy_event")?.forEach((handler) => handler({ payload }));

    const delay = 2000 + Math.random() * 5000;
    setTimeout(generate, delay);
  };

  setTimeout(generate, 3000);
}

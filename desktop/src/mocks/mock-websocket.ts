import type { HttpTransaction } from "@/entities/proxy";

import { tx } from "./mock-helpers";

/** WebSocket 업그레이드 트랜잭션 */
export const MOCK_WEBSOCKET_TRANSACTIONS: HttpTransaction[] = [
  // 19. WebSocket upgrade
  tx(
    {
      method: "GET",
      uri: "wss://ws.example.com/realtime",
      headers: {
        upgrade: "websocket",
        connection: "Upgrade",
        "sec-websocket-key": "dGhlIHNhbXBsZSBub25jZQ==",
        "sec-websocket-version": "13",
      },
      data_type: "Empty",
      body_size: 0,
    },
    {
      status: 101,
      headers: {
        upgrade: "websocket",
        connection: "Upgrade",
        "sec-websocket-accept": "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=",
      },
      data_type: "Empty",
      body_size: 0,
    },
  ),
];

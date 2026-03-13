import { describe, test, expect, beforeEach } from "bun:test";
import "../../test/mocks/tauri";

import type { WsMessageInfo } from "../../src/entities/websocket";

async function getStore() {
  const { useWebSocketStore } = await import("../../src/shared/stores/websocket-store");
  return useWebSocketStore;
}

function createMessage(
  connectionId: string,
  overrides: Partial<WsMessageInfo> = {},
): WsMessageInfo {
  return {
    connection_id: connectionId,
    direction: "client_to_server",
    payload: "hello",
    is_binary: false,
    time: Date.now(),
    content_type: "plain",
    ...overrides,
  } as WsMessageInfo;
}

describe("websocket-store", () => {
  let store: Awaited<ReturnType<typeof getStore>>;

  beforeEach(async () => {
    store = await getStore();
    store.setState({
      messages: [],
      connections: new Map(),
      selectedConnectionId: null,
      selectedMessage: null,
    });
  });

  test("초기 상태", () => {
    const state = store.getState();
    expect(state.messages).toEqual([]);
    expect(state.connections.size).toBe(0);
    expect(state.selectedConnectionId).toBeNull();
    expect(state.selectedMessage).toBeNull();
  });

  test("addMessages: 새 연결 자동 생성", () => {
    const msg = createMessage("conn-1");
    store.getState().addMessages([msg]);

    const state = store.getState();
    expect(state.messages).toHaveLength(1);
    expect(state.connections.has("conn-1")).toBe(true);
    const conn = state.connections.get("conn-1")!;
    expect(conn.status).toBe("connected");
    expect(conn.messageCount).toBe(1);
  });

  test("addMessages: 기존 연결에 메시지 추가", () => {
    store.getState().addMessages([createMessage("conn-1")]);
    store.getState().addMessages([createMessage("conn-1")]);
    store.getState().addMessages([createMessage("conn-1")]);

    const conn = store.getState().connections.get("conn-1")!;
    expect(conn.messageCount).toBe(3);
    expect(store.getState().messages).toHaveLength(3);
  });

  test("addMessages: 배치로 여러 메시지 한번에 추가", () => {
    store
      .getState()
      .addMessages([createMessage("conn-1"), createMessage("conn-1"), createMessage("conn-2")]);

    expect(store.getState().messages).toHaveLength(3);
    expect(store.getState().connections.size).toBe(2);
    expect(store.getState().connections.get("conn-1")!.messageCount).toBe(2);
    expect(store.getState().connections.get("conn-2")!.messageCount).toBe(1);
  });

  test("addMessages: 여러 연결 동시 관리", () => {
    store.getState().addMessages([createMessage("conn-1")]);
    store.getState().addMessages([createMessage("conn-2")]);

    expect(store.getState().connections.size).toBe(2);
    expect(store.getState().connections.get("conn-1")!.messageCount).toBe(1);
    expect(store.getState().connections.get("conn-2")!.messageCount).toBe(1);
  });

  test("updateConnection: connected 상태 설정", () => {
    const time = 1000;
    store.getState().updateConnection("conn-1", "connected", "wss://example.com", time);

    const conn = store.getState().connections.get("conn-1")!;
    expect(conn.status).toBe("connected");
    expect(conn.uri).toBe("wss://example.com");
    expect(conn.startTime).toBe(1000);
    expect(conn.messageCount).toBe(0);
  });

  test("updateConnection: disconnected 상태 설정", () => {
    store.getState().updateConnection("conn-1", "connected", "wss://example.com", 1000);
    store.getState().updateConnection("conn-1", "disconnected", undefined, 2000);

    const conn = store.getState().connections.get("conn-1")!;
    expect(conn.status).toBe("disconnected");
    expect(conn.endTime).toBe(2000);
  });

  test("updateConnection: 존재하지 않는 연결 disconnect 무시", () => {
    store.getState().updateConnection("nonexistent", "disconnected");
    expect(store.getState().connections.has("nonexistent")).toBe(false);
  });

  test("setSelectedConnectionId: 선택 시 메시지 선택 초기화", () => {
    const msg = createMessage("conn-1");
    store.getState().addMessages([msg]);
    store.getState().setSelectedMessage(msg);
    expect(store.getState().selectedMessage).not.toBeNull();

    store.getState().setSelectedConnectionId("conn-1");
    expect(store.getState().selectedConnectionId).toBe("conn-1");
    expect(store.getState().selectedMessage).toBeNull();
  });

  test("setSelectedMessage: 메시지 선택/해제", () => {
    const msg = createMessage("conn-1");
    store.getState().setSelectedMessage(msg);
    expect(store.getState().selectedMessage).toEqual(msg);

    store.getState().setSelectedMessage(null);
    expect(store.getState().selectedMessage).toBeNull();
  });

  test("clearAll: 전체 초기화", () => {
    store.getState().addMessages([createMessage("conn-1")]);
    store.getState().setSelectedConnectionId("conn-1");

    store.getState().clearAll();
    const state = store.getState();
    expect(state.messages).toEqual([]);
    expect(state.connections.size).toBe(0);
    expect(state.selectedConnectionId).toBeNull();
    expect(state.selectedMessage).toBeNull();
  });
});

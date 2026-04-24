import { mock } from "bun:test";

// @tauri-apps/api/core mock
// @tauri-apps/plugin-fs 2.5.0+가 Resource/Channel을 import하므로 함께 export해야 모듈 로드 실패로 인한 테스트 간 side-effect를 막을 수 있다.
class MockResource {
  readonly rid = 0;
  async close() {}
}
class MockChannel {
  onmessage?: (msg: unknown) => void;
  id = 0;
}
mock.module("@tauri-apps/api/core", () => ({
  invoke: mock(() => Promise.resolve()),
  Resource: MockResource,
  Channel: MockChannel,
  transformCallback: mock(() => 0),
  convertFileSrc: mock((path: string) => path),
  isTauri: mock(() => false),
  SERIALIZE_TO_IPC_FN: "__TAURI_TO_IPC_KEY__",
}));

// @tauri-apps/api/event mock
type EventCallback<T> = (event: { payload: T }) => void;
const eventListeners = new Map<string, Set<EventCallback<unknown>>>();

export function getEventListeners() {
  return eventListeners;
}

export function simulateEvent<T>(event: string, payload: T) {
  eventListeners.get(event)?.forEach((handler) => handler({ payload }));
}

export function clearEventListeners() {
  eventListeners.clear();
}

const mockListen = mock(async (event: string, handler: EventCallback<unknown>) => {
  if (!eventListeners.has(event)) {
    eventListeners.set(event, new Set());
  }
  eventListeners.get(event)!.add(handler);
  return () => {
    eventListeners.get(event)?.delete(handler);
  };
});

const mockEmitTo = mock(async (_target: string, _event: string, _payload?: unknown) => {});

mock.module("@tauri-apps/api/event", () => ({
  listen: mockListen,
  emit: mock(async () => {}),
  emitTo: mockEmitTo,
}));

// @tauri-apps/plugin-store mock (메모리 기반)
const storeData = new Map<string, Map<string, unknown>>();

mock.module("@tauri-apps/plugin-store", () => ({
  load: mock((_name: string) => {
    if (!storeData.has(_name)) storeData.set(_name, new Map());
    const data = storeData.get(_name)!;
    return Promise.resolve({
      get: mock((key: string) => Promise.resolve(data.get(key) ?? null)),
      set: mock((key: string, value: unknown) => {
        data.set(key, value);
        return Promise.resolve();
      }),
      delete: mock((key: string) => {
        data.delete(key);
        return Promise.resolve();
      }),
      save: mock(() => Promise.resolve()),
    });
  }),
}));

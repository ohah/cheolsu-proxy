import { mock } from "bun:test";

// @tauri-apps/api/core mock
mock.module("@tauri-apps/api/core", () => ({
  invoke: mock(() => Promise.resolve()),
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

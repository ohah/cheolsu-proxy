import { mock } from "bun:test";

// @tauri-apps/api/core mock
mock.module("@tauri-apps/api/core", () => ({
  invoke: mock(() => Promise.resolve()),
}));

// @tauri-apps/plugin-store mock
mock.module("@tauri-apps/plugin-store", () => ({
  load: mock(() =>
    Promise.resolve({
      get: mock(() => Promise.resolve([])),
      set: mock(() => Promise.resolve()),
      save: mock(() => Promise.resolve()),
    }),
  ),
}));

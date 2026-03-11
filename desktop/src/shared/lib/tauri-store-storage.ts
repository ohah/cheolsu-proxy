import { load } from "@tauri-apps/plugin-store";
import type { StateStorage } from "zustand/middleware";

const storeCache = new Map<string, Awaited<ReturnType<typeof load>>>();

async function getStore(name: string) {
  if (!storeCache.has(name)) {
    const store = await load(`${name}.json`, { defaults: {}, autoSave: true });
    storeCache.set(name, store);
  }
  return storeCache.get(name)!;
}

export function createTauriStorage(): StateStorage {
  return {
    getItem: async (name: string): Promise<string | null> => {
      const store = await getStore(name);
      return (await store.get<string>("state")) ?? null;
    },
    setItem: async (name: string, value: string): Promise<void> => {
      const store = await getStore(name);
      await store.set("state", value);
    },
    removeItem: async (name: string): Promise<void> => {
      const store = await getStore(name);
      await store.delete("state");
      await store.save();
    },
  };
}

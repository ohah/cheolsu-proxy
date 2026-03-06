const memoryStore = new Map<string, any>();

const storeInstance = {
  get: async (key: string) => memoryStore.get(key) ?? null,
  set: async (key: string, value: any) => {
    memoryStore.set(key, value);
  },
  save: async () => {},
  delete: async (key: string) => {
    memoryStore.delete(key);
  },
  clear: async () => {
    memoryStore.clear();
  },
  keys: async () => Array.from(memoryStore.keys()),
  values: async () => Array.from(memoryStore.values()),
  entries: async () => Array.from(memoryStore.entries()),
  length: async () => memoryStore.size,
  has: async (key: string) => memoryStore.has(key),
  onKeyChange: async () => () => {},
  onChange: async () => () => {},
  close: async () => {},
  reload: async () => {},
};

export async function load(_path: string, _options?: any) {
  return storeInstance;
}

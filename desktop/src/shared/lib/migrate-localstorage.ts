import { createTauriStorage } from "./tauri-store-storage";

const STORE_NAMES = [
  "cheolsu-app-settings",
  "cheolsu-proxy-store",
  "cheolsu-ssl-proxying",
  "cheolsu-intercept-rules",
  "cheolsu-host-mappings",
  "cheolsu-server-replay",
  "cheolsu-breakpoint-rules",
  "cheolsu-map-rules",
];

const MIGRATION_KEY = "cheolsu-store-migrated-v1";

export async function migrateLocalStorageToTauriStore(): Promise<void> {
  if (localStorage.getItem(MIGRATION_KEY)) return;

  const storage = createTauriStorage();

  for (const name of STORE_NAMES) {
    const data = localStorage.getItem(name);
    if (data) {
      await storage.setItem(name, data);
      localStorage.removeItem(name);
    }
  }

  localStorage.setItem(MIGRATION_KEY, Date.now().toString());
}

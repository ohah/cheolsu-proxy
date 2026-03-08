import { LazyStore } from "@tauri-apps/plugin-store";

export const trayStore = new LazyStore("tray-sync.json");

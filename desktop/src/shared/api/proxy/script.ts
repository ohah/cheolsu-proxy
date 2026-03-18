import { invoke } from "@tauri-apps/api/core";

export async function loadScript(path?: string, code?: string): Promise<void> {
  return invoke("load_script", { path, code });
}

export async function unloadScript(): Promise<void> {
  return invoke("unload_script");
}

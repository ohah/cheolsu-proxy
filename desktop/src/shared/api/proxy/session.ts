import { invoke } from "@tauri-apps/api/core";

export interface LoadSessionResult {
  name: string | null;
  description: string | null;
  transaction_count: number;
  transactions_json: string;
}

export async function saveSession(path: string, transactionsJson: string): Promise<void> {
  return invoke("save_session", { path, transactionsJson });
}

export async function loadSession(path: string): Promise<LoadSessionResult> {
  return invoke("load_session", { path });
}

export async function importHarFile(path: string): Promise<string> {
  return invoke("import_har_file_cmd", { path });
}

// ─── Auto Session ────────────────────────────────────────

export async function autosaveSession(transactionsJson: string): Promise<void> {
  return invoke("autosave_session", { transactionsJson });
}

export async function autoloadSession(): Promise<LoadSessionResult | null> {
  return invoke("autoload_session");
}

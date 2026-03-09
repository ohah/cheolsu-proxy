import { register, unregister, isRegistered } from "@tauri-apps/plugin-global-shortcut";

const STORAGE_KEY = "proxy_toggle_shortcut";
const DEFAULT_SHORTCUT = "CommandOrControl+Shift+P";

// 현재 등록된 단축키 추적 (모듈 레벨 — 싱글톤)
let currentRegisteredShortcut: string | null = null;

export function getStoredShortcut(): string {
  return localStorage.getItem(STORAGE_KEY) || DEFAULT_SHORTCUT;
}

export function setStoredShortcut(shortcut: string) {
  localStorage.setItem(STORAGE_KEY, shortcut);
}

export function getShortcutEnabled(): boolean {
  const val = localStorage.getItem(STORAGE_KEY + "_enabled");
  return val === null ? true : val === "true";
}

export function setShortcutEnabled(enabled: boolean) {
  localStorage.setItem(STORAGE_KEY + "_enabled", String(enabled));
}

export async function registerShortcut(shortcut: string, onPressed: () => void) {
  // 이전 단축키 해제
  if (currentRegisteredShortcut) {
    try {
      const wasRegistered = await isRegistered(currentRegisteredShortcut);
      if (wasRegistered) {
        await unregister(currentRegisteredShortcut);
      }
    } catch {
      // 해제 실패 무시
    }
    currentRegisteredShortcut = null;
  }

  if (!shortcut) return;

  await register(shortcut, (event) => {
    if (event.state === "Pressed") {
      onPressed();
    }
  });
  currentRegisteredShortcut = shortcut;
}

export async function unregisterShortcut() {
  if (currentRegisteredShortcut) {
    try {
      await unregister(currentRegisteredShortcut);
    } catch {
      // 무시
    }
    currentRegisteredShortcut = null;
  }
}

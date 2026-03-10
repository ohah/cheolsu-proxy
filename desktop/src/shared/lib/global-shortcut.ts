import { register, unregister, isRegistered } from "@tauri-apps/plugin-global-shortcut";
import { useAppSettingsStore } from "@/shared/stores/app-settings-store";

// 현재 등록된 단축키 추적 (모듈 레벨 — 싱글톤)
let currentRegisteredShortcut: string | null = null;

export function getStoredShortcut(): string {
  return useAppSettingsStore.getState().proxyToggleShortcut;
}

export function setStoredShortcut(shortcut: string) {
  useAppSettingsStore.getState().setProxyToggleShortcut(shortcut);
}

export function getShortcutEnabled(): boolean {
  return useAppSettingsStore.getState().proxyToggleShortcutEnabled;
}

export function setShortcutEnabled(enabled: boolean) {
  useAppSettingsStore.getState().setProxyToggleShortcutEnabled(enabled);
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

import { useEffect } from "react";
import { register, unregister, isRegistered } from "@tauri-apps/plugin-global-shortcut";
import { useProxyStore } from "@/shared/stores";
import { startProxyV2, stopProxyV2 } from "@/shared/api/proxy";
import { toast } from "sonner";
import { trayStore } from "@/shared/stores/tray-sync-store";

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

export async function toggleProxy() {
  const { isConnected, port } = useProxyStore.getState();

  try {
    if (isConnected) {
      await stopProxyV2();
      useProxyStore.getState().setConnected(false);
      await trayStore.set("proxyConnected", false);
      await trayStore.save();
      toast.info("Proxy stopped");
    } else {
      await startProxyV2(port);
      useProxyStore.getState().setConnected(true);
      await trayStore.set("proxyConnected", true);
      await trayStore.save();
      toast.success("Proxy started");
    }
  } catch {
    toast.error("Proxy toggle failed");
  }
}

export async function registerShortcut(shortcut: string) {
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

  if (!shortcut || !getShortcutEnabled()) return;

  await register(shortcut, (event) => {
    if (event.state === "Pressed") {
      toggleProxy();
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

/**
 * 앱 시작 시 글로벌 단축키를 자동 등록하는 훅.
 * App.tsx에서 한 번만 호출해야 합니다.
 */
export function useGlobalShortcut() {
  useEffect(() => {
    if (getShortcutEnabled()) {
      const shortcut = getStoredShortcut();
      registerShortcut(shortcut).catch(() => {
        // 앱 시작 시 등록 실패 무시
      });
    }

    return () => {
      unregisterShortcut();
    };
  }, []);
}

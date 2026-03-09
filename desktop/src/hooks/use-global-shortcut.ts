import { useEffect, useRef, useCallback } from "react";
import { register, unregister, isRegistered } from "@tauri-apps/plugin-global-shortcut";
import { useProxyStore } from "@/shared/stores";
import { startProxyV2, stopProxyV2 } from "@/shared/api/proxy";
import { toast } from "sonner";
import { trayStore } from "@/shared/stores/tray-sync-store";

const STORAGE_KEY = "proxy_toggle_shortcut";
const DEFAULT_SHORTCUT = "CommandOrControl+Shift+P";

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

export function useGlobalShortcut() {
  const currentShortcutRef = useRef<string | null>(null);

  const toggleProxy = useCallback(async () => {
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
    } catch (e) {
      console.error("Proxy toggle failed:", e);
      toast.error("Proxy toggle failed");
    }
  }, []);

  const registerShortcut = useCallback(
    async (shortcut: string) => {
      // 이전 단축키 해제
      if (currentShortcutRef.current) {
        try {
          const wasRegistered = await isRegistered(currentShortcutRef.current);
          if (wasRegistered) {
            await unregister(currentShortcutRef.current);
          }
        } catch {
          // 해제 실패 무시
        }
        currentShortcutRef.current = null;
      }

      if (!shortcut || !getShortcutEnabled()) return;

      try {
        await register(shortcut, (event) => {
          if (event.state === "Pressed") {
            toggleProxy();
          }
        });
        currentShortcutRef.current = shortcut;
      } catch (e) {
        console.error("Failed to register shortcut:", shortcut, e);
      }
    },
    [toggleProxy],
  );

  const unregisterShortcut = useCallback(async () => {
    if (currentShortcutRef.current) {
      try {
        await unregister(currentShortcutRef.current);
      } catch {
        // 무시
      }
      currentShortcutRef.current = null;
    }
  }, []);

  // 앱 시작 시 저장된 단축키 등록
  useEffect(() => {
    if (getShortcutEnabled()) {
      const shortcut = getStoredShortcut();
      registerShortcut(shortcut);
    }

    return () => {
      unregisterShortcut();
    };
  }, [registerShortcut, unregisterShortcut]);

  return { registerShortcut, unregisterShortcut };
}

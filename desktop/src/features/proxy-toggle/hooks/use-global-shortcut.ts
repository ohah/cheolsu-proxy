import { useEffect } from "react";
import {
  getShortcutEnabled,
  getStoredShortcut,
  registerShortcut,
  unregisterShortcut,
} from "@/shared/lib/global-shortcut";
import { toggleProxy } from "../lib/toggle-proxy";

/**
 * 앱 시작 시 글로벌 단축키를 자동 등록하는 훅.
 * App.tsx에서 한 번만 호출해야 합니다.
 */
export function useGlobalShortcut() {
  useEffect(() => {
    if (getShortcutEnabled()) {
      const shortcut = getStoredShortcut();
      registerShortcut(shortcut, toggleProxy).catch(() => {
        // 앱 시작 시 등록 실패 무시
      });
    }

    return () => {
      unregisterShortcut();
    };
  }, []);
}

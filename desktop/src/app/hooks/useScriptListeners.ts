import { useEffect } from "react";
import { useScriptStore } from "@/shared/stores";
import { listen } from "@tauri-apps/api/event";

/**
 * 스크립트 로그 및 상태 이벤트 수신을 담당하는 훅
 */
export function useScriptListeners() {
  const setScriptStatus = useScriptStore((s) => s.setStatus);
  const addScriptLog = useScriptStore((s) => s.addLog);

  // 스크립트 이벤트 수신
  useEffect(() => {
    const unlistenLog = listen<{ level: string; message: string }>("script_log", (event) => {
      addScriptLog(event.payload.level, event.payload.message);
    });
    const unlistenStatus = listen<{ active: boolean; path: string | null }>(
      "script_status",
      (event) => {
        setScriptStatus(event.payload.active, event.payload.path);
      },
    );

    return () => {
      unlistenLog.then((f) => f());
      unlistenStatus.then((f) => f());
    };
  }, [addScriptLog, setScriptStatus]);
}

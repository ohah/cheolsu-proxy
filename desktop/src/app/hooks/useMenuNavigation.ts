import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { router } from "../providers/router-provider";

/**
 * 네이티브 메뉴 클릭 시 페이지 이동을 담당하는 훅
 */
export function useMenuNavigation() {
  // 네이티브 메뉴 클릭 시 페이지 이동
  useEffect(() => {
    const unlisten = listen<string>("menu_navigate", (event) => {
      router.navigate(event.payload);
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, []);
}

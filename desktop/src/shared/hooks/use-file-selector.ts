import { useCallback } from "react";
import { open } from "@tauri-apps/plugin-dialog";

export function useFileSelector() {
  return useCallback(async (options: { extensions?: string[]; title?: string }) => {
    const selected = await open({
      multiple: false,
      filters: options.extensions ? [{ name: "Files", extensions: options.extensions }] : undefined,
      title: options.title,
    });
    return typeof selected === "string" ? selected : null;
  }, []);
}

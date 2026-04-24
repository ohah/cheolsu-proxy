import { useCallback } from "react";
import { useTheme } from "next-themes";
import type { Monaco } from "@monaco-editor/react";

import { getMonacoThemeName, setupMonacoThemes } from "@/shared/lib/monaco-theme";

/**
 * Monaco Editor에 cheolsu 테마를 적용하기 위한 훅.
 * `beforeMount`에서 전체 테마를 등록하고, `theme`은 현재 resolvedTheme에 맞춰 선택한다.
 *
 * 사용 예:
 * ```tsx
 * const { theme, beforeMount } = useMonacoTheme();
 * <Editor theme={theme} beforeMount={beforeMount} ... />
 * ```
 */
export function useMonacoTheme() {
  const { resolvedTheme } = useTheme();

  const beforeMount = useCallback((monaco: Monaco) => {
    setupMonacoThemes(monaco);
  }, []);

  return {
    theme: getMonacoThemeName(resolvedTheme),
    beforeMount,
  };
}

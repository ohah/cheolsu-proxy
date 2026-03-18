import type { editor } from "monaco-editor";

/** Custom dark theme keys — single source of truth for all theme registrations */
export const CUSTOM_THEME_KEYS = [
  "dracula",
  "nord",
  "monokai",
  "solarized-dark",
  "github-dark",
] as const;

export type CustomThemeKey = (typeof CUSTOM_THEME_KEYS)[number];

/** Custom dark themes are registered with per-theme background/foreground colors */
const CUSTOM_DARK_THEMES: Record<
  string,
  {
    bg: string;
    fg: string;
    widgetBg: string;
    widgetBorder: string;
    selectBg: string;
    hoverBg: string;
  }
> = {
  dracula: {
    bg: "#282a36",
    fg: "#f8f8f2",
    widgetBg: "#21222c",
    widgetBorder: "#44475a",
    selectBg: "#44475a",
    hoverBg: "#44475a",
  },
  nord: {
    bg: "#2e3440",
    fg: "#d8dee9",
    widgetBg: "#3b4252",
    widgetBorder: "#4c566a",
    selectBg: "#4c566a",
    hoverBg: "#3b4252",
  },
  monokai: {
    bg: "#272822",
    fg: "#f8f8f2",
    widgetBg: "#1e1f1c",
    widgetBorder: "#3e3d32",
    selectBg: "#3e3d32",
    hoverBg: "#3e3d32",
  },
  "solarized-dark": {
    bg: "#002b36",
    fg: "#839496",
    widgetBg: "#073642",
    widgetBorder: "#073642",
    selectBg: "#073642",
    hoverBg: "#073642",
  },
  "github-dark": {
    bg: "#0d1117",
    fg: "#c9d1d9",
    widgetBg: "#161b22",
    widgetBorder: "#30363d",
    selectBg: "#30363d",
    hoverBg: "#161b22",
  },
};

export function getMonacoThemeName(resolvedTheme: string | undefined): string {
  if (!resolvedTheme || resolvedTheme === "light") return "cheolsu-light";
  if (resolvedTheme === "dark") return "cheolsu-dark";
  return `cheolsu-${resolvedTheme}`;
}

export function buildCustomMonacoTheme(themeKey: string): editor.IStandaloneThemeData | null {
  const colors = CUSTOM_DARK_THEMES[themeKey];
  if (!colors) return null;
  return {
    base: "vs-dark",
    inherit: true,
    rules: DARK_THEME.rules,
    colors: {
      ...DARK_THEME.colors,
      "editor.background": colors.bg,
      "editor.foreground": colors.fg,
      "editorWidget.background": colors.widgetBg,
      "editorWidget.border": colors.widgetBorder,
      "editorWidget.foreground": colors.fg,
      "editorSuggestWidget.background": colors.widgetBg,
      "editorSuggestWidget.border": colors.widgetBorder,
      "editorSuggestWidget.foreground": colors.fg,
      "editorSuggestWidget.selectedBackground": colors.selectBg,
      "editorSuggestWidget.selectedForeground": colors.fg,
      "list.hoverBackground": colors.hoverBg,
      "list.hoverForeground": colors.fg,
      "list.activeSelectionBackground": colors.selectBg,
      "list.activeSelectionForeground": colors.fg,
      "list.inactiveSelectionBackground": colors.hoverBg,
      "list.focusBackground": colors.selectBg,
      "list.focusForeground": colors.fg,
    },
  };
}

export const LIGHT_THEME: editor.IStandaloneThemeData = {
  base: "vs",
  inherit: true,
  rules: [
    { token: "keyword", foreground: "0d0d0d", fontStyle: "bold" },
    { token: "keyword.control", foreground: "e74322", fontStyle: "bold" },
    { token: "string", foreground: "5aab9f" },
    { token: "operator", foreground: "00bcd5", fontStyle: "bold" },
    { token: "delimiter", foreground: "94a3b8" },
  ],
  colors: {
    "editor.background": "#ffffff",
    "editor.foreground": "#0d0d0d",

    // 자동완성 위젯 색상
    "editorWidget.background": "#ffffff",
    "editorWidget.border": "#e5e7eb",
    "editorWidget.foreground": "#0d0d0d",

    // 자동완성 제안 색상 - 더 강조
    "editorSuggestWidget.background": "#ffffff",
    "editorSuggestWidget.border": "#e5e7eb",
    "editorSuggestWidget.foreground": "#0d0d0d", // 기본 텍스트
    "editorSuggestWidget.selectedBackground": "#e0f2fe", // 선택된 항목 배경 (밝은 파란색)
    "editorSuggestWidget.selectedForeground": "#0c4a6e", // 선택된 항목 텍스트
    "editorSuggestWidget.highlightForeground": "#0ea5e9", // 매칭된 텍스트 강조
    "editorSuggestWidget.focusHighlightForeground": "#0284c7", // 포커스된 매칭 텍스트

    // 리스트 색상
    "list.hoverBackground": "#f1f5f9",
    "list.hoverForeground": "#0d0d0d",
    "list.activeSelectionBackground": "#e0f2fe",
    "list.activeSelectionForeground": "#0c4a6e",
    "list.inactiveSelectionBackground": "#f1f5f9",
    "list.inactiveSelectionForeground": "#0d0d0d",
    "list.focusBackground": "#e0f2fe",
    "list.focusForeground": "#0c4a6e",
  },
};

export const DARK_THEME: editor.IStandaloneThemeData = {
  base: "vs-dark",
  inherit: true,
  rules: [
    { token: "keyword", foreground: "45cbda", fontStyle: "bold" },
    { token: "keyword.control", foreground: "f87171", fontStyle: "bold" },
    { token: "string", foreground: "5aab9f" },
    { token: "operator", foreground: "45cbda", fontStyle: "bold" },
    { token: "delimiter", foreground: "64748b" },
  ],
  colors: {
    "editor.background": "#0a0a0a",
    "editor.foreground": "#b4b4b9",

    // 자동완성 위젯 색상
    "editorWidget.background": "#141414",
    "editorWidget.border": "#262626",
    "editorWidget.foreground": "#b4b4b9",

    // 자동완성 제안 색상 - 더 강조
    "editorSuggestWidget.background": "#141414",
    "editorSuggestWidget.border": "#262626",
    "editorSuggestWidget.foreground": "#b4b4b9", // 기본 텍스트 (밝게)
    "editorSuggestWidget.selectedBackground": "#0e7490", // 선택된 항목 배경 (청록색)
    "editorSuggestWidget.selectedForeground": "#b4b4b9", // 선택된 항목 텍스트
    "editorSuggestWidget.highlightForeground": "#22d3ee", // 매칭된 텍스트 강조
    "editorSuggestWidget.focusHighlightForeground": "#06b6d4", // 포커스된 매칭 텍스트

    // 검색 색상
    "editor.findMatchBackground": "#0e749080",
    "editor.findMatchHighlightBackground": "#45cbda30",
    "editor.findMatchBorder": "#0e7490",
    "editor.findMatchHighlightBorder": "#45cbda50",
    "editorFindWidget.background": "#141414",
    "editorFindWidget.border": "#262626",
    "editorFindWidget.foreground": "#b4b4b9",

    // 리스트 색상
    "list.hoverBackground": "#1c1c1c",
    "list.hoverForeground": "#b4b4b9",
    "list.activeSelectionBackground": "#0e7490",
    "list.activeSelectionForeground": "#b4b4b9",
    "list.inactiveSelectionBackground": "#1c1c1c",
    "list.inactiveSelectionForeground": "#d4d4d8",
    "list.focusBackground": "#0e7490",
    "list.focusForeground": "#b4b4b9",
  },
};

import type { editor } from "monaco-editor";

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

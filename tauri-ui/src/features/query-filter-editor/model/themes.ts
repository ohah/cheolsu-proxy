import type { editor } from 'monaco-editor';

export const LIGHT_THEME: editor.IStandaloneThemeData = {
  base: 'vs',
  inherit: true,
  rules: [
    { token: 'keyword', foreground: '0d0d0d', fontStyle: 'bold' },
    { token: 'keyword.control', foreground: 'e74322', fontStyle: 'bold' },
    { token: 'string', foreground: '5aab9f' },
    { token: 'operator', foreground: '00bcd5', fontStyle: 'bold' },
    { token: 'delimiter', foreground: '94a3b8' },
  ],
  colors: {
    'editor.background': '#ffffff',
    'editor.foreground': '#0d0d0d',

    // 자동완성 위젯 색상
    'editorWidget.background': '#ffffff',
    'editorWidget.border': '#e5e7eb',
    'editorWidget.foreground': '#0d0d0d',

    // 자동완성 제안 색상 - 더 강조
    'editorSuggestWidget.background': '#ffffff',
    'editorSuggestWidget.border': '#e5e7eb',
    'editorSuggestWidget.foreground': '#0d0d0d', // 기본 텍스트
    'editorSuggestWidget.selectedBackground': '#e0f2fe', // 선택된 항목 배경 (밝은 파란색)
    'editorSuggestWidget.selectedForeground': '#0c4a6e', // 선택된 항목 텍스트
    'editorSuggestWidget.highlightForeground': '#0ea5e9', // 매칭된 텍스트 강조
    'editorSuggestWidget.focusHighlightForeground': '#0284c7', // 포커스된 매칭 텍스트

    // 리스트 색상
    'list.hoverBackground': '#f1f5f9',
    'list.hoverForeground': '#0d0d0d',
    'list.activeSelectionBackground': '#e0f2fe',
    'list.activeSelectionForeground': '#0c4a6e',
    'list.inactiveSelectionBackground': '#f1f5f9',
    'list.inactiveSelectionForeground': '#0d0d0d',
    'list.focusBackground': '#e0f2fe',
    'list.focusForeground': '#0c4a6e',
  },
};

export const DARK_THEME: editor.IStandaloneThemeData = {
  base: 'vs-dark',
  inherit: true,
  rules: [
    { token: 'keyword', foreground: '45cbda', fontStyle: 'bold' },
    { token: 'keyword.control', foreground: 'f87171', fontStyle: 'bold' },
    { token: 'string', foreground: '5aab9f' },
    { token: 'operator', foreground: '45cbda', fontStyle: 'bold' },
    { token: 'delimiter', foreground: '64748b' },
  ],
  colors: {
    'editor.background': '#0f172a',
    'editor.foreground': '#f1f5f9',

    // 자동완성 위젯 색상
    'editorWidget.background': '#1e293b',
    'editorWidget.border': '#334155',
    'editorWidget.foreground': '#f1f5f9',

    // 자동완성 제안 색상 - 더 강조
    'editorSuggestWidget.background': '#1e293b',
    'editorSuggestWidget.border': '#334155',
    'editorSuggestWidget.foreground': '#f1f5f9', // 기본 텍스트 (밝게)
    'editorSuggestWidget.selectedBackground': '#0e7490', // 선택된 항목 배경 (청록색)
    'editorSuggestWidget.selectedForeground': '#ffffff', // 선택된 항목 텍스트 (흰색)
    'editorSuggestWidget.highlightForeground': '#22d3ee', // 매칭된 텍스트 강조
    'editorSuggestWidget.focusHighlightForeground': '#06b6d4', // 포커스된 매칭 텍스트

    // 리스트 색상
    'list.hoverBackground': '#334155',
    'list.hoverForeground': '#f1f5f9',
    'list.activeSelectionBackground': '#0e7490',
    'list.activeSelectionForeground': '#ffffff',
    'list.inactiveSelectionBackground': '#334155',
    'list.inactiveSelectionForeground': '#e2e8f0',
    'list.focusBackground': '#0e7490',
    'list.focusForeground': '#ffffff',
  },
};

import type { Monaco } from "@monaco-editor/react";
import {
  LIGHT_THEME,
  DARK_THEME,
  FILTER_KEYWORDS,
  LOGICAL_OPERATORS,
  CUSTOM_THEME_KEYS,
  buildCustomMonacoTheme,
} from "../model";

let isLanguageRegistered = false;

export const setupMonacoLanguage = (monaco: Monaco): void => {
  if (isLanguageRegistered) return;

  // 언어 등록
  monaco.languages.register({ id: "cheolsu-query" });

  // 토크나이저 설정
  monaco.languages.setMonarchTokensProvider("cheolsu-query", {
    tokenizer: {
      root: [
        [new RegExp(`\\b(${FILTER_KEYWORDS.join("|")})\\b`), "keyword"],
        [new RegExp(`\\b(${LOGICAL_OPERATORS.join("|")})\\b`, "i"), "keyword.control"],
        [/"(?:[^"\\]|\\.)*"/, "string"],
        [/'(?:[^'\\]|\\.)*'/, "string"],
        [/`(?:[^`\\]|\\.)*`/, "string"],
        [/(\|=|\|~|!=|!~|=)/, "operator"],
        [/,/, "delimiter"],
      ],
    },
  });

  // 언어 설정
  monaco.languages.setLanguageConfiguration("cheolsu-query", {
    wordPattern: /(-?\d*\.\d\w*)|([^`~!@#%^&*()\-=+[{\]}\\|;:'",.<>/?\s]+)/g,
    brackets: [
      ['"', '"'],
      ["'", "'"],
      ["`", "`"],
    ],
    autoClosingPairs: [
      { open: '"', close: '"' },
      { open: "'", close: "'" },
      { open: "`", close: "`" },
    ],
  });

  // 테마 등록
  monaco.editor.defineTheme("cheolsu-light", LIGHT_THEME);
  monaco.editor.defineTheme("cheolsu-dark", DARK_THEME);

  // 커스텀 다크 테마 등록
  for (const key of CUSTOM_THEME_KEYS) {
    const theme = buildCustomMonacoTheme(key);
    if (theme) {
      monaco.editor.defineTheme(`cheolsu-${key}`, theme);
    }
  }

  isLanguageRegistered = true;
};

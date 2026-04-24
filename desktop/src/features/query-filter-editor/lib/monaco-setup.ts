import type { Monaco } from "@monaco-editor/react";
import { FILTER_KEYWORDS, LOGICAL_OPERATORS } from "../model";

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

  isLanguageRegistered = true;
};

import type { Monaco } from "@monaco-editor/react";
import type { languages, IRange, IDisposable, editor, Position } from "monaco-editor";
import { HTTP_METHODS, STATUS_CODES } from "../model";

export const createCompletionProvider = (monaco: Monaco): IDisposable => {
  return monaco.languages.registerCompletionItemProvider("cheolsu-query", {
    triggerCharacters: ['"', ",", " ", "|", "!", "="],
    provideCompletionItems: (model: editor.ITextModel, position: Position) => {
      const textUntilPosition = model.getValueInRange({
        startLineNumber: 1,
        startColumn: 1,
        endLineNumber: position.lineNumber,
        endColumn: position.column,
      });

      const word = model.getWordUntilPosition(position);
      const range: IRange = {
        startLineNumber: position.lineNumber,
        endLineNumber: position.lineNumber,
        startColumn: word.startColumn,
        endColumn: word.endColumn,
      };

      // 연산자 제안
      if (/(method|methods|status|url|client)\s*$/.test(textUntilPosition)) {
        return {
          suggestions: createOperatorSuggestions(monaco, range),
        };
      }

      // Method 값 제안
      if (/(method|methods)\s*(=|\|=|\|~|!=|!~)\s*"[^"]*$/.test(textUntilPosition)) {
        return {
          suggestions: createMethodSuggestions(monaco, range),
        };
      }

      // Status 값 제안
      if (/status\s*(=|\|=|\|~|!=|!~)\s*"[^"]*$/.test(textUntilPosition)) {
        return {
          suggestions: createStatusSuggestions(monaco, range),
        };
      }

      // URL 값 제안
      if (/url\s*(=|\|=|\|~|!=|!~)\s*"[^"]*$/.test(textUntilPosition)) {
        return {
          suggestions: createUrlSuggestions(monaco, range),
        };
      }

      // 기본 키워드 제안
      return {
        suggestions: createKeywordSuggestions(monaco, range),
      };
    },
  });
};

const createOperatorSuggestions = (monaco: Monaco, range: IRange): languages.CompletionItem[] => {
  return [
    {
      label: "=",
      kind: monaco.languages.CompletionItemKind.Operator,
      insertText: '="',
      detail: "Equals",
      documentation: "Exact match",
      range,
    },
    {
      label: "|=",
      kind: monaco.languages.CompletionItemKind.Operator,
      insertText: '|="',
      detail: "Contains",
      documentation: "String contains",
      range,
    },
    {
      label: "|~",
      kind: monaco.languages.CompletionItemKind.Operator,
      insertText: '|~"',
      detail: "Regex",
      documentation: "Regular expression match",
      range,
    },
    {
      label: "!=",
      kind: monaco.languages.CompletionItemKind.Operator,
      insertText: '!="',
      detail: "Not equals",
      documentation: "Does not equal",
      range,
    },
    {
      label: "!~",
      kind: monaco.languages.CompletionItemKind.Operator,
      insertText: '!~"',
      detail: "Not regex",
      documentation: "Does not match regex",
      range,
    },
  ];
};

const createMethodSuggestions = (monaco: Monaco, range: IRange): languages.CompletionItem[] => {
  return HTTP_METHODS.map((method) => ({
    label: method,
    kind: monaco.languages.CompletionItemKind.EnumMember,
    insertText: method,
    detail: "HTTP Method",
    range,
  }));
};

const createStatusSuggestions = (monaco: Monaco, range: IRange): languages.CompletionItem[] => {
  return STATUS_CODES.map((item) => ({
    label: item.code,
    kind: monaco.languages.CompletionItemKind.EnumMember,
    insertText: item.code,
    detail: item.detail,
    range,
  }));
};

const createUrlSuggestions = (monaco: Monaco, range: IRange): languages.CompletionItem[] => {
  return [
    {
      label: "*",
      kind: monaco.languages.CompletionItemKind.Text,
      insertText: "*",
      detail: "Wildcard",
      range,
    },
    {
      label: "api",
      kind: monaco.languages.CompletionItemKind.Text,
      insertText: "api",
      detail: "API paths",
      range,
    },
  ];
};

const createKeywordSuggestions = (monaco: Monaco, range: IRange): languages.CompletionItem[] => {
  return [
    {
      label: "method",
      kind: monaco.languages.CompletionItemKind.Keyword,
      insertText: "method",
      detail: "Filter by HTTP method",
      range,
    },
    {
      label: "status",
      kind: monaco.languages.CompletionItemKind.Keyword,
      insertText: "status",
      detail: "Filter by status code",
      range,
    },
    {
      label: "url",
      kind: monaco.languages.CompletionItemKind.Keyword,
      insertText: "url",
      detail: "Filter by URL",
      range,
    },
    {
      label: "client",
      kind: monaco.languages.CompletionItemKind.Keyword,
      insertText: "client",
      detail: "Filter by client IP, tag, or auth user",
      range,
    },
    {
      label: "or",
      kind: monaco.languages.CompletionItemKind.Keyword,
      insertText: "or ",
      detail: "Logical OR",
      range,
    },
    {
      label: "and",
      kind: monaco.languages.CompletionItemKind.Keyword,
      insertText: "and ",
      detail: "Logical AND",
      range,
    },
  ];
};

import type { editor } from "monaco-editor";
import type { IDisposable } from "monaco-editor";

export const setupAutoCompleteTriggers = (editor: editor.IStandaloneCodeEditor): IDisposable => {
  return editor.onDidChangeModelContent(() => {
    const model = editor.getModel();
    if (!model) return;

    const position = editor.getPosition();
    if (!position) return;

    const text = model.getValue();
    const cursorOffset = model.getOffsetAt(position);
    const textBeforeCursor = text.substring(0, cursorOffset);

    // 트리거 조건: 키워드, 연산자, 따옴표 입력 후
    if (shouldTriggerAutoComplete(textBeforeCursor)) {
      setTimeout(() => {
        editor.trigger("", "editor.action.triggerSuggest", {});
      }, 0);
    }
  });
};

const shouldTriggerAutoComplete = (text: string): boolean => {
  return (
    /(method|methods|status|url)$/.test(text) ||
    /(=|\|=|\|~|!=|!~)"$/.test(text) ||
    /"[^"]*,$/.test(text)
  );
};

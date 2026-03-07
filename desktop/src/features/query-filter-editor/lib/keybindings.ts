import type { editor } from "monaco-editor";
import type { Monaco } from "@monaco-editor/react";

interface KeybindingHandlers {
  onApply: (value: string) => void;
  onRevert: (value: string) => void;
}

export const setupKeybindings = (
  editor: editor.IStandaloneCodeEditor,
  monaco: Monaco,
  handlers: KeybindingHandlers,
  appliedValue: string,
): void => {
  // Cmd+Enter로 적용
  editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, () => {
    const currentValue = editor.getValue();
    handlers.onApply(currentValue);
  });

  // Escape로 되돌리기
  editor.addCommand(monaco.KeyCode.Escape, () => {
    editor.setValue(appliedValue);
    handlers.onRevert(appliedValue);
  });
};

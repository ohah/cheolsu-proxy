import { useRef, useEffect } from "react";
import type { editor } from "monaco-editor";
import type { Monaco } from "@monaco-editor/react";
import {
  setupMonacoLanguage,
  createCompletionProvider,
  setupKeybindings,
  setupAutoCompleteTriggers,
} from "../lib";

interface UseMonacoEditorProps {
  onApply: (value: string) => void;
  onChange: (value: string) => void;
  appliedValue: string;
}

export const useMonacoEditor = ({ onApply, onChange, appliedValue }: UseMonacoEditorProps) => {
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  const monacoRef = useRef<Monaco | null>(null);
  const disposablesRef = useRef<any[]>([]);

  const handleEditorDidMount = (editor: editor.IStandaloneCodeEditor, monaco: Monaco) => {
    editorRef.current = editor;
    monacoRef.current = monaco;

    // Monaco 언어/테마 설정
    setupMonacoLanguage(monaco);

    // 자동완성 프로바이더 등록
    const completionProvider = createCompletionProvider(monaco);
    disposablesRef.current.push(completionProvider);

    // 키바인딩 설정
    setupKeybindings(
      editor,
      monaco,
      {
        onApply,
        onRevert: onChange,
      },
      appliedValue,
    );

    // 자동완성 트리거 설정
    const triggerDisposable = setupAutoCompleteTriggers(editor);
    disposablesRef.current.push(triggerDisposable);
  };

  useEffect(() => {
    return () => {
      disposablesRef.current.forEach((disposable) => {
        if (disposable && disposable.dispose) {
          disposable.dispose();
        }
      });
      disposablesRef.current = [];
    };
  }, []);

  return {
    editorRef,
    monacoRef,
    handleEditorDidMount,
  };
};

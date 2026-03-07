import { useState, useCallback, useRef, useEffect } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { useTheme } from "next-themes";
import MonacoEditor, { type Monaco } from "@monaco-editor/react";
import type { editor } from "monaco-editor";
import { loadScript, unloadScript } from "@/shared/api/proxy";
import { useScriptStore } from "@/shared/stores/script-store";
import { Button, Input, Badge, Tabs, TabsList, TabsTrigger, TabsContent } from "@/shared/ui";
import { ResizablePanelGroup, ResizablePanel, ResizableHandle } from "@/shared/ui/resizable";
import { open } from "@tauri-apps/plugin-dialog";
import { readTextFile } from "@tauri-apps/plugin-fs";
import { toast } from "sonner";
import { CHEOLSU_TYPE_DEFS } from "../lib/cheolsu-types";

const DEFAULT_SCRIPT = `// Cheolsu Proxy Script
// cheolsu.onRequest / cheolsu.onResponse / cheolsu.onWebSocketMessage

cheolsu.onRequest((req) => {
  console.log(\`\${req.method} \${req.url}\`);
  return { action: "forward" };
});
`;

export function ScriptPage() {
  const { t } = useLingui();
  const { resolvedTheme } = useTheme();
  const active = useScriptStore((s) => s.active);
  const scriptPath = useScriptStore((s) => s.path);
  const logs = useScriptStore((s) => s.logs);
  const clearLogs = useScriptStore((s) => s.clearLogs);

  const [code, setCode] = useState(DEFAULT_SCRIPT);
  const [pathInput, setPathInput] = useState("");
  const [loading, setLoading] = useState(false);
  const [tab, setTab] = useState<string>("editor");
  const logEndRef = useRef<HTMLDivElement>(null);
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);

  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs.length]);

  const handleEditorMount = useCallback(
    (editorInstance: editor.IStandaloneCodeEditor, monaco: Monaco) => {
      editorRef.current = editorInstance;

      monaco.languages.typescript.typescriptDefaults.addExtraLib(CHEOLSU_TYPE_DEFS, "cheolsu.d.ts");
      monaco.languages.typescript.javascriptDefaults.addExtraLib(CHEOLSU_TYPE_DEFS, "cheolsu.d.ts");

      editorInstance.addAction({
        id: "run-script",
        label: "Run Script",
        keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter],
        run: () => handleRunCode(),
      });
    },
    [],
  );

  const handleRunCode = useCallback(async () => {
    const currentCode = editorRef.current?.getValue() || code;
    if (!currentCode.trim()) return;
    setLoading(true);
    try {
      await loadScript(undefined, currentCode);
      toast.success(t`Script loaded from editor`);
    } catch (e) {
      toast.error(String(e));
    } finally {
      setLoading(false);
    }
  }, [code, t]);

  const handleLoadFile = useCallback(async () => {
    if (!pathInput.trim()) return;
    setLoading(true);
    try {
      await loadScript(pathInput.trim());
      toast.success(t`Script loaded`);
    } catch (e) {
      toast.error(String(e));
    } finally {
      setLoading(false);
    }
  }, [pathInput, t]);

  const handleBrowse = useCallback(async () => {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Script", extensions: ["js", "ts", "mjs", "mts"] }],
    });
    if (selected) {
      const filePath = selected as string;
      setPathInput(filePath);
      try {
        const content = await readTextFile(filePath);
        setCode(content);
        setTab("editor");
      } catch {
        // 파일 읽기 실패 시 경로만 설정
      }
    }
  }, []);

  const handleUnload = useCallback(async () => {
    try {
      await unloadScript();
      setPathInput("");
      toast.success(t`Script unloaded`);
    } catch (e) {
      toast.error(String(e));
    }
  }, [t]);

  const handleReload = useCallback(async () => {
    if (!scriptPath) return;
    setLoading(true);
    try {
      await loadScript(scriptPath);
      toast.success(t`Script reloaded`);
    } catch (e) {
      toast.error(String(e));
    } finally {
      setLoading(false);
    }
  }, [scriptPath, t]);

  const levelColor = (level: string) => {
    switch (level) {
      case "error":
        return "text-red-500";
      case "warn":
        return "text-yellow-500";
      case "debug":
        return "text-muted-foreground";
      default:
        return "text-cyan-500";
    }
  };

  const levelTag = (level: string) => {
    switch (level) {
      case "error":
        return "[ERR]";
      case "warn":
        return "[WRN]";
      case "debug":
        return "[DBG]";
      default:
        return "[LOG]";
    }
  };

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden">
      {/* Header */}
      <div className="p-6 pb-2 flex-shrink-0">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold text-foreground">
              <Trans>Script</Trans>
            </h1>
            <p className="text-muted-foreground">
              <Trans>Write or load JavaScript/TypeScript proxy scripts</Trans>
            </p>
          </div>
          <div className="flex items-center gap-3">
            <Badge variant={active ? "default" : "secondary"}>
              {active ? t`Active` : t`Inactive`}
            </Badge>
            {active && (
              <>
                <Button variant="outline" size="sm" onClick={handleReload} disabled={loading}>
                  {t`Reload`}
                </Button>
                <Button variant="destructive" size="sm" onClick={handleUnload}>
                  {t`Unload`}
                </Button>
              </>
            )}
          </div>
        </div>
        {active && scriptPath && (
          <p className="text-xs text-muted-foreground font-mono mt-1">{scriptPath}</p>
        )}
      </div>

      {/* Resizable Editor + Console */}
      <div className="flex-1 min-h-0 px-6 pb-6">
        <ResizablePanelGroup direction="vertical">
          {/* Editor Panel */}
          <ResizablePanel defaultSize={70} minSize={30}>
            <Tabs value={tab} onValueChange={setTab} className="h-full flex flex-col">
              <div className="flex items-center justify-between mb-2 flex-shrink-0">
                <TabsList>
                  <TabsTrigger value="editor">
                    <Trans>Editor</Trans>
                  </TabsTrigger>
                  <TabsTrigger value="file">
                    <Trans>File</Trans>
                  </TabsTrigger>
                </TabsList>

                {tab === "editor" && (
                  <div className="flex gap-2">
                    <Button variant="outline" size="sm" onClick={handleBrowse}>
                      {t`Open File`}
                    </Button>
                    <Button size="sm" onClick={handleRunCode} disabled={loading}>
                      {loading ? t`Loading...` : t`Run`}
                      <span className="ml-1 text-xs text-muted-foreground">⌘↵</span>
                    </Button>
                  </div>
                )}
              </div>

              <TabsContent value="editor" className="flex-1 min-h-0 mt-0">
                <div className="border rounded-lg overflow-hidden h-full">
                  <MonacoEditor
                    height="100%"
                    defaultLanguage="typescript"
                    value={code}
                    onChange={(v) => setCode(v || "")}
                    onMount={handleEditorMount}
                    theme={resolvedTheme === "dark" ? "vs-dark" : "vs"}
                    options={{
                      minimap: { enabled: false },
                      fontSize: 13,
                      lineNumbers: "on",
                      scrollBeyondLastLine: false,
                      automaticLayout: true,
                      tabSize: 2,
                      wordWrap: "on",
                      padding: { top: 8 },
                    }}
                  />
                </div>
              </TabsContent>

              <TabsContent value="file" className="mt-0">
                <div className="border rounded-lg p-5 space-y-4">
                  <p className="text-sm text-muted-foreground">
                    <Trans>
                      Load a script file from disk. The file will be watched for changes and
                      auto-reloaded.
                    </Trans>
                  </p>
                  <div className="flex gap-2">
                    <Input
                      className="flex-1 font-mono text-sm"
                      placeholder={t`Script file path (.js, .ts)`}
                      value={pathInput}
                      onChange={(e) => setPathInput(e.target.value)}
                      onKeyDown={(e) => e.key === "Enter" && handleLoadFile()}
                    />
                    <Button variant="outline" onClick={handleBrowse}>
                      {t`Browse`}
                    </Button>
                    <Button onClick={handleLoadFile} disabled={loading || !pathInput.trim()}>
                      {loading ? t`Loading...` : t`Load`}
                    </Button>
                  </div>
                </div>
              </TabsContent>
            </Tabs>
          </ResizablePanel>

          {/* Drag Handle */}
          <ResizableHandle className="my-1 h-px data-[resize-handle-state=hover]:bg-primary data-[resize-handle-state=drag]:bg-primary" />

          {/* Console Panel */}
          <ResizablePanel defaultSize={30} minSize={10}>
            <div className="border rounded-lg flex flex-col h-full">
              <div className="flex items-center justify-between px-4 py-2 border-b bg-muted/30 flex-shrink-0">
                <h2 className="text-xs font-semibold">
                  <Trans>Console</Trans>
                  {logs.length > 0 && (
                    <span className="text-muted-foreground ml-1">({logs.length})</span>
                  )}
                </h2>
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-6 text-xs"
                  onClick={clearLogs}
                  disabled={logs.length === 0}
                >
                  {t`Clear`}
                </Button>
              </div>
              <div className="flex-1 overflow-auto px-4 py-2 font-mono text-xs space-y-0.5">
                {logs.length === 0 ? (
                  <p className="text-muted-foreground text-center py-4">
                    <Trans>No console output yet</Trans>
                  </p>
                ) : (
                  logs.map((entry, i) => (
                    <div key={i} className="flex gap-2">
                      <span className={`${levelColor(entry.level)} whitespace-nowrap`}>
                        {levelTag(entry.level)}
                      </span>
                      <span className="text-foreground break-all">{entry.message}</span>
                    </div>
                  ))
                )}
                <div ref={logEndRef} />
              </div>
            </div>
          </ResizablePanel>
        </ResizablePanelGroup>
      </div>
    </div>
  );
}

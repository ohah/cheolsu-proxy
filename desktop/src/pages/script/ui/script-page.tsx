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
        <ResizablePanelGroup orientation="vertical">
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
                  <TabsTrigger value="reference">
                    <Trans>API Reference</Trans>
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

              <TabsContent value="reference" className="flex-1 min-h-0 mt-0 overflow-auto">
                <ApiReference />
              </TabsContent>
            </Tabs>
          </ResizablePanel>

          {/* Drag Handle */}
          <ResizableHandle className="h-1.5 w-full rounded bg-transparent hover:bg-primary/20 data-[resize-handle-state=drag]:bg-primary/30" />

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

function ApiReference() {
  return (
    <div className="border rounded-lg p-5 space-y-6 text-sm overflow-auto h-full">
      <div>
        <h3 className="font-semibold text-base mb-2">cheolsu.onRequest(handler)</h3>
        <p className="text-muted-foreground mb-2">
          HTTP 요청이 프록시를 통과할 때 호출됩니다. 요청을 검사, 수정, 차단할 수 있습니다.
        </p>
        <div className="space-y-1 text-xs">
          <p>
            <strong>handler</strong>:{" "}
            <code className="bg-muted px-1 rounded">(request: Request) =&gt; RequestResult</code>
          </p>
          <p className="text-muted-foreground ml-4">
            <strong>request.method</strong> — HTTP 메서드 (GET, POST, PUT 등)
            <br />
            <strong>request.url</strong> — 전체 요청 URL
            <br />
            <strong>request.headers</strong> — 요청 헤더 (key-value 객체)
            <br />
            <strong>request.body</strong> — 요청 본문 (선택)
          </p>
          <p className="text-muted-foreground ml-4 mt-2">
            <strong>반환값:</strong>
            <br />
            <code className="bg-muted px-1 rounded">{'{ action: "forward" }'}</code> — 원본 요청
            그대로 전달
            <br />
            <code className="bg-muted px-1 rounded">{'{ action: "modify", request }'}</code> —
            수정된 요청 전달
            <br />
            <code className="bg-muted px-1 rounded">{'{ action: "respond", response }'}</code> —
            서버 대신 직접 응답
          </p>
        </div>
        <pre className="bg-muted rounded p-3 mt-2 text-xs overflow-x-auto">{`cheolsu.onRequest((req) => {
  if (req.url.includes("blocked.com")) {
    return { action: "respond", response: { status: 403, headers: {}, body: "Blocked" } };
  }
  return { action: "forward" };
});`}</pre>
      </div>

      <div>
        <h3 className="font-semibold text-base mb-2">cheolsu.onResponse(handler)</h3>
        <p className="text-muted-foreground mb-2">
          HTTP 응답이 프록시를 통과할 때 호출됩니다. 응답을 검사하거나 수정할 수 있습니다.
        </p>
        <div className="space-y-1 text-xs">
          <p>
            <strong>handler</strong>:{" "}
            <code className="bg-muted px-1 rounded">
              (request: Request, response: Response) =&gt; ResponseResult
            </code>
          </p>
          <p className="text-muted-foreground ml-4">
            <strong>response.status</strong> — HTTP 상태 코드
            <br />
            <strong>response.headers</strong> — 응답 헤더 (key-value 객체)
            <br />
            <strong>response.body</strong> — 응답 본문 (선택)
          </p>
          <p className="text-muted-foreground ml-4 mt-2">
            <strong>반환값:</strong>
            <br />
            <code className="bg-muted px-1 rounded">{'{ action: "forward" }'}</code> — 원본 응답
            그대로 전달
            <br />
            <code className="bg-muted px-1 rounded">{'{ action: "modify", response }'}</code> —
            수정된 응답 전달
          </p>
        </div>
        <pre className="bg-muted rounded p-3 mt-2 text-xs overflow-x-auto">{`cheolsu.onResponse((req, res) => {
  res.headers["X-Proxy"] = "cheolsu";
  return { action: "modify", response: res };
});`}</pre>
      </div>

      <div>
        <h3 className="font-semibold text-base mb-2">cheolsu.onWebSocketMessage(handler)</h3>
        <p className="text-muted-foreground mb-2">
          WebSocket 메시지가 프록시를 통과할 때 호출됩니다. 메시지를 검사, 수정, 차단할 수 있습니다.
        </p>
        <div className="space-y-1 text-xs">
          <p>
            <strong>handler</strong>:{" "}
            <code className="bg-muted px-1 rounded">(message: WsMessage) =&gt; WsResult</code>
          </p>
          <p className="text-muted-foreground ml-4">
            <strong>message.connection_id</strong> — WebSocket 연결 ID
            <br />
            <strong>message.url</strong> — WebSocket URL
            <br />
            <strong>message.direction</strong> — "to_server" | "to_client"
            <br />
            <strong>message.payload</strong> — 메시지 내용
            <br />
            <strong>message.is_binary</strong> — 바이너리 여부
          </p>
          <p className="text-muted-foreground ml-4 mt-2">
            <strong>반환값:</strong>
            <br />
            <code className="bg-muted px-1 rounded">{'{ action: "forward" }'}</code> — 원본 메시지
            전달
            <br />
            <code className="bg-muted px-1 rounded">
              {'{ action: "modify", payload, is_binary }'}
            </code>{" "}
            — 수정된 메시지 전달
            <br />
            <code className="bg-muted px-1 rounded">{'{ action: "drop" }'}</code> — 메시지 차단
          </p>
        </div>
        <pre className="bg-muted rounded p-3 mt-2 text-xs overflow-x-auto">{`cheolsu.onWebSocketMessage((msg) => {
  console.log(\`[WS \${msg.direction}] \${msg.payload}\`);
  return { action: "forward" };
});`}</pre>
      </div>

      <div>
        <h3 className="font-semibold text-base mb-2">console.log / warn / error / debug</h3>
        <p className="text-muted-foreground">
          표준 콘솔 메서드를 사용할 수 있습니다. 출력은 하단 콘솔 패널에 표시됩니다.
        </p>
      </div>
    </div>
  );
}

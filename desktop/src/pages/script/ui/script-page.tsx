import { useState, useCallback, useRef, useEffect } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { loadScript, unloadScript } from "@/shared/api/proxy";
import { useScriptStore } from "@/shared/stores/script-store";
import { Button, Input, Badge } from "@/shared/ui";
import { open } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";

export function ScriptPage() {
  const { t } = useLingui();
  const active = useScriptStore((s) => s.active);
  const scriptPath = useScriptStore((s) => s.path);
  const logs = useScriptStore((s) => s.logs);
  const clearLogs = useScriptStore((s) => s.clearLogs);

  const [pathInput, setPathInput] = useState("");
  const [loading, setLoading] = useState(false);
  const logEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs.length]);

  const handleLoad = useCallback(async () => {
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
      setPathInput(selected as string);
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
    <div className="flex-1 flex flex-col h-full overflow-auto">
      <div className="p-6 space-y-6 flex flex-col h-full">
        <div>
          <h1 className="text-2xl font-bold text-foreground">
            <Trans>Script</Trans>
          </h1>
          <p className="text-muted-foreground">
            <Trans>Load and manage JavaScript/TypeScript proxy scripts</Trans>
          </p>
        </div>

        {/* Status & Controls */}
        <div className="border rounded-lg p-5 space-y-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <h2 className="text-lg font-semibold">
                <Trans>Script Status</Trans>
              </h2>
              <Badge variant={active ? "default" : "secondary"}>
                {active ? t`Active` : t`Inactive`}
              </Badge>
            </div>
            {active && (
              <div className="flex gap-2">
                <Button variant="outline" size="sm" onClick={handleReload} disabled={loading}>
                  {t`Reload`}
                </Button>
                <Button variant="destructive" size="sm" onClick={handleUnload}>
                  {t`Unload`}
                </Button>
              </div>
            )}
          </div>

          {scriptPath && <p className="text-sm text-muted-foreground font-mono">{scriptPath}</p>}

          {!active && (
            <div className="flex gap-2">
              <Input
                className="flex-1 font-mono text-sm"
                placeholder={t`Script file path (.js, .ts)`}
                value={pathInput}
                onChange={(e) => setPathInput(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleLoad()}
              />
              <Button variant="outline" onClick={handleBrowse}>
                {t`Browse`}
              </Button>
              <Button onClick={handleLoad} disabled={loading || !pathInput.trim()}>
                {loading ? t`Loading...` : t`Load`}
              </Button>
            </div>
          )}
        </div>

        {/* Console Logs */}
        <div className="border rounded-lg flex-1 flex flex-col min-h-0">
          <div className="flex items-center justify-between p-3 border-b">
            <h2 className="text-sm font-semibold">
              <Trans>Console</Trans>
              {logs.length > 0 && (
                <span className="text-muted-foreground ml-2">({logs.length})</span>
              )}
            </h2>
            <Button variant="ghost" size="sm" onClick={clearLogs} disabled={logs.length === 0}>
              {t`Clear`}
            </Button>
          </div>
          <div className="flex-1 overflow-auto p-3 font-mono text-xs space-y-0.5">
            {logs.length === 0 ? (
              <p className="text-muted-foreground text-center py-8">
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
      </div>
    </div>
  );
}

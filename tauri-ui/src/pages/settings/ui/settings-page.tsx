import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button, Input, Switch, Badge } from "@/shared/ui";

interface UpstreamProxyConfig {
  host: string;
  port: number;
  auth: { username: string; password: string } | null;
  bypass: string[];
}

export function SettingsPage() {
  const [enabled, setEnabled] = useState(false);
  const [host, setHost] = useState("");
  const [port, setPort] = useState("8080");
  const [useAuth, setUseAuth] = useState(false);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [bypass, setBypass] = useState("localhost, 127.0.0.1");
  const [saving, setSaving] = useState(false);
  const [status, setStatus] = useState<"idle" | "saved" | "error">("idle");

  // 로컬 스토리지에서 설정 불러오기
  useEffect(() => {
    const saved = localStorage.getItem("upstream_proxy_config");
    if (saved) {
      try {
        const parsed = JSON.parse(saved);
        setEnabled(parsed.enabled ?? false);
        setHost(parsed.host ?? "");
        setPort(String(parsed.port ?? "8080"));
        setUseAuth(!!parsed.auth);
        setUsername(parsed.auth?.username ?? "");
        setPassword(parsed.auth?.password ?? "");
        setBypass((parsed.bypass ?? []).join(", "));
      } catch {
        // 파싱 실패 시 무시
      }
    }
  }, []);

  const handleSave = useCallback(async () => {
    setSaving(true);
    setStatus("idle");

    try {
      const config: UpstreamProxyConfig | null = enabled
        ? {
            host,
            port: Number.parseInt(port, 10) || 8080,
            auth: useAuth ? { username, password } : null,
            bypass: bypass
              .split(",")
              .map((s) => s.trim())
              .filter(Boolean),
          }
        : null;

      await invoke("update_upstream_proxy", { config });

      // 로컬 스토리지에 저장
      localStorage.setItem("upstream_proxy_config", JSON.stringify({ enabled, ...config }));

      setStatus("saved");
      setTimeout(() => setStatus("idle"), 2000);
    } catch (e) {
      console.error("Upstream proxy 설정 저장 실패:", e);
      setStatus("error");
    } finally {
      setSaving(false);
    }
  }, [enabled, host, port, useAuth, username, password, bypass]);

  return (
    <div className="flex-1 flex flex-col h-full overflow-auto">
      <div className="p-6 space-y-6">
        <div>
          <h1 className="text-2xl font-bold text-foreground">Settings</h1>
          <p className="text-muted-foreground">Proxy configuration and preferences</p>
        </div>

        {/* Upstream Proxy Section */}
        <div className="border rounded-lg p-5 space-y-5">
          <div className="flex items-center justify-between">
            <div>
              <h2 className="text-lg font-semibold">Upstream Proxy</h2>
              <p className="text-sm text-muted-foreground">
                Route traffic through an external proxy server
              </p>
            </div>
            <Switch checked={enabled} onCheckedChange={setEnabled} />
          </div>

          {enabled && (
            <div className="space-y-4 pt-2">
              {/* Host & Port */}
              <div className="flex gap-3">
                <div className="flex-1">
                  <label className="text-sm font-medium mb-1.5 block">Host</label>
                  <Input
                    placeholder="proxy.company.com"
                    value={host}
                    onChange={(e) => setHost(e.target.value)}
                  />
                </div>
                <div className="w-28">
                  <label className="text-sm font-medium mb-1.5 block">Port</label>
                  <Input
                    type="number"
                    placeholder="8080"
                    value={port}
                    onChange={(e) => setPort(e.target.value)}
                  />
                </div>
              </div>

              {/* Authentication */}
              <div className="space-y-3">
                <div className="flex items-center gap-2">
                  <Switch checked={useAuth} onCheckedChange={setUseAuth} />
                  <label className="text-sm font-medium">Authentication</label>
                </div>
                {useAuth && (
                  <div className="flex gap-3 pl-1">
                    <div className="flex-1">
                      <Input
                        placeholder="Username"
                        value={username}
                        onChange={(e) => setUsername(e.target.value)}
                      />
                    </div>
                    <div className="flex-1">
                      <Input
                        type="password"
                        placeholder="Password"
                        value={password}
                        onChange={(e) => setPassword(e.target.value)}
                      />
                    </div>
                  </div>
                )}
              </div>

              {/* Bypass */}
              <div>
                <label className="text-sm font-medium mb-1.5 block">Bypass List</label>
                <Input
                  placeholder="localhost, 127.0.0.1, *.internal.com"
                  value={bypass}
                  onChange={(e) => setBypass(e.target.value)}
                />
                <p className="text-xs text-muted-foreground mt-1">
                  Comma-separated list of hosts to connect directly (supports *.domain.com
                  wildcards)
                </p>
              </div>
            </div>
          )}

          {/* Save Button */}
          <div className="flex items-center gap-3 pt-2">
            <Button onClick={handleSave} disabled={saving}>
              {saving ? "Saving..." : "Save"}
            </Button>
            {status === "saved" && (
              <Badge variant="outline" className="text-green-600 border-green-600">
                Saved
              </Badge>
            )}
            {status === "error" && (
              <Badge variant="outline" className="text-red-600 border-red-600">
                Failed — is the proxy running?
              </Badge>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

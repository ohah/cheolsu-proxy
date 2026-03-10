import { useState, useEffect, useCallback } from "react";
import { useAppSettingsStore } from "@/shared/stores/app-settings-store";
import { invoke } from "@tauri-apps/api/core";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { Button, Input, Switch, Badge } from "@/shared/ui";

interface UpstreamProxyConfig {
  host: string;
  port: number;
  auth: { username: string; password: string } | null;
  bypass: string[];
}

export function ProxySettings() {
  const { t } = useLingui();
  const [enabled, setEnabled] = useState(false);
  const [host, setHost] = useState("");
  const [port, setPort] = useState("8080");
  const [useAuth, setUseAuth] = useState(false);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [bypass, setBypass] = useState("localhost, 127.0.0.1");
  const [saving, setSaving] = useState(false);
  const [status, setStatus] = useState<"idle" | "saved" | "error">("idle");

  // store에서 설정 불러오기
  useEffect(() => {
    const config = useAppSettingsStore.getState().upstreamProxyConfig;
    setEnabled(config.enabled);
    setHost(config.host);
    setPort(String(config.port));
    setUseAuth(!!config.auth);
    setUsername(config.auth?.username ?? "");
    setPassword(config.auth?.password ?? "");
    setBypass(config.bypass.join(", "));
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

      // store에 저장 (persist가 자동으로 localStorage에 반영)
      useAppSettingsStore.getState().setUpstreamProxyConfig({
        enabled,
        host,
        port: Number.parseInt(port, 10) || 8080,
        auth: useAuth ? { username, password } : null,
        bypass: bypass.split(",").map((s) => s.trim()).filter(Boolean),
      });

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
    <div className="border rounded-lg p-5 space-y-5">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold">
            <Trans>Upstream Proxy</Trans>
          </h2>
          <p className="text-sm text-muted-foreground">
            <Trans>Route traffic through an external proxy server</Trans>
          </p>
        </div>
        <Switch checked={enabled} onCheckedChange={setEnabled} />
      </div>

      {enabled && (
        <div className="space-y-4 pt-2">
          {/* Host & Port */}
          <div className="flex gap-3">
            <div className="flex-1">
              <label className="text-sm font-medium mb-1.5 block">
                <Trans>Host</Trans>
              </label>
              <Input
                placeholder={t`proxy.company.com`}
                value={host}
                onChange={(e) => setHost(e.target.value)}
              />
            </div>
            <div className="w-28">
              <label className="text-sm font-medium mb-1.5 block">
                <Trans>Port</Trans>
              </label>
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
              <label className="text-sm font-medium">
                <Trans>Authentication</Trans>
              </label>
            </div>
            {useAuth && (
              <div className="flex gap-3 pl-1">
                <div className="flex-1">
                  <Input
                    placeholder={t`Username`}
                    value={username}
                    onChange={(e) => setUsername(e.target.value)}
                  />
                </div>
                <div className="flex-1">
                  <Input
                    type="password"
                    placeholder={t`Password`}
                    value={password}
                    onChange={(e) => setPassword(e.target.value)}
                  />
                </div>
              </div>
            )}
          </div>

          {/* Bypass */}
          <div>
            <label className="text-sm font-medium mb-1.5 block">
              <Trans>Bypass List</Trans>
            </label>
            <Input
              placeholder={t`localhost, 127.0.0.1, *.internal.com`}
              value={bypass}
              onChange={(e) => setBypass(e.target.value)}
            />
            <p className="text-xs text-muted-foreground mt-1">
              <Trans>
                Comma-separated list of hosts to connect directly (supports *.domain.com wildcards)
              </Trans>
            </p>
          </div>
        </div>
      )}

      {/* Save Button */}
      <div className="flex items-center gap-3 pt-2">
        <Button onClick={handleSave} disabled={saving}>
          {saving ? t`Saving...` : t`Save`}
        </Button>
        {status === "saved" && (
          <Badge variant="outline" className="text-green-600 border-green-600">
            <Trans>Saved</Trans>
          </Badge>
        )}
        {status === "error" && (
          <Badge variant="outline" className="text-red-600 border-red-600">
            <Trans>Failed — is the proxy running?</Trans>
          </Badge>
        )}
      </div>
    </div>
  );
}

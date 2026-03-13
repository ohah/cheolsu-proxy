import { useState, useCallback } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { type SslProxyingEntry, DEFAULT_PASSTHROUGH_DOMAINS } from "@/shared/api/proxy";
import { Button, Input, Switch } from "@/shared/ui";
import { Select, SelectTrigger, SelectContent, SelectItem, SelectValue } from "@/shared/ui";
import { useSettingsForm } from "../settings-form";

export function SslProxyingSection() {
  const { t } = useLingui();
  const form = useSettingsForm();
  const mode = form.watch("sslProxying.mode");
  const entries: SslProxyingEntry[] = form.watch("sslProxying.entries");
  const defaultPassthroughEntries: SslProxyingEntry[] = form.watch(
    "sslProxying.defaultPassthroughEntries",
  );
  const [newPattern, setNewPattern] = useState("");
  const [showDefaultDomains, setShowDefaultDomains] = useState(false);
  const [newDefaultPattern, setNewDefaultPattern] = useState("");

  const handleAdd = useCallback(() => {
    const pattern = newPattern.trim();
    if (!pattern || entries.some((e) => e.pattern === pattern)) return;
    form.setValue("sslProxying.entries", [...entries, { pattern, enabled: true }], {
      shouldDirty: true,
    });
    setNewPattern("");
  }, [newPattern, entries, form]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        handleAdd();
      }
    },
    [handleAdd],
  );

  const handleRemove = useCallback(
    (pattern: string) => {
      form.setValue(
        "sslProxying.entries",
        entries.filter((e) => e.pattern !== pattern),
        { shouldDirty: true },
      );
    },
    [entries, form],
  );

  const handleToggle = useCallback(
    (pattern: string) => {
      form.setValue(
        "sslProxying.entries",
        entries.map((e) => (e.pattern === pattern ? { ...e, enabled: !e.enabled } : e)),
        { shouldDirty: true },
      );
    },
    [entries, form],
  );

  const handleModeChange = useCallback(
    (v: string) => {
      form.setValue("sslProxying.mode", v as "blacklist" | "whitelist", { shouldDirty: true });
    },
    [form],
  );

  // --- 기본 패스스루 도메인 핸들러 ---

  const handleDefaultToggle = useCallback(
    (pattern: string) => {
      form.setValue(
        "sslProxying.defaultPassthroughEntries",
        defaultPassthroughEntries.map((e) =>
          e.pattern === pattern ? { ...e, enabled: !e.enabled } : e,
        ),
        { shouldDirty: true },
      );
    },
    [defaultPassthroughEntries, form],
  );

  const handleDefaultRemove = useCallback(
    (pattern: string) => {
      form.setValue(
        "sslProxying.defaultPassthroughEntries",
        defaultPassthroughEntries.filter((e) => e.pattern !== pattern),
        { shouldDirty: true },
      );
    },
    [defaultPassthroughEntries, form],
  );

  const handleDefaultAdd = useCallback(() => {
    const pattern = newDefaultPattern.trim();
    if (!pattern || defaultPassthroughEntries.some((e) => e.pattern === pattern)) return;
    form.setValue(
      "sslProxying.defaultPassthroughEntries",
      [...defaultPassthroughEntries, { pattern, enabled: true }],
      { shouldDirty: true },
    );
    setNewDefaultPattern("");
  }, [newDefaultPattern, defaultPassthroughEntries, form]);

  const handleDefaultKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        handleDefaultAdd();
      }
    },
    [handleDefaultAdd],
  );

  const handleRestoreDefaults = useCallback(() => {
    form.setValue(
      "sslProxying.defaultPassthroughEntries",
      DEFAULT_PASSTHROUGH_DOMAINS.map((e) => ({ ...e })),
      { shouldDirty: true },
    );
  }, [form]);

  const enabledCount = entries.filter((e) => e.enabled).length;
  const defaultEnabledCount = defaultPassthroughEntries.filter((e) => e.enabled).length;

  return (
    <div className="border rounded-lg p-5 space-y-4">
      <div>
        <h2 className="text-lg font-semibold">
          <Trans>SSL Proxying</Trans>
        </h2>
        <p className="text-sm text-muted-foreground">
          {mode === "blacklist" ? (
            enabledCount === 0 ? (
              <Trans>
                All HTTPS traffic is intercepted. {defaultEnabledCount} built-in domain(s) are
                automatically excluded.
              </Trans>
            ) : (
              <Trans>
                All HTTPS traffic is intercepted except {enabledCount} excluded domain(s) and{" "}
                {defaultEnabledCount} built-in domain(s).
              </Trans>
            )
          ) : enabledCount === 0 ? (
            <Trans>All HTTPS traffic is being intercepted (no whitelist configured)</Trans>
          ) : (
            <Trans>
              Only whitelisted domains ({enabledCount}) will have HTTPS traffic intercepted
            </Trans>
          )}
        </p>
      </div>
      <div className="flex items-center gap-3">
        <span className="text-sm font-medium whitespace-nowrap">
          <Trans>Mode</Trans>
        </span>
        <Select
          value={mode}
          onValueChange={(v) => {
            if (v) handleModeChange(v);
          }}
        >
          <SelectTrigger className="w-48">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="blacklist">
              <Trans>Blacklist (Exclude)</Trans>
            </SelectItem>
            <SelectItem value="whitelist">
              <Trans>Whitelist (Include)</Trans>
            </SelectItem>
          </SelectContent>
        </Select>
      </div>
      <div className="flex items-center gap-2">
        <Input
          placeholder={t`example.com, *.example.com, or example.com:443`}
          value={newPattern}
          onChange={(e) => setNewPattern(e.target.value)}
          onKeyDown={handleKeyDown}
          className="flex-1"
        />
        <Button type="button" onClick={handleAdd} disabled={!newPattern.trim()}>
          <Trans>Add</Trans>
        </Button>
      </div>
      <p className="text-xs text-muted-foreground">
        {mode === "blacklist" ? (
          <Trans>
            Domains in this list will NOT be intercepted (pass-through). Built-in domains can be
            managed below.
          </Trans>
        ) : (
          <Trans>
            Only domains in this list will be intercepted. When the list is empty, all domains are
            intercepted.
          </Trans>
        )}
      </p>
      {entries.length > 0 && (
        <div className="border rounded-lg divide-y">
          {entries.map((entry) => (
            <div key={entry.pattern} className="flex items-center justify-between px-4 py-2">
              <div className="flex items-center gap-3">
                <Switch
                  checked={entry.enabled}
                  onCheckedChange={() => handleToggle(entry.pattern)}
                />
                <span
                  className={`font-mono text-sm ${entry.enabled ? "text-foreground" : "text-muted-foreground line-through"}`}
                >
                  {entry.pattern}
                </span>
              </div>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => handleRemove(entry.pattern)}
                className="text-muted-foreground hover:text-destructive"
              >
                <Trans>Remove</Trans>
              </Button>
            </div>
          ))}
        </div>
      )}

      {/* 기본 패스스루 도메인 섹션 */}
      {mode === "blacklist" && (
        <div className="border rounded-lg p-4 space-y-3 bg-muted/30">
          <div className="flex items-center justify-between">
            <button
              type="button"
              onClick={() => setShowDefaultDomains(!showDefaultDomains)}
              className="flex items-center gap-2 text-sm font-medium hover:text-foreground transition-colors"
            >
              <span className={`transition-transform ${showDefaultDomains ? "rotate-90" : ""}`}>
                ▶
              </span>
              <Trans>
                Built-in Passthrough Domains ({defaultPassthroughEntries.length})
              </Trans>
            </button>
            <Button type="button" variant="outline" size="sm" onClick={handleRestoreDefaults}>
              <Trans>Restore Defaults</Trans>
            </Button>
          </div>
          <p className="text-xs text-muted-foreground">
            <Trans>
              These domains are automatically excluded from HTTPS interception to prevent OAuth/authentication failures. You can toggle, add, or remove domains.
            </Trans>
          </p>
          {showDefaultDomains && (
            <>
              <div className="flex items-center gap-2">
                <Input
                  placeholder={t`Add built-in domain pattern...`}
                  value={newDefaultPattern}
                  onChange={(e) => setNewDefaultPattern(e.target.value)}
                  onKeyDown={handleDefaultKeyDown}
                  className="flex-1"
                  size={1}
                />
                <Button
                  type="button"
                  variant="secondary"
                  onClick={handleDefaultAdd}
                  disabled={!newDefaultPattern.trim()}
                >
                  <Trans>Add</Trans>
                </Button>
              </div>
              {defaultPassthroughEntries.length > 0 && (
                <div className="border rounded-lg divide-y bg-background">
                  {defaultPassthroughEntries.map((entry) => (
                    <div
                      key={entry.pattern}
                      className="flex items-center justify-between px-4 py-2"
                    >
                      <div className="flex items-center gap-3">
                        <Switch
                          checked={entry.enabled}
                          onCheckedChange={() => handleDefaultToggle(entry.pattern)}
                        />
                        <span
                          className={`font-mono text-sm ${entry.enabled ? "text-foreground" : "text-muted-foreground line-through"}`}
                        >
                          {entry.pattern}
                        </span>
                      </div>
                      <Button
                        type="button"
                        variant="ghost"
                        size="sm"
                        onClick={() => handleDefaultRemove(entry.pattern)}
                        className="text-muted-foreground hover:text-destructive"
                      >
                        <Trans>Remove</Trans>
                      </Button>
                    </div>
                  ))}
                </div>
              )}
            </>
          )}
        </div>
      )}
    </div>
  );
}

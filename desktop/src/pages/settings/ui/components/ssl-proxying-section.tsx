import { useState, useCallback } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { type SslProxyingEntry, getDefaultPassthroughDomains } from "@/shared/api/proxy";
import { Button, Input, Switch } from "@/shared/ui";
import { Select, SelectTrigger, SelectContent, SelectItem, SelectValue } from "@/shared/ui";
import { useSettingsForm } from "../settings-form";
import { SettingsSection } from "./settings-section";

type FormFieldPath = "sslProxying.entries" | "sslProxying.defaultPassthroughEntries";

function useEntryListHandlers(fieldPath: FormFieldPath, entries: SslProxyingEntry[]) {
  const form = useSettingsForm();

  const handleToggle = useCallback(
    (pattern: string) => {
      form.setValue(
        fieldPath,
        entries.map((e) => (e.pattern === pattern ? { ...e, enabled: !e.enabled } : e)),
        { shouldDirty: true },
      );
    },
    [entries, form, fieldPath],
  );

  const handleRemove = useCallback(
    (pattern: string) => {
      form.setValue(
        fieldPath,
        entries.filter((e) => e.pattern !== pattern),
        { shouldDirty: true },
      );
    },
    [entries, form, fieldPath],
  );

  const handleAdd = useCallback(
    (pattern: string) => {
      if (!pattern || entries.some((e) => e.pattern === pattern)) return;
      form.setValue(fieldPath, [...entries, { pattern, enabled: true }], {
        shouldDirty: true,
      });
    },
    [entries, form, fieldPath],
  );

  return { handleToggle, handleRemove, handleAdd };
}

function EntryList({
  entries,
  onToggle,
  onRemove,
  className,
}: {
  entries: SslProxyingEntry[];
  onToggle: (pattern: string) => void;
  onRemove: (pattern: string) => void;
  className?: string;
}) {
  if (entries.length === 0) return null;
  return (
    <div className={`border rounded-lg divide-y ${className ?? ""}`}>
      {entries.map((entry) => (
        <div key={entry.pattern} className="flex items-center justify-between px-4 py-2">
          <div className="flex items-center gap-3">
            <Switch checked={entry.enabled} onCheckedChange={() => onToggle(entry.pattern)} />
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
            onClick={() => onRemove(entry.pattern)}
            className="text-muted-foreground hover:text-destructive"
          >
            <Trans>Remove</Trans>
          </Button>
        </div>
      ))}
    </div>
  );
}

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

  const entryHandlers = useEntryListHandlers("sslProxying.entries", entries);
  const defaultHandlers = useEntryListHandlers(
    "sslProxying.defaultPassthroughEntries",
    defaultPassthroughEntries,
  );

  const handleAddEntry = useCallback(() => {
    entryHandlers.handleAdd(newPattern.trim());
    setNewPattern("");
  }, [newPattern, entryHandlers]);

  const handleAddDefault = useCallback(() => {
    defaultHandlers.handleAdd(newDefaultPattern.trim());
    setNewDefaultPattern("");
  }, [newDefaultPattern, defaultHandlers]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        handleAddEntry();
      }
    },
    [handleAddEntry],
  );

  const handleDefaultKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        handleAddDefault();
      }
    },
    [handleAddDefault],
  );

  const handleModeChange = useCallback(
    (v: string) => {
      form.setValue("sslProxying.mode", v as "blacklist" | "whitelist", {
        shouldDirty: true,
      });
    },
    [form],
  );

  const handleRestoreDefaults = useCallback(async () => {
    const defaults = await getDefaultPassthroughDomains();
    form.setValue("sslProxying.defaultPassthroughEntries", defaults, {
      shouldDirty: true,
    });
  }, [form]);

  const enabledCount = entries.filter((e) => e.enabled).length;
  const defaultEnabledCount = defaultPassthroughEntries.filter((e) => e.enabled).length;

  return (
    <SettingsSection
      title={<Trans>SSL Proxying</Trans>}
      description={
        mode === "blacklist" ? (
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
        )
      }
    >
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
        <Button type="button" onClick={handleAddEntry} disabled={!newPattern.trim()}>
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
      {mode === "whitelist" && enabledCount === 0 && (
        <div className="rounded-md border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-xs text-amber-700 dark:text-amber-300">
          <Trans>
            Empty whitelist currently means all HTTPS traffic is intercepted. Add at least one
            enabled domain to limit SSL Proxying to selected hosts.
          </Trans>
        </div>
      )}
      <EntryList
        entries={entries}
        onToggle={entryHandlers.handleToggle}
        onRemove={entryHandlers.handleRemove}
      />

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
              <Trans>Built-in Passthrough Domains ({defaultPassthroughEntries.length})</Trans>
            </button>
            <Button type="button" variant="outline" size="sm" onClick={handleRestoreDefaults}>
              <Trans>Restore Defaults</Trans>
            </Button>
          </div>
          <p className="text-xs text-muted-foreground">
            <Trans>
              These domains are automatically excluded from HTTPS interception to prevent
              OAuth/authentication failures. You can toggle, add, or remove domains.
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
                  onClick={handleAddDefault}
                  disabled={!newDefaultPattern.trim()}
                >
                  <Trans>Add</Trans>
                </Button>
              </div>
              <EntryList
                entries={defaultPassthroughEntries}
                onToggle={defaultHandlers.handleToggle}
                onRemove={defaultHandlers.handleRemove}
                className="bg-background"
              />
            </>
          )}
        </div>
      )}
    </SettingsSection>
  );
}

import { useState, useCallback, useEffect, useRef } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { getNeverPassthroughDomains } from "@/shared/api/proxy";
import { Button, Input } from "@/shared/ui";
import { X } from "lucide-react";
import { useSettingsForm } from "../settings-form";
import { SettingsSection } from "./settings-section";

export function NeverPassthroughSection() {
  const { t } = useLingui();
  const form = useSettingsForm();
  const entries: string[] = form.watch("neverPassthrough.entries");
  const [newPattern, setNewPattern] = useState("");

  const loaded = useRef(false);
  useEffect(() => {
    if (loaded.current) return;
    loaded.current = true;
    getNeverPassthroughDomains().then((domains) => {
      form.setValue("neverPassthrough.entries", domains, { shouldDirty: false });
    });
  }, [form]);

  const handleAdd = useCallback(() => {
    const pattern = newPattern.trim();
    if (!pattern || entries.includes(pattern)) return;
    form.setValue("neverPassthrough.entries", [...entries, pattern], {
      shouldDirty: true,
    });
    setNewPattern("");
  }, [newPattern, entries, form]);

  const handleRemove = useCallback(
    (pattern: string) => {
      form.setValue(
        "neverPassthrough.entries",
        entries.filter((e) => e !== pattern),
        { shouldDirty: true },
      );
    },
    [entries, form],
  );

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        handleAdd();
      }
    },
    [handleAdd],
  );

  return (
    <SettingsSection
      title={<Trans>Never Passthrough</Trans>}
      description={
        <Trans>
          Domains in this list will never fall back to passthrough when TLS handshake fails. The
          proxy will keep trying to intercept these domains, allowing you to see the actual error
          instead of silently bypassing.
        </Trans>
      }
    >
      <div className="flex items-center gap-2">
        <Input
          placeholder={t`example.com or *.example.com`}
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
        <Trans>
          Supports wildcard patterns: * matches any string, ? matches a single character
        </Trans>
      </p>
      {entries.length > 0 && (
        <div className="border rounded-lg divide-y">
          {entries.map((pattern) => (
            <div key={pattern} className="flex items-center justify-between px-4 py-2">
              <span className="font-mono text-sm text-foreground">{pattern}</span>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => handleRemove(pattern)}
                className="text-muted-foreground hover:text-destructive h-7 w-7 p-0"
              >
                <X className="w-4 h-4" />
              </Button>
            </div>
          ))}
        </div>
      )}
    </SettingsSection>
  );
}

import { useState, useCallback } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { Button, Switch } from "@/shared/ui";
import { platform } from "@tauri-apps/plugin-os";
import { useSettingsForm } from "../settings-form";

const isMac = platform() === "macos";

function ShortcutDisplay({ shortcut }: { shortcut: string }) {
  const parts = shortcut.split("+").map((part) => {
    switch (part) {
      case "CommandOrControl":
        return isMac ? "\u2318" : "Ctrl";
      case "Shift":
        return "\u21E7";
      case "Alt":
        return isMac ? "\u2325" : "Alt";
      case "Space":
        return "Space";
      default:
        return part;
    }
  });
  return (
    <div className="flex items-center gap-1">
      {parts.map((part, i) => (
        <span key={i}>
          {i > 0 && <span className="text-muted-foreground mx-0.5">+</span>}
          <kbd className="px-1.5 py-0.5 bg-muted border rounded text-xs font-mono">{part}</kbd>
        </span>
      ))}
    </div>
  );
}

export function ShortcutSection() {
  const { t } = useLingui();
  const { watch, setValue } = useSettingsForm();
  const enabled = watch("shortcut.enabled");
  const hotkey = watch("shortcut.key");
  const [isRecording, setIsRecording] = useState(false);

  const handleHotkeyRecord = useCallback(
    (e: React.KeyboardEvent) => {
      if (!isRecording) return;
      e.preventDefault();
      e.stopPropagation();
      const parts: string[] = [];
      const hasCtrlOrCmd = e.metaKey || e.ctrlKey;
      const hasAlt = e.altKey;
      if (hasCtrlOrCmd) parts.push("CommandOrControl");
      if (hasAlt) parts.push("Alt");
      if (e.shiftKey) parts.push("Shift");
      const key = e.key;
      if (["Control", "Meta", "Alt", "Shift"].includes(key)) return;
      if (!hasCtrlOrCmd && !hasAlt) return;
      let keyName = key;
      if (key.length === 1) keyName = key.toUpperCase();
      else if (key === " ") keyName = "Space";
      parts.push(keyName);
      setValue("shortcut.key", parts.join("+"), { shouldDirty: true });
      setIsRecording(false);
    },
    [isRecording, setValue],
  );

  return (
    <div className="border rounded-lg p-5 space-y-5">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold">
            <Trans>Global Shortcut</Trans>
          </h2>
          <p className="text-sm text-muted-foreground">
            <Trans>Toggle proxy on/off with a global keyboard shortcut</Trans>
          </p>
        </div>
        <Switch
          checked={enabled}
          onCheckedChange={(v) => setValue("shortcut.enabled", v, { shouldDirty: true })}
        />
      </div>
      {enabled && (
        <div className="space-y-3 pt-2">
          <div>
            <label className="text-sm font-medium mb-1.5 block">
              <Trans>Shortcut Key</Trans>
            </label>
            <div className="flex gap-3 items-center">
              <div
                tabIndex={0}
                role="button"
                className={`flex-1 h-9 px-3 border rounded-md flex items-center text-sm cursor-pointer focus:outline-none ${
                  isRecording
                    ? "border-primary ring-2 ring-primary/30 text-muted-foreground"
                    : "bg-background"
                }`}
                onKeyDown={handleHotkeyRecord}
                onClick={() => setIsRecording(true)}
                onBlur={() => setIsRecording(false)}
              >
                {isRecording ? (
                  <span className="text-muted-foreground">
                    <Trans>Press a key combination...</Trans>
                  </span>
                ) : (
                  <ShortcutDisplay shortcut={hotkey} />
                )}
              </div>
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={() => setIsRecording(!isRecording)}
              >
                {isRecording ? t`Cancel` : t`Change`}
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

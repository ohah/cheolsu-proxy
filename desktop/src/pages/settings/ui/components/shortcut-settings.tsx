import { useState, useCallback } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { useSettingsStore } from "@/shared/stores/use-settings-store";
import {
  setStoredShortcut,
  setShortcutEnabled,
  registerShortcut,
  unregisterShortcut,
} from "@/shared/lib/global-shortcut";
import { toggleProxy } from "@/features/proxy-toggle";
import { platform } from "@tauri-apps/plugin-os";
import { Button, Switch, Badge } from "@/shared/ui";

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

export function ShortcutSettings() {
  const { t } = useLingui();
  const hotkeyEnabled = useSettingsStore((s) => s.hotkeyEnabled);
  const storeSetHotkeyEnabled = useSettingsStore((s) => s.setHotkeyEnabled);
  const hotkey = useSettingsStore((s) => s.hotkey);
  const storeSetHotkey = useSettingsStore((s) => s.setHotkey);

  const [localHotkeyEnabled, setLocalHotkeyEnabled] = useState(hotkeyEnabled);
  const [localHotkey, setLocalHotkey] = useState(hotkey);
  const [isRecording, setIsRecording] = useState(false);
  const [hotkeyStatus, setHotkeyStatus] = useState<"idle" | "saved" | "error">("idle");

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
      // modifier 키만 누른 경우 무시
      if (["Control", "Meta", "Alt", "Shift"].includes(key)) return;

      // CommandOrControl 또는 Alt 필수 (Shift 단독은 일반 타이핑과 충돌)
      if (!hasCtrlOrCmd && !hasAlt) return;

      // 알파벳/숫자/F키 등
      let keyName = key;
      if (key.length === 1) {
        keyName = key.toUpperCase();
      } else if (key === " ") {
        keyName = "Space";
      }

      parts.push(keyName);
      setLocalHotkey(parts.join("+"));
      setIsRecording(false);
    },
    [isRecording],
  );

  const handleHotkeySave = useCallback(async () => {
    try {
      setStoredShortcut(localHotkey);
      setShortcutEnabled(localHotkeyEnabled);
      storeSetHotkey(localHotkey);
      storeSetHotkeyEnabled(localHotkeyEnabled);

      if (localHotkeyEnabled) {
        await registerShortcut(localHotkey, toggleProxy);
      } else {
        await unregisterShortcut();
      }

      setHotkeyStatus("saved");
      setTimeout(() => setHotkeyStatus("idle"), 2000);
    } catch {
      setHotkeyStatus("error");
    }
  }, [localHotkey, localHotkeyEnabled, storeSetHotkey, storeSetHotkeyEnabled]);

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
          checked={localHotkeyEnabled}
          onCheckedChange={(checked) => {
            setLocalHotkeyEnabled(checked);
          }}
        />
      </div>

      {localHotkeyEnabled && (
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
                  <ShortcutDisplay shortcut={localHotkey} />
                )}
              </div>
              <Button variant="outline" size="sm" onClick={() => setIsRecording(!isRecording)}>
                {isRecording ? t`Cancel` : t`Change`}
              </Button>
            </div>
          </div>
        </div>
      )}

      <div className="flex items-center gap-3 pt-2">
        <Button onClick={handleHotkeySave}>{t`Save`}</Button>
        {hotkeyStatus === "saved" && (
          <Badge variant="outline" className="text-green-600 border-green-600">
            <Trans>Saved</Trans>
          </Badge>
        )}
        {hotkeyStatus === "error" && (
          <Badge variant="outline" className="text-red-600 border-red-600">
            <Trans>Failed to register shortcut</Trans>
          </Badge>
        )}
      </div>
    </div>
  );
}

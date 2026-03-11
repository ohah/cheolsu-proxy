import { useCallback } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { useTheme } from "next-themes";
import { loadCatalog, locales, type Locale } from "@/shared/lib/i18n";
import { useAppSettingsStore } from "@/shared/stores/app-settings-store";
import { Switch, Select, SelectTrigger, SelectContent, SelectItem, SelectValue } from "@/shared/ui";
import { useSettingsForm } from "../settings-form";

const THEME_OPTIONS = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
] as const;

export function GeneralSettings() {
  const { t } = useLingui();
  const { theme, setTheme } = useTheme();

  const locale = useAppSettingsStore((s) => s.locale);
  const setLocale = useAppSettingsStore((s) => s.setLocale);

  const { watch, setValue } = useSettingsForm();
  const noCaching = watch("quickSettings.noCaching");
  const blockCookies = watch("quickSettings.blockCookies");
  const noGzip = watch("quickSettings.noGzip");
  const autosaveSession = watch("quickSettings.autosaveSession");
  const showConnectRequests = watch("quickSettings.showConnectRequests");

  const handleLocaleChange = useCallback(
    async (newLocale: string | null) => {
      if (!newLocale) return;
      const loc = newLocale as Locale;
      setLocale(loc);
      await loadCatalog(loc);
    },
    [setLocale],
  );

  return (
    <>
      {/* Language Section */}
      <div className="border rounded-lg p-5 space-y-5">
        <div>
          <h2 className="text-lg font-semibold">
            <Trans>Language</Trans>
          </h2>
        </div>
        <Select value={locale} onValueChange={handleLocaleChange}>
          <SelectTrigger className="w-48">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {Object.entries(locales).map(([code, name]) => (
              <SelectItem key={code} value={code} label={name}>
                {name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {/* Theme Section */}
      <div className="border rounded-lg p-5 space-y-5">
        <div>
          <h2 className="text-lg font-semibold">
            <Trans>Theme</Trans>
          </h2>
        </div>
        <Select
          value={theme}
          onValueChange={(v) => {
            if (v) setTheme(v);
          }}
        >
          <SelectTrigger className="w-48">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {THEME_OPTIONS.map((opt) => {
              const label =
                opt.value === "system" ? t`System` : opt.value === "light" ? t`Light` : t`Dark`;
              return (
                <SelectItem key={opt.value} value={opt.value} label={label}>
                  {label}
                </SelectItem>
              );
            })}
          </SelectContent>
        </Select>
      </div>

      {/* Quick Settings Section */}
      <div className="border rounded-lg p-5 space-y-5">
        <div>
          <h2 className="text-lg font-semibold">
            <Trans>Quick Settings</Trans>
          </h2>
          <p className="text-sm text-muted-foreground">
            <Trans>Quick toggles for common proxy behaviors</Trans>
          </p>
        </div>

        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <label className="text-sm font-medium">
                <Trans>No Caching</Trans>
              </label>
              <p className="text-xs text-muted-foreground">
                <Trans>
                  Prevent caching by removing conditional headers and adding no-cache directives
                </Trans>
              </p>
            </div>
            <Switch
              checked={noCaching}
              onCheckedChange={(v) => setValue("quickSettings.noCaching", v, { shouldDirty: true })}
            />
          </div>

          <div className="flex items-center justify-between">
            <div>
              <label className="text-sm font-medium">
                <Trans>Block Cookies</Trans>
              </label>
              <p className="text-xs text-muted-foreground">
                <Trans>
                  Remove Cookie headers from requests and Set-Cookie headers from responses
                </Trans>
              </p>
            </div>
            <Switch
              checked={blockCookies}
              onCheckedChange={(v) =>
                setValue("quickSettings.blockCookies", v, { shouldDirty: true })
              }
            />
          </div>

          <div className="flex items-center justify-between">
            <div>
              <label className="text-sm font-medium">
                <Trans>No Gzip</Trans>
              </label>
              <p className="text-xs text-muted-foreground">
                <Trans>
                  Remove Accept-Encoding header from requests to prevent compressed responses
                </Trans>
              </p>
            </div>
            <Switch
              checked={noGzip}
              onCheckedChange={(v) => setValue("quickSettings.noGzip", v, { shouldDirty: true })}
            />
          </div>

          <div className="flex items-center justify-between">
            <div>
              <label className="text-sm font-medium">
                <Trans>Auto Save Session</Trans>
              </label>
              <p className="text-xs text-muted-foreground">
                <Trans>
                  Automatically save the current session when the app closes and restore it on next
                  launch
                </Trans>
              </p>
            </div>
            <Switch
              checked={autosaveSession}
              onCheckedChange={(v) =>
                setValue("quickSettings.autosaveSession", v, { shouldDirty: true })
              }
            />
          </div>

          <div className="flex items-center justify-between">
            <div>
              <label className="text-sm font-medium">
                <Trans>Show CONNECT Requests</Trans>
              </label>
              <p className="text-xs text-muted-foreground">
                <Trans>Display CONNECT tunnel requests in the network list</Trans>
              </p>
            </div>
            <Switch
              checked={showConnectRequests}
              onCheckedChange={(v) =>
                setValue("quickSettings.showConnectRequests", v, { shouldDirty: true })
              }
            />
          </div>
        </div>
      </div>
    </>
  );
}

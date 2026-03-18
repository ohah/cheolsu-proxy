import { useCallback, useMemo } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { useTheme } from "next-themes";
import { loadCatalog, locales, type Locale } from "@/shared/lib/i18n";
import { useAppSettingsStore, type DetailsPanelLayout } from "@/shared/stores/app-settings-store";
import { Switch, Select, SelectTrigger, SelectContent, SelectItem, SelectValue } from "@/shared/ui";
import { TABLE_COLUMNS, type ColumnKey } from "@/widgets/network-table/model";
import { CUSTOM_THEME_KEYS } from "@/features/query-filter-editor/model/themes";
import { useSettingsForm } from "../settings-form";

/** Theme display names — custom themes use their key with title case as label (proper nouns, not translated) */
const CUSTOM_THEME_LABELS: Record<string, string> = {
  dracula: "Dracula",
  nord: "Nord",
  monokai: "Monokai",
  "solarized-dark": "Solarized Dark",
  "github-dark": "GitHub Dark",
};

const THEME_OPTIONS = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
  ...CUSTOM_THEME_KEYS.map((key) => ({ value: key, label: CUSTOM_THEME_LABELS[key] ?? key })),
] as const;

export function GeneralSettings() {
  const { t } = useLingui();
  const { theme, setTheme } = useTheme();

  const locale = useAppSettingsStore((s) => s.locale);
  const setLocale = useAppSettingsStore((s) => s.setLocale);
  const detailsPanelLayout = useAppSettingsStore((s) => s.detailsPanelLayout);
  const setDetailsPanelLayout = useAppSettingsStore((s) => s.setDetailsPanelLayout);
  const storedColumns = useAppSettingsStore((s) => s.visibleColumns);
  const setStoredColumns = useAppSettingsStore((s) => s.setVisibleColumns);
  const visibleColumnsSet = useMemo(() => new Set(storedColumns as ColumnKey[]), [storedColumns]);

  const handleToggleColumn = useCallback(
    (key: ColumnKey, checked: boolean) => {
      const next = new Set(visibleColumnsSet);
      if (checked) {
        next.add(key);
      } else {
        if (next.size <= 1) return;
        next.delete(key);
      }
      setStoredColumns([...next]);
    },
    [visibleColumnsSet, setStoredColumns],
  );

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
              const themeLabels: Record<string, string> = {
                system: t`System`,
                light: t`Light`,
                dark: t`Dark`,
              };
              const label = themeLabels[opt.value] ?? opt.label;
              return (
                <SelectItem key={opt.value} value={opt.value} label={label}>
                  {label}
                </SelectItem>
              );
            })}
          </SelectContent>
        </Select>
      </div>

      {/* Details Panel Layout Section */}
      <div className="border rounded-lg p-5 space-y-5">
        <div>
          <h2 className="text-lg font-semibold">
            <Trans>Details Panel Layout</Trans>
          </h2>
          <p className="text-sm text-muted-foreground">
            <Trans>Choose where the transaction details panel appears</Trans>
          </p>
        </div>
        <Select
          value={detailsPanelLayout}
          onValueChange={(v) => {
            if (v) setDetailsPanelLayout(v as DetailsPanelLayout);
          }}
        >
          <SelectTrigger className="w-48">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="right" label={t`Right`}>
              {t`Right`}
            </SelectItem>
            <SelectItem value="bottom" label={t`Bottom`}>
              {t`Bottom`}
            </SelectItem>
          </SelectContent>
        </Select>
      </div>

      {/* Network Table Columns Section */}
      <div className="border rounded-lg p-5 space-y-5">
        <div>
          <h2 className="text-lg font-semibold">
            <Trans>Network Table Columns</Trans>
          </h2>
          <p className="text-sm text-muted-foreground">
            <Trans>Choose which columns to display in the network table</Trans>
          </p>
        </div>
        <div className="space-y-4">
          {TABLE_COLUMNS.map((col) => (
            <div key={col.key} className="flex items-center justify-between">
              <label className="text-sm font-medium">{col.label}</label>
              <Switch
                checked={visibleColumnsSet.has(col.key)}
                onCheckedChange={(checked) => handleToggleColumn(col.key, checked)}
                disabled={visibleColumnsSet.has(col.key) && visibleColumnsSet.size <= 1}
              />
            </div>
          ))}
        </div>
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
                setValue("quickSettings.autosaveSession", v, {
                  shouldDirty: true,
                })
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
                setValue("quickSettings.showConnectRequests", v, {
                  shouldDirty: true,
                })
              }
            />
          </div>
        </div>
      </div>
    </>
  );
}

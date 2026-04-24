import { type ReactNode, useCallback, useMemo, useState } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { useTheme } from "next-themes";
import { loadCatalog, locales, type Locale } from "@/shared/lib/i18n";
import { useAppSettingsStore, type DetailsPanelLayout } from "@/shared/stores/app-settings-store";
import { useTransactionStore } from "@/shared/stores";
import {
  Switch,
  Select,
  SelectTrigger,
  SelectContent,
  SelectItem,
  SelectValue,
  Button,
} from "@/shared/ui";
import { TABLE_COLUMNS, type ColumnKey } from "@/widgets/network-table/model";
import { CUSTOM_THEME_KEYS } from "@/shared/lib/monaco-theme";
import { formatBytes } from "@/shared/lib/format-bytes";
import { cleanOldProxyCache } from "@/shared/api/proxy";
import { useSettingsForm } from "../settings-form";
import { SettingsSection } from "./settings-section";

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

const CACHE_LIMIT_OPTIONS = [
  { value: 1, label: "1 GB" },
  { value: 2, label: "2 GB" },
  { value: 5, label: "5 GB" },
  { value: 10, label: "10 GB" },
  { value: 0, label: "Unlimited" },
] as const;

function LanguageSection() {
  const locale = useAppSettingsStore((s) => s.locale);
  const setLocale = useAppSettingsStore((s) => s.setLocale);

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
    <SettingsSection title={<Trans>Language</Trans>}>
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
    </SettingsSection>
  );
}

function ThemeSection() {
  const { t } = useLingui();
  const { theme, setTheme } = useTheme();

  const themeLabels: Record<string, string> = {
    system: t`System`,
    light: t`Light`,
    dark: t`Dark`,
  };

  return (
    <SettingsSection title={<Trans>Theme</Trans>}>
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
            const label = themeLabels[opt.value] ?? opt.label;
            return (
              <SelectItem key={opt.value} value={opt.value} label={label}>
                {label}
              </SelectItem>
            );
          })}
        </SelectContent>
      </Select>
    </SettingsSection>
  );
}

function DetailsPanelSection() {
  const { t } = useLingui();
  const detailsPanelLayout = useAppSettingsStore((s) => s.detailsPanelLayout);
  const setDetailsPanelLayout = useAppSettingsStore((s) => s.setDetailsPanelLayout);

  return (
    <SettingsSection
      title={<Trans>Details Panel Layout</Trans>}
      description={<Trans>Choose where the transaction details panel appears</Trans>}
    >
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
    </SettingsSection>
  );
}

function NetworkColumnsSection() {
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

  return (
    <SettingsSection
      title={<Trans>Network Table Columns</Trans>}
      description={<Trans>Choose which columns to display in the network table</Trans>}
    >
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
    </SettingsSection>
  );
}

interface QuickSettingItem {
  key:
    | "quickSettings.noCaching"
    | "quickSettings.blockCookies"
    | "quickSettings.noGzip"
    | "quickSettings.blockQuic"
    | "quickSettings.autosaveSession"
    | "quickSettings.showConnectRequests";
  label: ReactNode;
  description: ReactNode;
}

const QUICK_SETTINGS: QuickSettingItem[] = [
  {
    key: "quickSettings.noCaching",
    label: <Trans>No Caching</Trans>,
    description: (
      <Trans>Prevent caching by removing conditional headers and adding no-cache directives</Trans>
    ),
  },
  {
    key: "quickSettings.blockCookies",
    label: <Trans>Block Cookies</Trans>,
    description: (
      <Trans>Remove Cookie headers from requests and Set-Cookie headers from responses</Trans>
    ),
  },
  {
    key: "quickSettings.noGzip",
    label: <Trans>No Gzip</Trans>,
    description: (
      <Trans>Remove Accept-Encoding header from requests to prevent compressed responses</Trans>
    ),
  },
  {
    key: "quickSettings.blockQuic",
    label: <Trans>Block QUIC</Trans>,
    description: (
      <Trans>
        Strip Alt-Svc headers from responses to prevent QUIC/HTTP3 upgrades and force TCP/TLS
        connections through the proxy
      </Trans>
    ),
  },
  {
    key: "quickSettings.autosaveSession",
    label: <Trans>Auto Save Session</Trans>,
    description: (
      <Trans>
        Automatically save the current session when the app closes and restore it on next launch
      </Trans>
    ),
  },
  {
    key: "quickSettings.showConnectRequests",
    label: <Trans>Show CONNECT Requests</Trans>,
    description: <Trans>Display CONNECT tunnel requests in the network list</Trans>,
  },
];

function QuickSettingsSection() {
  const { watch, setValue } = useSettingsForm();

  return (
    <SettingsSection
      title={<Trans>Quick Settings</Trans>}
      description={<Trans>Quick toggles for common proxy behaviors</Trans>}
    >
      <div className="space-y-4">
        {QUICK_SETTINGS.map((setting) => (
          <div key={setting.key} className="flex items-center justify-between">
            <div>
              <label className="text-sm font-medium">{setting.label}</label>
              <p className="text-xs text-muted-foreground">{setting.description}</p>
            </div>
            <Switch
              checked={watch(setting.key)}
              onCheckedChange={(v) => setValue(setting.key, v, { shouldDirty: true })}
            />
          </div>
        ))}
      </div>
    </SettingsSection>
  );
}

function CacheManagementSection() {
  const { t } = useLingui();
  const cacheLimitBytes = useAppSettingsStore((s) => s.cacheLimitBytes);
  const setCacheLimitBytes = useAppSettingsStore((s) => s.setCacheLimitBytes);
  const totalSizeBytes = useTransactionStore((s) => s.totalSizeBytes);
  const clearTransactions = useTransactionStore((s) => s.clearTransactions);
  const [cacheCleanStatus, setCacheCleanStatus] = useState<"idle" | "cleaning" | "done">("idle");

  return (
    <SettingsSection
      title={<Trans>Cache Management</Trans>}
      description={<Trans>Manage network transaction storage and file cache</Trans>}
    >
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <label className="text-sm font-medium">
              <Trans>Current Usage</Trans>
            </label>
            <p className="text-xs text-muted-foreground">
              <Trans>In-memory transaction data size</Trans>
            </p>
          </div>
          <span className="text-sm font-mono">
            {formatBytes(totalSizeBytes)}
            {cacheLimitBytes > 0 && ` / ${formatBytes(cacheLimitBytes)}`}
          </span>
        </div>

        <div className="flex items-center justify-between">
          <div>
            <label className="text-sm font-medium">
              <Trans>Cache Limit</Trans>
            </label>
            <p className="text-xs text-muted-foreground">
              <Trans>Maximum memory for network transaction records</Trans>
            </p>
          </div>
          <Select
            value={String(cacheLimitBytes / (1024 * 1024 * 1024))}
            onValueChange={(v) => {
              if (v) setCacheLimitBytes(Number(v) * 1024 * 1024 * 1024);
            }}
          >
            <SelectTrigger className="w-36">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {CACHE_LIMIT_OPTIONS.map((opt) => {
                const label = opt.value === 0 ? t`Unlimited` : opt.label;
                return (
                  <SelectItem key={opt.value} value={String(opt.value)} label={label}>
                    {label}
                  </SelectItem>
                );
              })}
            </SelectContent>
          </Select>
        </div>

        <div className="flex gap-2 pt-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              clearTransactions();
            }}
          >
            <Trans>Clear Transaction Records</Trans>
          </Button>
          <Button
            variant="outline"
            size="sm"
            disabled={cacheCleanStatus === "cleaning"}
            onClick={async () => {
              setCacheCleanStatus("cleaning");
              try {
                await cleanOldProxyCache(0);
                setCacheCleanStatus("done");
                setTimeout(() => setCacheCleanStatus("idle"), 2000);
              } catch {
                setCacheCleanStatus("idle");
              }
            }}
          >
            {cacheCleanStatus === "cleaning" ? (
              <Trans>Cleaning...</Trans>
            ) : cacheCleanStatus === "done" ? (
              <Trans>Cleaned!</Trans>
            ) : (
              <Trans>Clear File Cache</Trans>
            )}
          </Button>
        </div>
      </div>
    </SettingsSection>
  );
}

export function GeneralSettings() {
  return (
    <>
      <LanguageSection />
      <ThemeSection />
      <DetailsPanelSection />
      <NetworkColumnsSection />
      <QuickSettingsSection />
      <CacheManagementSection />
    </>
  );
}

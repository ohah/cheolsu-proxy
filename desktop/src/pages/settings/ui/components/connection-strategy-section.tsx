import { Trans } from "@lingui/react/macro";
import { Select, SelectTrigger, SelectContent, SelectItem, SelectValue } from "@/shared/ui";
import { type ConnectionStrategy } from "@/shared/api/proxy";
import { useSettingsForm } from "../settings-form";
import { SettingsSection } from "./settings-section";

const STRATEGY_OPTIONS: { value: ConnectionStrategy; label: string }[] = [
  { value: "lazy", label: "Lazy" },
  { value: "eager", label: "Eager" },
  { value: "eager_with_fallback", label: "Eager with Fallback" },
];

export function ConnectionStrategySection() {
  const { watch, setValue } = useSettingsForm();
  const strategy = watch("connectionStrategy");

  return (
    <SettingsSection
      title={<Trans>Connection Strategy</Trans>}
      description={
        <Trans>Controls when the proxy connects to upstream servers for certificate sniffing</Trans>
      }
    >
      <div className="space-y-3">
        <div>
          <label className="text-sm font-medium mb-1.5 block">
            <Trans>Strategy</Trans>
          </label>
          <Select
            value={strategy}
            onValueChange={(v) => {
              if (v)
                setValue("connectionStrategy", v as ConnectionStrategy, {
                  shouldDirty: true,
                });
            }}
          >
            <SelectTrigger className="w-72">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {STRATEGY_OPTIONS.map((opt) => (
                <SelectItem key={opt.value} value={opt.value} label={opt.label}>
                  {opt.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        <p className="text-xs text-muted-foreground">
          {strategy === "lazy" && (
            <Trans>Connects to the server only when needed (default, sequential sniffing)</Trans>
          )}
          {strategy === "eager" && (
            <Trans>
              Starts background server connection immediately after ClientHello detection
            </Trans>
          )}
          {strategy === "eager_with_fallback" && (
            <Trans>Tries Eager first, falls back to Lazy on failure</Trans>
          )}
        </p>
      </div>
    </SettingsSection>
  );
}

import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { Input, Switch } from "@/shared/ui";
import { useSettingsForm } from "../settings-form";

export function UpstreamProxySection() {
  const { t } = useLingui();
  const { register, watch, setValue } = useSettingsForm();
  const enabled = watch("upstreamProxy.enabled");
  const useAuth = watch("upstreamProxy.useAuth");

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
        <Switch
          checked={enabled}
          onCheckedChange={(v) => setValue("upstreamProxy.enabled", v, { shouldDirty: true })}
        />
      </div>
      {enabled && (
        <div className="space-y-4 pt-2">
          <div className="flex gap-3">
            <div className="flex-1">
              <label className="text-sm font-medium mb-1.5 block">
                <Trans>Host</Trans>
              </label>
              <Input placeholder={t`proxy.company.com`} {...register("upstreamProxy.host")} />
            </div>
            <div className="w-28">
              <label className="text-sm font-medium mb-1.5 block">
                <Trans>Port</Trans>
              </label>
              <Input type="number" placeholder="8080" {...register("upstreamProxy.port")} />
            </div>
          </div>
          <div className="space-y-3">
            <div className="flex items-center gap-2">
              <Switch
                checked={useAuth}
                onCheckedChange={(v) => setValue("upstreamProxy.useAuth", v, { shouldDirty: true })}
              />
              <label className="text-sm font-medium">
                <Trans>Authentication</Trans>
              </label>
            </div>
            {useAuth && (
              <div className="flex gap-3 pl-1">
                <div className="flex-1">
                  <Input placeholder={t`Username`} {...register("upstreamProxy.username")} />
                </div>
                <div className="flex-1">
                  <Input
                    type="password"
                    placeholder={t`Password`}
                    {...register("upstreamProxy.password")}
                  />
                </div>
              </div>
            )}
          </div>
          <div>
            <label className="text-sm font-medium mb-1.5 block">
              <Trans>Bypass List</Trans>
            </label>
            <Input
              placeholder={t`localhost, 127.0.0.1, *.internal.com`}
              {...register("upstreamProxy.bypass")}
            />
            <p className="text-xs text-muted-foreground mt-1">
              <Trans>
                Comma-separated list of hosts to connect directly (supports *.domain.com wildcards)
              </Trans>
            </p>
          </div>
        </div>
      )}
    </div>
  );
}

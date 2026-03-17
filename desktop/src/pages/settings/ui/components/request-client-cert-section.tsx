import { useCallback } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { Button, Input, Switch } from "@/shared/ui";
import { useSettingsForm } from "../settings-form";

export function RequestClientCertSection() {
  const { t } = useLingui();
  const { watch, setValue } = useSettingsForm();
  const enabled = watch("requestClientCert.enabled");
  const caCertPath = watch("requestClientCert.caCertPath");
  const required = watch("requestClientCert.required");

  const handleSelectCaCert = useCallback(async () => {
    const selected = await openFileDialog({
      multiple: false,
      filters: [{ name: "Certificate", extensions: ["pem", "crt", "cer"] }],
    });
    if (selected) {
      setValue("requestClientCert.caCertPath", selected as string, {
        shouldDirty: true,
      });
    }
  }, [setValue]);

  return (
    <div className="border rounded-lg p-5 space-y-5">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold">
            <Trans>Request Client Certificate</Trans>
          </h2>
          <p className="text-sm text-muted-foreground">
            <Trans>Request a client certificate from connecting clients (mTLS server-side)</Trans>
          </p>
        </div>
        <Switch
          checked={enabled}
          onCheckedChange={(v) => setValue("requestClientCert.enabled", v, { shouldDirty: true })}
        />
      </div>
      {enabled && (
        <div className="space-y-4 pt-2">
          <div>
            <label className="text-sm font-medium mb-1.5 block">
              <Trans>CA Certificate File (optional)</Trans>
            </label>
            <div className="flex gap-2">
              <Input
                readOnly
                placeholder={t`Select CA certificate to verify clients (.pem, .crt)`}
                value={caCertPath}
                className="flex-1"
              />
              <Button
                type="button"
                variant="outline"
                onClick={handleSelectCaCert}
              >{t`Browse`}</Button>
              {caCertPath && (
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={() =>
                    setValue("requestClientCert.caCertPath", "", {
                      shouldDirty: true,
                    })
                  }
                >
                  <Trans>Clear</Trans>
                </Button>
              )}
            </div>
            <p className="text-xs text-muted-foreground mt-1">
              <Trans>
                If not specified, any client certificate will be accepted without verification
              </Trans>
            </p>
          </div>
          <div className="flex items-center gap-3">
            <Switch
              checked={required}
              onCheckedChange={(v) =>
                setValue("requestClientCert.required", v, { shouldDirty: true })
              }
            />
            <div>
              <span className="text-sm font-medium">
                <Trans>Require certificate</Trans>
              </span>
              <p className="text-xs text-muted-foreground">
                {required ? (
                  <Trans>Clients without a valid certificate will be rejected</Trans>
                ) : (
                  <Trans>Certificate is optional — clients without one can still connect</Trans>
                )}
              </p>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

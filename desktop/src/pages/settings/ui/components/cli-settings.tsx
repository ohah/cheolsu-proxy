import { useState, useEffect, useCallback } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { installCli, uninstallCli, checkCliInstalled } from "@/shared/api/proxy";
import { Button, Badge } from "@/shared/ui";

export function CliSettings() {
  const { t } = useLingui();
  const [cliInstalled, setCliInstalled] = useState(false);
  const [cliInstalling, setCliInstalling] = useState(false);
  const [cliMessage, setCliMessage] = useState("");

  // CLI 설치 상태 확인
  useEffect(() => {
    checkCliInstalled().then(setCliInstalled);
  }, []);

  const handleInstallCli = useCallback(async () => {
    setCliInstalling(true);
    setCliMessage("");
    try {
      const msg = await installCli();
      setCliMessage(msg);
      setCliInstalled(true);
    } catch (e) {
      setCliMessage(String(e));
    } finally {
      setCliInstalling(false);
    }
  }, []);

  const handleUninstallCli = useCallback(async () => {
    setCliInstalling(true);
    setCliMessage("");
    try {
      const msg = await uninstallCli();
      setCliMessage(msg);
      setCliInstalled(false);
    } catch (e) {
      setCliMessage(String(e));
    } finally {
      setCliInstalling(false);
    }
  }, []);

  return (
    <div className="border rounded-lg p-5 space-y-4">
      <div>
        <h2 className="text-lg font-semibold">
          <Trans>Terminal Command</Trans>
        </h2>
        <p className="text-sm text-muted-foreground">
          <Trans>
            Install the <code className="text-xs bg-muted px-1 py-0.5 rounded">cheolsu</code>{" "}
            command to use the TUI from your terminal
          </Trans>
        </p>
      </div>
      <div className="flex items-center gap-3">
        <Button onClick={handleInstallCli} disabled={cliInstalling}>
          {cliInstalling ? t`Installing...` : cliInstalled ? t`Reinstall` : t`Install`}
        </Button>
        {cliInstalled && (
          <Button variant="outline" onClick={handleUninstallCli} disabled={cliInstalling}>
            {t`Uninstall`}
          </Button>
        )}
        {cliInstalled && (
          <Badge variant="outline" className="text-green-600 border-green-600">
            <Trans>Installed</Trans>
          </Badge>
        )}
      </div>
      {cliMessage && <p className="text-xs text-muted-foreground">{cliMessage}</p>}
    </div>
  );
}

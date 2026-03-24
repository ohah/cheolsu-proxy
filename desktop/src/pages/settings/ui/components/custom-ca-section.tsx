import { useState, useEffect, useCallback } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import {
  importCustomCa,
  importCustomCaPkcs12,
  removeCustomCa,
  getCustomCaStatus,
  type CertificateInfo,
} from "@/shared/api/proxy";
import { useProxyStore } from "@/shared/stores/proxy-store";
import { useFileSelector } from "@/shared/hooks/use-file-selector";
import { Button, Badge, Input } from "@/shared/ui";
import { SettingsSection } from "./settings-section";

export function CustomCaSection() {
  const { t } = useLingui();
  const isProxyConnected = useProxyStore((s) => s.isConnected);

  const [mode, setMode] = useState<"pem" | "pkcs12">("pem");
  const [certPath, setCertPath] = useState("");
  const [keyPath, setKeyPath] = useState("");
  const [p12Path, setP12Path] = useState("");
  const [p12Password, setP12Password] = useState("");
  const [importing, setImporting] = useState(false);
  const [status, setStatus] = useState<"idle" | "imported" | "removed" | "error">("idle");
  const [errorMessage, setErrorMessage] = useState("");
  const [customCaInfo, setCustomCaInfo] = useState<CertificateInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const selectFile = useFileSelector();

  // 초기 로드: 현재 커스텀 CA 상태 확인
  useEffect(() => {
    getCustomCaStatus()
      .then((info) => {
        setCustomCaInfo(info);
        setLoading(false);
      })
      .catch(() => setLoading(false));
  }, []);

  const handleSelectCert = useCallback(async () => {
    const path = await selectFile({ extensions: ["pem", "crt", "cer", "der"] });
    if (path) {
      setCertPath(path);
      setStatus("idle");
    }
  }, [selectFile]);

  const handleSelectKey = useCallback(async () => {
    const path = await selectFile({ extensions: ["pem", "key"] });
    if (path) {
      setKeyPath(path);
      setStatus("idle");
    }
  }, [selectFile]);

  const handleSelectP12 = useCallback(async () => {
    const path = await selectFile({ extensions: ["p12", "pfx"] });
    if (path) {
      setP12Path(path);
      setStatus("idle");
    }
  }, [selectFile]);

  const handleImport = useCallback(async () => {
    setImporting(true);
    setStatus("idle");
    setErrorMessage("");
    try {
      let info: CertificateInfo;
      if (mode === "pem") {
        if (!certPath || !keyPath) {
          setErrorMessage(t`Please select both certificate and key files`);
          setStatus("error");
          setImporting(false);
          return;
        }
        info = await importCustomCa(certPath, keyPath);
      } else {
        if (!p12Path) {
          setErrorMessage(t`Please select a PKCS12 file`);
          setStatus("error");
          setImporting(false);
          return;
        }
        info = await importCustomCaPkcs12(p12Path, p12Password);
      }
      setCustomCaInfo(info);
      setStatus("imported");
    } catch (e) {
      setErrorMessage(String(e));
      setStatus("error");
    } finally {
      setImporting(false);
    }
  }, [mode, certPath, keyPath, p12Path, p12Password, t]);

  const handleRemove = useCallback(async () => {
    setImporting(true);
    setStatus("idle");
    setErrorMessage("");
    try {
      await removeCustomCa();
      setCustomCaInfo(null);
      setCertPath("");
      setKeyPath("");
      setP12Path("");
      setP12Password("");
      setStatus("removed");
    } catch (e) {
      setErrorMessage(String(e));
      setStatus("error");
    } finally {
      setImporting(false);
    }
  }, []);

  if (loading) return null;

  return (
    <SettingsSection
      title={<Trans>Custom CA Certificate</Trans>}
      description={<Trans>Use your own CA certificate instead of the auto-generated one</Trans>}
    >
      {/* 현재 커스텀 CA 정보 */}
      {customCaInfo && (
        <div className="bg-muted/50 rounded-lg p-4 space-y-3">
          <div className="flex items-center justify-between">
            <h3 className="text-sm font-semibold">
              <Trans>Active Custom CA</Trans>
            </h3>
            <Badge variant="outline" className="text-green-600 border-green-600">
              <Trans>Active</Trans>
            </Badge>
          </div>
          <div className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 text-sm">
            {customCaInfo.subject_cn && (
              <>
                <span className="text-muted-foreground">CN</span>
                <span className="font-mono">{customCaInfo.subject_cn}</span>
              </>
            )}
            {customCaInfo.organization && (
              <>
                <span className="text-muted-foreground">
                  <Trans>Organization</Trans>
                </span>
                <span className="font-mono">{customCaInfo.organization}</span>
              </>
            )}
            <>
              <span className="text-muted-foreground">
                <Trans>Valid Until</Trans>
              </span>
              <span className="font-mono">{customCaInfo.not_after}</span>
            </>
            <>
              <span className="text-muted-foreground">SHA-256</span>
              <span className="font-mono text-xs break-all">{customCaInfo.fingerprint_sha256}</span>
            </>
          </div>
          <div className="flex items-center gap-2 pt-1">
            <Button variant="destructive" size="sm" onClick={handleRemove} disabled={importing}>
              <Trans>Remove Custom CA</Trans>
            </Button>
            {isProxyConnected && (
              <p className="text-xs text-yellow-600">
                <Trans>Restart proxy to apply changes</Trans>
              </p>
            )}
          </div>
        </div>
      )}

      {/* 임포트 폼 */}
      {!customCaInfo && (
        <div className="space-y-4">
          {/* 모드 선택 */}
          <div className="flex gap-2">
            <Button
              variant={mode === "pem" ? "default" : "outline"}
              size="sm"
              onClick={() => setMode("pem")}
            >
              PEM / DER
            </Button>
            <Button
              variant={mode === "pkcs12" ? "default" : "outline"}
              size="sm"
              onClick={() => setMode("pkcs12")}
            >
              PKCS12 (.p12/.pfx)
            </Button>
          </div>

          {mode === "pem" ? (
            <div className="space-y-3">
              <div>
                <label className="text-sm font-medium mb-1.5 block">
                  <Trans>CA Certificate File</Trans>
                </label>
                <div className="flex gap-2">
                  <Input
                    readOnly
                    placeholder={t`Select CA certificate (.pem, .crt, .cer, .der)`}
                    value={certPath}
                    className="flex-1"
                  />
                  <Button variant="outline" onClick={handleSelectCert}>
                    {t`Browse`}
                  </Button>
                </div>
              </div>
              <div>
                <label className="text-sm font-medium mb-1.5 block">
                  <Trans>CA Key File</Trans>
                </label>
                <div className="flex gap-2">
                  <Input
                    readOnly
                    placeholder={t`Select CA key file (.pem, .key)`}
                    value={keyPath}
                    className="flex-1"
                  />
                  <Button variant="outline" onClick={handleSelectKey}>
                    {t`Browse`}
                  </Button>
                </div>
              </div>
            </div>
          ) : (
            <div className="space-y-3">
              <div>
                <label className="text-sm font-medium mb-1.5 block">
                  <Trans>PKCS12 File</Trans>
                </label>
                <div className="flex gap-2">
                  <Input
                    readOnly
                    placeholder={t`Select PKCS12 file (.p12, .pfx)`}
                    value={p12Path}
                    className="flex-1"
                  />
                  <Button variant="outline" onClick={handleSelectP12}>
                    {t`Browse`}
                  </Button>
                </div>
              </div>
              <div>
                <label className="text-sm font-medium mb-1.5 block">
                  <Trans>Password</Trans>
                </label>
                <Input
                  type="password"
                  placeholder={t`PKCS12 password (leave empty if none)`}
                  value={p12Password}
                  onChange={(e) => setP12Password(e.target.value)}
                />
              </div>
            </div>
          )}

          <div className="flex items-center gap-3">
            <Button onClick={handleImport} disabled={importing}>
              {importing ? t`Importing...` : t`Import`}
            </Button>
            {status === "imported" && (
              <Badge variant="outline" className="text-green-600 border-green-600">
                <Trans>Imported</Trans>
              </Badge>
            )}
            {status === "error" && (
              <Badge variant="outline" className="text-red-600 border-red-600">
                <Trans>Failed</Trans>
              </Badge>
            )}
          </div>

          {status === "error" && errorMessage && (
            <p className="text-xs text-red-600">{errorMessage}</p>
          )}

          <p className="text-xs text-muted-foreground">
            <Trans>
              The certificate must be a CA certificate (BasicConstraints CA=true). After importing,
              restart the proxy to use the new CA. You also need to install this CA on your system
              and devices.
            </Trans>
          </p>
        </div>
      )}

      {status === "removed" && (
        <div className="flex items-center gap-2">
          <Badge variant="outline" className="text-green-600 border-green-600">
            <Trans>Removed</Trans>
          </Badge>
          {isProxyConnected && (
            <p className="text-xs text-yellow-600">
              <Trans>Restart proxy to use auto-generated CA</Trans>
            </p>
          )}
        </div>
      )}
    </SettingsSection>
  );
}

import { useState, useCallback } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import {
  parseCertificateInfo,
  type CertificateInfo,
  type DomainClientCertConfig,
} from "@/shared/api/proxy";
import { Button, Input, Switch, Badge } from "@/shared/ui";
import { useSettingsForm } from "../settings-form";

export function ClientCertificateSection() {
  const { t } = useLingui();
  const { watch, setValue } = useSettingsForm();
  const enabled = watch("clientCert.enabled");
  const certPath = watch("clientCert.certPath");
  const keyPath = watch("clientCert.keyPath");
  const domainCerts: DomainClientCertConfig[] = watch("clientCert.domainCerts");

  const [certInfo, setCertInfo] = useState<CertificateInfo | null>(null);
  const [certInfoLoading, setCertInfoLoading] = useState(false);
  const [newDomainPattern, setNewDomainPattern] = useState("");
  const [newDomainCertPath, setNewDomainCertPath] = useState("");
  const [newDomainKeyPath, setNewDomainKeyPath] = useState("");

  const loadCertInfo = useCallback(async (path: string) => {
    setCertInfoLoading(true);
    try {
      setCertInfo(await parseCertificateInfo(path));
    } catch {
      setCertInfo(null);
    } finally {
      setCertInfoLoading(false);
    }
  }, []);

  const handleSelectCert = useCallback(async () => {
    const selected = await openFileDialog({
      multiple: false,
      filters: [{ name: "Certificate", extensions: ["pem", "crt", "cer"] }],
    });
    if (selected) {
      const path = selected as string;
      setValue("clientCert.certPath", path, { shouldDirty: true });
      loadCertInfo(path);
    }
  }, [setValue, loadCertInfo]);

  const handleSelectKey = useCallback(async () => {
    const selected = await openFileDialog({
      multiple: false,
      filters: [{ name: "Key", extensions: ["pem", "key"] }],
    });
    if (selected) {
      setValue("clientCert.keyPath", selected as string, { shouldDirty: true });
    }
  }, [setValue]);

  const handleSelectDomainCert = useCallback(async () => {
    const selected = await openFileDialog({
      multiple: false,
      filters: [{ name: "Certificate", extensions: ["pem", "crt", "cer"] }],
    });
    if (selected) {
      setNewDomainCertPath(selected as string);
    }
  }, []);

  const handleSelectDomainKey = useCallback(async () => {
    const selected = await openFileDialog({
      multiple: false,
      filters: [{ name: "Key", extensions: ["pem", "key"] }],
    });
    if (selected) {
      setNewDomainKeyPath(selected as string);
    }
  }, []);

  const handleAddDomainCert = useCallback(() => {
    const pattern = newDomainPattern.trim();
    if (!pattern || !newDomainCertPath || !newDomainKeyPath) return;
    if (domainCerts.some((dc) => dc.domain_pattern === pattern)) return;
    setValue(
      "clientCert.domainCerts",
      [
        ...domainCerts,
        {
          domain_pattern: pattern,
          cert_path: newDomainCertPath,
          key_path: newDomainKeyPath,
          enabled: true,
        },
      ],
      { shouldDirty: true },
    );
    setNewDomainPattern("");
    setNewDomainCertPath("");
    setNewDomainKeyPath("");
  }, [newDomainPattern, newDomainCertPath, newDomainKeyPath, domainCerts, setValue]);

  const toggleDomainCert = useCallback(
    (idx: number) => {
      setValue(
        "clientCert.domainCerts",
        domainCerts.map((dc, i: number) => (i === idx ? { ...dc, enabled: !dc.enabled } : dc)),
        { shouldDirty: true },
      );
    },
    [domainCerts, setValue],
  );

  const removeDomainCert = useCallback(
    (idx: number) => {
      setValue(
        "clientCert.domainCerts",
        domainCerts.filter((_, i: number) => i !== idx),
        { shouldDirty: true },
      );
    },
    [domainCerts, setValue],
  );

  return (
    <div className="border rounded-lg p-5 space-y-5">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold">
            <Trans>Client Certificate</Trans>
          </h2>
          <p className="text-sm text-muted-foreground">
            <Trans>
              Present a client certificate when connecting to servers that require mTLS
              authentication
            </Trans>
          </p>
        </div>
        <Switch
          checked={enabled}
          onCheckedChange={(v) => setValue("clientCert.enabled", v, { shouldDirty: true })}
        />
      </div>

      {enabled && (
        <div className="space-y-4 pt-2">
          <div>
            <label className="text-sm font-medium mb-1.5 block">
              <Trans>Certificate File</Trans>
            </label>
            <div className="flex gap-2">
              <Input
                readOnly
                placeholder={t`Select certificate file (.pem, .crt)`}
                value={certPath}
                className="flex-1"
              />
              <Button
                type="button"
                variant="outline"
                onClick={handleSelectCert}
              >{t`Browse`}</Button>
            </div>
          </div>
          <div>
            <label className="text-sm font-medium mb-1.5 block">
              <Trans>Key File</Trans>
            </label>
            <div className="flex gap-2">
              <Input
                readOnly
                placeholder={t`Select key file (.pem, .key)`}
                value={keyPath}
                className="flex-1"
              />
              <Button type="button" variant="outline" onClick={handleSelectKey}>{t`Browse`}</Button>
            </div>
          </div>

          {certInfoLoading && (
            <p className="text-sm text-muted-foreground">
              <Trans>Loading certificate info...</Trans>
            </p>
          )}

          {certInfo && !certInfoLoading && (
            <div className="bg-muted/50 rounded-lg p-4 space-y-3">
              <h3 className="text-sm font-semibold">
                <Trans>Certificate Details</Trans>
              </h3>
              <div className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 text-sm">
                {certInfo.subject_cn && (
                  <>
                    <span className="text-muted-foreground">CN</span>
                    <span className="font-mono">{certInfo.subject_cn}</span>
                  </>
                )}
                {certInfo.issuer_cn && (
                  <>
                    <span className="text-muted-foreground">
                      <Trans>Issuer</Trans>
                    </span>
                    <span className="font-mono">{certInfo.issuer_cn}</span>
                  </>
                )}
                {certInfo.organization && (
                  <>
                    <span className="text-muted-foreground">
                      <Trans>Organization</Trans>
                    </span>
                    <span className="font-mono">{certInfo.organization}</span>
                  </>
                )}
                {certInfo.sans_dns.length > 0 && (
                  <>
                    <span className="text-muted-foreground">SAN (DNS)</span>
                    <span className="font-mono">{certInfo.sans_dns.join(", ")}</span>
                  </>
                )}
                {certInfo.sans_ip.length > 0 && (
                  <>
                    <span className="text-muted-foreground">SAN (IP)</span>
                    <span className="font-mono">{certInfo.sans_ip.join(", ")}</span>
                  </>
                )}
                <>
                  <span className="text-muted-foreground">
                    <Trans>Valid From</Trans>
                  </span>
                  <span className="font-mono">{certInfo.not_before}</span>
                </>
                <>
                  <span className="text-muted-foreground">
                    <Trans>Valid Until</Trans>
                  </span>
                  <span className="font-mono">{certInfo.not_after}</span>
                </>
                <>
                  <span className="text-muted-foreground">
                    <Trans>Serial Number</Trans>
                  </span>
                  <span className="font-mono text-xs break-all">{certInfo.serial_number}</span>
                </>
                <>
                  <span className="text-muted-foreground">SHA-256</span>
                  <span className="font-mono text-xs break-all">{certInfo.fingerprint_sha256}</span>
                </>
                <>
                  <span className="text-muted-foreground">CA</span>
                  <span className="font-mono">{certInfo.is_ca ? "Yes" : "No"}</span>
                </>
                <>
                  <span className="text-muted-foreground">
                    <Trans>Chain Length</Trans>
                  </span>
                  <span className="font-mono">{certInfo.chain_length}</span>
                </>
              </div>
              {new Date(certInfo.not_after) < new Date() && (
                <Badge variant="outline" className="text-red-600 border-red-600">
                  <Trans>Certificate expired</Trans>
                </Badge>
              )}
            </div>
          )}

          <p className="text-xs text-muted-foreground">
            <Trans>Supports PEM-encoded certificates and keys (RSA, ECDSA, PKCS#8)</Trans>
          </p>

          {/* Domain-specific Certificates */}
          <div className="space-y-4 pt-4 border-t">
            <div>
              <h3 className="text-sm font-semibold">
                <Trans>Domain-specific Certificates</Trans>
              </h3>
              <p className="text-xs text-muted-foreground mt-1">
                <Trans>
                  Use different client certificates for specific domains. Supports wildcards
                  (*.example.com). Currently validates certificates only — domain-specific routing
                  coming soon.
                </Trans>
              </p>
            </div>
            <div className="space-y-2">
              <Input
                placeholder="*.example.com"
                value={newDomainPattern}
                onChange={(e) => setNewDomainPattern(e.target.value)}
              />
              <div className="flex gap-2">
                <Input
                  readOnly
                  placeholder={t`Certificate file`}
                  value={newDomainCertPath}
                  className="flex-1"
                />
                <Button
                  type="button"
                  variant="outline"
                  onClick={handleSelectDomainCert}
                >{t`Browse`}</Button>
              </div>
              <div className="flex gap-2">
                <Input
                  readOnly
                  placeholder={t`Key file`}
                  value={newDomainKeyPath}
                  className="flex-1"
                />
                <Button
                  type="button"
                  variant="outline"
                  onClick={handleSelectDomainKey}
                >{t`Browse`}</Button>
              </div>
              <Button
                type="button"
                onClick={handleAddDomainCert}
                disabled={!newDomainPattern.trim() || !newDomainCertPath || !newDomainKeyPath}
              >
                <Trans>Add Domain Certificate</Trans>
              </Button>
            </div>
            {domainCerts.length > 0 && (
              <div className="border rounded-lg divide-y">
                {domainCerts.map((dc: DomainClientCertConfig, idx: number) => (
                  <div
                    key={dc.domain_pattern}
                    className="flex items-center justify-between px-4 py-2"
                  >
                    <div className="flex items-center gap-3">
                      <Switch checked={dc.enabled} onCheckedChange={() => toggleDomainCert(idx)} />
                      <div>
                        <span
                          className={`font-mono text-sm ${dc.enabled ? "text-foreground" : "text-muted-foreground line-through"}`}
                        >
                          {dc.domain_pattern}
                        </span>
                        <span className="text-xs text-muted-foreground block truncate max-w-xs">
                          {dc.cert_path}
                        </span>
                      </div>
                    </div>
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      onClick={() => removeDomainCert(idx)}
                      className="text-muted-foreground hover:text-destructive"
                    >
                      <Trans>Remove</Trans>
                    </Button>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

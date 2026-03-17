import { useMemo, useState } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { Copy, Check, ShieldAlert, ShieldCheck, Lock, Globe } from "lucide-react";
import type { ServerCertInfo } from "@/entities/proxy";

import { Badge, Button, Tooltip, TooltipTrigger, TooltipContent } from "@/shared/ui";

interface TransactionCertificateProps {
  serverCert?: ServerCertInfo | null;
}

function CopyButton({ value }: { value: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard.writeText(value);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <Tooltip>
      <TooltipTrigger render={<div />}>
        <Button variant="ghost" size="icon" className="h-6 w-6 shrink-0" onClick={handleCopy}>
          {copied ? <Check className="h-3 w-3 text-green-500" /> : <Copy className="h-3 w-3" />}
        </Button>
      </TooltipTrigger>
      <TooltipContent>
        {copied ? <Trans>Copied!</Trans> : <Trans>Copy to clipboard</Trans>}
      </TooltipContent>
    </Tooltip>
  );
}

function SectionTitle({ children }: { children: React.ReactNode }) {
  return (
    <h3 className="text-sm font-semibold text-foreground mb-2">{children}</h3>
  );
}

function InfoRow({ label, children }: { label: React.ReactNode; children: React.ReactNode }) {
  return (
    <div className="grid grid-cols-[140px_1fr] items-start gap-2 py-1.5">
      <span className="text-xs text-muted-foreground">{label}</span>
      <div className="text-xs">{children}</div>
    </div>
  );
}

export function TransactionCertificate({ serverCert }: TransactionCertificateProps) {
  const { t } = useLingui();

  const isExpired = useMemo(() => {
    if (!serverCert?.not_after) return false;
    return new Date(serverCert.not_after) < new Date();
  }, [serverCert?.not_after]);

  const isNotYetValid = useMemo(() => {
    if (!serverCert?.not_before) return false;
    return new Date(serverCert.not_before) > new Date();
  }, [serverCert?.not_before]);

  if (!serverCert) {
    return (
      <div className="flex flex-col items-center justify-center text-muted-foreground text-sm py-8 gap-2">
        <Globe className="h-8 w-8" />
        <Trans>HTTP connection (no TLS)</Trans>
      </div>
    );
  }

  return (
    <div className="space-y-5">
      {/* Subject */}
      <div className="rounded-lg border border-border p-4">
        <SectionTitle><Trans>Subject</Trans></SectionTitle>
        <div className="space-y-0">
          {serverCert.subject_cn && (
            <InfoRow label={t`Common Name (CN)`}>
              <div className="flex items-center gap-1.5">
                <Lock className="h-3 w-3 text-muted-foreground shrink-0" />
                <span className="font-medium">{serverCert.subject_cn}</span>
              </div>
            </InfoRow>
          )}
          {serverCert.organization && (
            <InfoRow label={t`Organization`}>
              {serverCert.organization}
            </InfoRow>
          )}
          <InfoRow label={t`CA Certificate`}>
            {serverCert.is_ca ? (
              <Badge variant="secondary"><Trans>Yes</Trans></Badge>
            ) : (
              <Badge variant="outline"><Trans>No</Trans></Badge>
            )}
          </InfoRow>
        </div>
      </div>

      {/* Issuer */}
      <div className="rounded-lg border border-border p-4">
        <SectionTitle><Trans>Issuer</Trans></SectionTitle>
        <div className="space-y-0">
          {serverCert.issuer_cn ? (
            <InfoRow label={t`Common Name (CN)`}>
              {serverCert.issuer_cn}
            </InfoRow>
          ) : (
            <div className="text-xs text-muted-foreground">
              <Trans>Issuer information not available</Trans>
            </div>
          )}
        </div>
      </div>

      {/* Validity */}
      <div className="rounded-lg border border-border p-4">
        <SectionTitle><Trans>Validity Period</Trans></SectionTitle>
        <div className="space-y-0">
          {serverCert.not_before && (
            <InfoRow label={t`Not Before`}>
              <span className={isNotYetValid ? "text-red-500 font-medium" : ""}>
                {serverCert.not_before}
              </span>
              {isNotYetValid && (
                <Badge variant="destructive" className="ml-2 text-[10px]">
                  <Trans>Not yet valid</Trans>
                </Badge>
              )}
            </InfoRow>
          )}
          {serverCert.not_after && (
            <InfoRow label={t`Not After`}>
              <div className="flex items-center gap-2">
                <span className={isExpired ? "text-red-500 font-medium" : ""}>
                  {serverCert.not_after}
                </span>
                {isExpired ? (
                  <Badge variant="destructive" className="text-[10px]">
                    <ShieldAlert className="h-3 w-3 mr-1" />
                    <Trans>Expired</Trans>
                  </Badge>
                ) : (
                  <Badge variant="secondary" className="text-[10px]">
                    <ShieldCheck className="h-3 w-3 mr-1" />
                    <Trans>Valid</Trans>
                  </Badge>
                )}
              </div>
            </InfoRow>
          )}
        </div>
      </div>

      {/* SAN */}
      {(serverCert.sans_dns.length > 0 || serverCert.sans_ip.length > 0) && (
        <div className="rounded-lg border border-border p-4">
          <SectionTitle><Trans>Subject Alternative Names (SAN)</Trans></SectionTitle>
          <div className="space-y-2">
            {serverCert.sans_dns.length > 0 && (
              <div>
                <div className="text-xs text-muted-foreground mb-1">DNS</div>
                <div className="flex flex-wrap gap-1.5">
                  {serverCert.sans_dns.map((dns) => (
                    <Badge key={dns} variant="outline" className="font-mono text-[11px]">
                      {dns}
                    </Badge>
                  ))}
                </div>
              </div>
            )}
            {serverCert.sans_ip.length > 0 && (
              <div>
                <div className="text-xs text-muted-foreground mb-1">IP</div>
                <div className="flex flex-wrap gap-1.5">
                  {serverCert.sans_ip.map((ip) => (
                    <Badge key={ip} variant="outline" className="font-mono text-[11px]">
                      {ip}
                    </Badge>
                  ))}
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Fingerprint & Serial */}
      <div className="rounded-lg border border-border p-4">
        <SectionTitle><Trans>Certificate Details</Trans></SectionTitle>
        <div className="space-y-0">
          {serverCert.serial_number && (
            <InfoRow label={t`Serial Number`}>
              <div className="flex items-center gap-1">
                <code className="font-mono text-[11px] break-all">{serverCert.serial_number}</code>
                <CopyButton value={serverCert.serial_number} />
              </div>
            </InfoRow>
          )}
          {serverCert.fingerprint_sha256 && (
            <InfoRow label={t`SHA-256 Fingerprint`}>
              <div className="flex items-center gap-1">
                <code className="font-mono text-[11px] break-all">{serverCert.fingerprint_sha256}</code>
                <CopyButton value={serverCert.fingerprint_sha256} />
              </div>
            </InfoRow>
          )}
        </div>
      </div>

      {/* Chain & ALPN */}
      <div className="rounded-lg border border-border p-4">
        <SectionTitle><Trans>Connection Info</Trans></SectionTitle>
        <div className="space-y-0">
          <InfoRow label={t`Certificate Chain`}>
            <Trans>{serverCert.chain_length} certificate(s)</Trans>
          </InfoRow>
          {serverCert.negotiated_alpn && (
            <InfoRow label={t`Negotiated ALPN`}>
              <Badge variant="secondary" className="font-mono text-[11px]">
                {serverCert.negotiated_alpn}
              </Badge>
            </InfoRow>
          )}
        </div>
      </div>
    </div>
  );
}

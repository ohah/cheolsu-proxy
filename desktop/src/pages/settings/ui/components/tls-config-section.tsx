import { useState, useEffect, useCallback } from "react";
import { Trans } from "@lingui/react/macro";
import { Button, Badge } from "@/shared/ui";
import { invoke } from "@tauri-apps/api/core";
import { Trash2 } from "lucide-react";

interface TlsConfigRule {
  domain_pattern: string;
  require_openssl: boolean;
  handshake_timeout_secs: number | null;
  disable_cert_verify: boolean;
  priority: number;
}

interface LearnedTlsStrategy {
  domain: string;
  strategy: string;
}

export function TlsConfigSection() {
  const [rules, setRules] = useState<TlsConfigRule[]>([]);
  const [strategies, setStrategies] = useState<LearnedTlsStrategy[]>([]);
  const [loading, setLoading] = useState(true);

  const fetchData = useCallback(async () => {
    try {
      const [rulesResult, strategiesResult] = await Promise.all([
        invoke<TlsConfigRule[]>("get_tls_config_rules").catch(() => []),
        invoke<LearnedTlsStrategy[]>("get_learned_tls_strategies").catch(() => []),
      ]);
      setRules(rulesResult);
      setStrategies(strategiesResult);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  const handleClearStrategies = useCallback(async () => {
    await invoke("clear_learned_tls_strategies").catch(() => {});
    setStrategies([]);
  }, []);

  if (loading) return null;

  return (
    <div className="space-y-4">
      {/* TLS 규칙 목록 */}
      <div className="border rounded-lg p-5 space-y-4">
        <div>
          <h2 className="text-lg font-semibold">
            <Trans>TLS Engine Rules</Trans>
          </h2>
          <p className="text-sm text-muted-foreground">
            <Trans>
              Domain-specific TLS engine rules. Domains matched here use OpenSSL instead of rustls
              for compatibility.
            </Trans>
          </p>
        </div>

        {rules.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            <Trans>No rules configured.</Trans>
          </p>
        ) : (
          <div className="border rounded-lg divide-y">
            {rules.map((rule) => (
              <div
                key={rule.domain_pattern}
                className="flex items-center justify-between px-4 py-2"
              >
                <div className="flex items-center gap-3">
                  <span className="font-mono text-sm">{rule.domain_pattern}</span>
                  {rule.require_openssl && (
                    <Badge variant="outline" className="text-amber-600 border-amber-500/50 text-xs">
                      OpenSSL
                    </Badge>
                  )}
                  {rule.handshake_timeout_secs && (
                    <Badge variant="outline" className="text-xs">
                      {rule.handshake_timeout_secs}s
                    </Badge>
                  )}
                  {rule.disable_cert_verify && (
                    <Badge
                      variant="outline"
                      className="text-xs text-orange-500 border-orange-400/50"
                    >
                      <Trans>No Verify</Trans>
                    </Badge>
                  )}
                </div>
                <span className="text-xs text-muted-foreground">p={rule.priority}</span>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* 학습된 TLS 전략 */}
      <div className="border rounded-lg p-5 space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-lg font-semibold">
              <Trans>Learned TLS Strategies</Trans>
            </h2>
            <p className="text-sm text-muted-foreground">
              <Trans>
                Domains that failed with the default TLS engine are recorded here. On the next
                connection, the alternate engine will be used automatically.
              </Trans>
            </p>
          </div>
          {strategies.length > 0 && (
            <Button type="button" variant="outline" size="sm" onClick={handleClearStrategies}>
              <Trash2 className="size-3.5 mr-1.5" />
              <Trans>Clear All</Trans>
            </Button>
          )}
        </div>

        {strategies.length === 0 ? (
          <p className="text-sm text-muted-foreground italic">
            <Trans>No learned strategies yet. They will appear after TLS fallback events.</Trans>
          </p>
        ) : (
          <div className="border rounded-lg divide-y">
            {strategies.map((s) => (
              <div key={s.domain} className="flex items-center justify-between px-4 py-2">
                <span className="font-mono text-sm">{s.domain}</span>
                <Badge
                  variant="outline"
                  className={
                    s.strategy === "OpenSslOnly"
                      ? "text-amber-600 border-amber-500/50 text-xs"
                      : "text-blue-600 border-blue-500/50 text-xs"
                  }
                >
                  {s.strategy === "OpenSslOnly" ? "OpenSSL" : "rustls"}
                </Badge>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

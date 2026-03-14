import { useMemo } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import type { TimingWaterfall } from "@/entities/proxy";

interface TransactionTimingProps {
  timing?: TimingWaterfall;
}

interface TimingSegment {
  label: string;
  key: keyof Omit<TimingWaterfall, "total_ms">;
  color: string;
  bgClass: string;
}

const TIMING_SEGMENTS: TimingSegment[] = [
  {
    label: "DNS Lookup",
    key: "dns_lookup_ms",
    color: "#2dd4bf",
    bgClass: "bg-teal-400",
  },
  {
    label: "TCP Connection",
    key: "tcp_connection_ms",
    color: "#fb923c",
    bgClass: "bg-orange-400",
  },
  {
    label: "TLS Handshake",
    key: "tls_handshake_ms",
    color: "#a78bfa",
    bgClass: "bg-violet-400",
  },
  {
    label: "TTFB (Time to First Byte)",
    key: "ttfb_ms",
    color: "#4ade80",
    bgClass: "bg-green-400",
  },
  {
    label: "Content Download",
    key: "content_download_ms",
    color: "#60a5fa",
    bgClass: "bg-blue-400",
  },
];

function formatMs(ms: number): string {
  if (ms < 1) return "< 1 ms";
  if (ms < 1000) return `${Math.round(ms)} ms`;
  return `${(ms / 1000).toFixed(2)} s`;
}

export function TransactionTiming({ timing }: TransactionTimingProps) {
  const { t } = useLingui();

  const segments = useMemo(() => {
    if (!timing) return [];
    return TIMING_SEGMENTS.map((seg) => ({
      ...seg,
      value: timing[seg.key] as number | undefined,
    }));
  }, [timing]);

  if (!timing) {
    return (
      <div className="flex items-center justify-center text-muted-foreground text-sm py-8">
        <Trans>No timing data available</Trans>
      </div>
    );
  }

  const maxMs = timing.total_ms || 1;

  return (
    <div className="space-y-4">
      {/* Timing bar chart */}
      <div className="space-y-2">
        {segments.map((seg) => {
          const hasValue = seg.value != null;
          const widthPercent = hasValue ? Math.max((seg.value! / maxMs) * 100, 1) : 0;

          return (
            <div key={seg.key} className="grid grid-cols-[180px_1fr_80px] items-center gap-3">
              <div className="flex items-center gap-2 text-xs">
                <div className={`w-3 h-3 rounded-sm ${seg.bgClass} flex-shrink-0`} />
                <span className="text-muted-foreground">{seg.label}</span>
              </div>

              <div className="h-5 bg-muted/30 rounded relative overflow-hidden">
                {hasValue && (
                  <div
                    className="h-full rounded"
                    style={{
                      width: `${widthPercent}%`,
                      backgroundColor: seg.color,
                      minWidth: "2px",
                    }}
                  />
                )}
              </div>

              <div className="text-xs text-right font-mono text-muted-foreground">
                {hasValue ? formatMs(seg.value!) : "N/A"}
              </div>
            </div>
          );
        })}
      </div>

      {/* Total */}
      <div className="border-t border-border pt-3">
        <div className="grid grid-cols-[180px_1fr_80px] items-center gap-3">
          <div className="text-xs font-medium">{t`Total`}</div>
          <div />
          <div className="text-xs text-right font-mono font-medium">
            {formatMs(timing.total_ms)}
          </div>
        </div>
      </div>

      {/* Stacked bar visualization */}
      <div className="border-t border-border pt-3">
        <div className="text-xs text-muted-foreground mb-2">
          <Trans>Request Lifecycle</Trans>
        </div>
        <div className="h-6 flex rounded overflow-hidden bg-muted/30">
          {segments.map((seg) => {
            if (seg.value == null || seg.value === 0) return null;
            const widthPercent = (seg.value / maxMs) * 100;
            return (
              <div
                key={seg.key}
                className="h-full transition-all"
                style={{
                  width: `${widthPercent}%`,
                  backgroundColor: seg.color,
                  minWidth: "2px",
                }}
                title={`${seg.label}: ${formatMs(seg.value)}`}
              />
            );
          })}
        </div>
      </div>
    </div>
  );
}

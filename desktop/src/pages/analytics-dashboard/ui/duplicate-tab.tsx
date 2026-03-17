import { Trans } from "@lingui/react/macro";

import { Badge } from "@/shared/ui";
import type { DuplicateRequestGroup } from "../lib";

interface DuplicateTabProps {
  groups: DuplicateRequestGroup[];
}

export function DuplicateTab({ groups }: DuplicateTabProps) {
  return (
    <div className="mt-3">
      {groups.length === 0 ? (
        <div className="text-sm text-muted-foreground py-8 text-center">
          <Trans>No duplicate requests detected</Trans>
        </div>
      ) : (
        <div className="border rounded-lg overflow-hidden">
          <div className="grid grid-cols-[80px_1fr_60px_80px_160px_160px] gap-2 px-3 py-2 bg-muted text-xs font-medium text-muted-foreground border-b">
            <div>
              <Trans>Method</Trans>
            </div>
            <div>URL</div>
            <div className="text-right">
              <Trans>Count</Trans>
            </div>
            <div className="text-right">
              <Trans>Window</Trans>
            </div>
            <div className="text-right">
              <Trans>First Seen</Trans>
            </div>
            <div className="text-right">
              <Trans>Last Seen</Trans>
            </div>
          </div>
          <div className="max-h-[400px] overflow-auto">
            {groups.map((group, i) => (
              <div
                key={`${group.method}-${group.url}-${i}`}
                className="grid grid-cols-[80px_1fr_60px_80px_160px_160px] gap-2 px-3 py-2 text-xs border-b last:border-b-0 hover:bg-muted/50"
              >
                <div>
                  <Badge variant="outline" className="text-[10px] px-1.5">
                    {group.method}
                  </Badge>
                </div>
                <div className="truncate" title={group.url}>
                  {group.url}
                </div>
                <div className="text-right font-mono">
                  <Badge variant="destructive" className="text-[10px] px-1.5">
                    {group.count}x
                  </Badge>
                </div>
                <div className="text-right font-mono text-muted-foreground">
                  {group.windowMs} ms
                </div>
                <div className="text-right text-muted-foreground">
                  {new Date(group.firstSeen).toLocaleTimeString()}
                </div>
                <div className="text-right text-muted-foreground">
                  {new Date(group.lastSeen).toLocaleTimeString()}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

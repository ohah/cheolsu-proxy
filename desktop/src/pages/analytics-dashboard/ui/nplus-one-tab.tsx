import { Trans } from "@lingui/react/macro";

import { Badge } from "@/shared/ui";
import type { NPlusOnePattern } from "../lib";

interface NPlusOneTabProps {
  patterns: NPlusOnePattern[];
}

export function NPlusOneTab({ patterns }: NPlusOneTabProps) {
  return (
    <div className="mt-3">
      {patterns.length === 0 ? (
        <div className="text-sm text-muted-foreground py-8 text-center">
          <Trans>No N+1 patterns detected</Trans>
        </div>
      ) : (
        <div className="space-y-3">
          {patterns.map((pattern, i) => (
            <div
              key={`${pattern.baseUrl}-${i}`}
              className="border rounded-lg p-3 space-y-2 hover:bg-muted/30"
            >
              <div className="flex items-center gap-2">
                <Badge variant="destructive" className="text-[10px]">
                  N+1
                </Badge>
                <span className="text-sm font-mono font-medium">{pattern.pattern}</span>
                <Badge variant="outline" className="ml-auto">
                  {pattern.count} <Trans>requests</Trans>
                </Badge>
              </div>
              <div className="text-xs text-muted-foreground">
                <Trans>Base URL</Trans>: <span className="font-mono">{pattern.baseUrl}</span>
              </div>
              <div className="text-xs text-amber-600 dark:text-amber-400 bg-amber-50 dark:bg-amber-950/30 rounded px-2 py-1">
                {pattern.suggestion}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

import { memo, useMemo } from "react";
import { AlertTriangle } from "lucide-react";

import { Badge } from "@/shared/ui";
import { getStatusColor } from "@/entities/transaction";

import type { TableCellProps } from "../../model";

export const StatusCell = memo<TableCellProps>(({ data }) => {
  const method = data.transaction.request?.method;
  const status = data.transaction.response?.status || 0;
  const isConnect = method === "CONNECT";

  const { hasErrors, hasWarnings, count } = useMemo(() => {
    const violations = data.transaction.validations?.flatMap((r) => r.violations) ?? [];
    return {
      hasErrors: violations.some((v) => v.severity === "Error"),
      hasWarnings: violations.length > 0,
      count: violations.length,
    };
  }, [data.transaction.validations]);

  return (
    <div className="flex items-center gap-1">
      <Badge variant="outline" className={`text-xs ${getStatusColor(status)}`}>
        {isConnect ? "TUNNEL" : status}
      </Badge>
      {hasWarnings && (
        <span title={`${count} contract violation(s)`}>
          <AlertTriangle
            className={`h-3.5 w-3.5 shrink-0 ${hasErrors ? "text-destructive" : "text-yellow-500"}`}
          />
        </span>
      )}
    </div>
  );
});

StatusCell.displayName = "StatusCell";

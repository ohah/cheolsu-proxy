import { useMemo } from "react";
import { useLingui } from "@lingui/react/macro";

import { Badge } from "@/shared/ui";

interface NetworkStatsProps {
  totalCount: number;
  filteredCount: number;
}

export const NetworkStats = ({ totalCount, filteredCount }: NetworkStatsProps) => {
  const { t } = useLingui();

  const count = useMemo(() => {
    if (totalCount !== filteredCount) {
      return t`${filteredCount} of ${totalCount} transactions`;
    }
    return t`${totalCount} transactions`;
  }, [totalCount, filteredCount, t]);

  return (
    <div className="flex items-center gap-2">
      <Badge variant="secondary" className="text-xs">
        {count}
      </Badge>
    </div>
  );
};

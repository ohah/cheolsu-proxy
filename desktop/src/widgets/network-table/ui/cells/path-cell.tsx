import { memo } from "react";

import type { TableCellProps } from "../../model";

export const PathCell = memo<TableCellProps>(({ data }) => {
  const { authority, pathname, transaction } = data;
  const fallback = transaction.tls_fallback_used;

  return (
    <div className="flex flex-col gap-1 min-w-0">
      <div className="flex items-center gap-1 min-w-0">
        <span className="font-mono text-sm truncate" title={authority}>
          {authority}
        </span>
        {fallback && (
          <span
            className="shrink-0 rounded bg-amber-500/15 px-1 py-px text-[10px] font-semibold leading-none text-amber-600 dark:text-amber-400"
            title="TLS fallback — learned strategy used"
          >
            FB
          </span>
        )}
      </div>
      <div className="font-mono text-sm truncate text-muted-foreground" title={pathname}>
        {pathname}
      </div>
    </div>
  );
});

PathCell.displayName = "PathCell";

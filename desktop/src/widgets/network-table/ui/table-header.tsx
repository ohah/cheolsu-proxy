import { Check } from "lucide-react";

import { TABLE_COLUMNS, HEADER_CLASSES, buildGridTemplate, type ColumnKey } from "../model";
import { ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuTrigger } from "@/shared/ui";

interface TableHeaderProps {
  allChecked: boolean;
  someChecked: boolean;
  onToggleAll: () => void;
  visibleColumns: Set<ColumnKey>;
  onToggleColumn: (key: ColumnKey) => void;
}

export const TableHeader = ({
  allChecked,
  someChecked,
  onToggleAll,
  visibleColumns,
  onToggleColumn,
}: TableHeaderProps) => {
  const gridTemplate = buildGridTemplate(visibleColumns);

  return (
    <div className="border-b border-border bg-background">
      <ContextMenu>
        <ContextMenuTrigger>
          <div
            className={`grid gap-4 p-3.5 ${HEADER_CLASSES}`}
            style={{ gridTemplateColumns: gridTemplate }}
          >
            <div className="flex items-center justify-center w-5">
              <input
                type="checkbox"
                checked={allChecked}
                ref={(el) => {
                  if (el) el.indeterminate = someChecked && !allChecked;
                }}
                onChange={onToggleAll}
                className="cursor-pointer accent-primary"
              />
            </div>
            {TABLE_COLUMNS.filter((col) => visibleColumns.has(col.key)).map((column) => (
              <div key={column.key}>{column.label}</div>
            ))}
          </div>
        </ContextMenuTrigger>
        <ContextMenuContent>
          {TABLE_COLUMNS.map((column) => (
            <ContextMenuItem
              key={column.key}
              onClick={() => onToggleColumn(column.key)}
              disabled={visibleColumns.has(column.key) && visibleColumns.size <= 1}
            >
              <div className="w-4 h-4 flex items-center justify-center flex-shrink-0">
                {visibleColumns.has(column.key) && <Check className="w-3.5 h-3.5" />}
              </div>
              {column.label}
            </ContextMenuItem>
          ))}
        </ContextMenuContent>
      </ContextMenu>
    </div>
  );
};

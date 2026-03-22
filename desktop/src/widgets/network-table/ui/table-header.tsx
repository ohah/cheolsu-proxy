import { Check, ArrowUp, ArrowDown } from "lucide-react";
import { flexRender, type Table } from "@tanstack/react-table";

import { TABLE_COLUMNS, HEADER_CLASSES, buildGridTemplate, type ColumnKey } from "../model";
import type { TableRowData } from "../model";
import { ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuTrigger } from "@/shared/ui";

interface TableHeaderProps {
  allChecked: boolean;
  someChecked: boolean;
  onToggleAll: () => void;
  visibleColumns: Set<ColumnKey>;
  onToggleColumn: (key: ColumnKey) => void;
  table: Table<TableRowData>;
}

export const TableHeader = ({
  allChecked,
  someChecked,
  onToggleAll,
  visibleColumns,
  onToggleColumn,
  table,
}: TableHeaderProps) => {
  const headerGroups = table.getHeaderGroups();
  const headers = headerGroups[0]?.headers ?? [];
  const gridTemplate = buildGridTemplate(visibleColumns);

  return (
    <div className="border-b border-border bg-background">
      <ContextMenu>
        <ContextMenuTrigger>
          <div
            className={`grid items-center gap-4 px-3 py-3.5 border-l-4 border-l-transparent ${HEADER_CLASSES}`}
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
            {headers
              .filter((header) => header.column.getIsVisible())
              .map((header) => {
                const canSort = header.column.getCanSort();
                const sorted = header.column.getIsSorted();

                return (
                  <div
                    key={header.id}
                    className={
                      canSort
                        ? "flex items-center gap-1 cursor-pointer select-none hover:text-foreground transition-colors"
                        : ""
                    }
                    onClick={canSort ? header.column.getToggleSortingHandler() : undefined}
                  >
                    {flexRender(header.column.columnDef.header, header.getContext())}
                    {sorted === "asc" && <ArrowUp className="w-3 h-3" />}
                    {sorted === "desc" && <ArrowDown className="w-3 h-3" />}
                  </div>
                );
              })}
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

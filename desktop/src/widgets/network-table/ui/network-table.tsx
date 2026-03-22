import { useMemo, useCallback, useState } from "react";
import {
  useReactTable,
  getCoreRowModel,
  getSortedRowModel,
  type SortingState,
  type VisibilityState,
} from "@tanstack/react-table";

import type { HttpTransaction } from "@/entities/proxy";

import { TableHeader } from "./table-header";
import { TableBody } from "./table-body";
import { useTableData } from "../hooks";
import { type ColumnKey, columns } from "../model";
import { useAppSettingsStore } from "@/shared/stores/app-settings-store";
import { Pin } from "lucide-react";

interface NetworkTableProps {
  transactions: HttpTransaction[];
  selectedTransaction: HttpTransaction | null;
  pinnedTransactionIds: Set<string>;
  checkedTransactionIds: Set<string>;
  createTransactionSelectHandler: (transaction: HttpTransaction) => () => void;
  createTransactionDeleteHandler: (id: string) => () => void;
  createTransactionPinHandler: (id: string) => () => void;
  createTransactionCheckHandler: (id: string) => () => void;
  onAdvancedRepeat?: (transaction: HttpTransaction) => void;
  onToggleCheckAll: () => void;
}

export const NetworkTable = ({
  transactions,
  selectedTransaction,
  pinnedTransactionIds,
  checkedTransactionIds,
  createTransactionSelectHandler,
  createTransactionDeleteHandler,
  createTransactionPinHandler,
  createTransactionCheckHandler,
  onAdvancedRepeat,
  onToggleCheckAll,
}: NetworkTableProps) => {
  const storedColumns = useAppSettingsStore((s) => s.visibleColumns);
  const setStoredColumns = useAppSettingsStore((s) => s.setVisibleColumns);

  const [sorting, setSorting] = useState<SortingState>([]);

  // ColumnKey[] → TanStack VisibilityState 변환
  const columnVisibility = useMemo<VisibilityState>(() => {
    const allKeys: ColumnKey[] = [
      "path",
      "method",
      "status",
      "size",
      "time",
      "client",
      "waterfall",
    ];
    const visible = new Set(storedColumns as ColumnKey[]);
    const vis: VisibilityState = {};
    for (const key of allKeys) {
      vis[key] = visible.has(key);
    }
    return vis;
  }, [storedColumns]);

  const visibleColumns = useMemo(() => new Set(storedColumns as ColumnKey[]), [storedColumns]);

  const handleToggleColumn = useCallback(
    (key: ColumnKey) => {
      const next = new Set(visibleColumns);
      if (next.has(key)) {
        if (next.size <= 1) return;
        next.delete(key);
      } else {
        next.add(key);
      }
      setStoredColumns([...next]);
    },
    [visibleColumns, setStoredColumns],
  );

  const { pinnedTransactions, unpinnedTransactions } = useMemo(() => {
    const pinned: HttpTransaction[] = [];
    const unpinned: HttpTransaction[] = [];

    transactions.forEach((transaction) => {
      const id = transaction.request?.id;
      if (id !== undefined && pinnedTransactionIds.has(id)) {
        pinned.push(transaction);
      } else {
        unpinned.push(transaction);
      }
    });

    return { pinnedTransactions: pinned, unpinnedTransactions: unpinned };
  }, [transactions, pinnedTransactionIds]);

  const { tableData: pinnedTableData } = useTableData({
    transactions: pinnedTransactions,
    selectedTransaction,
  });

  const { tableData: unpinnedTableData } = useTableData({
    transactions: unpinnedTransactions,
    selectedTransaction,
  });

  // TanStack Table 인스턴스 (unpinned 데이터에 대해 정렬 적용)
  const table = useReactTable({
    data: unpinnedTableData,
    columns,
    state: {
      sorting,
      columnVisibility,
    },
    onSortingChange: setSorting,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
  });

  // 정렬된 행 데이터 추출
  const sortedUnpinnedData = useMemo(
    () => table.getRowModel().rows.map((row) => row.original),
    [table.getRowModel()],
  );

  const allIds = useMemo(
    () => transactions.map((t) => t.request?.id).filter((id): id is string => !!id),
    [transactions],
  );

  const allChecked = allIds.length > 0 && allIds.every((id) => checkedTransactionIds.has(id));
  const someChecked = allIds.some((id) => checkedTransactionIds.has(id));

  return (
    <div className="flex flex-col flex-1 h-full overflow-hidden">
      <TableHeader
        allChecked={allChecked}
        someChecked={someChecked}
        onToggleAll={onToggleCheckAll}
        visibleColumns={visibleColumns}
        onToggleColumn={handleToggleColumn}
        table={table}
      />
      <div className="flex-1 flex flex-col overflow-hidden">
        {pinnedTableData.length > 0 && (
          <div className="border-b-2 border-border bg-gradient-to-b from-muted/80 to-muted/40 shadow-sm">
            <div className="px-4 py-2 bg-muted/60 border-b border-border flex items-center gap-2 text-xs font-semibold text-muted-foreground uppercase tracking-wide">
              <Pin className="w-4 h-4 fill-muted-foreground" />
              Pinned Transactions ({pinnedTableData.length})
            </div>
            <TableBody
              data={pinnedTableData}
              pinnedTransactionIds={pinnedTransactionIds}
              checkedTransactionIds={checkedTransactionIds}
              createTransactionSelectHandler={createTransactionSelectHandler}
              createTransactionDeleteHandler={createTransactionDeleteHandler}
              createTransactionPinHandler={createTransactionPinHandler}
              createTransactionCheckHandler={createTransactionCheckHandler}
              onAdvancedRepeat={onAdvancedRepeat}
              isPinnedSection
              visibleColumns={visibleColumns}
            />
          </div>
        )}
        <TableBody
          data={sortedUnpinnedData}
          pinnedTransactionIds={pinnedTransactionIds}
          checkedTransactionIds={checkedTransactionIds}
          createTransactionSelectHandler={createTransactionSelectHandler}
          createTransactionDeleteHandler={createTransactionDeleteHandler}
          createTransactionPinHandler={createTransactionPinHandler}
          createTransactionCheckHandler={createTransactionCheckHandler}
          onAdvancedRepeat={onAdvancedRepeat}
          isPinnedSection={false}
          visibleColumns={visibleColumns}
        />
      </div>
    </div>
  );
};

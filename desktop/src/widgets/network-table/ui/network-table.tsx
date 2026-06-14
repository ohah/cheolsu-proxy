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
import { useTableData, computeTimelineRange } from "../hooks";
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

  const visibleColumns = useMemo(() => new Set(storedColumns as ColumnKey[]), [storedColumns]);

  // TanStack VisibilityState를 visibleColumns Set에서 파생
  const columnVisibility = useMemo<VisibilityState>(() => {
    const vis: VisibilityState = {};
    for (const col of columns) {
      vis[col.id!] = visibleColumns.has(col.id as ColumnKey);
    }
    return vis;
  }, [visibleColumns]);

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

  // pinned/unpinned가 동일한 Waterfall 시간축을 쓰도록 전체 트랜잭션 기준으로 한 번만 계산해 공유한다.
  const timelineRange = useMemo(() => computeTimelineRange(transactions), [transactions]);

  const { tableData: pinnedTableData } = useTableData({
    transactions: pinnedTransactions,
    selectedTransaction,
    timelineRange,
  });

  const { tableData: unpinnedTableData } = useTableData({
    transactions: unpinnedTransactions,
    selectedTransaction,
    timelineRange,
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
  const sortedUnpinnedData = table.getRowModel().rows.map((row) => row.original);

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

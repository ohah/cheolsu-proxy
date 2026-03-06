import { useCallback, useMemo } from "react";

import type { HttpTransaction } from "@/entities/proxy";

import { TableHeader } from "./table-header";
import { TableBody } from "./table-body";
import { useTableData } from "../hooks";
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
  onToggleCheckAll,
}: NetworkTableProps) => {
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
      />
      <div className="flex-1 flex flex-col overflow-hidden">
        {pinnedTableData.length > 0 && (
          <div className="border-b-2 border-slate-300 bg-gradient-to-b from-slate-100/80 to-slate-50/40 shadow-sm">
            <div className="px-4 py-2 bg-slate-200/60 border-b border-slate-300 flex items-center gap-2 text-xs font-semibold text-slate-700 uppercase tracking-wide">
              <Pin className="w-4 h-4 fill-slate-600" />
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
              isPinnedSection
            />
          </div>
        )}
        <TableBody
          data={unpinnedTableData}
          pinnedTransactionIds={pinnedTransactionIds}
          checkedTransactionIds={checkedTransactionIds}
          createTransactionSelectHandler={createTransactionSelectHandler}
          createTransactionDeleteHandler={createTransactionDeleteHandler}
          createTransactionPinHandler={createTransactionPinHandler}
          createTransactionCheckHandler={createTransactionCheckHandler}
          isPinnedSection={false}
        />
      </div>
    </div>
  );
};

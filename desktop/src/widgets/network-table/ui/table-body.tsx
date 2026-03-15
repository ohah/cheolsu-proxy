import { useCallback, useMemo } from "react";
import { Trans } from "@lingui/react/macro";

import type { HttpTransaction } from "@/entities/proxy";
import { VirtualizedScrollArea } from "@/shared/ui";

import type { TableRowData } from "../model";
import type { ColumnKey } from "../model";

import { TableRow } from "./table-row";

interface TableBodyProps {
  data: TableRowData[];
  pinnedTransactionIds: Set<string>;
  checkedTransactionIds: Set<string>;
  createTransactionSelectHandler: (request: HttpTransaction) => () => void;
  createTransactionDeleteHandler: (id: string) => () => void;
  createTransactionPinHandler: (id: string) => () => void;
  createTransactionCheckHandler: (id: string) => () => void;
  onAdvancedRepeat?: (transaction: HttpTransaction) => void;
  isPinnedSection: boolean;
  visibleColumns: Set<ColumnKey>;
}

export const TableBody = ({
  data,
  pinnedTransactionIds,
  checkedTransactionIds,
  createTransactionSelectHandler,
  createTransactionDeleteHandler,
  createTransactionPinHandler,
  createTransactionCheckHandler,
  onAdvancedRepeat,
  isPinnedSection,
  visibleColumns,
}: TableBodyProps) => {
  const rowHandlers = useMemo(() => {
    return data.map((rowData) => {
      const id = rowData.transaction.request?.id ?? "";
      return {
        onSelect: createTransactionSelectHandler(rowData.transaction),
        onDelete: createTransactionDeleteHandler(id),
        onPin: createTransactionPinHandler(id),
        onCheck: createTransactionCheckHandler(id),
        isPinned: pinnedTransactionIds.has(id),
        isChecked: checkedTransactionIds.has(id),
      };
    });
  }, [
    data,
    createTransactionSelectHandler,
    createTransactionDeleteHandler,
    createTransactionPinHandler,
    createTransactionCheckHandler,
    pinnedTransactionIds,
    checkedTransactionIds,
  ]);

  const renderItem = useCallback(
    (index: number) => {
      const rowData = data[index];
      const handlers = rowHandlers[index];

      return (
        <TableRow
          data={rowData}
          onSelect={handlers.onSelect}
          onDelete={handlers.onDelete}
          onPin={handlers.onPin}
          isPinned={handlers.isPinned}
          isChecked={handlers.isChecked}
          onCheck={handlers.onCheck}
          onAdvancedRepeat={onAdvancedRepeat}
          visibleColumns={visibleColumns}
        />
      );
    },
    [data, rowHandlers],
  );

  if (data.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center p-8">
        <p className="text-muted-foreground">
          <Trans>No transactions to display</Trans>
        </p>
      </div>
    );
  }

  if (isPinnedSection) {
    return (
      <div className="overflow-hidden">
        {data.map((_, index) => (
          <div key={data[index].transaction.request?.id ?? index}>{renderItem(index)}</div>
        ))}
      </div>
    );
  }

  return (
    <VirtualizedScrollArea
      itemCount={data.length}
      itemSize={69}
      className="flex-1 min-h-0"
      renderItem={renderItem}
      overscan={30}
    />
  );
};

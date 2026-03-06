import { memo, useCallback, useMemo } from "react";

import { PathCell, MethodCell, StatusCell, SizeCell, TimeCell } from "./cells";

import {
  ROW_BASE_CLASSES,
  SELECTED_ROW_CLASSES,
  GRID_COLS_CLASS,
  PINNED_ROW_CLASSES,
} from "../model";
import type { TableRowData } from "../model";
import { ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuTrigger } from "@/shared/ui";
import { generateCurlCommand } from "@/features/transaction-details";
import { toast } from "sonner";
import { Code, Pin, PinOff, Trash2 } from "lucide-react";

interface TableRowProps {
  data: TableRowData;
  onSelect: () => void;
  onDelete: () => void;
  onPin: () => void;
  isPinned: boolean;
  isChecked: boolean;
  onCheck: () => void;
}

export const TableRow = memo(function TableRow({
  data,
  onSelect,
  onDelete,
  onPin,
  isPinned,
  isChecked,
  onCheck,
}: TableRowProps) {
  const { isSelected } = data;

  const rowClasses = useMemo(() => {
    let classes = `${ROW_BASE_CLASSES} ${GRID_COLS_CLASS}`;

    if (isSelected && !isPinned) {
      classes += ` ${SELECTED_ROW_CLASSES}`;
    }
    if (isSelected && isPinned) {
      classes += ` ${PINNED_ROW_CLASSES}`;
    }
    return classes;
  }, [isSelected, isPinned]);

  const handleClickCopyCurlCommand = useCallback(() => {
    const curlCommand = generateCurlCommand(data.transaction);
    navigator.clipboard.writeText(curlCommand);
    toast.success("Curl command copied to clipboard");
  }, [data]);

  const handleClickDeleteTransaction = useCallback(() => {
    onDelete();
    toast.success("Transaction deleted");
  }, [onDelete]);

  const handleClickPinTransaction = useCallback(() => {
    onPin();
    toast.success(isPinned ? "Transaction unpinned" : "Transaction pinned to top");
  }, [onPin, isPinned]);

  const handleCheckboxClick = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      onCheck();
    },
    [onCheck],
  );

  return (
    <ContextMenu>
      <ContextMenuTrigger>
        <div className={rowClasses} onClick={onSelect}>
          <div className="flex items-center justify-center w-5" onClick={handleCheckboxClick}>
            <input
              type="checkbox"
              checked={isChecked}
              onChange={() => {}}
              className="cursor-pointer accent-primary"
            />
          </div>
          <PathCell data={data} />
          <MethodCell data={data} />
          <StatusCell data={data} />
          <SizeCell data={data} />
          <TimeCell data={data} />
        </div>
      </ContextMenuTrigger>
      <ContextMenuContent className="w-3xs">
        <ContextMenuItem onClick={handleClickPinTransaction}>
          {isPinned ? <PinOff /> : <Pin />}
          {isPinned ? "Unpin from Top" : "Pin to Top"}
        </ContextMenuItem>
        <ContextMenuItem onClick={handleClickCopyCurlCommand}>
          <Code />
          Copy Curl Command
        </ContextMenuItem>
        <ContextMenuItem onClick={handleClickDeleteTransaction}>
          <Trash2 />
          Delete Transaction
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  );
});

TableRow.displayName = "TableRow";

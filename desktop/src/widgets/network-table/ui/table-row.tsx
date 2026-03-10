import { memo, useCallback, useMemo } from "react";

import { PathCell, MethodCell, StatusCell, SizeCell, TimeCell } from "./cells";

import {
  ROW_BASE_CLASSES,
  SELECTED_ROW_CLASSES,
  GRID_COLS_CLASS,
  PINNED_ROW_CLASSES,
} from "../model";
import type { TableRowData } from "../model";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuSub,
  ContextMenuSubContent,
  ContextMenuSubTrigger,
  ContextMenuTrigger,
} from "@/shared/ui";
import {
  generateCurlCommand,
  generateFetchCommand,
  generateHttpieCommand,
  generatePythonRequestsCommand,
} from "@/shared/lib";
import { useInterceptRuleDialogStore } from "@/shared/stores";
import { toast } from "sonner";
import { Code, Pin, PinOff, Repeat, Shield, Trash2 } from "lucide-react";
import { Trans, useLingui } from "@lingui/react/macro";
import type { HttpTransaction } from "@/entities/proxy";

interface TableRowProps {
  data: TableRowData;
  onSelect: () => void;
  onDelete: () => void;
  onPin: () => void;
  isPinned: boolean;
  isChecked: boolean;
  onCheck: () => void;
  onAdvancedRepeat?: (transaction: HttpTransaction) => void;
}

export const TableRow = memo(function TableRow({
  data,
  onSelect,
  onDelete,
  onPin,
  isPinned,
  isChecked,
  onCheck,
  onAdvancedRepeat,
}: TableRowProps) {
  const { isSelected } = data;
  const { t } = useLingui();

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

  const handleCopyAs = useCallback(
    (generator: (t: HttpTransaction) => string, label: string) => {
      const code = generator(data.transaction);
      navigator.clipboard.writeText(code);
      toast.success(t`${label} copied to clipboard`);
    },
    [data],
  );

  const handleClickDeleteTransaction = useCallback(() => {
    onDelete();
    toast.success(t`Transaction deleted`);
  }, [onDelete]);

  const openInterceptRuleDialog = useInterceptRuleDialogStore((s) => s.openWithValues);

  const handleClickAddInterceptRule = useCallback(() => {
    const request = data.transaction.request;
    if (!request) return;
    openInterceptRuleDialog({
      pattern: request.uri,
      method: request.method,
    });
  }, [data, openInterceptRuleDialog]);

  const handleClickPinTransaction = useCallback(() => {
    onPin();
    toast.success(isPinned ? t`Transaction unpinned` : t`Transaction pinned to top`);
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
          {isPinned ? <Trans>Unpin from Top</Trans> : <Trans>Pin to Top</Trans>}
        </ContextMenuItem>
        <ContextMenuSub>
          <ContextMenuSubTrigger>
            <Code />
            <Trans>Copy as...</Trans>
          </ContextMenuSubTrigger>
          <ContextMenuSubContent>
            <ContextMenuItem onClick={() => handleCopyAs(generateCurlCommand, "cURL")}>
              <Trans>cURL</Trans>
            </ContextMenuItem>
            <ContextMenuItem onClick={() => handleCopyAs(generateFetchCommand, "fetch")}>
              <Trans>JavaScript Fetch</Trans>
            </ContextMenuItem>
            <ContextMenuItem onClick={() => handleCopyAs(generateHttpieCommand, "HTTPie")}>
              <Trans>HTTPie</Trans>
            </ContextMenuItem>
            <ContextMenuItem
              onClick={() => handleCopyAs(generatePythonRequestsCommand, "Python requests")}
            >
              <Trans>Python Requests</Trans>
            </ContextMenuItem>
          </ContextMenuSubContent>
        </ContextMenuSub>
        <ContextMenuItem onClick={() => onAdvancedRepeat?.(data.transaction)}>
          <Repeat />
          <Trans>Advanced Repeat</Trans>
        </ContextMenuItem>
        <ContextMenuItem onClick={handleClickDeleteTransaction}>
          <Trash2 />
          <Trans>Delete Transaction</Trans>
        </ContextMenuItem>
        <ContextMenuSeparator />
        <ContextMenuItem onClick={handleClickAddInterceptRule}>
          <Shield />
          <Trans>Add Intercept Rule</Trans>
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  );
});

TableRow.displayName = "TableRow";

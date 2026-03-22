import { memo, useCallback, useMemo } from "react";

import {
  PathCell,
  MethodCell,
  StatusCell,
  SizeCell,
  TimeCell,
  TimestampCell,
  ClientCell,
  WaterfallCell,
} from "./cells";

import {
  TABLE_COLUMNS,
  ROW_BASE_CLASSES,
  SELECTED_ROW_CLASSES,
  PINNED_ROW_CLASSES,
  buildGridTemplate,
  type ColumnKey,
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
  generatePowerShellCommand,
  generateNodejsFetchCommand,
  generateVitestCode,
  generatePlaywrightCode,
  generateK6Code,
  transactionToReplayEntry,
} from "@/shared/lib";
import {
  useInterceptRuleDialogStore,
  useMapRuleDialogStore,
  useHostMappingDialogStore,
  useServerReplayStore,
} from "@/shared/stores";
import { writeText as clipboardWriteText } from "@tauri-apps/plugin-clipboard-manager";
import { toast } from "sonner";
import {
  Code,
  Pin,
  PinOff,
  Repeat,
  Shield,
  Trash2,
  GitBranch,
  FileDown,
  Globe,
  Database,
} from "lucide-react";
import { Trans, useLingui } from "@lingui/react/macro";
import type { HttpTransaction } from "@/entities/proxy";

const CELL_MAP: Record<ColumnKey, React.ComponentType<{ data: TableRowData }>> = {
  path: PathCell,
  method: MethodCell,
  status: StatusCell,
  size: SizeCell,
  time: TimeCell,
  timestamp: TimestampCell,
  client: ClientCell,
  waterfall: WaterfallCell,
};

interface TableRowProps {
  data: TableRowData;
  onSelect: () => void;
  onDelete: () => void;
  onPin: () => void;
  isPinned: boolean;
  isChecked: boolean;
  onCheck: () => void;
  onAdvancedRepeat?: (transaction: HttpTransaction) => void;
  visibleColumns: Set<ColumnKey>;
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
  visibleColumns,
}: TableRowProps) {
  const { isSelected } = data;
  const { t } = useLingui();

  const gridTemplate = buildGridTemplate(visibleColumns);

  const rowClasses = useMemo(() => {
    let classes = ROW_BASE_CLASSES;

    if (isSelected && !isPinned) {
      classes += ` ${SELECTED_ROW_CLASSES}`;
    }
    if (isSelected && isPinned) {
      classes += ` ${PINNED_ROW_CLASSES}`;
    }
    return classes;
  }, [isSelected, isPinned]);

  const handleCopyAs = useCallback(
    async (generator: (t: HttpTransaction) => string, label: string) => {
      try {
        const code = generator(data.transaction);
        await clipboardWriteText(code);
        toast.success(t`${label} copied to clipboard`);
      } catch (error) {
        toast.error(
          t`Failed to copy ${label}: ${error instanceof Error ? error.message : String(error)}`,
        );
      }
    },
    [data],
  );

  const handleClickDeleteTransaction = useCallback(() => {
    onDelete();
    toast.success(t`Transaction deleted`);
  }, [onDelete]);

  const openInterceptRuleDialog = useInterceptRuleDialogStore((s) => s.openWithValues);
  const openMapRuleDialog = useMapRuleDialogStore((s) => s.openWithValues);
  const openHostMappingDialog = useHostMappingDialogStore((s) => s.openWithValues);
  const addServerReplayEntry = useServerReplayStore((s) => s.addEntry);

  const handleClickAddInterceptRule = useCallback(() => {
    const request = data.transaction.request;
    if (!request) return;
    openInterceptRuleDialog({
      pattern: request.uri,
      method: request.method,
    });
  }, [data, openInterceptRuleDialog]);

  const handleClickAddMapRule = useCallback(
    (mapType: "map_remote" | "map_local") => {
      const request = data.transaction.request;
      if (!request) return;
      openMapRuleDialog({ pattern: request.uri, method: request.method, mapType });
    },
    [data, openMapRuleDialog],
  );

  const handleClickAddHostMapping = useCallback(() => {
    const request = data.transaction.request;
    if (!request) return;
    try {
      const url = new URL(request.uri);
      openHostMappingDialog({
        sourceHost: url.hostname,
        sourcePort: url.port || "",
      });
    } catch {
      openHostMappingDialog({ sourceHost: request.uri, sourcePort: "" });
    }
  }, [data, openHostMappingDialog]);

  const handleClickAddServerReplay = useCallback(() => {
    const entry = transactionToReplayEntry(data.transaction);
    if (!entry) {
      toast.error(t`Cannot add: no response data`);
      return;
    }
    addServerReplayEntry(entry);
    toast.success(t`Added to server replay`);
  }, [data, addServerReplayEntry]);

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

  const visibleColumnKeys = useMemo(
    () => TABLE_COLUMNS.filter((col) => visibleColumns.has(col.key)).map((col) => col.key),
    [visibleColumns],
  );

  return (
    <ContextMenu>
      <ContextMenuTrigger>
        <div
          className={rowClasses}
          style={{ gridTemplateColumns: gridTemplate }}
          onClick={onSelect}
        >
          <div className="flex items-center justify-center w-5" onClick={handleCheckboxClick}>
            <input
              type="checkbox"
              checked={isChecked}
              onChange={() => {}}
              className="cursor-pointer accent-primary"
            />
          </div>
          {visibleColumnKeys.map((key) => {
            const Cell = CELL_MAP[key];
            return <Cell key={key} data={data} />;
          })}
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
            <ContextMenuItem
              onClick={() => handleCopyAs(generateNodejsFetchCommand, "Node.js fetch")}
            >
              <Trans>Node.js Fetch</Trans>
            </ContextMenuItem>
            <ContextMenuItem onClick={() => handleCopyAs(generatePowerShellCommand, "PowerShell")}>
              <Trans>PowerShell</Trans>
            </ContextMenuItem>
            <ContextMenuItem onClick={() => handleCopyAs(generateHttpieCommand, "HTTPie")}>
              <Trans>HTTPie</Trans>
            </ContextMenuItem>
            <ContextMenuItem
              onClick={() => handleCopyAs(generatePythonRequestsCommand, "Python requests")}
            >
              <Trans>Python Requests</Trans>
            </ContextMenuItem>
            <ContextMenuSeparator />
            <ContextMenuItem onClick={() => handleCopyAs(generateVitestCode, "Vitest")}>
              <Trans>Vitest/Jest Test</Trans>
            </ContextMenuItem>
            <ContextMenuItem onClick={() => handleCopyAs(generatePlaywrightCode, "Playwright")}>
              <Trans>Playwright Test</Trans>
            </ContextMenuItem>
            <ContextMenuItem onClick={() => handleCopyAs(generateK6Code, "k6")}>
              <Trans>k6 Load Test</Trans>
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
        <ContextMenuSub>
          <ContextMenuSubTrigger>
            <Shield />
            <Trans>Add Rule...</Trans>
          </ContextMenuSubTrigger>
          <ContextMenuSubContent>
            <ContextMenuItem onClick={handleClickAddInterceptRule}>
              <Shield />
              <Trans>Intercept Rule</Trans>
            </ContextMenuItem>
            <ContextMenuItem onClick={() => handleClickAddMapRule("map_remote")}>
              <GitBranch />
              <Trans>Map Remote</Trans>
            </ContextMenuItem>
            <ContextMenuItem onClick={() => handleClickAddMapRule("map_local")}>
              <FileDown />
              <Trans>Map Local</Trans>
            </ContextMenuItem>
            <ContextMenuItem onClick={handleClickAddHostMapping}>
              <Globe />
              <Trans>Host Mapping</Trans>
            </ContextMenuItem>
            <ContextMenuSeparator />
            <ContextMenuItem
              onClick={handleClickAddServerReplay}
              disabled={!data.transaction.response}
            >
              <Database />
              <Trans>Server Replay</Trans>
            </ContextMenuItem>
          </ContextMenuSubContent>
        </ContextMenuSub>
      </ContextMenuContent>
    </ContextMenu>
  );
});

TableRow.displayName = "TableRow";

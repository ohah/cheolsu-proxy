import { useCallback, useMemo } from "react";

import {
  TransactionDetails,
  SequenceReplayDialog,
  ReplayDialog,
  AdvancedRepeatDialog,
} from "@/features/transaction-details";
import { QueryFilterEditor, QueryBuilder } from "@/features/query-filter-editor";
import { RuleFormDialog } from "@/features/intercept-rule-form";
import { DiffView } from "@/features/traffic-diff";

import { NetworkHeader } from "@/widgets/network-header";
import { NetworkTable } from "@/widgets/network-table";
import { HostPathTree } from "@/widgets/host-path-tree";

import { ResizableHandle, ResizablePanel, ResizablePanelGroup, Button } from "@/shared/ui";
import { useDefaultLayout } from "react-resizable-panels";
import { Play, X, GitCompareArrows } from "lucide-react";

import {
  useTransactionFilters,
  useResizablePanelController,
  useNetworkDialogs,
  useNetworkExport,
  useNetworkDiff,
  useFilterEditor,
} from "../hooks";
import { useInterceptRuleDialogStore, useAppSettingsStore } from "@/shared/stores";
import {
  useTransactionData,
  useTransactionActions,
  useTransactionSelection,
} from "@/shared/hooks/use-transaction-selectors";

export const NetworkDashboard = () => {
  const { transactions, selectedTransaction, pinnedTransactionIds, checkedTransactionIds, paused } =
    useTransactionData();

  const { clearTransactions, deleteTransaction, setTransactions, appendTransactions, togglePause } =
    useTransactionActions();

  const {
    toggleSelectedTransaction,
    setSelectedTransaction,
    clearSelectedTransaction,
    togglePinTransaction,
    toggleCheckTransaction,
    checkAllTransactions,
    clearCheckedTransactions,
  } = useTransactionSelection();

  const {
    filterQueryString,
    appliedQueryString,
    filteredTransactions,
    onFilterQueryChange,
    onApplyFilter,
    filteredCount,
    totalCount,
  } = useTransactionFilters({ transactions });

  const detailsLayout = useAppSettingsStore((s) => s.detailsPanelLayout);
  const isBottomLayout = detailsLayout === "bottom";

  const detailsPanelRef = useResizablePanelController({
    isExpanded: !!selectedTransaction,
  });

  const layoutId = isBottomLayout ? "network-dashboard-layout-bottom" : "network-dashboard-layout";

  const { defaultLayout, onLayoutChanged } = useDefaultLayout({
    id: layoutId,
    storage: localStorage,
  });

  const {
    editorMode,
    setEditorMode,
    builderState,
    handleModeChange,
    handleBuilderStateChange,
    handleBuilderApply,
  } = useFilterEditor({ filterQueryString, onFilterQueryChange, onApplyFilter });

  const {
    sequenceReplayOpen,
    setSequenceReplayOpen,
    composeOpen,
    setComposeOpen,
    advancedRepeatTarget,
    setAdvancedRepeatTarget,
  } = useNetworkDialogs();

  const checkedTransactions = useMemo(
    () => transactions.filter((t) => t.request?.id && checkedTransactionIds.has(t.request.id)),
    [transactions, checkedTransactionIds],
  );

  const {
    busy,
    handleExportHar,
    handleSaveSession,
    handleLoadSession,
    handleImportHar,
    handleExportOpenApi,
  } = useNetworkExport({
    transactions,
    filteredTransactions,
    checkedTransactions,
    appliedQueryString,
    setTransactions,
    appendTransactions,
  });

  const { diffResult, diffLoading, canCompare, handleCompare, closeDiff } = useNetworkDiff({
    checkedTransactionIds,
    checkedTransactions,
  });

  const interceptRuleDialogOpen = useInterceptRuleDialogStore((s) => s.open);
  const interceptRuleInitialValues = useInterceptRuleDialogStore((s) => s.initialValues);
  const closeInterceptRuleDialog = useInterceptRuleDialogStore((s) => s.close);

  const createTransactionDeleteHandler = useCallback(
    (id: string) => () => {
      deleteTransaction(id);

      if (selectedTransaction?.request?.id === id) {
        clearSelectedTransaction();
      }
    },
    [clearSelectedTransaction, deleteTransaction, selectedTransaction],
  );

  const createTransactionPinHandler = useCallback(
    (id: string) => () => {
      togglePinTransaction(id);
    },
    [togglePinTransaction],
  );

  const createTransactionCheckHandler = useCallback(
    (id: string) => () => {
      toggleCheckTransaction(id);
    },
    [toggleCheckTransaction],
  );

  const createTransactionToggleHandler = useCallback(
    (transaction: import("@/entities/proxy").HttpTransaction) => () => {
      toggleSelectedTransaction(transaction);
    },
    [toggleSelectedTransaction],
  );

  const createTransactionSelectHandler = useCallback(
    (transaction: import("@/entities/proxy").HttpTransaction) => () => {
      setSelectedTransaction(transaction);
    },
    [setSelectedTransaction],
  );

  const handleToggleCheckAll = useCallback(() => {
    const allIds = filteredTransactions
      .map((t) => t.request?.id)
      .filter((id): id is string => !!id);
    checkAllTransactions(allIds);
  }, [filteredTransactions, checkAllTransactions]);

  const networkTableProps = {
    transactions: filteredTransactions,
    pinnedTransactionIds,
    checkedTransactionIds,
    selectedTransaction,
    createTransactionSelectHandler: createTransactionToggleHandler,
    createTransactionDeleteHandler,
    createTransactionPinHandler,
    createTransactionCheckHandler,
    onAdvancedRepeat: setAdvancedRepeatTarget,
    onToggleCheckAll: handleToggleCheckAll,
  } as const;

  const hostPathTreeProps = {
    transactions: filteredTransactions,
    selectedTransaction,
    createTransactionSelectHandler,
  } as const;

  return (
    <>
      <div className="flex-1 flex flex-col h-full overflow-x-hidden">
        <NetworkHeader
          paused={paused}
          hasTransactions={filteredTransactions.length > 0}
          exporting={busy}
          togglePause={togglePause}
          clearTransactions={clearTransactions}
          onExportHar={handleExportHar}
          onExportOpenApi={handleExportOpenApi}
          onSaveSession={handleSaveSession}
          onLoadSession={handleLoadSession}
          onImportHar={handleImportHar}
          onCompose={() => setComposeOpen(true)}
          filterSlot={
            <QueryFilterEditor
              totalCount={totalCount}
              filteredCount={filteredCount}
              value={filterQueryString}
              appliedValue={appliedQueryString}
              onChange={onFilterQueryChange}
              onApply={onApplyFilter}
              mode={editorMode}
              onModeChange={handleModeChange}
              builderState={builderState}
            />
          }
        />

        {editorMode === "builder" && (
          <QueryBuilder
            builderState={builderState}
            onBuilderStateChange={handleBuilderStateChange}
            onApply={handleBuilderApply}
            onClose={() => setEditorMode("code")}
            totalCount={totalCount}
            filteredCount={filteredCount}
          />
        )}

        <div className="flex-1 flex flex-col overflow-hidden relative">
          {isBottomLayout ? (
            <ResizablePanelGroup
              orientation="vertical"
              defaultLayout={
                defaultLayout ?? {
                  "network-top": 60,
                  "transaction-details-bottom": 0,
                }
              }
              onLayoutChanged={onLayoutChanged}
              className="flex-1 flex border border-b-0 shadow-[0_0_10px_0_rgba(0,0,0,0.05)] bg-background"
            >
              <ResizablePanel id="network-top" className="flex overflow-hidden">
                <ResizablePanelGroup orientation="horizontal" className="flex-1">
                  <ResizablePanel
                    id="host-path-tree-bottom"
                    className="h-full overflow-hidden"
                    maxSize="40%"
                    minSize="10%"
                    collapsible
                  >
                    <HostPathTree {...hostPathTreeProps} />
                  </ResizablePanel>

                  <ResizableHandle withHandle />

                  <ResizablePanel
                    id="network-table-bottom"
                    className="flex flex-1 h-full overflow-hidden"
                  >
                    <NetworkTable {...networkTableProps} />
                  </ResizablePanel>
                </ResizablePanelGroup>
              </ResizablePanel>

              <ResizableHandle withHandle={!!selectedTransaction} orientation="vertical" />
              <ResizablePanel
                panelRef={detailsPanelRef}
                id="transaction-details-bottom"
                maxSize="70%"
                minSize="20%"
                collapsible
                collapsedSize="0%"
                className="h-full overflow-y-auto"
              >
                {selectedTransaction && (
                  <TransactionDetails
                    transaction={selectedTransaction}
                    clearSelectedTransaction={clearSelectedTransaction}
                    layout="bottom"
                  />
                )}
              </ResizablePanel>
            </ResizablePanelGroup>
          ) : (
            <ResizablePanelGroup
              orientation="horizontal"
              defaultLayout={
                defaultLayout ?? {
                  "host-path-tree": 25,
                  "network-table": 75,
                  "transaction-details": 0,
                }
              }
              onLayoutChanged={onLayoutChanged}
              className="flex-1 flex border border-b-0 shadow-[0_0_10px_0_rgba(0,0,0,0.05)] bg-background"
            >
              <ResizablePanel
                id="host-path-tree"
                className="h-full overflow-hidden"
                maxSize="40%"
                minSize="10%"
                collapsible
              >
                <HostPathTree {...hostPathTreeProps} />
              </ResizablePanel>

              <ResizableHandle withHandle />

              <ResizablePanel id="network-table" className="flex flex-1 h-full overflow-hidden">
                <NetworkTable {...networkTableProps} />
              </ResizablePanel>

              <ResizableHandle withHandle={!!selectedTransaction} />
              <ResizablePanel
                panelRef={detailsPanelRef}
                id="transaction-details"
                maxSize="50%"
                minSize="25%"
                collapsible
                collapsedSize="0%"
                className="w-96 h-full overflow-y-auto"
              >
                {selectedTransaction && (
                  <TransactionDetails
                    transaction={selectedTransaction}
                    clearSelectedTransaction={clearSelectedTransaction}
                  />
                )}
              </ResizablePanel>
            </ResizablePanelGroup>
          )}

          {checkedTransactionIds.size > 0 && (
            <div className="absolute bottom-4 left-1/2 -translate-x-1/2 flex items-center gap-3 bg-primary text-primary-foreground px-4 py-2.5 rounded-lg shadow-lg z-10">
              <span className="text-sm font-medium">{checkedTransactionIds.size} selected</span>
              <Button size="sm" variant="secondary" onClick={() => setSequenceReplayOpen(true)}>
                <Play className="w-4 h-4 mr-1" />
                Replay
              </Button>
              {canCompare && (
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={handleCompare}
                  disabled={diffLoading}
                >
                  <GitCompareArrows className="w-4 h-4 mr-1" />
                  {diffLoading ? "Comparing..." : "Compare"}
                </Button>
              )}
              <Button
                size="sm"
                variant="ghost"
                className="text-primary-foreground hover:text-primary-foreground/80 hover:bg-primary/80"
                onClick={clearCheckedTransactions}
              >
                <X className="w-4 h-4" />
              </Button>
            </div>
          )}

          {diffResult && (
            <div className="absolute inset-0 z-20 bg-background border rounded-lg shadow-xl overflow-hidden">
              <DiffView diff={diffResult} onClose={closeDiff} />
            </div>
          )}
        </div>
      </div>

      <ReplayDialog open={composeOpen} onOpenChange={setComposeOpen} hideTrigger />

      <SequenceReplayDialog
        open={sequenceReplayOpen}
        onOpenChange={setSequenceReplayOpen}
        transactions={checkedTransactions}
        onComplete={clearCheckedTransactions}
      />

      {advancedRepeatTarget && (
        <AdvancedRepeatDialog
          open={!!advancedRepeatTarget}
          onOpenChange={(open) => {
            if (!open) setAdvancedRepeatTarget(null);
          }}
          transaction={advancedRepeatTarget}
        />
      )}

      <RuleFormDialog
        open={interceptRuleDialogOpen}
        onOpenChange={(open) => {
          if (!open) closeInterceptRuleDialog();
        }}
        editingRule={null}
        initialValues={interceptRuleInitialValues}
      />
    </>
  );
};

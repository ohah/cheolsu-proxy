import { useCallback, useMemo, useState } from "react";

import { TransactionDetails, SequenceReplayDialog } from "@/features/transaction-details";

import { NetworkHeader } from "@/widgets/network-header";
import { AppSidebar } from "@/shared/app-sidebar";
import { NetworkTable } from "@/widgets/network-table";

import { ResizableHandle, ResizablePanel, ResizablePanelGroup, Button } from "@/shared/ui";
import { useDefaultLayout } from "react-resizable-panels";
import { Play, X } from "lucide-react";

import {
  useProxyEventControl,
  useTransactionFilters,
  useTransactions,
  useResizablePanelController,
} from "../hooks";
import { useProxyStore } from "@/shared/stores";
import { HostPathTree } from "@/widgets/host-path-tree/ui/host-path-tree";

export const NetworkDashboard = () => {
  const { isConnected } = useProxyStore();

  const {
    transactions,
    addTransaction,
    clearTransactions,
    deleteTransaction,
    selectedTransaction,
    createTransactionToggleHandler,
    createTransactionSelectHandler,
    clearSelectedTransaction,
    togglePinTransaction,
    pinnedTransactionIds,
    checkedTransactionIds,
    toggleCheckTransaction,
    checkAllTransactions,
    clearCheckedTransactions,
  } = useTransactions();

  const {
    filterQueryString,
    appliedQueryString,
    filteredTransactions,
    onFilterQueryChange,
    onApplyFilter,
    filteredCount,
    totalCount,
  } = useTransactionFilters({ transactions });

  const detailsPanelRef = useResizablePanelController({ isExpanded: !!selectedTransaction });

  const { defaultLayout, onLayoutChanged } = useDefaultLayout({
    id: "network-dashboard-layout",
    storage: localStorage,
  });

  const { paused, togglePause } = useProxyEventControl({ onTransactionReceived: addTransaction });

  const [sequenceReplayOpen, setSequenceReplayOpen] = useState(false);

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

  const handleToggleCheckAll = useCallback(() => {
    const allIds = filteredTransactions
      .map((t) => t.request?.id)
      .filter((id): id is string => !!id);
    checkAllTransactions(allIds);
  }, [filteredTransactions, checkAllTransactions]);

  const checkedTransactions = useMemo(
    () => transactions.filter((t) => t.request?.id && checkedTransactionIds.has(t.request.id)),
    [transactions, checkedTransactionIds],
  );

  return (
    <div className="flex h-[100vh] w-full">
      <AppSidebar isConnected={isConnected} />

      <div className="flex-1 flex flex-col h-full overflow-x-hidden">
        <NetworkHeader
          filterQueryString={filterQueryString}
          appliedQueryString={appliedQueryString}
          filteredCount={filteredCount}
          totalCount={totalCount}
          paused={paused}
          togglePause={togglePause}
          onFilterQueryChange={onFilterQueryChange}
          onApplyFilter={onApplyFilter}
          clearTransactions={clearTransactions}
        />

        <div className="flex-1 flex flex-col overflow-hidden relative">
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
              <HostPathTree
                transactions={filteredTransactions}
                selectedTransaction={selectedTransaction}
                createTransactionSelectHandler={createTransactionSelectHandler}
              />
            </ResizablePanel>

            <ResizableHandle withHandle />

            <ResizablePanel id="network-table" className="flex flex-1 h-full overflow-hidden">
              <NetworkTable
                transactions={filteredTransactions}
                pinnedTransactionIds={pinnedTransactionIds}
                checkedTransactionIds={checkedTransactionIds}
                selectedTransaction={selectedTransaction}
                createTransactionSelectHandler={createTransactionToggleHandler}
                createTransactionDeleteHandler={createTransactionDeleteHandler}
                createTransactionPinHandler={createTransactionPinHandler}
                createTransactionCheckHandler={createTransactionCheckHandler}
                onToggleCheckAll={handleToggleCheckAll}
              />
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

          {checkedTransactionIds.size > 0 && (
            <div className="absolute bottom-4 left-1/2 -translate-x-1/2 flex items-center gap-3 bg-primary text-primary-foreground px-4 py-2.5 rounded-lg shadow-lg z-10">
              <span className="text-sm font-medium">{checkedTransactionIds.size}개 선택됨</span>
              <Button size="sm" variant="secondary" onClick={() => setSequenceReplayOpen(true)}>
                <Play className="w-4 h-4 mr-1" />
                Replay
              </Button>
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
        </div>
      </div>

      <SequenceReplayDialog
        open={sequenceReplayOpen}
        onOpenChange={setSequenceReplayOpen}
        transactions={checkedTransactions}
        onComplete={clearCheckedTransactions}
      />
    </div>
  );
};

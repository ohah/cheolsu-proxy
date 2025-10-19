import { useCallback } from 'react';

import { TransactionDetails } from '@/features/transaction-details';

import { NetworkHeader } from '@/widgets/network-header';
import { AppSidebar } from '@/shared/app-sidebar';
import { NetworkTable } from '@/widgets/network-table';

import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from '@/shared/ui';

import { useProxyEventControl, useTransactionFilters, useTransactions, useResizablePanelController } from '../hooks';
import { useProxyStore } from '@/shared/stores';
import { HostPathTree } from '@/widgets/host-path-tree/ui/host-path-tree';

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
  } = useTransactions();

  const {
    searchQuery,
    setMethodFilter,
    setStatusFilter,
    filteredTransactions,
    onSearchQueryChange,
    filteredCount,
    totalCount,
  } = useTransactionFilters({ transactions });

  const detailsPanelRef = useResizablePanelController({ isExpanded: !!selectedTransaction });

  const { paused, togglePause } = useProxyEventControl({ onTransactionReceived: addTransaction });

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

  return (
    <div className="flex h-[100vh] w-full">
      <AppSidebar isConnected={isConnected} />

      <div className="flex-1 flex flex-col h-full">
        <NetworkHeader
          searchQuery={searchQuery}
          filteredCount={filteredCount}
          totalCount={totalCount}
          paused={paused}
          togglePause={togglePause}
          onSearchQueryChange={onSearchQueryChange}
          onStatusFilterChange={setStatusFilter}
          onMethodFilterChange={setMethodFilter}
          clearTransactions={clearTransactions}
        />

        <ResizablePanelGroup
          direction="horizontal"
          autoSaveId="network-dashboard-layout"
          className="flex-1 flex border border-b-0 rounded-tl-lg shadow-[0_0_10px_0_rgba(0,0,0,0.05)] bg-background"
        >
          <ResizablePanel
            id="host-path-tree"
            className="h-full overflow-hidden"
            maxSize={40}
            minSize={10}
            defaultSize={25}
          >
            <HostPathTree
              transactions={filteredTransactions}
              selectedTransaction={selectedTransaction}
              createTransactionSelectHandler={createTransactionSelectHandler}
            />
          </ResizablePanel>

          <ResizableHandle withHandle />

          <ResizablePanel id="network-table" defaultSize={75} className="flex flex-1 h-full overflow-hidden">
            <NetworkTable
              transactions={filteredTransactions}
              pinnedTransactionIds={pinnedTransactionIds}
              selectedTransaction={selectedTransaction}
              createTransactionSelectHandler={createTransactionToggleHandler}
              createTransactionDeleteHandler={createTransactionDeleteHandler}
              createTransactionPinHandler={createTransactionPinHandler}
            />
          </ResizablePanel>

          <ResizableHandle withHandle />
          <ResizablePanel
            ref={detailsPanelRef}
            id="transaction-details"
            defaultSize={25}
            maxSize={50}
            minSize={25}
            collapsible
            collapsedSize={0}
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
      </div>
    </div>
  );
};

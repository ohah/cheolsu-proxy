import { useCallback } from 'react';

import { TransactionDetails } from '@/features/transaction-details';

import { NetworkHeader } from '@/widgets/network-header';
import { NetworkSidebar } from '@/widgets/network-sidebar';
import { NetworkTable } from '@/widgets/network-table';

import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from '@/shared/ui';

import { useProxyEventControl, useProxyInitialization, useTransactionFilters, useTransactions } from '../hooks';

export const NetworkDashboard = () => {
  const { isConnected } = useProxyInitialization();

  const {
    transactions,
    addTransaction,
    clearTransactions,
    deleteTransaction,
    selectedTransaction,
    createTransactionSelectHandler,
    clearSelectedTransaction,
  } = useTransactions();

  const { paused, togglePause } = useProxyEventControl({ onTransactionReceived: addTransaction });

  const {
    searchQuery,
    setMethodFilter,
    setStatusFilter,
    filteredTransactions,
    onSearchQueryChange,
    filteredCount,
    totalCount,
  } = useTransactionFilters({ transactions });

  const createTransactionDeleteHandler = useCallback(
    (id: number) => () => {
      deleteTransaction(id);

      if (selectedTransaction?.request?.time === id) {
        clearSelectedTransaction();
      }
    },
    [],
  );

  return (
    <div className="flex h-[100vh] w-full">
      <NetworkSidebar isConnected={isConnected} />

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
          className="flex-1 flex border border-b-0 rounded-tl-lg shadow-[0_0_10px_0_rgba(0,0,0,0.05)] bg-background"
        >
          <ResizablePanel className="flex-1 h-full overflow-hidden">
            <NetworkTable
              transactions={filteredTransactions}
              selectedTransaction={selectedTransaction}
              createTransactionSelectHandler={createTransactionSelectHandler}
              createTransactionDeleteHandler={createTransactionDeleteHandler}
            />
          </ResizablePanel>
          <ResizableHandle withHandle />
          {selectedTransaction && (
            <ResizablePanel className="w-96 h-full overflow-y-auto">
              <TransactionDetails
                transaction={selectedTransaction}
                clearSelectedTransaction={clearSelectedTransaction}
              />
            </ResizablePanel>
          )}
        </ResizablePanelGroup>
      </div>
    </div>
  );
};

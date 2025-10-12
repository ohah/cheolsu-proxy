import { useCallback } from 'react';

import type { HttpTransaction } from '@/entities/proxy';
import { useTransactionStore } from '@/shared/stores';

export const useTransactions = () => {
  const {
    transactions,
    selectedTransaction,
    addTransaction,
    clearTransactions,
    deleteTransaction,
    setSelectedTransaction,
  } = useTransactionStore();

  const createTransactionSelectHandler = useCallback(
    (transaction: HttpTransaction) => () => {
      setSelectedTransaction(transaction);
    },
    [setSelectedTransaction],
  );

  const clearSelectedTransaction = useCallback(() => {
    setSelectedTransaction(null);
  }, [setSelectedTransaction]);

  return {
    transactions,
    addTransaction,
    clearTransactions,
    deleteTransaction,
    selectedTransaction,
    createTransactionSelectHandler,
    clearSelectedTransaction,
  };
};

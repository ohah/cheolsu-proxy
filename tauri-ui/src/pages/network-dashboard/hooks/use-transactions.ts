import { useState, useCallback } from 'react';

import type { HttpTransaction } from '@/entities/proxy';

export const useTransactions = () => {
  const [transactions, setTransactions] = useState<HttpTransaction[]>([]);
  const [selectedTransaction, setSelectedTransaction] = useState<HttpTransaction | null>(null);

  const addTransaction = useCallback((transaction: HttpTransaction) => {
    setTransactions(prev => {
      const existingTransaction = prev.find(t => t.request?.time === transaction.request?.time);

      if (existingTransaction) {
        return prev;
      }

      return [...prev, transaction];
    });
  }, []);

  const clearTransactions = useCallback(() => {
    setTransactions([]);
    setSelectedTransaction(null)
  }, []);

  const deleteTransaction = useCallback((id: number) => {
    setTransactions(prev => prev.filter((transaction) => transaction?.request?.time !== id));
  }, []);


  const createTransactionSelectHandler = useCallback((transaction: HttpTransaction) => () => {
    setSelectedTransaction(transaction);
  }, []);

  const clearSelectedTransaction = useCallback(() => {
    setSelectedTransaction(null)
  }, [])

  return {
    transactions,
    addTransaction,
    clearTransactions,
    deleteTransaction,
    selectedTransaction,
    createTransactionSelectHandler,
    clearSelectedTransaction
  };
};

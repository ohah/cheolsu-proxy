import { useState, useCallback } from "react";

import type { HttpTransaction } from "@/entities/proxy";

export const useTransactions = () => {
  const [transactions, setTransactions] = useState<HttpTransaction[]>([]);
  const [selectedTransaction, setSelectedTransaction] = useState<HttpTransaction | null>(null);
  const [pinnedTransactionIds, setPinnedTransactionIds] = useState<Set<string>>(new Set());

  const addTransaction = useCallback((transaction: HttpTransaction) => {
    setTransactions((prev) => {
      const existingTransaction = prev.find((t) => t.request?.id === transaction.request?.id);

      if (existingTransaction) {
        return prev;
      }

      return [...prev, transaction];
    });
  }, []);

  const clearTransactions = useCallback(() => {
    setTransactions([]);
    setSelectedTransaction(null);
    setPinnedTransactionIds(new Set());
  }, []);

  const deleteTransaction = useCallback((id: string) => {
    setTransactions((prev) => prev.filter((transaction) => transaction?.request?.id !== id));
    setPinnedTransactionIds((prev) => {
      const newSet = new Set(prev);
      newSet.delete(id);
      return newSet;
    });
  }, []);

  const togglePinTransaction = useCallback((id: string) => {
    setPinnedTransactionIds((prev) => {
      const newSet = new Set(prev);
      if (newSet.has(id)) {
        newSet.delete(id);
      } else {
        newSet.add(id);
      }
      return newSet;
    });
  }, []);

  const createTransactionSelectHandler = useCallback(
    (transaction: HttpTransaction) => () => {
      setSelectedTransaction(transaction);
    },
    [],
  );

  const clearSelectedTransaction = useCallback(() => {
    setSelectedTransaction(null);
  }, []);

  const createTransactionToggleHandler = useCallback(
    (transaction: HttpTransaction) => () => {
      setSelectedTransaction((prev) =>
        prev?.request?.id === transaction.request?.id ? null : transaction,
      );
    },
    [],
  );

  return {
    transactions,
    addTransaction,
    clearTransactions,
    deleteTransaction,
    selectedTransaction,
    createTransactionSelectHandler,
    createTransactionToggleHandler,
    clearSelectedTransaction,
    togglePinTransaction,
    pinnedTransactionIds,
  };
};

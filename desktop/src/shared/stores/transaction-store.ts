import { create } from "zustand";

import type { HttpTransaction } from "@/entities/proxy";

export function getTransactionSize(t: HttpTransaction): number {
  return (t.request?.body_size ?? 0) + (t.response?.body_size ?? 0);
}

interface TransactionStoreState {
  transactions: HttpTransaction[];
  transactionIds: Set<string>;
  totalSizeBytes: number;
  selectedTransaction: HttpTransaction | null;
  pinnedTransactionIds: Set<string>;
  checkedTransactionIds: Set<string>;
  paused: boolean;
  addTransaction: (transaction: HttpTransaction) => void;
  clearTransactions: () => void;
  deleteTransaction: (id: string) => void;
  deleteTransactionsBatch: (ids: Set<string>) => void;
  setSelectedTransaction: (transaction: HttpTransaction | null) => void;
  toggleSelectedTransaction: (transaction: HttpTransaction) => void;
  clearSelectedTransaction: () => void;
  togglePinTransaction: (id: string) => void;
  toggleCheckTransaction: (id: string) => void;
  checkAllTransactions: (ids: string[]) => void;
  clearCheckedTransactions: () => void;
  setTransactions: (transactions: HttpTransaction[]) => void;
  appendTransactions: (transactions: HttpTransaction[]) => void;
  togglePause: () => void;
  setPaused: (paused: boolean) => void;
}

export const useTransactionStore = create<TransactionStoreState>()((set) => ({
  transactions: [],
  transactionIds: new Set(),
  totalSizeBytes: 0,
  selectedTransaction: null,
  pinnedTransactionIds: new Set(),
  checkedTransactionIds: new Set(),
  paused: false,

  addTransaction: (transaction) =>
    set((state) => {
      const id = transaction.request?.id;
      if (id && state.transactionIds.has(id)) return state;
      const transactionIds = new Set(state.transactionIds);
      if (id) transactionIds.add(id);
      const newSize = getTransactionSize(transaction);
      return {
        transactions: [...state.transactions, transaction],
        transactionIds,
        totalSizeBytes: state.totalSizeBytes + newSize,
      };
    }),

  clearTransactions: () =>
    set({
      transactions: [],
      transactionIds: new Set(),
      totalSizeBytes: 0,
      selectedTransaction: null,
      pinnedTransactionIds: new Set(),
      checkedTransactionIds: new Set(),
    }),

  deleteTransaction: (id) =>
    set((state) => {
      const target = state.transactions.find((t) => t.request?.id === id);
      const removedSize = target ? getTransactionSize(target) : 0;
      const transactionIds = new Set(state.transactionIds);
      transactionIds.delete(id);
      const pinnedTransactionIds = new Set(state.pinnedTransactionIds);
      pinnedTransactionIds.delete(id);
      const checkedTransactionIds = new Set(state.checkedTransactionIds);
      checkedTransactionIds.delete(id);
      return {
        transactions: state.transactions.filter((t) => t.request?.id !== id),
        transactionIds,
        totalSizeBytes: Math.max(0, state.totalSizeBytes - removedSize),
        pinnedTransactionIds,
        checkedTransactionIds,
      };
    }),

  deleteTransactionsBatch: (ids) =>
    set((state) => {
      if (ids.size === 0) return state;
      let removedSize = 0;
      const transactions = state.transactions.filter((t) => {
        const id = t.request?.id;
        if (id && ids.has(id)) {
          removedSize += getTransactionSize(t);
          return false;
        }
        return true;
      });
      const transactionIds = new Set(state.transactionIds);
      const pinnedTransactionIds = new Set(state.pinnedTransactionIds);
      const checkedTransactionIds = new Set(state.checkedTransactionIds);
      for (const id of ids) {
        transactionIds.delete(id);
        pinnedTransactionIds.delete(id);
        checkedTransactionIds.delete(id);
      }
      return {
        transactions,
        transactionIds,
        totalSizeBytes: Math.max(0, state.totalSizeBytes - removedSize),
        pinnedTransactionIds,
        checkedTransactionIds,
      };
    }),

  setSelectedTransaction: (transaction) => set({ selectedTransaction: transaction }),

  toggleSelectedTransaction: (transaction) =>
    set((state) => ({
      selectedTransaction:
        state.selectedTransaction?.request?.id === transaction.request?.id ? null : transaction,
    })),

  clearSelectedTransaction: () => set({ selectedTransaction: null }),

  togglePinTransaction: (id) =>
    set((state) => {
      const newSet = new Set(state.pinnedTransactionIds);
      if (newSet.has(id)) {
        newSet.delete(id);
      } else {
        newSet.add(id);
      }
      return { pinnedTransactionIds: newSet };
    }),

  toggleCheckTransaction: (id) =>
    set((state) => {
      const newSet = new Set(state.checkedTransactionIds);
      if (newSet.has(id)) {
        newSet.delete(id);
      } else {
        newSet.add(id);
      }
      return { checkedTransactionIds: newSet };
    }),

  checkAllTransactions: (ids) =>
    set((state) => {
      const allChecked = ids.every((id) => state.checkedTransactionIds.has(id));
      return { checkedTransactionIds: allChecked ? new Set() : new Set(ids) };
    }),

  clearCheckedTransactions: () => set({ checkedTransactionIds: new Set() }),

  setTransactions: (transactions) =>
    set({
      transactions,
      transactionIds: new Set(
        transactions.map((t) => t.request?.id).filter((id): id is string => !!id),
      ),
      totalSizeBytes: transactions.reduce((acc, t) => acc + getTransactionSize(t), 0),
      selectedTransaction: null,
      pinnedTransactionIds: new Set(),
      checkedTransactionIds: new Set(),
    }),

  appendTransactions: (newTransactions) =>
    set((state) => {
      const transactionIds = new Set(state.transactionIds);
      const filtered = newTransactions.filter((t) => {
        const id = t.request?.id;
        if (id && transactionIds.has(id)) return false;
        if (id) transactionIds.add(id);
        return true;
      });
      if (filtered.length === 0) return state;
      const addedSize = filtered.reduce((acc, t) => acc + getTransactionSize(t), 0);
      return {
        transactions: [...state.transactions, ...filtered],
        transactionIds,
        totalSizeBytes: state.totalSizeBytes + addedSize,
      };
    }),

  togglePause: () => set((state) => ({ paused: !state.paused })),
  setPaused: (paused: boolean) => set({ paused }),
}));

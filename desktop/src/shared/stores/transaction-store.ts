import { create } from "zustand";

import type { HttpTransaction } from "@/entities/proxy";

interface TransactionStoreState {
  transactions: HttpTransaction[];
  transactionIds: Set<string>;
  selectedTransaction: HttpTransaction | null;
  pinnedTransactionIds: Set<string>;
  checkedTransactionIds: Set<string>;
  paused: boolean;
  addTransaction: (transaction: HttpTransaction) => void;
  clearTransactions: () => void;
  deleteTransaction: (id: string) => void;
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
      return {
        transactions: [...state.transactions, transaction],
        transactionIds,
      };
    }),

  clearTransactions: () =>
    set({
      transactions: [],
      transactionIds: new Set(),
      selectedTransaction: null,
      pinnedTransactionIds: new Set(),
      checkedTransactionIds: new Set(),
    }),

  deleteTransaction: (id) =>
    set((state) => {
      const transactionIds = new Set(state.transactionIds);
      transactionIds.delete(id);
      const pinnedTransactionIds = new Set(state.pinnedTransactionIds);
      pinnedTransactionIds.delete(id);
      const checkedTransactionIds = new Set(state.checkedTransactionIds);
      checkedTransactionIds.delete(id);
      return {
        transactions: state.transactions.filter((t) => t.request?.id !== id),
        transactionIds,
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
      return {
        transactions: [...state.transactions, ...filtered],
        transactionIds,
      };
    }),

  togglePause: () => set((state) => ({ paused: !state.paused })),
  setPaused: (paused: boolean) => set({ paused }),
}));

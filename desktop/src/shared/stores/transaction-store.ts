import { create } from "zustand";

import type { HttpTransaction } from "@/entities/proxy";

interface TransactionStoreState {
  transactions: HttpTransaction[];
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
  togglePause: () => void;
}

export const useTransactionStore = create<TransactionStoreState>()((set) => ({
  transactions: [],
  selectedTransaction: null,
  pinnedTransactionIds: new Set(),
  checkedTransactionIds: new Set(),
  paused: false,

  addTransaction: (transaction) =>
    set((state) => {
      if (state.transactions.some((t) => t.request?.id === transaction.request?.id)) {
        return state;
      }
      return { transactions: [...state.transactions, transaction] };
    }),

  clearTransactions: () =>
    set({
      transactions: [],
      selectedTransaction: null,
      pinnedTransactionIds: new Set(),
      checkedTransactionIds: new Set(),
    }),

  deleteTransaction: (id) =>
    set((state) => {
      const pinnedTransactionIds = new Set(state.pinnedTransactionIds);
      pinnedTransactionIds.delete(id);
      const checkedTransactionIds = new Set(state.checkedTransactionIds);
      checkedTransactionIds.delete(id);
      return {
        transactions: state.transactions.filter((t) => t.request?.id !== id),
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

  togglePause: () => set((state) => ({ paused: !state.paused })),
}));

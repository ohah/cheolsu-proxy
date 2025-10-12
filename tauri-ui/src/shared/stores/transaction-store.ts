import { create } from 'zustand';
import { persist } from 'zustand/middleware';

import type { HttpTransaction } from '@/entities/proxy';

interface TransactionState {
  transactions: HttpTransaction[];
  selectedTransaction: HttpTransaction | null;
  isPaused: boolean;
  addTransaction: (transaction: HttpTransaction) => void;
  setSelectedTransaction: (transaction: HttpTransaction | null) => void;
  clearTransactions: () => void;
  deleteTransaction: (id: string) => void;
  setPaused: (paused: boolean) => void;
  togglePause: () => void;
}

export const useTransactionStore = create<TransactionState>()(
  persist(
    (set, get) => ({
      transactions: [],
      selectedTransaction: null,
      isPaused: false,

      addTransaction: (transaction: HttpTransaction) => {
        const { isPaused, transactions } = get();
        if (isPaused) return;

        // 중복 transaction 체크 (id 기준)
        const existingTransaction = transactions.find((t) => t.request?.id === transaction.request?.id);
        if (existingTransaction) return;

        set((state) => ({
          transactions: [transaction, ...state.transactions],
        }));
      },

      setSelectedTransaction: (transaction: HttpTransaction | null) => {
        set({ selectedTransaction: transaction });
      },

      clearTransactions: () => {
        set({ transactions: [], selectedTransaction: null });
      },

      deleteTransaction: (id: string) => {
        set((state) => {
          const filteredTransactions = state.transactions.filter((transaction) => transaction?.request?.id !== id);

          // 삭제된 transaction이 현재 선택된 transaction이면 선택 해제
          const newSelectedTransaction =
            state.selectedTransaction?.request?.id === id ? null : state.selectedTransaction;

          return {
            transactions: filteredTransactions,
            selectedTransaction: newSelectedTransaction,
          };
        });
      },

      setPaused: (paused: boolean) => {
        set({ isPaused: paused });
      },

      togglePause: () => {
        set((state) => ({ isPaused: !state.isPaused }));
      },
    }),
    {
      name: 'cheolsu-transaction-store',
      // transactions는 persist하지 않음 (앱 재시작 시 초기화되어야 함)
      // currentTransaction만 persist
      partialize: (state) => ({
        isPaused: state.isPaused,
      }),
    },
  ),
);

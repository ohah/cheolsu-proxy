import { useShallow } from "zustand/react/shallow";
import { useTransactionStore } from "@/shared/stores";

/** 트랜잭션 데이터 관련 선택자 */
export function useTransactionData() {
  return useTransactionStore(
    useShallow((s) => ({
      transactions: s.transactions,
      selectedTransaction: s.selectedTransaction,
      pinnedTransactionIds: s.pinnedTransactionIds,
      checkedTransactionIds: s.checkedTransactionIds,
      paused: s.paused,
    })),
  );
}

/** 트랜잭션 액션 관련 선택자 */
export function useTransactionActions() {
  return useTransactionStore(
    useShallow((s) => ({
      clearTransactions: s.clearTransactions,
      deleteTransaction: s.deleteTransaction,
      setTransactions: s.setTransactions,
      appendTransactions: s.appendTransactions,
      togglePause: s.togglePause,
    })),
  );
}

/** 트랜잭션 선택/핀/체크 관련 선택자 */
export function useTransactionSelection() {
  return useTransactionStore(
    useShallow((s) => ({
      toggleSelectedTransaction: s.toggleSelectedTransaction,
      setSelectedTransaction: s.setSelectedTransaction,
      clearSelectedTransaction: s.clearSelectedTransaction,
      togglePinTransaction: s.togglePinTransaction,
      toggleCheckTransaction: s.toggleCheckTransaction,
      checkAllTransactions: s.checkAllTransactions,
      clearCheckedTransactions: s.clearCheckedTransactions,
    })),
  );
}

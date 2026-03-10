import { useShallow } from "zustand/react/shallow";
import { useTransactionStore } from "@/shared/stores";

/**
 * useTransactionStore에서 자주 사용되는 선택자들을 하나로 묶어
 * useShallow를 통해 불필요한 리렌더링을 방지하는 훅
 */
export function useTransactionSelectors() {
  return useTransactionStore(
    useShallow((s) => ({
      transactions: s.transactions,
      selectedTransaction: s.selectedTransaction,
      pinnedTransactionIds: s.pinnedTransactionIds,
      checkedTransactionIds: s.checkedTransactionIds,
      paused: s.paused,
      clearTransactions: s.clearTransactions,
      deleteTransaction: s.deleteTransaction,
      toggleSelectedTransaction: s.toggleSelectedTransaction,
      setSelectedTransaction: s.setSelectedTransaction,
      clearSelectedTransaction: s.clearSelectedTransaction,
      togglePinTransaction: s.togglePinTransaction,
      toggleCheckTransaction: s.toggleCheckTransaction,
      checkAllTransactions: s.checkAllTransactions,
      clearCheckedTransactions: s.clearCheckedTransactions,
      setTransactions: s.setTransactions,
      appendTransactions: s.appendTransactions,
      togglePause: s.togglePause,
    })),
  );
}

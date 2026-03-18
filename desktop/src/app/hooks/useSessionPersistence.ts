import { useCallback, useEffect } from "react";
import { useTransactionStore } from "@/shared/stores";
import { emit, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { autosaveSession, autoloadSession } from "@/shared/api/proxy";
import { useAppSettingsStore } from "@/shared/stores/app-settings-store";
import type { HttpTransaction } from "@/entities/proxy";

/**
 * 세션 자동 저장/복원 및 앱 종료 시 저장 로직을 담당하는 훅
 */
export function useSessionPersistence() {
  const setTransactions = useTransactionStore((s) => s.setTransactions);

  // 앱 시작 시 자동 저장된 세션 복원
  useEffect(() => {
    const autoSessionEnabled = useAppSettingsStore.getState().autosaveSession;
    if (!autoSessionEnabled) return;

    autoloadSession()
      .then((result) => {
        if (result && result.transactions_json) {
          try {
            const tuples = JSON.parse(result.transactions_json) as [unknown, unknown][];
            const loaded: HttpTransaction[] = tuples.map(([request, response]) => ({
              request,
              response,
            })) as HttpTransaction[];
            if (loaded.length > 0) {
              // 세션 복원 중 도착한 트랜잭션이 있으면 세션 데이터 뒤에 병합
              const { transactions: arrived } = useTransactionStore.getState();
              if (arrived.length > 0) {
                setTransactions([...loaded, ...arrived]);
              } else {
                setTransactions(loaded);
              }
              console.info(`자동 세션 복원 완료: ${loaded.length}개 트랜잭션`);
            }
          } catch (e) {
            console.error("자동 세션 복원 파싱 실패:", e);
          }
        }
      })
      .catch((e) => {
        console.error("자동 세션 복원 실패:", e);
      });
  }, [setTransactions]);

  // 자동 세션 저장 로직
  const performAutosave = useCallback(async () => {
    const autoSessionEnabled = useAppSettingsStore.getState().autosaveSession;
    if (!autoSessionEnabled) return;

    try {
      const currentTransactions = useTransactionStore.getState().transactions;
      if (currentTransactions.length === 0) return;

      if (currentTransactions.length >= 5000) {
        console.warn(
          `[autosave] 트랜잭션 수가 ${currentTransactions.length}개로 많습니다. JSON 직렬화 시 UI 블로킹이 발생할 수 있습니다.`,
        );
      }

      const saveable = currentTransactions.filter((tx) => tx.request?.method !== "CONNECT");
      if (saveable.length === 0) return;

      const tuples = saveable.map((tx) => [tx.request, tx.response]);

      const t0 = performance.now();
      const transactionsJson = JSON.stringify(tuples);
      const t1 = performance.now();

      if (t1 - t0 > 100) {
        console.warn(
          `[autosave] JSON.stringify에 ${(t1 - t0).toFixed(1)}ms 소요 (트랜잭션 ${currentTransactions.length}개)`,
        );
      }

      await autosaveSession(transactionsJson);
      console.info(`자동 세션 저장 완료: ${currentTransactions.length}개 트랜잭션`);
    } catch (e) {
      console.error("자동 세션 저장 실패:", e);
    }
  }, []);

  // 윈도우 닫기(숨김) 시 자동 세션 저장
  useEffect(() => {
    const unlisten = getCurrentWindow().onCloseRequested(async () => {
      await performAutosave();
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, [performAutosave]);

  // 앱 완전 종료 시 자동 세션 저장 (트레이 메뉴 종료)
  useEffect(() => {
    const unlisten = listen("app_quit_requested", async () => {
      await performAutosave();
      // 저장 완료를 백엔드에 알려 즉시 종료할 수 있도록 함
      await emit("autosave_completed");
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, [performAutosave]);
}

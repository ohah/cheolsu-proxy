import { useEffect, useRef } from "react";
import {
  useProxyStore,
  useTransactionStore,
} from "@/shared/stores";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type { ProxyEventPayload } from "@/entities/proxy";
import { useAppSettingsStore } from "@/shared/stores/app-settings-store";
import { useInterceptRuleStore } from "@/shared/stores";
import { waitForDaemonRules } from "@/shared/stores/sync-rules";

/**
 * 프록시 초기화, 프록시 이벤트 수신, 프록시 상태 동기화, 녹화 일시정지 동기화를 담당하는 훅
 */
export function useProxyEventListeners() {
  const initializeProxy = useProxyStore((s) => s.initializeProxy);
  const setConnected = useProxyStore((s) => s.setConnected);
  const syncToProxy = useInterceptRuleStore((s) => s.syncToProxy);
  const addTransaction = useTransactionStore((s) => s.addTransaction);
  const paused = useTransactionStore((s) => s.paused);
  const setPaused = useTransactionStore((s) => s.setPaused);
  // 트레이에서 받은 이벤트로 paused가 바뀐 경우 Rust 역동기화를 스킵하기 위한 플래그
  const pausedFromTrayRef = useRef(false);

  // 앱 시작 시 프록시 초기화 → 데몬 규칙 수신 대기 → 저장된 규칙 동기화
  useEffect(() => {
    const port = useAppSettingsStore.getState().proxyPort;
    initializeProxy(port).then(() => waitForDaemonRules().then(() => syncToProxy()));
  }, [initializeProxy, syncToProxy]);

  // 프록시 이벤트를 전역적으로 수신하여 트랜잭션 store에 저장
  useEffect(() => {
    if (paused) return;

    const unlisten = listen<ProxyEventPayload>("proxy_event", (event) => {
      addTransaction(event.payload);
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, [addTransaction, paused]);

  // 트레이에서 프록시 시작/중지 시 메인 윈도우 상태 동기화
  useEffect(() => {
    const unlisten = listen<boolean>("proxy_status_changed", (event) => {
      setConnected(event.payload);
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, [setConnected]);

  // 트레이에서 녹화 토글 시 Rust 백엔드를 통해 동기화 수신
  useEffect(() => {
    const unlisten = listen<boolean>("recording_paused_changed", (event) => {
      pausedFromTrayRef.current = true;
      setPaused(event.payload);
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, [setPaused]);

  // 메인 윈도우에서 paused 변경 시 Rust 백엔드에 동기화
  // (트레이 이벤트로 인한 변경은 이미 Rust에 반영되어 있으므로 스킵)
  useEffect(() => {
    if (pausedFromTrayRef.current) {
      pausedFromTrayRef.current = false;
      return;
    }
    invoke("tray_set_recording_paused", { paused }).catch(() => {
      // 트레이 윈도우가 없는 경우 정상적으로 실패 — 무시
    });
  }, [paused]);
}

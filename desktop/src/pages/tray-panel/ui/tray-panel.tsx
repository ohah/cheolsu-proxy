import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emitTo } from "@tauri-apps/api/event";
import { LazyStore } from "@tauri-apps/plugin-store";
import {
  Circle,
  Power,
  Globe,
  Pause,
  Play,
  ShieldCheck,
  ShieldX,
  Trash2,
  FolderOpen,
  ExternalLink,
  X,
} from "lucide-react";

interface TrayInfo {
  is_connected: boolean;
  ca_installed: boolean;
  port: number;
}

const trayStore = new LazyStore("tray-sync.json");

export function TrayPanel() {
  const [info, setInfo] = useState<TrayInfo | null>(null);
  const [proxyOn, setProxyOn] = useState(false);
  const [recording, setRecording] = useState(true);
  const [transactionCount, setTransactionCount] = useState(0);

  // Rust 백엔드에서 프록시 상태 조회
  const fetchInfo = useCallback(async () => {
    try {
      const result = await invoke<TrayInfo>("tray_get_info");
      setInfo(result);
      setProxyOn(result.is_connected);
    } catch (e) {
      console.error("Failed to fetch tray info:", e);
    }
  }, []);

  // Tauri Store에서 메인 윈도우 상태 읽기
  const syncFromStore = useCallback(async () => {
    try {
      const paused = await trayStore.get<boolean>("paused");
      const count = await trayStore.get<number>("transactionCount");
      if (paused !== null && paused !== undefined) setRecording(!paused);
      if (count !== null && count !== undefined) setTransactionCount(count);
    } catch {
      // 스토어가 아직 초기화 안 된 경우 무시
    }
  }, []);

  useEffect(() => {
    fetchInfo();
    syncFromStore();

    const interval = setInterval(() => {
      fetchInfo();
      syncFromStore();
    }, 1500);

    // Store 변경 감지 (메인 윈도우에서 값 변경 시 즉시 반영)
    const unlistenPromise = trayStore.onChange<boolean | number>((key, value) => {
      if (key === "paused" && typeof value === "boolean") setRecording(!value);
      if (key === "transactionCount" && typeof value === "number") setTransactionCount(value);
    });

    return () => {
      clearInterval(interval);
      unlistenPromise.then((f) => f());
    };
  }, [fetchInfo, syncFromStore]);

  const handleToggleProxy = async () => {
    try {
      if (proxyOn) {
        await invoke("stop_proxy_v2");
        setProxyOn(false);
      } else {
        await invoke("start_proxy_v2", { addr: `127.0.0.1:${info?.port ?? 8100}` });
        setProxyOn(true);
      }
      fetchInfo();
    } catch (e) {
      console.error("Proxy toggle failed:", e);
    }
  };

  const handleToggleRecording = async () => {
    setRecording((prev) => !prev);
    await emitTo("main", "tray_toggle_pause", {});
  };

  const handleClearSession = async () => {
    await emitTo("main", "tray_clear_session", {});
    setTransactionCount(0);
  };

  const handleCleanCache = async () => {
    try {
      await invoke("clean_old_proxy_cache", { days: 1 });
    } catch (e) {
      console.error("Cache cleanup failed:", e);
    }
  };

  const handleShowMainWindow = async () => {
    await invoke("tray_show_main_window").catch(() => {});
  };

  const handleQuit = async () => {
    await invoke("tray_quit_app").catch(() => {});
  };

  const isConnected = info?.is_connected ?? false;
  const caInstalled = info?.ca_installed ?? false;
  const port = info?.port ?? 8100;

  return (
    <div className="tray-root">
      <div className="tray-panel">
        {/* 헤더 */}
        <div className="tray-header" data-tauri-drag-region>
          <div className="tray-header-left">
            <span className="tray-title">Cheolsu Proxy</span>
            <span className="tray-version">v0.1.0</span>
          </div>
          <div className="tray-header-right">
            <Circle
              size={8}
              className={isConnected ? "tray-dot-connected" : "tray-dot-disconnected"}
            />
            <span className="tray-port">:{port}</span>
          </div>
        </div>

        {/* 토글 */}
        <div className="tray-section">
          <TrayToggleRow
            icon={<Power size={14} />}
            label="프록시"
            checked={proxyOn}
            onChange={handleToggleProxy}
            activeColor="#34c759"
          />
          <TrayToggleRow
            icon={<Globe size={14} />}
            label="시스템 프록시"
            checked={false}
            onChange={() => {}}
            activeColor="#007aff"
            disabled
          />
          <TrayToggleRow
            icon={recording ? <Pause size={14} /> : <Play size={14} />}
            label="트래픽 기록"
            checked={recording}
            onChange={handleToggleRecording}
            activeColor="#ff9f0a"
          />
        </div>

        {/* 상태 */}
        <div className="tray-section tray-status-section">
          <div className="tray-status-row">
            <div className="tray-status-label">
              {caInstalled ? (
                <ShieldCheck size={13} className="tray-dot-connected" />
              ) : (
                <ShieldX size={13} style={{ color: "#ff9f0a" }} />
              )}
              <span>CA 인증서</span>
            </div>
            <span className={caInstalled ? "tray-status-ok" : "tray-status-warn"}>
              {caInstalled ? "설치됨" : "미설치"}
            </span>
          </div>
          <div className="tray-status-row">
            <span className="tray-status-label">요청 수</span>
            <span className="tray-status-count">{transactionCount}</span>
          </div>
        </div>

        {/* 액션 */}
        <div className="tray-section">
          <TrayActionButton
            icon={<ExternalLink size={13} />}
            label="메인 창 열기"
            onClick={handleShowMainWindow}
          />
          <TrayActionButton
            icon={<Trash2 size={13} />}
            label="세션 초기화"
            onClick={handleClearSession}
          />
          <TrayActionButton
            icon={<FolderOpen size={13} />}
            label="캐시 정리"
            onClick={handleCleanCache}
          />
        </div>

        {/* 종료 */}
        <div className="tray-section tray-section-last">
          <TrayActionButton
            icon={<X size={13} />}
            label="종료"
            onClick={handleQuit}
            variant="destructive"
          />
        </div>
      </div>
    </div>
  );
}

function TrayToggleRow({
  icon,
  label,
  checked,
  onChange,
  activeColor,
  disabled,
}: {
  icon: React.ReactNode;
  label: string;
  checked: boolean;
  onChange: () => void;
  activeColor: string;
  disabled?: boolean;
}) {
  return (
    <button
      className={`tray-toggle-row ${disabled ? "tray-disabled" : ""}`}
      onClick={disabled ? undefined : onChange}
      disabled={disabled}
    >
      <div className="tray-toggle-label">
        {icon}
        <span>{label}</span>
        {disabled && <span className="tray-badge-soon">준비 중</span>}
      </div>
      <div
        className="tray-switch"
        style={{ backgroundColor: checked ? activeColor : undefined }}
        data-checked={checked || undefined}
      >
        <div className="tray-switch-thumb" data-checked={checked || undefined} />
      </div>
    </button>
  );
}

function TrayActionButton({
  icon,
  label,
  onClick,
  variant,
}: {
  icon: React.ReactNode;
  label: string;
  onClick: () => void;
  variant?: "destructive";
}) {
  return (
    <button
      className={`tray-action-btn ${variant === "destructive" ? "tray-action-destructive" : ""}`}
      onClick={onClick}
    >
      {icon}
      <span>{label}</span>
    </button>
  );
}

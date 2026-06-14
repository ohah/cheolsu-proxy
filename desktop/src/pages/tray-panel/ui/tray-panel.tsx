import { useEffect, useState, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Circle,
  Power,
  ShieldCheck,
  ShieldX,
  FolderOpen,
  ExternalLink,
  X,
  Pause,
  Play,
  Activity,
} from "lucide-react";

interface TrayInfo {
  is_connected: boolean;
  ca_installed: boolean;
  port: number;
  transaction_count: number;
  recording_paused: boolean;
}

const POLL_INTERVAL_MS = 2000;

export function TrayPanel() {
  const [info, setInfo] = useState<TrayInfo | null>(null);
  const [proxyOn, setProxyOn] = useState(false);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

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

  // 패널이 보일 때 폴링 시작, 숨겨지면 정지
  useEffect(() => {
    fetchInfo();
    pollRef.current = setInterval(fetchInfo, POLL_INTERVAL_MS);

    const handleVisibility = () => {
      if (document.hidden) {
        if (pollRef.current) {
          clearInterval(pollRef.current);
          pollRef.current = null;
        }
      } else {
        fetchInfo();
        if (!pollRef.current) {
          pollRef.current = setInterval(fetchInfo, POLL_INTERVAL_MS);
        }
      }
    };

    document.addEventListener("visibilitychange", handleVisibility);

    return () => {
      if (pollRef.current) clearInterval(pollRef.current);
      document.removeEventListener("visibilitychange", handleVisibility);
    };
  }, [fetchInfo]);

  const handleToggleProxy = async () => {
    try {
      const result = await invoke<boolean>("tray_toggle_proxy", {
        addr: `127.0.0.1:${info?.port ?? 8100}`,
      });
      setProxyOn(result);
      fetchInfo();
    } catch (e) {
      console.error("Proxy toggle failed:", e);
    }
  };

  const handleToggleRecording = async () => {
    try {
      await invoke("tray_toggle_recording");
      fetchInfo();
    } catch (e) {
      console.error("Recording toggle failed:", e);
    }
  };

  const handleCleanCache = async () => {
    try {
      await invoke("clean_old_proxy_cache", { days: 1 });
    } catch (e) {
      console.error("Cache cleanup failed:", e);
    }
  };

  const handleShowMainWindow = async () => {
    await invoke("tray_show_main_window").catch((e) => {
      console.error("Failed to show main window:", e);
    });
  };

  const handleQuit = async () => {
    await invoke("tray_quit_app").catch((e) => {
      console.error("Failed to quit app:", e);
    });
  };

  const isConnected = info?.is_connected ?? false;
  const caInstalled = info?.ca_installed ?? false;
  const port = info?.port ?? 8100;
  const transactionCount = info?.transaction_count ?? 0;
  const recordingPaused = info?.recording_paused ?? false;

  return (
    <div className="tray-root">
      <div className="tray-panel">
        {/* 헤더 */}
        <div className="tray-header" data-tauri-drag-region>
          <div className="tray-header-left">
            <span className="tray-title">Cheolsu Proxy</span>
            <span className="tray-version">v0.1.2</span>
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
            icon={recordingPaused ? <Play size={14} /> : <Pause size={14} />}
            label="녹화"
            checked={!recordingPaused}
            onChange={handleToggleRecording}
            activeColor="#007aff"
            disabled={!isConnected}
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
            <div className="tray-status-label">
              <Activity size={13} className="tray-text-secondary" />
              <span>트랜잭션</span>
            </div>
            <span className="tray-status-count">{transactionCount.toLocaleString()}</span>
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

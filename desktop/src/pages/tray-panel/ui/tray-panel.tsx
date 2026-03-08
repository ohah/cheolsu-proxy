import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emitTo } from "@tauri-apps/api/event";
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

export function TrayPanel() {
  const [info, setInfo] = useState<TrayInfo | null>(null);
  const [proxyOn, setProxyOn] = useState(false);
  const [systemProxyOn, setSystemProxyOn] = useState(false);
  const [recording, setRecording] = useState(true);
  const [transactionCount, setTransactionCount] = useState(0);

  const fetchInfo = useCallback(async () => {
    try {
      const result = await invoke<TrayInfo>("tray_get_info");
      setInfo(result);
      setProxyOn(result.is_connected);
    } catch (e) {
      console.error("Failed to fetch tray info:", e);
    }
  }, []);

  useEffect(() => {
    fetchInfo();
    const interval = setInterval(fetchInfo, 2000);
    return () => clearInterval(interval);
  }, [fetchInfo]);

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
    try {
      await invoke("tray_show_main_window");
    } catch (e) {
      console.error("Show main window failed:", e);
    }
  };

  const handleQuit = async () => {
    try {
      await invoke("tray_quit_app");
    } catch (e) {
      console.error("Quit failed:", e);
    }
  };

  const isConnected = info?.is_connected ?? false;
  const caInstalled = info?.ca_installed ?? false;
  const port = info?.port ?? 8100;

  return (
    <div className="h-full w-full bg-background text-foreground select-none overflow-hidden rounded-lg border border-border shadow-xl">
      {/* 헤더 */}
      <div
        className="flex items-center justify-between px-4 py-3 border-b border-border bg-muted/30"
        data-tauri-drag-region
      >
        <div className="flex items-center gap-2">
          <span className="font-semibold text-sm">Cheolsu Proxy</span>
          <span className="text-xs text-muted-foreground">v0.1.0</span>
        </div>
        <div className="flex items-center gap-1.5">
          <Circle
            className={`w-2.5 h-2.5 ${
              isConnected ? "text-green-500 fill-green-500" : "text-red-500 fill-red-500"
            }`}
          />
          <span className="text-xs text-muted-foreground">:{port}</span>
        </div>
      </div>

      {/* 토글 섹션 */}
      <div className="px-4 py-2 space-y-1">
        <TrayToggleRow
          icon={<Power className="w-4 h-4" />}
          label="프록시"
          checked={proxyOn}
          onChange={handleToggleProxy}
          activeColor="green"
        />
        <TrayToggleRow
          icon={<Globe className="w-4 h-4" />}
          label="시스템 프록시"
          checked={systemProxyOn}
          onChange={() => setSystemProxyOn(!systemProxyOn)}
          activeColor="blue"
          disabled
          disabledTooltip="준비 중"
        />
        <TrayToggleRow
          icon={recording ? <Pause className="w-4 h-4" /> : <Play className="w-4 h-4" />}
          label="트래픽 기록"
          checked={recording}
          onChange={handleToggleRecording}
          activeColor="amber"
        />
      </div>

      {/* 상태 정보 */}
      <div className="px-4 py-2 border-t border-border">
        <div className="space-y-2">
          <div className="flex items-center justify-between text-xs">
            <div className="flex items-center gap-2 text-muted-foreground">
              {caInstalled ? (
                <ShieldCheck className="w-3.5 h-3.5 text-green-500" />
              ) : (
                <ShieldX className="w-3.5 h-3.5 text-yellow-500" />
              )}
              <span>CA 인증서</span>
            </div>
            <span className={caInstalled ? "text-green-600" : "text-yellow-600"}>
              {caInstalled ? "설치됨" : "미설치"}
            </span>
          </div>
          <div className="flex items-center justify-between text-xs">
            <span className="text-muted-foreground">요청 수</span>
            <span className="font-mono">{transactionCount}</span>
          </div>
        </div>
      </div>

      {/* 액션 버튼 */}
      <div className="px-4 py-2 border-t border-border space-y-1">
        <TrayActionButton
          icon={<ExternalLink className="w-3.5 h-3.5" />}
          label="메인 창 열기"
          onClick={handleShowMainWindow}
        />
        <TrayActionButton
          icon={<Trash2 className="w-3.5 h-3.5" />}
          label="세션 초기화"
          onClick={handleClearSession}
        />
        <TrayActionButton
          icon={<FolderOpen className="w-3.5 h-3.5" />}
          label="캐시 정리"
          onClick={handleCleanCache}
        />
      </div>

      {/* 종료 */}
      <div className="px-4 py-2 border-t border-border">
        <TrayActionButton
          icon={<X className="w-3.5 h-3.5" />}
          label="종료"
          onClick={handleQuit}
          variant="destructive"
        />
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
  disabledTooltip,
}: {
  icon: React.ReactNode;
  label: string;
  checked: boolean;
  onChange: () => void;
  activeColor: string;
  disabled?: boolean;
  disabledTooltip?: string;
}) {
  const colorMap: Record<string, string> = {
    green: "bg-green-500",
    blue: "bg-blue-500",
    amber: "bg-amber-500",
  };

  return (
    <button
      className={`w-full flex items-center justify-between px-3 py-2 rounded-md text-sm transition-colors ${
        disabled ? "opacity-50 cursor-not-allowed" : "hover:bg-muted cursor-pointer"
      }`}
      onClick={disabled ? undefined : onChange}
      disabled={disabled}
      title={disabled ? disabledTooltip : undefined}
    >
      <div className="flex items-center gap-2.5">
        {icon}
        <span>{label}</span>
      </div>
      <div
        className={`w-8 h-[18px] rounded-full transition-colors relative ${
          checked ? colorMap[activeColor] || "bg-primary" : "bg-zinc-300 dark:bg-zinc-600"
        }`}
      >
        <div
          className={`absolute top-[2px] w-[14px] h-[14px] rounded-full bg-white shadow transition-transform ${
            checked ? "translate-x-[16px]" : "translate-x-[2px]"
          }`}
        />
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
      className={`w-full flex items-center gap-2.5 px-3 py-2 rounded-md text-sm transition-colors cursor-pointer ${
        variant === "destructive" ? "hover:bg-red-500/10 text-red-500" : "hover:bg-muted"
      }`}
      onClick={onClick}
    >
      {icon}
      <span>{label}</span>
    </button>
  );
}

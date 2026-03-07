import { create } from "zustand";
import { persist } from "zustand/middleware";

import { startProxyV2, cleanOldProxyCache } from "@/shared/api/proxy";
import { toast } from "sonner";

interface ProxyState {
  isConnected: boolean;
  isInitialized: boolean;
  port: number;
  initializeProxy: (port?: number) => Promise<void>;
  setConnected: (connected: boolean) => void;
  setPort: (port: number) => void;
}

export const useProxyStore = create<ProxyState>()(
  persist(
    (set, get) => ({
      isConnected: false,
      isInitialized: false,
      port: 8100,

      initializeProxy: async (port: number = 8100) => {
        const { isInitialized } = get();

        // 이미 초기화되었으면 중복 실행 방지
        if (isInitialized) return;

        set({ isInitialized: true, port });

        try {
          // 앱 시작 시 오래된 캐시 정리 (1일 이상)
          try {
            await cleanOldProxyCache(1);
            console.log("🧹 오래된 캐시가 정리되었습니다");
          } catch (cacheError) {
            console.warn("⚠️ 캐시 정리 실패:", cacheError);
          }

          await startProxyV2(port);
          set({ isConnected: true });
          toast.success("Proxy started successfully");
        } catch (error) {
          console.error("Failed to start proxy:", error);
          set({ isConnected: false, isInitialized: false });
          toast.error("Failed to start proxy");
        }
      },

      setConnected: (connected: boolean) => set({ isConnected: connected }),
      setPort: (port: number) => set({ port }),
    }),
    {
      name: "cheolsu-proxy-store",
      // isInitialized는 persist하지 않음 (앱 재시작 시 다시 초기화되어야 함)
      partialize: (state) => ({
        isConnected: state.isConnected,
        port: state.port,
      }),
    },
  ),
);

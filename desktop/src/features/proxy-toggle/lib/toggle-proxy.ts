import { useProxyStore } from "@/shared/stores";
import { startProxyV2, stopProxyV2 } from "@/shared/api/proxy";
import { toast } from "sonner";
import { trayStore } from "@/shared/stores/tray-sync-store";

export async function toggleProxy() {
  const { isConnected, port } = useProxyStore.getState();

  try {
    if (isConnected) {
      await stopProxyV2();
      useProxyStore.getState().setConnected(false);
      await trayStore.set("proxyConnected", false);
      await trayStore.save();
      toast.info("Proxy stopped");
    } else {
      await startProxyV2(port);
      useProxyStore.getState().setConnected(true);
      await trayStore.set("proxyConnected", true);
      await trayStore.save();
      toast.success("Proxy started");
    }
  } catch {
    toast.error("Proxy toggle failed");
  }
}

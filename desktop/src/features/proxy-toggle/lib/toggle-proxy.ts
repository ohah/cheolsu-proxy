import { useProxyStore } from "@/shared/stores";
import { startProxyV2, stopProxyV2 } from "@/shared/api/proxy";
import { toast } from "sonner";

export async function toggleProxy() {
  const { isConnected, port } = useProxyStore.getState();

  try {
    if (isConnected) {
      await stopProxyV2();
      useProxyStore.getState().setConnected(false);
      toast.info("Proxy stopped");
    } else {
      await startProxyV2(port);
      useProxyStore.getState().setConnected(true);
      toast.success("Proxy started");
    }
  } catch {
    toast.error("Proxy toggle failed");
  }
}

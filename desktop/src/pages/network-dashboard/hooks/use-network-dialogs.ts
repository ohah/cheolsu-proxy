import { useState } from "react";
import type { HttpTransaction } from "@/entities/proxy";

export function useNetworkDialogs() {
  const [sequenceReplayOpen, setSequenceReplayOpen] = useState(false);
  const [composeOpen, setComposeOpen] = useState(false);
  const [advancedRepeatTarget, setAdvancedRepeatTarget] = useState<HttpTransaction | null>(null);

  return {
    sequenceReplayOpen,
    setSequenceReplayOpen,
    composeOpen,
    setComposeOpen,
    advancedRepeatTarget,
    setAdvancedRepeatTarget,
  };
}

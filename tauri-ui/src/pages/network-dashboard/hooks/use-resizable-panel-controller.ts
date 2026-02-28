import { useEffect, useRef } from "react";
import type { ImperativePanelHandle } from "react-resizable-panels";

interface UseResizablePanelControllerProps {
  isExpanded: boolean;
}

export const useResizablePanelController = ({ isExpanded }: UseResizablePanelControllerProps) => {
  const panelRef = useRef<ImperativePanelHandle>(null);

  useEffect(() => {
    const panel = panelRef.current;
    if (!panel) return;

    if (isExpanded) {
      panel.expand();
    } else {
      panel.collapse();
    }
  }, [isExpanded]);

  return panelRef;
};

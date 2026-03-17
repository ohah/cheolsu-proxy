import { forwardRef, useMemo, useState, useCallback } from "react";

import { useVirtualizer } from "@tanstack/react-virtual";

import { ScrollArea } from "./scroll-area";

interface VirtualizedScrollAreaProps {
  itemCount: number;
  itemSize: number;
  overscan?: number;
  className?: string;
  renderItem: (index: number) => React.ReactNode;
}

export const VirtualizedScrollArea = forwardRef<HTMLDivElement, VirtualizedScrollAreaProps>(
  ({ itemCount, itemSize, overscan = 10, className, renderItem }, _ref) => {
    const [viewport, setViewport] = useState<HTMLDivElement | null>(null);

    const scrollAreaRef = useCallback((node: HTMLDivElement | null) => {
      if (node) {
        const vp = node.querySelector(
          '[data-slot="scroll-area-viewport"]',
        ) as HTMLDivElement | null;
        setViewport(vp);
      }
    }, []);

    const getScrollElement = useCallback(() => viewport, [viewport]);
    const estimateSize = useCallback(() => itemSize, [itemSize]);

    const virtualizer = useVirtualizer({
      count: itemCount,
      getScrollElement,
      estimateSize,
      overscan,
    });

    const containerStyle = useMemo(
      () => ({
        height: `${virtualizer.getTotalSize()}px`,
        width: "100%",
        position: "relative" as const,
      }),
      [virtualizer.getTotalSize()],
    );

    const virtualItems = virtualizer.getVirtualItems();

    return (
      <ScrollArea ref={scrollAreaRef} className={`${className} h-full min-h-0`}>
        <div style={containerStyle}>
          {virtualItems.map((virtualItem) => {
            const itemStyle: React.CSSProperties = {
              position: "absolute",
              top: 0,
              left: 0,
              width: "100%",
              height: `${virtualItem.size}px`,
              transform: `translateY(${virtualItem.start}px)`,
            };

            return (
              <div key={virtualItem.key} style={itemStyle}>
                {renderItem(virtualItem.index)}
              </div>
            );
          })}
        </div>
      </ScrollArea>
    );
  },
);

VirtualizedScrollArea.displayName = "VirtualizedScrollArea";

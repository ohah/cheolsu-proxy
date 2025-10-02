import { useRef, forwardRef, useMemo, useEffect, useCallback } from 'react';

import { useVirtualizer } from '@tanstack/react-virtual';

import { ScrollArea } from './scroll-area';

interface VirtualizedScrollAreaProps {
  itemCount: number;
  itemSize: number;
  overscan?: number;
  className?: string;
  renderItem: (index: number) => React.ReactNode;
}

export const VirtualizedScrollArea = forwardRef<HTMLDivElement, VirtualizedScrollAreaProps>(
  ({ itemCount, itemSize, overscan = 10, className, renderItem }, _ref) => {
    const scrollAreaRef = useRef<HTMLDivElement>(null);
    const viewportRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
      if (scrollAreaRef.current) {
        const viewport = scrollAreaRef.current.querySelector('[data-slot="scroll-area-viewport"]') as HTMLDivElement;
        if (viewport) {
          viewportRef.current = viewport;
        }
      }
    }, []);

    const getScrollElement = useCallback(() => viewportRef.current, []);
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
        width: '100%',
        position: 'relative' as const,
      }),
      [virtualizer.getTotalSize()],
    );

    const virtualItems = virtualizer.getVirtualItems();

    return (
      <ScrollArea ref={scrollAreaRef} className={`${className} h-full min-h-0`}>
        <div style={containerStyle}>
          {virtualItems.map((virtualItem) => {
            const itemStyle: React.CSSProperties = {
              position: 'absolute',
              top: 0,
              left: 0,
              width: '100%',
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

VirtualizedScrollArea.displayName = 'VirtualizedScrollArea';

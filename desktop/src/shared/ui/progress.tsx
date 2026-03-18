import * as React from "react";

import { cn } from "@/shared/lib";

function Progress({
  className,
  value = 0,
  ...props
}: React.ComponentProps<"div"> & { value?: number }) {
  return (
    <div
      data-slot="progress"
      role="progressbar"
      aria-valuenow={value}
      aria-valuemin={0}
      aria-valuemax={100}
      className={cn("h-2 w-full rounded-full bg-muted overflow-hidden", className)}
      {...props}
    >
      <div
        className="h-full rounded-full bg-primary transition-all duration-150"
        style={{ width: `${Math.min(100, Math.max(0, value))}%` }}
      />
    </div>
  );
}

export { Progress };

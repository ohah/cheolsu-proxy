import type { ReactNode } from "react";

interface NetworkFiltersProps {
  children: ReactNode;
}

export const NetworkFilters = ({ children }: NetworkFiltersProps) => {
  return <div className="flex items-center gap-3 flex-1 min-w-0 h-[36px] w-full">{children}</div>;
};

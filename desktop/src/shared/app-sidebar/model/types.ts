import type { LucideIcon } from "lucide-react";
import type { MessageDescriptor } from "@lingui/core";

export interface SidebarSection {
  id: string;
  label: MessageDescriptor;
  icon: LucideIcon;
  description?: MessageDescriptor;
}

export interface SidebarCounts {
  [sectionId: string]: number;
}

import type { LucideIcon } from 'lucide-react';

export interface SidebarSection {
  id: string;
  label: string;
  icon: LucideIcon;
  description?: string;
}

export interface SidebarCounts {
  [sectionId: string]: number;
}


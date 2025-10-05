import { useSidebarStore } from '../model';

export const useSidebarCollapse = () => {
  const { collapsed, toggleCollapse } = useSidebarStore();

  return { collapsed, toggleCollapse };
};

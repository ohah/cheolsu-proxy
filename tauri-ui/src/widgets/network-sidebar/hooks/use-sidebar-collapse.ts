import { useState } from "react";

export const useSidebarCollapse = () => {
  const [collapsed, setCollapsed] = useState(false);

  const toggleCollapse = () => {
    setCollapsed(!collapsed);
  };

  return { collapsed, toggleCollapse };
}

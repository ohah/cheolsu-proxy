import { useState, useCallback } from 'react';

import { DEFAULT_ACTIVE_SECTION } from '../model';

interface UseSidebarNavigationProps {
  initialSection?: string;
  onSectionChange?: (sectionId: string) => void;
}

export const useSidebarNavigation = ({
  initialSection = DEFAULT_ACTIVE_SECTION,
  onSectionChange
}: UseSidebarNavigationProps = {}) => {
  const [activeSection, setActiveSection] = useState(initialSection);

  const createSelectionChangeHandler = useCallback((sectionId: string) => () => {
    setActiveSection(sectionId);
    onSectionChange?.(sectionId);
  }, [onSectionChange]);

  return { activeSection, createSelectionChangeHandler };
};

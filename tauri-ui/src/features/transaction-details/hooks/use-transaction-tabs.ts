import { useState, useMemo } from 'react';

import { TRANSACTION_DETAILS_TABS } from '../model';
import type { TransactionTab } from '../model';

export const useTransactionTabs = () => {
  const [activeTab, setActiveTab] = useState<TransactionTab>(TRANSACTION_DETAILS_TABS.HEADERS);

  const tabs = useMemo(() => {
    return Object.values(TRANSACTION_DETAILS_TABS) as TransactionTab[];
  }, []);

  const handleTabChange = (tab: string) => {
    setActiveTab(tab as TransactionTab);
  };

  return { activeTab, tabs, onTabChange: handleTabChange };
};

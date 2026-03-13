import { useState, useMemo } from "react";

import { TRANSACTION_DETAILS_TABS, TRANSACTION_DETAILS_BOTTOM_TABS } from "../model";
import type { TransactionTab, TransactionBottomTab } from "../model";

export const useTransactionTabs = (layout: "right" | "bottom" = "right") => {
  const [activeTab, setActiveTab] = useState<string>(TRANSACTION_DETAILS_TABS.HEADERS);

  const tabs = useMemo(() => {
    if (layout === "bottom") {
      return Object.values(TRANSACTION_DETAILS_BOTTOM_TABS) as TransactionBottomTab[];
    }
    return Object.values(TRANSACTION_DETAILS_TABS) as TransactionTab[];
  }, [layout]);

  const handleTabChange = (tab: string) => {
    setActiveTab(tab);
  };

  return { activeTab, tabs, onTabChange: handleTabChange };
};

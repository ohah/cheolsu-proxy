import type { ReactNode } from 'react';

interface TabViewProps {
  tabs: {
    id: string;
    label: string;
    content: ReactNode;
  }[];
  activeTab: string;
  onTabChange: (tabId: string) => void;
  className?: string;
}

export const TabView = ({ tabs, activeTab, onTabChange, className = '' }: TabViewProps) => {
  return (
    <div className={className}>
      <div className="tab-bar">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            className={activeTab === tab.id ? 'tab_selected' : ''}
            onClick={() => onTabChange(tab.id)}
          >
            {tab.label}
          </button>
        ))}
      </div>
      <div className="tab-content">{tabs.find((tab) => tab.id === activeTab)?.content}</div>
    </div>
  );
};

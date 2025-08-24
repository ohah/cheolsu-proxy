import { useState } from 'react';
import type { ProxiedRequest, ProxiedResponse } from '../../../entities/request';
import { TabView } from '../../../shared/ui/tab-view';
import { RequestTab } from './request-tab';
import { ResponseTab } from './response-tab';

interface RequestDetailsProps {
  request: ProxiedRequest;
  response: ProxiedResponse;
  onDeselect: () => void;
}

export const RequestDetails = ({ request, response, onDeselect }: RequestDetailsProps) => {
  const [activeTab, setActiveTab] = useState('request');

  const tabs = [
    {
      id: 'request',
      label: 'Request',
      content: <RequestTab request={request} />,
    },
    {
      id: 'response',
      label: 'Response',
      content: <ResponseTab response={response} />,
    },
  ];

  return (
    <div>
      <div className="modal-background" onClick={onDeselect} />
      <div className="modal-content">
        <button className="close_button" onClick={onDeselect} type="button">
          ×
        </button>
        <TabView tabs={tabs} activeTab={activeTab} onTabChange={setActiveTab} />
      </div>
    </div>
  );
};

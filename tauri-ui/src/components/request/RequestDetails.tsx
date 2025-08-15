
import React, { useState } from 'react';
import { ProxiedRequest, ProxiedResponse } from '../../types';
import RequestTab from './RequestTab';
import ResponseTab from './ResponseTab';

enum Tab {
  Request,
  Response,
}

interface RequestDetailsProps {
  request: ProxiedRequest;
  response: ProxiedResponse;
  onDeselect: () => void;
}

const RequestDetails: React.FC<RequestDetailsProps> = ({ request, response, onDeselect }) => {
  const [activeTab, setActiveTab] = useState<Tab>(Tab.Request);

  return (
    <div>
      <div className="modal-background" onClick={onDeselect}></div>
      <div className="modal-content">
        <button className="close_button" onClick={onDeselect}>×</button>
        <div className="tab-bar">
          <button
            className={activeTab === Tab.Request ? 'tab_selected' : ''}
            onClick={() => setActiveTab(Tab.Request)}
          >
            Request
          </button>
          <button
            className={activeTab === Tab.Response ? 'tab_selected' : ''}
            onClick={() => setActiveTab(Tab.Response)}
          >
            Response
          </button>
        </div>
        {activeTab === Tab.Request ? (
          <RequestTab request={request} />
        ) : (
          <ResponseTab response={response} />
        )}
      </div>
    </div>
  );
};

export default RequestDetails;

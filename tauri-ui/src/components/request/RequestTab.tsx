
import React from 'react';
import { ProxiedRequest } from '../../types';
import TabView from './TabView';

interface RequestTabProps {
  request: ProxiedRequest;
}

const RequestTab: React.FC<RequestTabProps> = ({ request }) => {
  return (
    <TabView headers={request.headers} body={request.body}>
      <div className="single_header">
        <strong>Method:</strong>
        <p>{request.method}</p>
      </div>
      <div className="single_header">
        <strong>Version:</strong>
        <p>{request.version}</p>
      </div>
      <div className="single_header">
        <strong>Timestamp:</strong>
        <p>{new Date(request.time / 1_000_000).toISOString()}</p>
      </div>
    </TabView>
  );
};

export default RequestTab;

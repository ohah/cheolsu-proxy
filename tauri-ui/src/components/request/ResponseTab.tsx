
import React from 'react';
import { ProxiedResponse } from '../../types';
import TabView from './TabView';

interface ResponseTabProps {
  response: ProxiedResponse;
}

const ResponseTab: React.FC<ResponseTabProps> = ({ response }) => {
  return (
    <TabView headers={response.headers} body={response.body}>
      <div className="single_header">
        <strong>Status:</strong>
        <p>{response.status}</p>
      </div>
      <div className="single_header">
        <strong>Version:</strong>
        <p>{response.version}</p>
      </div>
      <div className="single_header">
        <strong>Timestamp:</strong>
        <p>{new Date(response.time / 1_000_000).toISOString()}</p>
      </div>
    </TabView>
  );
};

export default ResponseTab;

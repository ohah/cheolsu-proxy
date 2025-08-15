import React, { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { RequestInfo } from '../../types';
import MultipleSelectInput from '../MultipleSelectInput';
import RequestRow from './RequestRow';
import RequestDetails from './RequestDetails';

const OPTIONS = [
  "POST", "GET", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS", "CONNECT", "TRACE", "OTHERS",
];

interface RequestTableProps {
  paused: boolean;
}

const filterRequest = (method: string, filters: string[]): boolean => {
  return filters.includes(method) || (!OPTIONS.includes(method) && filters.includes("OTHERS"));
};

const RequestTable: React.FC<RequestTableProps> = ({ paused }) => {
  const [requests, setRequests] = useState<RequestInfo[]>([]);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [filters, setFilters] = useState<string[]>(OPTIONS);

  useEffect(() => {
    if (paused) return;

    const unlisten = listen<RequestInfo>('proxy_event', (event) => {
      const [request, response] = event.payload;
      if (!requests.find(r => r.request?.time === request?.time)) {
        setRequests(prevRequests => [...prevRequests, {request, response}]);
      }
    });

    return () => {
      unlisten.then(f => f());
    };
  }, [paused]);

  const handleFilterChange = (newFilters: string[]) => {
    if (selectedId !== null) {
        const selectedRequest = requests[selectedId];
        if (selectedRequest?.request && !filterRequest(selectedRequest.request.method, newFilters)) {
            setSelectedId(null);
        }
    }
    setFilters(newFilters);
  };

  const handleDelete = (id: number) => {
    setRequests(prev => prev.filter((_, i) => i !== id));
    if (selectedId === id) {
      setSelectedId(null);
    }
  };

  const handleSelect = (id: number) => {
    setSelectedId(id);
  };

  const handleDeselect = () => {
    setSelectedId(null);
  };

  const filteredRequests = requests.filter(exchange => 
    exchange.request ? filterRequest(exchange.request.method, filters) : false
  );

  if (requests.length === 0) {
    return <div className="loader" />;
  }

  return (
    <div className="request-table-container">
      <table className="request-table">
        <thead>
          <tr>
            <th>Path</th>
            <th>
              Method ↓
              <MultipleSelectInput 
                options={OPTIONS} 
                selectedOptions={filters} 
                onChange={handleFilterChange} 
              />
            </th>
            <th>Status</th>
            <th>Size</th>
            <th>Time</th>
            <th>Action</th>
          </tr>
        </thead>
        <tbody>
          {filteredRequests.map((exchange, idx) => (
            <RequestRow
              key={exchange.request?.time ?? idx} // use time as a key if available
              idx={requests.indexOf(exchange)} // pass original index
              exchange={exchange}
              onDelete={handleDelete}
              onSelect={handleSelect}
            />
          ))}
        </tbody>
      </table>
      {selectedId !== null && requests[selectedId]?.request && requests[selectedId]?.response && (
        <RequestDetails
          request={requests[selectedId].request!}
          response={requests[selectedId].response!}
          onDeselect={handleDeselect}
        />
      )}
    </div>
  );
};

export default RequestTable;

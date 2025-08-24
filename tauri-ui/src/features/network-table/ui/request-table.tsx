import { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import type { RequestInfo } from '../../../entities/request';
import { HTTP_METHOD_OPTIONS, type HttpMethod, filterRequest } from '../model';
import { RequestRow } from './request-row';
import { RequestDetails } from './request-details';
import { MultipleSelectInput } from '../../../shared/ui/multiple-select-input';

interface RequestTableProps {
  paused: boolean;
}

export const RequestTable = ({ paused }: RequestTableProps) => {
  const [requests, setRequests] = useState<RequestInfo[]>([]);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [filters, setFilters] = useState<HttpMethod[]>([...HTTP_METHOD_OPTIONS]);

  useEffect(() => {
    if (paused) return;

    const unlisten = listen<RequestInfo>('proxy_event', (event) => {
      const [request, response] = event.payload;
      if (!requests.find((r) => r.request?.time === request?.time)) {
        setRequests((prevRequests) => [...prevRequests, { request, response }]);
      }
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, [paused, requests]);

  const handleFilterChange = (newFilters: HttpMethod[]) => {
    if (selectedId !== null) {
      const selectedRequest = requests[selectedId];
      if (selectedRequest?.request && !filterRequest(selectedRequest.request.method, newFilters)) {
        setSelectedId(null);
      }
    }
    setFilters(newFilters);
  };

  const handleDelete = (id: number) => {
    setRequests((prev) => prev.filter((_, i) => i !== id));
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

  const filteredRequests = requests.filter((exchange) =>
    exchange.request ? filterRequest(exchange.request.method, filters) : false,
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
              Method ...
              <MultipleSelectInput
                options={HTTP_METHOD_OPTIONS}
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
              key={exchange.request?.time ?? idx}
              exchange={exchange}
              idx={requests.indexOf(exchange)}
              onDelete={handleDelete}
              onSelect={handleSelect}
            />
          ))}
        </tbody>
      </table>
      {selectedId !== null && requests[selectedId]?.request && requests[selectedId]?.response && (
        <RequestDetails
          request={requests[selectedId].request}
          response={requests[selectedId].response}
          onDeselect={handleDeselect}
        />
      )}
    </div>
  );
};

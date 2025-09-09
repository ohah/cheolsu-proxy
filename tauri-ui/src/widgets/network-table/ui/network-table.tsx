'use client';

import { Badge, Button } from '@/shared/ui';
import type {RequestInfo} from '@/entities/proxy';
import { getStatusColor, getMethodColor, getAuthority } from '../lib';
import { Trash2 } from 'lucide-react';

interface NetworkTableProps {
  requests: RequestInfo[];
  selectedRequest: RequestInfo | null;
  onSelectRequest: (request: RequestInfo) => () => void;
}

export function NetworkTable({ requests, selectedRequest, onSelectRequest }: NetworkTableProps) {
  return (
    <div className="flex flex-col flex-1s h-full">
      <div className="border-b border-border bg-muted/50">
        <div className="grid grid-cols-10 gap-4 p-3 text-xs font-medium text-muted-foreground uppercase tracking-wide">
          <div className="col-span-5">Path</div>
          <div className="col-span-1">Method</div>
          <div className="col-span-1">Status</div>
          <div className="col-span-1">Size</div>
          <div className="col-span-1">Time</div>
          <div className="col-span-1">Action</div>
        </div>
      </div>

      <div className="flex-1 overflow-auto">
        {requests.map((exchange, index) => {
          const { request, response } = exchange;

          if (!request || !response) {
            return <div>Parsing Failed</div>;
          }

          const timeDiff = response.time && request.time ? Math.trunc((response.time - request.time) / 1e6) : 'N/A';

          return (
            <div
              key={request.time ?? index}
              className={`grid grid-cols-10 gap-4 p-3 border-b border-border cursor-pointer hover:bg-muted/50 transition-colors ${
                selectedRequest?.request?.time === exchange.request?.time ? 'bg-accent/10 border-l-4 border-l-accent' : ''
              }`}
              onClick={onSelectRequest(exchange)}
            >
              <div className='col-span-5 flex flex-col gap-1'>
                <div className="font-mono text-sm truncate">{getAuthority(request.uri)}</div>
                <div className="font-mono text-sm truncate text-gray-500">{new URL(request.uri).pathname}</div>
              </div>
              
              <div className="col-span-1">
                <Badge variant="outline" className={`text-xs ${getMethodColor(request.method)}`}>
                  {request.method}
                </Badge>
              </div>

              <div className="col-span-1">
                <Badge variant="outline" className={`text-xs ${getStatusColor(response.status)}`}>
                  {response.status}
                </Badge>
              </div>

              <div className="col-span-1 text-sm font-mono">{request.body.length}</div>

              <div className="col-span-1 text-sm font-mono">{timeDiff}</div>

              <div className="col-span-1 text-sm text-muted-foreground">
                <Button variant="outline" size="sm">
                  <Trash2 />
                </Button>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

import { useEffect, useRef, useState } from 'react';

import type { ProxyEventRequestInfo, RequestInfo } from '@/entities/proxy';
import { NetworkHeader } from '@/widgets/network-header';
import { NetworkSidebar } from '@/widgets/network-sidebar';
import { NetworkDetails } from '@/features/request-details';
import { listen } from '@tauri-apps/api/event';
import { startProxy } from '@/shared/api/proxy';
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from '@/shared/ui/resizable';
import { getAuthority, NetworkTable } from '@/widgets/network-table';

export const NetworkDashboard = () => {
  const isInit = useRef(false);

  const [requests, setRequests] = useState<RequestInfo[]>([]);
  const [selectedRequest, setSelectedRequest] = useState<RequestInfo | null>(null);

  const [pathFilter, setPathFilter] = useState<string>('');
  const [methodFilter, setMethodFilter] = useState<string[]>([]);
  const [statusFilter, setStatusFilter] = useState<string[]>([]);
  const [paused, setPaused] = useState<boolean>(false);

  console.log('methodFilter: ', methodFilter);
  console.log('statusFilter: ', statusFilter);

  // const filteredRequests = useMemo(() => {
  //   return requests.filter((exchange) => {
  //     const authority = getAuthority(exchange.request?.uri ?? '');

  //     const matchesPath = authority.includes(pathFilter);
  //     const matchesMethod = methodFilter.length === 0 ? true : methodFilter.includes(exchange.request?.method ?? '');

  //     return matchesPath && matchesMethod && matchesStatus;
  //   }
  // }, []);

  useEffect(() => {
    if (isInit.current) return;

    const startProxyFn = async () => {
      try {
        await startProxy('127.0.0.1:8100');
      } catch (error) {
        console.log('error: ', error);
      }
    };

    isInit.current = true;
    startProxyFn();
  }, []);

  useEffect(() => {
    if (paused) return;

    const unlisten = listen<ProxyEventRequestInfo>('proxy_event', (event) => {
      const [request, response] = event.payload;
      if (!requests.find((r) => r.request?.time === request?.time)) {
        setRequests((prevRequests) => [...prevRequests, { request, response }]);
      }
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, [paused]);

  const clearRequests = () => {
    setRequests([]);
  };

  const createRequestSelectHandler = (request: RequestInfo) => () => {
    setSelectedRequest(request);
  };

  const handleDelete = (id: number) => {
    setRequests((prev) => prev.filter((_, i) => i !== id));
    if (selectedRequest?.request?.time === id) {
      setSelectedRequest(null);
    }
  };

  return (
    <div className="flex h-[100vh] w-full bg-background">
      <NetworkSidebar />

      <div className="flex-1 flex flex-col h-full">
        <NetworkHeader
          paused={paused}
          setPaused={setPaused}
          clearRequests={clearRequests}
          filter={pathFilter}
          onFilterChange={setPathFilter}
          statusFilter={statusFilter}
          onStatusFilterChange={setStatusFilter}
          methodFilter={methodFilter}
          onMethodFilterChange={setMethodFilter}
          requestCount={requests.length}
        />

        <ResizablePanelGroup direction="horizontal" className="flex-1 flex overflow-hidden">
          <ResizablePanel className="flex-1 h-full overflow-hidden border-r border-border">
            <NetworkTable
              requests={requests}
              selectedRequest={selectedRequest}
              onSelectRequest={createRequestSelectHandler}
            />
          </ResizablePanel>
          <ResizableHandle />
          {selectedRequest && (
            <ResizablePanel className="w-96 h-full overflow-y-auto">
              <NetworkDetails exchange={selectedRequest} onClose={() => setSelectedRequest(null)} />
            </ResizablePanel>
          )}
        </ResizablePanelGroup>
      </div>
    </div>
  );
};

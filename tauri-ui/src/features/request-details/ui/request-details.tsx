'use client';

import { useState } from 'react';
import {
  Badge,
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from '@/shared/ui';
import type { RequestInfo } from '@/entities/proxy';
import { X, Copy, Download } from 'lucide-react';
import { getStatusColor } from '@/widgets/network-table';

interface NetworkDetailsProps {
  exchange: RequestInfo;
  onClose: () => void;
}

export function NetworkDetails({ exchange, onClose }: NetworkDetailsProps) {
  const [activeTab, setActiveTab] = useState('headers');

  const { request, response } = exchange;

  if (!request || !response) {
    return null;
  }

  const renderRequestBody = () => {
    try {
      const text = new TextDecoder('utf-8', { fatal: true }).decode(request.body);
      return text;
    } catch (e) {
      return `Binary data (bytes): ${Array.from(request.body).join(', ')}`;
    }
  };

  const renderResponseBody = () => {
    try {
      const body = Array.isArray(response?.body) ? new Uint8Array(response.body) : response.body;
      const text = new TextDecoder('utf-8', { fatal: true }).decode(body);
      // JSON인 경우 포맷팅
      try {
        const json = JSON.parse(text);
        return JSON.stringify(json, null, 2);
      } catch {
        return text;
      }
    } catch (e) {
      try {
        return `Binary data (bytes): ${Array.from(response.body).join(', ')}`;
      } catch (e) {
        return 'Error';
      }
    }
  };

  return (
    <div className="h-full bg-card border-l border-border flex flex-col">
      <div className="flex items-center justify-between p-4 border-b border-border">
        <div className="flex items-center gap-2">
          <h2 className="font-semibold text-card-foreground">Request Details</h2>
          <Badge variant="outline" className={`text-xs ${getStatusColor(response.status)}`}>
            {response.status}
          </Badge>
        </div>
        <Button variant="ghost" size="sm" onClick={onClose}>
          <X className="w-4 h-4" />
        </Button>
      </div>

      <div className="flex-1 overflow-auto p-4">
        <div className="space-y-4">
          <Card>
            <CardHeader className="pb-3">
              <CardTitle className="text-sm">Properties</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <div className="grid grid-cols-3 gap-2 text-sm">
                <span className="text-muted-foreground">Method:</span>
                <span className="col-span-2 font-mono break-all">{request.method}</span>
              </div>
              <div className="grid grid-cols-3 gap-2 text-sm">
                <span className="text-muted-foreground">Version:</span>
                <span className="col-span-2">{request.version}</span>
              </div>
              <div className="grid grid-cols-3 gap-2 text-sm">
                <span className="text-muted-foreground">Timestamp:</span>
                <span className="col-span-2">{new Date(request.time / 1_000_000).toISOString()}</span>
              </div>
            </CardContent>
          </Card>

          <Tabs value={activeTab} onValueChange={setActiveTab}>
            <TabsList className="grid w-full grid-cols-3">
              <TabsTrigger value="headers">Headers</TabsTrigger>
              {request.body && request.body.length > 0 && <TabsTrigger value="body">Body</TabsTrigger>}
              <TabsTrigger value="response">Response</TabsTrigger>
            </TabsList>

            <TabsContent value="headers" className="mt-4">
              <Card>
                <CardHeader className="pb-3">
                  <div className="flex items-center justify-between">
                    <CardTitle className="text-sm">Request Headers</CardTitle>
                    <Button variant="ghost" size="sm">
                      <Copy className="w-4 h-4" />
                    </Button>
                  </div>
                </CardHeader>
                <CardContent>
                  <div className="space-y-2">
                    {request.headers &&
                      Object.entries(request.headers).map(([key, value]) => (
                        <div key={key} className="grid grid-cols-3 gap-2 text-sm">
                          <span className="text-muted-foreground font-mono">{key}:</span>
                          <span className="col-span-2 font-mono break-all">{value}</span>
                        </div>
                      ))}
                  </div>
                </CardContent>
              </Card>
            </TabsContent>

            <TabsContent value="response" className="mt-4">
              <Card>
                <CardHeader className="pb-3">
                  <div className="flex items-center justify-between">
                    <CardTitle className="text-sm">Response</CardTitle>
                    <div className="flex gap-2">
                      <Button variant="ghost" size="sm">
                        <Copy className="w-4 h-4" />
                      </Button>
                      <Button variant="ghost" size="sm">
                        <Download className="w-4 h-4" />
                      </Button>
                    </div>
                  </div>
                </CardHeader>
                <CardContent>
                  <pre className="text-xs bg-muted p-3 rounded-md overflow-auto">{renderResponseBody()}</pre>
                </CardContent>
              </Card>
            </TabsContent>

            <TabsContent value="body" className="mt-4">
              <Card>
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm h-[32px] flex items-center">Request Body</CardTitle>
                </CardHeader>
                <CardContent>
                  <pre className="text-xs bg-muted p-3 rounded-md overflow-auto">{renderRequestBody()}</pre>
                </CardContent>
              </Card>
            </TabsContent>
          </Tabs>
        </div>
      </div>
    </div>
  );
}

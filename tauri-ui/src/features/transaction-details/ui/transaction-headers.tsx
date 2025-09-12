import { Copy } from 'lucide-react';

import type { HttpTransaction } from '@/entities/proxy';

import { Button, Card, CardContent, CardHeader, CardTitle } from '@/shared/ui';

interface TransactionHeadersProps {
  transaction: HttpTransaction;
}

export const TransactionHeaders = ({ transaction }: TransactionHeadersProps) => {
  const { request } = transaction;

  if (!request?.headers) return null;

  const handleCopy = () => {
    const headersText = Object.entries(request.headers)
      .map(([key, value]) => `${key}: ${value}`)
      .join('\n');
    navigator.clipboard.writeText(headersText);
  };

  return (
    <Card>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <CardTitle className="text-sm">Request Headers</CardTitle>
          <Button variant="ghost" size="sm" onClick={handleCopy}>
            <Copy className="w-4 h-4" />
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        <div className="space-y-2">
          {Object.entries(request.headers).map(([key, value]) => (
            <div key={key} className="grid grid-cols-3 gap-2 text-sm">
              <span className="text-muted-foreground font-mono">{key}:</span>
              <span className="col-span-2 font-mono break-all">{value}</span>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
};

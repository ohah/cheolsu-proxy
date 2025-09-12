import type { HttpTransaction } from '@/entities/proxy';

import { Card, CardContent, CardHeader, CardTitle } from '@/shared/ui';

import { formatTimestamp } from '../lib';

interface TransactionPropertiesProps {
  transaction: HttpTransaction;
}

export const TransactionProperties = ({ transaction }: TransactionPropertiesProps) => {
  const { request } = transaction;

  if (!request) return null;

  return (
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
          <span className="col-span-2">{formatTimestamp(request.time)}</span>
        </div>
      </CardContent>
    </Card>
  );
};

import type { HttpTransaction } from '@/entities/proxy';
import { Button, Card, CardContent, CardHeader, CardTitle } from '@/shared/ui';

import { formatBody } from '../lib';
import { useMemo } from 'react';
import { Copy } from 'lucide-react';

interface TransactionBodyProps {
  transaction: HttpTransaction;
}

export const TransactionBody = ({ transaction }: TransactionBodyProps) => {
  const { request } = transaction;

  const requestText = useMemo(() => {
    if (!request?.body || request.body.length === 0) {
      return 'No body content';
    }

    return formatBody(request.body);
  }, [request]);

  const handleCopy = () => {
    navigator.clipboard.writeText(requestText);
  };

  return (
    <Card>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <CardTitle className="text-sm">Request Body</CardTitle>
          <div className="flex gap-2">
            <Button variant="ghost" size="sm" onClick={handleCopy}>
              <Copy className="w-4 h-4" />
            </Button>
          </div>
        </div>
      </CardHeader>
      <CardContent>
        <pre className="text-xs bg-muted p-3 rounded-md overflow-auto whitespace-pre-wrap">{requestText}</pre>
      </CardContent>
    </Card>
  );
};

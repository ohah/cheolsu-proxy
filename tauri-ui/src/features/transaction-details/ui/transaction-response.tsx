import { Copy } from 'lucide-react';

import type { HttpTransaction } from '@/entities/proxy';

import { Button, Card, CardContent, CardHeader, CardTitle } from '@/shared/ui';

import { formatBody } from '../lib';
import { useMemo } from 'react';

interface TransactionResponseProps {
  transaction: HttpTransaction;
}

export const TransactionResponse = ({ transaction }: TransactionResponseProps) => {
  const { response } = transaction;

  if (!response) return null;

  const responseText = useMemo(() => formatBody(response.body), [response]);

  const handleCopy = () => {
    navigator.clipboard.writeText(responseText);
  };

  return (
    <Card>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <CardTitle className="text-sm">Response</CardTitle>
          <div className="flex gap-2">
            <Button variant="ghost" size="sm" onClick={handleCopy}>
              <Copy className="w-4 h-4" />
            </Button>
          </div>
        </div>
      </CardHeader>
      <CardContent>
        <pre className="text-xs bg-muted p-3 rounded-md overflow-auto whitespace-pre-wrap">{responseText}</pre>
      </CardContent>
    </Card>
  );
};

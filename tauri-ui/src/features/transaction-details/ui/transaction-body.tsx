import type { HttpTransaction } from '@/entities/proxy';
import { Button, Card, CardContent, CardHeader, CardTitle, Textarea } from '@/shared/ui';
import type { AppFormInstance } from '../context/form-context';

import { formatBody } from '../lib';
import { useMemo } from 'react';
import { Copy } from 'lucide-react';

interface TransactionBodyProps {
  transaction: HttpTransaction;
  isEditing?: boolean;
  form?: AppFormInstance;
}

export const TransactionBody = ({ transaction, isEditing = false, form }: TransactionBodyProps) => {
  const { request } = transaction;

  const requestText = useMemo(() => {
    if (!request?.body || request.body.length === 0) {
      return '';
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
        {form && isEditing ? (
          <form.Field
            name="request.body"
            children={(field: any) => (
              <Textarea
                value={field.state.value || ''}
                onChange={(e) => field.handleChange(e.target.value)}
                placeholder="Enter request body content..."
                className="min-h-[200px] font-mono text-xs"
              />
            )}
          />
        ) : (
          <pre className="text-xs bg-muted p-3 rounded-md overflow-auto whitespace-pre-wrap">{requestText}</pre>
        )}
      </CardContent>
    </Card>
  );
};

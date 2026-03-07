import { Copy } from "lucide-react";

import type { HttpTransaction } from "@/entities/proxy";

import { Button, Card, CardContent, CardHeader } from "@/shared/ui";
import { toast } from "sonner";

interface TransactionHeadersProps {
  transaction: HttpTransaction;
}

export const TransactionHeaders = ({ transaction }: TransactionHeadersProps) => {
  const { request } = transaction;

  if (!request?.headers) return null;

  const handleCopy = () => {
    const headersText = Object.entries(request.headers)
      .map(([key, value]) => `${key}: ${value}`)
      .join("\n");
    navigator.clipboard.writeText(headersText);
    toast.success("Request headers copied to clipboard");
  };

  return (
    <Card className="gap-0">
      <CardHeader>
        <div className="flex items-center justify-end gap-2">
          <Button variant="ghost" size="sm" onClick={handleCopy}>
            <Copy className="w-4 h-4" />
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        <div className="space-y-2">
          {Object.entries(request.headers).map(([key, value]) => (
            <div key={key} className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground font-mono flex-1">{key}:</span>
              <span className="font-mono break-all flex-2">{value}</span>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
};

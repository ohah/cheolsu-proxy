import { useState, useCallback, useEffect } from "react";
import { Play, Loader2 } from "lucide-react";

import type { HttpTransaction } from "@/entities/proxy";
import { isTextBasedDataType } from "@/entities/proxy/model/data-type";
import {
  replayRequest,
  type ReplayRequestParams,
  type ReplayResponse,
} from "@/shared/api/proxy";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  Button,
  Badge,
  Textarea,
  Input,
  ScrollArea,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  Separator,
} from "@/shared/ui";
import { getStatusColor } from "@/entities/transaction";
import { uint8ArrayToString } from "../lib";

const HTTP_METHODS = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"] as const;

interface ReplayDialogProps {
  transaction: HttpTransaction;
}

function headersToText(headers: Record<string, string>): string {
  return Object.entries(headers)
    .map(([k, v]) => `${k}: ${v}`)
    .join("\n");
}

function textToHeaders(text: string): Record<string, string> {
  const headers: Record<string, string> = {};
  for (const line of text.split("\n")) {
    const idx = line.indexOf(":");
    if (idx > 0) {
      const key = line.slice(0, idx).trim();
      const value = line.slice(idx + 1).trim();
      if (key) headers[key] = value;
    }
  }
  return headers;
}

export function ReplayDialog({ transaction }: ReplayDialogProps) {
  const { request } = transaction;
  const [open, setOpen] = useState(false);
  const [method, setMethod] = useState(request?.method || "GET");
  const [url, setUrl] = useState(request?.uri || "");
  const [headersText, setHeadersText] = useState("");
  const [body, setBody] = useState("");
  const [loading, setLoading] = useState(false);
  const [response, setResponse] = useState<ReplayResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState("request");

  useEffect(() => {
    if (open && request) {
      setMethod(request.method);
      setUrl(request.uri);
      setHeadersText(headersToText(request.headers || {}));
      if (request.body && request.data_type && isTextBasedDataType(request.data_type)) {
        setBody(uint8ArrayToString(request.body, request.data_type));
      } else {
        setBody("");
      }
      setResponse(null);
      setError(null);
      setActiveTab("request");
    }
  }, [open, request]);

  const handleReplay = useCallback(async () => {
    setLoading(true);
    setError(null);
    setResponse(null);

    const params: ReplayRequestParams = {
      method,
      url,
      headers: textToHeaders(headersText),
      body: body || undefined,
    };

    try {
      const res = await replayRequest(params);
      setResponse(res);
      setActiveTab("response");
    } catch (e: any) {
      setError(typeof e === "string" ? e : e.message || "요청 실패");
    } finally {
      setLoading(false);
    }
  }, [method, url, headersText, body]);

  const formatResponseBody = (res: ReplayResponse): string => {
    if (!res.body) return "(empty)";
    if (res.body.startsWith("base64:")) {
      return `[Binary data - ${res.body_size} bytes]`;
    }
    try {
      return JSON.stringify(JSON.parse(res.body), null, 2);
    } catch {
      return res.body;
    }
  };

  return (
    <>
      <Button variant="ghost" size="sm" title="Replay request" onClick={() => setOpen(true)}>
        <Play className="w-4 h-4" />
      </Button>

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="sm:max-w-2xl max-h-[80vh] flex flex-col">
          <DialogHeader>
            <DialogTitle>Traffic Replay</DialogTitle>
          </DialogHeader>

          <Tabs
            value={activeTab}
            onValueChange={setActiveTab}
            className="flex-1 flex flex-col min-h-0"
          >
            <TabsList className="grid w-full grid-cols-2 flex-shrink-0">
              <TabsTrigger value="request">Request</TabsTrigger>
              <TabsTrigger value="response">
                Response
                {response && (
                  <Badge
                    variant="outline"
                    className={`ml-2 text-xs ${getStatusColor(response.status)}`}
                  >
                    {response.status}
                  </Badge>
                )}
              </TabsTrigger>
            </TabsList>

            <TabsContent value="request" className="flex-1 mt-4 space-y-4 min-h-0">
              <div className="flex gap-2">
                <Select value={method} onValueChange={(v) => v && setMethod(v)}>
                  <SelectTrigger className="w-28">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {HTTP_METHODS.map((m) => (
                      <SelectItem key={m} value={m}>
                        {m}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <Input
                  value={url}
                  onChange={(e) => setUrl(e.target.value)}
                  placeholder="URL"
                  className="flex-1 font-mono text-xs"
                />
              </div>

              <div className="space-y-2">
                <label className="text-sm font-medium">Headers</label>
                <Textarea
                  value={headersText}
                  onChange={(e) => setHeadersText(e.target.value)}
                  placeholder="Content-Type: application/json"
                  className="font-mono text-xs min-h-[100px] resize-y"
                />
              </div>

              {method !== "GET" && method !== "HEAD" && (
                <div className="space-y-2">
                  <label className="text-sm font-medium">Body</label>
                  <Textarea
                    value={body}
                    onChange={(e) => setBody(e.target.value)}
                    placeholder="Request body"
                    className="font-mono text-xs min-h-[100px] resize-y"
                  />
                </div>
              )}

              <Button onClick={handleReplay} disabled={loading || !url} className="w-full">
                {loading ? (
                  <>
                    <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                    Sending...
                  </>
                ) : (
                  <>
                    <Play className="w-4 h-4 mr-2" />
                    Send Request
                  </>
                )}
              </Button>

              {error && (
                <div className="p-3 rounded-md bg-destructive/10 text-destructive text-sm">
                  {error}
                </div>
              )}
            </TabsContent>

            <TabsContent value="response" className="flex-1 mt-4 min-h-0 flex flex-col">
              {response ? (
                <div className="space-y-4 flex-1 flex flex-col min-h-0">
                  <div className="flex items-center gap-4 flex-shrink-0">
                    <Badge
                      variant="outline"
                      className={`text-sm ${getStatusColor(response.status)}`}
                    >
                      {response.status}
                    </Badge>
                    <span className="text-xs text-muted-foreground">{response.elapsed_ms}ms</span>
                    <span className="text-xs text-muted-foreground">
                      {response.body_size} bytes
                    </span>
                  </div>

                  <Separator />

                  <div className="space-y-2 flex-shrink-0">
                    <label className="text-sm font-medium">Response Headers</label>
                    <ScrollArea className="max-h-[120px]">
                      <div className="font-mono text-xs space-y-0.5">
                        {Object.entries(response.headers).map(([k, v]) => (
                          <div key={k}>
                            <span className="text-muted-foreground">{k}:</span> {v}
                          </div>
                        ))}
                      </div>
                    </ScrollArea>
                  </div>

                  <Separator />

                  <div className="space-y-2 flex-1 min-h-0 flex flex-col">
                    <label className="text-sm font-medium">Response Body</label>
                    <ScrollArea className="flex-1 min-h-[150px] max-h-[300px]">
                      <pre className="font-mono text-xs whitespace-pre-wrap break-all p-2 bg-muted rounded-md">
                        {formatResponseBody(response)}
                      </pre>
                    </ScrollArea>
                  </div>
                </div>
              ) : (
                <div className="flex items-center justify-center h-40 text-muted-foreground text-sm">
                  Send a request to see the response
                </div>
              )}
            </TabsContent>
          </Tabs>
        </DialogContent>
      </Dialog>
    </>
  );
}

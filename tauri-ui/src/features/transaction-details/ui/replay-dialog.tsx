import { useState, useCallback, useEffect } from "react";
import { Play, Loader2, Plus, Trash2 } from "lucide-react";
import { Editor } from "@monaco-editor/react";

import type { HttpTransaction } from "@/entities/proxy";
import { isTextBasedDataType } from "@/entities/proxy/model/data-type";
import { replayRequest, type ReplayRequestParams, type ReplayResponse } from "@/shared/api/proxy";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  Button,
  Badge,
  Input,
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

interface HeaderEntry {
  key: string;
  value: string;
}

function headersToEntries(headers: Record<string, string>): HeaderEntry[] {
  return Object.entries(headers).map(([key, value]) => ({ key, value }));
}

function entriesToHeaders(entries: HeaderEntry[]): Record<string, string> {
  const headers: Record<string, string> = {};
  for (const { key, value } of entries) {
    const k = key.trim();
    if (k) headers[k] = value;
  }
  return headers;
}

function detectLanguage(body: string | undefined | null): string {
  if (!body) return "plaintext";
  const trimmed = body.trimStart();
  if (trimmed.startsWith("{") || trimmed.startsWith("[")) return "json";
  if (trimmed.startsWith("<")) return "xml";
  return "plaintext";
}

function formatBody(body: string | undefined | null, bodySize?: number): string {
  if (!body) return "(empty)";
  if (body.startsWith("base64:")) {
    return `[Binary data - ${bodySize ?? 0} bytes]`;
  }
  try {
    return JSON.stringify(JSON.parse(body), null, 2);
  } catch {
    return body;
  }
}

function getResponseBody(response: HttpTransaction["response"]): string | null {
  if (!response) return null;

  // body_json이 있으면 우선 사용
  if (response.body_json !== undefined && response.body_json !== null) {
    return typeof response.body_json === "string"
      ? response.body_json
      : JSON.stringify(response.body_json, null, 2);
  }

  // Uint8Array body가 있으면 변환
  if (response.body && response.data_type && isTextBasedDataType(response.data_type)) {
    return uint8ArrayToString(response.body, response.data_type);
  }

  // body가 있지만 텍스트가 아닌 경우
  if (response.body_size > 0) {
    return `[Binary data - ${response.body_size} bytes]`;
  }

  return null;
}

function ResponseView({
  status,
  headers,
  body,
  bodySize,
  elapsedMs,
}: {
  status: number;
  headers: Record<string, string>;
  body?: string | null;
  bodySize: number;
  elapsedMs?: number;
}) {
  const headerEntries = Object.entries(headers);

  return (
    <div className="h-full overflow-y-auto space-y-4 px-1">
      <div className="flex items-center gap-4">
        <Badge variant="outline" className={`text-sm ${getStatusColor(status)}`}>
          {status}
        </Badge>
        {elapsedMs !== undefined && (
          <span className="text-xs text-muted-foreground">{elapsedMs}ms</span>
        )}
        <span className="text-xs text-muted-foreground">{bodySize} bytes</span>
      </div>

      <Separator />

      {headerEntries.length > 0 && (
        <>
          <div className="space-y-2">
            <label className="text-sm font-medium">Headers ({headerEntries.length})</label>
            <div className="rounded-md border">
              <table className="w-full text-xs font-mono">
                <tbody>
                  {headerEntries.map(([k, v]) => (
                    <tr key={k} className="border-b last:border-b-0">
                      <td className="px-3 py-1.5 text-muted-foreground font-medium whitespace-nowrap align-top w-[220px]">
                        {k}
                      </td>
                      <td className="px-3 py-1.5 break-all">{v}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>

          <Separator />
        </>
      )}

      <div className="space-y-2">
        <label className="text-sm font-medium">Body</label>
        <div className="rounded-md border overflow-hidden">
          <Editor
            height="300px"
            language={detectLanguage(body)}
            value={formatBody(body, bodySize)}
            theme="vs-light"
            options={{
              readOnly: true,
              minimap: { enabled: false },
              fontSize: 12,
              lineNumbers: "off",
              scrollBeyondLastLine: false,
              wordWrap: "on",
              tabSize: 2,
              automaticLayout: true,
              padding: { top: 8, bottom: 8 },
            }}
          />
        </div>
      </div>
    </div>
  );
}

export function ReplayDialog({ transaction }: ReplayDialogProps) {
  const { request, response: originalResponse } = transaction;
  const [open, setOpen] = useState(false);
  const [method, setMethod] = useState(request?.method || "GET");
  const [url, setUrl] = useState(request?.uri || "");
  const [headers, setHeaders] = useState<HeaderEntry[]>([]);
  const [body, setBody] = useState("");
  const [loading, setLoading] = useState(false);
  const [replayResponse, setReplayResponse] = useState<ReplayResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState("request");

  useEffect(() => {
    if (open && request) {
      setMethod(request.method);
      setUrl(request.uri);
      setHeaders(headersToEntries(request.headers || {}));
      if (request.body && request.data_type && isTextBasedDataType(request.data_type)) {
        setBody(uint8ArrayToString(request.body, request.data_type));
      } else if (request.body_json) {
        setBody(
          typeof request.body_json === "string"
            ? request.body_json
            : JSON.stringify(request.body_json, null, 2),
        );
      } else {
        setBody("");
      }
      setReplayResponse(null);
      setError(null);
      setActiveTab("request");
    }
  }, [open, request]);

  const handleReplay = useCallback(async () => {
    setLoading(true);
    setError(null);
    setReplayResponse(null);

    const params: ReplayRequestParams = {
      method,
      url,
      headers: entriesToHeaders(headers),
      body: body || undefined,
    };

    try {
      const res = await replayRequest(params);
      setReplayResponse(res);
      setActiveTab("replay");
    } catch (e: any) {
      setError(typeof e === "string" ? e : e.message || "요청 실패");
    } finally {
      setLoading(false);
    }
  }, [method, url, headers, body]);

  const addHeader = () => setHeaders([...headers, { key: "", value: "" }]);
  const removeHeader = (index: number) => setHeaders(headers.filter((_, i) => i !== index));
  const updateHeader = (index: number, field: "key" | "value", value: string) => {
    const updated = [...headers];
    updated[index] = { ...updated[index], [field]: value };
    setHeaders(updated);
  };

  const bodyLanguage = detectLanguage(body);
  const originalResponseBody = getResponseBody(originalResponse);
  const hasOriginalResponse = !!originalResponse;
  const hasReplayResponse = !!replayResponse;

  const tabCount = 1 + (hasOriginalResponse ? 1 : 0) + 1;

  return (
    <>
      <Button variant="ghost" size="sm" title="Replay request" onClick={() => setOpen(true)}>
        <Play className="w-4 h-4" />
      </Button>

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="!max-w-none !rounded-none w-screen h-screen flex flex-col">
          <DialogHeader className="flex-shrink-0">
            <DialogTitle>Traffic Replay</DialogTitle>
          </DialogHeader>

          <Tabs
            value={activeTab}
            onValueChange={setActiveTab}
            className="flex-1 flex flex-col min-h-0"
          >
            <TabsList
              className={`grid w-full flex-shrink-0 ${tabCount === 3 ? "grid-cols-3" : "grid-cols-2"}`}
            >
              <TabsTrigger value="request">Request</TabsTrigger>
              {hasOriginalResponse && (
                <TabsTrigger value="response">
                  Original
                  <Badge
                    variant="outline"
                    className={`ml-1 text-xs ${getStatusColor(originalResponse.status)}`}
                  >
                    {originalResponse.status}
                  </Badge>
                </TabsTrigger>
              )}
              <TabsTrigger value="replay">
                Replay
                {hasReplayResponse && (
                  <Badge
                    variant="outline"
                    className={`ml-1 text-xs ${getStatusColor(replayResponse.status)}`}
                  >
                    {replayResponse.status}
                  </Badge>
                )}
              </TabsTrigger>
            </TabsList>

            <TabsContent
              value="request"
              className="mt-4 min-h-0 flex-1 flex flex-col justify-between"
            >
              <div className="flex-1 min-h-0 overflow-y-auto space-y-4 px-1">
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
                  <div className="flex items-center justify-between">
                    <label className="text-sm font-medium">Headers ({headers.length})</label>
                    <Button variant="ghost" size="sm" onClick={addHeader}>
                      <Plus className="w-3.5 h-3.5 mr-1" />
                      Add
                    </Button>
                  </div>
                  <div className="rounded-md border">
                    <table className="w-full text-xs font-mono">
                      <tbody>
                        {headers.map((header, i) => (
                          <tr key={i} className="border-b last:border-b-0 group">
                            <td className="px-2 py-1 w-[220px]">
                              <Input
                                value={header.key}
                                onChange={(e) => updateHeader(i, "key", e.target.value)}
                                placeholder="Header name"
                                className="h-7 text-xs font-mono border-0 shadow-none focus-visible:ring-0 px-1"
                              />
                            </td>
                            <td className="px-2 py-1">
                              <Input
                                value={header.value}
                                onChange={(e) => updateHeader(i, "value", e.target.value)}
                                placeholder="Value"
                                className="h-7 text-xs font-mono border-0 shadow-none focus-visible:ring-0 px-1"
                              />
                            </td>
                            <td className="px-1 py-1 w-8">
                              <Button
                                variant="ghost"
                                size="sm"
                                className="h-7 w-7 p-0 opacity-0 group-hover:opacity-100"
                                onClick={() => removeHeader(i)}
                              >
                                <Trash2 className="w-3 h-3 text-destructive" />
                              </Button>
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </div>

                {method !== "GET" && method !== "HEAD" && (
                  <div className="space-y-2">
                    <label className="text-sm font-medium">Body</label>
                    <div className="rounded-md border overflow-hidden">
                      <Editor
                        height="200px"
                        language={bodyLanguage}
                        value={body}
                        onChange={(v) => setBody(v ?? "")}
                        theme="vs-light"
                        options={{
                          minimap: { enabled: false },
                          fontSize: 12,
                          lineNumbers: "off",
                          scrollBeyondLastLine: false,
                          wordWrap: "on",
                          tabSize: 2,
                          automaticLayout: true,
                          padding: { top: 8, bottom: 8 },
                        }}
                      />
                    </div>
                  </div>
                )}
              </div>

              <div className="flex-shrink-0 pt-4 space-y-3">
                {error && (
                  <div className="p-3 rounded-md bg-destructive/10 text-destructive text-sm">
                    {error}
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
              </div>
            </TabsContent>

            {hasOriginalResponse && (
              <TabsContent value="response" className="mt-4 min-h-0 flex-1 overflow-hidden">
                <ResponseView
                  status={originalResponse.status}
                  headers={originalResponse.headers || {}}
                  body={originalResponseBody}
                  bodySize={originalResponse.body_size}
                />
              </TabsContent>
            )}

            <TabsContent value="replay" className="mt-4 min-h-0 flex-1 overflow-hidden">
              {hasReplayResponse ? (
                <ResponseView
                  status={replayResponse.status}
                  headers={replayResponse.headers}
                  body={replayResponse.body}
                  bodySize={replayResponse.body_size}
                  elapsedMs={replayResponse.elapsed_ms}
                />
              ) : (
                <div className="flex items-center justify-center h-40 text-muted-foreground text-sm">
                  Send a request to see the replay response
                </div>
              )}
            </TabsContent>
          </Tabs>
        </DialogContent>
      </Dialog>
    </>
  );
}

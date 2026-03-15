import { useState, useCallback, useEffect, useRef } from "react";
import { Play, Loader2, Plus, Trash2, Maximize2, Minimize2 } from "lucide-react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { Editor } from "@monaco-editor/react";
import { useTheme } from "next-themes";
import { toast } from "sonner";

import type { HttpTransaction } from "@/entities/proxy";
import { isTextBasedDataType } from "@/entities/proxy";
import { replayRequest, type ReplayRequestParams, type ReplayResponse } from "@/shared/api/proxy";
import {
  useHeaderEditor,
  headersToEntries,
  entriesToHeaders,
} from "@/shared/hooks/use-header-editor";
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
  Tooltip,
  TooltipTrigger,
  TooltipContent,
} from "@/shared/ui";
import { getStatusColor } from "@/entities/transaction";
import { uint8ArrayToString } from "../lib";

const HTTP_METHODS = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"] as const;

interface ReplayDialogProps {
  transaction?: HttpTransaction;
  /** 외부에서 다이얼로그 open 상태를 제어할 때 사용 */
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  /** 트리거 버튼을 숨길 때 사용 (Compose 모드) */
  hideTrigger?: boolean;
}

function isAllowedUrl(url: string): boolean {
  const trimmed = url.trim();
  if (!trimmed) return false;
  // Only allow http:// and https:// protocols
  if (/^https?:\/\//i.test(trimmed)) return true;
  // Allow URLs without protocol (will be treated as http by backend)
  if (!/^[a-zA-Z][a-zA-Z0-9+.-]*:/.test(trimmed)) return true;
  return false;
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
  const { t } = useLingui();
  const { resolvedTheme } = useTheme();
  const [expanded, setExpanded] = useState(false);
  const headerEntries = Object.entries(headers);

  if (expanded) {
    return (
      <div className="h-full flex flex-col px-1">
        <div className="flex items-center justify-between mb-2">
          <label className="text-sm font-medium">
            <Trans>Body</Trans>
          </label>
          <Button variant="ghost" size="sm" onClick={() => setExpanded(false)}>
            <Minimize2 className="w-3.5 h-3.5 mr-1" />
            <Trans>Collapse</Trans>
          </Button>
        </div>
        <div className="flex-1 rounded-md border overflow-hidden">
          <Editor
            height="100%"
            language={detectLanguage(body)}
            value={formatBody(body, bodySize)}
            theme={resolvedTheme === "dark" ? "vs-dark" : "vs-light"}
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
    );
  }

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
            <label className="text-sm font-medium">{t`Headers (${headerEntries.length})`}</label>
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
        <div className="flex items-center justify-between">
          <label className="text-sm font-medium">
            <Trans>Body</Trans>
          </label>
          <Button variant="ghost" size="sm" onClick={() => setExpanded(true)}>
            <Maximize2 className="w-3.5 h-3.5 mr-1" />
            <Trans>Expand</Trans>
          </Button>
        </div>
        <div className="rounded-md border overflow-hidden">
          <Editor
            height="300px"
            language={detectLanguage(body)}
            value={formatBody(body, bodySize)}
            theme={resolvedTheme === "dark" ? "vs-dark" : "vs-light"}
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

export function ReplayDialog({
  transaction,
  open: controlledOpen,
  onOpenChange,
  hideTrigger,
}: ReplayDialogProps) {
  const { t } = useLingui();
  const { resolvedTheme } = useTheme();
  const request = transaction?.request;
  const originalResponse = transaction?.response;
  const isComposeMode = !transaction;
  const isControlled = controlledOpen !== undefined;
  const [internalOpen, setInternalOpen] = useState(false);
  const open = isControlled ? controlledOpen : internalOpen;
  const setOpen = useCallback(
    (v: boolean) => {
      if (!isControlled) {
        setInternalOpen(v);
      }
      onOpenChange?.(v);
    },
    [isControlled, onOpenChange],
  );
  const [method, setMethod] = useState(request?.method || "GET");
  const [url, setUrl] = useState(request?.uri || "");
  const { headers, addHeader, removeHeader, updateHeader, resetHeaders } = useHeaderEditor();
  const [body, setBody] = useState("");
  const [loading, setLoading] = useState(false);
  const [replayResponse, setReplayResponse] = useState<ReplayResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState("request");
  const [bodyExpanded, setBodyExpanded] = useState(false);

  // Use a ref to track the request identity for useEffect stability
  const requestRef = useRef(request);
  requestRef.current = request;

  // 다이얼로그가 닫힌 후 비동기 응답이 도착해도 state 업데이트 방지
  const openRef = useRef(open);
  openRef.current = open;

  useEffect(() => {
    if (open) {
      const req = requestRef.current;
      if (req) {
        setMethod(req.method);
        setUrl(req.uri);
        resetHeaders(headersToEntries(req.headers || {}));
        if (req.body && req.data_type && isTextBasedDataType(req.data_type)) {
          setBody(uint8ArrayToString(req.body, req.data_type));
        } else if (req.body_json) {
          setBody(
            typeof req.body_json === "string"
              ? req.body_json
              : JSON.stringify(req.body_json, null, 2),
          );
        } else {
          setBody("");
        }
      } else {
        setMethod("GET");
        setUrl("");
        resetHeaders([]);
        setBody("");
      }
      setReplayResponse(null);
      setError(null);
      setActiveTab("request");
    }
    // resetHeaders는 안정적인 참조이므로 deps에서 제외
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  const handleReplay = useCallback(async () => {
    if (!isAllowedUrl(url)) {
      toast.error(t`Only http:// and https:// URLs are allowed`);
      return;
    }

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
      if (!openRef.current) return;
      setReplayResponse(res);
      setActiveTab("replay");
    } catch (e: unknown) {
      if (!openRef.current) return;
      setError(typeof e === "string" ? e : e instanceof Error ? e.message : t`Request failed`);
    } finally {
      if (openRef.current) setLoading(false);
    }
  }, [method, url, headers, body]);

  const bodyLanguage = detectLanguage(body);
  const originalResponseBody = getResponseBody(originalResponse ?? null);
  const hasOriginalResponse = !!originalResponse;
  const hasReplayResponse = !!replayResponse;

  const tabCount = 1 + (hasOriginalResponse ? 1 : 0) + 1;

  return (
    <>
      {!hideTrigger && (
        <Tooltip>
          <TooltipTrigger render={<div />}>
            <Button variant="ghost" size="sm" onClick={() => setOpen(true)}>
              <Play className="w-4 h-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent sideOffset={4}>
            <Trans>Replay request</Trans>
          </TooltipContent>
        </Tooltip>
      )}

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="!max-w-none !rounded-none w-screen h-screen flex flex-col">
          <DialogHeader className="flex-shrink-0">
            <DialogTitle>
              {isComposeMode ? <Trans>Compose Request</Trans> : <Trans>Traffic Replay</Trans>}
            </DialogTitle>
          </DialogHeader>

          <Tabs
            value={activeTab}
            onValueChange={setActiveTab}
            className="flex-1 flex flex-col min-h-0"
          >
            <TabsList
              className={`grid w-full flex-shrink-0 ${tabCount === 3 ? "grid-cols-3" : "grid-cols-2"}`}
            >
              <TabsTrigger value="request">
                <Trans>Request</Trans>
              </TabsTrigger>
              {hasOriginalResponse && (
                <TabsTrigger value="response">
                  <Trans>Original</Trans>
                  <Badge
                    variant="outline"
                    className={`ml-1 text-xs ${getStatusColor(originalResponse.status)}`}
                  >
                    {originalResponse.status}
                  </Badge>
                </TabsTrigger>
              )}
              <TabsTrigger value="replay">
                {isComposeMode ? <Trans>Response</Trans> : <Trans>Replay</Trans>}
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
              <div
                className={`flex-1 min-h-0 space-y-4 px-1 ${bodyExpanded ? "flex flex-col" : "overflow-y-auto"}`}
              >
                {!bodyExpanded && (
                  <>
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
                        placeholder={isComposeMode ? "https://api.example.com/endpoint" : t`URL`}
                        className="flex-1 font-mono text-xs"
                      />
                    </div>

                    <div className="space-y-2">
                      <div className="flex items-center justify-between">
                        <label className="text-sm font-medium">{t`Headers (${headers.length})`}</label>
                        <Button variant="ghost" size="sm" onClick={addHeader}>
                          <Plus className="w-3.5 h-3.5 mr-1" />
                          <Trans>Add</Trans>
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
                                    placeholder={t`Header name`}
                                    className="h-7 text-xs font-mono border-0 shadow-none focus-visible:ring-0 px-1"
                                  />
                                </td>
                                <td className="px-2 py-1">
                                  <Input
                                    value={header.value}
                                    onChange={(e) => updateHeader(i, "value", e.target.value)}
                                    placeholder={t`Value`}
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
                  </>
                )}

                {method !== "GET" && method !== "HEAD" && (
                  <div
                    className={`space-y-2 ${bodyExpanded ? "flex-1 flex flex-col min-h-0" : ""}`}
                  >
                    <div className="flex items-center justify-between">
                      <label className="text-sm font-medium">
                        <Trans>Body</Trans>
                      </label>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => setBodyExpanded(!bodyExpanded)}
                      >
                        {bodyExpanded ? (
                          <>
                            <Minimize2 className="w-3.5 h-3.5 mr-1" />
                            <Trans>Collapse</Trans>
                          </>
                        ) : (
                          <>
                            <Maximize2 className="w-3.5 h-3.5 mr-1" />
                            <Trans>Expand</Trans>
                          </>
                        )}
                      </Button>
                    </div>
                    <div
                      className={`rounded-md border overflow-hidden ${bodyExpanded ? "flex-1" : ""}`}
                    >
                      <Editor
                        height={bodyExpanded ? "100%" : "200px"}
                        language={bodyLanguage}
                        value={body}
                        onChange={(v) => setBody(v ?? "")}
                        theme={resolvedTheme === "dark" ? "vs-dark" : "vs-light"}
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
                      <Trans>Sending...</Trans>
                    </>
                  ) : (
                    <>
                      <Play className="w-4 h-4 mr-2" />
                      <Trans>Send Request</Trans>
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
                  {isComposeMode ? (
                    <Trans>Send a request to see the response</Trans>
                  ) : (
                    <Trans>Send a request to see the replay response</Trans>
                  )}
                </div>
              )}
            </TabsContent>
          </Tabs>
        </DialogContent>
      </Dialog>
    </>
  );
}

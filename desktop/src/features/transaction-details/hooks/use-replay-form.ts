import { useState, useCallback, useEffect, useRef } from "react";
import { useLingui } from "@lingui/react/macro";
import { toast } from "sonner";

import type { HttpTransaction } from "@/entities/proxy";
import { isTextBasedDataType } from "@/entities/proxy";
import { replayRequest, type ReplayRequestParams, type ReplayResponse } from "@/shared/api/proxy";
import {
  useHeaderEditor,
  headersToEntries,
  entriesToHeaders,
} from "@/shared/hooks/use-header-editor";
import { uint8ArrayToString } from "../lib";

interface UseReplayFormOptions {
  transaction?: HttpTransaction;
  open: boolean;
}

export function useReplayForm({ transaction, open }: UseReplayFormOptions) {
  const { t } = useLingui();
  const request = transaction?.request;
  const originalResponse = transaction?.response;

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
      // 요청 중 다이얼로그를 닫아도 loading은 반드시 해제해야 재오픈 시 버튼이 고착되지 않는다
      setLoading(false);
    }
  }, [method, url, headers, body, t]);

  const toggleBodyExpanded = useCallback(() => {
    setBodyExpanded((prev) => !prev);
  }, []);

  return {
    method,
    setMethod,
    url,
    setUrl,
    headers,
    addHeader,
    removeHeader,
    updateHeader,
    body,
    setBody,
    loading,
    replayResponse,
    error,
    activeTab,
    setActiveTab,
    bodyExpanded,
    toggleBodyExpanded,
    handleReplay,
    originalResponse,
  };
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

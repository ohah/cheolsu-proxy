import { useState, useCallback, useEffect } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { Play, Loader2 } from "lucide-react";
import { Editor } from "@monaco-editor/react";
import { useTheme } from "next-themes";

import type { WsMessageInfo } from "@/entities/websocket";
import { wsInjectMessage, type WsInjectParams } from "@/shared/api/proxy";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  Button,
  Badge,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  Alert,
  AlertDescription,
} from "@/shared/ui";

interface WsReplayDialogProps {
  message: WsMessageInfo;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

function detectLanguage(payload: string, isBinary: boolean): string {
  if (isBinary) return "plaintext";
  const trimmed = payload.trimStart();
  try {
    JSON.parse(trimmed);
    return "json";
  } catch {
    if (trimmed.startsWith("<")) return "xml";
    return "plaintext";
  }
}

function formatPayload(payload: string, isBinary: boolean): string {
  if (isBinary) return payload;
  try {
    return JSON.stringify(JSON.parse(payload), null, 2);
  } catch {
    return payload;
  }
}

export function WsReplayDialog({ message, open, onOpenChange }: WsReplayDialogProps) {
  const { t } = useLingui();
  const { resolvedTheme } = useTheme();
  const [direction, setDirection] = useState<"to_client" | "to_server">(
    message.direction === "client_to_server" ? "to_server" : "to_client",
  );
  const [payload, setPayload] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);

  useEffect(() => {
    if (open) {
      setDirection(message.direction === "client_to_server" ? "to_server" : "to_client");
      setPayload(formatPayload(message.payload, message.is_binary));
      setError(null);
      setSuccess(false);
    }
  }, [open, message]);

  const handleSend = useCallback(async () => {
    setLoading(true);
    setError(null);
    setSuccess(false);

    const params: WsInjectParams = {
      connection_id: message.connection_id,
      direction,
      payload,
      is_binary: message.is_binary,
    };

    try {
      await wsInjectMessage(params);
      setSuccess(true);
    } catch (e: unknown) {
      const errorMessage =
        e instanceof Error ? e.message : typeof e === "string" ? e : t`Failed to send`;
      setError(errorMessage);
    } finally {
      setLoading(false);
    }
  }, [message.connection_id, message.is_binary, direction, payload]);

  const language = detectLanguage(payload, message.is_binary);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="!max-w-2xl h-[70vh] flex flex-col">
        <DialogHeader className="flex-shrink-0">
          <DialogTitle>
            <Trans>WebSocket Replay</Trans>
          </DialogTitle>
        </DialogHeader>

        <div className="flex-1 flex flex-col min-h-0 gap-4">
          {/* Connection info */}
          <div className="flex items-center gap-3 text-xs">
            <span className="text-muted-foreground">
              <Trans>Connection</Trans>:
            </span>
            <code className="font-mono text-foreground bg-muted px-2 py-0.5 rounded truncate max-w-[400px]">
              {message.connection_id}
            </code>
            <Badge variant="outline" className="text-[10px]">
              {message.is_binary ? "binary" : "text"}
            </Badge>
          </div>

          {/* Direction selector */}
          <div className="flex items-center gap-3">
            <span className="text-sm text-muted-foreground whitespace-nowrap">
              <Trans>Direction</Trans>:
            </span>
            <Select
              value={direction}
              onValueChange={(v) => setDirection(v as "to_client" | "to_server")}
            >
              <SelectTrigger className="w-52">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem
                  value="to_server"
                  label={t`To Server (Client → Server)`}
                >{t`To Server (Client → Server)`}</SelectItem>
                <SelectItem
                  value="to_client"
                  label={t`To Client (Server → Client)`}
                >{t`To Client (Server → Client)`}</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {/* Payload editor */}
          <div className="flex-1 min-h-0 flex flex-col">
            <label className="text-sm font-medium mb-2">
              <Trans>Payload</Trans>
            </label>
            <div className="flex-1 rounded-md border overflow-hidden">
              <Editor
                height="100%"
                language={language}
                value={payload}
                onChange={(v) => setPayload(v ?? "")}
                theme={resolvedTheme === "dark" ? "vs-dark" : "vs"}
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

          {/* Status messages */}
          {error && (
            <Alert variant="destructive">
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}
          {success && (
            <Alert variant="success">
              <AlertDescription>
                <Trans>Message sent successfully</Trans>
              </AlertDescription>
            </Alert>
          )}

          {/* Send button */}
          <Button
            onClick={handleSend}
            disabled={loading || !payload}
            className="w-full flex-shrink-0"
          >
            {loading ? (
              <>
                <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                <Trans>Sending...</Trans>
              </>
            ) : (
              <>
                <Play className="w-4 h-4 mr-2" />
                <Trans>Send Message</Trans>
              </>
            )}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}

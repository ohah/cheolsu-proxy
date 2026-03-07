import { useState, useEffect } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
  Button,
  Input,
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
  Switch,
} from "@/shared/ui";
import { Plus, Trash2, FolderOpen } from "lucide-react";
import { toast } from "sonner";
import { useMapRuleStore } from "@/shared/stores";
import type { InterceptRule, MapLocalAction, MapRemoteAction } from "@/entities/intercept-rule";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";

type MapType = "map_local" | "map_remote";

interface MapRuleFormDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  editingRule: InterceptRule | null;
}

const HTTP_METHODS = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];

export const MapRuleFormDialog = ({
  open: isOpen,
  onOpenChange,
  editingRule,
}: MapRuleFormDialogProps) => {
  const { t } = useLingui();
  const { addRule, updateRule } = useMapRuleStore();

  const [name, setName] = useState("");
  const [pattern, setPattern] = useState("");
  const [method, setMethod] = useState<string>("*");
  const [mapType, setMapType] = useState<MapType>("map_local");

  // Map Local fields
  const [filePath, setFilePath] = useState("");
  const [statusCode, setStatusCode] = useState("200");
  const [headers, setHeaders] = useState<Array<{ key: string; value: string }>>([]);

  // Map Remote fields
  const [targetUrl, setTargetUrl] = useState("");
  const [preservePath, setPreservePath] = useState(true);

  useEffect(() => {
    if (!isOpen) return;

    if (editingRule) {
      setName(editingRule.name);
      setPattern(editingRule.pattern);
      setMethod(editingRule.method ?? "*");

      if (editingRule.action.type === "map_local") {
        const action = editingRule.action as MapLocalAction;
        setMapType("map_local");
        setFilePath(action.file_path);
        setStatusCode(String(action.status_code));
        setHeaders(Object.entries(action.headers).map(([key, value]) => ({ key, value })));
        setTargetUrl("");
        setPreservePath(true);
      } else if (editingRule.action.type === "map_remote") {
        const action = editingRule.action as MapRemoteAction;
        setMapType("map_remote");
        setTargetUrl(action.target_url);
        setPreservePath(action.preserve_path);
        setFilePath("");
        setStatusCode("200");
        setHeaders([]);
      }
    } else {
      setName("");
      setPattern("");
      setMethod("*");
      setMapType("map_local");
      setFilePath("");
      setStatusCode("200");
      setHeaders([]);
      setTargetUrl("");
      setPreservePath(true);
    }
  }, [isOpen, editingRule]);

  const handleSelectFile = async () => {
    try {
      const selected = await openFileDialog({
        multiple: false,
        title: t`Select local file`,
      });
      if (selected) {
        setFilePath(selected as string);
      }
    } catch {
      // user cancelled
    }
  };

  const handleSubmit = () => {
    if (!pattern.trim()) {
      toast.error(t`Pattern is required`);
      return;
    }

    if (mapType === "map_local" && !filePath.trim()) {
      toast.error(t`File path is required`);
      return;
    }

    if (mapType === "map_remote" && !targetUrl.trim()) {
      toast.error(t`Target URL is required`);
      return;
    }

    const action =
      mapType === "map_local"
        ? {
            type: "map_local" as const,
            file_path: filePath.trim(),
            status_code: parseInt(statusCode) || 200,
            headers: Object.fromEntries(
              headers.filter((h) => h.key.trim()).map((h) => [h.key.trim(), h.value]),
            ),
          }
        : {
            type: "map_remote" as const,
            target_url: targetUrl.trim(),
            preserve_path: preservePath,
          };

    const rule: InterceptRule = {
      id: editingRule?.id ?? crypto.randomUUID(),
      name: name.trim() || pattern.trim(),
      enabled: editingRule?.enabled ?? true,
      pattern: pattern.trim(),
      method: method === "*" ? null : method,
      action,
    };

    if (editingRule) {
      updateRule(rule);
      toast.success(t`Rule updated`);
    } else {
      addRule(rule);
      toast.success(t`Rule added`);
    }

    onOpenChange(false);
  };

  const addHeader = () => setHeaders([...headers, { key: "", value: "" }]);
  const removeHeader = (index: number) => setHeaders(headers.filter((_, i) => i !== index));
  const updateHeader = (index: number, field: "key" | "value", value: string) => {
    const updated = [...headers];
    updated[index] = { ...updated[index], [field]: value };
    setHeaders(updated);
  };

  return (
    <Dialog open={isOpen} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[560px] max-h-[80vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>
            {editingRule ? <Trans>Edit Map Rule</Trans> : <Trans>Add Map Rule</Trans>}
          </DialogTitle>
          <DialogDescription>
            <Trans>
              Map Local: respond with a local file. Map Remote: redirect to another URL.
            </Trans>
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {/* Name */}
          <div className="space-y-1.5">
            <label className="text-sm font-medium">
              <Trans>Name</Trans>
            </label>
            <Input
              placeholder={t`Rule name (optional)`}
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </div>

          {/* Pattern */}
          <div className="space-y-1.5">
            <label className="text-sm font-medium">
              <Trans>URL Pattern</Trans> <span className="text-destructive">*</span>
            </label>
            <Input
              placeholder="*.example.com/api/* or https://api.example.com/data"
              value={pattern}
              onChange={(e) => setPattern(e.target.value)}
            />
            <p className="text-xs text-muted-foreground">
              <Trans>* = any string, ? = single character</Trans>
            </p>
          </div>

          {/* Method & Map Type */}
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1.5">
              <label className="text-sm font-medium">
                <Trans>Method</Trans>
              </label>
              <Select value={method} onValueChange={(v) => v && setMethod(v)}>
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="*">{t`All Methods`}</SelectItem>
                  {HTTP_METHODS.map((m) => (
                    <SelectItem key={m} value={m}>
                      {m}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-1.5">
              <label className="text-sm font-medium">
                <Trans>Type</Trans>
              </label>
              <Select value={mapType} onValueChange={(v) => v && setMapType(v as MapType)}>
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="map_local">{t`Map Local`}</SelectItem>
                  <SelectItem value="map_remote">{t`Map Remote`}</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>

          {/* Map Local fields */}
          {mapType === "map_local" && (
            <>
              <div className="space-y-1.5">
                <label className="text-sm font-medium">
                  <Trans>Local File Path</Trans> <span className="text-destructive">*</span>
                </label>
                <div className="flex gap-2">
                  <Input
                    placeholder="/path/to/response.json"
                    value={filePath}
                    onChange={(e) => setFilePath(e.target.value)}
                    className="flex-1"
                  />
                  <Button variant="outline" size="sm" onClick={handleSelectFile}>
                    <FolderOpen className="w-4 h-4" />
                  </Button>
                </div>
              </div>

              <div className="space-y-1.5">
                <label className="text-sm font-medium">
                  <Trans>Status Code</Trans>
                </label>
                <Input
                  type="number"
                  placeholder="200"
                  value={statusCode}
                  onChange={(e) => setStatusCode(e.target.value)}
                />
              </div>

              <div className="space-y-1.5">
                <div className="flex items-center justify-between">
                  <label className="text-sm font-medium">
                    <Trans>Response Headers</Trans>
                  </label>
                  <Button variant="ghost" size="sm" onClick={addHeader}>
                    <Plus className="w-3.5 h-3.5 mr-1" />
                    <Trans>Add</Trans>
                  </Button>
                </div>
                {headers.map((header, i) => (
                  <div key={i} className="flex items-center gap-2">
                    <Input
                      placeholder={t`Header name`}
                      value={header.key}
                      onChange={(e) => updateHeader(i, "key", e.target.value)}
                      className="flex-1"
                    />
                    <Input
                      placeholder={t`Value`}
                      value={header.value}
                      onChange={(e) => updateHeader(i, "value", e.target.value)}
                      className="flex-1"
                    />
                    <Button variant="ghost" size="sm" onClick={() => removeHeader(i)}>
                      <Trash2 className="w-3.5 h-3.5 text-destructive" />
                    </Button>
                  </div>
                ))}
              </div>
            </>
          )}

          {/* Map Remote fields */}
          {mapType === "map_remote" && (
            <>
              <div className="space-y-1.5">
                <label className="text-sm font-medium">
                  <Trans>Target URL</Trans> <span className="text-destructive">*</span>
                </label>
                <Input
                  placeholder="http://localhost:3000 or https://staging.example.com"
                  value={targetUrl}
                  onChange={(e) => setTargetUrl(e.target.value)}
                />
              </div>

              <div className="flex items-center gap-3">
                <Switch checked={preservePath} onCheckedChange={setPreservePath} />
                <div>
                  <label className="text-sm font-medium">
                    <Trans>Preserve Path</Trans>
                  </label>
                  <p className="text-xs text-muted-foreground">
                    <Trans>Append the original request path to the target URL</Trans>
                  </p>
                </div>
              </div>
            </>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            <Trans>Cancel</Trans>
          </Button>
          <Button onClick={handleSubmit}>
            {editingRule ? <Trans>Update</Trans> : <Trans>Add Rule</Trans>}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

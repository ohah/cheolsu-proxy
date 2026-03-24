import { useState, useEffect, useCallback } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/shared/ui";
import { Trash2, Plus } from "lucide-react";
import { toast } from "sonner";
import { SettingsSection } from "./settings-section";

export function ProtoFilesSection() {
  const { t } = useLingui();
  const [protoFiles, setProtoFiles] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);

  const loadFileList = useCallback(async () => {
    try {
      const files = await invoke<string[]>("list_proto_files");
      setProtoFiles(files);
    } catch {
      // proto 파일 기능 미지원 시 무시
    }
  }, []);

  useEffect(() => {
    loadFileList();
  }, [loadFileList]);

  const handleAddFile = useCallback(async () => {
    try {
      const filePaths = await open({
        filters: [{ name: "Proto Files", extensions: ["proto"] }],
        multiple: true,
      });
      if (!filePaths || filePaths.length === 0) return;

      setLoading(true);
      const paths = Array.isArray(filePaths) ? filePaths : [filePaths];
      const count = await invoke<number>("load_proto_files", { paths });
      toast.success(t`Loaded ${count} service(s) from proto files`);
      await loadFileList();
    } catch (err) {
      toast.error(String(err));
    } finally {
      setLoading(false);
    }
  }, [loadFileList, t]);

  const handleRemoveFile = useCallback(
    async (path: string) => {
      try {
        await invoke("remove_proto_file", { path });
        toast.success(t`Proto file removed`);
        await loadFileList();
      } catch (err) {
        toast.error(String(err));
      }
    },
    [loadFileList, t],
  );

  return (
    <SettingsSection
      title={<Trans>gRPC Proto Files</Trans>}
      description={
        <Trans>
          Register .proto files to decode gRPC traffic with actual field names instead of field
          numbers.
        </Trans>
      }
    >
      <Button onClick={handleAddFile} disabled={loading} size="sm">
        <Plus className="w-4 h-4 mr-1" />
        {loading ? <Trans>Loading...</Trans> : <Trans>Add Proto Files</Trans>}
      </Button>

      {protoFiles.length > 0 && (
        <div className="space-y-2">
          {protoFiles.map((file) => (
            <div
              key={file}
              className="flex items-center justify-between bg-muted/50 rounded px-3 py-2"
            >
              <span className="text-sm font-mono truncate flex-1">{file}</span>
              <Button variant="ghost" size="sm" onClick={() => handleRemoveFile(file)}>
                <Trash2 className="w-3.5 h-3.5 text-destructive" />
              </Button>
            </div>
          ))}
        </div>
      )}

      {protoFiles.length === 0 && (
        <p className="text-xs text-muted-foreground">
          <Trans>
            No proto files registered. gRPC messages will be decoded using wire format heuristics.
          </Trans>
        </p>
      )}
    </SettingsSection>
  );
}

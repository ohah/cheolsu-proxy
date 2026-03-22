import { useState, useEffect } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { toast } from "sonner";

import { useHostMappingStore } from "@/shared/stores";
import type { HostMappingInitialValues } from "@/shared/stores";
import type { HostMapping } from "@/shared/api/proxy";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
  Button,
  Input,
} from "@/shared/ui";

function generateId(): string {
  return `hm_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
}

interface HostMappingFormDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  initialValues?: HostMappingInitialValues | null;
}

export const HostMappingFormDialog = ({
  open,
  onOpenChange,
  initialValues,
}: HostMappingFormDialogProps) => {
  const { t } = useLingui();
  const { addMapping } = useHostMappingStore();

  const [sourceHost, setSourceHost] = useState("");
  const [sourcePort, setSourcePort] = useState("");
  const [targetHost, setTargetHost] = useState("");
  const [targetPort, setTargetPort] = useState("");

  useEffect(() => {
    if (!open) return;
    if (initialValues) {
      setSourceHost(initialValues.sourceHost);
      setSourcePort(initialValues.sourcePort);
    } else {
      setSourceHost("");
      setSourcePort("");
    }
    setTargetHost("");
    setTargetPort("");
  }, [open, initialValues]);

  const isValidPort = (port: string): boolean => {
    if (!port) return true;
    const num = parseInt(port, 10);
    return Number.isInteger(num) && num >= 1 && num <= 65535;
  };

  const handleSubmit = () => {
    if (!sourceHost.trim() || !targetHost.trim()) {
      toast.error(t`Source host and target host are required`);
      return;
    }

    if (!isValidPort(sourcePort) || !isValidPort(targetPort)) {
      toast.error(t`Port must be between 1 and 65535`);
      return;
    }

    const mapping: HostMapping = {
      id: generateId(),
      source_host: sourceHost.trim(),
      source_port: sourcePort ? parseInt(sourcePort, 10) : null,
      target_host: targetHost.trim(),
      target_port: targetPort ? parseInt(targetPort, 10) : null,
      enabled: true,
    };

    addMapping(mapping);
    toast.success(t`Host mapping added`);
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[480px]">
        <DialogHeader>
          <DialogTitle>
            <Trans>Add Host Mapping</Trans>
          </DialogTitle>
          <DialogDescription>
            <Trans>Map DNS hostnames to different target hosts for testing and development</Trans>
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-1.5">
              <label className="text-sm font-medium">
                <Trans>Source Host</Trans>
              </label>
              <Input
                placeholder="*.api.example.com"
                value={sourceHost}
                onChange={(e) => setSourceHost(e.target.value)}
              />
            </div>
            <div className="space-y-1.5">
              <label className="text-sm font-medium">
                <Trans>Source Port</Trans>{" "}
                <span className="text-muted-foreground text-xs">
                  (<Trans>optional</Trans>)
                </span>
              </label>
              <Input
                placeholder="443"
                type="number"
                value={sourcePort}
                onChange={(e) => setSourcePort(e.target.value)}
              />
            </div>
          </div>
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-1.5">
              <label className="text-sm font-medium">
                <Trans>Target Host</Trans>
              </label>
              <Input
                placeholder="192.168.1.100"
                value={targetHost}
                onChange={(e) => setTargetHost(e.target.value)}
              />
            </div>
            <div className="space-y-1.5">
              <label className="text-sm font-medium">
                <Trans>Target Port</Trans>{" "}
                <span className="text-muted-foreground text-xs">
                  (<Trans>optional</Trans>)
                </span>
              </label>
              <Input
                placeholder="8443"
                type="number"
                value={targetPort}
                onChange={(e) => setTargetPort(e.target.value)}
              />
            </div>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            <Trans>Cancel</Trans>
          </Button>
          <Button onClick={handleSubmit}>
            <Trans>Add</Trans>
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

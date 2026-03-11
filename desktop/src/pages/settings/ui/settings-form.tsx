import { type ReactNode } from "react";
import { useForm, useFormContext, FormProvider, type UseFormReturn } from "react-hook-form";
import { useAppSettingsStore } from "@/shared/stores/app-settings-store";
import type { ConnectionStrategy } from "@/shared/api/proxy";
import type { DomainClientCertConfig } from "@/shared/api/proxy";

export interface SettingsFormValues {
  // Throttle
  throttle: {
    enabled: boolean;
    preset: string;
    download: string;
    upload: string;
    latency: string;
  };

  // Connection Strategy
  connectionStrategy: ConnectionStrategy;

  // Upstream Proxy
  upstreamProxy: {
    enabled: boolean;
    host: string;
    port: string;
    useAuth: boolean;
    username: string;
    password: string;
    bypass: string;
  };

  // Proxy Auth
  proxyAuth: {
    enabled: boolean;
    username: string;
    password: string;
  };

  // Shortcut
  shortcut: {
    enabled: boolean;
    key: string;
  };

  // Client Certificate
  clientCert: {
    enabled: boolean;
    certPath: string;
    keyPath: string;
    domainCerts: DomainClientCertConfig[];
  };

  // Request Client Certificate
  requestClientCert: {
    enabled: boolean;
    caCertPath: string;
    required: boolean;
  };
}

export function getDefaultValues(): SettingsFormValues {
  const store = useAppSettingsStore.getState();
  return {
    throttle: {
      enabled: store.throttleConfig.enabled,
      preset: store.throttleConfig.preset,
      download: store.throttleConfig.download,
      upload: store.throttleConfig.upload,
      latency: store.throttleConfig.latency,
    },
    connectionStrategy: store.connectionStrategy,
    upstreamProxy: {
      enabled: store.upstreamProxyConfig.enabled,
      host: store.upstreamProxyConfig.host,
      port: String(store.upstreamProxyConfig.port),
      useAuth: !!store.upstreamProxyConfig.auth,
      username: store.upstreamProxyConfig.auth?.username ?? "",
      password: store.upstreamProxyConfig.auth?.password ?? "",
      bypass: store.upstreamProxyConfig.bypass.join(", "),
    },
    proxyAuth: {
      enabled: store.proxyAuthConfig.enabled,
      username: store.proxyAuthConfig.username,
      password: store.proxyAuthConfig.password,
    },
    shortcut: {
      enabled: store.proxyToggleShortcutEnabled,
      key: store.proxyToggleShortcut,
    },
    clientCert: {
      enabled: false,
      certPath: "",
      keyPath: "",
      domainCerts: [],
    },
    requestClientCert: {
      enabled: false,
      caCertPath: "",
      required: false,
    },
  };
}

export function SettingsFormProvider({ children }: { children: ReactNode }) {
  const form = useForm<SettingsFormValues>({
    defaultValues: getDefaultValues(),
  });

  return <FormProvider {...form}>{children}</FormProvider>;
}

export function useSettingsForm(): UseFormReturn<SettingsFormValues> {
  return useFormContext<SettingsFormValues>();
}

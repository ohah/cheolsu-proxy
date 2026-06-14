import { useEffect } from "react";
import {
  useInterceptRuleStore,
  useMapRuleStore,
  useBreakpointStore,
  useHostMappingStore,
  useReverseProxyStore,
} from "@/shared/stores";
import { useSslProxyingStore } from "@/shared/stores/ssl-proxying-store";
import { useContractStore } from "@/shared/stores/contract-store";
import { listen } from "@tauri-apps/api/event";
import { updateDaemonRules } from "@/shared/stores/sync-rules";
import type { InterceptRule } from "@/entities/intercept-rule";
import type { BreakpointRule, PendingBreakpoint } from "@/entities/breakpoint";
import type {
  HostMapping,
  ReverseProxyRule,
  SslProxyingMode,
  SslProxyingEntry,
} from "@/shared/api/proxy";
import type { ContractSpecInfo } from "@/entities/proxy";

/**
 * 데몬에서 전달되는 규칙 변경 이벤트(인터셉트, 브레이크포인트, 호스트 매핑,
 * 리버스 프록시, SSL Proxying, Contract Testing 등)를 수신하는 훅
 */
export function useDaemonSyncListeners() {
  const setInterceptRules = useInterceptRuleStore((s) => s.setRules);
  const setMapRules = useMapRuleStore((s) => s.setRules);
  const setBreakpointRules = useBreakpointStore((s) => s.setRules);
  const addPendingBreakpoint = useBreakpointStore((s) => s.addPendingBreakpoint);
  const setHostMappings = useHostMappingStore((s) => s.setMappings);
  const setReverseProxyRules = useReverseProxyStore((s) => s.setRules);
  const setSslFromDaemon = useSslProxyingStore((s) => s.setFromDaemon);
  const setDefaultPassthroughFromDaemon = useSslProxyingStore(
    (s) => s.setDefaultPassthroughFromDaemon,
  );
  const setContractSpecs = useContractStore((s) => s.setSpecs);

  // 데몬에서 인터셉트 규칙 변경 수신 (MCP 등 외부 클라이언트에서 변경 시 동기화)
  useEffect(() => {
    const unlisten = listen<InterceptRule[]>("intercept_rules_updated", (event) => {
      const rules = event.payload;
      updateDaemonRules(rules);
      const mapRules = rules.filter(
        (r) => r.action.type === "map_local" || r.action.type === "map_remote",
      );
      // map 규칙을 제외한 모든 규칙(block/modify/rewrite/throttle 등)은 intercept로 분류한다.
      // (과거엔 화이트리스트에서 rewrite/throttle이 빠져 브로드캐스트 시 GUI에서 소실됐다)
      const interceptRules = rules.filter(
        (r) => r.action.type !== "map_local" && r.action.type !== "map_remote",
      );
      setInterceptRules(interceptRules);
      setMapRules(mapRules);
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, [setInterceptRules, setMapRules]);

  // Breakpoint 이벤트 수신
  useEffect(() => {
    const unlistenRules = listen<BreakpointRule[]>("breakpoint_rules_updated", (event) => {
      setBreakpointRules(event.payload);
    });
    const unlistenHit = listen<PendingBreakpoint>("breakpoint_hit", (event) => {
      addPendingBreakpoint(event.payload);
    });

    return () => {
      unlistenRules.then((f) => f());
      unlistenHit.then((f) => f());
    };
  }, [setBreakpointRules, addPendingBreakpoint]);

  // 데몬에서 호스트 매핑 변경 수신
  useEffect(() => {
    const unlisten = listen<HostMapping[]>("host_mappings_updated", (event) => {
      setHostMappings(event.payload);
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, [setHostMappings]);

  // 데몬에서 리버스 프록시 규칙 변경 수신
  useEffect(() => {
    const unlisten = listen<ReverseProxyRule[]>("reverse_proxy_rules_updated", (event) => {
      setReverseProxyRules(event.payload);
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, [setReverseProxyRules]);

  // 데몬에서 SSL Proxying 목록 변경 수신
  useEffect(() => {
    const unlisten = listen<{
      mode: SslProxyingMode;
      entries: SslProxyingEntry[];
    }>("ssl_proxying_list_updated", (event) => {
      setSslFromDaemon(event.payload.mode, event.payload.entries);
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, [setSslFromDaemon]);

  // 데몬에서 기본 패스스루 도메인 변경 수신
  useEffect(() => {
    const unlisten = listen<{ entries: SslProxyingEntry[] }>(
      "default_passthrough_domains_updated",
      (event) => {
        setDefaultPassthroughFromDaemon(event.payload.entries);
      },
    );

    return () => {
      unlisten.then((f) => f());
    };
  }, [setDefaultPassthroughFromDaemon]);

  // Contract Testing 스펙 업데이트 수신
  useEffect(() => {
    const unlisten = listen<ContractSpecInfo[]>("contract_specs_updated", (event) => {
      setContractSpecs(event.payload);
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, [setContractSpecs]);
}

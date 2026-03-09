import { updateInterceptRules } from "@/shared/api/proxy";
import type { InterceptRule } from "@/entities/intercept-rule";

type StoreGetter = () => { rules: InterceptRule[] };

const ruleGetters: StoreGetter[] = [];

/** 데몬 broadcast로 받은 현재 규칙 (외부 규칙 보존용) */
let daemonRules: InterceptRule[] = [];

export function registerRuleStore(getter: StoreGetter) {
  ruleGetters.push(getter);
}

/** 데몬에서 broadcast된 전체 규칙을 추적 */
export function updateDaemonRules(rules: InterceptRule[]) {
  daemonRules = rules;
}

export async function syncAllRulesToProxy() {
  const appRules = ruleGetters.flatMap((getter) => getter().rules);
  const appRuleIds = new Set(appRules.map((r) => r.id));
  // 앱 store에 없는 외부 규칙(MCP/TUI 등) 보존
  const externalRules = daemonRules.filter((r) => !appRuleIds.has(r.id));
  await updateInterceptRules([...appRules, ...externalRules]);
}

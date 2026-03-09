import { updateInterceptRules } from "@/shared/api/proxy";
import type { InterceptRule } from "@/entities/intercept-rule";

type StoreGetter = () => { rules: InterceptRule[] };

const ruleGetters: StoreGetter[] = [];

/** 데몬 broadcast로 받은 현재 규칙 (외부 규칙 보존용) */
let daemonRules: InterceptRule[] = [];

/** 첫 번째 데몬 규칙 수신을 기다리는 resolve 함수 */
let daemonRulesResolve: (() => void) | null = null;
let daemonRulesReady = new Promise<void>((resolve) => {
  daemonRulesResolve = resolve;
});

export function registerRuleStore(getter: StoreGetter) {
  ruleGetters.push(getter);
}

/** 데몬에서 broadcast된 전체 규칙을 추적 */
export function updateDaemonRules(rules: InterceptRule[]) {
  daemonRules = rules;
  if (daemonRulesResolve) {
    daemonRulesResolve();
    daemonRulesResolve = null;
  }
}

/** 첫 번째 데몬 규칙 broadcast 수신까지 대기 (최대 2초 타임아웃) */
export function waitForDaemonRules(): Promise<void> {
  return Promise.race([daemonRulesReady, new Promise<void>((r) => setTimeout(r, 2000))]);
}

/** 테스트용: 모듈 상태 초기화 */
export function _resetForTest() {
  ruleGetters.length = 0;
  daemonRules = [];
  daemonRulesReady = new Promise<void>((resolve) => {
    daemonRulesResolve = resolve;
  });
}

export async function syncAllRulesToProxy() {
  const appRules = ruleGetters.flatMap((getter) => getter().rules);
  const appRuleIds = new Set(appRules.map((r) => r.id));
  // 앱 store에 없는 외부 규칙(MCP/TUI 등) 보존
  const externalRules = daemonRules.filter((r) => !appRuleIds.has(r.id));
  await updateInterceptRules([...appRules, ...externalRules]);
}

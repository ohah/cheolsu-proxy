import { updateInterceptRules } from "@/shared/api/proxy";
import type { InterceptRule } from "@/entities/intercept-rule";

type StoreGetter = () => { rules: InterceptRule[] };

const ruleGetters: StoreGetter[] = [];

export function registerRuleStore(getter: StoreGetter) {
  ruleGetters.push(getter);
}

export async function syncAllRulesToProxy() {
  const allRules = ruleGetters.flatMap((getter) => getter().rules);
  await updateInterceptRules(allRules);
}

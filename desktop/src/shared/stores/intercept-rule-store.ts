import { createRuleStore } from "./create-rule-store";

export const useInterceptRuleStore = createRuleStore({
  storeName: "cheolsu-intercept-rules",
  errorLabel: "intercept",
});

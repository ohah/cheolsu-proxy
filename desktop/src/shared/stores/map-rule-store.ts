import { createRuleStore } from "./create-rule-store";

export const useMapRuleStore = createRuleStore({
  storeName: "cheolsu-map-rules",
  errorLabel: "map",
});

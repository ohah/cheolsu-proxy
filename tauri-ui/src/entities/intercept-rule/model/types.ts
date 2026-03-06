export type InterceptActionType = "block" | "modify_request" | "modify_response";

export interface BlockAction {
  type: "block";
  status_code: number;
  body: string;
}

export interface ModifyRequestAction {
  type: "modify_request";
  add_headers: Record<string, string>;
  remove_headers: string[];
  set_body: string | null;
}

export interface ModifyResponseAction {
  type: "modify_response";
  set_status: number | null;
  add_headers: Record<string, string>;
  remove_headers: string[];
  set_body: string | null;
}

export type InterceptAction = BlockAction | ModifyRequestAction | ModifyResponseAction;

export interface InterceptRule {
  id: string;
  name: string;
  enabled: boolean;
  pattern: string;
  method: string | null;
  action: InterceptAction;
}

export interface InterceptRuleStoreState {
  rules: InterceptRule[];
  addRule: (rule: InterceptRule) => void;
  updateRule: (rule: InterceptRule) => void;
  removeRule: (id: string) => void;
  toggleRule: (id: string) => void;
  clearRules: () => void;
  syncToProxy: () => Promise<void>;
}

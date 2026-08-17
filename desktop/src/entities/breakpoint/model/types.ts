export interface BreakpointRule {
  id: string;
  pattern: string;
  break_on_request: boolean;
  break_on_response: boolean;
  enabled: boolean;
}

export type BreakpointPhase = "request" | "response";

export interface BreakpointData {
  method: string;
  url: string;
  headers: Record<string, string>;
  body?: string;
  status?: number;
}

export interface PendingBreakpoint {
  id: string;
  transaction_id: string;
  phase: BreakpointPhase;
  data: BreakpointData;
}

export type BreakpointActionType = "forward" | "modify_and_forward" | "drop" | "abort";

export interface ForwardAction {
  type: "forward";
}

export interface ModifyAndForwardAction {
  type: "modify_and_forward";
  headers?: Record<string, string>;
  body?: string;
  status?: number;
}

export interface DropAction {
  type: "drop";
}

export interface AbortAction {
  type: "abort";
}

export type BreakpointAction = ForwardAction | ModifyAndForwardAction | DropAction | AbortAction;

export interface BreakpointStoreState {
  rules: BreakpointRule[];
  pendingBreakpoints: PendingBreakpoint[];
  addRule: (rule: BreakpointRule) => void;
  removeRule: (id: string) => void;
  toggleRule: (id: string) => void;
  clearRules: () => void;
  setRules: (rules: BreakpointRule[]) => void;
  addPendingBreakpoint: (bp: PendingBreakpoint) => void;
  removePendingBreakpoint: (id: string) => void;
  resolveBreakpoint: (id: string, action: BreakpointAction) => Promise<void>;
  syncToProxy: () => void;
}

import { HttpTransaction } from "@/entities/proxy"

export interface HostNode {
  name: string
  path: string
  children: Map<string, HostNode>
  transactions: HttpTransaction[]
  type: "host" | "folder" | "endpoint"
}

export type NodeType = HostNode['type']

export interface TreeNodeAction {
  onToggleExpanded: (path: string) => void
  onTransactionSelect: (transaction: HttpTransaction) => void
}

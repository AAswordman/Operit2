export type Value = { value: string } | { nodeId: string };
export type NodeKind = "trigger" | "execute" | "condition" | "logic" | "extract";
export type Comparison = "EQ" | "NE" | "GT" | "GTE" | "LT" | "LTE" | "CONTAINS" | "NOT_CONTAINS" | "IN" | "NOT_IN";
export type ExtractMode = "REGEX" | "JSON" | "SUB" | "CONCAT" | "RANDOM_INT" | "RANDOM_STRING";
export interface BaseNode { id: string; type: NodeKind; name: string; description: string; position: { x: number; y: number } }
export interface Trigger extends BaseNode { type: "trigger"; triggerType: "manual" | "schedule" | "app_open" | "event"; triggerConfig: Record<string, string> }
export interface Execute extends BaseNode { type: "execute"; actionType: string; actionConfig: Record<string, Value>; jsCode: string | null }
export interface Condition extends BaseNode { type: "condition"; left: Value; operator: Comparison; right: Value }
export interface Logic extends BaseNode { type: "logic"; operator: "AND" | "OR" }
export interface Extract extends BaseNode {
  type: "extract"; source: Value; mode: ExtractMode; expression: string; group: number;
  others: Value[]; startIndex: number; length: number; randomMin: number; randomMax: number;
  randomStringLength: number; randomStringCharset: string; useFixed: boolean; fixedValue: string;
}
export type WorkflowNode = Trigger | Execute | Condition | Logic | Extract;
export interface Connection { id: string; sourceNodeId: string; targetNodeId: string; condition: string | null }
export interface Workflow {
  id: string; name: string; description: string; nodes: WorkflowNode[]; connections: Connection[];
  createdAt: number; updatedAt: number; enabled: boolean; revision: number;
  totalExecutions: number; successfulExecutions: number; failedExecutions: number;
  lastExecutionTime: number | null; lastExecutionStatus: RunStatus | null;
}
export type RunStatus = "RUNNING" | "SUCCESS" | "FAILED" | "CANCELLED";
export type NodeStatus = "pending" | "running" | "success" | "failed" | "skipped";
export interface NodeResult { status: NodeStatus; output: string; startedAt: number | null; finishedAt: number | null }
export interface LogEntry { time: number; nodeId: string; level: "info" | "error"; message: string }
export interface Run {
  id: string; workflowId: string; workflowName: string; triggerId: string | null;
  status: RunStatus; startedAt: number; finishedAt: number | null;
  nodes: Record<string, NodeResult>; logs: LogEntry[];
}
export interface ToolParameterSchema {
  name: string;
  type: string;
  description: string;
  required: boolean;
  default: string | null;
}
export interface ToolDefinition {
  name: string;
  description: string;
  parameters: ToolParameterSchema[];
  category: string;
  source: string;
  packageName?: string;
}
export interface ManifestWorkflowTemplate {
  sourceToolPkgId: string;
  sourceVersion: string;
  templateId: string;
  displayName: string;
  description: string;
  resourceKey: string;
  workflow: Workflow;
}
export interface Snapshot {
  workflows: Workflow[];
  runs: Run[];
  manifestTemplates: ManifestWorkflowTemplate[];
  tools?: ToolDefinition[];
}
export const STYLES: Record<NodeKind, { label: string; color: string; tint: string; border: string }> = {
  trigger: { label: "触发", color: "#4CAF50", tint: "#E8F5E9", border: "#81C784" },
  execute: { label: "执行", color: "#2196F3", tint: "#E3F2FD", border: "#64B5F6" },
  condition: { label: "条件", color: "#FF9800", tint: "#FFF3E0", border: "#FFB74D" },
  logic: { label: "逻辑", color: "#7E57C2", tint: "#F3E5F5", border: "#B39DDB" },
  extract: { label: "运算", color: "#009688", tint: "#E0F2F1", border: "#4DB6AC" },
};

/** Creates a collision-resistant identifier within a plugin runtime. */
export function id(prefix: string): string {
  return `${prefix}_${Date.now().toString(36)}_${Math.random().toString(36).slice(2)}_${Math.random().toString(36).slice(2)}`;
}

/** Copies JSON-owned workflow state without sharing mutable references. */
export function copy<T>(value: T): T { return JSON.parse(JSON.stringify(value)) as T; }

/** Creates the explicit initial configuration for each supported node type. */
export function newNode(type: NodeKind, x = 40, y = 40): WorkflowNode {
  const base = { id: id("node"), name: STYLES[type].label, description: "", position: { x, y } };
  switch (type) {
    case "trigger": return { ...base, type, triggerType: "manual", triggerConfig: {} };
    case "execute": return { ...base, type, actionType: "", actionConfig: {}, jsCode: null };
    case "condition": return { ...base, type, left: { value: "" }, operator: "EQ", right: { value: "" } };
    case "logic": return { ...base, type, operator: "AND" };
    case "extract": return { ...base, type, source: { value: "" }, mode: "REGEX", expression: "", group: 0,
      others: [], startIndex: 0, length: -1, randomMin: 0, randomMax: 100, randomStringLength: 8,
      randomStringCharset: "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789", useFixed: false, fixedValue: "" };
  }
}

/** Creates the blank workflow shown by the Kotlin creation dialog. */
export function newWorkflow(name: string, description = ""): Workflow {
  return { id: id("workflow"), name, description, nodes: [], connections: [],
    createdAt: Date.now(), updatedAt: Date.now(), enabled: true, revision: 0, totalExecutions: 0,
    successfulExecutions: 0, failedExecutions: 0, lastExecutionTime: null, lastExecutionStatus: null };
}

/** Resets identities and statistics while preserving all internal references. */
export function duplicate(workflow: Workflow): Workflow {
  const result = copy(workflow);
  const ids = new Map(result.nodes.map(node => [node.id, id("node")]));
  /** Rewrites a parameter reference into the duplicated graph. */
  function remap(value: Value): Value {
    if ("value" in value) return value;
    const next = ids.get(value.nodeId);
    if (next === undefined) throw new Error(`引用节点不存在：${value.nodeId}`);
    return { nodeId: next };
  }
  for (const node of result.nodes) {
    node.id = ids.get(node.id)!;
    if (node.type === "execute") node.actionConfig = Object.fromEntries(Object.entries(node.actionConfig).map(([key, value]) => [key, remap(value)]));
    if (node.type === "condition") { node.left = remap(node.left); node.right = remap(node.right); }
    if (node.type === "extract") { node.source = remap(node.source); node.others = node.others.map(remap); }
  }
  result.connections = result.connections.map(edge => ({ ...edge, id: id("edge"), sourceNodeId: ids.get(edge.sourceNodeId)!, targetNodeId: ids.get(edge.targetNodeId)! }));
  return { ...result, id: id("workflow"), revision: 0, createdAt: Date.now(), updatedAt: Date.now(),
    totalExecutions: 0, successfulExecutions: 0, failedExecutions: 0, lastExecutionTime: null, lastExecutionStatus: null };
}

/** Returns the reference-bearing parameters of a node. */
export function values(node: WorkflowNode): Value[] {
  switch (node.type) {
    case "execute": return Object.values(node.actionConfig);
    case "condition": return [node.left, node.right];
    case "extract": return [node.source, ...node.others];
    default: return [];
  }
}

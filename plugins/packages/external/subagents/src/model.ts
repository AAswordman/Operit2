import type { Chat } from "../../../../types/chat.js";
import type { MessageSendOutcome } from "../../../../types/results.js";

export const PACKAGE_ID = "com.operit.subagents";
export const CHANNEL = "subagents.control";
export const SCHEMA_VERSION = 1;
export const DEFAULT_WAIT_MS = 30_000;
export const MAX_ACTIVE_AGENTS = 8;

export type AgentStatus = "starting" | "running" | "completed" | "cancelled" | "failed" | "closing" | "closed" | "interrupted";

/** Keeps the exact native outcome while excluding host result methods from durable records. */
export interface Receipt {
  response: string | null;
  outcome: MessageSendOutcome;
}

/** Stores one whole semantic message snapshot; revisions replace parts rather than append deltas. */
export interface MessageSnapshot {
  messageTimestamp: number;
  revision: number;
  parts: Chat.MessagePart[];
}

/** Describes one agent's latest turn without duplicating its potentially large streamed parts. */
export interface AgentSummary {
  schemaVersion: 1;
  id: string;
  parentChatId: string;
  chatId: string;
  name: string;
  task: string;
  participantId: string | null;
  status: AgentStatus;
  runNumber: number;
  lastInput: string;
  createdAt: number;
  updatedAt: number;
  error: string | null;
  result: Receipt | null;
}

/** Persists current execution details independently of the parent composer's small status record. */
export interface Agent extends AgentSummary {
  messages: MessageSnapshot[];
}

/** Publishes only the children belonging to one exact originating conversation. */
export interface ParentSnapshot {
  schemaVersion: 1;
  agents: AgentSummary[];
}

/** Reports the observed turns; timeout belongs to this wait, never to the agents themselves. */
export interface WaitResult {
  timed_out: boolean;
  agents: AgentSummary[];
}

export type Request =
  | { action: "spawn"; parentChatId: string; task: string; name: string; participantId: string | null }
  | { action: "list"; parentChatId: string }
  | { action: "wait"; parentChatId: string; agentIds: string[]; timeoutMs: number }
  | { action: "send"; parentChatId: string; agentId: string; message: string; interrupt: boolean }
  | { action: "close"; parentChatId: string; agentId: string };

export type Response = AgentSummary | ParentSnapshot | WaitResult;

/** Requires authored nonblank text without silently converting or replacing invalid input. */
export function requireText(value: unknown, label: string): string {
  if (typeof value !== "string" || value.trim() === "") throw new Error(label + " must be nonblank text");
  return value;
}

/** Formats an actual thrown value for the task record and diagnostics. */
export function errorText(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/** Projects a durable agent into its small composer and tool representation. */
export function summary(agent: Agent): AgentSummary {
  const { messages: _messages, ...value } = agent;
  return value;
}

/** Separates terminal task states from execution and cancellation still in progress. */
export function isActive(status: AgentStatus): boolean {
  return status === "starting" || status === "running" || status === "closing";
}

/** Validates durable records before they can drive controls or be displayed as execution state. */
export function parseAgent(value: unknown): Agent {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error("Invalid subagent record");
  const agent = value as Agent;
  if (agent.schemaVersion !== SCHEMA_VERSION) throw new Error("Unsupported subagent schema version");
  for (const field of ["id", "parentChatId", "chatId", "name", "task", "lastInput"] as const) requireText(agent[field], field);
  const statuses: AgentStatus[] = ["starting", "running", "completed", "cancelled", "failed", "closing", "closed", "interrupted"];
  if (statuses.indexOf(agent.status) === -1) throw new Error("Invalid subagent status");
  if (!Number.isSafeInteger(agent.runNumber) || agent.runNumber < 1) throw new Error("Invalid subagent turn number");
  if (!Number.isFinite(agent.createdAt) || !Number.isFinite(agent.updatedAt)) throw new Error("Invalid subagent timestamps");
  if (agent.participantId !== null) requireText(agent.participantId, "participantId");
  if (agent.error !== null && typeof agent.error !== "string") throw new Error("Invalid subagent error");
  if (!Array.isArray(agent.messages)) throw new Error("Invalid subagent message snapshots");
  for (const message of agent.messages) {
    if (!Number.isSafeInteger(message.messageTimestamp) || !Number.isSafeInteger(message.revision) || !Array.isArray(message.parts)) throw new Error("Invalid subagent message snapshot");
  }
  if (agent.result !== null) {
    if (typeof agent.result !== "object" || agent.result.outcome === null || typeof agent.result.outcome !== "object") throw new Error("Invalid subagent receipt");
    if (agent.result.response !== null && typeof agent.result.response !== "string") throw new Error("Invalid subagent response");
    const outcome = agent.result.outcome;
    if (outcome.type !== "committed" && outcome.type !== "not_persisted" && outcome.type !== "blocked" && outcome.type !== "consumed") throw new Error("Invalid subagent outcome");
    if ((outcome.type === "committed" || outcome.type === "not_persisted") && outcome.status !== "completed" && outcome.status !== "cancelled") throw new Error("Invalid native turn status");
  }
  return agent;
}

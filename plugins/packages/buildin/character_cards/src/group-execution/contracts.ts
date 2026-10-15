import { jsonValue } from "../domain";
import { assertInteger } from "../validation";
import type { ToolPkg as ToolPkgTypes } from "../../../../../types/index";

import type { Chat } from "../../../../../types/chat";
import type { ChatDisplayParticipant } from "../api";
import type { GroupResponsePlan } from "./planner";

/** Identifies an actual canonical message revision without looking for the newest assistant message. */
export interface GroupMessageLocator { messageTimestamp: number; variantIndex: number }
/** Reuses the SDK's complete attachment contract without redeclaring its fields. */
export type GroupSubmissionAttachment = Chat.SendAttachment;
/** Retains the complete original input; these plugin-local types are not declarations of unimplemented SDK methods. */
export interface GroupSubmission {
  submissionId: string;
  chatId: string;
  selection: string;
  runtime: "main" | "floating";
  notifyReply: boolean;
  text: string;
  attachments: GroupSubmissionAttachment[];
  replyToMessageTimestamp: number | null;
}
/** Selects an explicit opaque participant; the native resolver authors the complete persisted snapshot exactly once. */
export interface GroupSelectedTurn { requestKey: string; selection: string; participantId: string; notifyReply: boolean }
/** Distinguishes real user-only persistence from execution instead of assigning an arbitrary speaker to an empty plan. */
export type GroupInitialTurn =
  | { kind: "execute"; submission: GroupSubmission; turn: GroupSelectedTurn }
  | { kind: "record_only"; submission: GroupSubmission; requestKey: string };
/** Reuses a real persisted user message and cannot submit the original input a second time. */
export interface GroupContinuationTurn {
  submissionId: string;
  chatId: string;
  runtime: "main" | "floating";
  userMessageTimestamp: number;
  turn: GroupSelectedTurn;
}
/** Requires explicit completion state and exact persisted reply identity from the transport. */
export interface GroupTurnReceipt {
  status: "completed" | "cancelled";
  chatId: string;
  requestKey: string;
  userMessageTimestamp: number | null;
  assistant: GroupMessageLocator | null;
}
/** Preserves actual noncommit and input-ownership results rather than treating them as completed generations. */
export type GroupSendDisposition =
  | { type: "not_persisted"; status: "completed" | "cancelled" }
  | { type: "blocked"; message: string | null }
  | { type: "consumed"; metadata: ToolPkgTypes.JsonObject };
/** Distinguishes a validated native commit receipt from the three explicit noncommit outcomes. */
export type GroupSendDecision = { type: "committed"; receipt: GroupTurnReceipt } | GroupSendDisposition;

/** Describes one complete sequence for generic finalization after all planned replies have settled. */
export interface GroupSequenceCompletion {
  submissionId: string;
  chatId: string;
  runtime: "main" | "floating";
  userMessageTimestamp: number;
  receipts: GroupTurnReceipt[];
}
/** Requires actual submission, continuation, cancellation and finalization implementations; no default transport exists. */
export interface GroupTurnTransport {
  /** Persists the full initial input exactly once and optionally executes the explicitly selected snapshot. */
  submit(input: GroupInitialTurn): Promise<MessageSendResultData>;
  /** Executes against the named existing user message without writing another user message. */
  continue(input: GroupContinuationTurn): Promise<MessageSendResultData>;
  /** Checks the local active attempt and asks the host to capture this authenticated owner's execution in the named chat. */
  cancel(input: { chatId: string; requestKey: string; runtime: "main" | "floating" }): Promise<void>;
  /** Verifies plugin-local planning completion after every actual send has been finalized by the host. */
  finish(input: GroupSequenceCompletion): Promise<void>;
  /** Closes plugin-local planning state after a stopped or failed plan; native cleanup remains host-owned. */
  abandon(input: { submissionId: string; chatId: string; runtime: "main" | "floating" }): Promise<void>;
}
/** Connects the controller to the existing single service and real functional planner, never another repository. */
export interface GroupExecutionDependencies {
  /** Requires the current owner-isolated selection from the actual conversation record. */
  readSelection(chatId: string): Promise<string>;
  /** Reads real ordered group members from the shared plugin domain service. */
  readParticipants(selection: string): Promise<ChatDisplayParticipant[]>;
  /** Calls the actual functional planner or an explicitly controlled test AI boundary. */
  plan(participants: ChatDisplayParticipant[], text: string): Promise<GroupResponsePlan>;
  transport: GroupTurnTransport;
}
/** Reports exact progress; cancellation during planning may truthfully have no persisted user message yet. */
export interface GroupExecutionOutcome {
  status: "completed" | "cancelled" | "blocked" | "consumed" | "not_persisted";
  disposition: GroupSendDisposition | null;
  submissionId: string;
  chatId: string;
  selection: string;
  cursor: number;
  plannedTurns: number;
  userMessageTimestamp: number | null;
  receipts: GroupTurnReceipt[];
}

import type { MessageSendResultData } from "../../../../../types/results";

/** Validates real JSON objects without casting an unimplemented SDK transport or accepting alternate field names. */
function object(value: unknown, path: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error(path + " must be an object");
  return value as Record<string, unknown>;
}

/** Checks every required tagged-outcome field against the actual Rust serde contract. */
function fields(value: Record<string, unknown>, expected: string[]): void {
  const keys = Object.keys(value).sort();
  if (keys.join(",") !== [...expected].sort().join(",")) throw new Error("Native send outcome has unexpected or missing fields");
}

/** Requires a genuine safe canonical message revision rather than deriving one from sentAt or receivedAt. */
function assistant(value: unknown): GroupMessageLocator {
  const record = object(value, "Native assistant locator"); fields(record, ["messageTimestamp", "variantIndex"]);
  assertInteger(record.messageTimestamp, "native assistant timestamp", 0); assertInteger(record.variantIndex, "native assistant variant", 0);
  if (record.variantIndex > 2147483647) throw new Error("Native assistant variant exceeds the host i32 range");
  return { messageTimestamp: record.messageTimestamp, variantIndex: record.variantIndex };
}

/** Requires the native completion state without translating hook ownership into an execution status. */
function status(value: unknown): "completed" | "cancelled" {
  if (value !== "completed" && value !== "cancelled") throw new Error("Native send status must be completed or cancelled");
  return value;
}

/** Maps an actual send result's tagged outcome, retaining nullable cancellation locators and every explicit noncommit state. */
export function decodeNativeGroupSendResult(
  value: MessageSendResultData, input: GroupInitialTurn | GroupContinuationTurn, previousTimestamp: number | null,
): GroupSendDecision {
  const result = object(value, "Native send result"), initial = "kind" in input;
  const chatId = initial ? input.submission.chatId : input.chatId;
  const requestKey = initial && input.kind === "record_only" ? input.requestKey : input.turn.requestKey;
  if (result.chatId !== chatId) throw new Error("Native send result belongs to a different chat");
  if (typeof result.message !== "string") throw new Error("Native send result must retain its real submitted text");
  assertInteger(result.sentAt, "native send sentAt", 0);
  const outcome = object(result.outcome, "Native send outcome");
  switch (outcome.type) {
    case "committed": {
      fields(outcome, ["type", "status", "userMessageTimestamp", "assistant"]);
      const terminal = status(outcome.status);
      let userMessageTimestamp: number | null;
      if (outcome.userMessageTimestamp === null) userMessageTimestamp = null;
      else { const timestamp = outcome.userMessageTimestamp; assertInteger(timestamp, "native user message timestamp", 1); userMessageTimestamp = timestamp; }
      if (previousTimestamp !== null && userMessageTimestamp !== null && userMessageTimestamp !== previousTimestamp) throw new Error("Group continuation created or referenced a different user message");
      const located = outcome.assistant === null ? null : assistant(outcome.assistant);
      if (terminal === "completed" && userMessageTimestamp === null) throw new Error("Completed group commit requires the actual user message timestamp");
      if (located !== null && userMessageTimestamp === null) throw new Error("Group assistant commit has no actual originating user message");
      if (initial && input.kind === "record_only" && located !== null) throw new Error("User-only group commit cannot declare an assistant reply");
      if (!(initial && input.kind === "record_only") && terminal === "completed" && located === null) throw new Error("Completed group execution requires an actual assistant locator");
      return { type: "committed", receipt: { status: terminal, chatId, requestKey, userMessageTimestamp, assistant: located } };
    }
    case "not_persisted": fields(outcome, ["type", "status"]); return { type: "not_persisted", status: status(outcome.status) };
    case "blocked": {
      fields(outcome, ["type", "message"]);
      if (outcome.message !== null && typeof outcome.message !== "string") throw new Error("Blocked send message must be explicit text or null");
      return { type: "blocked", message: outcome.message };
    }
    case "consumed": {
      fields(outcome, ["type", "metadata"]);
      const metadata = jsonValue(outcome.metadata, "native consumed metadata");
      if (metadata === null || typeof metadata !== "object" || Array.isArray(metadata)) throw new Error("Consumed send metadata must be a JSON object");
      return { type: "consumed", metadata };
    }
    default: throw new Error("Unknown native send outcome type");
  }
}

import type { ToolPkg as ToolPkgTypes } from "../../../../../types/index";
import { decodeChatMarker } from "../chat-markers";
import { parseChatSelection } from "../chat-bindings";
import { jsonValue } from "../domain";
import { getService } from "../service-runtime";
import { assertObject, requireId } from "../validation";
import type { GroupExecutionOutcome, GroupSubmission } from "./contracts";
import { createGroupExecutionController } from "./domain-adapter";
import { GroupExecutionController } from "./executor";
import { NativeGroupTurnTransport } from "./native-transport";

/** Retains one exact admitted submission and its genuine asynchronous outcome for each conversation. */
interface InputExecution {
  input: GroupSubmission;
  status: "running" | "settled" | "failed";
  outcome: GroupExecutionOutcome | null;
  failure: unknown;
}

let controller: GroupExecutionController | null = null;
let submissionSequence = 0;
const executions = new Map<string, InputExecution>();

/** Maps the SDK-typed submit payload while enforcing only its nullable fields and attachment consistency. */
function submission(payload: ToolPkgTypes.ChatInputEventPayload, chatId: string, selection: string): GroupSubmission {
  const { runtime, text, attachments } = payload;
  if (runtime === null) throw new Error("Group input requires an actual send-capable runtime slot");
  if (text === undefined) throw new Error("Group input requires the submitted text field");
  if (attachments === null) throw new Error("Group input requires complete native attachments");
  if (payload.attachmentCount !== attachments.length || payload.hasAttachments !== (attachments.length !== 0)) {
    throw new Error("Group input attachment counts differ from the complete native input");
  }
  return {
    submissionId: "group-input-" + (++submissionSequence), chatId, selection,
    runtime, notifyReply: payload.notifyReply, text, attachments,
    replyToMessageTimestamp: payload.replyToMessageTimestamp,
  };
}

/** Records actual settlement or the original failure without retrying or manufacturing a successful receipt. */
function observe(execution: InputExecution, work: Promise<GroupExecutionOutcome>): void {
  void work.then(
    /** Publishes only the controller's validated result and precise native message locators. */
    outcome => { execution.status = "settled"; execution.outcome = outcome; },
    /** Retains the original rejected error and exposes the background failure through the plugin log. */
    failure => { execution.status = "failed"; execution.failure = failure; NativeInterface.logError("Group input " + execution.input.submissionId + " failed: " + String(failure)); },
  );
}

/** Consumes host-origin group input without awaiting planning or generation inside the submit hook. */
export async function onGroupInputSubmit(event: ToolPkgTypes.ChatInputHookEvent): Promise<ToolPkgTypes.ChatInputHookObjectResult | null> {
  if (event.eventName !== "submit_requested") return null;
  const payload = event.eventPayload;
  // Native plugin sends already carry an explicit turn; this hook never recursively replans those sends.
  if (payload.source === "Sequence") return null;
  const chatId = requireId(payload.chatId, "submitted chatId");
  const extension = await Tools.Chat.readExtension({ kind: "chat", chatId });
  // An unassociated chat is outside this plugin's ownership, not an unsuccessful group lookup.
  if (extension === null) return null;
  const binding = decodeChatMarker(extension);
  if (parseChatSelection(binding.selection).kind !== "group") return null;
  const service = await getService();
  const input = submission(payload, chatId, binding.selection);
  const current = executions.get(chatId);
  if (current !== undefined && (current.status === "running" || (current.outcome !== null && current.outcome.status === "cancelled"))) {
    return { action: "Block", clearInput: false, message: "An existing group submission must settle or resume before another input", metadata: { submissionId: current.input.submissionId } };
  }
  if (controller === null) controller = createGroupExecutionController(service, new NativeGroupTurnTransport());
  const work = controller.submit(input);
  const execution: InputExecution = { input, status: "running", outcome: null, failure: null };
  executions.set(chatId, execution);
  observe(execution, work);
  return { action: "Consume", clearInput: true, metadata: { submissionId: input.submissionId, chatId, selection: input.selection, runtime: input.runtime } };
}

/** Defines exact-submission controls and a separate read-only discovery operation. */
export type GroupExecutionRequest = { chatId: string; action: "current" } | { chatId: string; submissionId: string; action: "status" | "cancel" | "resume" };
/** Exposes only the actual retained submission's progress and failure at the package boundary. */
export interface GroupExecutionStatus {
  submissionId: string; chatId: string; status: "running" | "settled" | "failed";
  outcome: GroupExecutionOutcome | null; error: string | null;
}

/** Decodes the raw package IPC envelope once before entering the typed execution-control chain. */
function decodeGroupExecutionRequest(payload: unknown): GroupExecutionRequest {
  assertObject(payload, "group execution request");
  const chatId = requireId(payload.chatId, "group execution chatId");
  const action = payload.action;
  const keys = Object.keys(payload);
  const expected = action === "current" ? ["chatId", "action"] : ["chatId", "submissionId", "action"];
  if (keys.length !== expected.length || keys.some(key => expected.indexOf(key) < 0)) throw new Error("Group execution request has unexpected or missing fields");
  if (action === "current") return { chatId, action };
  const submissionId = requireId(payload.submissionId, "group execution submissionId");
  if (action !== "status" && action !== "cancel" && action !== "resume") throw new Error("Unknown group execution action");
  return { chatId, submissionId, action };
}

/** Requires the exact typed submission identity instead of acting on the newest execution implicitly. */
function retained(payload: Exclude<GroupExecutionRequest, { action: "current" }>): { execution: InputExecution; action: GroupExecutionRequest["action"]; controller: GroupExecutionController } {
  const execution = executions.get(payload.chatId);
  if (execution === undefined || execution.input.submissionId !== payload.submissionId || controller === null) throw new Error("Group execution has no matching retained submission");
  return { execution, action: payload.action, controller };
}

/** Reports real progress and supports precise cancellation or explicit resumption through package-local IPC. */
async function groupExecutionRequest(payload: GroupExecutionRequest): Promise<ToolPkgTypes.JsonValue> {
  if (payload.action === "current") {
    const execution = executions.get(payload.chatId);
    return execution === undefined ? null : jsonValue(executionStatus(execution));
  }
  const target = retained(payload), execution = target.execution;
  if (target.action === "cancel") {
    if (execution.status !== "running") throw new Error("Only a running group submission can be cancelled");
    await target.controller.cancel(execution.input.chatId, execution.input.submissionId);
  } else if (target.action === "resume") {
    if (execution.status !== "settled" || execution.outcome === null || execution.outcome.status !== "cancelled") throw new Error("Only a settled cancelled group submission can resume");
    const work = target.controller.resume(execution.input.chatId, execution.input.submissionId);
    execution.status = "running"; execution.outcome = null; execution.failure = null;
    observe(execution, work);
  }
  return jsonValue(executionStatus(execution));
}

/** Copies the actual retained state without making failure or pending work look settled. */
function executionStatus(execution: InputExecution): GroupExecutionStatus {
  return { submissionId: execution.input.submissionId, chatId: execution.input.chatId, status: execution.status,
    outcome: execution.outcome, error: execution.status === "failed" ? String(execution.failure) : null };
}

/** Registers the actual exported input handler and package-local control channel without opening business storage. */
export function registerGroupExecutionHooks(): void {
  ToolPkg.registerChatInputHook({ id: "group-input-submit", function: onGroupInputSubmit });
  ToolPkg.ipc.on<unknown, ToolPkgTypes.JsonValue>("character-memory.group-execution",
    /** Decodes only messages entering through the external package IPC channel. */
    payload => groupExecutionRequest(decodeGroupExecutionRequest(payload)),
  );
}

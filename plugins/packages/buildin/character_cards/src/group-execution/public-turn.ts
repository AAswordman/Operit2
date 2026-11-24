import type { ChatGroupTurnRequest, ChatGroupTurnStatus, ChatGroupTurnReply } from "../api";
import { chatSelection } from "../chat-storage";
import { parseChatSelection } from "../chat-bindings";
import { getService } from "../service-runtime";
import { assertObject, requireId } from "../validation";
import type { GroupExecutionOutcome, GroupSubmission } from "./contracts";
import { createGroupExecutionController } from "./domain-adapter";
import { NativeGroupTurnTransport } from "./native-transport";

interface TurnController {
  submit(input: GroupSubmission): Promise<GroupExecutionOutcome>;
  cancel(chatId: string, submissionId: string): Promise<void>;
}
interface TurnDependencies {
  selection(chatId: string): Promise<string>;
  controller(reply: (value: ChatGroupTurnReply) => void): Promise<TurnController>;
}
interface Execution {
  owner: string; chatId: string; requestId: string; progress: ChatGroupTurnStatus;
  cancelled: boolean; controller: TurnController | null; completion: Promise<void>;
}

/** Retains exact caller-owned submissions and shares the existing group execution engine. */
export function createGroupTurnApi(dependencies: TurnDependencies) {
  const executions = new Map<string, Execution>();
  const chats = new Map<string, Execution>();
  function snapshot(execution: Execution): ChatGroupTurnStatus {
    return { ...execution.progress, replies: execution.progress.replies.map(reply => ({ ...reply })) };
  }
  async function execute(execution: Execution, input: Extract<ChatGroupTurnRequest, { action: "start" }>): Promise<void> {
    try {
      const selection = await dependencies.selection(input.chatId);
      if (execution.cancelled) { execution.progress.status = "cancelled"; return; }
      if (parseChatSelection(selection).kind !== "group") { execution.progress.status = "not_group"; return; }
      const controller = await dependencies.controller(reply => { execution.progress.replies.push(reply); });
      execution.controller = controller;
      if (execution.cancelled) { execution.progress.status = "cancelled"; return; }
      const work = controller.submit({ submissionId: input.requestId, chatId: input.chatId, selection,
        runtime: "floating", notifyReply: false, text: input.text, attachments: input.attachments,
        replyToMessageTimestamp: null });
      const outcome = await work;
      if (outcome.status === "completed" || outcome.status === "cancelled") execution.progress.status = outcome.status;
      else throw new Error(outcome.disposition?.type === "blocked" ? outcome.disposition.message || "Group turn blocked" : "Group turn did not complete: " + outcome.status);
    } catch (failure) {
      execution.progress.status = "failed";
      execution.progress.error = String(failure);
    } finally {
      if (chats.get(execution.chatId) === execution) chats.delete(execution.chatId);
    }
  }
  return async (event: ToolPkg.PublicApiEvent<ChatGroupTurnRequest>): Promise<ChatGroupTurnStatus> => {
    const input = event.payload;
    assertObject(input, "group turn");
    requireId(input.chatId, "group turn chatId"); requireId(input.requestId, "group turn requestId");
    const owner = requireId(event.callerPackage, "group turn caller");
    const key = JSON.stringify([owner, input.chatId, input.requestId]);
    if (input.action === "start") {
      if (typeof input.text !== "string" || !Array.isArray(input.attachments)) throw new Error("Group turn requires text and attachments");
      if (executions.has(key)) throw new Error("Group turn request already exists");
      const previous = chats.get(input.chatId);
      if (previous && !previous.cancelled) throw new Error("A group turn is already running in this chat");
      // Keep one settled turn per owner and chat; active work always retains its cancellation target.
      for (const [id, previous] of executions) {
        if (previous.owner === owner && previous.chatId === input.chatId && previous.progress.status !== "running") executions.delete(id);
      }
      const execution: Execution = { owner, chatId: input.chatId, requestId: input.requestId,
        progress: { status: "running", replies: [], error: null }, cancelled: false, controller: null, completion: Promise.resolve() };
      executions.set(key, execution); chats.set(input.chatId, execution);
      // A new utterance can be recorded immediately while the cancelled native turn releases its lease.
      execution.completion = (previous?.completion ?? Promise.resolve()).then(() => execute(execution, input));
      return snapshot(execution);
    }
    if (input.action !== "status" && input.action !== "cancel") throw new Error("Unknown group turn action");
    const execution = executions.get(key);
    if (!execution) throw new Error("No matching group turn for this caller");
    if (input.action === "cancel" && execution.progress.status === "running") {
      execution.cancelled = true;
      if (execution.controller) {
        try { await execution.controller.cancel(input.chatId, input.requestId); }
        catch (failure) {
          // Native finalization can finish between the status read and the cancellation request.
          await execution.completion;
          if (snapshot(execution).status === "failed") throw failure;
        }
      }
    }
    return snapshot(execution);
  };
}

/** Publishes only validated committed replies, using each saved participant's speech configuration. */
export const groupTurnApi = createGroupTurnApi({
  selection: chatSelection,
  async controller(reply) {
    const service = await getService();
    const transport = new NativeGroupTurnTransport(async (turn, text) => {
      const card = await service.getCharacter(turn.turn.participantId);
      reply({ participantId: card.id, name: card.name, avatarUri: card.avatarUri, ttsConfigId: card.ttsConfigId, text });
    });
    return createGroupExecutionController(service, transport);
  },
});

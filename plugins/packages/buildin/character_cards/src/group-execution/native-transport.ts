import type { MessageSendResultData } from "../../../../../types/results";
import { decodeMessageMarker } from "../chat-markers";
import { assertBoolean, assertInteger } from "../validation";
import type { GroupContinuationTurn, GroupInitialTurn, GroupMessageLocator, GroupSelectedTurn, GroupSendDecision, GroupSequenceCompletion, GroupTurnTransport } from "./contracts";
import { decodeNativeGroupSendResult } from "./contracts";

/** Derives the sole public send request from the real SDK contract. */
type SendRequest = Parameters<typeof Tools.Chat.sendMessage>[0];
/** Correlates a local controller attempt without granting native execution authority. */
interface Attempt { chatId: string; runtime: "main" | "floating"; active: boolean }
/** Tracks plugin planning receipts only; the host owns and finalizes every actual native send. */
interface Sequence {
  chatId: string; runtime: "main" | "floating"; notifyReply: boolean;
  last: GroupSendDecision | null; userTimestamp: number | null;
  assistants: GroupMessageLocator[]; finished: boolean;
}

/** Builds an unambiguous local sequence key without sending it as host authority. */
function sequenceKey(chatId: string, submissionId: string): string { return JSON.stringify([chatId, submissionId]); }

/** Requires the real persisted assistant snapshot to identify the participant selected by this exact turn. */
async function verifySnapshot(chatId: string, selected: GroupSelectedTurn, assistant: GroupMessageLocator): Promise<void> {
  const extension = await Tools.Chat.readExtension({ kind: "message", chatId, messageTimestamp: assistant.messageTimestamp, variantIndex: assistant.variantIndex });
  if (extension === null) throw new Error("Committed group assistant has no participant message extension");
  const marker = decodeMessageMarker(extension);
  if (marker.selection !== selected.selection || marker.profile.id !== selected.participantId || marker.promptFunctionType !== "CHAT") {
    throw new Error("Committed group assistant snapshot identifies a different participant or selection");
  }
}

/** Uses only sendMessage and cancel; no native execution handle or separate finalization API reaches this plugin. */
export class NativeGroupTurnTransport implements GroupTurnTransport {
  private readonly sequences = new Map<string, Sequence>();
  private readonly attempts = new Map<string, Attempt>();

  /** Sends the complete original input once, or explicitly records it without requesting an arbitrary participant. */
  async submit(input: GroupInitialTurn): Promise<MessageSendResultData> {
    const submitted = input.submission, key = sequenceKey(submitted.chatId, submitted.submissionId), prior = this.sequences.get(key);
    if (prior !== undefined && (prior.last === null || prior.last.type !== "committed" || prior.last.receipt.status !== "cancelled" || prior.userTimestamp !== null)) {
      throw new Error("A second initial submit requires explicitly resumed cancellation before user-message persistence");
    }
    const sequence: Sequence = { chatId: submitted.chatId, runtime: submitted.runtime, notifyReply: submitted.notifyReply,
      last: null, userTimestamp: null, assistants: [], finished: false };
    this.sequences.set(key, sequence);
    const request: SendRequest = { kind: "submit", chatId: submitted.chatId, runtime: submitted.runtime,
      notifyReply: input.kind === "record_only" ? false : input.turn.notifyReply,
      input: { text: submitted.text, attachments: submitted.attachments, replyToMessageTimestamp: submitted.replyToMessageTimestamp },
      turn: input.kind === "record_only" ? { kind: "record_only" } : { kind: "execute", participantId: input.turn.participantId } };
    return this.run(sequence, input, request);
  }

  /** Refers to the actual committed user record and never resubmits its text, attachments or reply target. */
  async continue(input: GroupContinuationTurn): Promise<MessageSendResultData> {
    const sequence = this.requireSequence(input.chatId, input.submissionId, input.runtime);
    if (sequence.finished || sequence.userTimestamp !== input.userMessageTimestamp) throw new Error("Group continuation requires its acknowledged user timestamp");
    return this.run(sequence, input, { kind: "continue", chatId: input.chatId, runtime: input.runtime,
      userMessageTimestamp: input.userMessageTimestamp, participantId: input.turn.participantId, notifyReply: input.turn.notifyReply });
  }

  /** Cancels only a currently correlated local attempt through the host's authenticated owner/chat capture. */
  async cancel(input: { chatId: string; requestKey: string; runtime: "main" | "floating" }): Promise<void> {
    const attempt = this.attempts.get(input.requestKey);
    if (attempt === undefined || attempt.chatId !== input.chatId || attempt.runtime !== input.runtime || !attempt.active) throw new Error("Cancellation has no matching active send attempt");
    const result = await Tools.Chat.cancel(input.chatId);
    if (result.chatId !== input.chatId) throw new Error("Native cancellation identifies a different conversation");
    assertBoolean(result.cancelRequested, "native cancelRequested");
  }

  /** Checks plugin sequence completion against real per-send receipts after host finalization has already finished. */
  async finish(input: GroupSequenceCompletion): Promise<void> {
    const sequence = this.requireSequence(input.chatId, input.submissionId, input.runtime);
    if (sequence.finished || sequence.userTimestamp !== input.userMessageTimestamp || sequence.last === null || sequence.last.type !== "committed" || sequence.last.receipt.status !== "completed") {
      throw new Error("Group completion does not match its actual completed sends");
    }
    const assistants = input.receipts.flatMap(
      /** Counts only actual assistant locators acknowledged by the controller. */
      receipt => receipt.assistant === null ? [] : [receipt.assistant],
    );
    if (assistants.length !== sequence.assistants.length) throw new Error("Group completion changed its acknowledged assistant count");
    for (let index = 0; index < assistants.length; index++) {
      const actual = assistants[index], expected = sequence.assistants[index];
      if (actual.messageTimestamp !== expected.messageTimestamp || actual.variantIndex !== expected.variantIndex) throw new Error("Group completion changed assistant order or revision identity");
    }
    sequence.finished = true;
  }

  /** Ends plugin-local planning state; accepted native sends clean up even when plugin code stops observing. */
  async abandon(input: { submissionId: string; chatId: string; runtime: "main" | "floating" }): Promise<void> {
    this.requireSequence(input.chatId, input.submissionId, input.runtime).finished = true;
  }

  /** Requires the exact retained local submission rather than selecting another conversation's state. */
  private requireSequence(chatId: string, submissionId: string, runtime: "main" | "floating"): Sequence {
    const sequence = this.sequences.get(sequenceKey(chatId, submissionId));
    if (sequence === undefined || sequence.chatId !== chatId || sequence.runtime !== runtime) throw new Error("Group submission has no matching send sequence");
    return sequence;
  }

  /** Awaits one real finalized send, validates its originating receipt and checks the native-authored snapshot. */
  private async run(sequence: Sequence, input: GroupInitialTurn | GroupContinuationTurn, request: SendRequest): Promise<MessageSendResultData> {
    const requestKey = "kind" in input && input.kind === "record_only" ? input.requestKey : input.turn.requestKey;
    if (this.attempts.has(requestKey)) throw new Error("Group request correlation key was already used");
    const attempt: Attempt = { chatId: sequence.chatId, runtime: sequence.runtime, active: true };
    this.attempts.set(requestKey, attempt);
    try {
      const actual = await Tools.Chat.sendMessage(request);
      const decision = decodeNativeGroupSendResult(actual, input, "kind" in input ? null : input.userMessageTimestamp);
      sequence.last = decision;
      if (decision.type === "committed") {
        if (decision.receipt.userMessageTimestamp !== null) {
          assertInteger(decision.receipt.userMessageTimestamp, "native user timestamp", 1);
          sequence.userTimestamp = decision.receipt.userMessageTimestamp;
        }
        if (decision.receipt.assistant !== null) {
          sequence.assistants.push({ ...decision.receipt.assistant });
          if ("kind" in input && input.kind === "record_only") throw new Error("Record-only native send committed an assistant");
          await verifySnapshot(sequence.chatId, input.turn, decision.receipt.assistant);
        }
      }
      return actual;
    } finally { attempt.active = false; }
  }
}

import type { MessageSendResultData } from "../../../../../types/results";
import type { ToolPkg as ToolPkgTypes } from "../../../../../types/index";
import { parseChatSelection } from "../chat-bindings";
import { jsonValue } from "../domain";
import { requireId } from "../validation";
import type { GroupContinuationTurn, GroupExecutionDependencies, GroupExecutionOutcome, GroupInitialTurn, GroupSubmissionAttachment, GroupSendDisposition, GroupSubmission, GroupSelectedTurn, GroupTurnReceipt } from "./contracts";
import { parseGroupResponsePlan } from "./planner";
import { decodeNativeGroupSendResult } from "./contracts";

/** Owns one runtime-local sequence, including its retained failure and cancellation cursor. */
interface Session {
  input: GroupSubmission;
  inputKey: string;
  selection: string | null;
  speakers: string[];
  planned: boolean;
  transportStarted: boolean;
  finalizationAttempted: boolean;
  cursor: number;
  attempt: number;
  userMessageTimestamp: number | null;
  receipts: GroupTurnReceipt[];
  disposition: GroupSendDisposition | null;
  phase: "planning" | "running" | "cancelled" | "finishing" | "completed" | "failed" | "stopped";
  cancelled: boolean;
  activeRequestKey: string | null;
  cancellation: Promise<void> | null;
  retained: Promise<GroupExecutionOutcome> | null;
}

/** Compares JSON structurally so source object property order cannot change duplicate-submit or snapshot validation. */
function canonical(value: ToolPkgTypes.JsonValue): string {
  if (value === null || typeof value !== "object") return JSON.stringify(value);
  const parts: string[] = [];
  if (Array.isArray(value)) {
    for (const item of value) parts.push(canonical(item));
    return "[" + parts.join(",") + "]";
  }
  for (const key of Object.keys(value).sort()) parts.push(JSON.stringify(key) + ":" + canonical(value[key]));
  return "{" + parts.join(",") + "}";
}

/** Copies the SDK-typed attachment and enforces its byte-size range without reinterpreting its structure. */
function attachment(value: GroupSubmissionAttachment): GroupSubmissionAttachment {
  if (!Number.isSafeInteger(value.fileSize) || value.fileSize < 0) throw new Error("Group attachment size must be a nonnegative safe integer");
  return { ...value };
}

/** Validates and owns the exact submitted input before any planner, model or persistence call begins. */
export function validateGroupSubmission(value: GroupSubmission): GroupSubmission {
  requireId(value.selection, "group selection");
  parseChatSelection(value.selection);
  requireId(value.submissionId, "group submission id"); requireId(value.chatId, "group chat id");
  const attachments: GroupSubmissionAttachment[] = [];
  for (const item of value.attachments) attachments.push(attachment(item));
  if (value.text.trim().length === 0 && attachments.length === 0) throw new Error("Group submission requires actual text or attachments");
  if (value.replyToMessageTimestamp !== null && (!Number.isSafeInteger(value.replyToMessageTimestamp) || value.replyToMessageTimestamp < 1)) {
    throw new Error("Group reply target must be a positive safe integer timestamp");
  }
  return { submissionId: value.submissionId, chatId: value.chatId, selection: value.selection, runtime: value.runtime, notifyReply: value.notifyReply, text: value.text, attachments, replyToMessageTimestamp: value.replyToMessageTimestamp };
}

/** Retains both original failures when execution and its separately requested cancellation fail. */
export class GroupExecutionFailures extends Error {
  /** Exposes both original error objects rather than rewriting or discarding either failure. */
  constructor(readonly executionFailure: unknown, readonly cancellationFailure: unknown) {
    super("Group execution and cancellation both failed"); this.name = "GroupExecutionFailures";
  }
}

/** Retains the original execution failure and the independent native sequence release failure. */
export class GroupExecutionReleaseFailures extends Error {
  /** Preserves both exact failure objects without replacing either with a manufactured completion result. */
  constructor(readonly executionFailure: unknown, readonly releaseFailure: unknown) {
    super("Group execution and native sequence release both failed"); this.name = "GroupExecutionReleaseFailures";
  }
}

/** Runs ordered plugin-owned sequences using mandatory real transports, with no SDK method invented by this controller. */
export class GroupExecutionController {
  private readonly sessions = new Map<string, Session>();
  private readonly chats = new Map<string, string>();

  /** Accepts only explicit domain and transport dependencies; it never creates a default or partially working backend. */
  constructor(private readonly dependencies: GroupExecutionDependencies) {}

  /** Starts one retained submission and rejects a changed payload or competing sequence before doing any work. */
  submit(input: GroupSubmission): Promise<GroupExecutionOutcome> {
    const saved = validateGroupSubmission(input), key = JSON.stringify([saved.chatId, saved.submissionId]), inputKey = canonical(jsonValue(saved));
    const existing = this.sessions.get(key);
    if (existing !== undefined) {
      if (existing.inputKey !== inputKey) throw new Error("Duplicate group submission id has a different input");
      if (existing.retained === null) throw new Error("Group submission retained result was not initialized");
      return existing.retained;
    }
    if (this.chats.has(saved.chatId)) throw new Error("Another group sequence already owns this chat");
    const session: Session = { input: saved, inputKey, selection: null, speakers: [], planned: false, transportStarted: false, finalizationAttempted: false, cursor: 0, attempt: 0, userMessageTimestamp: null,
      receipts: [], disposition: null, phase: "planning", cancelled: false, activeRequestKey: null, cancellation: null, retained: null };
    this.sessions.set(key, session); this.chats.set(saved.chatId, key);
    session.retained = Promise.resolve().then(
      /** Starts after retaining identity so two synchronous submits cannot both begin planning. */
      () => this.run(session, true),
    );
    return session.retained;
  }

  /** Records cancellation and invokes only the active correlated turn's real cancellation capability. */
  cancel(chatId: string, submissionId: string): Promise<void> {
    const session = this.session(chatId, submissionId);
    if (session.phase === "completed" || session.phase === "failed" || session.phase === "finishing" || session.phase === "stopped") throw new Error("Group sequence is no longer cancellable");
    session.cancelled = true;
    if (session.cancellation !== null) return session.cancellation;
    if (session.activeRequestKey === null) return Promise.resolve();
    const requestKey = session.activeRequestKey;
    session.cancellation = Promise.resolve().then(
      /** Retains synchronous and asynchronous host cancellation failures identically without retrying the request. */
      () => this.dependencies.transport.cancel({ chatId, requestKey, runtime: session.input.runtime }),
    );
    return session.cancellation;
  }

  /** Resumes only a settled cancelled cursor without rerunning the planner or submitting the original user input twice. */
  resume(chatId: string, submissionId: string): Promise<GroupExecutionOutcome> {
    const session = this.session(chatId, submissionId);
    if (session.phase !== "cancelled" || session.activeRequestKey !== null) throw new Error("Only a settled cancelled group sequence can resume");
    session.cancelled = false; session.cancellation = null; session.phase = session.planned ? "running" : "planning";
    session.retained = Promise.resolve().then(
      /** Reuses the acknowledged cursor and calls the planner only when cancellation prevented its first invocation. */
      () => this.run(session, !session.planned),
    );
    return session.retained;
  }

  /** Requires an exact retained session and never reconstructs one from current selection or history guesses. */
  private session(chatId: string, submissionId: string): Session {
    requireId(chatId, "group chat id"); requireId(submissionId, "group submission id");
    const session = this.sessions.get(JSON.stringify([chatId, submissionId]));
    if (session === undefined) throw new Error("Group submission does not exist in this runtime");
    return session;
  }

  /** Rechecks actual binding and member references before every turn, including a cancelled turn's resumed execution. */
  private async current(session: Session, participantId: string | null): Promise<void> {
    const selection = await this.dependencies.readSelection(session.input.chatId), parsed = parseChatSelection(selection);
    if (parsed.kind !== "group") throw new Error("Group execution requires a real group selection");
    if (session.input.selection !== selection) throw new Error("Submitted group selection differs from the actual conversation binding");
    if (session.selection === null) session.selection = selection;
    else if (session.selection !== selection) throw new Error("Chat selection changed during group execution");
    const participants = await this.dependencies.readParticipants(selection);
    if (participantId !== null) {
      let matches = 0;
      for (const participant of participants) if (participant.id === participantId) matches++;
      if (matches !== 1) throw new Error("Planned group participant was deleted or removed: " + participantId);
    }
  }

  /** Produces a progress copy from real receipts and the retained cursor, including explicit cancellation before persistence. */
  private outcome(session: Session, status: GroupExecutionOutcome["status"]): GroupExecutionOutcome {
    if (session.selection === null) throw new Error("Group outcome has no authenticated selection");
    const receipts: GroupTurnReceipt[] = [];
    for (const item of session.receipts) receipts.push({ ...item, assistant: item.assistant === null ? null : { ...item.assistant } });
    return { status, disposition: session.disposition, submissionId: session.input.submissionId, chatId: session.input.chatId, selection: session.selection,
      cursor: session.cursor, plannedTurns: session.speakers.length, userMessageTimestamp: session.userMessageTimestamp, receipts };
  }

  /** Plans once, executes serially, preserves genuine cancellation receipts and propagates every original failure without retry. */
  private async run(session: Session, plan: boolean): Promise<GroupExecutionOutcome> {
    try {
      await this.current(session, null);
      if (session.selection === null) throw new Error("Group execution selection is not initialized");
      if (session.cancelled) { session.phase = "cancelled"; return this.outcome(session, "cancelled"); }
      if (plan) {
        const participants = await this.dependencies.readParticipants(session.selection);
        if (session.cancelled) { session.phase = "cancelled"; return this.outcome(session, "cancelled"); }
        const planned = await this.dependencies.plan(participants, session.input.text);
        const checked = parseGroupResponsePlan(JSON.stringify(planned), participants);
        for (const round of checked.rounds) for (const entry of round) if (entry.speak) session.speakers.push(entry.id);
        session.planned = true;
      }
      session.phase = "running";
      while (true) {
        if (session.cancelled) { session.phase = "cancelled"; return this.outcome(session, "cancelled"); }
        if (session.cursor >= session.speakers.length && session.userMessageTimestamp !== null) break;
        const participantId = session.cursor < session.speakers.length ? session.speakers[session.cursor] : null;
        await this.current(session, participantId);
        const requestKey = JSON.stringify([session.input.chatId, session.input.submissionId, session.cursor, session.attempt++]);
        const selected: GroupSelectedTurn | null = participantId === null ? null : { requestKey, selection: session.selection, participantId, notifyReply: session.input.notifyReply && session.cursor === session.speakers.length - 1 };
        if (session.cancelled) { session.phase = "cancelled"; return this.outcome(session, "cancelled"); }
        session.activeRequestKey = requestKey;
        let requested: GroupInitialTurn | GroupContinuationTurn;
        let value: MessageSendResultData;
        session.transportStarted = true;
        if (session.userMessageTimestamp === null) {
          requested = selected === null ? { kind: "record_only", submission: session.input, requestKey } : { kind: "execute", submission: session.input, turn: selected };
          value = await this.dependencies.transport.submit(requested);
        } else {
          if (selected === null) throw new Error("A group continuation must identify an actual planned speaker");
          requested = { submissionId: session.input.submissionId, chatId: session.input.chatId, runtime: session.input.runtime,
            userMessageTimestamp: session.userMessageTimestamp, turn: selected };
          value = await this.dependencies.transport.continue(requested);
        }
        const decision = decodeNativeGroupSendResult(value, requested, session.userMessageTimestamp);
        session.activeRequestKey = null;
        if (session.cancellation !== null) await session.cancellation;
        if (decision.type !== "committed") {
          session.disposition = decision;
          session.finalizationAttempted = true;
          await this.dependencies.transport.abandon({ submissionId: session.input.submissionId, chatId: session.input.chatId, runtime: session.input.runtime });
          session.phase = "stopped"; this.chats.delete(session.input.chatId);
          return this.outcome(session, decision.type);
        }
        const accepted = decision.receipt;
        if (accepted.userMessageTimestamp !== null) session.userMessageTimestamp = accepted.userMessageTimestamp;
        session.receipts.push(accepted);
        if (accepted.status === "completed" && participantId !== null) session.cursor++;
        if (accepted.status === "cancelled" || session.cancelled) { session.phase = "cancelled"; return this.outcome(session, "cancelled"); }
      }
      if (session.userMessageTimestamp === null) throw new Error("Completed group sequence has no actual persisted user message");
      session.phase = "finishing"; session.finalizationAttempted = true;
      await this.dependencies.transport.finish({ submissionId: session.input.submissionId, chatId: session.input.chatId, runtime: session.input.runtime,
        userMessageTimestamp: session.userMessageTimestamp, receipts: this.outcome(session, "completed").receipts });
      session.phase = "completed"; this.chats.delete(session.input.chatId);
      return this.outcome(session, "completed");
    } catch (error) {
      session.phase = "failed"; session.activeRequestKey = null; this.chats.delete(session.input.chatId);
      let failure = error;
      if (session.cancellation !== null) {
        try { await session.cancellation; }
        catch (cancellationError) {
          if (cancellationError !== error) failure = new GroupExecutionFailures(error, cancellationError);
        }
      }
      if (session.transportStarted && !session.finalizationAttempted) {
        session.finalizationAttempted = true;
        try { await this.dependencies.transport.abandon({ submissionId: session.input.submissionId, chatId: session.input.chatId, runtime: session.input.runtime }); }
        catch (releaseError) { throw new GroupExecutionReleaseFailures(failure, releaseError); }
      }
      throw failure;
    }
  }
}

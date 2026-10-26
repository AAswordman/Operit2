import type { CharacterRepository, CharacterRepositoryOwner } from "../canonical";
import { memoryJobProducer } from "./producer";
import type { MemoryCandidate, MemoryChatMessage, MemoryRebuild, MemoryRebuildProgress, MemoryRebuildTask } from "../model";
import { assertInteger, assertObject, assertStrings, requireId } from "../validation";
import { extractMessages } from "./analysis";
import { chatParticipants, planWindows, primaryOwner, readMessages, requireChat, requireChatOwner, requireMessageOwner, requireMessagesOwner, currentMessageMemoryAccess } from "./chat";

export type CandidateSource = "reply_finalized_auto" | "selected_user_message";
export interface CandidateRequest { chatId: string; timestamp: number; variantIndex: number; sourceType: CandidateSource; ownerKey: string | null }
export interface CandidateResult { owners: string[]; candidateIds: string[] }

/** Preserves the actual failure text for durable job status without substituting a success. */
function errorText(error: unknown): string {
  if (error !== null && typeof error === "object" && "message" in error && typeof error.message === "string") return error.message;
  return String(error);
}

/** Retains both execution and failed status-publication errors instead of hiding either failure. */
export class MemoryJobStatusError extends Error {
  /** Exposes the original execution error together with the status write that also failed. */
  constructor(readonly executionError: unknown, readonly statusPublicationError: unknown) {
    super("Memory job failed: " + errorText(executionError) + "; status publication also failed: " + errorText(statusPublicationError));
    this.name = "MemoryJobStatusError";
  }
}

/** Retains every failed owner after independent owners finish their own work. */
export class MemoryJobPassError extends Error {
  /** Keeps original failures and completed work available to diagnostics without reporting false success. */
  constructor(readonly failures: Array<{ ownerKey: string; error: unknown }>, readonly report: MemoryJobReport) {
    super("Memory jobs failed for " + failures.length + " owner(s): " + failures.map(
      /** Identifies each failed owner while retaining its original failure text. */
      failure => failure.ownerKey + ": " + errorText(failure.error),
    ).join("; "));
    this.name = "MemoryJobPassError";
  }
}

/** Records a failed work unit so independent jobs can continue after its status is committed. */
async function publishJobFailure(error: unknown, publishStatus: () => Promise<unknown>): Promise<void> {
  try { await publishStatus(); }
  catch (statusError) { throw new MemoryJobStatusError(error, statusError); }
}

/** Compares every stored revision field structurally so a host codec reordering persisted keys cannot fail an unchanged plan. */
function sameMessageRevision(current: MemoryChatMessage, planned: MemoryChatMessage): boolean {
  return current.sender === planned.sender && current.content === planned.content && current.timestamp === planned.timestamp
    && current.variantIndex === planned.variantIndex && current.variantCount === planned.variantCount
    && current.provider === planned.provider && current.modelName === planned.modelName;
}

/** Enqueues a real selected message or finalized reply for exact bound memory owners. */
export async function enqueueCandidate(input: CandidateRequest, repository: CharacterRepository): Promise<CandidateResult> {
  if (input.sourceType === "reply_finalized_auto" && !(await repository.readGlobalMemoryPreferences()).autoUpdateEnabled) return { owners: [], candidateIds: [] };
  assertInteger(input.timestamp, "candidate message timestamp", 0); assertInteger(input.variantIndex, "candidate variant index", 0);
  if (input.variantIndex > 2147483647) throw new Error("Candidate variant exceeds the host range");
  const messages = await readMessages(input.chatId), matches = messages.filter(
    /** Resolves the message by its genuine persisted timestamp. */
    message => message.timestamp === input.timestamp && message.variantIndex === input.variantIndex,
  );
  if (matches.length !== 1) throw new Error("Candidate does not identify exactly one source message");
  const message = matches[0];
  switch (input.sourceType) {
    case "reply_finalized_auto": if (message.sender !== "ai" && message.sender !== "assistant") throw new Error("Automatic candidate requires an assistant reply"); break;
    case "selected_user_message": if (message.sender !== "user") throw new Error("Selected candidate requires a user message"); break;
    default: throw new Error("Unknown candidate source");
  }
  if (message.content.trim() === "") throw new Error("Memory candidate source is blank");
  const owners = new Set<string>();
  if (input.sourceType === "reply_finalized_auto") {
    const access = await currentMessageMemoryAccess(input.chatId, message, repository);
    const ownerKey = input.ownerKey === null ? access.primaryOwnerKey : input.ownerKey;
    if (ownerKey === "") return { owners: [], candidateIds: [] };
    if (!access.resources.some(
      /** Authorizes only the exact submitted resource through the already-read current author permissions. */
      resource => resource.key === ownerKey && resource.writable,
    )) throw new Error("Current message author cannot write this memory owner: " + ownerKey);
    await repository.readMemoryJobState(ownerKey); owners.add(ownerKey);
  } else if (input.ownerKey !== null) {
    await requireChatOwner(input.chatId, input.ownerKey, repository); owners.add(input.ownerKey);
  } else {
    const participants = await chatParticipants(input.chatId, repository);
    if (participants.length !== 1) throw new Error("A group selected-user candidate requires an explicit owner");
    owners.add(await primaryOwner(participants[0], repository));
  }
  const result: CandidateResult = { owners: [...owners], candidateIds: [] }, now = Date.now();
  const producerNodeId = await memoryJobProducer();
  for (const ownerKey of owners) {
    const space = await repository.readMemoryJobState(ownerKey), previous = space.candidates.filter(
      /** Uses the full owner-scoped source identity for genuine hook idempotency. */
      candidate => candidate.chatId === input.chatId && candidate.triggerMessageTimestamp === input.timestamp && candidate.triggerVariantIndex === input.variantIndex && candidate.sourceType === input.sourceType,
    );
    if (previous.length !== 0) {
      // Keep every historical task's status and expose its identity without enqueueing another request.
      for (const candidate of previous) result.candidateIds.push(candidate.id);
      continue;
    }
    const id = await repository.allocateRecordId();
    space.candidates.push({ id, producerNodeId, chatId: input.chatId, triggerMessageTimestamp: input.timestamp, triggerVariantIndex: input.variantIndex, createdAt: now, updatedAt: now, status: "pending", attemptCount: 0, lastError: "", sourceType: input.sourceType });
    if (space.settings.autoSaveIntervalMinutes > 0 && space.settings.nextAutoSaveRunAtMs === 0) space.settings.nextAutoSaveRunAtMs = now + space.settings.autoSaveIntervalMinutes * 60000;
    await repository.writeMemoryJobState(ownerKey, space); result.candidateIds.push(id);
  }
  return result;
}

/** Returns genuine terminal failures to the pending queue so a later interval can retry them. */
export async function retryFailedCandidates(ownerKey: string, repository: CharacterRepository): Promise<{ retried: number }> {
  const space = await repository.readMemoryJobState(ownerKey), now = Date.now(), producerNodeId = await memoryJobProducer();
  let retried = 0;
  for (const candidate of space.candidates) if (candidate.status === "failed") {
    candidate.producerNodeId = producerNodeId; candidate.status = "pending"; candidate.lastError = ""; candidate.updatedAt = now; retried += 1;
  }
  if (retried !== 0) await repository.writeMemoryJobState(ownerKey, space);
  return { retried };
}

/** Removes only terminal failed records so the queue stops reporting permanent failures. */
export async function clearFailedCandidates(ownerKey: string, repository: CharacterRepository): Promise<{ cleared: number }> {
  const space = await repository.readMemoryJobState(ownerKey);
  const remaining = space.candidates.filter(
    /** Retains pending and processing records for their real owners. */
    candidate => candidate.status !== "failed",
  ), cleared = space.candidates.length - remaining.length;
  if (cleared !== 0) { space.candidates = remaining; await repository.writeMemoryJobState(ownerKey, space); }
  return { cleared };
}

/** Plans and persists a genuine requested rebuild for later existing host interval execution. */
export async function startMemoryRebuild(ownerKey: string, rebuild: MemoryRebuild, repository: CharacterRepository): Promise<MemoryRebuildProgress> {
  assertObject(rebuild, "memory rebuild"); assertStrings(rebuild.chatIds, "rebuild chat ids"); assertInteger(rebuild.windowMessageCount, "rebuild window size", 1);
  for (const key of ["fromInclusive", "toInclusive"] as const) if (rebuild[key] !== null) assertInteger(rebuild[key], key, 0);
  if (rebuild.fromInclusive !== null && rebuild.toInclusive !== null && rebuild.fromInclusive > rebuild.toInclusive) throw new Error("Rebuild start must not follow its end");
  if (rebuild.chatIds.length === 0) throw new Error("Rebuild requires selected chats");
  const space = await repository.readMemoryJobState(ownerKey);
  if (space.rebuildProgress.status === "preparing" || space.rebuildProgress.status === "running") throw new Error("Memory rebuild is already active");
  const task: MemoryRebuildTask = { id: await repository.allocateRecordId(), producerNodeId: await memoryJobProducer(), windows: [], nextWindow: 0 }, ids = new Set<string>();
  for (const id of rebuild.chatIds) {
    requireId(id, "rebuild chat id"); if (ids.has(id)) throw new Error("Duplicate selected rebuild chat: " + id); ids.add(id);
    const chat = await requireChat(id), messages = await readMessages(id); await requireMessagesOwner(id, messages, ownerKey, repository);
    const windows = planWindows(chat, messages, rebuild);
    for (const window of windows) await requireMessagesOwner(id, window.messages, ownerKey, repository);
    task.windows.push(...windows);
  }
  if (task.windows.length === 0) throw new Error("Selected histories have no user context in the specified range");
  let totalSourceMessages = 0; const plannedChats = new Set<string>();
  for (const window of task.windows) { totalSourceMessages += window.sourceMessageCount; plannedChats.add(window.chatId); }
  space.rebuildTask = task; space.rebuildProgress = { status: "preparing", totalChats: plannedChats.size, completedChats: 0,
    totalWindows: task.windows.length, completedWindows: 0, totalSourceMessages, processedSourceMessages: 0, failedWindows: 0, currentChatTitle: "", lastError: "" };
  await repository.writeMemoryJobState(ownerKey, space); return space.rebuildProgress;
}

/** Advances a finished window and counts visited sources independently of the failure counter. */
function advanceRebuildWindow(task: MemoryRebuildTask, progress: MemoryRebuildProgress): void {
  const window = task.windows[task.nextWindow];
  if (window === undefined) throw new Error("Rebuild cursor has no source window");
  task.nextWindow += 1; progress.completedWindows += 1; progress.processedSourceMessages += window.sourceMessageCount;
  progress.currentChatTitle = window.chatTitle;
  const next = task.windows[task.nextWindow];
  if (next === undefined || next.chatId !== window.chatId) progress.completedChats += 1;
  progress.status = next === undefined ? "completed" : "running";
}

/** Runs one planned window and records its outcome without blocking later windows. */
async function processRebuildWindow(ownerKey: string, taskId: string, owner: CharacterRepositoryOwner): Promise<void> {
  try {
    await owner.run(
      /** Keeps one model result, graph mutation and progress checkpoint in the same publication. */
      async repository => {
        const space = await repository.readMemoryJobState(ownerKey), task = space.rebuildTask;
        if (task === null || task.id !== taskId) throw new Error("Rebuild task identity changed");
        if (space.rebuildProgress.status === "cancelled") return;
        if (space.rebuildProgress.status !== "preparing" && space.rebuildProgress.status !== "running") throw new Error("Rebuild task is not runnable");
        const window = task.windows[task.nextWindow]; if (window === undefined) throw new Error("Rebuild cursor has no source window");
        const currentMessages = await readMessages(window.chatId);
        for (const source of window.messages) {
          const present = currentMessages.filter(
            /** Matches the exact planned revision before trusting its original extraction content. */
            message => message.timestamp === source.timestamp && message.variantIndex === source.variantIndex,
          );
          if (present.length !== 1 || !sameMessageRevision(present[0], source)) throw new Error("A planned rebuild source message revision changed or was deleted: " + window.chatId + " at " + source.timestamp + "/" + source.variantIndex);
        }
        await requireMessagesOwner(window.chatId, window.messages, ownerKey, repository);
        await extractMessages(ownerKey, window.messages, repository);
        const updated = await repository.readMemoryJobState(ownerKey), savedTask = updated.rebuildTask;
        if (savedTask === null || savedTask.id !== taskId || savedTask.nextWindow !== task.nextWindow) throw new Error("Rebuild cursor changed within execution");
        advanceRebuildWindow(savedTask, updated.rebuildProgress);
        await repository.writeMemoryJobState(ownerKey, updated);
      },
    );
  } catch (error) {
    await publishJobFailure(error,
      /** Commits the failed window and its cursor before later work is allowed to run. */
      () => owner.run(
        /** Retains the error and skips the failed window without publishing partial memory edits. */
        async repository => {
          const space = await repository.readMemoryJobState(ownerKey);
          if (space.rebuildTask === null || space.rebuildTask.id !== taskId) throw new Error("Failed rebuild task identity changed");
          if (space.rebuildProgress.status === "cancelled") return;
          space.rebuildProgress.failedWindows += 1; space.rebuildProgress.lastError = errorText(error);
          advanceRebuildWindow(space.rebuildTask, space.rebuildProgress);
          await repository.writeMemoryJobState(ownerKey, space);
        },
      ),
    );
  }
}

/** Bounds how long a claimed record may stay processing before a surviving runtime may reclaim it. */
const CANDIDATE_PROCESSING_LEASE_MS = 30 * 60 * 1000;

/** Retains one failed candidate with its exact cause and the time it was processed. */
function failCandidate(candidate: MemoryCandidate, error: string): void {
  candidate.status = "failed"; candidate.lastError = error; candidate.updatedAt = Date.now();
}

/** Extracts valid candidates while retaining individual source failures and isolating batch errors. */
async function processCandidates(ownerKey: string, candidates: MemoryCandidate[], owner: CharacterRepositoryOwner): Promise<void> {
  const failures = new Map<string, string>(), chatId = candidates[0].chatId;
  try {
    await owner.run(
      /** Validates each source independently before committing valid extraction and failed metadata together. */
      async repository => {
        let messages = await readMessages(chatId);
        const valid: MemoryCandidate[] = [], excludedTimestamps = new Set<number>();
        for (const candidate of candidates) {
          try {
            const source = messages.filter(
              /** Resolves only the queued revision rather than substituting another message. */
              message => message.timestamp === candidate.triggerMessageTimestamp && message.variantIndex === candidate.triggerVariantIndex,
            );
            if (source.length === 0) throw new Error("源消息已删除或所选版本已变化，已跳过该候选：" + candidate.id);
            if (source.length !== 1) throw new Error("源消息无法唯一定位，已跳过该候选：" + candidate.id);
            const sender = source[0].sender;
            if (candidate.sourceType === "selected_user_message" ? sender !== "user" : sender !== "ai" && sender !== "assistant") throw new Error("源消息的发送者已变化，已跳过该候选：" + candidate.id);
            if (candidate.sourceType === "reply_finalized_auto") await requireMessageOwner(chatId, source[0], ownerKey, repository);
            else await requireChatOwner(chatId, ownerKey, repository);
            valid.push(candidate);
          } catch (error) {
            failures.set(candidate.id, errorText(error)); excludedTimestamps.add(candidate.triggerMessageTimestamp);
          }
        }
        const completed = new Set<string>();
        for (const candidate of valid) { completed.add(candidate.id); excludedTimestamps.delete(candidate.triggerMessageTimestamp); }
        if (valid.length !== 0) {
          if (valid[0].sourceType === "selected_user_message") {
            const timestamps = new Set<number>(); for (const candidate of valid) timestamps.add(candidate.triggerMessageTimestamp);
            messages = messages.filter(
              /** Extracts only the selected user messages that passed source validation. */
              message => message.sender === "user" && timestamps.has(message.timestamp),
            );
          } else {
            let lastTimestamp = 0; for (const candidate of valid) lastTimestamp = Math.max(lastTimestamp, candidate.triggerMessageTimestamp);
            messages = messages.filter(
              /** Keeps bounded context while excluding rejected sources and their changed revisions. */
              message => message.timestamp <= lastTimestamp && !excludedTimestamps.has(message.timestamp),
            ).slice(-48);
          }
          await extractMessages(ownerKey, messages, repository);
        }
        const space = await repository.readMemoryJobState(ownerKey);
        for (const candidate of space.candidates) {
          const failure = failures.get(candidate.id);
          if (failure !== undefined) failCandidate(candidate, failure);
        }
        space.candidates = space.candidates.filter(
          /** Deletes only candidates included in the committed extraction. */
          candidate => !completed.has(candidate.id),
        );
        await repository.writeMemoryJobState(ownerKey, space);
      },
    );
  } catch (error) {
    for (const candidate of candidates) if (!failures.has(candidate.id)) failures.set(candidate.id, errorText(error));
    await publishJobFailure(error,
      /** Commits this batch's failures before continuing independent batches and owners. */
      () => owner.run(
        /** Preserves individual source errors when a later extraction also fails. */
        async repository => {
          const space = await repository.readMemoryJobState(ownerKey);
          for (const candidate of space.candidates) {
            const failure = failures.get(candidate.id);
            if (failure !== undefined) failCandidate(candidate, failure);
          }
          await repository.writeMemoryJobState(ownerKey, space);
        },
      ),
    );
  }
}

interface MemoryJobReport {
  owners: number;
  rebuildWindows: number;
  candidateBatches: number;
}

/** Coalesces interval notifications; durable job claims remain in repository transactions. */
export class MemoryJobRunner {
  private running: Promise<MemoryJobReport> | null = null;

  /** Uses the shared transactional repository for all job and failure publications. */
  constructor(private readonly owner: CharacterRepositoryOwner) {}

  /** Shares the active pass while durable job failures remain visible in their own statuses. */
  tick(): Promise<MemoryJobReport> {
    if (this.running === null) {
      this.running = this.runTick().finally(() => {
        this.running = null;
      });
    }
    return this.running;
  }

  /** Advances only tasks produced by this device while preserving independent owner failures. */
  private async runTick(): Promise<MemoryJobReport> {
    await this.owner.run(
      /** Removes memory owners whose backing entity disappeared. */
      repository => repository.pruneOrphanMemoryOwners(),
    );
    const keys = await this.owner.run(
      /** Enumerates initialized memory owners without loading their corpora. */
      repository => repository.listMemoryOwnerKeys(),
    ), report = { owners: keys.length, rebuildWindows: 0, candidateBatches: 0 };
    const failures: Array<{ ownerKey: string; error: unknown }> = [], producerNodeId = await memoryJobProducer();
    for (const ownerKey of keys) {
      try { await this.runOwner(ownerKey, producerNodeId, report); }
      catch (error) { failures.push({ ownerKey, error }); }
    }
    if (failures.length !== 0) throw new MemoryJobPassError(failures, report);
    return report;
  }

  /** Advances one owner's rebuild window and due candidate batches. */
  private async runOwner(ownerKey: string, producerNodeId: string, report: MemoryJobReport): Promise<void> {
    const runnable = await this.owner.run(
      /** Recovers this device's interrupted work and exposes released tasks with unknown provenance. */
      async repository => {
        const space = await repository.readMemoryJobState(ownerKey), now = Date.now();
        let changed = false;
        for (const candidate of space.candidates) {
          if (candidate.producerNodeId === undefined && candidate.status !== "failed") {
            failCandidate(candidate, "This legacy task has no producing device. Retry it on the device that should process it."); changed = true;
          } else if (candidate.producerNodeId === producerNodeId && candidate.status === "processing" && now - candidate.updatedAt >= CANDIDATE_PROCESSING_LEASE_MS) {
            candidate.status = "pending"; candidate.updatedAt = now; changed = true;
          }
        }
        const task = space.rebuildTask, active = space.rebuildProgress.status === "preparing" || space.rebuildProgress.status === "running";
        if (task !== null && task.producerNodeId === undefined && active) {
          space.rebuildProgress.status = "failed";
          space.rebuildProgress.lastError = "This legacy rebuild has no producing device. Start a new rebuild on the device that should process it.";
          changed = true;
        }
        if (changed) await repository.writeMemoryJobState(ownerKey, space);
        return (task !== null && task.producerNodeId === producerNodeId && active)
          || space.candidates.some(/** Selects only runnable requests created on this device. */ candidate => candidate.producerNodeId === producerNodeId && candidate.status === "pending");
      },
    );
    if (!runnable) return;
    const taskId = await this.owner.run(
      /** Reads a persisted runnable rebuild identity without fabricating a job. */
      async repository => {
        const space = await repository.readMemoryJobState(ownerKey);
        return space.rebuildTask !== null && space.rebuildTask.producerNodeId === producerNodeId && (space.rebuildProgress.status === "preparing" || space.rebuildProgress.status === "running") ? space.rebuildTask.id : null;
      },
    );
    if (taskId !== null) { await processRebuildWindow(ownerKey, taskId, this.owner); report.rebuildWindows += 1; }
    const batches = await this.owner.run(
      /** Claims only due pending records; failed attempts are never automatically retried. */
      async repository => {
        if (!(await repository.readGlobalMemoryPreferences()).autoUpdateEnabled) return [] as MemoryCandidate[][];
        const space = await repository.readMemoryJobState(ownerKey), now = Date.now();
        if (space.settings.autoSaveIntervalMinutes === 0 || now < space.settings.nextAutoSaveRunAtMs) return [] as MemoryCandidate[][];
        const pending = space.candidates.filter(
          /** Selects genuinely pending records, not terminal failures. */
          candidate => candidate.producerNodeId === producerNodeId && candidate.status === "pending",
        );
        const batches: MemoryCandidate[][] = [];
        if (pending.length >= 5) {
          const grouped = new Map<string, MemoryCandidate[]>();
          for (const candidate of pending) { const list = grouped.get(candidate.chatId); if (list === undefined) grouped.set(candidate.chatId, [candidate]); else list.push(candidate); }
          for (const records of grouped.values()) {
            const capped = records.sort(
              /** Applies the Kotlin oldest-source order before the per-chat twenty-record cap. */
              (left, right) => left.triggerMessageTimestamp - right.triggerMessageTimestamp || left.createdAt - right.createdAt,
            ).slice(0, 20);
            for (const sourceType of ["selected_user_message", "reply_finalized_auto"] as const) {
              const batch = capped.filter(
                /** Preserves the original selected/automatic source partition. */
                candidate => candidate.sourceType === sourceType,
              );
              if (batch.length !== 0) batches.push(batch);
            }
          }
        }
        space.settings.nextAutoSaveRunAtMs = now + space.settings.autoSaveIntervalMinutes * 60000;
        await repository.writeMemoryJobState(ownerKey, space); return batches;
      },
    );
    for (const batch of batches) {
      const claimed = await this.owner.run(
        /** Claims each batch immediately before processing so independent failures cannot strand later work. */
        async repository => {
          if (!(await repository.readGlobalMemoryPreferences()).autoUpdateEnabled) return false;
          const space = await repository.readMemoryJobState(ownerKey), ids = new Set<string>();
          for (const candidate of batch) ids.add(candidate.id);
          let count = 0;
          for (const candidate of space.candidates) if (ids.has(candidate.id)) {
            if (candidate.producerNodeId !== producerNodeId || candidate.status !== "pending") throw new Error("Candidate is no longer pending on this device: " + candidate.id);
            candidate.status = "processing"; candidate.attemptCount += 1; candidate.updatedAt = Date.now(); count += 1;
          }
          if (count !== batch.length) throw new Error("A selected candidate no longer exists");
          await repository.writeMemoryJobState(ownerKey, space); return true;
        },
      );
      if (!claimed) break;
      await processCandidates(ownerKey, batch, this.owner); report.candidateBatches += 1;
    }
  }
}

import type { CharacterRepository, CharacterRepositoryOwner } from "../canonical";
import type { MemoryCandidate, MemoryRebuild, MemoryRebuildProgress, MemoryRebuildTask } from "../model";
import { assertInteger, assertObject, assertStrings, requireId } from "../validation";
import { extractMessages } from "./analysis";
import { chatParticipants, planWindows, primaryOwner, readMessages, requireChat, requireChatOwner, requireMessageOwner, requireMessagesOwner } from "./chat";
import { readMessageExtensionMarker } from "../chat-extensions";

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

/** Publishes a real terminal status and propagates the execution error or the two explicit failures. */
async function propagateJobFailure(error: unknown, publishStatus: () => Promise<unknown>): Promise<never> {
  try { await publishStatus(); }
  catch (statusError) { throw new MemoryJobStatusError(error, statusError); }
  throw error;
}

/** Enqueues a real selected message or finalized reply for exact bound memory owners. */
export async function enqueueCandidate(input: CandidateRequest, repository: CharacterRepository): Promise<CandidateResult> {
  await requireChat(input.chatId); assertInteger(input.timestamp, "candidate message timestamp", 0); assertInteger(input.variantIndex, "candidate variant index", 0);
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
    const marker = await readMessageExtensionMarker(input.chatId, message.timestamp, message.variantIndex);
    const ownerKey = input.ownerKey === null ? marker.primaryOwnerKey : input.ownerKey;
    if (!marker.profile.resources.some(
      /** Authorizes only the exact submitted resource through the already-read historical snapshot. */
      resource => resource.key === ownerKey && resource.writable,
    )) throw new Error("Saved candidate identity cannot write this memory owner: " + ownerKey);
    await repository.readMemorySpace(ownerKey); owners.add(ownerKey);
  } else if (input.ownerKey !== null) {
    await requireChatOwner(input.chatId, input.ownerKey, repository); owners.add(input.ownerKey);
  } else {
    const participants = await chatParticipants(input.chatId, repository);
    if (participants.length !== 1) throw new Error("A group selected-user candidate requires an explicit owner");
    owners.add(await primaryOwner(participants[0], repository));
  }
  const result: CandidateResult = { owners: [...owners], candidateIds: [] }, now = Date.now();
  for (const ownerKey of owners) {
    const space = await repository.readMemorySpace(ownerKey), previous = space.candidates.filter(
      /** Uses the full owner-scoped source identity for genuine hook idempotency. */
      candidate => candidate.chatId === input.chatId && candidate.triggerMessageTimestamp === input.timestamp && candidate.triggerVariantIndex === input.variantIndex && candidate.sourceType === input.sourceType,
    );
    if (previous.length > 1) throw new Error("Duplicate persisted memory candidate source");
    if (previous.length === 1) { result.candidateIds.push(previous[0].id); continue; }
    const id = await repository.allocateRecordId();
    space.candidates.push({ id, chatId: input.chatId, triggerMessageTimestamp: input.timestamp, triggerVariantIndex: input.variantIndex, createdAt: now, updatedAt: now, status: "pending", attemptCount: 0, lastError: "", sourceType: input.sourceType });
    if (space.settings.autoSaveIntervalMinutes > 0 && space.settings.nextAutoSaveRunAtMs === 0) space.settings.nextAutoSaveRunAtMs = now + space.settings.autoSaveIntervalMinutes * 60000;
    await repository.writeMemorySpace(ownerKey, space); result.candidateIds.push(id);
  }
  return result;
}

/** Plans and persists a genuine requested rebuild for later existing host interval execution. */
export async function startMemoryRebuild(ownerKey: string, rebuild: MemoryRebuild, repository: CharacterRepository): Promise<MemoryRebuildProgress> {
  assertObject(rebuild, "memory rebuild"); assertStrings(rebuild.chatIds, "rebuild chat ids"); assertInteger(rebuild.windowMessageCount, "rebuild window size", 1);
  for (const key of ["fromInclusive", "toInclusive"] as const) if (rebuild[key] !== null) assertInteger(rebuild[key], key, 0);
  if (rebuild.fromInclusive !== null && rebuild.toInclusive !== null && rebuild.fromInclusive > rebuild.toInclusive) throw new Error("Rebuild start must not follow its end");
  if (rebuild.chatIds.length === 0) throw new Error("Rebuild requires selected chats");
  const space = await repository.readMemorySpace(ownerKey);
  if (space.rebuildProgress.status === "preparing" || space.rebuildProgress.status === "running") throw new Error("Memory rebuild is already active");
  const task: MemoryRebuildTask = { id: await repository.allocateRecordId(), windows: [], nextWindow: 0 }, ids = new Set<string>();
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
  await repository.writeMemorySpace(ownerKey, space); return space.rebuildProgress;
}

/** Runs one real planned window and commits the domain edits with its durable progress cursor. */
async function processRebuildWindow(ownerKey: string, taskId: string, owner: CharacterRepositoryOwner): Promise<void> {
  try {
    await owner.run(
      /** Keeps one model result, graph mutation and progress checkpoint in the same publication. */
      async repository => {
        const space = await repository.readMemorySpace(ownerKey), task = space.rebuildTask;
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
          if (present.length !== 1 || JSON.stringify(present[0]) !== JSON.stringify(source)) throw new Error("A planned rebuild source message revision changed or was deleted");
        }
        await requireMessagesOwner(window.chatId, window.messages, ownerKey, repository);
        await extractMessages(ownerKey, window.messages, repository);
        const updated = await repository.readMemorySpace(ownerKey), savedTask = updated.rebuildTask;
        if (savedTask === null || savedTask.id !== taskId || savedTask.nextWindow !== task.nextWindow) throw new Error("Rebuild cursor changed within execution");
        savedTask.nextWindow += 1; updated.rebuildProgress.completedWindows += 1; updated.rebuildProgress.processedSourceMessages += window.sourceMessageCount;
        updated.rebuildProgress.currentChatTitle = window.chatTitle;
        const next = savedTask.windows[savedTask.nextWindow]; if (next === undefined || next.chatId !== window.chatId) updated.rebuildProgress.completedChats += 1;
        updated.rebuildProgress.status = next === undefined ? "completed" : "running";
        await repository.writeMemorySpace(ownerKey, updated);
      },
    );
  } catch (error) {
    await propagateJobFailure(error,
      /** Persists terminal metadata through the same authoritative queue without retrying the failed work. */
      () => owner.run(
        /** Records the real terminal failure after the unsuccessful operation has published no edits. */
        async repository => {
          const space = await repository.readMemorySpace(ownerKey);
          if (space.rebuildTask === null || space.rebuildTask.id !== taskId) throw new Error("Failed rebuild task identity changed");
          if (space.rebuildProgress.status !== "cancelled") { space.rebuildProgress.status = "failed"; space.rebuildProgress.failedWindows += 1; space.rebuildProgress.lastError = errorText(error); await repository.writeMemorySpace(ownerKey, space); }
        },
      ),
    );
  }
}

/** Executes one claimed candidate batch and deletes it only with a successful extraction commit. */
async function processCandidates(ownerKey: string, candidates: MemoryCandidate[], owner: CharacterRepositoryOwner): Promise<void> {
  const ids = new Set<string>(); for (const candidate of candidates) ids.add(candidate.id);
  const chatId = candidates[0].chatId;
  try {
    await owner.run(
      /** Validates the current chat owner and applies a genuine selected/automatic message batch. */
      async repository => {
        await requireChat(chatId);
        let messages = await readMessages(chatId);
        for (const candidate of candidates) {
          const source = messages.filter(
            /** Revalidates each claimed source identity before any analysis or destructive queue update. */
            message => message.timestamp === candidate.triggerMessageTimestamp && message.variantIndex === candidate.triggerVariantIndex,
          );
          if (source.length !== 1) throw new Error("A claimed candidate source no longer identifies exactly one message: " + candidate.id);
          const sender = source[0].sender;
          if (candidate.sourceType === "selected_user_message" ? sender !== "user" : sender !== "ai" && sender !== "assistant") throw new Error("A claimed candidate source has changed sender: " + candidate.id);
          if (candidate.sourceType === "reply_finalized_auto") await requireMessageOwner(chatId, source[0], ownerKey, repository);
          else await requireChatOwner(chatId, ownerKey, repository);
        }
        if (candidates.every(
          /** Recognizes only the exact declared selected-user source type. */
          candidate => candidate.sourceType === "selected_user_message",
        )) {
          const timestamps = new Set<number>(); for (const candidate of candidates) timestamps.add(candidate.triggerMessageTimestamp);
          messages = messages.filter(
            /** Selects the genuinely persisted requested user messages. */
            message => message.sender === "user" && timestamps.has(message.timestamp),
          );
          if (messages.length !== timestamps.size) throw new Error("A selected candidate message no longer exists");
        } else {
          let lastTimestamp = 0; for (const candidate of candidates) lastTimestamp = Math.max(lastTimestamp, candidate.triggerMessageTimestamp);
          messages = messages.filter(
            /** Bounds automatic extraction to actual source records at the final candidate. */
            message => message.timestamp <= lastTimestamp,
          ).slice(-48);
        }
        await extractMessages(ownerKey, messages, repository);
        const space = await repository.readMemorySpace(ownerKey);
        space.candidates = space.candidates.filter(
          /** Removes only the explicitly claimed batch after successful domain mutations. */
          candidate => !ids.has(candidate.id),
        );
        await repository.writeMemorySpace(ownerKey, space);
      },
    );
  } catch (error) {
    await propagateJobFailure(error,
      /** Persists terminal metadata through the same authoritative queue without retrying the failed work. */
      () => owner.run(
        /** Persists a real failed state without automatic retries or successful empty output. */
        async repository => {
          const space = await repository.readMemorySpace(ownerKey);
          for (const candidate of space.candidates) if (ids.has(candidate.id)) { candidate.status = "failed"; candidate.lastError = errorText(error); candidate.updatedAt = Date.now(); }
          await repository.writeMemorySpace(ownerKey, space);
        },
      ),
    );
  }
}

/** Owns runtime scheduling only; every durable candidate, plan and result remains in the shared file repository. */
export class MemoryJobRunner {
  private busy = false;
  private readonly waiters: (() => void)[] = [];
  /** Receives the same sole repository owner used by UI, commands and public APIs. */
  constructor(private readonly owner: CharacterRepositoryOwner) {}
  /** Serializes genuine interval invocations without launching overlapping extraction batches. */
  private async acquire(): Promise<void> {
    if (!this.busy) { this.busy = true; return; }
    await new Promise<void>(
      /** Parks only this invocation until the previous interval work completes. */
      resolve => this.waiters.push(resolve),
    );
  }
  /** Releases one waiting interval while retaining the same runtime owner. */
  private release(): void { const next = this.waiters.shift(); if (next === undefined) this.busy = false; else next(); }
  /** Advances actual rebuild windows and due pending candidates through existing interval hooks. */
  async tick(): Promise<{ owners: number; rebuildWindows: number; candidateBatches: number }> {
    await this.acquire();
    try {
      const keys = await this.owner.run(
        /** Enumerates only genuine initialized memory spaces. */
        repository => repository.listMemoryOwnerKeys(),
      ), report = { owners: keys.length, rebuildWindows: 0, candidateBatches: 0 };
      for (const ownerKey of keys) {
        const taskId = await this.owner.run(
          /** Reads a persisted runnable rebuild identity without fabricating a job. */
          async repository => {
            const space = await repository.readMemorySpace(ownerKey);
            return space.rebuildTask !== null && (space.rebuildProgress.status === "preparing" || space.rebuildProgress.status === "running") ? space.rebuildTask.id : null;
          },
        );
        if (taskId !== null) { await processRebuildWindow(ownerKey, taskId, this.owner); report.rebuildWindows += 1; }
        const batches = await this.owner.run(
          /** Claims only due pending records; failed attempts are never automatically retried. */
          async repository => {
            const space = await repository.readMemorySpace(ownerKey), now = Date.now();
            if (space.settings.autoSaveIntervalMinutes === 0 || now < space.settings.nextAutoSaveRunAtMs) return [] as MemoryCandidate[][];
            const pending = space.candidates.filter(
              /** Selects genuinely pending records, not terminal failures. */
              candidate => candidate.status === "pending",
            );
            const batches: MemoryCandidate[][] = [];
            if (pending.length >= 5) {
              const grouped = new Map<string, MemoryCandidate[]>();
              for (const candidate of pending) { const list = grouped.get(candidate.chatId); if (list === undefined) grouped.set(candidate.chatId, [candidate]); else list.push(candidate); }
              for (const records of grouped.values()) {
                const capped = records.slice(0, 20);
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
            await repository.writeMemorySpace(ownerKey, space); return batches;
          },
        );
        for (const batch of batches) {
          await this.owner.run(
            /** Claims only the next actual batch so a failure leaves later sources genuinely pending. */
            async repository => {
              const space = await repository.readMemorySpace(ownerKey), ids = new Set<string>();
              for (const candidate of batch) ids.add(candidate.id);
              let count = 0;
              for (const candidate of space.candidates) if (ids.has(candidate.id)) {
                if (candidate.status !== "pending") throw new Error("Candidate is no longer pending: " + candidate.id);
                candidate.status = "processing"; candidate.attemptCount += 1; candidate.updatedAt = Date.now(); count += 1;
              }
              if (count !== batch.length) throw new Error("A selected candidate no longer exists");
              await repository.writeMemorySpace(ownerKey, space);
            },
          );
          await processCandidates(ownerKey, batch, this.owner); report.candidateBatches += 1;
        }
      }
      return report;
    } finally { this.release(); }
  }
}

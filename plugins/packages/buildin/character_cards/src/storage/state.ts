import type { CharacterState, MemorySpace, Card } from "../model";
import { uuid } from "../identity";
import { operitDefaultContent } from "../default-character";
import { createCharacterDraft } from "../drafts";
import { assertEmbedding } from "../memory-jobs/embeddings";
import { assertBoolean, assertInteger, assertMemory, assertMemoryChatMessage, assertMemorySettings, assertNumber, assertObject, assertSearchConfig, assertString, requireId } from "../validation";

/** Creates an ordinary installation card with its own UUID and editable template content. */
export function createInstalledCharacter(now: number): Card {
  return { ...createCharacterDraft(), ...operitDefaultContent, id: uuid(), createdAt: now, updatedAt: now };
}
/** Resolves this plugin's real owner document path without permitting identity traversal. */
export function userDocumentPath(ownerKey: string): string { return "owners/" + encodeURIComponent(ownerKey) + "/USER.md"; }
/** Creates a genuine empty memory space when its owning character or library is created. */
export function createMemorySpace(ownerKey: string): MemorySpace {
  return {
    ownerKey, memories: [], links: [], chunks: [], candidates: [], userDocumentPath: userDocumentPath(ownerKey),
    settings: { autoSaveIntervalMinutes: 5, nextAutoSaveRunAtMs: 0, memoryExtractionCustomRules: "", profileAutoUpdateEnabled: true, profileAutoUpdateLocked: false, cloudEmbeddingEnabled: false, cloudEmbeddingEndpoint: "", cloudEmbeddingApiKey: "", cloudEmbeddingModel: "" },
    searchConfig: { scoreMode: "BALANCED", keywordWeight: 10, tagWeight: 0, vectorWeight: 0, edgeWeight: 0.4 },
    rebuildTask: null, embeddings: [],
    rebuildProgress: { status: "idle", totalChats: 0, completedChats: 0, totalWindows: 0, completedWindows: 0, totalSourceMessages: 0, processedSourceMessages: 0, failedWindows: 0, currentChatTitle: "", lastError: "" },
  };
}
/** Bounds the per-turn recall injection count; the menu input and persisted writes share this range. */
export const MEMORY_RECALL_LIMIT_RANGE = { min: 1, max: 50 } as const;
/** Defaults for recall settings that predate a given field in persisted states. */
export function defaultMemoryRecallSettings(): import("../model").MemoryRecallSettings {
  return { enabled: true, limit: 3, allowRepeated: false };
}
/** Validates one persisted recall-settings record, defaulting fields that older writers omitted. */
export function normalizeMemoryRecallSettings(value: unknown): import("../model").MemoryRecallSettings {
  assertObject(value, "memory recall settings");
  assertBoolean(value.enabled, "memory recall enabled");
  if (value.limit !== undefined) {
    assertInteger(value.limit, "memory recall limit", MEMORY_RECALL_LIMIT_RANGE.min);
    if (value.limit > MEMORY_RECALL_LIMIT_RANGE.max) throw new Error("memory recall limit 必须不超过 " + MEMORY_RECALL_LIMIT_RANGE.max);
  }
  if (value.allowRepeated !== undefined) assertBoolean(value.allowRepeated, "memory recall allow repeated");
  return { enabled: value.enabled, limit: value.limit ?? 3, allowRepeated: value.allowRepeated ?? false };
}
/** Creates first-install domain state without reading or migrating Core records. */
export function createInitialState(now: number): CharacterState {
  const card = createInstalledCharacter(now);
  return { version: 1, memoryPreferences: { userDescriptionEnabled: true, autoUpdateEnabled: true }, memoryRecall: defaultMemoryRecallSettings(), conversationGroups: [], cards: [card], groups: [], tags: [], stores: [], active: { CharacterCard: { id: card.id } }, owners: [createMemorySpace("character:" + card.id)] };
}
/** Requires a real array instead of supplying missing persisted records. */
function array(value: unknown, label: string): asserts value is unknown[] {
  if (!Array.isArray(value)) throw new Error(label + " must be an array");
}
/** Validates an identity list without silently discarding duplicates. */
function identities(records: unknown[], label: string): Set<string> {
  const ids = new Set<string>();
  for (const record of records) {
    assertObject(record, label);
    const id = requireId(record.id, label + " id");
    if (ids.has(id)) throw new Error(label + " has duplicate id: " + id);
    ids.add(id);
  }
  return ids;
}
/** Validates a complete memory space, including document chunks and relationship endpoints. */
export function assertMemorySpace(value: unknown): asserts value is MemorySpace {
  assertObject(value, "memory space"); requireId(value.ownerKey, "memory owner");
  array(value.memories, "memories"); array(value.links, "links"); array(value.chunks, "chunks"); array(value.candidates, "candidates");
  const memoryIds = identities(value.memories, "memories"); identities(value.links, "links"); identities(value.chunks, "chunks"); identities(value.candidates, "candidates");
  const memoriesByUuid = new Map<string, import("../model").Memory>();
  for (const memory of value.memories) {
    assertMemory(memory);
    if (memoriesByUuid.has(memory.uuid)) throw new Error("Duplicate memory UUID: " + memory.uuid);
    memoriesByUuid.set(memory.uuid, memory);
  }
  for (const link of value.links) {
    assertObject(link, "link"); requireId(link.id, "link id");
    const source = requireId(link.sourceMemoryId, "source memory id"), target = requireId(link.targetMemoryId, "target memory id");
    if (!memoryIds.has(source) || !memoryIds.has(target)) throw new Error("Relationship references a missing memory: " + link.id);
    assertString(link.type_, "link type"); assertNumber(link.weight, "link weight", 0, 1); assertString(link.description, "link description");
  }
  const indices = new Set<string>();
  for (const chunk of value.chunks) {
    assertObject(chunk, "chunk"); requireId(chunk.id, "chunk id"); const memoryUuid = requireId(chunk.memoryUuid, "chunk memory UUID");
    assertInteger(chunk.chunkIndex, "chunk index", 0); assertString(chunk.content, "chunk content");
    const memory = memoriesByUuid.get(memoryUuid);
    if (memory === undefined || !memory.isDocumentNode) throw new Error("Chunk references a missing document memory");
    const identity = JSON.stringify([chunk.memoryUuid, chunk.chunkIndex]);
    if (indices.has(identity)) throw new Error("Duplicate document chunk index"); indices.add(identity);
  }
  // Released queues may contain independent task IDs for the same migrated message revision.
  for (const candidate of value.candidates) {
    assertObject(candidate, "candidate"); requireId(candidate.id, "candidate id"); requireId(candidate.chatId, "candidate chat id");
    if (candidate.producerNodeId !== undefined) requireId(candidate.producerNodeId, "candidate producer node id");
    for (const field of ["triggerMessageTimestamp", "triggerVariantIndex", "createdAt", "updatedAt", "attemptCount"] as const) assertInteger(candidate[field], field, 0);
    for (const field of ["status", "lastError", "sourceType"] as const) assertString(candidate[field], field);
    if (candidate.status !== "pending" && candidate.status !== "processing" && candidate.status !== "failed") throw new Error("Invalid candidate status");
    if (candidate.sourceType !== "reply_finalized_auto" && candidate.sourceType !== "selected_user_message") throw new Error("Invalid candidate source type");
    if (typeof candidate.triggerVariantIndex !== "number" || candidate.triggerVariantIndex > 2147483647) throw new Error("Candidate message variant exceeds the host range");
  }
  assertMemorySettings(value.settings); assertSearchConfig(value.searchConfig); assertString(value.userDocumentPath, "USER.md path");
  if (value.userDocumentPath !== userDocumentPath(requireId(value.ownerKey, "owner key"))) throw new Error("Invalid owner USER.md path");
  assertObject(value.rebuildProgress, "rebuild progress");
  for (const field of ["status", "currentChatTitle", "lastError"] as const) assertString(value.rebuildProgress[field], field);
  for (const field of ["totalChats", "completedChats", "totalWindows", "completedWindows", "totalSourceMessages", "processedSourceMessages", "failedWindows"] as const) assertInteger(value.rebuildProgress[field], field, 0);
  if (!["idle", "preparing", "running", "completed", "failed", "cancelled"].some(
    /** Accepts only the declared durable task lifecycle states. */
    status => value.rebuildProgress !== null && typeof value.rebuildProgress === "object" && "status" in value.rebuildProgress && value.rebuildProgress.status === status,
  )) throw new Error("Invalid memory rebuild status");
  if (value.rebuildTask !== null) {
    assertObject(value.rebuildTask, "rebuild task"); requireId(value.rebuildTask.id, "rebuild task id");
    if (value.rebuildTask.producerNodeId !== undefined) requireId(value.rebuildTask.producerNodeId, "rebuild producer node id");
    array(value.rebuildTask.windows, "rebuild windows"); assertInteger(value.rebuildTask.nextWindow, "rebuild cursor", 0);
    if (value.rebuildTask.windows.length === 0 || value.rebuildTask.nextWindow > value.rebuildTask.windows.length) throw new Error("Invalid rebuild cursor or source plan");
    let sourceTotal = 0, processedTotal = 0, completedChats = 0, windowIndex = 0;
    const chats = new Set<string>(), finishedChats = new Set<string>(); let previousChat: string | null = null;
    for (const window of value.rebuildTask.windows) {
      assertObject(window, "rebuild window"); requireId(window.chatId, "rebuild chat id"); assertString(window.chatTitle, "rebuild chat title");
      assertInteger(window.sourceMessageCount, "source message count", 1); array(window.messages, "rebuild messages");
      if (window.messages.length < window.sourceMessageCount) throw new Error("Rebuild source count exceeds the stored messages");
      sourceTotal += window.sourceMessageCount;
      if (windowIndex < value.rebuildTask.nextWindow) processedTotal += window.sourceMessageCount;
      const chatId = requireId(window.chatId, "window chat id");
      if (previousChat !== null && previousChat !== chatId) {
        finishedChats.add(previousChat); if (finishedChats.has(chatId)) throw new Error("Rebuild plan repeats a completed chat block");
        if (windowIndex <= value.rebuildTask.nextWindow) completedChats += 1;
      }
      previousChat = chatId; chats.add(chatId); windowIndex += 1;
      for (const message of window.messages) {
        assertMemoryChatMessage(message);
        if (message.sender !== "user" && message.sender !== "ai" && message.sender !== "assistant") throw new Error("Rebuild window has an unsupported source sender");
        assertString(message.content, "source message content");
        if (message.content.trim() === "") throw new Error("Rebuild window contains a blank source message");
      }
    }
    if (value.rebuildTask.nextWindow === value.rebuildTask.windows.length) completedChats += 1;
    if (value.rebuildProgress.totalWindows !== value.rebuildTask.windows.length || value.rebuildProgress.completedWindows !== value.rebuildTask.nextWindow) throw new Error("Rebuild plan and progress disagree");
    if (value.rebuildProgress.totalSourceMessages !== sourceTotal || value.rebuildProgress.processedSourceMessages !== processedTotal || value.rebuildProgress.totalChats !== chats.size || value.rebuildProgress.completedChats !== completedChats) throw new Error("Rebuild counters disagree with the actual source plan");
    if (value.rebuildProgress.status === "completed" && value.rebuildTask.nextWindow !== value.rebuildTask.windows.length) throw new Error("Completed rebuild has unprocessed source windows");
    if (value.rebuildProgress.status === "preparing" && value.rebuildTask.nextWindow !== 0) throw new Error("Preparing rebuild already has processed source windows");
  } else if (value.rebuildProgress.status !== "idle") throw new Error("Rebuild progress is missing its source plan");
  array(value.embeddings, "embeddings"); const embeddingKeys = new Set<string>();
  for (const item of value.embeddings) {
    assertEmbedding(item); const key = JSON.stringify([item.endpoint, item.model, item.text]);
    if (embeddingKeys.has(key)) throw new Error("Duplicate stored embedding input"); embeddingKeys.add(key);
  }
}

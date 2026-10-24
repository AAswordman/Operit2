import type { CharacterState, MemorySpace, Card } from "../model";
import { uuid } from "../identity";
import { operitDefaultContent } from "../default-character";
import { createCharacterDraft } from "../drafts";
import { assertEmbedding } from "../memory-jobs/embeddings";
import { assertBoolean, assertCard, assertConversationGroups, assertGroup, assertInteger, assertMemory, assertMemoryChatMessage, assertMemorySettings, assertNumber, assertObject, assertSearchConfig, assertString, assertTag, requireId } from "../validation";

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
/** Validates every record and cross-record reference before reads or record publication. */
export function assertCharacterState(value: unknown): asserts value is CharacterState {
  assertObject(value, "character state");
  if (value.version !== 1) throw new Error("Unsupported character database version");
  const fields = new Set(["memoryPreferences", "memoryRecall", "version", "cards", "groups", "tags", "stores", "active", "owners", "conversationGroups"]);
  for (const key of Object.keys(value)) if (!fields.has(key)) throw new Error("Unsupported character state field: " + key);
  assertObject(value.memoryPreferences, "global memory preferences");
  assertBoolean(value.memoryPreferences.userDescriptionEnabled, "user description enabled");
  assertBoolean(value.memoryPreferences.autoUpdateEnabled, "auto update enabled");
  normalizeMemoryRecallSettings(value.memoryRecall);
  assertConversationGroups(value.conversationGroups);
  const cards = value.cards, groups = value.groups, tags = value.tags, stores = value.stores, owners = value.owners;
  array(cards, "cards"); array(groups, "groups"); array(tags, "tags"); array(stores, "stores"); array(owners, "owners");
  const cardIds = identities(cards, "cards"), groupIds = identities(groups, "groups"), tagIds = identities(tags, "tags"), storeIds = identities(stores, "stores");
  for (const card of cards) {
    assertCard(card);
    requireId(card.id, "card id");
    for (const id of card.attachedTagIds) if (!tagIds.has(id)) throw new Error("Missing attached tag: " + id);
    // Model binding validity belongs to explicit binding writes and chat execution, so damaged bindings remain editable.
    // Primary memory selections remain editable when their old library is absent; writes and execution validate the target.
    for (const mount of card.sharedMemoryMounts) if (!storeIds.has(mount.sharedMemoryId)) throw new Error("Missing shared memory mount: " + mount.sharedMemoryId);
  }
  for (const group of groups) {
    assertGroup(group);
    const members = new Set<string>();
    for (const member of group.members) {
      if (!cardIds.has(member.characterCardId)) throw new Error("Missing group member: " + member.characterCardId);
      if (members.has(member.characterCardId)) throw new Error("Duplicate group member: " + member.characterCardId);
      members.add(member.characterCardId);
    }
  }
  for (const tag of tags) assertTag(tag);
  for (const store of stores) {
    assertObject(store, "store"); assertString(store.name, "store name");
    assertInteger(store.createdAt, "store createdAt", 0); assertInteger(store.updatedAt, "store updatedAt", 0);
  }
  assertObject(value.active, "active");
  const activeKeys = Object.keys(value.active);
  if (activeKeys.length !== 1) throw new Error("Active prompt must select exactly one entity");
  const activeKey = activeKeys[0];
  if (activeKey !== "CharacterCard" && activeKey !== "CharacterGroup" && activeKey !== "Model") throw new Error("Invalid active prompt kind");
  const selected = value.active[activeKey]; assertObject(selected, "active selection");
  const activeId = requireId(selected.id, "active id");
  // Stored active identities remain available for explicit reselection after their role is deleted.
  if (activeKey === "Model" && activeId !== "chat") throw new Error("Invalid active model selection: " + activeId);
  const expectedOwners = new Set<string>();
  for (const id of cardIds) expectedOwners.add("character:" + id);
  for (const id of storeIds) expectedOwners.add("shared:" + id);
  const actualOwners = new Set<string>();
  for (const owner of owners) {
    assertObject(owner, "memory space");
    const ownerKey = requireId(owner.ownerKey, "owner key");
    if (!expectedOwners.has(ownerKey) || actualOwners.has(ownerKey)) throw new Error("Invalid or duplicate memory owner: " + ownerKey);
    actualOwners.add(ownerKey);
    assertMemorySpace(owner);
  }
  if (actualOwners.size !== expectedOwners.size) throw new Error("Character state is missing an owner memory space");
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

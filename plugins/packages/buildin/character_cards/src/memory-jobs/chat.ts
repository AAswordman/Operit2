import type { CharacterRepository } from "../canonical";
import type { Card, MemoryChat, MemoryChatListing, MemoryChatMessage, MemoryJobWindow, MemoryRebuild } from "../model";
import { requireChatSelection } from "../chat-bindings";
import type { ChatResourceRoute } from "../api";
import { readMessageAuthor } from "../chat-storage";
import { assertBoolean, assertInteger, assertMemoryChatMessage, assertObject, assertString, requireId } from "../validation";

/** Requires exact generic chat-summary fields without inventing unavailable history metadata. */
function assertChat(value: unknown): asserts value is MemoryChat {
  assertObject(value, "chat"); requireId(value.id, "chat id"); assertString(value.title, "chat title");
  for (const key of ["messageCount", "inputTokens", "outputTokens"] as const) assertInteger(value[key], key, 0);
  for (const key of ["createdAt", "updatedAt"] as const) assertString(value[key], key);
  assertBoolean(value.isCurrent, "current chat");
}

/** Requires an exact host identity through the existing lookup, without relying on capped title lists. */
export async function requireChat(chatId: string): Promise<MemoryChat> {
  requireId(chatId, "chat id");
  const result = await Tools.Chat.findChat({ query: chatId, match: "exact", index: 0 });
  assertInteger(result.matchedCount, "matched chat count", 1);
  assertChat(result.chat);
  if (result.matchedCount !== 1 || result.chat.id !== chatId) throw new Error("Chat lookup does not identify exactly one host record: " + chatId);
  return result.chat;
}

/** Resolves participants solely from the real persisted plugin chat selection. */
export async function chatParticipants(chatId: string, repository: CharacterRepository): Promise<Card[]> {
  const binding = await repository.chatBindings.readChatBinding(chatId), cards = await repository.listCharacters(), groups = await repository.listGroups();
  const selection = requireChatSelection(binding.selection, cards, groups);
  if (selection.kind === "model") return [];
  if (selection.kind === "card") return [await repository.getCharacter(selection.id)];
  const group = await repository.getGroup(selection.id);
  if (group.members.length === 0) throw new Error("Selected chat group has no participants: " + group.id);
  const members = [...group.members].sort(
    /** Preserves the exact stored participant order. */
    (left, right) => left.orderIndex - right.orderIndex,
  );
  const result: Card[] = [];
  for (const member of members) result.push(await repository.getCharacter(member.characterCardId));
  return result;
}

/** Resolves one actual primary memory owner while enforcing an existing shared-library reference. */
export async function primaryOwner(card: Card, repository: CharacterRepository): Promise<string> {
  switch (card.memoryBindingMode) {
    case "CHARACTER": return "character:" + card.id;
    case "SHARED": {
      const id = requireId(card.sharedMemoryId, "primary shared library"); await repository.getStore(id); return "shared:" + id;
    }
    default: throw new Error("Invalid memory binding mode: " + card.memoryBindingMode);
  }
}

/** Requires the selected participants to own or explicitly permit writes to this resource. */
export async function requireChatOwner(chatId: string, ownerKey: string, repository: CharacterRepository): Promise<void> {
  await repository.readMemoryJobState(ownerKey);
  const cards = await chatParticipants(chatId, repository);
  for (const card of cards) {
    if (await primaryOwner(card, repository) === ownerKey) return;
    for (const mount of card.sharedMemoryMounts) if (mount.writable && "shared:" + mount.sharedMemoryId === ownerKey) { await repository.getStore(mount.sharedMemoryId); return; }
  }
  throw new Error("Chat is not writable for this memory owner: " + chatId + " / " + ownerKey);
}

export interface CurrentMessageMemoryAccess { primaryOwnerKey: string; resources: ChatResourceRoute[] }

/** Resolves the message author against current memory bindings and grants. */
export async function currentMessageMemoryAccess(chatId: string, message: MemoryChatMessage, repository: CharacterRepository): Promise<CurrentMessageMemoryAccess> {
  assertMemoryChatMessage(message);
  const participantId = await readMessageAuthor(chatId, message.timestamp, message.variantIndex);
  if (participantId === "model:chat") return { primaryOwnerKey: "", resources: [] };
  if (participantId === null) throw new Error("Message has no character author identity");
  const card = await repository.getCharacter(participantId);
  const primaryOwnerKey = await primaryOwner(card, repository);
  const resources: ChatResourceRoute[] = [{ key: primaryOwnerKey, readable: true, writable: true }];
  for (const mount of card.sharedMemoryMounts) {
    await repository.getStore(mount.sharedMemoryId);
    const key = "shared:" + mount.sharedMemoryId;
    const existing = resources.find(resource => resource.key === key);
    if (existing) { existing.readable ||= mount.readable; existing.writable ||= mount.writable; }
    else resources.push({ key, readable: mount.readable, writable: mount.writable });
  }
  return { primaryOwnerKey, resources };
}

/** Checks current permissions for the real message author. */
export async function requireMessageOwner(chatId: string, message: MemoryChatMessage, ownerKey: string, repository: CharacterRepository): Promise<CurrentMessageMemoryAccess> {
  await repository.readMemoryJobState(ownerKey);
  const access = await currentMessageMemoryAccess(chatId, message, repository);
  if (!access.resources.some(resource => resource.key === ownerKey && resource.writable)) throw new Error("Message author is not currently writable for this memory owner: " + ownerKey);
  return access;
}

/** Validates extraction against current author permissions. */
export async function requireMessagesOwner(chatId: string, messages: MemoryChatMessage[], ownerKey: string, repository: CharacterRepository): Promise<void> {
  await repository.readMemoryJobState(ownerKey);
  for (const message of messages) {
    if (message.sender !== "ai" && message.sender !== "assistant") continue;
    const access = await currentMessageMemoryAccess(chatId, message, repository);
    if (access.resources.some(resource => resource.key === ownerKey && resource.writable)) return;
  }
  throw new Error("History has no assistant author currently writable for this memory owner: " + ownerKey);
}
/** Lists real host summaries under their stored plugin binding, reporting bindings whose Core conversation disappeared instead of failing the whole listing. */
export async function listMemoryChats(ownerKey: string, repository: Pick<CharacterRepository, "readMemorySettings" | "chatBindings">): Promise<MemoryChatListing> {
  await repository.readMemorySettings(ownerKey);
  const selected = await repository.chatBindings.chatsForMemoryOwner(ownerKey);
  const chats: MemoryChat[] = [], missingChatIds: string[] = [];
  for (let offset = 0; offset < selected.length; offset += 200) {
    const ids = selected.slice(offset, offset + 200), result = await Tools.Chat.listChats({ chat_ids: ids, limit: 200 });
    const remaining = new Set(ids);
    for (const chat of result.chats) { assertChat(chat); if (!remaining.delete(chat.id)) throw new Error("Chat batch returned an unrelated or duplicate identity"); chats.push(chat); }
    for (const chatId of remaining) missingChatIds.push(chatId);
  }
  return {
    chats: chats.sort(
      /** Keeps the host directory's updated-at descending presentation without limiting the bound records. */
      (left, right) => {
        const leftTime = Number(left.updatedAt), rightTime = Number(right.updatedAt);
        assertInteger(leftTime, "chat updated timestamp", 0); assertInteger(rightTime, "chat updated timestamp", 0);
        return rightTime - leftTime;
      },
    ),
    missingChatIds: missingChatIds.sort(),
  };
}

/** Deletes bindings and extraction candidates whose Core conversation or source message no longer exists. */
export async function pruneMissingMemoryChats(
  ownerKey: string,
  repository: Pick<CharacterRepository, "readMemorySettings" | "chatBindings" | "readMemoryJobState" | "writeMemoryJobState">,
): Promise<{ removedChatIds: string[]; removedCandidateIds: string[] }> {
  const { missingChatIds } = await listMemoryChats(ownerKey, repository);
  const removedChatIds: string[] = [];
  for (const chatId of missingChatIds) {
    if ((await repository.chatBindings.deleteChatBinding(chatId)).deleted) removedChatIds.push(chatId);
  }
  const space = await repository.readMemoryJobState(ownerKey);
  const candidateChats = [...new Set(space.candidates.map(candidate => candidate.chatId))];
  const messagesByChat = new Map<string, MemoryChatMessage[] | null>();
  for (let offset = 0; offset < candidateChats.length; offset += 200) {
    const ids = candidateChats.slice(offset, offset + 200), result = await Tools.Chat.listChats({ chat_ids: ids, limit: 200 });
    const found = new Set(result.chats.map(chat => String(chat.id)));
    for (const chatId of ids) messagesByChat.set(chatId, found.has(chatId) ? await readMessages(chatId) : null);
  }
  const removedCandidateIds: string[] = [];
  const survivors = space.candidates.filter(candidate => {
    const messages = messagesByChat.get(candidate.chatId) ?? null;
    const orphaned = messages === null || !messages.some(
      /** Matches the exact claimed source revision before keeping the queued extraction. */
      message => message.timestamp === candidate.triggerMessageTimestamp && message.variantIndex === candidate.triggerVariantIndex,
    );
    if (orphaned) removedCandidateIds.push(candidate.id);
    return !orphaned;
  });
  if (survivors.length !== space.candidates.length) { space.candidates = survivors; await repository.writeMemoryJobState(ownerKey, space); }
  return { removedChatIds, removedCandidateIds };
}

/** Reads genuine chronological source messages; no rich ChatMessage fields are synthesized. */
export async function readMessages(chatId: string): Promise<MemoryChatMessage[]> {
  const chat = await requireChat(chatId);
  if (chat.messageCount > 2147483647) throw new Error("Chat message count exceeds the host's supported index range");
  const end = chat.messageCount === 0 ? 0 : chat.messageCount - 1;
  const result = await Tools.Chat.getMessagesRange(chatId, { order: "asc", start: 0, end });
  if (result.order !== "asc" || result.start !== 0 || result.end !== end || result.limit !== end + 1) throw new Error("Chat message result does not match the requested source range");
  if (result.chatId !== chatId || !Array.isArray(result.messages)) throw new Error("Chat message result has a different identity or missing records");
  if (result.messages.length > chat.messageCount) throw new Error("Chat history changed while reading the planned source range");
  const messages: MemoryChatMessage[] = [], timestamps = new Set<number>();
  for (const message of result.messages) {
    assertMemoryChatMessage(message);
    if (timestamps.has(message.timestamp)) throw new Error("Chat range returned duplicate selected message revisions"); timestamps.add(message.timestamp);
    messages.push({ sender: message.sender, content: message.content, timestamp: message.timestamp, variantIndex: message.variantIndex, variantCount: message.variantCount, provider: message.provider, modelName: message.modelName });
  }
  const current = await requireChat(chatId);
  if (current.messageCount !== chat.messageCount || current.updatedAt !== chat.updatedAt) throw new Error("Chat history changed while reading the planned source range");
  return messages.sort(
    /** Orders genuine message timestamps with stable source ordering. */
    (left, right) => left.timestamp - right.timestamp,
  );
}

/** Ports bounded windows with carried user context, retaining each source reply exactly once. */
export function planWindows(chat: MemoryChat, messages: MemoryChatMessage[], options: MemoryRebuild): MemoryJobWindow[] {
  const size = Math.max(8, Math.min(48, options.windowMessageCount)), windows: MemoryJobWindow[] = [];
  let source: MemoryChatMessage[] = [], context: MemoryChatMessage[] = [], user: MemoryChatMessage | null = null;
  /** Emits a real source window without counting carried context twice. */
  function emit(): void {
    if (source.length === 0) return;
    windows.push({ chatId: chat.id, chatTitle: chat.title, messages: [...context, ...source], sourceMessageCount: source.length }); source = []; context = [];
  }
  for (const message of messages) {
    if (options.fromInclusive !== null && message.timestamp < options.fromInclusive) continue;
    if (options.toInclusive !== null && message.timestamp > options.toInclusive) continue;
    if (message.content.trim() === "") continue;
    switch (message.sender) {
      case "user": if (source.length >= size - 1) emit(); user = message; source.push(message); break;
      case "ai": case "assistant":
        if (user === null) continue;
        if (source.length >= size) { emit(); context.push(user); } source.push(message); break;
    }
  }
  emit(); return windows;
}

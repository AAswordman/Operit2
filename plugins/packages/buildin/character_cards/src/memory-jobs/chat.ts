import type { CharacterRepository } from "../canonical";
import type { Card, MemoryChat, MemoryChatMessage, MemoryJobWindow, MemoryRebuild } from "../model";
import { requireChatSelection } from "../chat-bindings";
import type { MessageMarker } from "../chat-markers";
import { readMessageExtensionMarker } from "../chat-extensions";
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
  const binding = await repository.readChatBinding(chatId), cards = await repository.listCharacters(), groups = await repository.listGroups();
  const selection = requireChatSelection(binding.selection, cards, groups);
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
  await repository.readMemorySpace(ownerKey);
  const cards = await chatParticipants(chatId, repository);
  for (const card of cards) {
    if (await primaryOwner(card, repository) === ownerKey) return;
    for (const mount of card.sharedMemoryMounts) if (mount.writable && "shared:" + mount.sharedMemoryId === ownerKey) { await repository.getStore(mount.sharedMemoryId); return; }
  }
  throw new Error("Chat is not writable for this memory owner: " + chatId + " / " + ownerKey);
}

/** Requires a writable destination from the exact saved message revision, never the current chat selection. */
export async function requireMessageOwner(chatId: string, message: MemoryChatMessage, ownerKey: string, repository: CharacterRepository): Promise<MessageMarker> {
  assertMemoryChatMessage(message); await repository.readMemorySpace(ownerKey);
  const marker = await readMessageExtensionMarker(chatId, message.timestamp, message.variantIndex);
  if (!marker.profile.resources.some(
    /** Checks only this saved participant's explicit writable resource grants. */
    resource => resource.key === ownerKey && resource.writable,
  )) throw new Error("Saved message identity is not writable for this memory owner: " + ownerKey);
  return marker;
}
/** Validates historical assistant snapshots before planning or executing an owner-scoped extraction window. */
export async function requireMessagesOwner(chatId: string, messages: MemoryChatMessage[], ownerKey: string, repository: CharacterRepository): Promise<void> {
  await repository.readMemorySpace(ownerKey); let permitted = false;
  for (const message of messages) {
    if (message.sender !== "ai" && message.sender !== "assistant") continue;
    const marker = await readMessageExtensionMarker(chatId, message.timestamp, message.variantIndex);
    if (marker.profile.resources.some(
      /** Requires an actual saved writable grant without deriving identity from a display name. */
      resource => resource.key === ownerKey && resource.writable,
    )) permitted = true;
  }
  if (!permitted) throw new Error("History has no saved assistant identity writable for this memory owner: " + ownerKey);
}
/** Lists real host summaries under their stored plugin binding, excluding genuinely unbound chats. */
export async function listMemoryChats(ownerKey: string, repository: CharacterRepository): Promise<MemoryChat[]> {
  await repository.readMemorySpace(ownerKey);
  const bindings = await repository.listChatBindings(), selected = new Set<string>();
  for (const binding of bindings) {
    const cards = await chatParticipants(binding.chatId, repository);
    for (const card of cards) {
      if (await primaryOwner(card, repository) === ownerKey) selected.add(binding.chatId);
      for (const mount of card.sharedMemoryMounts) if (mount.writable && "shared:" + mount.sharedMemoryId === ownerKey) selected.add(binding.chatId);
    }
  }
  const chats: MemoryChat[] = [];
  for (const id of selected) chats.push(await requireChat(id));
  return chats.sort(
    /** Keeps the host directory's updated-at descending presentation without limiting the bound records. */
    (left, right) => {
      const leftTime = Number(left.updatedAt), rightTime = Number(right.updatedAt);
      assertInteger(leftTime, "chat updated timestamp", 0); assertInteger(rightTime, "chat updated timestamp", 0);
      return rightTime - leftTime;
    },
  );
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

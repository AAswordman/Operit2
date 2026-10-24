import type { Card, Group, ChatBinding } from "./model";
import { decodeChatMarker, markerObject } from "./chat-markers";
import { assertCard, assertGroup, assertInteger, requireId } from "./validation";
import { CharacterStorage, documents } from "./storage/preferences";
import { equal } from "./storage/records";

let database: Promise<CharacterStorage> | null = null;
/** Retains the plugin's native preference owners for chat binding operations. */
export function chatDatabase(): Promise<CharacterStorage> {
  if (database === null) database = CharacterStorage.open();
  return database;
}
/** Reads the explicitly nullable binding from the plugin's own preference file. */
export async function findChatBinding(chatId: string): Promise<ChatBinding | null> {
  requireId(chatId, "chat id");
  const value = (await (await chatDatabase()).chats.data()).get("binding_" + chatId);
  return value === undefined ? null : { chatId, selection: decodeChatMarker(value).selection };
}
/** Requires a persisted binding without consulting global active selection. */
export async function readChatBinding(chatId: string): Promise<ChatBinding> {
  const binding = await findChatBinding(chatId);
  if (binding === null) throw new Error("Chat binding is unavailable: " + chatId);
  return binding;
}
/** Reads only the actual author ID attached by Core; execution settings are never stored here. */
export async function readMessageAuthor(chatId: string, messageTimestamp: number, variantIndex: number): Promise<string | null> {
  requireId(chatId, "chat id");
  assertInteger(messageTimestamp, "message timestamp", 0);
  assertInteger(variantIndex, "message variant index", 0);
  const { extension } = await Tools.Chat.readExtension({ kind: "message", chatId, messageTimestamp, variantIndex });
  if (extension === null) return null;
  const identity = markerObject(extension, "message author identity");
  if (Object.keys(identity).length !== 1) throw new Error("Invalid message author identity");
  return requireId(identity.participantId, "message author participant id");
}

/** Lists persisted bindings directly from one immutable preference snapshot. */
export async function listChatBindings(): Promise<ChatBinding[]> {
  return (await (await chatDatabase()).chats.data()).entries().filter(([key]) => key.startsWith("binding_")).map(([key, value]) => ({ chatId: requireId(key.slice(8), "stored chat id"), selection: decodeChatMarker(value).selection }));
}
/** Reads selected chat identities from a single JSON preference snapshot. */
export async function bindingsForChats(chatIds: readonly string[]): Promise<Map<string, string>> {
  const preferences = await (await chatDatabase()).chats.data(), result = new Map<string, string>();
  for (const chatId of chatIds) {
    requireId(chatId, "chat id");
    const value = preferences.get("binding_" + chatId);
    if (value !== undefined) result.set(chatId, decodeChatMarker(value).selection);
  }
  return result;
}

/** Resolves primary owners and writable mounts from complete character and group JSON documents. */
export async function chatsForMemoryOwner(ownerKey: string): Promise<string[]> {
  requireId(ownerKey, "memory owner");
  const database = await chatDatabase();
  const before = await database.data();
  const [cards, groups, bindings] = await Promise.all([
    Promise.resolve(documents<Card>(before.cards)), Promise.resolve(documents<Group>(before.groups)), listChatBindings(),
  ]);
  const owners = new Set<string>(), selections = new Set<string>();
  for (const card of cards) {
    assertCard(card);
    const primary = card.memoryBindingMode === "CHARACTER" ? "character:" + card.id : "shared:" + card.sharedMemoryId;
    if (primary === ownerKey || card.sharedMemoryMounts.some(mount => mount.writable && "shared:" + mount.sharedMemoryId === ownerKey)) {
      owners.add(card.id);
      selections.add("card:" + card.id);
    }
  }
  for (const group of groups) {
    assertGroup(group);
    if (group.members.some(member => owners.has(member.characterCardId))) selections.add("group:" + group.id);
  }
  const after = await database.data();
  if (!equal(before.cards.entries(), after.cards.entries()) || !equal(before.groups.entries(), after.groups.entries())) throw new Error("Character configuration changed during the read");
  return bindings.filter(binding => selections.has(binding.selection)).map(binding => binding.chatId);
}
/** Edits a validated chat binding through the native preference transaction. */
export async function writeChatBinding(binding: ChatBinding): Promise<ChatBinding> {
  requireId(binding.chatId, "chat id");
  const marker = decodeChatMarker({ version: 1, selection: binding.selection });
  await (await chatDatabase()).chats.edit(preferences => { preferences.set("binding_" + binding.chatId, { ...marker }); });
  return binding;
}
/** Allocates a draft binding without publishing it in the conversation catalog. */
export async function prepareChatBinding(binding: ChatBinding): Promise<void> {
  requireId(binding.chatId, "draft id");
  const marker = decodeChatMarker({ version: 1, selection: binding.selection });
  await (await chatDatabase()).chats.edit(preferences => {
    const key = "draft_" + binding.chatId;
    if (preferences.contains(key)) throw new Error("Chat draft binding already exists: " + binding.chatId);
    preferences.set(key, { ...marker });
  });
}
/** Resolves only the explicitly prepared draft identity. */
export async function readDraftBinding(chatId: string): Promise<ChatBinding> {
  requireId(chatId, "draft id");
  const value = (await (await chatDatabase()).chats.data()).get("draft_" + chatId);
  if (value === undefined) throw new Error("Chat draft binding is unavailable: " + chatId);
  return { chatId, selection: decodeChatMarker(value).selection };
}
/** Publishes the binding and removes its draft inside the same preference edit. */
export async function commitDraftBinding(chatId: string): Promise<void> {
  requireId(chatId, "draft id");
  await (await chatDatabase()).chats.edit(preferences => {
    const value = preferences.get("draft_" + chatId);
    if (value === undefined) throw new Error("Chat draft binding is unavailable: " + chatId);
    decodeChatMarker(value);
    if (preferences.contains("binding_" + chatId)) throw new Error("Chat binding already exists: " + chatId);
    preferences.set("binding_" + chatId, value);
    preferences.remove("draft_" + chatId);
  });
}
/** Aborts exactly one allocated draft without altering the committed binding. */
export async function abortDraftBinding(chatId: string): Promise<void> {
  requireId(chatId, "draft id");
  await (await chatDatabase()).chats.edit(preferences => {
    if (!preferences.contains("draft_" + chatId)) throw new Error("Chat draft binding is unavailable: " + chatId);
    preferences.remove("draft_" + chatId);
  });
}
/** Deletes the plugin-owned binding and reports whether it was present in the edit snapshot. */
export async function deleteChatBinding(chatId: string): Promise<{ chatId: string; deleted: boolean }> {
  requireId(chatId, "chat id");
  let deleted = false;
  await (await chatDatabase()).chats.edit(preferences => {
    deleted = preferences.contains("binding_" + chatId);
    preferences.remove("binding_" + chatId);
  });
  return { chatId, deleted };
}

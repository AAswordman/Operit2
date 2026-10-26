import type { Card, Group, ChatBinding } from "../model";
import { readCharacter } from "../drafts";
import { decodeChatMarker } from "../chat-markers";
import { requireChatSelection, type CharacterChatBindings } from "../chat-bindings";
import { assertGroup, requireId } from "../validation";
import { type CharacterStorage, documents } from "./preferences";
import { equal } from "./records";

/** Reads and edits bindings through the handles owned by one initialized repository. */
export class CharacterChatStore implements CharacterChatBindings {
  constructor(private readonly storage: CharacterStorage) {}

  /** Validates role references against the same repository's configuration owners. */
  private async requireSelection(selection: string): Promise<void> {
    const configuration = await this.storage.data();
    requireChatSelection(selection, documents<Card>(configuration.cards).map(card => readCharacter(card)), documents<Group>(configuration.groups));
  }

  /** Reads the explicitly nullable binding from the plugin's own preference file. */
  async findChatBinding(chatId: string): Promise<ChatBinding | null> {
    requireId(chatId, "chat id");
    const value = (await this.storage.chats.data()).get("binding_" + chatId);
    return value === undefined ? null : { chatId, selection: decodeChatMarker(value).selection };
  }
  /** Requires a persisted binding without consulting global active selection. */
  async readChatBinding(chatId: string): Promise<ChatBinding> {
    const binding = await this.findChatBinding(chatId);
    if (binding === null) throw new Error("Chat binding is unavailable: " + chatId);
    await this.requireSelection(binding.selection);
    return binding;
  }
  /** Lists persisted bindings directly from one immutable preference snapshot. */
  async listChatBindings(): Promise<ChatBinding[]> {
    return (await this.storage.chats.data()).entries().filter(([key]) => key.startsWith("binding_")).map(([key, value]) => ({ chatId: requireId(key.slice(8), "stored chat id"), selection: decodeChatMarker(value).selection }));
  }
  /** Reads selected chat identities from a single JSON preference snapshot. */
  async bindingsForChats(chatIds: readonly string[]): Promise<Map<string, string>> {
    const preferences = await this.storage.chats.data(), result = new Map<string, string>();
    for (const chatId of chatIds) {
      requireId(chatId, "chat id");
      const value = preferences.get("binding_" + chatId);
      if (value !== undefined) result.set(chatId, decodeChatMarker(value).selection);
    }
    return result;
  }

  /** Resolves primary owners and writable mounts from complete character and group JSON documents. */
  async chatsForMemoryOwner(ownerKey: string): Promise<string[]> {
    requireId(ownerKey, "memory owner");
    const database = this.storage;
    const before = await database.data();
    const [cards, groups, bindings] = await Promise.all([
      Promise.resolve(documents<Card>(before.cards).map(card => readCharacter(card))), Promise.resolve(documents<Group>(before.groups)), this.listChatBindings(),
    ]);
    const owners = new Set<string>(), selections = new Set<string>();
    for (const card of cards) {

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
  async writeChatBinding(binding: ChatBinding): Promise<ChatBinding> {
    requireId(binding.chatId, "chat id");
    const marker = decodeChatMarker({ version: 1, selection: binding.selection });
    await this.requireSelection(binding.selection);
    await this.storage.chats.edit(preferences => { preferences.set("binding_" + binding.chatId, { ...marker }); });
    return binding;
  }
  /** Allocates a draft binding without publishing it in the conversation catalog. */
  async prepareChatBinding(binding: ChatBinding): Promise<void> {
    requireId(binding.chatId, "draft id");
    const marker = decodeChatMarker({ version: 1, selection: binding.selection });
    await this.storage.chats.edit(preferences => {
      const key = "draft_" + binding.chatId;
      if (preferences.contains(key)) throw new Error("Chat draft binding already exists: " + binding.chatId);
      preferences.set(key, { ...marker });
    });
  }
  /** Resolves only the explicitly prepared draft identity. */
  async readDraftBinding(chatId: string): Promise<ChatBinding> {
    requireId(chatId, "draft id");
    const value = (await this.storage.chats.data()).get("draft_" + chatId);
    if (value === undefined) throw new Error("Chat draft binding is unavailable: " + chatId);
    return { chatId, selection: decodeChatMarker(value).selection };
  }
  /** Publishes the binding and removes its draft inside the same preference edit. */
  async commitDraftBinding(chatId: string): Promise<void> {
    requireId(chatId, "draft id");
    await this.storage.chats.edit(preferences => {
      const value = preferences.get("draft_" + chatId);
      if (value === undefined) throw new Error("Chat draft binding is unavailable: " + chatId);
      decodeChatMarker(value);
      if (preferences.contains("binding_" + chatId)) throw new Error("Chat binding already exists: " + chatId);
      preferences.set("binding_" + chatId, value);
      preferences.remove("draft_" + chatId);
    });
  }
  /** Aborts exactly one allocated draft without altering the committed binding. */
  async abortDraftBinding(chatId: string): Promise<void> {
    requireId(chatId, "draft id");
    await this.storage.chats.edit(preferences => {
      if (!preferences.contains("draft_" + chatId)) throw new Error("Chat draft binding is unavailable: " + chatId);
      preferences.remove("draft_" + chatId);
    });
  }
  /** Deletes the plugin-owned binding and reports whether it was present in the edit snapshot. */
  async deleteChatBinding(chatId: string): Promise<{ chatId: string; deleted: boolean }> {
    requireId(chatId, "chat id");
    let deleted = false;
    await this.storage.chats.edit(preferences => {
      deleted = preferences.contains("binding_" + chatId);
      preferences.remove("binding_" + chatId);
    });
    return { chatId, deleted };
  }
}

import { uuid } from "./identity";
import type { CharacterRepository } from "./canonical";
import type { ActivePrompt, BackupImportResult, Card, CharacterDirectories, CharacterState, ConversationGroupRecord, Group, MemorySpace, ModelSummary, Store, Tag, TagValues, ToolCatalog, TtsConfig, UserDocument, UserDocumentWrite } from "./model";
import { assertBoolean, assertConversationGroups, assertGroup, assertString, assertTag, assertTagValues, requireId, requireName } from "./validation";
import { clone } from "./storage/records";
import { createMemorySpace, normalizeMemoryRecallSettings } from "./storage/state";
import { DatabaseMemoryRepository } from "./storage/memory-repository";
import type { CharacterMemoryDatabase } from "./storage/memory";

import type { CharacterChatBindings } from "./chat-bindings";

/** Copies structured domain records so consumers cannot mutate the in-flight snapshot outside this repository. */
function copy<T>(value: T): T { return clone(value); }
/** Requires one exact record and never selects an arbitrary duplicate or manufactures an absent object. */
function get<T extends { id: string }>(records: T[], id: string, label: string): T {
  requireId(id, label + " id");
  const recordsWithId = records.filter(
    /** Resolves the exact identity rather than guessing names. */
    record => record.id === id,
  );
  if (recordsWithId.length !== 1) throw new Error(label + " does not identify exactly one record: " + id);
  return recordsWithId[0];
}
/** Rejects duplicate names when explicitly creating or renaming a domain entity. */
function uniqueName(records: { id: string; name: string }[], name: string, id: string): void {
  for (const record of records) if (record.id !== id && record.name === name) throw new Error("Name already exists: " + name);
}
/** Edits configuration snapshots and delegates memory operations to the shared database repository. */
export class RepositorySession extends DatabaseMemoryRepository implements CharacterRepository {
  private readonly documents = new Map<string, UserDocumentWrite>();

  /** Receives the operation snapshot and explicitly connected external directory readers. */
  constructor(private readonly state: CharacterState, private readonly directory: string, readonly chatBindings: CharacterChatBindings, memory: CharacterMemoryDatabase, private readonly directories: CharacterDirectories | null = null) { super(memory); }
  async readGlobalMemoryPreferences(): Promise<import("./model").GlobalMemoryPreferences> { return copy(this.state.memoryPreferences); }
  async writeGlobalMemoryPreference(key: keyof import("./model").GlobalMemoryPreferences, value: boolean): Promise<import("./model").GlobalMemoryPreferences> {
    if (key !== "userDescriptionEnabled" && key !== "autoUpdateEnabled") throw new Error("Unknown global memory preference");
    assertBoolean(value, key); this.state.memoryPreferences[key] = value; return this.readGlobalMemoryPreferences();
  }
  async readMemoryRecallSettings(): Promise<import("./model").MemoryRecallSettings> { return copy(this.state.memoryRecall); }
  async writeMemoryRecallSettings(settings: import("./model").MemoryRecallSettings): Promise<import("./model").MemoryRecallSettings> {
    this.state.memoryRecall = normalizeMemoryRecallSettings(settings); return this.readMemoryRecallSettings();
  }
  /** Exposes staged owner documents to the sole file publisher without creating another persistent copy. */
  documentWrites(): UserDocumentWrite[] { return [...this.documents.values()].map(
    /** Copies the exact staged write envelope. */
    document => ({ ...document }),
  ); }
  /** Initializes a real owner document only during explicit owner creation. */
  private addSpace(ownerKey: string): void {
    const space = createMemorySpace(ownerKey); this.state.owners.push(space);
    this.documents.set(ownerKey, { ownerKey, path: this.directory + "/" + space.userDocumentPath, content: "", newDocument: true });
  }
  /** Requires an actual owner and its existing complete space. */
  private space(ownerKey: string): MemorySpace {
    const parts = ownerKey.split(":");
    if (parts.length !== 2) throw new Error("Invalid memory owner namespace");
    if (parts[0] === "character") get(this.state.cards, parts[1], "character");
    else if (parts[0] === "shared") get(this.state.stores, parts[1], "shared store");
    else throw new Error("Invalid memory owner namespace");
    const spaces = this.state.owners.filter(
      /** Selects the persisted exact owner key. */
      space => space.ownerKey === ownerKey,
    );
    if (spaces.length !== 1) throw new Error("Missing or duplicate memory space: " + ownerKey);
    return spaces[0];
  }
  /** Resolves only explicitly connected real external directory readers. */
  private external(): CharacterDirectories {
    if (this.directories === null) throw new Error("External model, TTS and full tool-source directory readers have not been connected");
    return this.directories;
  }
  /** Lists complete stored characters. */
  async listCharacters(): Promise<Card[]> { return copy(this.state.cards); }
  /** Reads one complete stored character. */
  async getCharacter(id: string): Promise<Card> { return copy(get(this.state.cards, id, "character")); }
  /** Removes memory owners whose backing entity no longer exists and returns their pruned owner keys.
    * Orphan owners appear when a character or shared store is lost outside the removal path
    * (for example a deleted-synced card); every scheduler transaction on such an owner throws,
    * so the interval prunes them up front instead of failing forever. */
  async pruneOrphanMemoryOwners(): Promise<string[]> {
    const alive = (ownerKey: string): boolean => {
      const parts = ownerKey.split(":");
      if (parts.length !== 2) return false;
      if (parts[0] === "character") return this.state.cards.some(card => card.id === parts[1]);
      if (parts[0] === "shared") return this.state.stores.some(store => store.id === parts[1]);
      return false;
    };
    const orphans = this.state.owners.filter(space => !alive(space.ownerKey)).map(space => space.ownerKey);
    if (orphans.length === 0) return [];
    this.state.owners = this.state.owners.filter(space => alive(space.ownerKey));
    for (const ownerKey of orphans) this.documents.delete(ownerKey);
    return orphans;
  }
  /** Creates all character fields and its genuine initialized memory space. */
  async createCharacter(card: Card): Promise<Card> {
    const name = requireName(card.name, "character name"); uniqueName(this.state.cards, name, "");
    const now = Date.now(), saved = { ...copy(card), id: uuid(), name, createdAt: now, updatedAt: now };
    this.state.cards.push(saved); this.addSpace("character:" + saved.id); return copy(saved);
  }
  /** Writes the full editable character while preserving its stable creation identity. */
  async updateCharacter(card: Card): Promise<Card> {
    const original = get(this.state.cards, card.id, "character"), name = requireName(card.name, "character name");
    uniqueName(this.state.cards, name, card.id);
    const saved = { ...copy(card), name, createdAt: original.createdAt, updatedAt: Date.now() };
    Object.assign(original, saved); return copy(original);
  }
  /** Removes a non-active character and all of its owned records. */
  async deleteCharacter(id: string): Promise<void> {
    const card = get(this.state.cards, id, "character");
    if ("CharacterCard" in this.state.active && this.state.active.CharacterCard.id === id) throw new Error("Select another character before deleting the active character");
    await this.requireUnboundSelection("card:" + id);
    for (const group of this.state.groups) for (const member of group.members) if (member.characterCardId === id) await this.requireUnboundSelection("group:" + group.id);
    this.state.cards.splice(this.state.cards.indexOf(card), 1); this.documents.delete("character:" + id);
    this.state.owners = this.state.owners.filter(
      /** Removes this exact deleted owner's full space. */
      space => space.ownerKey !== "character:" + id,
    );
    for (const group of this.state.groups) {
      const members = group.members.filter(
        /** Removes the deleted actor from actual group memberships. */
        member => member.characterCardId !== id,
      );
      if (members.length !== group.members.length) { group.members = members; group.updatedAt = Date.now(); }
    }
  }
  /** Archived compatibility data only; never used by sidebar projection or live folder operations. */
  async readConversationGroupsForBackup(): Promise<ConversationGroupRecord[]> { return copy(this.state.conversationGroups); }
  /** Restores an explicit full manual-group backup, preserving every field and identity. */
  async restoreConversationGroups(groups: ConversationGroupRecord[]): Promise<void> {
    assertConversationGroups(groups); this.state.conversationGroups = copy(groups);
  }

  /** Lists complete groups. */
  async listGroups(): Promise<Group[]> { return copy(this.state.groups); }
  /** Reads the exact stored group. */
  async getGroup(id: string): Promise<Group> { return copy(get(this.state.groups, id, "group")); }
  /** Stores a complete newly created group. */
  async createGroup(group: Group): Promise<Group> {
    assertGroup(group); const name = requireName(group.name, "group name"); uniqueName(this.state.groups, name, "");
    const now = Date.now(), saved = { ...copy(group), id: uuid(), name, createdAt: now, updatedAt: now };
    this.state.groups.push(saved); return copy(saved);
  }
  /** Writes every supplied group field. */
  async updateGroup(group: Group): Promise<Group> {
    assertGroup(group); const original = get(this.state.groups, group.id, "group"), name = requireName(group.name, "group name"); uniqueName(this.state.groups, name, group.id);
    Object.assign(original, copy(group), { name, createdAt: original.createdAt, updatedAt: Date.now() }); return copy(original);
  }
  /** Deletes only a group that is not selected as active. */
  async deleteGroup(id: string): Promise<void> {
    const group = get(this.state.groups, id, "group");
    if ("CharacterGroup" in this.state.active && this.state.active.CharacterGroup.id === id) throw new Error("Select another prompt before deleting the active group");
    await this.requireUnboundSelection("group:" + id);
    this.state.groups.splice(this.state.groups.indexOf(group), 1);
  }
  /** Lists full tags. */
  async listTags(): Promise<Tag[]> { return copy(this.state.tags); }
  /** Reads one full tag. */
  async getTag(id: string): Promise<Tag> { return copy(get(this.state.tags, id, "tag")); }
  /** Writes a new tag with all editable values. */
  async createTag(values: TagValues): Promise<Tag> {
    assertTagValues(values); const name = requireName(values.name, "tag name"); uniqueName(this.state.tags, name, "");
    const now = Date.now(), saved = { ...copy(values), name, id: uuid(), createdAt: now, updatedAt: now };
    this.state.tags.push(saved); return copy(saved);
  }
  /** Writes a full existing tag. */
  async updateTag(tag: Tag): Promise<Tag> {
    assertTag(tag); const original = get(this.state.tags, tag.id, "tag"), name = requireName(tag.name, "tag name"); uniqueName(this.state.tags, name, tag.id);
    Object.assign(original, copy(tag), { name, createdAt: original.createdAt, updatedAt: Date.now() }); return copy(original);
  }
  /** Deletes a tag and its attachments within this same private snapshot. */
  async deleteTag(id: string): Promise<void> {
    const tag = get(this.state.tags, id, "tag"); this.state.tags.splice(this.state.tags.indexOf(tag), 1);
    for (const card of this.state.cards) {
      const attached = card.attachedTagIds.filter(
        /** Removes only this exact deleted tag reference. */
        tagId => tagId !== id,
      );
      if (attached.length !== card.attachedTagIds.length) { card.attachedTagIds = attached; card.updatedAt = Date.now(); }
    }
  }
  /** Lists real shared libraries. */
  async listStores(): Promise<Store[]> { return copy(this.state.stores); }
  /** Reads one exact shared library. */
  async getStore(id: string): Promise<Store> { return copy(get(this.state.stores, id, "shared store")); }
  /** Creates the shared library and all of its real initial settings. */
  async createStore(name: string): Promise<Store> {
    name = requireName(name, "library name"); uniqueName(this.state.stores, name, ""); const now = Date.now();
    const store = { id: uuid(), name, createdAt: now, updatedAt: now };
    this.state.stores.push(store); this.addSpace("shared:" + store.id); return copy(store);
  }
  /** Renames one stored shared library. */
  async renameStore(id: string, name: string): Promise<Store> {
    const store = get(this.state.stores, id, "shared store"); name = requireName(name, "library name"); uniqueName(this.state.stores, name, id);
    store.name = name; store.updatedAt = Date.now(); return copy(store);
  }
  /** Deletes the library and explicitly detaches all existing bindings and mounts. */
  async deleteStore(id: string): Promise<void> {
    const store = get(this.state.stores, id, "shared store"); this.state.stores.splice(this.state.stores.indexOf(store), 1); this.documents.delete("shared:" + id);
    this.state.owners = this.state.owners.filter(
      /** Removes the deleted library's owned data, not another namespace. */
      space => space.ownerKey !== "shared:" + id,
    );
    for (const card of this.state.cards) {
      let changed = false;
      if (card.sharedMemoryId === id) { card.sharedMemoryId = null; card.memoryBindingMode = "CHARACTER"; changed = true; }
      const mounts = card.sharedMemoryMounts.filter(
        /** Detaches only this deleted library. */
        mount => mount.sharedMemoryId !== id,
      );
      if (mounts.length !== card.sharedMemoryMounts.length) { card.sharedMemoryMounts = mounts; changed = true; }
      if (changed) card.updatedAt = Date.now();
    }
  }
  /** Delegates only to a genuinely supplied external model reader. */
  listModels(): Promise<ModelSummary[]> { return this.external().listModels(); }
  /** Delegates only to a genuinely supplied external speech reader. */
  listTtsConfigs(): Promise<TtsConfig[]> { return this.external().listTtsConfigs(); }
  /** Delegates only to a genuinely supplied complete tool-source reader. */
  readToolCatalog(): Promise<ToolCatalog> { return this.external().readToolCatalog(); }
  /** Reads the exact active selection. */
  async readActive(): Promise<ActivePrompt> { return copy(this.state.active); }
  /** Writes an explicitly selected existing active entity. */
  async writeActive(active: ActivePrompt): Promise<void> {
    if ("CharacterCard" in active) get(this.state.cards, active.CharacterCard.id, "character");
    else if ("CharacterGroup" in active) get(this.state.groups, active.CharacterGroup.id, "group");
    else if (active.Model.id !== "chat") throw new Error("Invalid model selection");
    this.state.active = copy(active);
  }
  /** Rejects deletion while any stored chat binding references the selection. */
  private async requireUnboundSelection(selection: string): Promise<void> {
    const binding = (await this.chatBindings.listChatBindings()).find(
      /** Matches the exact selection proposed for deletion. */
      binding => binding.selection === selection,
    );
    if (binding !== undefined) throw new Error("Rebind or delete the chat extension before deleting its selection: " + binding.chatId);
  }
  /** Reads the actual owner USER.md file or the edit staged within this same operation. */
  async readUser(ownerKey: string): Promise<UserDocument> {
    const path = await this.readUserPath(ownerKey), document = this.documents.get(ownerKey);
    if (document !== undefined) return { ownerKey, content: document.content };
    const result = await Tools.Files.read(path); assertString(result.content, "USER.md content");
    return { ownerKey, content: result.content };
  }
  /** Requires the existing document before staging a save; missing files are never repaired. */
  async writeUser(ownerKey: string, content: string): Promise<void> {
    assertString(content, "USER.md"); await this.readUser(ownerKey);
    const document = this.documents.get(ownerKey);
    this.documents.set(ownerKey, { ownerKey, path: await this.readUserPath(ownerKey), content, newDocument: document !== undefined && document.newDocument });
  }
  /** Returns a genuine file path accepted by Tools.Files.read/write. */
  async readUserPath(ownerKey: string): Promise<string> { return this.directory + "/" + this.space(ownerKey).userDocumentPath; }
  /** Lists only owners that genuinely exist in this plugin snapshot. */
  async listMemoryOwnerKeys(): Promise<string[]> { return this.state.owners.map(
    /** Preserves each actual initialized namespace. */
    space => space.ownerKey,
  ); }
  /** Creates one device-independent identity for a private persisted job or candidate. */
  async allocateRecordId(): Promise<string> { return uuid(); }
  /** Restores explicit complete character/tag backup records through this snapshot. */
  async restoreCharacters(cards: Card[], tags: Tag[]): Promise<BackupImportResult> {
    const result = { new: 0, updated: 0, skipped: 0, total: cards.length };
    for (const tag of tags) {
      assertTag(tag); const matches = this.state.tags.filter(
        /** Resolves the imported tag by its full stable identity. */
        existing => existing.id === tag.id,
      );
      if (matches.length === 0) this.state.tags.push(copy(tag)); else Object.assign(matches[0], copy(tag));
    }
    for (const card of cards) {
      requireId(card.id, "backup character id"); const matches = this.state.cards.filter(
        /** Resolves the imported character by exact identity. */
        existing => existing.id === card.id,
      );
      if (matches.length === 0) { this.state.cards.push(copy(card)); this.addSpace("character:" + card.id); result.new += 1; }
      else { Object.assign(matches[0], copy(card)); result.updated += 1; }
    }
    return result;
  }
  /** Restores full group backups without creating missing character references. */
  async restoreGroups(groups: Group[]): Promise<BackupImportResult> {
    const result = { new: 0, updated: 0, skipped: 0, total: groups.length };
    for (const group of groups) {
      assertGroup(group); const matches = this.state.groups.filter(
        /** Resolves each imported group by exact stable identity. */
        existing => existing.id === group.id,
      );
      if (matches.length === 0) { this.state.groups.push(copy(group)); result.new += 1; } else { Object.assign(matches[0], copy(group)); result.updated += 1; }
    }
    return result;
  }

}

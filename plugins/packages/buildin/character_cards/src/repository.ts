import type { CharacterRepository } from "./canonical";
import type { ActivePrompt, BackupImportResult, Card, ChatBinding, CharacterDirectories, CharacterState, ConversationGroupChanges, ConversationGroupCreate, ConversationGroupDeletion, ConversationGroupMoveResult, ConversationGroupRecord, Group, Memory, MemoryAutoSaveStatus, MemoryGraph, MemoryImportResult, MemoryLink, MemoryLinkUpdate, MemoryLinkValues, MemoryRebuildProgress, MemorySearchConfig, MemorySearchOptions, MemorySettings, MemorySpace, MemoryValues, ModelSummary, Store, Tag, TagValues, ToolCatalog, TtsConfig, UserDocument, UserDocumentWrite } from "./model";
import { assertBoolean, assertCard, assertConversationGroupChanges, assertConversationGroups, assertConversationGroupScope, assertGroup, assertMemory, assertMemorySettings, assertMemoryValues, assertNumber, assertSearchConfig, assertSearchOptions, assertString, assertTag, assertTagValues, normalizeNames, requireDecimal, requireId, requireName } from "./validation";
import { createDefaultCharacter, createMemorySpace, assertMemorySpace } from "./storage/state";
import { searchMemorySpace } from "./memory-search";
import { requireChatSelection } from "./chat-bindings";
import { deleteChatExtensionBinding, listChatExtensionBindings, readChatExtensionBinding, writeChatExtensionBinding } from "./chat-extensions";

/** Copies JSON domain records so consumers cannot mutate the in-flight snapshot outside this repository. */
function copy<T>(value: T): T { return JSON.parse(JSON.stringify(value)); }
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
/** Normalizes an intentionally supplied folder path without consulting platform-specific code. */
function folder(value: string | null): string | null {
  if (value === null) return null;
  const parts = value.replace(/\\/g, "/").split("/").map(
    /** Trims individual folder names using the plugin's slash-delimited format. */
    part => part.trim(),
  ).filter(
    /** Removes separators, not missing persisted data. */
    part => part !== "",
  );
  for (const part of parts) if (part === "." || part === "..") throw new Error("Relative folder traversal is not allowed");
  return parts.length === 0 ? null : parts.join("/");
}
/** Creates a UUID for a new memory and never changes an existing memory UUID. */
function uuid(): string {
  const bytes: number[] = [];
  for (let index = 0; index < 16; index += 1) bytes.push(Math.floor(Math.random() * 256));
  bytes[6] = (bytes[6] & 15) | 64; bytes[8] = (bytes[8] & 63) | 128;
  const text = bytes.map(
    /** Encodes every byte as two hexadecimal digits. */
    byte => byte.toString(16).padStart(2, "0"),
  ).join("");
  return text.slice(0, 8) + "-" + text.slice(8, 12) + "-" + text.slice(12, 16) + "-" + text.slice(16, 20) + "-" + text.slice(20);
}
/** Implements domain operations on one private snapshot; no method writes Core or a second database. */
export class RepositorySession implements CharacterRepository {
  private readonly documents = new Map<string, UserDocumentWrite>();

  /** Receives the operation snapshot and explicitly connected external directory readers. */
  constructor(private readonly state: CharacterState, private readonly directory: string, private readonly directories: CharacterDirectories | null = null) {}
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
  /** Allocates lossless plugin-local decimal identities without converting them into Number. */
  private allocate(): string {
    const id = requireDecimal(this.state.nextId, "nextId", true), next = BigInt(id) + 1n;
    if (next > 9223372036854775807n) throw new Error("Plugin identity space is exhausted");
    this.state.nextId = next.toString(); return id;
  }
  /** Advances the counter for an explicitly imported full identity. */
  private reserve(id: string): void {
    const value = BigInt(requireDecimal(id, "imported id", true));
    if (value >= BigInt(this.state.nextId)) {
      if (value === 9223372036854775807n) throw new Error("Imported identity exhausts the plugin identity space");
      this.state.nextId = (value + 1n).toString();
    }
  }
  /** Reserves numeric suffixes from this plugin's explicit entity identity format. */
  private reserveEntity(id: string): void {
    const match = /^(character|group|tag|shared)-([1-9][0-9]*)$/.exec(id);
    if (match !== null) this.reserve(match[2]);
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
  /** Creates all character fields and its genuine initialized memory space. */
  async createCharacter(card: Card): Promise<Card> {
    assertCard(card); const name = requireName(card.name, "character name"); uniqueName(this.state.cards, name, "");
    if (card.isDefault) throw new Error("Only the installed product default can be a default character");
    const now = Date.now(), saved = { ...copy(card), id: "character-" + this.allocate(), name, createdAt: now, updatedAt: now };
    this.state.cards.push(saved); this.addSpace("character:" + saved.id); return copy(saved);
  }
  /** Writes the full editable character while preserving its stable creation identity. */
  async updateCharacter(card: Card): Promise<Card> {
    assertCard(card); const original = get(this.state.cards, card.id, "character"), name = requireName(card.name, "character name");
    uniqueName(this.state.cards, name, card.id);
    const saved = { ...copy(card), name, isDefault: original.isDefault, createdAt: original.createdAt, updatedAt: Date.now() };
    Object.assign(original, saved); return copy(original);
  }
  /** Removes a non-active non-default character and all of its owned records. */
  async deleteCharacter(id: string): Promise<void> {
    const card = get(this.state.cards, id, "character");
    if (card.isDefault) throw new Error("The default character cannot be deleted");
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
  /** Performs the explicit reset command against the genuine product default. */
  async resetDefaultCharacter(): Promise<Card> {
    const defaults = this.state.cards.filter(
      /** Identifies the one persisted default character. */
      card => card.isDefault,
    );
    if (defaults.length !== 1) throw new Error("The product default character is missing or ambiguous");
    const original = defaults[0], reset = createDefaultCharacter(Date.now());
    reset.id = original.id; reset.createdAt = original.createdAt; Object.assign(original, reset); return copy(original);
  }
  /** Lists full manual metadata in the exact supplied scope and persisted display order. */
  async listConversationGroups(ownerSelection: string | null): Promise<ConversationGroupRecord[]> {
    assertConversationGroupScope(ownerSelection);
    const groups = this.state.conversationGroups.filter(
      /** Uses opaque scope equality without reading Core role, workspace or group fields. */
      group => group.ownerSelection === ownerSelection,
    ).sort(
      /** Preserves the complete persisted scoped order. */
      (left, right) => left.displayOrder - right.displayOrder,
    );
    return copy(groups);
  }
  /** Reads every scoped manual record without dropping memberships or replacing opaque tokens. */
  async readConversationGroupsForBackup(): Promise<ConversationGroupRecord[]> { return copy(this.state.conversationGroups); }
  /** Creates a real empty group with explicit scope, pin state, identity and timestamps. */
  async createConversationGroup(values: ConversationGroupCreate): Promise<ConversationGroupRecord> {
    assertConversationGroupScope(values.ownerSelection); assertBoolean(values.pinned, "conversation group pinned");
    const name = requireName(values.name, "conversation group name"), groups = await this.listConversationGroups(values.ownerSelection);
    uniqueName(groups, name, ""); const now = Date.now();
    const created: ConversationGroupRecord = { id: this.allocate(), ownerSelection: values.ownerSelection, name, chatIds: [], displayOrder: groups.length, pinned: values.pinned, createdAt: now, updatedAt: now };
    this.state.conversationGroups.push(created); return copy(created);
  }
  /** Applies only explicit editable metadata while keeping complete membership and creation fields. */
  async updateConversationGroup(id: string, changes: ConversationGroupChanges): Promise<ConversationGroupRecord> {
    assertConversationGroupChanges(changes); const original = get(this.state.conversationGroups, requireDecimal(id, "conversation group id", true), "conversation group");
    const updated = { ...copy(original), ...copy(changes) }; updated.name = requireName(updated.name, "conversation group name");
    uniqueName(await this.listConversationGroups(original.ownerSelection), updated.name, original.id); updated.updatedAt = Date.now();
    Object.assign(original, updated); return copy(original);
  }
  /** Removes only this group's metadata, releases its chats and compacts its own scoped order. */
  async deleteConversationGroup(id: string): Promise<ConversationGroupDeletion> {
    const original = get(this.state.conversationGroups, requireDecimal(id, "conversation group id", true), "conversation group");
    const result: ConversationGroupDeletion = { id, deleted: true, releasedChatIds: copy(original.chatIds) };
    this.state.conversationGroups.splice(this.state.conversationGroups.indexOf(original), 1);
    const groups = await this.listConversationGroups(original.ownerSelection), now = Date.now();
    for (let index = 0; index < groups.length; index += 1) {
      const record = get(this.state.conversationGroups, groups[index].id, "conversation group");
      if (record.displayOrder !== index) { record.displayOrder = index; record.updatedAt = now; }
    }
    return result;
  }
  /** Transfers one membership atomically across the private snapshot without changing any chat binding. */
  async moveConversationGroupChat(chatId: string, groupId: string | null, ownerSelection: string | null): Promise<ConversationGroupMoveResult> {
    requireId(chatId, "conversation chat id"); assertConversationGroupScope(ownerSelection);
    const previous = this.state.conversationGroups.filter(
      /** Locates only the genuine stored membership of this exact chat. */
      group => group.chatIds.indexOf(chatId) !== -1,
    );
    if (previous.length > 1) throw new Error("Chat belongs to multiple manual groups: " + chatId);
    const target = groupId === null ? null : get(this.state.conversationGroups, requireDecimal(groupId, "conversation group id", true), "conversation group");
    if (target !== null && target.ownerSelection !== ownerSelection) throw new Error("Target conversation group has a different scope");
    if (target === null && previous.length === 1 && previous[0].ownerSelection !== ownerSelection) throw new Error("Cannot unassign a conversation group from another scope");
    const previousGroupId = previous.length === 0 ? null : previous[0].id;
    if (previousGroupId === groupId) return { chatId, previousGroupId, groupId };
    const now = Date.now();
    if (previous.length === 1) { previous[0].chatIds.splice(previous[0].chatIds.indexOf(chatId), 1); previous[0].updatedAt = now; }
    if (target !== null) { target.chatIds.push(chatId); target.updatedAt = now; }
    return { chatId, previousGroupId, groupId };
  }
  /** Requires the caller's complete scoped permutation before changing any persisted ordering. */
  async reorderConversationGroups(ownerSelection: string | null, ids: string[]): Promise<ConversationGroupRecord[]> {
    assertConversationGroupScope(ownerSelection);
    const groups = await this.listConversationGroups(ownerSelection), supplied = new Set<string>();
    if (ids.length !== groups.length) throw new Error("Reorder requires all conversation groups in the requested scope");
    for (const id of ids) {
      requireDecimal(id, "conversation group id", true);
      if (supplied.has(id)) throw new Error("Reorder contains a duplicate conversation group"); supplied.add(id);
      const group = get(this.state.conversationGroups, id, "conversation group");
      if (group.ownerSelection !== ownerSelection) throw new Error("Reorder contains a conversation group from another scope");
    }
    const now = Date.now();
    for (let index = 0; index < ids.length; index += 1) {
      const group = get(this.state.conversationGroups, ids[index], "conversation group");
      if (group.displayOrder !== index) { group.displayOrder = index; group.updatedAt = now; }
    }
    return this.listConversationGroups(ownerSelection);
  }
  /** Restores an explicit full manual-group backup, preserving every field and reserving lossless IDs. */
  async restoreConversationGroups(groups: ConversationGroupRecord[]): Promise<void> {
    assertConversationGroups(groups); for (const group of groups) this.reserve(group.id); this.state.conversationGroups = copy(groups);
  }

  /** Lists complete groups. */
  async listGroups(): Promise<Group[]> { return copy(this.state.groups); }
  /** Reads the exact stored group. */
  async getGroup(id: string): Promise<Group> { return copy(get(this.state.groups, id, "group")); }
  /** Stores a complete newly created group. */
  async createGroup(group: Group): Promise<Group> {
    assertGroup(group); const name = requireName(group.name, "group name"); uniqueName(this.state.groups, name, "");
    const now = Date.now(), saved = { ...copy(group), id: "group-" + this.allocate(), name, createdAt: now, updatedAt: now };
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
    const now = Date.now(), saved = { ...copy(values), name, id: "tag-" + this.allocate(), createdAt: now, updatedAt: now };
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
    const store = { id: "shared-" + this.allocate(), name, createdAt: now, updatedAt: now };
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
    else get(this.state.groups, active.CharacterGroup.id, "group");
    this.state.active = copy(active);
  }
  /** Rejects deletion using only selections on existing authoritative host records. */
  private async requireUnboundSelection(selection: string): Promise<void> {
    for (const binding of await this.listChatBindings()) if (binding.selection === selection) throw new Error("Rebind or delete the chat extension before deleting its selection: " + binding.chatId);
  }
  /** Enumerates only real host records and their authenticated plugin extension markers. */
  async listChatBindings(): Promise<ChatBinding[]> { return listChatExtensionBindings(); }
  /** Reads the sole record-extension source and validates its actual stored domain reference. */
  async readChatBinding(chatId: string): Promise<ChatBinding> {
    const binding = await readChatExtensionBinding(chatId);
    requireChatSelection(binding.selection, this.state.cards, this.state.groups); return binding;
  }
  /** Writes an explicit selector submission solely to the authenticated host record namespace. */
  async writeChatBinding(binding: ChatBinding): Promise<ChatBinding> {
    requireChatSelection(binding.selection, this.state.cards, this.state.groups);
    return writeChatExtensionBinding(binding);
  }
  /** Deletes only the authenticated record namespace without touching manual membership or file state. */
  async deleteChatBinding(chatId: string): Promise<{ chatId: string; deleted: boolean }> { return deleteChatExtensionBinding(chatId); }
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
  /** Allocates a lossless identity for a private persisted job or candidate. */
  async allocateRecordId(): Promise<string> { return this.allocate(); }
  /** Stores a complete private owner record without shadowing its USER.md content. */
  async writeMemorySpace(ownerKey: string, space: MemorySpace): Promise<void> {
    const original = this.space(ownerKey); assertMemorySpace(space);
    if (space.ownerKey !== ownerKey || space.userDocumentPath !== original.userDocumentPath) throw new Error("Memory owner identity or USER.md path cannot change");
    for (const records of [space.memories, space.links, space.chunks, space.candidates]) for (const record of records) this.reserve(record.id);
    if (space.rebuildTask !== null) this.reserve(space.rebuildTask.id);
    Object.assign(original, copy(space));
  }
  /** Reads the complete owner space for full backups. */
  async readMemorySpace(ownerKey: string): Promise<MemorySpace> { return copy(this.space(ownerKey)); }
  /** Lists full memory records, including every stored document node. */
  async listMemories(ownerKey: string): Promise<Memory[]> { return copy(this.space(ownerKey).memories); }
  /** Lists root and nested folders from actual full records. */
  async listMemoryFolders(ownerKey: string): Promise<string[]> {
    const folders = new Set<string>();
    for (const memory of this.space(ownerKey).memories) {
      if (memory.folderPath === null) { folders.add(""); continue; }
      const parts = memory.folderPath.split("/");
      for (let length = 1; length <= parts.length; length += 1) folders.add(parts.slice(0, length).join("/"));
    }
    return [...folders].sort();
  }
  /** Builds a faithful UUID graph from full stored records without dropping malformed links. */
  async readMemoryGraph(ownerKey: string): Promise<MemoryGraph> {
    const space = this.space(ownerKey), graph: MemoryGraph = { nodes: [], edges: [] };
    for (const memory of space.memories) {
      let color = 0xFFD3D3D3;
      if (memory.isDocumentNode) color = 0xFF9575CD;
      else if (memory.tags.length > 0 && memory.tags[0].name === "Person") color = 0xFF81C784;
      else if (memory.tags.length > 0 && memory.tags[0].name === "Concept") color = 0xFF64B5F6;
      graph.nodes.push({ id: memory.uuid, label: memory.title, color, metadata: { memoryId: memory.id } });
    }
    for (const link of space.links) {
      const source = get(space.memories, link.sourceMemoryId, "source memory"), target = get(space.memories, link.targetMemoryId, "target memory");
      graph.edges.push({ id: link.id, sourceId: source.uuid, targetId: target.uuid, label: link.type_, weight: link.weight, metadata: { description: link.description }, isCrossFolderLink: source.folderPath !== target.folderPath });
    }
    return graph;
  }
  /** Searches real full records using the local lexical/tag/graph algorithm. */
  async searchMemories(options: MemorySearchOptions): Promise<Memory[]> { assertSearchOptions(options); return copy(await searchMemorySpace(this.space(options.ownerKey), options)); }
  /** Creates an intentional new non-document memory. */
  async createMemory(ownerKey: string, values: MemoryValues): Promise<Memory> {
    assertMemoryValues(values); const now = Date.now();
    return this.saveMemory(ownerKey, { ...values, id: "", uuid: "", title: requireName(values.title, "memory title"), folderPath: folder(values.folderPath), documentPath: null, isDocumentNode: false, chunkIndexFilePath: null, createdAt: now, updatedAt: now, lastAccessedAt: now, tags: normalizeNames(values.tags).map(
      /** Initializes explicit new tag records using decimal identities. */
      (name, index) => ({ id: String(index + 1), name }),
    ), properties: [] });
  }
  /** Saves the entire memory record and preserves all fields not explicitly changed. */
  async saveMemory(ownerKey: string, memory: Memory): Promise<Memory> {
    const space = this.space(ownerKey), now = Date.now(), incoming = copy(memory);
    incoming.title = requireName(incoming.title, "memory title"); incoming.folderPath = folder(incoming.folderPath);
    if (incoming.id === "") {
      incoming.id = this.allocate(); if (incoming.uuid === "") incoming.uuid = uuid();
      incoming.createdAt = now; incoming.updatedAt = now; incoming.lastAccessedAt = now;
      assertMemory(incoming); uniqueName(space.memories.map(
        /** Checks title uniqueness for explicit creation. */
        item => ({ id: item.id, name: item.title }),
      ), incoming.title, incoming.id);
      for (const item of space.memories) if (item.uuid === incoming.uuid) throw new Error("Memory UUID already exists");
      space.memories.push(incoming); return copy(incoming);
    }
    requireDecimal(incoming.id, "memory id", true); const original = get(space.memories, incoming.id, "memory");
    if (incoming.uuid !== original.uuid) throw new Error("An existing memory UUID is immutable");
    incoming.createdAt = original.createdAt; incoming.updatedAt = now; assertMemory(incoming);
    uniqueName(space.memories.map(
      /** Checks title uniqueness while retaining the current memory identity. */
      item => ({ id: item.id, name: item.title }),
    ), incoming.title, incoming.id);
    Object.assign(original, incoming); return copy(original);
  }
  /** Updates editable fields without projecting away document metadata or properties. */
  async updateMemory(ownerKey: string, id: string, values: MemoryValues): Promise<Memory> {
    assertMemoryValues(values); const original = get(this.space(ownerKey).memories, id, "memory");
    return this.saveMemory(ownerKey, { ...copy(original), ...values, tags: normalizeNames(values.tags).map(
      /** Allocates the explicit replacement tag list without changing the memory identity. */
      (name, index) => ({ id: String(index + 1), name }),
    ) });
  }
  /** Deletes a memory and all referencing chunks and links in this operation. */
  async deleteMemory(ownerKey: string, id: string): Promise<void> {
    requireDecimal(id, "memory id", true); const space = this.space(ownerKey), memory = get(space.memories, id, "memory");
    space.memories.splice(space.memories.indexOf(memory), 1);
    space.chunks = space.chunks.filter(
      /** Removes the deleted document's exact UUID chunks. */
      chunk => chunk.memoryUuid !== memory.uuid,
    );
    space.links = space.links.filter(
      /** Removes links with either exact deleted endpoint. */
      link => link.sourceMemoryId !== id && link.targetMemoryId !== id,
    );
  }
  /** Moves only explicitly selected existing memories. */
  async moveMemories(ownerKey: string, ids: string[], folderPath: string): Promise<void> {
    const space = this.space(ownerKey), destination = folder(folderPath), selected = new Set(ids);
    for (const id of selected) { requireDecimal(id, "memory id", true); const memory = get(space.memories, id, "memory"); memory.folderPath = destination; memory.updatedAt = Date.now(); }
  }
  /** Lists full relationships including descriptions. */
  async listMemoryLinks(ownerKey: string): Promise<MemoryLink[]> { return copy(this.space(ownerKey).links); }
  /** Reads one exact full relationship. */
  async readMemoryLink(ownerKey: string, id: string): Promise<MemoryLink> { requireDecimal(id, "link id", true); return copy(get(this.space(ownerKey).links, id, "link")); }
  /** Creates a relationship only between existing distinct owner-scoped memories. */
  async createLink(ownerKey: string, values: MemoryLinkValues): Promise<MemoryLink> {
    const space = this.space(ownerKey); get(space.memories, values.sourceMemoryId, "source memory"); get(space.memories, values.targetMemoryId, "target memory");
    if (values.sourceMemoryId === values.targetMemoryId) throw new Error("Self relationships are not allowed");
    assertNumber(values.weight, "link weight", 0, 1); assertString(values.description, "link description"); const type_ = requireName(values.type_, "link type");
    const saved = { ...copy(values), type_, id: this.allocate() }; space.links.push(saved); return copy(saved);
  }
  /** Writes every editable field of one exact relationship. */
  async updateLink(ownerKey: string, id: string, values: MemoryLinkUpdate): Promise<MemoryLink> {
    const link = get(this.space(ownerKey).links, requireDecimal(id, "link id", true), "link");
    assertNumber(values.weight, "link weight", 0, 1); assertString(values.description, "link description"); Object.assign(link, copy(values), { type_: requireName(values.type_, "link type") }); return copy(link);
  }
  /** Removes one exact relationship. */
  async deleteLink(ownerKey: string, id: string): Promise<void> { const links = this.space(ownerKey).links, link = get(links, requireDecimal(id, "link id", true), "link"); links.splice(links.indexOf(link), 1); }
  /** Reads complete persisted memory settings. */
  async readMemorySettings(ownerKey: string): Promise<MemorySettings> { return copy(this.space(ownerKey).settings); }
  /** Writes complete memory settings without silently activating unsupported embedding. */
  async writeMemorySettings(ownerKey: string, settings: MemorySettings): Promise<void> { assertMemorySettings(settings); this.space(ownerKey).settings = copy(settings); }
  /** Reads complete persisted search configuration. */
  async readMemorySearchConfig(ownerKey: string): Promise<MemorySearchConfig> { return copy(this.space(ownerKey).searchConfig); }
  /** Writes all explicitly selected search weights. */
  async writeMemorySearchConfig(ownerKey: string, config: MemorySearchConfig): Promise<void> { assertSearchConfig(config); this.space(ownerKey).searchConfig = copy(config); }
  /** Counts genuine candidate metadata without pretending a scheduler is running. */
  async readMemoryAutoSaveStatus(ownerKey: string): Promise<MemoryAutoSaveStatus> {
    const space = this.space(ownerKey), chats = new Set<string>(); let pendingCandidates = 0, processingCandidates = 0, failedCandidates = 0, lastError = "";
    for (const candidate of space.candidates) {
      if (candidate.status === "pending") { pendingCandidates += 1; chats.add(candidate.chatId); }
      else if (candidate.status === "processing") processingCandidates += 1;
      else if (candidate.status === "failed") { failedCandidates += 1; lastError = candidate.lastError; }
    }
    const nextRunAtMs = space.settings.nextAutoSaveRunAtMs;
    return { ownerKey, pendingCandidates, pendingChats: chats.size, processingCandidates, failedCandidates, nextRunAtMs, minutesUntilNextRun: Math.max(0, Math.ceil((nextRunAtMs - Date.now()) / 60000)), lastError };
  }
  /** Reads actual persisted progress; the new-install idle state is not a fake running job. */
  async readMemoryRebuildProgress(ownerKey: string): Promise<MemoryRebuildProgress> { return copy(this.space(ownerKey).rebuildProgress); }
  /** Cancels only an actual active job record. */
  async cancelMemoryRebuild(ownerKey: string): Promise<void> {
    const progress = this.space(ownerKey).rebuildProgress;
    if (progress.status !== "running" && progress.status !== "preparing") throw new Error("No memory rebuild is active"); progress.status = "cancelled";
  }
  /** Restores explicit complete character/tag backup records through this snapshot. */
  async restoreCharacters(cards: Card[], tags: Tag[]): Promise<BackupImportResult> {
    const result = { new: 0, updated: 0, skipped: 0, total: cards.length };
    for (const tag of tags) {
      assertTag(tag); this.reserveEntity(tag.id); const matches = this.state.tags.filter(
        /** Resolves the imported tag by its full stable identity. */
        existing => existing.id === tag.id,
      );
      if (matches.length === 0) this.state.tags.push(copy(tag)); else Object.assign(matches[0], copy(tag));
    }
    for (const card of cards) {
      assertCard(card); requireId(card.id, "backup character id"); this.reserveEntity(card.id); const matches = this.state.cards.filter(
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
      assertGroup(group); this.reserveEntity(group.id); const matches = this.state.groups.filter(
        /** Resolves each imported group by exact stable identity. */
        existing => existing.id === group.id,
      );
      if (matches.length === 0) { this.state.groups.push(copy(group)); result.new += 1; } else { Object.assign(matches[0], copy(group)); result.updated += 1; }
    }
    return result;
  }
  /** Imports complete document-aware records, relationships, settings and USER text in this operation. */
  async importMemorySpace(ownerKey: string, imported: MemorySpace, strategy: "SKIP" | "UPDATE" | "CREATE_NEW"): Promise<MemoryImportResult> {
    assertMemorySpace(imported);
    const space = this.space(ownerKey), result = { newMemories: 0, updatedMemories: 0, skippedMemories: 0, newLinks: 0 };
    const idMapping = new Map<string, string>();
    for (const memory of imported.memories) {
      const existing = space.memories.find(
        /** Matches the import's actual UUID, not a guessed title. */
        item => item.uuid === memory.uuid,
      );
      if (existing !== undefined && strategy === "SKIP") { result.skippedMemories += 1; idMapping.set(memory.id, existing.id); continue; }
      const value = copy(memory);
      if (existing !== undefined && strategy === "UPDATE") { value.id = existing.id; Object.assign(existing, value); result.updatedMemories += 1; }
      else {
        if (strategy === "CREATE_NEW") { value.id = this.allocate(); value.uuid = uuid(); }
        else {
          if (space.memories.some(
            /** Rejects an occupied imported identity instead of remapping it silently. */
            item => item.id === value.id,
          )) throw new Error("Imported memory id is occupied: " + value.id);
          this.reserve(value.id);
        }
        space.memories.push(value); result.newMemories += 1;
      }
      idMapping.set(memory.id, value.id);
      space.chunks = space.chunks.filter(
        /** Replaces only the explicit updated document's prior chunks. */
        chunk => chunk.memoryUuid !== value.uuid,
      );
      for (const chunk of imported.chunks) if (chunk.memoryUuid === memory.uuid) {
        const incoming = { ...copy(chunk), memoryUuid: value.uuid, id: strategy === "CREATE_NEW" ? this.allocate() : chunk.id };
        if (space.chunks.some(
          /** Rejects duplicate chunk identities in the target space. */
          record => record.id === incoming.id,
        )) throw new Error("Imported chunk id is occupied: " + incoming.id);
        this.reserve(incoming.id); space.chunks.push(incoming);
      }
    }
    for (const link of imported.links) {
      const source = idMapping.get(link.sourceMemoryId), target = idMapping.get(link.targetMemoryId);
      if (source === undefined || target === undefined) throw new Error("Imported link has no mapped endpoint");
      const existing = space.links.find(
        /** Resolves only a matching relationship identity in non-copy imports. */
        record => record.id === link.id,
      );
      if (existing !== undefined && strategy === "SKIP") continue;
      const incoming = { ...copy(link), sourceMemoryId: source, targetMemoryId: target, id: strategy === "CREATE_NEW" ? this.allocate() : link.id };
      this.reserve(incoming.id);
      if (existing !== undefined && strategy === "UPDATE") Object.assign(existing, incoming);
      else { space.links.push(incoming); result.newLinks += 1; }
    }
    space.settings = copy(imported.settings); space.searchConfig = copy(imported.searchConfig);
    space.rebuildProgress = copy(imported.rebuildProgress); space.rebuildTask = copy(imported.rebuildTask); space.embeddings = copy(imported.embeddings);
    if (space.rebuildTask !== null) this.reserve(space.rebuildTask.id);
    for (const candidate of imported.candidates) {
      const incoming = { ...copy(candidate), id: strategy === "CREATE_NEW" ? this.allocate() : candidate.id };
      this.reserve(incoming.id);
      const existing = space.candidates.find(
        /** Resolves a candidate's actual persistent identity. */
        record => record.id === incoming.id,
      );
      if (existing === undefined) space.candidates.push(incoming);
      else if (strategy === "UPDATE") Object.assign(existing, incoming);
    }
    return result;
  }
}

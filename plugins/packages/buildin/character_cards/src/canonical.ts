import type {
  ActivePrompt, Card, ChatBinding, ConversationGroupChanges, ConversationGroupCreate, ConversationGroupDeletion, ConversationGroupMoveResult, ConversationGroupRecord, Group, Memory, MemoryGraph, MemoryLink, MemoryLinkUpdate, MemoryLinkValues,
  MemorySearchConfig, MemorySearchOptions, MemorySettings, MemoryValues, MemoryAutoSaveStatus,
  MemoryRebuildProgress, MemorySpace, ModelSummary, Store, Tag, TagValues, ToolCatalog, TtsConfig, UserDocument,
} from "./model";

/** Defines the plugin's local business repository, not an SDK host or global service. */
export interface CharacterRepository {
  /** Lists ordered full manual groups in exactly the caller's explicit sidebar scope. */
  listConversationGroups(ownerSelection: string | null): Promise<ConversationGroupRecord[]>;
  /** Lists all scoped manual groups only for full lossless backup operations. */
  readConversationGroupsForBackup(): Promise<ConversationGroupRecord[]>;
  /** Creates a genuinely new empty manual group independently from role selections. */
  createConversationGroup(values: ConversationGroupCreate): Promise<ConversationGroupRecord>;
  /** Saves name and pin edits without changing scope or membership. */
  updateConversationGroup(id: string, changes: ConversationGroupChanges): Promise<ConversationGroupRecord>;
  /** Deletes metadata and releases memberships without invoking host chat deletion. */
  deleteConversationGroup(id: string): Promise<ConversationGroupDeletion>;
  /** Moves one actual chat into one scoped manual group or explicitly unassigns it. */
  moveConversationGroupChat(chatId: string, groupId: string | null, ownerSelection: string | null): Promise<ConversationGroupMoveResult>;
  /** Commits a complete permutation of one scope without affecting other scopes. */
  reorderConversationGroups(ownerSelection: string | null, ids: string[]): Promise<ConversationGroupRecord[]>;
  /** Restores complete scoped records with original identities and timestamps in one operation. */
  restoreConversationGroups(groups: ConversationGroupRecord[]): Promise<void>;
  /** Lists full character records in this operation's snapshot. */
  listCharacters(): Promise<Card[]>;
  /** Requires the exact existing character identity. */
  getCharacter(id: string): Promise<Card>;
  /** Allocates a plugin identity and writes all supplied character fields. */
  createCharacter(card: Card): Promise<Card>;
  /** Writes a complete existing character while retaining its immutable identity. */
  updateCharacter(card: Card): Promise<Card>;
  /** Removes the character and its owned memory space without changing the active actor. */
  deleteCharacter(id: string): Promise<void>;
  /** Restores the explicit product default character, not a recovery record. */
  resetDefaultCharacter(): Promise<Card>;
  /** Lists complete groups. */
  listGroups(): Promise<Group[]>;
  /** Requires an exact existing group. */
  getGroup(id: string): Promise<Group>;
  /** Allocates and stores a complete new group. */
  createGroup(group: Group): Promise<Group>;
  /** Stores all fields of an existing group. */
  updateGroup(group: Group): Promise<Group>;
  /** Deletes a group that is not active. */
  deleteGroup(id: string): Promise<void>;
  /** Lists complete tags. */
  listTags(): Promise<Tag[]>;
  /** Requires an existing tag. */
  getTag(id: string): Promise<Tag>;
  /** Creates a tag with all editable fields. */
  createTag(values: TagValues): Promise<Tag>;
  /** Stores a complete existing tag. */
  updateTag(tag: Tag): Promise<Tag>;
  /** Deletes a tag and its character attachments in the same operation. */
  deleteTag(id: string): Promise<void>;
  /** Lists complete shared memory libraries. */
  listStores(): Promise<Store[]>;
  /** Requires an existing shared memory library. */
  getStore(id: string): Promise<Store>;
  /** Creates a shared library and its complete initialized memory space. */
  createStore(name: string): Promise<Store>;
  /** Renames an existing shared library. */
  renameStore(id: string, name: string): Promise<Store>;
  /** Removes a library, its bindings and its owned records in this operation. */
  deleteStore(id: string): Promise<void>;
  /** Requires a genuinely connected external model directory. */
  listModels(): Promise<ModelSummary[]>;
  /** Requires a genuinely connected external TTS directory. */
  listTtsConfigs(): Promise<TtsConfig[]>;
  /** Requires a complete externally connected tool-source directory. */
  readToolCatalog(): Promise<ToolCatalog>;
  /** Reads the actual active selection. */
  readActive(): Promise<ActivePrompt>;
  /** Selects an existing character or group. */
  writeActive(active: ActivePrompt): Promise<void>;
  /** Reads the authenticated host record namespace, never file or current global selection. */
  readChatBinding(chatId: string): Promise<ChatBinding>;
  /** Enumerates real host conversations and their record-extension selections for background tasks. */
  listChatBindings(): Promise<ChatBinding[]>;
  /** Writes an explicit selection only to the authenticated host record namespace. */
  writeChatBinding(binding: ChatBinding): Promise<ChatBinding>;
  /** Removes the authenticated namespace without deleting chats or file-owned membership. */
  deleteChatBinding(chatId: string): Promise<{ chatId: string; deleted: boolean }>;
  /** Reads the owner's persisted USER.md content. */
  readUser(ownerKey: string): Promise<UserDocument>;
  /** Stages the owner's complete USER.md content in the same authoritative file. */
  writeUser(ownerKey: string, content: string): Promise<void>;
  /** Returns the real owner USER.md path accepted by the generic Files capability. */
  readUserPath(ownerKey: string): Promise<string>;
  /** Lists genuinely initialized owner namespaces for the private job scheduler. */
  listMemoryOwnerKeys(): Promise<string[]>;
  /** Allocates one lossless private candidate or job identity in this snapshot. */
  allocateRecordId(): Promise<string>;
  /** Writes a complete validated owner space without replacing the separate USER.md file. */
  writeMemorySpace(ownerKey: string, space: MemorySpace): Promise<void>;
  /** Reads the complete memory space for full-record exports. */
  readMemorySpace(ownerKey: string): Promise<MemorySpace>;
  /** Imports a complete validated memory space in the same operation. */
  importMemorySpace(ownerKey: string, space: MemorySpace, strategy: "SKIP" | "UPDATE" | "CREATE_NEW"): Promise<import("./model").MemoryImportResult>;
  /** Builds the UUID graph from full memories and relationships. */
  readMemoryGraph(ownerKey: string): Promise<MemoryGraph>;
  /** Lists complete memories including document nodes. */
  listMemories(ownerKey: string): Promise<Memory[]>;
  /** Lists stored folders without graph projection. */
  listMemoryFolders(ownerKey: string): Promise<string[]>;
  /** Searches full records using the plugin's explicit scoring algorithm. */
  searchMemories(options: MemorySearchOptions): Promise<Memory[]>;
  /** Creates an intentional new memory with complete identity and metadata. */
  createMemory(ownerKey: string, values: MemoryValues): Promise<Memory>;
  /** Stores every field of a memory, allocating identity only for an explicit new draft. */
  saveMemory(ownerKey: string, memory: Memory): Promise<Memory>;
  /** Updates editable memory fields while preserving document records. */
  updateMemory(ownerKey: string, id: string, values: MemoryValues): Promise<Memory>;
  /** Deletes a memory together with its chunks and links. */
  deleteMemory(ownerKey: string, id: string): Promise<void>;
  /** Moves selected memories in the same operation. */
  moveMemories(ownerKey: string, ids: string[], folderPath: string): Promise<void>;
  /** Lists complete relationships, including their descriptions. */
  listMemoryLinks(ownerKey: string): Promise<MemoryLink[]>;
  /** Requires one stored relationship. */
  readMemoryLink(ownerKey: string, id: string): Promise<MemoryLink>;
  /** Creates a relationship whose endpoints exist in this memory space. */
  createLink(ownerKey: string, values: MemoryLinkValues): Promise<MemoryLink>;
  /** Updates all editable relationship fields. */
  updateLink(ownerKey: string, id: string, values: MemoryLinkUpdate): Promise<MemoryLink>;
  /** Removes one exact relationship. */
  deleteLink(ownerKey: string, id: string): Promise<void>;
  /** Reads the complete owner settings. */
  readMemorySettings(ownerKey: string): Promise<MemorySettings>;
  /** Writes the complete owner settings. */
  writeMemorySettings(ownerKey: string, settings: MemorySettings): Promise<void>;
  /** Reads all search weights and scoring mode. */
  readMemorySearchConfig(ownerKey: string): Promise<MemorySearchConfig>;
  /** Writes all search weights and scoring mode. */
  writeMemorySearchConfig(ownerKey: string, config: MemorySearchConfig): Promise<void>;
  /** Reads actual candidate counts from this owner snapshot. */
  readMemoryAutoSaveStatus(ownerKey: string): Promise<MemoryAutoSaveStatus>;
  /** Reads the persisted rebuild progress record. */
  readMemoryRebuildProgress(ownerKey: string): Promise<MemoryRebuildProgress>;
  /** Cancels a real active rebuild by recording its cancelled state. */
  cancelMemoryRebuild(ownerKey: string): Promise<void>;
  /** Restores complete backup records through this same operation. */
  restoreCharacters(cards: Card[], tags: Tag[]): Promise<import("./model").BackupImportResult>;
  /** Restores complete group records through this same operation. */
  restoreGroups(groups: Group[]): Promise<import("./model").BackupImportResult>;
}

/** Serializes service operations against the one plugin-owned file repository. */
export interface CharacterRepositoryOwner {
  /** Executes domain edits against a private snapshot and publishes it once on success. */
  run<T>(action: (repository: CharacterRepository) => Promise<T>): Promise<T>;
}

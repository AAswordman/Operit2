// Public client copied by dependency packages; keep this file self-contained.

/** Preserves the host JSON object contract without an external type import. */
export type JsonObject = Record<string, unknown>;
/** Describes transferable JSON values for public result validation. */
export type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };

/** Publishes the complete ConversationGroupRecord contract in the standalone dependency client. */
export interface ConversationGroupRecord {
  id: string;
  ownerSelection: string | null;
  name: string;
  chatIds: string[];
  displayOrder: number;
  pinned: boolean;
  createdAt: number;
  updatedAt: number;
}

/** Publishes the complete MemoryChat contract in the standalone dependency client. */
export interface MemoryChat {
  id: string; title: string; messageCount: number; createdAt: string; updatedAt: string;
  isCurrent: boolean; inputTokens: number; outputTokens: number;
}

/** Publishes the complete MemoryExtractionResult contract in the standalone dependency client. */
export interface MemoryExtractionResult { ownerKey: string; created: number; updated: number; merged: number; links: number; profileUpdated: boolean }

/** Publishes the complete MemoryRebuild contract in the standalone dependency client. */
export interface MemoryRebuild {
  chatIds: string[];
  windowMessageCount: number;
  fromInclusive: number | null;
  toInclusive: number | null;
}

/** Publishes the complete MemoryRebuildProgress contract in the standalone dependency client. */
export interface MemoryRebuildProgress {
  status: string;
  totalChats: number;
  completedChats: number;
  totalWindows: number;
  completedWindows: number;
  totalSourceMessages: number;
  processedSourceMessages: number;
  failedWindows: number;
  currentChatTitle: string;
  lastError: string;
}

/** Publishes the complete MemorySearchOptions contract in the standalone dependency client. */
export interface MemorySearchOptions {
  ownerKey: string;
  query: string;
  folderPath: string | null;
  relevanceThreshold: number;
  createdAtStartMs: number | null;
  createdAtEndMs: number | null;
}

/** Carries explicit application defaults to the plugin's chat configuration resolver. */
export interface ChatModelBinding { providerId: string; modelId: string }
/** Receives only owner-isolated record extensions and explicit generic execution defaults from Core. */
export interface ChatConfigurationRequest {
  purpose: "display" | "execution";
  chatId: string | null;
  chatExtension: JsonObject | null;
  messageExtension: JsonObject | null;
  participantId: string | null;
  promptFunctionType: "CHAT" | "VOICE";
  defaultModelBinding: ChatModelBinding;
  defaultTtsConfigId: string;
}
/** Projects the plugin-owned selection stored only in one actual chat's record extension. */
export interface ChatBindingRecord { chatId: string; selection: string }
/** Describes resource permissions without exposing character or shared-library binding rules. */
export interface ChatResourceRoute { key: string; readable: boolean; writable: boolean }
/** Supplies a complete neutral participant configuration rather than a stored character DTO. */
export interface ChatParticipantProfile {
  id: string;
  name: string;
  avatarUri: string | null;
  introPrompt: string;
  userPreferencesText: string;
  openingStatement: string;
  modelBinding: ChatModelBinding;
  ttsConfigId: string;
  toolAccess: CharacterRecord["toolAccessConfig"];
  resources: ChatResourceRoute[];
}
/** Returns one selected execution profile together with its available participants. */
export interface ChatConfigurationExecutionResult {
  contextKey: string;
  profile: ChatParticipantProfile;
  participants: ChatParticipantProfile[];
  messageExtension: JsonObject;
}

/** Describes presentation identity without resolving a model, speech voice or memory owner. */
export interface ChatDisplayIdentity { title: string; avatarUri: string | null }
/** Preserves the plugin-declared presentation order and real participant identity. */
export interface ChatDisplayParticipant { id: string; name: string; avatarUri: string | null }
/** Supplies an explicitly authored initial message and its complete immutable participant snapshot. */
export interface ChatInitialMessage { content: string; displayName: string; messageExtension: JsonObject }
/** Returns presentation data only; an empty initial-message array does not select an executor. */
export interface ChatConfigurationDisplayResult {
  contextKey: string | null;
  identity: ChatDisplayIdentity | null;
  participants: ChatDisplayParticipant[];
  initialMessages: ChatInitialMessage[];
}
/** Matches the real host purpose-specific results without placing a purpose discriminator in JSON. */
export type ChatConfigurationResult = ChatConfigurationDisplayResult | ChatConfigurationExecutionResult;

/** Defines the complete stored character record without a CLI transport. */
export interface CharacterRecord {
  id: string; name: string; description: string; characterSetting: string; openingStatement: string;
  otherContentChat: string; otherContentVoice: string; avatarUri: string | null; attachedTagIds: string[];
  advancedCustomPrompt: string; marks: string; chatModelBindingMode: string; chatModelId: string | null;
  ttsConfigId: string | null; themeConfigId: string | null; memoryBindingMode: string; sharedMemoryId: string | null;
  sharedMemoryMounts: { sharedMemoryId: string; readable: boolean; writable: boolean }[];
  toolAccessConfig: { enabled: boolean; allowedBuiltinTools: string[]; allowedPackages: string[]; allowedSkills: string[]; allowedMcpServers: string[] };
  isDefault: boolean; createdAt: number; updatedAt: number;
}
/** Defines the complete stored group record. */
export interface GroupRecord { id: string; name: string; description: string; themeConfigId: string | null; members: { characterCardId: string; orderIndex: number }[]; createdAt: number; updatedAt: number }
/** Defines a stored prompt tag. */
export interface TagRecord { id: string; name: string; description: string; promptContent: string; tagType: "TONE" | "CHARACTER" | "FUNCTION" | "CUSTOM"; createdAt: number; updatedAt: number }
/** Defines a shared memory library. */
export interface StoreRecord { id: string; name: string; createdAt: number; updatedAt: number }
/** Defines a complete stored memory item. */
export interface MemoryRecord {
  id: string; uuid: string; title: string; content: string; contentType: string; source: string; credibility: number; importance: number;
  documentPath: string | null; isDocumentNode: boolean; chunkIndexFilePath: string | null; folderPath: string | null;
  createdAt: number; updatedAt: number; lastAccessedAt: number;
  tags: { id: string; name: string }[]; properties: { id: string; key: string; value: string }[];
}
/** Defines a stored graph and its cross-folder edges. */
export interface MemoryGraphRecord {
  nodes: { id: string; label: string; color: number; metadata: Record<string, string> }[];
  edges: { id: string; sourceId: string; targetId: string; label: string | null; weight: number; metadata: Record<string, string>; isCrossFolderLink: boolean }[];
}
/** Defines a stored relationship between memory items. */
export interface LinkRecord { id: string; sourceMemoryId: string; targetMemoryId: string; type_: string; weight: number; description: string }
/** Defines the active prompt's externally tagged serialization. */
export type ActivePromptRecord = { CharacterCard: { id: string } } | { CharacterGroup: { id: string } };
/** Defines the editor's complete directory snapshot. */
export interface SnapshotRecord {
  cards: CharacterRecord[]; groups: GroupRecord[]; stores: StoreRecord[]; tags: TagRecord[];
  models: ModelSummaryRecord[]; ttsConfigs: TtsConfigRecord[]; toolCatalog: ToolCatalogRecord; active: ActivePromptRecord;
}
/** Defines the actual provider model summary. */
export interface ModelSummaryRecord {
  providerId: string; providerName: string; providerTypeId: string; endpoint: string; modelId: string;
  capabilities: { directImage: boolean; directAudio: boolean; directVideo: boolean; toolCall: boolean };
  pricing: { billingMode: "TOKEN" | "COUNT"; inputPricePerMillion: number; cachedInputPricePerMillion: number | null; cacheWritePricePerMillion: number | null; outputPricePerMillion: number; pricePerRequest: number; currency: "CNY" | "USD" } | null;
}
/** Defines a stored speech configuration. */
export interface TtsConfigRecord {
  id: string; name: string; providerType: string; endpoint: string; apiKey: string; model: string; voice: string; responseFormat: string;
  speed: number; httpMethod: string; requestBody: string; contentType: string; headers: { name: string; value: string }[];
  responsePipeline: { stepType: string; path: string; headers: { name: string; value: string }[] }[]; createdAt: number; updatedAt: number;
}
/** Defines the host-owned selectable tool sources. */
export interface ToolCatalogRecord {
  builtinTools: { name: string; displayName: string; description: string }[];
  packages: { name: string; displayName: string; description: string }[];
  skills: { name: string; displayName: string; description: string }[];
  mcpServers: { name: string; displayName: string; description: string }[];
}
/** Defines an owner-scoped memory configuration. */
export interface MemorySettingsRecord {
  autoSaveIntervalMinutes: number; nextAutoSaveRunAtMs: number; memoryExtractionCustomRules: string;
  profileAutoUpdateEnabled: boolean; profileAutoUpdateLocked: boolean; cloudEmbeddingEnabled: boolean;
  cloudEmbeddingEndpoint: string; cloudEmbeddingApiKey: string; cloudEmbeddingModel: string;
}
/** Defines the repository's search scoring configuration. */
export interface MemorySearchConfigRecord { scoreMode: "BALANCED" | "KEYWORD_FIRST" | "SEMANTIC_FIRST"; keywordWeight: number; tagWeight: number; vectorWeight: number; edgeWeight: number }
/** Defines character creation fields; omitted values retain the manager's creation semantics. */
export type CharacterCreate = Pick<CharacterRecord, "name"> & Partial<Omit<CharacterRecord, "name">>;
/** Defines an explicit character patch, including bindings, mounts, and tool permissions. */
export type CharacterChanges = Partial<Omit<CharacterRecord, "id">>;
/** Defines group creation fields. */
export type GroupCreate = Pick<GroupRecord, "name"> & Partial<Omit<GroupRecord, "name">>;
/** Defines an explicit group patch. */
export type GroupChanges = Partial<Omit<GroupRecord, "id">>;
/** Defines prompt tag creation fields. */
export type TagCreate = Pick<TagRecord, "name"> & Partial<Omit<TagRecord, "name">>;
/** Defines an explicit prompt tag patch. */
export type TagChanges = Partial<Omit<TagRecord, "id">>;
/** Defines memory creation fields with explicit tag names and property values. */
export type MemoryCreate = Pick<MemoryRecord, "title" | "content"> & Partial<Omit<MemoryRecord, "title" | "content" | "tags">> & { tags?: string[] };
/** Defines an explicit memory patch, including all stored metadata. */
export type MemoryChanges = Partial<Omit<MemoryRecord, "id" | "tags">> & { tags?: string[] };
/** Defines the supported character interchange formats. */
export type CharacterFormat = "operit" | "tavern";
/** Defines the repository's memory import strategies. */
export type MemoryImportStrategy = "SKIP" | "UPDATE" | "CREATE_NEW";
/** Defines manager backup import counts. */
export interface BackupImportResult { new: number; updated: number; skipped: number; total: number }
/** Defines a repository memory import report. */
export interface MemoryImportResult { newMemories: number; updatedMemories: number; skippedMemories: number; newLinks: number }
/** Defines an activation result without invoking another command. */
export interface ActivationResult { type: "character_card" | "character_group"; id: string; active: boolean }
/** Defines an explicitly supplied memory owner. */
export interface OwnerParams { ownerKey: string }
/** Defines stable identifier parameters. */
export interface IdParams { id: string }
/** Defines empty parameters for methods without inputs. */
export type EmptyParams = Record<string, never>;
/** Selects one participant's memory query and an explicit optional deduplication snapshot. */
export interface MemoryQueryRequest { participantId: string; query: string; limit: number; snapshotId: string | null }
/** Publishes a complete query match, including selected document chunks. */
export interface MemoryQueryMatch { ownerKey: string; title: string; content: string; source: string; tags: string[]; createdAt: string; chunkInfo: string | null; chunkIndices: number[] | null }
/** Reports query matches and owner-scoped deduplication state from the provider runtime. */
export interface MemoryQueryResult { memories: MemoryQueryMatch[]; snapshotId: string | null; snapshotCreated: boolean; excludedBySnapshotCount: number }
/** Maps every published domain method to its exact input and output contract. */
export interface DomainOperations {
  "memory.searchWithOptions": { input: MemorySearchOptions; output: { ownerKey: string; items: MemoryRecord[] } };
  "memory.chat.list": { input: { ownerKey: string }; output: MemoryChat[] };
  "memory.chat.update": { input: { ownerKey: string; chatId: string }; output: MemoryExtractionResult };
  "memory.categorize": { input: { ownerKey: string }; output: number };
  "memory.rebuild.start": { input: { ownerKey: string; rebuild: MemoryRebuild }; output: MemoryRebuildProgress };
  "memory.rebuild.progress": { input: { ownerKey: string }; output: MemoryRebuildProgress };
  "memory.rebuild.cancel": { input: { ownerKey: string }; output: MemoryRebuildProgress };
  "memory.embeddings.rebuild": { input: { ownerKey: string }; output: number };
  "memory.candidate.enqueue": { input: { chatId: string; timestamp: number; variantIndex: number; sourceType: "reply_finalized_auto" | "selected_user_message"; ownerKey: string | null }; output: { owners: string[]; candidateIds: string[] } };
  "chat.configuration.resolve": { input: ChatConfigurationRequest; output: ChatConfigurationResult };
  "chat.configuration.binding.read": { input: { chatId: string }; output: ChatBindingRecord };
  "chat.configuration.binding.write": { input: ChatBindingRecord; output: ChatBindingRecord };
  "chat.configuration.binding.delete": { input: { chatId: string }; output: { chatId: string; deleted: boolean } };
  snapshot: { input: EmptyParams; output: SnapshotRecord };
  "character.list": { input: EmptyParams; output: CharacterRecord[] };
  "character.get": { input: IdParams; output: CharacterRecord };
  "character.create": { input: { values: CharacterCreate }; output: CharacterRecord };
  "character.update": { input: { id: string; changes: CharacterChanges }; output: CharacterRecord };
  "character.delete": { input: IdParams; output: { id: string; deleted: boolean } };
  "character.setActive": { input: IdParams; output: ActivationResult };
  "character.combine": { input: { id: string; promptFunctionType?: "CHAT" | "VOICE"; additionalTagIds?: string[] }; output: { id: string; promptFunctionType: "CHAT" | "VOICE"; additionalTagIds: string[]; prompt: string } };
  "character.resetDefault": { input: EmptyParams; output: { defaultCharacterReset: boolean } };
  "character.export": { input: { id: string; format: CharacterFormat }; output: { id: string; format: CharacterFormat; content: string } };
  "character.import": { input: { format: CharacterFormat; content: string }; output: CharacterRecord };
  "character.exportBackup": { input: EmptyParams; output: { content: string } };
  "character.importBackup": { input: { content: string }; output: BackupImportResult };
  "group.list": { input: EmptyParams; output: GroupRecord[] };
  "group.get": { input: IdParams; output: GroupRecord };
  "group.create": { input: { values: GroupCreate }; output: GroupRecord };
  "group.update": { input: { id: string; changes: GroupChanges }; output: GroupRecord };
  "group.delete": { input: IdParams; output: { id: string; deleted: boolean } };
  "group.setActive": { input: IdParams; output: ActivationResult };
  "group.duplicate": { input: { id: string; newName?: string }; output: GroupRecord };
  "group.export": { input: IdParams; output: { id: string; content: string } };
  "group.import": { input: { content: string }; output: GroupRecord };
  "group.exportBackup": { input: EmptyParams; output: { content: string } };
  "group.importBackup": { input: { content: string }; output: BackupImportResult };
  "activePrompt.get": { input: EmptyParams; output: ActivePromptRecord };
  "activePrompt.setCard": { input: IdParams; output: ActivationResult };
  "activePrompt.setGroup": { input: IdParams; output: ActivationResult };
  "activePrompt.activateForChat": { input: { characterCardName: string | null; characterGroupId: string | null }; output: { characterCardName: string | null; characterGroupId: string | null; updated: boolean } };
  "activePrompt.resolvedCard": { input: EmptyParams; output: IdParams };
  "tag.list": { input: EmptyParams; output: TagRecord[] };
  "tag.get": { input: IdParams; output: TagRecord };
  "tag.create": { input: { values: TagCreate }; output: TagRecord };
  "tag.update": { input: { id: string; changes: TagChanges }; output: TagRecord };
  "tag.delete": { input: IdParams; output: { id: string; deleted: boolean } };
  "memory.shared.list": { input: EmptyParams; output: StoreRecord[] };
  "memory.shared.create": { input: { name: string }; output: StoreRecord };
  "memory.shared.rename": { input: { id: string; name: string }; output: StoreRecord };
  "memory.shared.delete": { input: IdParams; output: { sharedId: string; deleted: boolean; cleanedCharacters: number } };
  "memory.mount": { input: { characterId: string; sharedId: string; readable: boolean; writable: boolean }; output: { characterId: string; sharedId: string; mount: { sharedMemoryId: string; readable: boolean; writable: boolean }; mounted: boolean } };
  "memory.unmount": { input: { characterId: string; sharedId: string }; output: { characterId: string; sharedId: string; unmounted: boolean } };
  "memory.user.read": { input: OwnerParams; output: { ownerKey: string; content: string } };
  "memory.user.write": { input: OwnerParams & { content: string }; output: { ownerKey: string; contentLength: number; updated: boolean } };
  "memory.user.path": { input: OwnerParams; output: { ownerKey: string; path: string } };
  "memory.resolveOwner": { input: { characterId: string }; output: { kind: "CHARACTER" | "SHARED"; id: string } };
  "memory.settings.read": { input: OwnerParams; output: MemorySettingsRecord };
  "memory.settings.write": { input: OwnerParams & { settings: MemorySettingsRecord }; output: MemorySettingsRecord };
  "memory.searchConfig.read": { input: OwnerParams; output: MemorySearchConfigRecord };
  "memory.searchConfig.write": { input: OwnerParams & { config: MemorySearchConfigRecord }; output: MemorySearchConfigRecord };
  "memory.graph": { input: OwnerParams; output: { ownerKey: string; graph: MemoryGraphRecord } };
  "memory.list": { input: OwnerParams; output: { ownerKey: string; items: MemoryRecord[] } };
  "memory.search": { input: OwnerParams & { query: string }; output: { ownerKey: string; query: string; items: MemoryRecord[] } };
  "memory.get": { input: OwnerParams & { title: string }; output: { ownerKey: string; item: MemoryRecord } };
  "memory.create": { input: OwnerParams & { values: MemoryCreate }; output: { ownerKey: string; item: MemoryRecord; created: boolean } };
  "memory.update": { input: OwnerParams & { originalTitle: string; expectedId?: string; changes: MemoryChanges }; output: { ownerKey: string; item: MemoryRecord; updated: boolean } };
  "memory.delete": { input: OwnerParams & { id: string }; output: { ownerKey: string; id: string; deleted: boolean } };
  "memory.move": { input: OwnerParams & { ids: string[]; folderPath: string }; output: { ownerKey: string; ids: string[]; folder: string; moved: number } };
  "memory.link.create": { input: OwnerParams & { sourceTitle: string; targetTitle: string; linkType: string; weight: number; description: string }; output: { ownerKey: string; link: LinkRecord } };
  "memory.link.update": { input: OwnerParams & { linkId: string; changes: { linkType?: string; weight?: number; description?: string } }; output: { ownerKey: string; link: LinkRecord } };
  "memory.link.delete": { input: OwnerParams & { linkId: string }; output: { ownerKey: string; linkId: string; deleted: boolean } };
  "memory.export": { input: OwnerParams; output: { ownerKey: string; content: string } };
  "memory.import": { input: OwnerParams & { content: string; strategy: MemoryImportStrategy }; output: { ownerKey: string; result: MemoryImportResult } };
}
/** Selects a concrete public domain method. */
export type DomainOperation = keyof DomainOperations;
/** Selects the input type of one domain method. */
export type DomainInput<K extends DomainOperation> = DomainOperations[K]["input"];
/** Selects the result type of one domain method. */
export type DomainOutput<K extends DomainOperation> = DomainOperations[K]["output"];

/** Calls an explicitly registered method on the character package's dependency boundary. */
function call<K extends DomainOperation>(operation: K, payload: DomainInput<K>): Promise<DomainOutput<K>> {
  return ToolPkg.callDependency<DomainInput<K>, DomainOutput<K>>("com.operit.character_cards", operation, payload);
}
/** Exposes typed domain methods to packages declaring a dependency on character cards. */
export const characterCards = {

  chat: {
    configuration: {
      /** Resolves this chat's stored configuration without changing its selection. */
      resolve: (payload: ChatConfigurationRequest): Promise<ChatConfigurationResult> => call("chat.configuration.resolve", payload),
      binding: {
        /** Requires one existing chat selection. */
        read: (payload: { chatId: string }): Promise<ChatBindingRecord> => call("chat.configuration.binding.read", payload),
        /** Commits an explicit selection through the sole authoritative service. */
        write: (payload: ChatBindingRecord): Promise<ChatBindingRecord> => call("chat.configuration.binding.write", payload),
        /** Deletes an existing selection without inventing another actor. */
        delete: (payload: { chatId: string }): Promise<{ chatId: string; deleted: boolean }> => call("chat.configuration.binding.delete", payload),
      },
    },
  },
  /** Reads the editor directory. */
  snapshot: (): Promise<SnapshotRecord> => call("snapshot", {}),
  characters: {
    /** Lists complete characters. */
    list: (): Promise<CharacterRecord[]> => call("character.list", {}),
    /** Reads a character. */
    get: (payload: IdParams): Promise<CharacterRecord> => call("character.get", payload),
    /** Creates a character. */
    create: (payload: DomainInput<"character.create">): Promise<CharacterRecord> => call("character.create", payload),
    /** Applies a character patch. */
    update: (payload: DomainInput<"character.update">): Promise<CharacterRecord> => call("character.update", payload),
    /** Deletes a character. */
    delete: (payload: IdParams): Promise<DomainOutput<"character.delete">> => call("character.delete", payload),
    /** Activates a character through the domain service. */
    setActive: (payload: IdParams): Promise<ActivationResult> => call("character.setActive", payload),
    /** Combines the character prompt and selected tags. */
    combine: (payload: DomainInput<"character.combine">): Promise<DomainOutput<"character.combine">> => call("character.combine", payload),
    /** Resets the built-in character. */
    resetDefault: (): Promise<DomainOutput<"character.resetDefault">> => call("character.resetDefault", {}),
    /** Exports an explicitly selected character format. */
    export: (payload: DomainInput<"character.export">): Promise<DomainOutput<"character.export">> => call("character.export", payload),
    /** Imports an explicitly selected character format. */
    import: (payload: DomainInput<"character.import">): Promise<CharacterRecord> => call("character.import", payload),
    /** Exports all characters and prompt tags. */
    exportBackup: (): Promise<{ content: string }> => call("character.exportBackup", {}),
    /** Imports a character and tag backup. */
    importBackup: (payload: { content: string }): Promise<BackupImportResult> => call("character.importBackup", payload),
  },
  groups: {
    /** Lists complete groups. */
    list: (): Promise<GroupRecord[]> => call("group.list", {}),
    /** Reads a group. */
    get: (payload: IdParams): Promise<GroupRecord> => call("group.get", payload),
    /** Creates a group. */
    create: (payload: DomainInput<"group.create">): Promise<GroupRecord> => call("group.create", payload),
    /** Applies a group patch. */
    update: (payload: DomainInput<"group.update">): Promise<GroupRecord> => call("group.update", payload),
    /** Deletes a group. */
    delete: (payload: IdParams): Promise<DomainOutput<"group.delete">> => call("group.delete", payload),
    /** Activates a group through the domain service. */
    setActive: (payload: IdParams): Promise<ActivationResult> => call("group.setActive", payload),
    /** Duplicates a group. */
    duplicate: (payload: DomainInput<"group.duplicate">): Promise<GroupRecord> => call("group.duplicate", payload),
    /** Exports one native group. */
    export: (payload: IdParams): Promise<DomainOutput<"group.export">> => call("group.export", payload),
    /** Imports one native group. */
    import: (payload: { content: string }): Promise<GroupRecord> => call("group.import", payload),
    /** Exports all groups. */
    exportBackup: (): Promise<{ content: string }> => call("group.exportBackup", {}),
    /** Imports a group backup. */
    importBackup: (payload: { content: string }): Promise<BackupImportResult> => call("group.importBackup", payload),
  },
  activePrompt: {
    /** Reads the active prompt. */
    get: (): Promise<ActivePromptRecord> => call("activePrompt.get", {}),
    /** Activates a character card. */
    setCard: (payload: IdParams): Promise<ActivationResult> => call("activePrompt.setCard", payload),
    /** Activates a character group. */
    setGroup: (payload: IdParams): Promise<ActivationResult> => call("activePrompt.setGroup", payload),
    /** Applies the chat binding through the manager. */
    activateForChat: (payload: DomainInput<"activePrompt.activateForChat">): Promise<DomainOutput<"activePrompt.activateForChat">> => call("activePrompt.activateForChat", payload),
    /** Resolves the active sending card. */
    resolvedCard: (): Promise<IdParams> => call("activePrompt.resolvedCard", {}),
  },
  tags: {
    /** Lists prompt tags. */
    list: (): Promise<TagRecord[]> => call("tag.list", {}),
    /** Reads a prompt tag. */
    get: (payload: IdParams): Promise<TagRecord> => call("tag.get", payload),
    /** Creates a prompt tag. */
    create: (payload: DomainInput<"tag.create">): Promise<TagRecord> => call("tag.create", payload),
    /** Applies a prompt tag patch. */
    update: (payload: DomainInput<"tag.update">): Promise<TagRecord> => call("tag.update", payload),
    /** Deletes a prompt tag. */
    delete: (payload: IdParams): Promise<DomainOutput<"tag.delete">> => call("tag.delete", payload),
  },
  memory: {
    /** Queries participant-bound memories using provider-owned snapshots and document matching. */
    query: (payload: MemoryQueryRequest): Promise<MemoryQueryResult> => ToolPkg.callDependency<MemoryQueryRequest, MemoryQueryResult>("com.operit.character_cards", "memory.query", payload),
    chats: {
      /** Lists actual generic chat summaries selected by persisted plugin bindings. */
      list: (payload: OwnerParams): Promise<MemoryChat[]> => call("memory.chat.list", payload),
      /** Applies genuine functional MEMORY extraction to the selected real chat. */
      update: (payload: DomainInput<"memory.chat.update">): Promise<MemoryExtractionResult> => call("memory.chat.update", payload),
    },
    /** Categorizes full root memories using the configured functional MEMORY model. */
    categorize: (payload: OwnerParams): Promise<number> => call("memory.categorize", payload),
    rebuild: {
      /** Persists a real rebuild plan for interval execution. */
      start: (payload: DomainInput<"memory.rebuild.start">): Promise<MemoryRebuildProgress> => call("memory.rebuild.start", payload),
      /** Reads actual durable progress counters. */
      progress: (payload: OwnerParams): Promise<MemoryRebuildProgress> => call("memory.rebuild.progress", payload),
      /** Cancels a genuine active plan before another source window. */
      cancel: (payload: OwnerParams): Promise<MemoryRebuildProgress> => call("memory.rebuild.cancel", payload),
    },
    embeddings: {
      /** Recomputes real provider vectors over every complete node and chunk. */
      rebuild: (payload: OwnerParams): Promise<number> => call("memory.embeddings.rebuild", payload),
    },
    candidates: {
      /** Enqueues an actual finalized reply or explicit selected-user message. */
      enqueue: (payload: DomainInput<"memory.candidate.enqueue">): Promise<DomainOutput<"memory.candidate.enqueue">> => call("memory.candidate.enqueue", payload),
    },
    shared: {
      /** Lists shared memory libraries. */
      list: (): Promise<StoreRecord[]> => call("memory.shared.list", {}),
      /** Creates a shared memory library. */
      create: (payload: { name: string }): Promise<StoreRecord> => call("memory.shared.create", payload),
      /** Renames a shared memory library. */
      rename: (payload: DomainInput<"memory.shared.rename">): Promise<StoreRecord> => call("memory.shared.rename", payload),
      /** Deletes a library and cleans character mounts. */
      delete: (payload: IdParams): Promise<DomainOutput<"memory.shared.delete">> => call("memory.shared.delete", payload),
    },
    /** Mounts a shared library with explicit permissions. */
    mount: (payload: DomainInput<"memory.mount">): Promise<DomainOutput<"memory.mount">> => call("memory.mount", payload),
    /** Removes a shared library mount. */
    unmount: (payload: DomainInput<"memory.unmount">): Promise<DomainOutput<"memory.unmount">> => call("memory.unmount", payload),
    user: {
      /** Reads the owner's USER.md. */
      read: (payload: OwnerParams): Promise<DomainOutput<"memory.user.read">> => call("memory.user.read", payload),
      /** Writes the owner's USER.md. */
      write: (payload: DomainInput<"memory.user.write">): Promise<DomainOutput<"memory.user.write">> => call("memory.user.write", payload),
      /** Reads the real plugin-owned USER.md path accepted by Files. */
      path: (payload: OwnerParams): Promise<DomainOutput<"memory.user.path">> => call("memory.user.path", payload),
    },
    /** Resolves a character's actual bound memory owner. */
    resolveOwner: (payload: { characterId: string }): Promise<DomainOutput<"memory.resolveOwner">> => call("memory.resolveOwner", payload),
    settings: {
      /** Reads owner-scoped memory settings. */
      read: (payload: OwnerParams): Promise<MemorySettingsRecord> => call("memory.settings.read", payload),
      /** Writes owner-scoped memory settings. */
      write: (payload: DomainInput<"memory.settings.write">): Promise<MemorySettingsRecord> => call("memory.settings.write", payload),
    },
    searchConfig: {
      /** Reads the search scoring configuration. */
      read: (payload: OwnerParams): Promise<MemorySearchConfigRecord> => call("memory.searchConfig.read", payload),
      /** Writes the search scoring configuration. */
      write: (payload: DomainInput<"memory.searchConfig.write">): Promise<MemorySearchConfigRecord> => call("memory.searchConfig.write", payload),
    },
    /** Reads a memory graph. */
    graph: (payload: OwnerParams): Promise<DomainOutput<"memory.graph">> => call("memory.graph", payload),
    /** Lists memory records. */
    list: (payload: OwnerParams): Promise<DomainOutput<"memory.list">> => call("memory.list", payload),
    /** Searches full records with explicit filters through the sole service and its durable embedding cache. */
    searchWithOptions: (payload: MemorySearchOptions): Promise<DomainOutput<"memory.searchWithOptions">> => call("memory.searchWithOptions", payload),
    /** Searches memory records. */
    search: (payload: DomainInput<"memory.search">): Promise<DomainOutput<"memory.search">> => call("memory.search", payload),
    /** Reads a memory record by title. */
    get: (payload: DomainInput<"memory.get">): Promise<DomainOutput<"memory.get">> => call("memory.get", payload),
    /** Creates a memory record. */
    create: (payload: DomainInput<"memory.create">): Promise<DomainOutput<"memory.create">> => call("memory.create", payload),
    /** Applies a memory record patch. */
    update: (payload: DomainInput<"memory.update">): Promise<DomainOutput<"memory.update">> => call("memory.update", payload),
    /** Deletes a memory record by its stable string ID. */
    delete: (payload: DomainInput<"memory.delete">): Promise<DomainOutput<"memory.delete">> => call("memory.delete", payload),
    /** Moves memory records to a folder. */
    move: (payload: DomainInput<"memory.move">): Promise<DomainOutput<"memory.move">> => call("memory.move", payload),
    links: {
      /** Creates a memory relationship. */
      create: (payload: DomainInput<"memory.link.create">): Promise<DomainOutput<"memory.link.create">> => call("memory.link.create", payload),
      /** Applies a memory relationship patch. */
      update: (payload: DomainInput<"memory.link.update">): Promise<DomainOutput<"memory.link.update">> => call("memory.link.update", payload),
      /** Deletes a memory relationship. */
      delete: (payload: DomainInput<"memory.link.delete">): Promise<DomainOutput<"memory.link.delete">> => call("memory.link.delete", payload),
    },
    /** Exports a repository memory backup. */
    export: (payload: OwnerParams): Promise<DomainOutput<"memory.export">> => call("memory.export", payload),
    /** Imports a backup using the explicitly selected strategy. */
    import: (payload: DomainInput<"memory.import">): Promise<DomainOutput<"memory.import">> => call("memory.import", payload),
  },
};

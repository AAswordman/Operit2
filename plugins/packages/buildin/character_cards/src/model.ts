import type { ComposeDslContext, ComposeNode, ComposeThemeSnapshot } from "../../../../types/compose-dsl";

export interface Definition { id: string; title: string; icon: string; order: number }

/** Defines the complete plugin-owned character record with single-direction references to independent configurations. */
export interface Card {
  id: string;
  name: string;
  description: string;
  characterSetting: string;
  openingStatement: string;
  otherContentChat: string;
  otherContentVoice: string;
  avatarUri: string | null;
  attachedTagIds: string[];
  advancedCustomPrompt: string;
  marks: string;
  chatModelBindingMode: string;
  chatModelId: string | null;
  ttsConfigId: string | null;
  themeConfigId: string | null;
  memoryBindingMode: string;
  sharedMemoryId: string | null;
  sharedMemoryMounts: SharedMemoryMount[];
  toolAccessConfig: ToolAccessConfig;
  isDefault: boolean;
  createdAt: number;
  updatedAt: number;
}

export interface SharedMemoryMount { sharedMemoryId: string; readable: boolean; writable: boolean }
export interface ToolAccessConfig {
  enabled: boolean;
  allowedBuiltinTools: string[];
  allowedPackages: string[];
  allowedSkills: string[];
  allowedMcpServers: string[];
}

/** Defines a plugin-owned role-composition group with its own independent theme reference. */
export interface Group {
  id: string;
  name: string;
  description: string;
  members: GroupMember[];
  themeConfigId: string | null;
  createdAt: number;
  updatedAt: number;
}
export interface GroupMember { characterCardId: string; orderIndex: number }

/** Stores a manual conversation group independently from role-composition groups or Core chat fields. */
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
/** Requires an explicit scope and pin state when creating a genuinely new empty manual group. */
export interface ConversationGroupCreate { ownerSelection: string | null; name: string; pinned: boolean }
/** Restricts manual group edits to their independent editable metadata. */
export interface ConversationGroupChanges { name?: string; pinned?: boolean }
/** Reports explicit membership without inventing an ungrouped record. */
export interface ConversationGroupMoveResult { chatId: string; previousGroupId: string | null; groupId: string | null }
/** Releases all manual memberships without deleting any host chats. */
export interface ConversationGroupDeletion { id: string; deleted: true; releasedChatIds: string[] }

export type GroupValues = Pick<Group, "id" | "name" | "description" | "members" | "themeConfigId">;

/** Matches the complete serde record in operit_model::Memory::SharedMemoryStore. */
export interface Store { id: string; name: string; createdAt: number; updatedAt: number }
export type StoreValues = Pick<Store, "id" | "name">;

/** Matches the complete serde record in operit_model::PromptTag. */
export interface Tag {
  id: string;
  name: string;
  description: string;
  promptContent: string;
  tagType: "TONE" | "CHARACTER" | "FUNCTION" | "CUSTOM";
  createdAt: number;
  updatedAt: number;
}
export type TagValues = Pick<Tag, "name" | "description" | "promptContent" | "tagType">;
export type TagEdit = TagValues & { id: string };
export interface TagChanges {
  created: { draftId: string; values: TagValues }[];
  updated: TagEdit[];
  deleted: string[];
}

/** Matches the externally tagged serde enum in operit_model::ActivePrompt. */
export type ActivePrompt = { CharacterCard: { id: string } } | { CharacterGroup: { id: string } };

/** Matches ProviderModelSummary without inventing the absent modelName field. */
export interface ModelSummary {
  providerId: string;
  providerName: string;
  providerTypeId: string;
  endpoint: string;
  modelId: string;
  capabilities: { directImage: boolean; directAudio: boolean; directVideo: boolean; toolCall: boolean };
  pricing: ModelPricing | null;
}
export interface ModelPricing {
  billingMode: "TOKEN" | "COUNT";
  inputPricePerMillion: number;
  cachedInputPricePerMillion: number | null;
  cacheWritePricePerMillion: number | null;
  outputPricePerMillion: number;
  pricePerRequest: number;
  currency: "CNY" | "USD";
}

/** Matches the complete serde record in operit_model::TtsConfig. */
export interface TtsConfig {
  id: string;
  name: string;
  providerType: string;
  endpoint: string;
  apiKey: string;
  model: string;
  voice: string;
  responseFormat: string;
  speed: number;
  httpMethod: string;
  requestBody: string;
  contentType: string;
  headers: { name: string; value: string }[];
  responsePipeline: { stepType: string; path: string; headers: { name: string; value: string }[] }[];
  createdAt: number;
  updatedAt: number;
}

/** Exposes selectable source identities from the actual host tool registries. */
export interface ToolCatalogEntry { name: string; displayName: string; description: string }
export interface ToolCatalog {
  builtinTools: ToolCatalogEntry[];
  packages: ToolCatalogEntry[];
  skills: ToolCatalogEntry[];
  mcpServers: ToolCatalogEntry[];
}

/** Matches the complete serde record in operit_model::Memory::Memory. */
export interface Memory {
  id: string;
  uuid: string;
  title: string;
  content: string;
  contentType: string;
  source: string;
  credibility: number;
  importance: number;
  documentPath: string | null;
  isDocumentNode: boolean;
  chunkIndexFilePath: string | null;
  folderPath: string | null;
  createdAt: number;
  updatedAt: number;
  lastAccessedAt: number;
  tags: { id: string; name: string }[];
  properties: { id: string; key: string; value: string }[];
}
/** Preserves the complete DocumentChunk serde record, including its lossless SQLite identity. */
export interface DocumentChunk { id: string; memoryUuid: string; chunkIndex: number; content: string }
/** Persists one exact owner-scoped message revision candidate and its durable execution metadata. */
export interface MemoryCandidate {
  id: string; chatId: string; triggerMessageTimestamp: number; triggerVariantIndex: number; createdAt: number; updatedAt: number;
  status: string; attemptCount: number; lastError: string; sourceType: string;
}
export interface MemoryValues {
  title: string;
  content: string;
  contentType: string;
  source: string;
  credibility: number;
  importance: number;
  folderPath: string | null;
  tags: string[];
}
export interface MemorySearchOptions {
  ownerKey: string;
  query: string;
  folderPath: string | null;
  relevanceThreshold: number;
  createdAtStartMs: number | null;
  createdAtEndMs: number | null;
}
export interface MemoryLink {
  id: string;
  sourceMemoryId: string;
  targetMemoryId: string;
  type_: string;
  weight: number;
  description: string;
}
export type MemoryLinkValues = Omit<MemoryLink, "id">;
export type MemoryLinkUpdate = Pick<MemoryLink, "type_" | "weight" | "description">;
export interface MemoryGraph {
  nodes: { id: string; label: string; color: number; metadata: Record<string, string> }[];
  edges: { id: string; sourceId: string; targetId: string; label: string | null; weight: number; metadata: Record<string, string>; isCrossFolderLink: boolean }[];
}
export interface MemoryGraphResult { graph: MemoryGraph; items: Memory[] }
export interface UserDocument { ownerKey: string; content: string }
export interface MemoryOwner { kind: "CHARACTER" | "SHARED"; id: string }

/** Matches the complete owner-scoped serde record in operit_model::MemorySettings. */
export interface MemorySettings {
  autoSaveIntervalMinutes: number;
  nextAutoSaveRunAtMs: number;
  memoryExtractionCustomRules: string;
  profileAutoUpdateEnabled: boolean;
  profileAutoUpdateLocked: boolean;
  cloudEmbeddingEnabled: boolean;
  cloudEmbeddingEndpoint: string;
  cloudEmbeddingApiKey: string;
  cloudEmbeddingModel: string;
}
export interface MemorySearchConfig {
  scoreMode: "BALANCED" | "KEYWORD_FIRST" | "SEMANTIC_FIRST";
  keywordWeight: number;
  tagWeight: number;
  vectorWeight: number;
  edgeWeight: number;
}

/** Aggregates independent canonical reads only after every requested read succeeds. */
export interface Snapshot {
  cards: Card[];
  groups: Group[];
  stores: Store[];
  tags: Tag[];
  models: ModelSummary[];
  ttsConfigs: TtsConfig[];
  toolCatalog: ToolCatalog;
  active: ActivePrompt;
}

/** Keeps UI and registered domain commands on one finite, plugin-owned request contract. */
export type Request =
  | { action: "snapshot" }
  | { action: "listThemeChoices" }
  | { action: "listCharacters" }
  | { action: "getCharacter"; id: string }
  | { action: "saveCharacter"; card: Card; create: boolean; tagChanges: TagChanges }
  | { action: "deleteCharacter"; id: string }
  | { action: "activate"; type: "card" | "group"; id: string }
  | { action: "readActive" }
  | { action: "readChatBinding"; chatId: string }
  | { action: "writeChatBinding"; chatId: string; selection: string }
  | { action: "deleteChatBinding"; chatId: string }
  | { action: "listConversationGroups"; ownerSelection: string | null }
  | { action: "createConversationGroup"; values: ConversationGroupCreate }
  | { action: "updateConversationGroup"; id: string; changes: ConversationGroupChanges }
  | { action: "deleteConversationGroup"; id: string }
  | { action: "moveConversationGroupChat"; chatId: string; groupId: string | null; ownerSelection: string | null }
  | { action: "reorderConversationGroups"; ownerSelection: string | null; ids: string[] }
  | { action: "listGroups" }
  | { action: "getGroup"; id: string }
  | { action: "saveGroup"; group: GroupValues; create: boolean }
  | { action: "deleteGroup"; id: string }
  | { action: "listTags" }
  | { action: "saveTag"; tag: TagEdit; create: boolean }
  | { action: "deleteTag"; id: string }
  | { action: "listStores" }
  | { action: "saveStore"; store: StoreValues; create: boolean }
  | { action: "deleteStore"; id: string }
  | { action: "listModels" }
  | { action: "listTtsConfigs" }
  | { action: "readToolCatalog" }
  | { action: "resolveMemoryOwner"; characterId: string }
  | { action: "readUser"; ownerKey: string }
  | { action: "writeUser"; ownerKey: string; content: string }
  | { action: "graph"; ownerKey: string }
  | { action: "listMemories"; ownerKey: string }
  | { action: "searchMemory"; ownerKey: string; query: string }
  | { action: "searchMemories"; options: MemorySearchOptions }
  | { action: "saveMemory"; ownerKey: string; title: string; originalTitle: string | null; content: string; folderPath: string; tags: string; contentType: string; source: string; credibility: number; importance: number }
  | { action: "deleteMemory"; ownerKey: string; title: string }
  | { action: "createLink"; ownerKey: string; sourceTitle: string; targetTitle: string; linkType: string; weight: number; description: string }
  | { action: "deleteLink"; ownerKey: string; linkId: string }
  | { action: "updateLink"; ownerKey: string; linkId: string; linkType: string; weight: number; description: string }
  | { action: "readMemorySettings"; ownerKey: string }
  | { action: "writeMemorySettings"; ownerKey: string; settings: MemorySettings }
  | { action: "readMemorySearchConfig"; ownerKey: string }
  | { action: "writeMemorySearchConfig"; ownerKey: string; config: MemorySearchConfig }
  | { action: "combinePrompts"; characterId: string; additionalTagIds: string[]; promptFunctionType: "CHAT" | "VOICE" }
  | { action: "importCharacter"; format: "operit" | "tavern"; content: string }
  | { action: "exportCharacter"; id: string; format: "operit" | "tavern" }
  | { action: "importGroup"; content: string }
  | { action: "exportGroup"; id: string }
  | { action: "listMemoryFolders"; ownerKey: string }
  | { action: "readMemoryAutoSaveStatus"; ownerKey: string }
  | { action: "listMemoryChats"; ownerKey: string }
  | { action: "updateChatMemory"; ownerKey: string; chatId: string }
  | { action: "autoCategorizeMemory"; ownerKey: string }
  | { action: "startMemoryRebuild"; ownerKey: string; rebuild: MemoryRebuild }
  | { action: "readMemoryRebuildProgress"; ownerKey: string }
  | { action: "cancelMemoryRebuild"; ownerKey: string }
  | { action: "rebuildMemoryEmbeddings"; ownerKey: string }
  | { action: "exportMemory"; ownerKey: string }
  | { action: "importMemory"; ownerKey: string; content: string; strategy: "SKIP" | "UPDATE" | "CREATE_NEW" };

/** Matches every field of operit_model::MemoryAutoSaveStatus. */
export interface MemoryAutoSaveStatus {
  ownerKey: string;
  pendingCandidates: number;
  pendingChats: number;
  processingCandidates: number;
  failedCandidates: number;
  nextRunAtMs: number;
  minutesUntilNextRun: number;
  lastError: string;
}

/** Carries the exact generic chat directory summary, without fabricating absent ChatHistory fields. */
export interface MemoryChat {
  id: string; title: string; messageCount: number; createdAt: string; updatedAt: string;
  isCurrent: boolean; inputTokens: number; outputTokens: number;
}

/** Uses only message fields actually supplied by the generic chat message reader. */
export interface MemoryChatMessage {
  sender: string; content: string; timestamp: number; variantIndex: number; variantCount: number; provider: string; modelName: string;
}

/** Persists one planned extraction window, including the original source-message count. */
export interface MemoryJobWindow { chatId: string; chatTitle: string; messages: MemoryChatMessage[]; sourceMessageCount: number }
/** Persists the private rebuild cursor in the same authoritative owner snapshot. */
export interface MemoryRebuildTask { id: string; windows: MemoryJobWindow[]; nextWindow: number }
/** Stores a real provider embedding, keyed by exact configured source and input text. */
export interface MemoryEmbedding { endpoint: string; model: string; text: string; vector: number[]; updatedAt: number }
/** Reports actual committed domain operations for a manual or background extraction. */
export interface MemoryExtractionResult { ownerKey: string; created: number; updated: number; merged: number; links: number; profileUpdated: boolean }

/** Matches the complete MessagePart serde record used by extraction windows. */
export interface MemoryMessagePart {
  partId: string;
  sequence: number;
  kind: "markdown" | "thinking" | "tool_call" | "tool_result" | "status";
  content: string;
  toolCallId: string | null;
  toolName: string | null;
  attributes: Record<string, string>;
}

/** Describes an opaque JSON value without inventing the transport's stream implementation. */
export type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };

/** Matches the explicit arguments of MemoryManagementService::startRebuild. */
export interface MemoryRebuild {
  chatIds: string[];
  windowMessageCount: number;
  fromInclusive: number | null;
  toInclusive: number | null;
}

/** Matches every field of operit_model::MemoryRebuildProgress. */
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

export type Screen = (ctx: ComposeDslContext) => ComposeNode;
export type Theme = ComposeThemeSnapshot;
/** Holds the complete plugin-owned memory space in the same durable snapshot as its owner. */
export interface MemorySpace {
  ownerKey: string;
  memories: Memory[];
  links: MemoryLink[];
  chunks: DocumentChunk[];
  candidates: MemoryCandidate[];
  settings: MemorySettings;
  searchConfig: MemorySearchConfig;
  userDocumentPath: string;
  rebuildProgress: MemoryRebuildProgress;
  rebuildTask: MemoryRebuildTask | null;
  embeddings: MemoryEmbedding[];
}
/** Owns the file schema for this plugin only; no runtime-wide storage schema is imposed. */
export interface CharacterState {
  version: 3;
  nextId: string;
  cards: Card[];
  groups: Group[];
  tags: Tag[];
  stores: Store[];
  active: ActivePrompt;
  owners: MemorySpace[];
  conversationGroups: ConversationGroupRecord[];
}
/** Reports the result of a validated identity-preserving character or group backup import. */
export interface BackupImportResult { new: number; updated: number; skipped: number; total: number }
/** Reports actual memory and relationship changes performed by a portable import. */
export interface MemoryImportResult { newMemories: number; updatedMemories: number; skippedMemories: number; newLinks: number }
/** Supplies external directories explicitly; the file repository never impersonates their owner. */
export interface CharacterDirectories {
  /** Reads complete model summaries from their real configured source. */
  listModels(): Promise<ModelSummary[]>;
  /** Reads complete TTS configurations from their real configured source. */
  listTtsConfigs(): Promise<TtsConfig[]>;
  /** Reads the real builtin, package, skill and MCP source directories. */
  readToolCatalog(): Promise<ToolCatalog>;
}

/** Projects an explicit record-extension selection; this DTO is never stored in plugin files. */
export interface ChatBinding { chatId: string; selection: string }

/** Stages an owner document edit for the same serialized service operation. */
export interface UserDocumentWrite { ownerKey: string; path: string; content: string; newDocument: boolean }

/** Maps every finite editor action to the exact result returned by its service branch. */
export interface RequestResults {
  snapshot: Snapshot;
  listThemeChoices: import("./selection-application").ThemeChoice[];
  listCharacters: Card[];
  getCharacter: Card;
  saveCharacter: Snapshot;
  deleteCharacter: Snapshot;
  activate: Snapshot;
  readActive: ActivePrompt;
  readChatBinding: import("./api").ChatBindingRecord;
  writeChatBinding: import("./api").ChatBindingRecord;
  deleteChatBinding: { chatId: string; deleted: boolean; };
  listConversationGroups: ConversationGroupRecord[];
  createConversationGroup: ConversationGroupRecord;
  updateConversationGroup: ConversationGroupRecord;
  deleteConversationGroup: ConversationGroupDeletion;
  moveConversationGroupChat: ConversationGroupMoveResult;
  reorderConversationGroups: ConversationGroupRecord[];
  listGroups: Group[];
  getGroup: Group;
  saveGroup: Snapshot;
  deleteGroup: Snapshot;
  listTags: Tag[];
  saveTag: Tag;
  deleteTag: Snapshot;
  listStores: Store[];
  saveStore: Snapshot;
  deleteStore: Snapshot;
  listModels: ModelSummary[];
  listTtsConfigs: TtsConfig[];
  readToolCatalog: ToolCatalog;
  resolveMemoryOwner: string;
  readUser: UserDocument;
  writeUser: { saved: boolean; };
  graph: MemoryGraphResult;
  listMemories: Memory[];
  searchMemory: Memory[];
  searchMemories: Memory[];
  saveMemory: MemoryGraphResult;
  deleteMemory: MemoryGraphResult;
  createLink: MemoryGraphResult;
  deleteLink: MemoryGraphResult;
  updateLink: MemoryGraphResult;
  readMemorySettings: MemorySettings;
  writeMemorySettings: MemorySettings;
  readMemorySearchConfig: MemorySearchConfig;
  writeMemorySearchConfig: MemorySearchConfig;
  combinePrompts: string;
  importCharacter: Snapshot;
  exportCharacter: string;
  importGroup: Snapshot;
  exportGroup: string;
  listMemoryFolders: string[];
  readMemoryAutoSaveStatus: MemoryAutoSaveStatus;
  listMemoryChats: MemoryChat[];
  updateChatMemory: MemoryExtractionResult;
  autoCategorizeMemory: number;
  startMemoryRebuild: MemoryRebuildProgress;
  readMemoryRebuildProgress: MemoryRebuildProgress;
  cancelMemoryRebuild: MemoryRebuildProgress;
  rebuildMemoryEmbeddings: number;
  exportMemory: { ownerKey: string; content: string; };
  importMemory: { ownerKey: string; result: MemoryImportResult; };
}

/** Preserves the response type selected by the request action across service and IPC calls. */
export type RequestOutput<R extends Request = Request> = RequestResults[R["action"]];

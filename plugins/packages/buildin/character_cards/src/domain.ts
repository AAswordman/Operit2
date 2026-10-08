import { requireChatSelection } from "./chat-bindings";
import { decodeChatMarker, decodeMessageMarker, encodeMessageMarker, markerObject } from "./chat-markers";
import { resolveChatDisplay } from "./group-execution/display";
import type { ChatConfigurationExecutionResult, ChatConfigurationRequest, ChatConfigurationResult, ChatParticipantProfile, CharacterRecord, DomainInput, DomainOperation, DomainOutput, SnapshotRecord } from "./api";
export type { DomainInput, DomainOperation, DomainOutput, DomainOperations } from "./api";

type Validator = (value: unknown, path: string) => void;
interface Field { validate: Validator; required: boolean }
type Fields = Record<string, Field>;

/** Rejects non-object input before inspecting its declared fields. */
export function record(value: unknown, path: string): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new Error(`${path} must be a JSON object`);
  return value as Record<string, unknown>;
}
/** Validates text without trimming user-authored content. */
function text(value: unknown, path: string): void {
  if (typeof value !== "string") throw new Error(`${path} must be a string`);
}
/** Validates nonblank names and identifiers without changing their value. */
function nonblank(value: unknown, path: string): void {
  text(value, path);
  if ((value as string).trim() === "") throw new Error(`${path} must not be blank`);
}
/** Validates a lossless plugin record identity without accepting or coercing numeric JSON. */
export function parseMemoryIdentifier(value: unknown, path: string, positive = true): string {
  if (typeof value !== "string" || !/^(0|[1-9][0-9]*)$/.test(value)) throw new Error(`${path} must be a canonical decimal string`);
  const id = BigInt(value);
  if (id > 9223372036854775807n || (positive && id === 0n)) throw new Error(`${path} is outside its declared identity range`);
  return value;
}
/** Validates an existing memory or relationship identity as an unchanged string. */
function memoryIdentifier(value: unknown, path: string): void { parseMemoryIdentifier(value, path); }
/** Validates a stored tag or property identity, including its explicit zero spelling. */
function propertyIdentifier(value: unknown, path: string): void { parseMemoryIdentifier(value, path, false); }
/** Validates an explicit boolean rather than coercing strings or numbers. */
function boolean(value: unknown, path: string): void {
  if (typeof value !== "boolean") throw new Error(`${path} must be a boolean`);
}
/** Validates finite numeric input. */
function number(value: unknown, path: string): void {
  if (typeof value !== "number" || !Number.isFinite(value)) throw new Error(`${path} must be a finite number`);
}
/** Validates safe integer timestamps, counters, and member-order positions. */
function integer(value: unknown, path: string): void {
  if (typeof value !== "number" || !Number.isSafeInteger(value)) throw new Error(`${path} must be a safe integer`);
}
/** Requires one explicit persisted message revision inside the generic host's i32 range. */
function messageVariant(value: unknown, path: string): void {
  integer(value, path);
  if (typeof value !== "number" || value < 0 || value > 2147483647) throw new Error(path + " must be a nonnegative i32 message variant index");
}
/** Validates unit-interval credibility, importance, and relationship weights. */
function score(value: unknown, path: string): void {
  number(value, path);
  if ((value as number) < 0 || (value as number) > 1) throw new Error(`${path} must be between 0 and 1`);
}
/** Validates nonnegative search weights and intervals. */
function nonnegative(value: unknown, path: string): void {
  number(value, path);
  if ((value as number) < 0) throw new Error(`${path} must not be negative`);
}
/** Validates a declared enum without spelling guesses or normalization. */
function enumeration(...values: string[]): Validator {
  const allowed = new Set(values);
  /** Checks one value against the exact declared alternatives. */
  return (value, path) => {
    if (typeof value !== "string" || !allowed.has(value)) throw new Error(`${path} must be ${values.join(" | ")}`);
  };
}
/** Permits the record's explicit null spelling. */
function nullable(validate: Validator): Validator {
  /** Validates each non-null value using the declared field type. */
  return (value, path) => { if (value !== null) validate(value, path); };
}
/** Validates every element of a declared array. */
function array(validate: Validator): Validator {
  /** Rejects non-arrays and identifies an invalid element by its index. */
  return (value, path) => {
    if (!Array.isArray(value)) throw new Error(`${path} must be an array`);
    for (let index = 0; index < value.length; index += 1) validate(value[index], `${path}[${index}]`);
  };
}
/** Marks a field as required in its input contract. */
function required(validate: Validator): Field { return { validate, required: true }; }
/** Marks a field as optional without inserting a replacement value. */
function optional(validate: Validator): Field { return { validate, required: false }; }
/** Checks all declared fields and rejects unknown keys and empty patches. */
function shape(fields: Fields, nonempty = false): Validator {
  /** Validates one JSON object against its exact schema. */
  return (value, path) => {
    const object = record(value, path);
    const keys = Object.keys(object);
    if (nonempty && keys.length === 0) throw new Error(`${path} must contain at least one field`);
    for (const key of keys) {
      if (!Object.prototype.hasOwnProperty.call(fields, key)) throw new Error(`${path}.${key} is not a supported field`);
      fields[key].validate(object[key], `${path}.${key}`);
    }
    for (const key of Object.keys(fields)) {
      if (fields[key].required && !Object.prototype.hasOwnProperty.call(object, key)) throw new Error(`${path}.${key} is required`);
    }
  };
}
/** Validates an owner using the exact host-supported namespace grammar. */
function owner(value: unknown, path: string): void {
  if (typeof value !== "string" || !/^(character|shared):[^:\s]+$/.test(value)) throw new Error(`${path} must be character:<id> or shared:<id>`);
}

const strings = array(text);
const identifiers = array(nonblank);
const member = shape({ characterCardId: required(nonblank), orderIndex: required(integer) });
const mount = shape({ sharedMemoryId: required(nonblank), readable: required(boolean), writable: required(boolean) });
const toolAccess = shape({ enabled: required(boolean), allowedBuiltinTools: required(strings), allowedPackages: required(strings), allowedSkills: required(strings), allowedMcpServers: required(strings) });
const property = shape({ id: required(propertyIdentifier), key: required(text), value: required(text) });
const characterFields: Fields = {
  name: optional(nonblank), description: optional(text), characterSetting: optional(text), openingStatement: optional(text),
  otherContentChat: optional(text), otherContentVoice: optional(text), avatarUri: optional(nullable(text)), attachedTagIds: optional(identifiers),
  advancedCustomPrompt: optional(text), marks: optional(text), chatModelBindingMode: optional(enumeration("FOLLOW_GLOBAL", "FIXED_MODEL")),
  chatModelId: optional(nullable(text)), ttsConfigId: optional(nullable(text)), themeConfigId: optional(nullable(text)), memoryBindingMode: optional(enumeration("CHARACTER", "SHARED")),
  sharedMemoryId: optional(nullable(text)), sharedMemoryMounts: optional(array(mount)), toolAccessConfig: optional(toolAccess),
  isDefault: optional(boolean), createdAt: optional(integer), updatedAt: optional(integer),
};
const groupFields: Fields = { name: optional(nonblank), description: optional(text), themeConfigId: optional(nullable(text)), members: optional(array(member)), createdAt: optional(integer), updatedAt: optional(integer) };
const tagFields: Fields = { name: optional(nonblank), description: optional(text), promptContent: optional(text), tagType: optional(enumeration("TONE", "CHARACTER", "FUNCTION", "CUSTOM")), createdAt: optional(integer), updatedAt: optional(integer) };
const memoryFields: Fields = {
  uuid: optional(text), title: optional(nonblank), content: optional(text), contentType: optional(text), source: optional(text),
  credibility: optional(score), importance: optional(score), documentPath: optional(nullable(text)), isDocumentNode: optional(boolean),
  chunkIndexFilePath: optional(nullable(text)), folderPath: optional(nullable(text)), createdAt: optional(integer), updatedAt: optional(integer),
  lastAccessedAt: optional(integer), tags: optional(strings), properties: optional(array(property)),
};
const settings = shape({
  autoSaveIntervalMinutes: required(nonnegative), nextAutoSaveRunAtMs: required(integer), memoryExtractionCustomRules: required(text),
  profileAutoUpdateEnabled: required(boolean), profileAutoUpdateLocked: required(boolean), cloudEmbeddingEnabled: required(boolean),
  cloudEmbeddingEndpoint: required(text), cloudEmbeddingApiKey: required(text), cloudEmbeddingModel: required(text),
});
const searchConfig = shape({ scoreMode: required(enumeration("BALANCED", "KEYWORD_FIRST", "SEMANTIC_FIRST")), keywordWeight: required(nonnegative), tagWeight: required(nonnegative), vectorWeight: required(nonnegative), edgeWeight: required(nonnegative) });
const empty = shape({});
const id = shape({ id: required(nonblank) });
const ownerOnly = shape({ ownerKey: required(owner) });
const contentOnly = shape({ content: required(text) });
const format = enumeration("operit", "tavern");
const validators: Record<DomainOperation, Validator> = {
  "conversation-group.list": shape({ ownerSelection: required(nullable(nonblank)) }),
  "conversation-group.create": shape({ ownerSelection: required(nullable(nonblank)), name: required(nonblank), pinned: required(boolean) }),
  "conversation-group.update": shape({ id: required(memoryIdentifier), changes: required(shape({ name: optional(nonblank), pinned: optional(boolean) }, true)) }),
  "conversation-group.delete": shape({ id: required(memoryIdentifier) }),
  "conversation-group.moveChat": shape({ chatId: required(nonblank), groupId: required(nullable(memoryIdentifier)), ownerSelection: required(nullable(nonblank)) }),
  "conversation-group.reorder": shape({ ownerSelection: required(nullable(nonblank)), ids: required(array(memoryIdentifier)) }),
  "memory.searchWithOptions": shape({ ownerKey: required(owner), query: required(text), folderPath: required(nullable(text)), relevanceThreshold: required(nonnegative), createdAtStartMs: required(nullable(integer)), createdAtEndMs: required(nullable(integer)) }),
  "memory.chat.list": ownerOnly,
  "memory.chat.update": shape({ ownerKey: required(owner), chatId: required(nonblank) }),
  "memory.categorize": ownerOnly,
  "memory.rebuild.start": shape({ ownerKey: required(owner), rebuild: required(shape({ chatIds: required(strings), windowMessageCount: required(integer), fromInclusive: required(nullable(integer)), toInclusive: required(nullable(integer)) })) }),
  "memory.rebuild.progress": ownerOnly,
  "memory.rebuild.cancel": ownerOnly,
  "memory.embeddings.rebuild": ownerOnly,
  "memory.candidate.enqueue": shape({ chatId: required(nonblank), timestamp: required(integer), variantIndex: required(messageVariant), sourceType: required(enumeration("reply_finalized_auto", "selected_user_message")), ownerKey: required(nullable(owner)) }),
  "chat.configuration.resolve": parseChatConfigurationRequest,
  "chat.configuration.binding.read": shape({ chatId: required(nonblank) }),
  "chat.configuration.binding.write": shape({ chatId: required(nonblank), selection: required(nonblank) }),
  "chat.configuration.binding.delete": shape({ chatId: required(nonblank) }),
  snapshot: empty,
  "character.list": empty,
  "character.get": id,
  "character.create": shape({ values: required(shape({ ...characterFields, id: optional(text), name: required(nonblank) })) }),
  "character.update": shape({ id: required(nonblank), changes: required(shape(characterFields, true)) }),
  "character.delete": id,
  "character.setActive": id,
  "character.combine": shape({ id: required(nonblank), promptFunctionType: optional(enumeration("CHAT", "VOICE")), additionalTagIds: optional(identifiers) }),
  "character.resetDefault": empty,
  "character.export": shape({ id: required(nonblank), format: required(format) }),
  "character.import": shape({ format: required(format), content: required(nonblank) }),
  "character.exportBackup": empty,
  "character.importBackup": contentOnly,
  "group.list": empty,
  "group.get": id,
  "group.create": shape({ values: required(shape({ ...groupFields, id: optional(text), name: required(nonblank) })) }),
  "group.update": shape({ id: required(nonblank), changes: required(shape(groupFields, true)) }),
  "group.delete": id,
  "group.setActive": id,
  "group.duplicate": shape({ id: required(nonblank), newName: optional(nonblank) }),
  "group.export": id,
  "group.import": contentOnly,
  "group.exportBackup": empty,
  "group.importBackup": contentOnly,
  "activePrompt.get": empty,
  "activePrompt.setCard": id,
  "activePrompt.setGroup": id,
  "activePrompt.activateForChat": shape({ characterCardName: required(nullable(nonblank)), characterGroupId: required(nullable(nonblank)) }),
  "activePrompt.resolvedCard": empty,
  "tag.list": empty,
  "tag.get": id,
  "tag.create": shape({ values: required(shape({ ...tagFields, id: optional(text), name: required(nonblank) })) }),
  "tag.update": shape({ id: required(nonblank), changes: required(shape(tagFields, true)) }),
  "tag.delete": id,
  "memory.shared.list": empty,
  "memory.shared.create": shape({ name: required(nonblank) }),
  "memory.shared.rename": shape({ id: required(nonblank), name: required(nonblank) }),
  "memory.shared.delete": id,
  "memory.mount": shape({ characterId: required(nonblank), sharedId: required(nonblank), readable: required(boolean), writable: required(boolean) }),
  "memory.unmount": shape({ characterId: required(nonblank), sharedId: required(nonblank) }),
  "memory.user.read": ownerOnly,
  "memory.user.write": shape({ ownerKey: required(owner), content: required(text) }),
  "memory.user.path": ownerOnly,
  "memory.resolveOwner": shape({ characterId: required(nonblank) }),
  "memory.settings.read": ownerOnly,
  "memory.settings.write": shape({ ownerKey: required(owner), settings: required(settings) }),
  "memory.searchConfig.read": ownerOnly,
  "memory.searchConfig.write": shape({ ownerKey: required(owner), config: required(searchConfig) }),
  "memory.graph": ownerOnly,
  "memory.list": ownerOnly,
  "memory.search": shape({ ownerKey: required(owner), query: required(nonblank) }),
  "memory.get": shape({ ownerKey: required(owner), title: required(nonblank) }),
  "memory.create": shape({ ownerKey: required(owner), values: required(shape({ ...memoryFields, id: optional(memoryIdentifier), title: required(nonblank), content: required(text) })) }),
  "memory.update": shape({ ownerKey: required(owner), originalTitle: required(nonblank), expectedId: optional(memoryIdentifier), changes: required(shape(memoryFields, true)) }),
  "memory.delete": shape({ ownerKey: required(owner), id: required(memoryIdentifier) }),
  "memory.move": shape({ ownerKey: required(owner), ids: required(array(memoryIdentifier)), folderPath: required(text) }),
  "memory.link.create": shape({ ownerKey: required(owner), sourceTitle: required(nonblank), targetTitle: required(nonblank), linkType: required(nonblank), weight: required(score), description: required(text) }),
  "memory.link.update": shape({ ownerKey: required(owner), linkId: required(memoryIdentifier), changes: required(shape({ linkType: optional(nonblank), weight: optional(score), description: optional(text) }, true)) }),
  "memory.link.delete": shape({ ownerKey: required(owner), linkId: required(memoryIdentifier) }),
  "memory.export": ownerOnly,
  "memory.import": shape({ ownerKey: required(owner), content: required(nonblank), strategy: required(enumeration("SKIP", "UPDATE", "CREATE_NEW")) }),
};

/** Validates one typed public input before it reaches the shared business service. */
export function parseDomainPayload<K extends DomainOperation>(operation: K, payload: unknown): DomainInput<K> {
  if (!Object.prototype.hasOwnProperty.call(validators, operation)) throw new Error(`Unknown character domain operation: ${String(operation)}`);
  validators[operation](payload, operation);
  return payload as DomainInput<K>;
}
/** Parses an explicit JSON object argument while preserving the original parsing error. */
export function parseJsonObject(raw: string, path: string): Record<string, unknown> {
  let value: unknown;
  try { value = JSON.parse(raw); }
  catch (error) { throw new Error(`${path}: invalid JSON: ${error instanceof Error ? error.message : String(error)}`); }
  return record(value, path);
}
/** Validates a character field patch used by the legacy field-update command. */
export function parseCharacterChanges(value: unknown): DomainInput<"character.update">["changes"] {
  shape(characterFields, true)(value, "character changes");
  return value as DomainInput<"character.update">["changes"];
}
/** Validates a group field patch used by the legacy field-update command. */
export function parseGroupChanges(value: unknown): DomainInput<"group.update">["changes"] {
  shape(groupFields, true)(value, "group changes");
  return value as DomainInput<"group.update">["changes"];
}
/** Validates a tag field patch used by the legacy field-update command. */
export function parseTagChanges(value: unknown): DomainInput<"tag.update">["changes"] {
  shape(tagFields, true)(value, "tag changes");
  return value as DomainInput<"tag.update">["changes"];
}
/** Converts a domain result into SDK JSON without dropping unsupported values. */
export function jsonValue(value: unknown, path = "result"): ToolPkg.JsonValue {
  if (value === null || typeof value === "string" || typeof value === "boolean") return value;
  if (typeof value === "number") { number(value, path); return value; }
  if (Array.isArray(value)) {
    /** Validates each returned array element's JSON representation. */
    return value.map((item, index) => jsonValue(item, `${path}[${index}]`));
  }
  const object = record(value, path);
  const result: ToolPkg.JsonObject = {};
  for (const key of Object.keys(object)) result[key] = jsonValue(object[key], `${path}.${key}`);
  return result;
}

/** Exposes only the shared business service operations used by resolved chat configuration. */
export interface ChatConfigurationService {
  /** Reads the real persisted plugin directory. */
  snapshot(): Promise<SnapshotRecord>;
  /** Calls a typed business operation without invoking another command. */
  dispatchDomain<K extends DomainOperation>(operation: K, input: DomainInput<K>): Promise<DomainOutput<K>>;
}

/** Validates every chat configuration field without supplying implicit defaults. */
export function parseChatConfigurationRequest(value: unknown): ChatConfigurationRequest {
  shape({
    purpose: required(enumeration("display", "execution")),
    chatId: required(nullable(nonblank)), chatExtension: required(nullable(markerObject)), messageExtension: required(nullable(markerObject)),
    participantId: required(nullable(nonblank)), promptFunctionType: required(enumeration("CHAT", "VOICE")),
    defaultModelBinding: required(shape({ providerId: required(text), modelId: required(text) })),
    defaultTtsConfigId: required(text),
  })(value, "chat.configuration.resolve");
  const request = value as ChatConfigurationRequest;
  if (request.purpose === "display") {
    if (request.participantId !== null || request.messageExtension !== null) throw new Error("Display configuration cannot request an execution participant or message snapshot");
  } else {
    nonblank(request.defaultModelBinding.providerId, "execution default provider");
    nonblank(request.defaultModelBinding.modelId, "execution default model");
    nonblank(request.defaultTtsConfigId, "execution default TTS configuration");
  }
  return request;
}

/** Requires one exact stored identity and rejects ambiguous or removed configuration. */
function chatRecord<T extends { id: string }>(records: T[], id: string, kind: string): T {
  const matches = records.filter(
    /** Matches the supplied identifier without interpreting its spelling. */
    item => item.id === id,
  );
  if (matches.length !== 1) throw new Error(
    matches.length === 0 ? kind + " not found: " + id : "Duplicate " + kind + ": " + id,
  );
  return matches[0];
}

/** Resolves model, voice, prompt and resource policy exclusively from complete plugin records. */
async function chatParticipant(
  card: CharacterRecord, request: ChatConfigurationRequest, directory: SnapshotRecord, service: ChatConfigurationService,
): Promise<ChatParticipantProfile> {
  nonblank(card.name, "profile.name");
  let modelBinding: ChatConfigurationRequest["defaultModelBinding"];
  switch (card.chatModelBindingMode) {
    case "FOLLOW_GLOBAL": modelBinding = { ...request.defaultModelBinding }; break;
    case "FIXED_MODEL": {
      nonblank(card.chatModelId, "profile.fixedModelId");
      const candidates = directory.models.filter(
        /** Resolves the legacy record's deliberately provider-free model selection exactly. */
        model => model.modelId === card.chatModelId,
      );
      if (candidates.length !== 1) throw new Error(
        candidates.length === 0 ? "Fixed model unavailable: " + card.chatModelId : "Fixed model ambiguous: " + card.chatModelId,
      );
      modelBinding = { providerId: candidates[0].providerId, modelId: candidates[0].modelId };
      break;
    }
    default: throw new Error("Invalid model binding mode: " + card.chatModelBindingMode);
  }
  const resolvedModels = directory.models.filter(
    /** Requires the actual configured provider/model pair. */
    model => model.providerId === modelBinding.providerId && model.modelId === modelBinding.modelId,
  );
  if (resolvedModels.length !== 1) throw new Error("Model binding unavailable or duplicated: " + modelBinding.providerId + "/" + modelBinding.modelId);
  const ttsConfigId = card.ttsConfigId === null ? request.defaultTtsConfigId : card.ttsConfigId;
  nonblank(ttsConfigId, "profile.ttsConfigId");
  chatRecord(directory.ttsConfigs, ttsConfigId, "TTS configuration");
  let primary: string;
  switch (card.memoryBindingMode) {
    case "CHARACTER": primary = "character:" + card.id; break;
    case "SHARED":
      nonblank(card.sharedMemoryId, "profile.sharedMemoryId");
      chatRecord(directory.stores, card.sharedMemoryId as string, "shared memory");
      primary = "shared:" + card.sharedMemoryId;
      break;
    default: throw new Error("Invalid memory binding mode: " + card.memoryBindingMode);
  }
  const routes = new Map<string, { key: string; readable: boolean; writable: boolean }>();
  routes.set(primary, { key: primary, readable: true, writable: true });
  for (const mount of card.sharedMemoryMounts) {
    chatRecord(directory.stores, mount.sharedMemoryId, "mounted shared memory");
    const key = "shared:" + mount.sharedMemoryId;
    const existing = routes.get(key);
    routes.set(key, { key, readable: mount.readable || existing?.readable === true, writable: mount.writable || existing?.writable === true });
  }
  const prompt = await service.dispatchDomain("character.combine", { id: card.id, promptFunctionType: request.promptFunctionType, additionalTagIds: [] });
  const document = await service.dispatchDomain("memory.user.read", { ownerKey: primary });
  return {
    id: card.id, name: card.name, avatarUri: card.avatarUri,
    introPrompt: prompt.prompt, userPreferencesText: document.content,
    openingStatement: card.openingStatement, modelBinding, ttsConfigId,
    toolAccess: { ...card.toolAccessConfig,
      allowedBuiltinTools: [...card.toolAccessConfig.allowedBuiltinTools], allowedPackages: [...card.toolAccessConfig.allowedPackages],
      allowedSkills: [...card.toolAccessConfig.allowedSkills], allowedMcpServers: [...card.toolAccessConfig.allowedMcpServers],
    },
    resources: [...routes.values()],
  };
}

/** Resolves an explicit execution participant without moving domain selection rules into Core. */
export async function resolveChatConfiguration(request: ChatConfigurationRequest, service: ChatConfigurationService): Promise<ChatConfigurationResult> {
  if (request.purpose === "display") return resolveChatDisplay(request, service, resolveChatExecution);
  return resolveChatExecution(request, service);
}

/** Resolves only an explicitly requested execution or its exact saved historical message snapshot. */
export async function resolveChatExecution(request: ChatConfigurationRequest, service: ChatConfigurationService): Promise<ChatConfigurationExecutionResult> {
  if (request.purpose !== "execution") throw new Error("Execution configuration requires purpose execution");
  if (request.messageExtension !== null) {
    const marker = decodeMessageMarker(request.messageExtension);
    if (request.participantId !== null && request.participantId !== marker.profile.id) throw new Error("Historical execution participant does not match the saved message snapshot");
    return { contextKey: marker.selection, profile: marker.profile, participants: marker.participants, messageExtension: markerObject(request.messageExtension, "historical message extension") };
  }
  const marker = decodeChatMarker(request.chatExtension);
  const directory = await service.snapshot();
  const selection = requireChatSelection(marker.selection, directory.cards, directory.groups);
  let cards: CharacterRecord[];
  if (selection.kind === "card") cards = [chatRecord(directory.cards, selection.id, "character")];
  else {
    const group = chatRecord(directory.groups, selection.id, "group");
    const members = [...group.members].sort(
      /** Preserves the persisted participant order without planning turns in Core. */
      (left, right) => left.orderIndex - right.orderIndex,
    );
    if (members.length === 0) throw new Error("Selected group has no participants: " + group.id);
    const ids = new Set<string>();
    cards = members.map(
      /** Rejects duplicate or deleted group members before producing any execution profile. */
      member => {
        if (ids.has(member.characterCardId)) throw new Error("Duplicate group participant: " + member.characterCardId);
        ids.add(member.characterCardId);
        return chatRecord(directory.cards, member.characterCardId, "group participant");
      },
    );
  }
  if (request.participantId === null && cards.length !== 1) throw new Error("A planned conversation requires an explicit execution participant");
  if (request.participantId !== null) chatRecord(cards, request.participantId, "selected participant");
  const participants: ChatParticipantProfile[] = [];
  for (const card of cards) participants.push(await chatParticipant(card, request, directory, service));
  let profile: ChatParticipantProfile;
  if (request.participantId !== null) profile = chatRecord(participants, request.participantId, "selected participant");
  else {
    if (participants.length !== 1) throw new Error("A planned conversation requires an explicit execution participant");
    profile = participants[0];
  }
  const owner = await service.dispatchDomain("memory.resolveOwner", { characterId: profile.id });
  const primaryOwnerKey = (owner.kind === "CHARACTER" ? "character:" : "shared:") + owner.id;
  const messageExtension = encodeMessageMarker({ version: 1, selection: marker.selection, promptFunctionType: request.promptFunctionType, primaryOwnerKey, profile, participants });
  return { contextKey: marker.selection, profile, participants, messageExtension };
}

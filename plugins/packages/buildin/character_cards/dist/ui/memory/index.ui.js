"use strict";
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, { get: all[name], enumerable: true });
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);

// src/ui/memory/index.ui.ts
var index_ui_exports = {};
__export(index_ui_exports, {
  default: () => Screen
});
module.exports = __toCommonJS(index_ui_exports);

// src/model-binding.ts
function characterModelBindingMode(mode) {
  return mode === "FIXED_CONFIG" ? "FIXED_MODEL" : mode;
}
function sameCharacterModelBinding(left, right) {
  return left.chatModelBindingMode === right.chatModelBindingMode && left.chatProviderId === right.chatProviderId && left.chatModelId === right.chatModelId;
}
function characterModelBindingStatus(card, models) {
  switch (characterModelBindingMode(card.chatModelBindingMode)) {
    case "FOLLOW_GLOBAL":
      return { needsRebind: false, label: "跟随全局模型" };
    case "FIXED_MODEL": {
      const matches = models.filter(
        /** Matches the exact provider and model identities selected by the user. */
        (model) => model.providerId === card.chatProviderId && model.modelId === card.chatModelId
      );
      if (matches.length !== 1) return { needsRebind: true, label: "固定模型绑定无效，请重新绑定" };
      return { needsRebind: false, label: matches[0].providerName + " · " + matches[0].modelId };
    }
    default:
      return { needsRebind: true, label: "模型绑定模式无效，请重新绑定" };
  }
}

// src/validation.ts
function assertObject(value, label) {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new Error(`${label} 必须是对象`);
}
function assertString(value, label) {
  if (typeof value !== "string") throw new Error(`${label} 必须是字符串`);
}
function assertBoolean(value, label) {
  if (typeof value !== "boolean") throw new Error(`${label} 必须是布尔值`);
}
function assertNumber(value, label, min, max) {
  if (typeof value !== "number" || !Number.isFinite(value) || value < min || value > max) throw new Error(`${label} 必须在 ${min} 到 ${max} 之间`);
}
function assertInteger(value, label, min) {
  assertNumber(value, label, min, Number.MAX_SAFE_INTEGER);
  if (!Number.isSafeInteger(value)) throw new Error(`${label} 必须是安全整数`);
}
function requireId(value, label) {
  assertString(value, label);
  if (value.trim() === "" || value !== value.trim()) throw new Error(`${label} 无效`);
  return value;
}
function assertMemorySettings(value) {
  assertObject(value, "记忆设置");
  assertInteger(value.autoSaveIntervalMinutes, "自动保存间隔", 1);
  if (value.autoSaveIntervalMinutes > 30) throw new Error("自动保存间隔必须在 1 到 30 分钟之间");
  assertInteger(value.nextAutoSaveRunAtMs, "下次自动保存时间", 0);
  for (const field of ["memoryExtractionCustomRules", "cloudEmbeddingEndpoint", "cloudEmbeddingApiKey", "cloudEmbeddingModel"]) assertString(value[field], field);
  for (const field of ["profileAutoUpdateEnabled", "profileAutoUpdateLocked", "cloudEmbeddingEnabled"]) assertBoolean(value[field], field);
}
function assertSearchConfig(value) {
  assertObject(value, "记忆搜索设置");
  if (value.scoreMode !== "BALANCED" && value.scoreMode !== "KEYWORD_FIRST" && value.scoreMode !== "SEMANTIC_FIRST") throw new Error("记忆评分模式无效");
  for (const field of ["keywordWeight", "tagWeight", "vectorWeight", "edgeWeight"]) assertNumber(value[field], field, 0, Number.MAX_VALUE);
}

// src/chat-bindings.ts
function parseChatSelection(selection2) {
  requireId(selection2, "chat selection");
  const parsed = /^(card|group|model):([^\s]+)$/u.exec(selection2);
  if (parsed === null) throw new Error("Invalid plugin chat selection: " + selection2);
  const kind = parsed[1];
  if (kind === "model" && parsed[2] !== "chat") throw new Error("Invalid model selection: " + selection2);
  if (kind !== "card" && kind !== "group" && kind !== "model") throw new Error("Invalid chat selection kind: " + kind);
  return { kind, id: parsed[2] };
}
function selectionForActive(active) {
  if ("Model" in active) return "model:chat";
  if ("CharacterCard" in active) return "card:" + active.CharacterCard.id;
  return "group:" + active.CharacterGroup.id;
}

// src/chat-markers.ts
function markerValue(value, path) {
  if (value === null || typeof value === "string" || typeof value === "boolean") return value;
  if (typeof value === "number") {
    if (!Number.isFinite(value)) throw new Error(path + " must be a finite JSON number");
    return value;
  }
  if (Array.isArray(value)) return value.map(
    /** Validates and copies every actual nested JSON value. */
    (item, index) => markerValue(item, path + "[" + index + "]")
  );
  return markerObject(value, path);
}
function markerObject(value, path) {
  assertObject(value, path);
  const result = {};
  for (const [key, item] of Object.entries(value)) Object.defineProperty(result, key, { value: markerValue(item, path + "." + key), enumerable: true, configurable: true, writable: true });
  return result;
}

// src/domain.ts
function record(value, path) {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new Error(`${path} must be a JSON object`);
  return value;
}
function text(value, path) {
  if (typeof value !== "string") throw new Error(`${path} must be a string`);
}
function nonblank(value, path) {
  text(value, path);
  if (value.trim() === "") throw new Error(`${path} must not be blank`);
}
function parseMemoryIdentifier(value, path, _positive = true) {
  if (typeof value !== "string" || value.trim() === "" || value !== value.trim() || value.length > 256) throw new Error(`${path} must be a nonblank identity`);
  return value;
}
function memoryIdentifier(value, path) {
  parseMemoryIdentifier(value, path);
}
function propertyIdentifier(value, path) {
  parseMemoryIdentifier(value, path, false);
}
function boolean(value, path) {
  if (typeof value !== "boolean") throw new Error(`${path} must be a boolean`);
}
function number(value, path) {
  if (typeof value !== "number" || !Number.isFinite(value)) throw new Error(`${path} must be a finite number`);
}
function integer(value, path) {
  if (typeof value !== "number" || !Number.isSafeInteger(value)) throw new Error(`${path} must be a safe integer`);
}
function nonnegativeInteger(value, path) {
  integer(value, path);
  nonnegative(value, path);
}
function messageVariant(value, path) {
  integer(value, path);
  if (typeof value !== "number" || value < 0 || value > 2147483647) throw new Error(path + " must be a nonnegative i32 message variant index");
}
function score(value, path) {
  number(value, path);
  if (value < 0 || value > 1) throw new Error(`${path} must be between 0 and 1`);
}
function nonnegative(value, path) {
  number(value, path);
  if (value < 0) throw new Error(`${path} must not be negative`);
}
function enumeration(...values) {
  const allowed = new Set(values);
  return (value, path) => {
    if (typeof value !== "string" || !allowed.has(value)) throw new Error(`${path} must be ${values.join(" | ")}`);
  };
}
function nullable(validate) {
  return (value, path) => {
    if (value !== null) validate(value, path);
  };
}
function array(validate) {
  return (value, path) => {
    if (!Array.isArray(value)) throw new Error(`${path} must be an array`);
    for (let index = 0; index < value.length; index += 1) validate(value[index], `${path}[${index}]`);
  };
}
function required(validate) {
  return { validate, required: true };
}
function optional(validate) {
  return { validate, required: false };
}
function shape(fields2, nonempty = false) {
  return (value, path) => {
    const object = record(value, path);
    const keys = Object.keys(object);
    if (nonempty && keys.length === 0) throw new Error(`${path} must contain at least one field`);
    for (const key of keys) {
      if (!Object.prototype.hasOwnProperty.call(fields2, key)) throw new Error(`${path}.${key} is not a supported field`);
      fields2[key].validate(object[key], `${path}.${key}`);
    }
    for (const key of Object.keys(fields2)) {
      if (fields2[key].required && !Object.prototype.hasOwnProperty.call(object, key)) throw new Error(`${path}.${key} is required`);
    }
  };
}
function owner(value, path) {
  if (typeof value !== "string" || !/^(character|shared):[^:\s]+$/.test(value)) throw new Error(`${path} must be character:<id> or shared:<id>`);
}
var strings = array(text);
var identifiers = array(nonblank);
var member = shape({ characterCardId: required(nonblank), orderIndex: required(integer) });
var mount = shape({ sharedMemoryId: required(nonblank), readable: required(boolean), writable: required(boolean) });
var toolAccess = shape({ enabled: required(boolean), allowedBuiltinTools: required(strings), allowedPackages: required(strings), allowedSkills: required(strings), allowedMcpServers: required(strings) });
var property = shape({ id: required(propertyIdentifier), key: required(text), value: required(text) });
var characterFields = {
  name: optional(nonblank),
  description: optional(text),
  characterSetting: optional(text),
  openingStatement: optional(text),
  otherContentChat: optional(text),
  otherContentVoice: optional(text),
  avatarUri: optional(nullable(text)),
  attachedTagIds: optional(identifiers),
  advancedCustomPrompt: optional(text),
  marks: optional(text),
  chatModelBindingMode: optional(enumeration("FOLLOW_GLOBAL", "FIXED_MODEL")),
  chatModelId: optional(nullable(text)),
  chatProviderId: optional(nullable(text)),
  ttsConfigId: optional(nullable(text)),
  themeConfigId: optional(nullable(text)),
  memoryBindingMode: optional(enumeration("CHARACTER", "SHARED")),
  sharedMemoryId: optional(nullable(text)),
  sharedMemoryMounts: optional(array(mount)),
  toolAccessConfig: optional(toolAccess),
  createdAt: optional(integer),
  updatedAt: optional(integer)
};
var groupFields = { name: optional(nonblank), description: optional(text), themeConfigId: optional(nullable(text)), members: optional(array(member)), createdAt: optional(integer), updatedAt: optional(integer) };
var tagFields = { name: optional(nonblank), description: optional(text), promptContent: optional(text), tagType: optional(enumeration("TONE", "CHARACTER", "FUNCTION", "CUSTOM")), createdAt: optional(integer), updatedAt: optional(integer) };
var memoryFields = {
  uuid: optional(text),
  title: optional(nonblank),
  content: optional(text),
  contentType: optional(text),
  source: optional(text),
  credibility: optional(score),
  importance: optional(score),
  documentPath: optional(nullable(text)),
  isDocumentNode: optional(boolean),
  chunkIndexFilePath: optional(nullable(text)),
  folderPath: optional(nullable(text)),
  createdAt: optional(integer),
  updatedAt: optional(integer),
  lastAccessedAt: optional(integer),
  tags: optional(strings),
  properties: optional(array(property))
};
var settings = shape({
  autoSaveIntervalMinutes: required(nonnegative),
  nextAutoSaveRunAtMs: required(integer),
  memoryExtractionCustomRules: required(text),
  profileAutoUpdateEnabled: required(boolean),
  profileAutoUpdateLocked: required(boolean),
  cloudEmbeddingEnabled: required(boolean),
  cloudEmbeddingEndpoint: required(text),
  cloudEmbeddingApiKey: required(text),
  cloudEmbeddingModel: required(text)
});
var searchConfig = shape({ scoreMode: required(enumeration("BALANCED", "KEYWORD_FIRST", "SEMANTIC_FIRST")), keywordWeight: required(nonnegative), tagWeight: required(nonnegative), vectorWeight: required(nonnegative), edgeWeight: required(nonnegative) });
var empty = shape({});
var id = shape({ id: required(nonblank) });
var ownerOnly = shape({ ownerKey: required(owner) });
var contentOnly = shape({ content: required(text) });
var format = enumeration("operit", "tavern");
var memoryQueryRequest = shape({ participantId: required(nonblank), query: required(nonblank), limit: required(nonnegative), snapshotId: required(nullable(nonblank)) });
var validators = {
  "memory.chunks": shape({ ownerKey: required(owner), memoryId: required(nonblank), query: required(nonblank), count: required(nonnegativeInteger) }),
  "memory.searchWithOptions": shape({ limit: optional(integer), excludeIds: optional(strings), ownerKey: required(owner), query: required(nonblank), folderPath: required(nullable(text)), relevanceThreshold: required(nonnegative), createdAtStartMs: required(nullable(integer)), createdAtEndMs: required(nullable(integer)) }),
  "memory.chat.list": ownerOnly,
  "memory.chat.pruneMissing": ownerOnly,
  "memory.chat.update": shape({ ownerKey: required(owner), chatId: required(nonblank) }),
  "memory.categorize": ownerOnly,
  "memory.rebuild.start": shape({ ownerKey: required(owner), rebuild: required(shape({ chatIds: required(strings), windowMessageCount: required(integer), fromInclusive: required(nullable(integer)), toInclusive: required(nullable(integer)) })) }),
  "memory.rebuild.progress": ownerOnly,
  "memory.rebuild.cancel": ownerOnly,
  "memory.embeddings.rebuild": ownerOnly,
  "memory.candidate.enqueue": shape({ chatId: required(nonblank), timestamp: required(integer), variantIndex: required(messageVariant), sourceType: required(enumeration("reply_finalized_auto", "selected_user_message")), ownerKey: required(nullable(owner)) }),
  "memory.candidates.retryFailed": ownerOnly,
  "memory.candidates.clearFailed": ownerOnly,
  "chat.configuration.resolve": parseChatConfigurationRequest,
  "chat.configuration.binding.find": shape({ chatId: required(nonblank) }),
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
  "activePrompt.setModel": empty,
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
  "memory.import": shape({ ownerKey: required(owner), content: required(nonblank), strategy: required(enumeration("SKIP", "UPDATE", "CREATE_NEW")) })
};
function parseDomainPayload(operation, payload) {
  if (!Object.prototype.hasOwnProperty.call(validators, operation)) throw new Error(`Unknown character domain operation: ${String(operation)}`);
  validators[operation](payload, operation);
  return payload;
}
function parseChatConfigurationRequest(value) {
  shape({
    purpose: required(enumeration("display", "execution")),
    chatId: required(nullable(nonblank)),
    chatExtension: required(nullable(markerObject)),
    participantId: required(nullable(nonblank)),
    promptFunctionType: required(enumeration("CHAT", "VOICE")),
    defaultModelBinding: required(shape({ providerId: required(text), modelId: required(text) })),
    defaultTtsConfigId: required(text)
  })(value, "chat.configuration.resolve");
  const request2 = value;
  if (request2.purpose === "display") {
    if (request2.participantId !== null) throw new Error("Display configuration cannot request an execution participant");
  } else {
    nonblank(request2.defaultModelBinding.providerId, "execution default provider");
    nonblank(request2.defaultModelBinding.modelId, "execution default model");
    nonblank(request2.defaultTtsConfigId, "execution default TTS configuration");
  }
  return request2;
}

// src/presentation.ts
function fields(value, keys, path) {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new Error(`${path} must be an object`);
  const object = value, actual = Object.keys(object);
  if (actual.length !== keys.length || actual.some(
    /** Compares exact field identities rather than interpreting string fragments. */
    (key) => keys.indexOf(key) < 0
  )) throw new Error(`${path} must have exactly ${keys.join(", ")}`);
  return object;
}
function text2(value, path) {
  if (typeof value !== "string") throw new Error(`${path} must be a string`);
  return value;
}
function identity(value, path) {
  const result = text2(value, path);
  if (result.trim() === "") throw new Error(`${path} must not be blank`);
  return result;
}
function entityKind(value) {
  if (value !== "card" && value !== "group") throw new Error("screen.entity must be card or group");
  return value;
}
function selection(value) {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new Error("screen.selection must be an active prompt object");
  const keys = Object.keys(value);
  if (keys.length !== 1) throw new Error("screen.selection must have one active prompt discriminator");
  const object = value;
  switch (keys[0]) {
    case "Model": {
      const model = fields(object.Model, ["id"], "screen.selection.Model");
      if (model.id !== "chat") throw new Error("Invalid model selection identity");
      return { Model: { id: "chat" } };
    }
    case "CharacterCard":
      return { CharacterCard: { id: identity(fields(object.CharacterCard, ["id"], "screen.selection.CharacterCard").id, "screen.selection.id") } };
    case "CharacterGroup":
      return { CharacterGroup: { id: identity(fields(object.CharacterGroup, ["id"], "screen.selection.CharacterGroup").id, "screen.selection.id") } };
    default:
      throw new Error("screen.selection has an unknown active prompt discriminator");
  }
}
function parseScreenInput(value) {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new Error("screen.input must be an object");
  const object = value;
  switch (object.mode) {
    case "manage": {
      if (object.view === void 0) {
        fields(object, ["mode"], "screen.input");
        return { mode: "manage" };
      }
      fields(object, ["mode", "view"], "screen.input");
      if (object.view !== "characters" && object.view !== "memory") throw new Error("screen.view must be characters or memory");
      return { mode: "manage", view: object.view };
    }
    case "select": {
      fields(object, ["mode", "kind", "selected"], "screen.input");
      if (object.kind !== "card" && object.kind !== "group" && object.kind !== "all") throw new Error("screen.kind must be card, group, or all");
      const selected = object.selected === null ? null : selection(object.selected);
      if (selected !== null && (object.kind === "card" && !("CharacterCard" in selected) && !("Model" in selected) || object.kind === "group" && !("CharacterGroup" in selected))) throw new Error("screen.selected does not match screen.kind");
      return { mode: "select", kind: object.kind, selected };
    }
    case "preview":
    case "edit":
      fields(object, ["mode", "entity", "id"], "screen.input");
      return { mode: object.mode, entity: entityKind(object.entity), id: identity(object.id, "screen.id") };
    case "memory-attachment": {
      fields(object, ["mode", "ownerKey", "folderPath"], "screen.input");
      const ownerKey = object.ownerKey === null ? null : identity(object.ownerKey, "screen.ownerKey");
      const folderPath = object.folderPath === null ? null : text2(object.folderPath, "screen.folderPath");
      if (ownerKey === null && folderPath !== null) throw new Error("screen.folderPath requires an explicitly selected ownerKey");
      return { mode: "memory-attachment", ownerKey, folderPath };
    }
    default:
      throw new Error("screen.input has an unknown mode");
  }
}
function parseScreenResult(value, input) {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new Error("screen.result must be an object");
  const object = value;
  if (object.mode !== input.mode) throw new Error("screen.result.mode does not match the active presentation");
  switch (input.mode) {
    case "select": {
      fields(object, ["mode", "selection"], "screen.result");
      const result = selection(object.selection);
      if (input.kind === "card" && !("CharacterCard" in result) && !("Model" in result) || input.kind === "group" && !("CharacterGroup" in result)) throw new Error("screen.result.selection does not match screen.kind");
      return { mode: "select", selection: result };
    }
    case "preview":
    case "edit": {
      fields(object, input.mode === "edit" ? ["mode", "entity", "id", "operation"] : ["mode", "entity", "id"], "screen.result");
      if (object.entity !== input.entity || object.id !== input.id) throw new Error("screen.result does not match the presented entity");
      if (input.mode === "preview") return { mode: "preview", entity: input.entity, id: input.id };
      if (object.operation !== "saved" && object.operation !== "deleted") throw new Error("screen.result.operation must be saved or deleted");
      return { mode: "edit", entity: input.entity, id: input.id, operation: object.operation };
    }
    case "memory-attachment": {
      fields(object, ["mode", "ownerKey", "folderPath", "content"], "screen.result");
      const ownerKey = identity(object.ownerKey, "screen.result.ownerKey"), folderPath = text2(object.folderPath, "screen.result.folderPath");
      if (input.ownerKey !== null && ownerKey !== input.ownerKey || input.folderPath !== null && folderPath !== input.folderPath) throw new Error("screen.result does not match the presented attachment scope");
      return { mode: "memory-attachment", ownerKey, folderPath, content: text2(object.content, "screen.result.content") };
    }
  }
}
function createScreenSession(presentation) {
  let input, requestId;
  if (presentation === null) {
    input = { mode: "manage" };
    requestId = null;
  } else {
    const envelope = fields(presentation, ["requestId", "input"], "presentation");
    requestId = identity(envelope.requestId, "presentation.requestId");
    input = parseScreenInput(envelope.input);
    if (input.mode === "manage") throw new Error("The management route must not have a presentation request");
  }
  let finished = false;
  function activeRequest() {
    if (requestId === null || input.mode === "manage") throw new Error("The management route has no presentation result channel");
    if (finished) throw new Error(`Presentation ${requestId} has already finished`);
    return { requestId, input };
  }
  return {
    /** Copies screen parameters so WebView consumers cannot mutate this session's contract. */
    currentScreen() {
      return { requestId, input: parseScreenInput(input) };
    },
    /** Returns the explicit generic completion discriminator only after validating this result. */
    completeScreen(value) {
      const active = activeRequest(), result = parseScreenResult(value, active.input);
      finished = true;
      return { type: "toolpkg.presentation.complete", requestId: active.requestId, value: result };
    },
    /** Returns the explicit cancellation discriminator without a synthetic domain result. */
    cancelScreen() {
      const active = activeRequest();
      finished = true;
      return { type: "toolpkg.presentation.cancel", requestId: active.requestId };
    }
  };
}

// src/drafts.ts
function createCharacterDraft() {
  return {
    id: "",
    name: "",
    description: "",
    characterSetting: "",
    openingStatement: "",
    otherContentChat: "",
    otherContentVoice: "",
    avatarUri: null,
    attachedTagIds: [],
    advancedCustomPrompt: "",
    marks: "",
    chatModelBindingMode: "FOLLOW_GLOBAL",
    chatModelId: null,
    chatProviderId: null,
    ttsConfigId: null,
    themeConfigId: null,
    memoryBindingMode: "CHARACTER",
    sharedMemoryId: null,
    sharedMemoryMounts: [],
    toolAccessConfig: { enabled: false, allowedBuiltinTools: [], allowedPackages: [], allowedSkills: [], allowedMcpServers: [] },
    createdAt: 0,
    updatedAt: 0
  };
}
function createGroupDraft() {
  return { id: "", name: "", description: "", members: [], themeConfigId: null, createdAt: 0, updatedAt: 0 };
}

// src/memory-jobs/scheduler.ts
var CANDIDATE_PROCESSING_LEASE_MS = 30 * 60 * 1e3;

// src/definition.ts
var definition = { id: "com.operit.character_cards", title: "角色卡", icon: "Badge", order: 150 };
var chatUiRoutes = {
  editor: `toolpkg:${definition.id}:ui:main`,
  selection: `toolpkg:${definition.id}:ui:selection`,
  execution: `toolpkg:${definition.id}:ui:group-execution`
};

// src/ui-contributions.ts
function callMainDomain(operation, input) {
  const checked = parseDomainPayload(operation, input);
  return ToolPkg.ipc.call("character-memory.domain", { operation, input: checked }, { targetRuntime: "main" });
}
function chatIdentity(value, label) {
  if (typeof value !== "string" || value.trim() === "") throw new Error(label + " must be a nonblank string");
  return value;
}
function encodeSelection(selection2) {
  return selectionForActive(selection2);
}
function createUiScreenSession(presentation) {
  let selectorTarget = null;
  let checked = presentation;
  if (presentation !== null && presentation.input.mode === "select") {
    const { chatId, ...screenInput } = presentation.input;
    if (chatId !== null) chatIdentity(chatId, "presentation.input.chatId");
    selectorTarget = { chatId };
    checked = { requestId: presentation.requestId, input: screenInput };
  }
  const session = createScreenSession(checked);
  let completing = false, finished = false;
  return {
    /** Reads only the package screen parameters needed by the reused typed Web view. */
    currentScreen: () => session.currentScreen(),
    /** Persists a selector choice through main IPC before acknowledging its caller-owned request. */
    async completeScreen(value) {
      if (finished) throw new Error("This presentation has already finished");
      if (completing) throw new Error("A presentation completion is already running");
      const current = session.currentScreen();
      if (current.input.mode === "manage") throw new Error("Management has no presentation completion channel");
      const result = parseScreenResult(value, current.input);
      completing = true;
      try {
        let completion;
        switch (result.mode) {
          case "select": {
            if (selectorTarget === null) throw new Error("Selector has no explicit global or chat target");
            const selection2 = encodeSelection(result.selection);
            if (selectorTarget.chatId === null) {
              const identity2 = parseChatSelection(selection2);
              const active = identity2.kind === "model" ? await callMainDomain("activePrompt.setModel", {}) : identity2.kind === "card" ? await callMainDomain("activePrompt.setCard", { id: identity2.id }) : await callMainDomain("activePrompt.setGroup", { id: identity2.id });
              if (!active.active || active.id !== identity2.id || active.type !== (identity2.kind === "model" ? "model" : identity2.kind === "card" ? "character_card" : "character_group")) throw new Error("Main runtime did not confirm the selected global active prompt");
            } else {
              const chatId = selectorTarget.chatId;
              const binding = await callMainDomain("chat.configuration.binding.write", { chatId, selection: selection2 });
              if (binding.chatId !== chatId || binding.selection !== selection2) throw new Error("Main runtime did not confirm the selected chat binding");
            }
            const complete = session.completeScreen(result);
            completion = { ...complete, value: { selection: selection2, contextKey: selection2 } };
            break;
          }
          case "memory-attachment": {
            const complete = session.completeScreen(result);
            completion = { ...complete, value: { type: "text", name: "记忆附件", content: result.content, mediaType: "text/plain" } };
            break;
          }
          case "preview":
          case "edit":
            completion = session.completeScreen(result);
            break;
        }
        finished = true;
        return completion;
      } finally {
        completing = false;
      }
    },
    /** Cancels the same caller request without invoking any binding write or returning domain data. */
    cancelScreen() {
      if (finished) throw new Error("This presentation has already finished");
      if (completing) throw new Error("Cannot cancel during a presentation commit");
      const cancellation = session.cancelScreen();
      finished = true;
      return cancellation;
    }
  };
}

// src/image-source.ts
var MAX_AVATAR_BYTES = 8 * 1024 * 1024;
function readAvatarImport(file) {
  if (!Number.isSafeInteger(file.size) || file.size < 0) throw new Error("头像图片大小无效");
  if (file.size > MAX_AVATAR_BYTES) throw new Error("头像图片不能超过 8 MB");
  const b = file.contentBase64;
  if (typeof b !== "string") throw new Error("选择器没有返回头像图片内容");
  const padding = b.endsWith("==") ? 2 : b.endsWith("=") ? 1 : 0;
  if (b.length % 4 !== 0 || b.length / 4 * 3 - padding !== file.size || !/^[A-Za-z0-9+/]*={0,2}$/.test(b)) throw new Error("头像图片内容不完整");
  if (!(b.startsWith("iVBOR") || b.startsWith("/9j/") || b.startsWith("R0lGOD") || b.startsWith("UklGR") || b.startsWith("Qk"))) {
    throw new Error("请选择 PNG、JPEG、GIF、WebP 或 BMP 图片");
  }
  return b;
}

// src/ui-entity-preview.ts
function renderEntityPreview(ctx, input) {
  const colors = ctx.MaterialTheme.colorScheme;
  const entity = input.card ?? input.group;
  if (entity === null) return [];
  const card = input.card;
  const selectable = (key, value, subdued = false) => ctx.UI.SelectionContainer({ key }, ctx.UI.Text({ text: value, fillMaxWidth: true, style: "bodyMedium", color: subdued ? colors.onSurfaceVariant : colors.onSurface }));
  const pill = (key, label) => ctx.UI.Box({
    key,
    paddingHorizontal: 10,
    paddingVertical: 5,
    backgroundColor: colors.primaryContainer.copy({ alpha: 0.38 }),
    backgroundShape: { cornerRadius: 8 }
  }, ctx.UI.Text({ text: label, style: "labelMedium", color: colors.onPrimaryContainer }));
  const attachedTags = card === null ? [] : input.tags.filter((tag) => card.attachedTagIds.includes(tag.id));
  const identity2 = ctx.UI.Column({ key: "entity-preview-identity", fillMaxWidth: true, spacing: 14, paddingBottom: 22 }, [
    ctx.UI.Row({ fillMaxWidth: true, spacing: 16, verticalAlignment: "center" }, [
      card === null ? ctx.UI.Box({ width: 72, height: 72, contentAlignment: "center", backgroundColor: colors.primaryContainer.copy({ alpha: 0.48 }), backgroundShape: { cornerRadius: 20 } }, ctx.UI.Icon({ name: "GroupsOutlined", size: 34, tint: colors.primary })) : ctx.UI.Image({
        key: "entity-avatar",
        ...card.avatarUri === null ? { path: input.avatar ?? void 0 } : { uri: card.avatarUri },
        width: 72,
        height: 72,
        contentScale: "crop",
        contentDescription: card.name,
        modifier: ctx.Modifier.clip({ cornerRadius: 20 })
      }),
      ctx.UI.Column({ weight: 1, spacing: 6 }, [
        ctx.UI.Text({ key: "entity-preview-name", text: entity.name, style: "headlineSmall", fontSize: 24, fontWeight: "700", fillMaxWidth: true }),
        ...card === null ? [ctx.UI.Text({ text: input.group.members.length + " 位角色", style: "labelMedium", color: colors.onSurfaceVariant })] : []
      ])
    ]),
    ...entity.description.trim() === "" ? [] : [selectable("entity-preview-description", entity.description, true)],
    ...attachedTags.length === 0 ? [] : [ctx.UI.FlowRow({ key: "entity-preview-tags", spacing: 6, runSpacing: 6 }, attachedTags.map((tag) => pill("entity-preview-tag-" + tag.id, tag.name)))]
  ]);
  const contentSection = (key, title, value, quote = false) => value.trim() === "" ? [] : [
    ctx.UI.Box(
      { key: "entity-preview-" + key, fillMaxWidth: true, paddingBottom: 22 },
      ctx.UI.Column({
        fillMaxWidth: true,
        spacing: 10,
        ...quote ? { padding: 18, backgroundColor: colors.primaryContainer.copy({ alpha: 0.22 }), backgroundShape: { cornerRadius: 16 } } : {}
      }, [
        ctx.UI.Row({ fillMaxWidth: true, spacing: 8, verticalAlignment: "center" }, [
          ...quote ? [ctx.UI.Icon({ name: "FormatQuote", size: 20, tint: colors.primary })] : [],
          ctx.UI.Text({ text: title, style: "titleSmall", fontWeight: "600", color: quote ? colors.primary : colors.onSurfaceVariant, weight: 1 })
        ]),
        selectable("entity-preview-" + key + "-text", value)
      ])
    )
  ];
  const themeLabel = entity.themeConfigId === null ? "跟随当前主题" : input.themeError !== "" ? "主题名称暂时无法读取" : input.themes === null ? "正在读取主题名称…" : input.themes.find((theme) => theme.id === entity.themeConfigId)?.label ?? "主题配置已不可用";
  const rows = [];
  const contents = [identity2];
  if (card !== null) {
    contents.push(
      ...contentSection("opening", "开场白", card.openingStatement, true),
      ...contentSection("setting", "角色设定", card.characterSetting),
      ...contentSection("chat", "聊天附加内容", card.otherContentChat),
      ...contentSection("voice", "语音附加内容", card.otherContentVoice),
      ...contentSection("prompt", "高级自定义 Prompt", card.advancedCustomPrompt),
      ...contentSection("notes", "备注", card.marks)
    );
    rows.push(
      { key: "model", label: "聊天模型", value: characterModelBindingStatus(card, input.snapshot.models).label },
      { key: "theme", label: "主题", value: themeLabel },
      { key: "tts", label: "语音", value: card.ttsConfigId === null ? "跟随全局语音配置" : input.snapshot.ttsConfigs.find((config) => config.id === card.ttsConfigId)?.name ?? "语音配置已不可用" },
      { key: "memory", label: "记忆库", value: card.memoryBindingMode === "SHARED" ? input.snapshot.stores.find((store) => store.id === card.sharedMemoryId)?.name ?? "共享记忆库已不可用" : "角色独立记忆库" }
    );
    if (card.sharedMemoryMounts.length !== 0) rows.push({ key: "mounts", label: "共享记忆挂载", value: card.sharedMemoryMounts.map((mount2) => {
      const name = input.snapshot.stores.find((store) => store.id === mount2.sharedMemoryId)?.name ?? "记忆库已不可用";
      const permissions = [mount2.readable ? "可读" : "", mount2.writable ? "可写" : ""].filter(Boolean).join("、") || "无访问权限";
      return name + "（" + permissions + "）";
    }).join("\n") });
    const policy = card.toolAccessConfig;
    const toolNames = [
      { ids: policy.allowedBuiltinTools, catalog: input.snapshot.toolCatalog.builtinTools },
      { ids: policy.allowedPackages, catalog: input.snapshot.toolCatalog.packages },
      { ids: policy.allowedSkills, catalog: input.snapshot.toolCatalog.skills },
      { ids: policy.allowedMcpServers, catalog: input.snapshot.toolCatalog.mcpServers }
    ].flatMap((source) => source.ids.map((id2) => source.catalog.find((item) => item.name === id2)?.displayName ?? "来源已不可用"));
    rows.push({ key: "tools", label: "工具访问", value: !policy.enabled ? "跟随全局工具配置" : toolNames.length === 0 ? "未允许任何工具来源" : toolNames.join("、") });
  } else {
    const members = [...input.group.members].sort((a, b) => a.orderIndex - b.orderIndex);
    contents.push(ctx.UI.Column({ key: "entity-preview-members", fillMaxWidth: true, spacing: 10, paddingBottom: 22 }, [
      ctx.UI.Text({ text: "组内角色", style: "titleSmall", fontWeight: "600", color: colors.onSurfaceVariant }),
      ...members.length === 0 ? [ctx.UI.Text({ text: "暂未添加角色", style: "bodyMedium", color: colors.onSurfaceVariant })] : members.map((member2, index) => {
        const record2 = input.snapshot.cards.find((card2) => card2.id === member2.characterCardId);
        return ctx.UI.Row({ key: "entity-preview-member-" + member2.characterCardId, fillMaxWidth: true, paddingVertical: 10, spacing: 12, verticalAlignment: "center" }, [
          ctx.UI.Text({ text: String(index + 1).padStart(2, "0"), style: "labelMedium", color: colors.onSurfaceVariant }),
          ctx.UI.Image({ width: 40, height: 40, ...record2?.avatarUri == null ? { path: input.avatar ?? void 0 } : { uri: record2.avatarUri }, contentScale: "crop", contentDescription: record2?.name ?? "角色已移除", modifier: ctx.Modifier.clip({ cornerRadius: 12 }) }),
          ctx.UI.Column({ weight: 1, spacing: 4 }, [
            ctx.UI.Text({ text: record2?.name ?? "角色已移除", style: "titleSmall", fontWeight: "600", fillMaxWidth: true }),
            ...record2?.description.trim() ? [ctx.UI.Text({ text: record2.description, style: "bodySmall", color: colors.onSurfaceVariant, fillMaxWidth: true })] : []
          ])
        ]);
      })
    ]));
    rows.push({ key: "theme", label: "主题", value: themeLabel });
  }
  contents.push(ctx.UI.Column({ key: "entity-preview-config", fillMaxWidth: true, backgroundColor: colors.surfaceContainerLow, backgroundShape: { cornerRadius: 16 } }, [
    ctx.UI.Surface(
      { key: "entity-preview-config-toggle", fillMaxWidth: true, color: "transparent", shape: { cornerRadius: 16 }, onClick: input.onToggleConfiguration },
      ctx.UI.Row({ fillMaxWidth: true, padding: 16, spacing: 10, verticalAlignment: "center" }, [
        ctx.UI.Icon({ name: "Tune", size: 20, tint: colors.onSurfaceVariant }),
        ctx.UI.Column({ weight: 1, spacing: 3 }, [
          ctx.UI.Text({ text: "配置摘要", style: "titleSmall", fontWeight: "600" }),
          ctx.UI.Text({ text: card === null ? "主题配置" : "模型、语音与记忆", style: "bodySmall", color: colors.onSurfaceVariant })
        ]),
        ctx.UI.Icon({ name: input.configurationExpanded ? "ExpandLess" : "ExpandMore", size: 20, tint: colors.onSurfaceVariant })
      ])
    ),
    ...input.configurationExpanded ? [ctx.UI.Column(
      { key: "entity-preview-config-details", fillMaxWidth: true, paddingStart: 16, paddingEnd: 16, paddingBottom: 18, spacing: 14 },
      rows.map((row) => ctx.UI.Column({ key: "entity-preview-config-" + row.key, fillMaxWidth: true, spacing: 4 }, [
        ctx.UI.Text({ text: row.label, style: "labelMedium", color: colors.onSurfaceVariant }),
        selectable("entity-preview-config-" + row.key + "-value", row.value)
      ]))
    )] : []
  ]));
  return contents;
}

// src/ui-entity-native.ts
function copy(value) {
  return JSON.parse(JSON.stringify(value));
}
function request(input) {
  return ToolPkg.ipc.call("character-memory.request", input, { targetRuntime: "main" });
}
function renderManagedEntityScreen(ctx, pending, finish) {
  const assertOwner = () => {
    const [active] = ctx.useState("managed-entity", null);
    if (active?.requestId !== pending.requestId) throw new Error("Native management entity request is no longer active");
  };
  return renderNativeEntity(ctx, pending.input, {
    assertOwner,
    async complete(operation, id2) {
      assertOwner();
      finish(operation === void 0 ? null : { operation, id: id2 ?? pending.input.id });
      return null;
    },
    cancel() {
      assertOwner();
      finish(null);
      return null;
    }
  }, "managed-entity:" + pending.requestId + ":");
}
function renderNativeEntity(ctx, input, lifecycle, statePrefix = "") {
  const [state, setState] = ctx.useState(statePrefix + "native-entity-state", {
    snapshot: null,
    card: null,
    group: null,
    editing: input.mode !== "preview",
    tab: 0,
    previewConfigurationExpanded: false,
    loading: true,
    busy: false,
    error: "",
    fullscreen: false,
    deleting: false,
    finished: false,
    tags: [],
    tagChanges: { created: [], updated: [], deleted: [] },
    tagDraft: null,
    tagDeleting: false,
    nextTag: 0,
    expanded: null,
    picker: null,
    tagManaging: false,
    toolDraft: null,
    toolTab: 0,
    toolQuery: "",
    exporting: false,
    exportContent: "",
    exportName: "",
    avatar: null,
    themes: null,
    themeError: ""
  });
  const live = ctx.useRef(statePrefix + "native-entity-live", state);
  live.current = state;
  const assertOwner = lifecycle.assertOwner;
  assertOwner();
  function update(patch) {
    live.current = { ...live.current, ...patch };
    setState(live.current);
  }
  async function act(operation) {
    assertOwner();
    if (live.current.busy || live.current.finished) return null;
    update({ busy: true, error: "" });
    try {
      const value = await operation();
      if (!live.current.finished) assertOwner();
      return value ?? null;
    } catch (error) {
      assertOwner();
      update({ error: String(error) });
    } finally {
      update({ busy: false });
    }
    return null;
  }
  function card() {
    if (live.current.card === null) throw new Error("角色卡尚未加载");
    return live.current.card;
  }
  function group() {
    if (live.current.group === null) throw new Error("群组尚未加载");
    return live.current.group;
  }
  function changeCard(patch) {
    assertOwner();
    if (!live.current.busy) update({ card: { ...card(), ...patch } });
  }
  function changeGroup(patch) {
    assertOwner();
    if (!live.current.busy) update({ group: { ...group(), ...patch } });
  }
  async function loadThemes() {
    try {
      const themes = await request({ action: "listThemeChoices" });
      if (live.current.finished) return;
      assertOwner();
      const picker = live.current.picker;
      update({ themes, themeError: "", ...picker !== null && (picker.key === "theme" || picker.key === "entity-theme-picker") ? { picker: { ...picker, choices: themes } } : {} });
    } catch (error) {
      if (live.current.finished) return;
      try {
        assertOwner();
      } catch {
        return;
      }
      update({ themeError: String(error) });
    }
  }
  async function loadDefaultAvatar() {
    try {
      const avatar = await ToolPkg.readResource("character_default_avatar", "operit-avatar.png");
      if (avatar.trim() === "" || live.current.finished) return;
      assertOwner();
      update({ avatar });
    } catch {
    }
  }
  async function load() {
    await act(async () => {
      const snapshot = await request({ action: "snapshot" });
      assertOwner();
      const record2 = input.mode === "create" ? input.entity === "card" ? createCharacterDraft() : createGroupDraft() : input.entity === "card" ? snapshot.cards.find((value) => value.id === input.id) : snapshot.groups.find((value) => value.id === input.id);
      if (record2 === void 0) throw new Error("预览实体不存在：" + input.id);
      assertOwner();
      update({ snapshot, tags: copy(snapshot.tags), card: input.entity === "card" ? copy(record2) : null, group: input.entity === "group" ? copy(record2) : null, loading: false });
      if (input.entity === "group" || record2.avatarUri === null) void loadDefaultAvatar();
      if (live.current.editing) void loadThemes();
    });
    if (live.current.loading) update({ loading: false });
  }
  async function close() {
    assertOwner();
    if (live.current.busy || live.current.finished) return null;
    if (live.current.expanded !== null) {
      update({ expanded: null, fullscreen: false });
      return null;
    }
    if (live.current.picker !== null) {
      update({ picker: null });
      return null;
    }
    if (live.current.toolDraft !== null) {
      update({ toolDraft: null });
      return null;
    }
    if (live.current.exporting) {
      update({ exporting: false, exportContent: "" });
      return null;
    }
    if (live.current.tagManaging && live.current.tagDraft === null) {
      update({ tagManaging: false });
      return null;
    }
    if (live.current.tagDraft !== null) {
      update({ tagDraft: null, tagDeleting: false });
      return null;
    }
    if (live.current.deleting) {
      update({ deleting: false });
      return null;
    }
    if (input.mode === "preview" && live.current.editing) {
      const snapshot = live.current.snapshot;
      if (snapshot === null) throw new Error("预览数据尚未加载");
      update({
        editing: false,
        tab: 0,
        error: "",
        tags: copy(snapshot.tags),
        tagChanges: { created: [], updated: [], deleted: [] },
        card: input.entity === "card" ? copy(snapshot.cards.find((value) => value.id === input.id) ?? card()) : null,
        group: input.entity === "group" ? copy(snapshot.groups.find((value) => value.id === input.id) ?? group()) : null
      });
      return null;
    }
    update({ finished: true });
    return input.mode === "preview" ? lifecycle.complete() : lifecycle.cancel();
  }
  async function edit() {
    assertOwner();
    update({ editing: true, tab: 0 });
    if (live.current.themes === null) void loadThemes();
  }
  function togglePreviewConfiguration() {
    assertOwner();
    const expanded = !live.current.previewConfigurationExpanded;
    update({ previewConfigurationExpanded: expanded });
    if (expanded && (live.current.card ?? live.current.group)?.themeConfigId != null && live.current.themes === null) void loadThemes();
  }
  async function save() {
    return act(async () => {
      let snapshot;
      if (input.entity === "card") {
        const draft = card();
        if (draft.name.trim() === "") throw new Error("角色名称不能为空");
        let bindingChanged = input.mode === "create";
        let memoryBindingChanged = input.mode === "create";
        if (input.mode !== "create") {
          const original = live.current.snapshot.cards.find((value) => value.id === input.id);
          if (original === void 0) throw new Error("角色卡已不存在");
          bindingChanged = !sameCharacterModelBinding(original, draft);
          memoryBindingChanged = original.memoryBindingMode !== draft.memoryBindingMode || original.sharedMemoryId !== draft.sharedMemoryId;
        }
        const bindingStatus = characterModelBindingStatus(draft, live.current.snapshot.models);
        if (bindingChanged && bindingStatus.needsRebind) throw new Error(bindingStatus.label);
        if (memoryBindingChanged && draft.memoryBindingMode === "SHARED" && !live.current.snapshot?.stores.some((store) => store.id === draft.sharedMemoryId)) throw new Error("请选择有效的共享记忆库");
        snapshot = await request({ action: "saveCharacter", card: copy(draft), create: input.mode === "create", tagChanges: copy(live.current.tagChanges) });
      } else {
        const draft = group();
        if (draft.name.trim() === "") throw new Error("群组名称不能为空");
        snapshot = await request({ action: "saveGroup", group: copy(draft), create: input.mode === "create" });
      }
      const previousSnapshot = live.current.snapshot;
      assertOwner();
      update({
        snapshot,
        editing: false,
        tab: 0,
        tags: copy(snapshot.tags),
        tagChanges: { created: [], updated: [], deleted: [] },
        card: input.entity === "card" ? copy(snapshot.cards.find((value) => value.id === input.id) ?? card()) : null,
        group: input.entity === "group" ? copy(snapshot.groups.find((value) => value.id === input.id) ?? group()) : null
      });
      if (input.mode !== "preview") {
        const before = previousSnapshot;
        const saved = input.mode === "create" ? input.entity === "card" ? snapshot.cards.find((value) => !before.cards.some((old) => old.id === value.id)) : snapshot.groups.find((value) => !before.groups.some((old) => old.id === value.id)) : input.entity === "card" ? snapshot.cards.find((value) => value.id === input.id) : snapshot.groups.find((value) => value.id === input.id);
        if (saved === void 0) throw new Error("保存结果缺少实体标识");
        update({ finished: true });
        return lifecycle.complete("saved", saved.id);
      }
      return null;
    });
  }
  async function stageTag() {
    return act(async () => {
      const draft = live.current.tagDraft;
      if (draft === null) throw new Error("没有标签草稿");
      const changes = copy(live.current.tagChanges);
      let tags = copy(live.current.tags), attachedTagIds = [...card().attachedTagIds];
      if (live.current.tagDeleting) {
        tags = tags.filter((tag) => tag.id !== draft.id);
        attachedTagIds = attachedTagIds.filter((id2) => id2 !== draft.id);
        if (changes.created.some((tag) => tag.draftId === draft.id)) changes.created = changes.created.filter((tag) => tag.draftId !== draft.id);
        else {
          changes.updated = changes.updated.filter((tag) => tag.id !== draft.id);
          changes.deleted.push(draft.id);
        }
      } else {
        if (draft.name.trim() === "") throw new Error("标签名称不能为空");
        const id2 = draft.id === "" ? "draft_prompt_tag_" + (live.current.nextTag + 1) : draft.id;
        const tag = { ...draft, id: id2, name: draft.name.trim(), description: draft.description.trim() };
        const { id: _id, ...values } = tag;
        if (draft.id === "") {
          tags.push(tag);
          attachedTagIds.push(id2);
          changes.created.push({ draftId: id2, values });
        } else {
          tags = tags.map((value) => value.id === id2 ? tag : value);
          const created = changes.created.find((value) => value.draftId === id2);
          if (created !== void 0) created.values = values;
          else {
            changes.updated = changes.updated.filter((value) => value.id !== id2);
            changes.updated.push(tag);
          }
        }
      }
      update({ tags, tagChanges: changes, card: { ...card(), attachedTagIds }, tagDraft: null, tagDeleting: false, nextTag: live.current.nextTag + 1 });
    });
  }
  async function remove() {
    return act(async () => {
      await request(input.entity === "card" ? { action: "deleteCharacter", id: input.id } : { action: "deleteGroup", id: input.id });
      assertOwner();
      update({ finished: true });
      return lifecycle.complete(input.mode === "preview" ? void 0 : "deleted");
    });
  }
  async function chooseAvatar() {
    await act(async () => {
      const selected = await ctx.openFilePicker({ picker: "image", allowMultiple: false, mimeTypes: ["image/*"], maxContentBytes: MAX_AVATAR_BYTES });
      assertOwner();
      if (selected.cancelled) return;
      if (selected.files.length !== 1) throw new Error("请选择一个头像文件");
      const contentBase64 = readAvatarImport(selected.files[0]);
      assertOwner();
      const directory = ToolPkg.getConfigDir().replace(/\/$/, "") + "/avatars";
      const created = await Tools.Files.mkdir(directory, true);
      if (!created.successful) throw new Error(created.details);
      const uri = directory + "/" + Date.now() + "-" + Math.random().toString(36).slice(2) + ".image";
      const written = await Tools.Files.writeBinary(uri, contentBase64);
      if (!written.successful) throw new Error(written.details);
      assertOwner();
      update({ card: { ...card(), avatarUri: uri } });
    });
  }
  const colors = ctx.MaterialTheme.colorScheme, enabled = !state.busy && !state.finished;
  const text3 = (value) => ctx.UI.Text({ text: value, fillMaxWidth: true, style: "bodyMedium" });
  const caption = (value) => ctx.UI.Text({ text: value, fillMaxWidth: true, style: "bodySmall", color: colors.onSurfaceVariant });
  const section = (title, value) => ctx.UI.Column({ key: "preview-" + title, fillMaxWidth: true, paddingBottom: 12, spacing: 6 }, [
    ctx.UI.Text({ text: title, style: "labelLarge", color: colors.onSurfaceVariant }),
    ctx.UI.SelectionContainer({}, text3(value))
  ]);
  const field = (key, label, value, apply, maxLines = 1) => ctx.UI.Box({ fillMaxWidth: true, paddingBottom: 12 }, ctx.UI.TextField({
    key: "entity-field-" + key,
    label,
    value,
    fillMaxWidth: true,
    enabled,
    singleLine: maxLines === 1,
    minLines: maxLines,
    maxLines,
    style: {},
    onValueChange: apply,
    ...maxLines === 1 ? {} : { suffixIconWidth: 36, suffixIconHeight: 36, trailingIcon: ctx.UI.Box({ width: 36, height: 36, contentAlignment: "center" }, ctx.UI.IconButton({
      key: "entity-expand-" + key,
      width: 28,
      height: 28,
      contentPadding: { horizontal: 0, vertical: 0 },
      enabled,
      onClick: () => {
        assertOwner();
        update({ expanded: { key, label, value, apply }, fullscreen: true });
      }
    }, ctx.UI.Icon({ name: "Fullscreen", size: 18 }))) }
  }));
  const check = (key, label, checked, apply) => ctx.UI.Row({ key, fillMaxWidth: true, verticalAlignment: "center", spacing: 8 }, [ctx.UI.Checkbox({ checked, enabled, onCheckedChange: apply }), ctx.UI.Text({ text: label, weight: 1 })]);
  const configure = (key, label, value, apply) => ctx.UI.Box(
    { fillMaxWidth: true, paddingBottom: 12 },
    ctx.UI.Surface(
      { key, fillMaxWidth: true, color: "transparent", onClick: enabled ? apply : void 0 },
      ctx.UI.InputDecorator({ label, enabled }, ctx.UI.Row({ fillMaxWidth: true, verticalAlignment: "center", spacing: 12 }, [ctx.UI.Text({ text: value, weight: 1, maxLines: 1, overflow: "ellipsis", style: "bodyMedium" }), ctx.UI.Icon({ name: "Tune", size: 24 })]))
    )
  );
  const select = (key, label, value, choices, apply, empty2 = "不绑定") => configure(key, label, choices.find((option) => option.id === value)?.label ?? (value === null ? empty2 : "已绑定配置不存在：" + value), () => {
    assertOwner();
    update({ picker: { title: "选择" + label, key, value, choices, apply, query: "" } });
  });
  const binding = (key, title, subtitleOff, subtitleOn, checked, apply, children, switchEnabled = true) => ctx.UI.Box({ key, fillMaxWidth: true, paddingBottom: 7 }, ctx.UI.Box(
    { fillMaxWidth: true, modifier: ctx.Modifier.border(1, colors.outlineVariant.copy({ alpha: 0.24 }), { cornerRadius: 12 }), paddingHorizontal: 10, paddingVertical: 6 },
    ctx.UI.Column({ fillMaxWidth: true }, [
      ctx.UI.Row({ fillMaxWidth: true, modifier: ctx.Modifier.heightIn({ minHeight: 46 }), verticalAlignment: "center", spacing: 8 }, [
        ctx.UI.Column({ weight: 1, spacing: 1 }, [ctx.UI.Text({ text: title, fontSize: 14, fontWeight: "700", color: enabled && switchEnabled ? colors.onSurface : colors.onSurface.copy({ alpha: 0.46 }) }), ctx.UI.Text({ text: checked ? subtitleOn : subtitleOff, fontSize: 12, color: enabled && switchEnabled ? colors.onSurfaceVariant : colors.onSurfaceVariant.copy({ alpha: 0.46 }) })]),
        ctx.UI.Switch({ key: key + "-switch", checked, enabled: enabled && switchEnabled, modifier: ctx.Modifier.scale(0.82), onCheckedChange: apply })
      ]),
      ...checked || !switchEnabled ? [ctx.UI.Spacer({ height: 6 }), ...children] : []
    ])
  ));
  const body = (key, children, padding = { paddingStart: 18, paddingTop: 10, paddingEnd: 18, paddingBottom: 10 }) => ctx.UI.LazyColumn({ key, fillMaxWidth: true }, children.map((child, index) => ctx.UI.Box({
    key: key + "-item-" + String(child.props?.key ?? index),
    fillMaxWidth: true,
    paddingStart: padding.paddingStart,
    paddingEnd: padding.paddingEnd,
    paddingTop: index === 0 ? padding.paddingTop : 0,
    paddingBottom: index === children.length - 1 ? padding.paddingBottom : 0
  }, child)));
  const tabs = (key, labels, selected, apply) => ctx.UI.PrimaryTabRow({
    key,
    fillMaxWidth: true,
    selectedTabIndex: selected,
    tabs: labels.map((label, tab) => ctx.UI.Tab({ key: key + "-" + tab, selected: selected === tab, enabled, onClick: () => {
      assertOwner();
      apply(tab);
    } }, ctx.UI.Text({ text: label, style: "titleSmall" })))
  });
  const standardButton = (key, label, action, filled = false) => filled ? ctx.UI.Button({ key, text: label, enabled, onClick: action }) : ctx.UI.TextButton({ key, text: label, enabled, onClick: action });
  function shell(title, child, actions, width = 760, height = 820, plainBody = false, reading = false) {
    return ctx.UI.Column({ key: "native-entity-panel", ...state.fullscreen ? { fillMaxSize: true } : { width, height }, modifier: ctx.Modifier.clip({ cornerRadius: state.fullscreen ? 0 : 28 }) }, [
      ctx.UI.Row({ key: "entity-header", fillMaxWidth: true, paddingStart: 24, paddingTop: reading ? 18 : 22, paddingEnd: 16, paddingBottom: reading ? 8 : 12, verticalAlignment: "center" }, [
        ctx.UI.Text({ text: title, style: reading ? "labelLarge" : "headlineSmall", fontWeight: reading ? "600" : "700", ...reading ? { color: colors.onSurfaceVariant, fontSize: 14 } : {}, maxLines: 2, overflow: "ellipsis", weight: 1 }),
        ctx.UI.IconButton({ key: "entity-close", icon: "Close", enabled, onClick: close })
      ]),
      ...state.busy ? [ctx.UI.LinearProgressIndicator({ fillMaxWidth: true })] : [],
      ...state.error === "" ? [] : [ctx.UI.Text({ key: "entity-error", text: state.error, color: colors.error, fillMaxWidth: true, paddingHorizontal: 24, paddingBottom: 12 })],
      ctx.UI.Box({ key: "entity-content", weight: 1, fillMaxWidth: true, ...plainBody ? {} : { paddingStart: 24, paddingTop: 16, paddingEnd: 24, paddingBottom: 20 } }, child),
      ctx.UI.FlowRow({ key: "entity-footer", fillMaxWidth: true, paddingStart: 24, paddingTop: 0, paddingEnd: 24, paddingBottom: 20, horizontalArrangement: "end", spacing: 8, runSpacing: 8 }, actions)
    ]);
  }
  let panel;
  const cancel = standardButton("entity-cancel", "取消", close);
  if (state.expanded !== null) {
    const expanded = state.expanded;
    panel = shell(expanded.label, ctx.UI.TextField({ key: "entity-expanded-input", label: expanded.label, value: expanded.value, fillMaxSize: true, minLines: 16, maxLines: 40, enabled, onValueChange: (value) => {
      assertOwner();
      if (live.current.expanded !== null) update({ expanded: { ...live.current.expanded, value } });
    } }), [
      cancel,
      standardButton("entity-primary", "保存", () => {
        assertOwner();
        const draft = live.current.expanded;
        if (draft !== null) {
          draft.apply(draft.value);
          update({ expanded: null, fullscreen: false });
        }
      }, true)
    ], 760, 620);
  } else if (state.picker !== null) {
    const picker = state.picker;
    const themePending = (picker.key === "theme" || picker.key === "entity-theme-picker") && state.themes === null;
    const choose = (id2) => {
      assertOwner();
      picker.apply(id2);
      update({ picker: null });
    };
    const choices = picker.choices.filter((option) => (option.label + " " + (option.subtitle ?? "")).toLowerCase().includes(picker.query.trim().toLowerCase()));
    panel = shell(picker.title, ctx.UI.Column({ fillMaxSize: true }, [
      field("picker-query", "搜索", picker.query, (query) => {
        assertOwner();
        if (live.current.picker !== null) update({ picker: { ...live.current.picker, query } });
      }),
      ...themePending ? [state.themeError === "" ? caption("正在读取主题配置…") : ctx.UI.Column({ fillMaxWidth: true }, [ctx.UI.Text({ text: state.themeError, color: colors.error }), standardButton("entity-picker-retry", "重新加载主题", loadThemes)])] : [],
      ctx.UI.LazyColumn({ key: "entity-picker-list", weight: 1, fillMaxWidth: true }, [
        ctx.UI.Row({ key: "entity-picker-clear", fillMaxWidth: true, paddingHorizontal: 8, paddingVertical: 9, verticalAlignment: "center", spacing: 10, onClick: () => choose(null) }, [ctx.UI.Icon({ name: picker.value === null ? "CheckCircle" : "CircleOutlined", size: 20, tint: picker.value === null ? colors.primary : colors.onSurfaceVariant }), text3("跟随全局 / 不绑定")]),
        ...choices.map((option) => ctx.UI.Row({ key: "entity-picker-" + option.id, fillMaxWidth: true, paddingHorizontal: 8, paddingVertical: 9, verticalAlignment: "center", spacing: 10, onClick: () => choose(option.id) }, [ctx.UI.Icon({ name: picker.value === option.id ? "CheckCircle" : "CircleOutlined", size: 20, tint: picker.value === option.id ? colors.primary : colors.onSurfaceVariant }), ctx.UI.Column({ weight: 1 }, [text3(option.label), ...option.subtitle === void 0 ? [] : [caption(option.subtitle)]])]))
      ])
    ]), [cancel], 580, 560);
  } else if (state.toolDraft !== null && state.snapshot !== null) {
    const draft = state.toolDraft, catalog = state.snapshot.toolCatalog;
    const names = ["内置工具", "插件", "技能", "MCP"], keys = ["allowedBuiltinTools", "allowedPackages", "allowedSkills", "allowedMcpServers"];
    const key = keys[state.toolTab], options = [catalog.builtinTools, catalog.packages, catalog.skills, catalog.mcpServers][state.toolTab];
    const children = options.filter((option) => (option.displayName + " " + option.description).toLowerCase().includes(state.toolQuery.toLowerCase())).map((option) => check("entity-tool-" + key + "-" + option.name, option.displayName, draft[key].includes(option.name), (checked) => {
      assertOwner();
      const policy = live.current.toolDraft;
      if (policy !== null) update({ toolDraft: { ...policy, [key]: checked ? [...policy[key], option.name] : policy[key].filter((name) => name !== option.name) } });
    }));
    panel = shell("配置允许使用的工具", ctx.UI.Column({ fillMaxSize: true }, [
      tabs("entity-tool-tabs", names, state.toolTab, (toolTab) => update({ toolTab })),
      ctx.UI.Box({ fillMaxWidth: true, paddingHorizontal: 18, paddingTop: 10 }, field("tool-query", "搜索", state.toolQuery, (toolQuery) => update({ toolQuery }))),
      ctx.UI.Box({ weight: 1, fillMaxWidth: true }, body("entity-tool-list", children))
    ]), [cancel, standardButton("entity-primary", "确定", () => {
      assertOwner();
      if (live.current.toolDraft !== null) changeCard({ toolAccessConfig: copy(live.current.toolDraft) });
      update({ toolDraft: null });
    }, true)], 760, 620, true);
  } else if (state.exporting) {
    const exportFile = async (format2) => {
      await act(async () => {
        const content = input.entity === "card" ? await request({ action: "exportCharacter", id: input.id, format: format2 }) : await request({ action: "exportGroup", id: input.id });
        assertOwner();
        const name = input.entity === "card" ? card().name : group().name;
        update({ exportContent: content, exportName: name.replace(/[<>:"/\\|?*\u0000-\u001f]/g, "_") + "-" + format2 + ".json" });
      });
    };
    const writeExport = async () => {
      await act(async () => {
        const result = await ctx.saveFile({ suggestedName: live.current.exportName, content: live.current.exportContent, mimeType: "application/json", extensions: ["json"] });
        assertOwner();
        if (result.cancelled) return;
        update({ exporting: false, exportContent: "", exportName: "" });
      });
    };
    panel = shell("导出", state.exportContent === "" ? ctx.UI.Column({ fillMaxWidth: true, spacing: 8 }, [standardButton("entity-export-json", "导出 JSON", () => exportFile("operit")), ...input.entity === "card" ? [standardButton("entity-export-tavern", "导出酒馆 JSON", () => exportFile("tavern"))] : []]) : body("entity-export-body", [caption("点击保存文件选择保存位置。"), section("JSON", state.exportContent)]), [cancel, ...state.exportContent === "" ? [] : [standardButton("entity-export-save", "保存文件", writeExport, true)]], state.exportContent === "" ? 420 : 760, state.exportContent === "" ? 240 : 620);
  } else if (state.tagDraft !== null) {
    const draft = state.tagDraft;
    const children = state.tagDeleting ? [text3(`确定删除标签“${draft.name}”？`)] : [["name", "标签名称", 1], ["description", "描述", 1], ["promptContent", "提示词内容", 6]].map(([key, label, lines]) => field("tag-" + key, label, draft[key], (value) => {
      assertOwner();
      if (live.current.tagDraft !== null) update({ tagDraft: { ...live.current.tagDraft, [key]: value } });
    }, lines));
    panel = shell(state.tagDeleting ? "删除标签" : draft.id === "" ? "创建标签" : "编辑标签", body("entity-tag-body", children, { paddingStart: 0, paddingTop: 0, paddingEnd: 0, paddingBottom: 0 }), [
      ...draft.id === "" || state.tagDeleting ? [] : [standardButton("entity-tag-delete", "删除", () => {
        assertOwner();
        update({ tagDeleting: true });
      })],
      cancel,
      standardButton("entity-primary", state.tagDeleting ? "删除" : "保存", stageTag, true)
    ], 620, state.tagDeleting ? 240 : 520);
  } else if (state.tagManaging) {
    panel = shell("管理标签", body("entity-tag-manager", state.tags.map((tag) => ctx.UI.Row({ key: "entity-tag-row-" + tag.id, fillMaxWidth: true, verticalAlignment: "center" }, [
      ctx.UI.Box({ weight: 1 }, check("entity-tag-" + tag.id, tag.name, state.card?.attachedTagIds.includes(tag.id) ?? false, (checked) => changeCard({ attachedTagIds: checked ? [...card().attachedTagIds, tag.id] : card().attachedTagIds.filter((id2) => id2 !== tag.id) }))),
      standardButton("entity-tag-edit-" + tag.id, "编辑", () => {
        assertOwner();
        update({ tagDraft: copy(tag) });
      }),
      standardButton("entity-tag-remove-" + tag.id, "删除", () => {
        assertOwner();
        update({ tagDraft: copy(tag), tagDeleting: true });
      })
    ])), { paddingStart: 0, paddingTop: 0, paddingEnd: 0, paddingBottom: 0 }), [standardButton("entity-tag-create", "创建标签", () => {
      assertOwner();
      update({ tagDraft: { id: "", name: "", description: "", promptContent: "", tagType: "CUSTOM" } });
    }), standardButton("entity-cancel", "完成", close)], 620, 560);
  } else if (state.deleting) {
    panel = shell("确认删除", text3(`确定删除“${state.card?.name ?? state.group?.name}”？此操作不可撤销。`), [cancel, standardButton("entity-primary", "删除", remove, true)], 560, 240);
  } else {
    const pages = [[], [], []];
    if (state.loading) pages[0].push(ctx.UI.Box({ fillMaxSize: true, contentAlignment: "center" }, ctx.UI.LoadingIndicator({ size: 32 })));
    else if (state.snapshot === null) pages[0].push(standardButton("entity-retry", "重新加载", load));
    else if (state.card !== null) {
      const draft = state.card, snapshot = state.snapshot;
      if (!state.editing) {
        pages[0].push(...renderEntityPreview(ctx, {
          card: draft,
          group: null,
          snapshot,
          tags: state.tags,
          avatar: state.avatar,
          themes: state.themes,
          themeError: state.themeError,
          configurationExpanded: state.previewConfigurationExpanded,
          onToggleConfiguration: togglePreviewConfiguration
        }));
      } else {
        pages[0].push(
          field("name", "角色名称", draft.name, (name) => changeCard({ name })),
          field("description", "描述", draft.description, (description) => changeCard({ description })),
          ctx.UI.Box({ fillMaxWidth: true, paddingBottom: 12 }, ctx.UI.InputDecorator({ label: "角色头像", enabled }, ctx.UI.Row({ fillMaxWidth: true, verticalAlignment: "center" }, [
            ctx.UI.Image({ key: "entity-edit-avatar", ...draft.avatarUri === null ? { path: state.avatar ?? void 0 } : { uri: draft.avatarUri }, width: 44, height: 44, contentScale: "crop", contentDescription: "角色头像", modifier: ctx.Modifier.clip({ type: "circle" }) }),
            ctx.UI.Spacer({ width: 12 }),
            ctx.UI.Text({ text: draft.avatarUri ?? "未设置", style: "bodyMedium", weight: 1, maxLines: 1, overflow: "ellipsis" }),
            ctx.UI.Spacer({ width: 8 }),
            standardButton("entity-choose-avatar", "选择", chooseAvatar),
            ...draft.avatarUri === null ? [] : [standardButton("entity-clear-avatar", "清除", async () => {
              await act(async () => {
                assertOwner();
                update({ card: { ...card(), avatarUri: null } });
                if (live.current.avatar === null) void loadDefaultAvatar();
              });
            })]
          ]))),
          field("characterSetting", "角色设定", draft.characterSetting, (characterSetting) => changeCard({ characterSetting }), 6),
          field("openingStatement", "开场白", draft.openingStatement, (openingStatement) => changeCard({ openingStatement }), 3),
          ctx.UI.Column({ fillMaxWidth: true, paddingBottom: 12 }, [
            ctx.UI.Row({ fillMaxWidth: true, verticalAlignment: "center" }, [ctx.UI.Text({ text: "标签", weight: 1, fontWeight: "700" }), ctx.UI.TextButton({ key: "entity-tags-manage", enabled, onClick: () => {
              assertOwner();
              update({ tagManaging: true });
            } }, ctx.UI.Row({ spacing: 8, verticalAlignment: "center" }, [ctx.UI.Icon({ name: "Tune", size: 18 }), ctx.UI.Text({ text: "管理标签" })]))]),
            ctx.UI.Spacer({ height: 6 }),
            state.tags.length === 0 ? caption("暂无提示词标签") : ctx.UI.FlowRow({ spacing: 8, runSpacing: 8 }, state.tags.map((tag) => ctx.UI.FilterChip({ key: "entity-tag-" + tag.id, selected: draft.attachedTagIds.includes(tag.id), enabled, label: ctx.UI.Text({ text: tag.name, maxLines: 1, overflow: "ellipsis", modifier: ctx.Modifier.widthIn({ maxWidth: 160 }) }), onClick: () => {
              assertOwner();
              changeCard({ attachedTagIds: card().attachedTagIds.includes(tag.id) ? card().attachedTagIds.filter((id2) => id2 !== tag.id) : [...card().attachedTagIds, tag.id] });
            } })))
          ])
        );
        for (const [key, label, lines] of [["otherContentChat", "聊天附加内容", 4], ["otherContentVoice", "语音附加内容", 4], ["advancedCustomPrompt", "高级自定义 Prompt", 4], ["marks", "备注", 3]]) pages[1].push(field(key, label, draft[key], (value) => changeCard({ [key]: value }), lines));
        const modelStatus = characterModelBindingStatus(draft, snapshot.models);
        const tts = snapshot.ttsConfigs.find((config) => config.id === draft.ttsConfigId);
        const policy = draft.toolAccessConfig, count = policy.allowedBuiltinTools.length + policy.allowedPackages.length + policy.allowedSkills.length + policy.allowedMcpServers.length;
        pages[2].push(
          ...modelStatus.needsRebind ? [
            ctx.UI.Text({ key: "entity-model-binding-error", text: modelStatus.label, color: colors.error, style: "bodyMedium" }),
            standardButton("entity-model-follow-global", "跟随全局模型", () => changeCard({ chatModelBindingMode: "FOLLOW_GLOBAL", chatProviderId: null, chatModelId: null }))
          ] : [],
          binding("entity-model", "聊天模型", modelStatus.label, "使用固定模型配置", characterModelBindingMode(draft.chatModelBindingMode) === "FIXED_MODEL", (checked) => changeCard({ chatModelBindingMode: checked ? "FIXED_MODEL" : "FOLLOW_GLOBAL", chatModelId: checked ? card().chatModelId : null, chatProviderId: checked ? card().chatProviderId : null }), [select("entity-model-picker", "聊天模型配置", draft.chatProviderId === null || draft.chatModelId === null ? null : JSON.stringify([draft.chatProviderId, draft.chatModelId]), snapshot.models.map((value) => ({ id: JSON.stringify([value.providerId, value.modelId]), label: value.modelId, subtitle: value.providerName })), (selected) => {
            const model = snapshot.models.find((value) => JSON.stringify([value.providerId, value.modelId]) === selected);
            if (model === void 0) throw new Error("请选择有效的提供商与模型");
            changeCard({ chatModelBindingMode: "FIXED_MODEL", chatProviderId: model.providerId, chatModelId: model.modelId });
          }, "请选择聊天模型")]),
          binding("entity-theme", "主题配置", "不改变当前独立主题", "使用角色卡主题配置", draft.themeConfigId !== null, (checked) => {
            if (!checked) changeCard({ themeConfigId: null });
            else update({ picker: { title: "选择主题配置", key: "theme", value: draft.themeConfigId, choices: state.themes ?? [], apply: (themeConfigId) => changeCard({ themeConfigId }), query: "" } });
          }, [select("entity-theme-picker", "主题配置", draft.themeConfigId, state.themes ?? [], (themeConfigId) => changeCard({ themeConfigId }))]),
          binding("entity-tts", "TTS 配置", "跟随全局 TTS 配置", "使用角色卡 TTS 配置", draft.ttsConfigId !== null, (checked) => {
            if (!checked) changeCard({ ttsConfigId: null });
            else update({ picker: { title: "选择 TTS 配置", key: "tts", value: draft.ttsConfigId, choices: snapshot.ttsConfigs.map((config) => ({ id: config.id, label: config.name, subtitle: config.providerType + " · " + config.voice })), apply: (ttsConfigId) => changeCard({ ttsConfigId }), query: "" } });
          }, [select("entity-tts-picker", "TTS 配置", draft.ttsConfigId, snapshot.ttsConfigs.map((config) => ({ id: config.id, label: config.name, subtitle: config.providerType + " · " + config.voice })), (ttsConfigId) => changeCard({ ttsConfigId }), tts?.name ?? "请选择 TTS 配置")]),
          binding("entity-memory", "记忆绑定", "使用角色记忆", "使用共享记忆", draft.memoryBindingMode === "SHARED", (checked) => changeCard({ memoryBindingMode: checked ? "SHARED" : "CHARACTER", sharedMemoryId: checked ? card().sharedMemoryId : null }), [snapshot.stores.length === 0 ? caption("还没有共享记忆库") : select("entity-store-picker", "共享记忆库", draft.sharedMemoryId, snapshot.stores.map((store) => ({ id: store.id, label: store.name })), (sharedMemoryId) => changeCard({ sharedMemoryId }), "请选择共享记忆库")], snapshot.stores.length !== 0 || draft.memoryBindingMode === "SHARED"),
          binding("entity-tool", "工具访问", "跟随全局工具配置", "自定义允许使用的工具", policy.enabled, (checked) => changeCard({ toolAccessConfig: { ...card().toolAccessConfig, enabled: checked } }), [configure("entity-tool-configure", "配置允许使用的工具", "已配置 " + count + " 个工具来源", () => {
            assertOwner();
            update({ toolDraft: copy(card().toolAccessConfig), toolTab: 0, toolQuery: "" });
          })])
        );
        if (draft.sharedMemoryMounts.length !== 0) pages[2].push(section("共享记忆挂载", JSON.stringify(draft.sharedMemoryMounts, null, 2)));
        if (state.themeError !== "") pages[2].push(ctx.UI.Text({ text: state.themeError, color: colors.error }), standardButton("entity-theme-retry", "重新加载主题", async () => {
          await act(loadThemes);
        }));
      }
    } else if (state.group !== null && state.snapshot !== null) {
      const draft = state.group, snapshot = state.snapshot;
      if (!state.editing) pages[0].push(...renderEntityPreview(ctx, {
        card: null,
        group: draft,
        snapshot,
        tags: state.tags,
        avatar: state.avatar,
        themes: state.themes,
        themeError: state.themeError,
        configurationExpanded: state.previewConfigurationExpanded,
        onToggleConfiguration: togglePreviewConfiguration
      }));
      else {
        pages[0].push(field("name", "群组名称", draft.name, (name) => changeGroup({ name })), field("description", "描述", draft.description, (description) => changeGroup({ description })), select("entity-theme-picker", "主题配置", draft.themeConfigId, state.themes ?? [], (themeConfigId) => changeGroup({ themeConfigId })), ctx.UI.Text({ text: "组内角色", style: "titleSmall", fontWeight: "700", paddingTop: 8, paddingBottom: 8 }));
        for (const option of snapshot.cards) {
          const selected = draft.members.some((member2) => member2.characterCardId === option.id);
          pages[0].push(ctx.UI.Box({ fillMaxWidth: true, paddingBottom: 6 }, ctx.UI.Surface({
            key: "entity-member-" + option.id,
            fillMaxWidth: true,
            color: selected ? colors.primaryContainer.copy({ alpha: 0.16 }) : colors.surfaceContainerHighest.copy({ alpha: 0.28 }),
            shape: { cornerRadius: 10 },
            modifier: ctx.Modifier.border(1, selected ? colors.primary.copy({ alpha: 0.45 }) : colors.outlineVariant.copy({ alpha: 0.28 }), { cornerRadius: 10 }),
            onClick: () => {
              assertOwner();
              const members = selected ? group().members.filter((member2) => member2.characterCardId !== option.id) : [...group().members, { characterCardId: option.id, orderIndex: group().members.length }];
              changeGroup({ members: members.map((member2, orderIndex) => ({ ...member2, orderIndex })) });
            }
          }, ctx.UI.Row({ fillMaxWidth: true, paddingHorizontal: 12, paddingVertical: 8, spacing: 10, verticalAlignment: "center" }, [ctx.UI.Icon({ name: selected ? "CheckCircle" : "CircleOutlined", size: 18, tint: selected ? colors.primary : colors.onSurfaceVariant }), ctx.UI.Image({ width: 28, height: 28, ...option.avatarUri === null ? { path: state.avatar ?? void 0 } : { uri: option.avatarUri }, modifier: ctx.Modifier.clip({ type: "circle" }), contentScale: "crop", contentDescription: option.name }), ctx.UI.Column({ weight: 1 }, [ctx.UI.Text({ text: option.name, style: "bodyMedium", fontWeight: "700", maxLines: 1, overflow: "ellipsis" }), ...option.description.trim() === "" ? [] : [caption(option.description)]])]))));
        }
      }
    }
    const isCard = state.card !== null;
    const reading = !state.editing;
    const page = reading ? body("entity-preview-body", pages[0], { paddingStart: 24, paddingTop: 8, paddingEnd: 24, paddingBottom: 24 }) : isCard ? ctx.UI.Column({ fillMaxSize: true }, [tabs("entity-tab", ["基础", "内容", "绑定"], state.tab, (tab) => update({ tab })), ctx.UI.HorizontalPager({ key: "entity-pager", weight: 1, fillMaxWidth: true, currentPage: state.tab, onPageChanged: (tab) => {
      assertOwner();
      if (tab >= 0 && tab < 3) update({ tab });
    } }, pages.map((children, index) => body("entity-body-" + index, children)))]) : body("entity-body", pages[0], { paddingStart: 0, paddingTop: 0, paddingEnd: 0, paddingBottom: 0 });
    const title = state.editing ? input.mode === "create" ? input.entity === "card" ? "创建角色卡" : "创建群组" : input.entity === "card" ? "编辑角色卡" : "编辑群组" : input.entity === "card" ? "角色卡" : "角色群组";
    panel = shell(title, page, [
      ...state.editing && input.mode !== "create" ? [
        standardButton("entity-delete", "删除", () => {
          assertOwner();
          update({ deleting: true });
        }),
        standardButton("entity-export", "导出", () => {
          assertOwner();
          update({ exporting: true });
        })
      ] : [],
      standardButton("entity-cancel", state.editing ? "取消" : "关闭", close),
      ...state.snapshot === null || state.finished ? [] : [standardButton("entity-primary", state.editing ? "保存" : "编辑", state.editing ? save : edit, true)]
    ], reading ? isCard ? 680 : 620 : isCard ? 760 : 620, reading ? 760 : 820, reading || isCard, reading);
  }
  return ctx.UI.Dialog({
    key: "character-presentation",
    onLoad: load,
    closeOnDismissRequest: false,
    onDismissRequest: close,
    containerColor: colors.surfaceContainerHigh,
    shape: { cornerRadius: state.fullscreen ? 0 : 28 },
    properties: { fullscreen: state.fullscreen, usePlatformDefaultWidth: false, dismissOnClickOutside: false }
  }, panel);
}

// src/ui-initialization-recovery.ts
function initializationRecoveryUi(ctx, onRecovered) {
  const [state, setState] = ctx.useState("initialization-recovery", { failure: "", inspection: null, busy: false, confirmed: false });
  const live = ctx.useRef("initialization-recovery-live", state);
  live.current = state;
  function update(patch) {
    live.current = { ...live.current, ...patch };
    setState(live.current);
  }
  function reportFailure(failure) {
    update({ failure: String(failure), inspection: null, confirmed: false });
  }
  async function run(repair) {
    if (live.current.busy) return;
    const inspection2 = live.current.inspection;
    let request2 = { action: "inspect" };
    if (repair) {
      if (!live.current.confirmed) throw new Error("请先确认修复说明。");
      if (inspection2?.status === "repairable") request2 = { action: "repair", source: inspection2.source };
      else if (inspection2?.status === "planned") request2 = { action: "resume" };
      else throw new Error("请先检查初始化数据。");
    }
    update({ busy: true, confirmed: false });
    try {
      const result = await ToolPkg.ipc.call("character-memory.initialization", request2, { targetRuntime: "main" });
      if (repair) {
        if (result.status !== "ready") throw new Error("初始化修复未完成。");
        update({ failure: "", inspection: null });
        onRecovered();
      } else update({ inspection: result });
    } catch (failure) {
      update({ failure: String(failure) });
    } finally {
      update({ busy: false });
    }
  }
  if (state.failure === "") return { panel: null, reportFailure };
  const inspection = state.inspection;
  const repairable = inspection?.status === "repairable" || inspection?.status === "planned";
  const lines = [
    ctx.UI.Text({ text: "角色卡初始化未完成", style: "titleLarge" }),
    ctx.UI.Text({ text: state.failure, style: "bodyMedium" }),
    ctx.UI.TextButton({ key: "initialization-inspect", text: state.busy ? "正在处理…" : "检查初始化数据", enabled: !state.busy, onClick: () => run(false) })
  ];
  if (inspection?.status === "ready") lines.push(ctx.UI.Text({ text: "初始化元数据已存在。当前错误不属于缺失首次初始化信息，不会修改现有数据。" }));
  if (inspection?.status === "repairable") lines.push(
    ctx.UI.Text({ text: "检测到旧版首次安装中断。将保留此角色的 ID 和全部内容，并将其设为当前角色：" }),
    ctx.UI.Text({ text: inspection.source.card.name, fontWeight: "700" }),
    ctx.UI.Text({ text: inspection.source.card.id, style: "bodySmall" }),
    ctx.UI.Text({ text: "仅补齐缺失的初始化信息、记忆空间和 USER.md。已有记忆设置与文件保持不变；缺失的全局记忆开关及召回设置使用初始值。不会删除或重置角色卡。" })
  );
  if (inspection?.status === "planned") lines.push(ctx.UI.Text({ text: `已有“${inspection.cardName}”的初始化计划。将继续这份计划，保留原 ID 和已有文件，不重新安装。` }));
  if (repairable) lines.push(
    ctx.UI.Row({ spacing: 8, verticalAlignment: "center" }, [
      ctx.UI.Checkbox({ key: "initialization-confirm", checked: state.confirmed, enabled: !state.busy, onCheckedChange: (checked) => update({ confirmed: checked }) }),
      ctx.UI.Text({ text: "我已核对上述角色，确认按说明修复初始化", weight: 1 })
    ]),
    ctx.UI.Button({ key: "initialization-repair", text: "修复初始化", enabled: state.confirmed && !state.busy, onClick: () => run(true) })
  );
  return { panel: ctx.UI.Column({ key: "initialization-recovery-panel", fillMaxSize: true, padding: 20, spacing: 16 }, lines), reportFailure };
}

// src/ui-controls.ts
function uiRequest(input) {
  return ToolPkg.ipc.call("character-memory.request", input, { targetRuntime: "main" });
}
function uiState(ctx, key, initial) {
  const [state, setState] = ctx.useState(key, initial);
  const live = ctx.useRef(key + ":live", state);
  live.current = state;
  function update(patch) {
    live.current = { ...live.current, ...patch };
    setState(live.current);
  }
  return { state, live, update };
}
async function readUiJson(ctx) {
  const result = await ctx.openFilePicker({ picker: "document", allowMultiple: false, mimeTypes: ["application/json"] });
  if (result.cancelled) return null;
  if (result.files.length !== 1) throw new Error("请选择一个 JSON 文件");
  const file = await Tools.Files.read(result.files[0].path);
  const content = file.content.replace(/^\uFEFF/, "");
  if (content.trim() === "") throw new Error("所选 JSON 文件为空");
  return content;
}
function uiNumber(value, label, min, max = Infinity, integer2 = false) {
  const number2 = Number(value);
  if (value.trim() === "" || !Number.isFinite(number2) || number2 < min || number2 > max || integer2 && !Number.isInteger(number2)) throw new Error(label + "的范围或格式无效");
  return number2;
}
function uiDate(value, end) {
  if (value === "") return null;
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) throw new Error("日期格式应为 YYYY-MM-DD");
  const [year, month, day] = value.split("-").map(Number), date = new Date(year, month - 1, day);
  if (date.getFullYear() !== year || date.getMonth() !== month - 1 || date.getDate() !== day || date.getTime() < 0) throw new Error("日期无效");
  if (end) {
    date.setDate(date.getDate() + 1);
    return date.getTime() - 1;
  }
  return date.getTime();
}
function nativeUi(ctx, enabled = true) {
  const colors = ctx.MaterialTheme.colorScheme;
  const [size, setSize] = ctx.useState("character-ui-size", { width: 0, height: 0 });
  const measure = ctx.Modifier.onSizeChanged((value) => {
    if (value.width !== size.width || value.height !== size.height) setSize(value);
  });
  function text3(value, title = false, key) {
    return ctx.UI.Text({ key, text: value, style: title ? "titleMedium" : "bodyMedium", color: title ? colors.onSurface : colors.onSurfaceVariant });
  }
  function button(key, label, action, active = true) {
    return ctx.UI.TextButton({ key, height: 38, text: label, enabled: enabled && active, onClick: action });
  }
  function primary(key, label, action, active = true) {
    return ctx.UI.Button({ key, height: 38, text: label, enabled: enabled && active, onClick: action });
  }
  function iconButton(key, label, name, action, active = true) {
    return ctx.UI.IconButton({ key, tooltip: label, width: 32, height: 32, enabled: enabled && active, contentPadding: { horizontal: 6, vertical: 6 }, onClick: action }, ctx.UI.Icon({ name, size: 20, tint: colors.onSurfaceVariant }));
  }
  function sectionAction(key, label, name, action, active = true) {
    return ctx.UI.TextButton({ key, height: 32, enabled: enabled && active, contentPadding: { horizontal: 8, vertical: 4 }, onClick: action }, ctx.UI.Row({ spacing: 7, verticalAlignment: "center" }, [...name === "" ? [] : [ctx.UI.Icon({ name, size: 20, tint: colors.primary })], ctx.UI.Text({ text: label, fontSize: 13, fontWeight: "600", color: colors.primary })]));
  }
  function section(key, title, name, controls, children) {
    return ctx.UI.Box({ key, fillMaxWidth: true, paddingBottom: 14 }, ctx.UI.Surface({ fillMaxWidth: true, color: colors.surfaceContainer.copy({ alpha: 0.65 }), shape: { cornerRadius: 16 }, modifier: ctx.Modifier.border(1, colors.outlineVariant.copy({ alpha: 0.36 }), { cornerRadius: 16 }) }, ctx.UI.Column({ fillMaxWidth: true, padding: size.width <= 600 ? 12 : 14, spacing: 0 }, [
      ctx.UI.Row({ fillMaxWidth: true, height: 32, spacing: 10, verticalAlignment: "center" }, [ctx.UI.Icon({ name, size: 20, tint: colors.primary }), ctx.UI.Text({ text: title, fontSize: 16, fontWeight: "700", weight: 1, maxLines: 1 }), ctx.UI.FlowRow({ spacing: 2, runSpacing: 4, itemVerticalAlignment: "center" }, controls)]),
      ctx.UI.Spacer({ height: 12 }),
      ...children.flatMap((child, index) => index === 0 ? [child] : [ctx.UI.Spacer({ height: 8 }), child])
    ])));
  }
  function tile(key, title, subtitle, badges, controls, active = false, onClick, avatar, name = "PersonOutline") {
    const portraitImage = ctx.UI.Box({ width: 32, height: 32, contentAlignment: "center", modifier: ctx.Modifier.clip({ type: "circle" }).background(active ? colors.primaryContainer : colors.surfaceContainerHighest) }, avatar ? ctx.UI.Image({ uri: avatar, width: 32, height: 32, contentScale: "crop" }) : ctx.UI.Icon({ name, size: 24 }));
    const portrait = ctx.UI.Box({ width: 32, height: 32 }, [portraitImage, ...active && onClick ? [ctx.UI.Box({ modifier: ctx.Modifier.align("bottomEnd").clip({ type: "circle" }).background(colors.primary), width: 14, height: 14, contentAlignment: "center" }, ctx.UI.Icon({ name: "Check", size: 9, tint: colors.onPrimary }))] : []]);
    const content = ctx.UI.Row({ key: key + "-open", ...size.width > 440 ? { weight: 1 } : { fillMaxWidth: true }, verticalAlignment: "center", spacing: 10, ...enabled && onClick ? { onClick } : {} }, [portrait, ctx.UI.Column({ weight: 1, spacing: 4 }, [ctx.UI.Text({ text: title, fontSize: 14, fontWeight: "700", maxLines: 1, overflow: "ellipsis" }), ...subtitle === "" ? [] : [ctx.UI.Text({ text: subtitle, fontSize: 12, color: colors.onSurfaceVariant, maxLines: 1, overflow: "ellipsis" })], ctx.UI.FlowRow({ spacing: 6, runSpacing: 5, itemVerticalAlignment: "center" }, badges.map(badge))])]);
    const actionRow = ctx.UI.FlowRow({ ...size.width > 440 ? {} : { fillMaxWidth: true }, horizontalArrangement: "end", spacing: 4, runSpacing: 4, itemVerticalAlignment: "center" }, controls);
    const body = size.width > 440 ? ctx.UI.Row({ fillMaxWidth: true, paddingHorizontal: 12, paddingVertical: 10, spacing: 8, verticalAlignment: "center" }, [content, actionRow]) : ctx.UI.Column({ fillMaxWidth: true, paddingHorizontal: 12, paddingVertical: 10, spacing: 14 }, [content, actionRow]);
    return ctx.UI.Surface({ key, fillMaxWidth: true, color: active ? colors.primaryContainer.copy({ alpha: 0.16 }) : colors.surfaceContainerHighest.copy({ alpha: 0.28 }), shape: { cornerRadius: 12 }, modifier: ctx.Modifier.border(1, active ? colors.primary.copy({ alpha: 0.45 }) : colors.outlineVariant.copy({ alpha: 0.28 }), { cornerRadius: 12 }) }, body);
  }
  function badge(label) {
    const active = label === "当前使用", shape2 = { cornerRadius: 6 };
    const modifier = ctx.Modifier.clip(shape2).background((active ? colors.primary : colors.onSurfaceVariant).copy({ alpha: 0.08 }));
    return ctx.UI.Box({ height: 20, paddingHorizontal: 7, contentAlignment: "center", modifier: active ? modifier.border(1, colors.primary.copy({ alpha: 0.32 }), shape2) : modifier }, ctx.UI.Text({ text: label, fontSize: 11, fontWeight: active ? "600" : "400", color: active ? colors.primary : colors.onSurfaceVariant, maxLines: 1 }));
  }
  function activePill() {
    return badge("当前使用");
  }
  function actions(children) {
    return ctx.UI.FlowRow({ fillMaxWidth: true, spacing: 4, runSpacing: 4 }, children);
  }
  function field(key, label, value, change, lines = 1) {
    return ctx.UI.Box({ key: key + "-box", fillMaxWidth: true, paddingTop: 8, paddingBottom: 8 }, ctx.UI.TextField({ key, label, value, readOnly: !enabled, decoration: { filled: false, isDense: true, cornerRadius: 8, borderColor: colors.outline, focusedBorderColor: colors.primary, contentPadding: { horizontal: 12, vertical: 14 } }, style: { fontSize: 14 }, singleLine: lines === 1, minLines: lines, maxLines: lines === 1 ? 1 : Math.max(lines, 12), onValueChange: (value2) => {
      if (enabled) change(value2);
    }, fillMaxWidth: true }));
  }
  function toggle(key, label, checked, change) {
    return ctx.UI.Row({ key: key + "-row", fillMaxWidth: true, verticalAlignment: "center", spacing: 12, paddingVertical: 8 }, [ctx.UI.Text({ text: label, weight: 1 }), ctx.UI.Switch({ key, checked, enabled, onCheckedChange: change })]);
  }
  function list(key, children) {
    return ctx.UI.LazyColumn({ key, weight: 1, fillMaxWidth: true }, children.map((child, index) => ctx.UI.Box({ key: key + ":" + String(child.props?.key ?? index), fillMaxWidth: true, paddingHorizontal: 16, paddingVertical: 6 }, child)));
  }
  function choices(key, label, selected, options, change) {
    return ctx.UI.Column({ key, fillMaxWidth: true, spacing: 4 }, [...label === "" ? [] : [text3(label)], ctx.UI.FlowRow({ spacing: 6, runSpacing: 6 }, options.map((option) => ctx.UI.FilterChip({ key: key + ":" + option.id, selected: selected === option.id, enabled, label: ctx.UI.Text({ text: option.label, fontSize: 13 }), onClick: () => change(option.id) })))]);
  }
  function card(key, title, description, controls, onClick, avatar) {
    return ctx.UI.Surface({ key, fillMaxWidth: true, color: colors.surfaceContainer, shape: { cornerRadius: 16 } }, ctx.UI.Column({ fillMaxWidth: true, padding: 16, spacing: 8 }, [
      ctx.UI.Row({ fillMaxWidth: true, verticalAlignment: "center", spacing: 12, ...enabled && onClick ? { onClick } : {} }, [
        ...avatar ? [ctx.UI.Image({ uri: avatar, width: 44, height: 44, contentScale: "crop", modifier: ctx.Modifier.clip({ cornerRadius: 12 }) })] : [],
        ctx.UI.Column({ weight: 1, spacing: 4 }, [text3(title, true), ...description === "" ? [] : [text3(description)]])
      ]),
      actions(controls)
    ]));
  }
  function menu(key, label, name, entries, iconOnly = false) {
    return ctx.UI.MenuAnchor({ key, tooltip: label, itemHeight: 40, items: entries.map((entry) => ({ enabled: enabled && entry.enabled !== false })), anchor: ctx.UI.Row({ height: 32, paddingHorizontal: iconOnly ? 6 : 8, spacing: 7, verticalAlignment: "center" }, [ctx.UI.Icon({ name, size: 20, tint: colors.primary }), ...iconOnly ? [] : [ctx.UI.Text({ text: label, fontSize: 13, fontWeight: "600", color: colors.primary }), ctx.UI.Icon({ name: "ArrowDropDown", size: 16, tint: colors.primary })]]), onSelect: async (index) => {
      const entry = entries[index];
      if (enabled && entry !== void 0 && entry.enabled !== false) await entry.action();
    } }, entries.map((entry) => ctx.UI.Row({ key: entry.key, spacing: 10, verticalAlignment: "center" }, [ctx.UI.Icon({ name: entry.name, size: 20 }), ctx.UI.Text({ text: entry.label, fontSize: 13 })])));
  }
  function choose(key, label, selected, options, change, compact = false) {
    if (compact) return ctx.UI.MenuAnchor({ key, tooltip: label, items: options.map(() => ({ enabled })), anchor: ctx.UI.Row({ height: 32, paddingHorizontal: 10, spacing: 8, verticalAlignment: "center", modifier: ctx.Modifier.border(1, colors.outlineVariant, { cornerRadius: 8 }) }, [ctx.UI.Icon({ name: "FolderOutlined", size: 16 }), ctx.UI.Text({ text: options.find((option) => option.id === selected).label, fontSize: 13 }), ctx.UI.Icon({ name: "KeyboardArrowDown", size: 16 })]), onSelect: (index) => {
      if (enabled && options[index] !== void 0) change(options[index].id);
    } }, options.map((option) => ctx.UI.Text({ text: option.label, fontSize: 13 })));
    return ctx.UI.Box({ key: key + "-box", fillMaxWidth: true, paddingTop: 8, paddingBottom: 8 }, ctx.UI.DropdownMenu({ key, label, text: options.find((option) => option.id === selected)?.label ?? selected, enabled, fillMaxWidth: true, onClick: (index) => {
      if (enabled && options[index] !== void 0) change(options[index].id);
    } }, options.map((option) => ctx.UI.Text({ text: option.label, paddingVertical: 8 }))));
  }
  function score2(key, label, value, change) {
    return ctx.UI.Column({ fillMaxWidth: true, spacing: 4 }, [text3(label + " " + Number(value).toFixed(2)), ctx.UI.Slider({ key, fillMaxWidth: true, enabled, min: 0, max: 1, divisions: 100, value: Number(value), label: Number(value).toFixed(2), onValueChange: (number2) => change(number2.toFixed(2)) })]);
  }
  function dialog(key, title, body, footer, close, variant = "normal", onLoad, header, desiredHeight) {
    const phone = size.width <= 600, margin = phone && variant === "graph" ? 0 : phone ? 24 : 48;
    const width = Math.max(0, Math.min(variant === "compact" ? 500 : variant === "graph" ? 1400 : 760, size.width - margin));
    const height = Math.max(0, Math.min(desiredHeight ?? (variant === "compact" ? 240 : variant === "normal" ? 620 : variant === "large" ? 820 : size.height), size.height - margin));
    return ctx.UI.Dialog({ key, onLoad, closeOnDismissRequest: false, onDismissRequest: () => {
      if (enabled) close();
    }, containerColor: colors.surface.copy({ alpha: 0 }), properties: { fullscreen: true, usePlatformDefaultWidth: false, dismissOnClickOutside: false } }, ctx.UI.Surface({ width, height, color: variant === "graph" ? colors.surface : colors.surfaceContainerHigh, shape: { cornerRadius: phone && variant === "graph" ? 0 : phone ? 20 : 24 } }, ctx.UI.Column({ fillMaxSize: true }, [
      header ?? ctx.UI.Row({ fillMaxWidth: true, paddingStart: phone ? 18 : 22, paddingTop: phone ? 16 : 18, paddingEnd: 12, paddingBottom: 10, verticalAlignment: "center" }, [ctx.UI.Text({ text: title, fontSize: phone ? 18 : 20, fontWeight: "700", weight: 1 }), iconButton(key + "-close", "关闭", "Close", close)]),
      ...enabled ? [] : [ctx.UI.LinearProgressIndicator({ fillMaxWidth: true })],
      ctx.UI.Box({ weight: 1, fillMaxWidth: true }, body),
      ...footer.length === 0 ? [] : [ctx.UI.FlowRow({ fillMaxWidth: true, paddingHorizontal: 18, paddingTop: 12, paddingBottom: 18, horizontalArrangement: "end", spacing: 8, runSpacing: 8 }, footer)]
    ])));
  }
  return { primary, menu, size, measure, choose, score: score2, dialog, text: text3, button, iconButton, sectionAction, section, tile, activePill, actions, field, toggle, list, choices, card };
}

// src/ui-memory-graph.ts
function renderMemoryGraph(ctx, key, display, select, fail) {
  const controller = ctx.createWebViewController(key + ":controller");
  const [path, setPath] = ctx.useState(key + ":path", "");
  const current = ctx.useRef(key + ":display", display);
  current.current = display;
  const ready = ctx.useRef(key + ":ready", false);
  const active = ctx.useRef(key + ":active", true);
  const selectLive = ctx.useRef(key + ":select", select);
  selectLive.current = select;
  const origin = "https://memory-graph.operit.local/";
  ctx.useEffect(key + ":lifetime", () => {
    active.current = true;
    return () => {
      active.current = false;
      ready.current = false;
    };
  }, []);
  async function initialize() {
    ready.current = false;
    try {
      controller.addJavascriptInterface("MemoryGraphHost", {
        /** Returns an already-authorized graph projection without exposing service requests. */
        load: (...args) => {
          if (!active.current) throw new Error("Graph display is no longer active");
          if (args.length !== 1 || !Array.isArray(args[0]) || args[0].length !== 0) throw new Error("Graph load expects no arguments");
          ready.current = true;
          return current.current;
        },
        /** Validates canvas selections against the exact graph currently displayed. */
        select: (...args) => {
          if (!active.current) throw new Error("Graph display is no longer active");
          if (args.length !== 1 || !Array.isArray(args[0]) || args[0].length !== 2) throw new Error("Graph select expects kind and id");
          const [kind, id2] = args[0];
          if (typeof id2 !== "string" || kind !== "memory" && kind !== "edge") throw new Error("Invalid graph selection");
          const data = current.current.data;
          if (kind === "memory" ? !data.graph.nodes.some((node) => node.id === id2) : !data.graph.edges.some((edge) => edge.id === id2)) throw new Error("Graph selection no longer exists");
          selectLive.current(kind, id2);
          return null;
        }
      });
      const resource = await ToolPkg.readResource("character_memory_html", "character-memory.html");
      if (active.current) setPath(resource);
    } catch (error) {
      fail(String(error));
    }
  }
  ctx.useEffect(key + ":projection", () => {
    if (ready.current) void controller.evaluateJavascript("window.updateMemoryGraph(" + JSON.stringify(display) + ");").catch((error) => fail(String(error)));
    return () => {
    };
  }, [JSON.stringify(display)]);
  async function camera(command) {
    if (!ready.current) return;
    try {
      await controller.evaluateJavascript("window.memoryGraphCamera(" + JSON.stringify(command) + ");");
    } catch (error) {
      fail(String(error));
    }
  }
  return ctx.UI.Box({ key: "memory-graph-display", fillMaxSize: true }, [
    ctx.UI.Box({ key: "memory-graph-loader", fillMaxSize: true, onLoad: initialize }, path === "" ? ctx.UI.Text({ text: "正在加载记忆图谱…" }) : ctx.UI.WebView({
      key,
      controller,
      fillMaxSize: true,
      url: origin,
      javaScriptEnabled: true,
      domStorageEnabled: false,
      supportZoom: false,
      /** Allows only the offline graph document to navigate this view. */
      onShouldOverrideUrlLoading: (request2) => ({ action: request2.url === origin ? "allow" : "cancel" }),
      /** Serves a single local resource and blocks all other requests. */
      onInterceptRequest: (request2) => request2.url === origin ? { action: "respond", response: { mimeType: "text/html", encoding: "utf-8", statusCode: 200, reasonPhrase: "OK", filePath: path } } : { action: "block" },
      /** Stops updates when the display leaves the native composition. */
      onLifecycleEvent: (event) => {
        if (event.type === "Disposed") ready.current = false;
      }
    })),
    ctx.UI.Box({ modifier: ctx.Modifier.align("bottomStart"), padding: 12 }, ctx.UI.Surface({ shape: { cornerRadius: 12 }, color: ctx.MaterialTheme.colorScheme.surfaceContainerHigh }, ctx.UI.Row({}, [
      ...[["in", "放大", "Add"], ["out", "缩小", "Remove"], ["fit", "适应画布", "FitScreen"]].map(([command, label, icon]) => ctx.UI.IconButton({ key: "graph-" + command, tooltip: label, width: 36, height: 36, icon, onClick: () => camera(command) }))
    ])))
  ]);
}

// src/ui-memory-settings.ts
function renderMemorySettings(ctx, target, close, key) {
  const { state, live, update } = uiState(ctx, key, { settings: null, config: null, listing: null, queue: null, progress: null, tab: "automatic", busy: false, error: "", statusError: "", notice: "", numbers: {}, query: "", threshold: "0", result: null, selected: [], window: "32", from: "", to: "", clearFailed: false });
  const ui = nativeUi(ctx, !state.busy), ownerKey = target.ownerKey;
  const alive = ctx.useRef(key + ":alive", true);
  async function status() {
    const [queue, progress] = await Promise.all([uiRequest({ action: "readMemoryAutoSaveStatus", ownerKey }), uiRequest({ action: "readMemoryRebuildProgress", ownerKey })]);
    if (alive.current) update({ queue, progress, statusError: "" });
  }
  async function act(operation) {
    if (live.current.busy || !alive.current) return;
    update({ busy: true, error: "", notice: "" });
    try {
      await operation();
    } catch (error) {
      if (alive.current) update({ error: String(error) });
    } finally {
      if (alive.current) update({ busy: false });
    }
  }
  async function load() {
    await act(async () => {
      const [settings3, config2, listing, queue, progress] = await Promise.all([
        uiRequest({ action: "readMemorySettings", ownerKey }),
        uiRequest({ action: "readMemorySearchConfig", ownerKey }),
        uiRequest({ action: "listMemoryChats", ownerKey }),
        uiRequest({ action: "readMemoryAutoSaveStatus", ownerKey }),
        uiRequest({ action: "readMemoryRebuildProgress", ownerKey })
      ]);
      assertMemorySettings(settings3);
      assertSearchConfig(config2);
      if (!alive.current) return;
      update({ settings: settings3, config: config2, listing, queue, progress, numbers: { interval: String(settings3.autoSaveIntervalMinutes), keywordWeight: String(config2.keywordWeight), tagWeight: String(config2.tagWeight), vectorWeight: String(config2.vectorWeight), edgeWeight: String(config2.edgeWeight) } });
    });
  }
  ctx.useEffect(key + ":poll", () => {
    alive.current = true;
    let stopped = false, timer;
    async function tick() {
      try {
        if (!stopped && !live.current.busy && live.current.settings !== null) await status();
      } catch (error) {
        if (!stopped) update({ statusError: "状态读取失败：" + String(error) });
      } finally {
        if (!stopped) timer = setTimeout(tick, 3e3);
      }
    }
    timer = setTimeout(tick, 3e3);
    return () => {
      stopped = true;
      alive.current = false;
      clearTimeout(timer);
    };
  }, []);
  function settingsChange(patch) {
    if (live.current.settings !== null && !live.current.busy) update({ settings: { ...live.current.settings, ...patch } });
  }
  function numberChange(name, value) {
    update({ numbers: { ...live.current.numbers, [name]: value } });
  }
  function configDraft() {
    if (live.current.config === null) throw new Error("检索设置尚未读取成功");
    const draft = { ...live.current.config };
    for (const name of ["keywordWeight", "tagWeight", "vectorWeight", "edgeWeight"]) draft[name] = uiNumber(live.current.numbers[name], name, 0);
    assertSearchConfig(draft);
    return draft;
  }
  async function save() {
    await act(async () => {
      if (live.current.settings === null) throw new Error("记忆设置尚未读取成功");
      const settings3 = { ...live.current.settings, autoSaveIntervalMinutes: uiNumber(live.current.numbers.interval, "检查间隔", 1, 30, true) }, config2 = configDraft();
      assertMemorySettings(settings3);
      const result = await uiRequest({ action: "writeMemoryOwnerSettings", ownerKey, settings: settings3, config: config2 });
      if (alive.current) update({ settings: result.settings, config: result.config, notice: "设置已保存" });
      await status();
    });
  }
  const children = [ui.text("所属记忆库：" + ownerKey)];
  if (state.error !== "") children.push(ctx.UI.Text({ key: "memory-settings-error", text: state.error, color: ctx.MaterialTheme.colorScheme.error }));
  if (state.statusError !== "") children.push(ui.text(state.statusError));
  if (state.notice !== "") children.push(ui.text(state.notice));
  const settings2 = state.settings, config = state.config;
  if (settings2 === null || config === null || state.listing === null) children.push(ui.text(state.busy ? "正在读取记忆设置…" : "记忆设置尚未读取成功"), ui.button("memory-settings-retry", "重试", load));
  else {
    if (state.listing.missingChatIds.length > 0) children.push(ui.text(`检测到 ${state.listing.missingChatIds.length} 条已失效的会话绑定，自动提取与历史重建会跳过它们。`), ui.button("memory-prune-bindings", "清理失效绑定", () => act(async () => {
      const result = await uiRequest({ action: "pruneMissingChatBindings", ownerKey });
      const listing = await uiRequest({ action: "listMemoryChats", ownerKey });
      if (alive.current) update({ listing, notice: `已清理 ${result.removedChatIds.length} 条会话绑定和 ${result.removedCandidateIds.length} 条提取候选` });
    })));
    if (state.tab === "automatic") {
      const queue = state.queue;
      if (queue !== null) {
        children.push(ui.text(`待处理 ${queue.pendingCandidates} 条 / ${queue.pendingChats} 个聊天 · 处理中 ${queue.processingCandidates} · 失败 ${queue.failedCandidates}`), ui.text(queue.nextRunAtMs === 0 ? "尚未安排下次检查" : `下次检查：约 ${queue.minutesUntilNextRun} 分钟后`));
        if (queue.lastError !== "") children.push(ui.text("提取失败：" + queue.lastError));
        if (queue.failedCandidates > 0) children.push(ui.actions([
          ui.button("memory-retry-failed", "重试失败候选", () => act(async () => {
            const result = await uiRequest({ action: "retryFailedCandidates", ownerKey });
            if (alive.current) update({ notice: `已重新排队 ${result.retried} 条失败候选` });
            await status();
          })),
          ui.button("memory-clear-failed", state.clearFailed ? "确认清除失败记录" : "清除失败记录", () => state.clearFailed ? act(async () => {
            const result = await uiRequest({ action: "clearFailedCandidates", ownerKey });
            if (alive.current) update({ clearFailed: false, notice: `已清除 ${result.cleared} 条失败记录` });
            await status();
          }) : update({ clearFailed: true })),
          ...state.clearFailed ? [ui.button("memory-clear-failed-cancel", "取消清除", () => update({ clearFailed: false }))] : []
        ]));
      }
      children.push(ui.text("至少积累 5 个候选后自动提取；清除失败记录不会删除已有记忆。"), ui.field("memory-interval", "检查间隔（1–30 分钟）", state.numbers.interval, (value) => numberChange("interval", value)), ui.field("memory-extraction-rules", "记忆提取自定义规则", settings2.memoryExtractionCustomRules, (memoryExtractionCustomRules) => settingsChange({ memoryExtractionCustomRules }), 5), ui.toggle("memory-profile-auto", "自动更新 USER.md", settings2.profileAutoUpdateEnabled, (profileAutoUpdateEnabled) => settingsChange({ profileAutoUpdateEnabled })), ui.toggle("memory-profile-locked", "锁定 USER.md，禁止模型覆盖", settings2.profileAutoUpdateLocked, (profileAutoUpdateLocked) => settingsChange({ profileAutoUpdateLocked })));
    } else if (state.tab === "search") {
      for (const [name, label] of [["keywordWeight", "关键词权重"], ["tagWeight", "标签权重"], ["vectorWeight", "向量权重"], ["edgeWeight", "关系权重"]]) children.push(ui.field("memory-" + name, label, state.numbers[name], (value) => numberChange(name, value)));
      children.push(ui.toggle("memory-embedding", "云端 Embedding", settings2.cloudEmbeddingEnabled, (cloudEmbeddingEnabled) => settingsChange({ cloudEmbeddingEnabled })));
      if (settings2.cloudEmbeddingEnabled) children.push(ui.text("搜索文本与记忆文本会发送至指定服务。修改服务地址、模型或密钥后，请先保存再重建。"), ui.field("memory-embedding-endpoint", "Embedding 完整请求地址", settings2.cloudEmbeddingEndpoint, (cloudEmbeddingEndpoint) => settingsChange({ cloudEmbeddingEndpoint })), ui.field("memory-embedding-model", "Embedding 模型", settings2.cloudEmbeddingModel, (cloudEmbeddingModel) => settingsChange({ cloudEmbeddingModel })), ctx.UI.TextField({ key: "memory-embedding-key", label: "API Key", value: settings2.cloudEmbeddingApiKey, enabled: !state.busy, isPassword: true, fillMaxWidth: true, onValueChange: (cloudEmbeddingApiKey) => settingsChange({ cloudEmbeddingApiKey }) }), ui.button("memory-embedding-rebuild", "使用已保存设置重建 Embedding", () => act(async () => {
        const count = await uiRequest({ action: "rebuildMemoryEmbeddings", ownerKey });
        if (alive.current) update({ notice: `已重建 ${count} 条记忆的 Embedding` });
      })));
      children.push(ui.text("检索调试", true), ui.field("memory-debug-query", "查询", state.query, (query) => update({ query })), ui.field("memory-debug-threshold", "相关性阈值（0–1）", state.threshold, (threshold) => update({ threshold })), ui.text("使用当前权重执行真实检索，不保存权重；向量检索使用已保存的服务配置。"), ui.button("memory-debug-search", "使用当前权重检索", () => act(async () => {
        const result = await uiRequest({ action: "searchMemoryDebug", ownerKey, query: live.current.query, config: configDraft(), relevanceThreshold: uiNumber(live.current.threshold, "相关性阈值", 0, 1) });
        if (alive.current) update({ result });
      })));
      if (state.result !== null) {
        children.push(ui.text(`范围 ${state.result.memoriesInScopeCount} 条 · 命中 ${state.result.candidates.filter((hit) => hit.passedThreshold).length} 条`));
        for (const hit of state.result.candidates) children.push(ui.card("memory-hit:" + hit.memory.uuid, hit.memory.title, `总分 ${hit.score.toFixed(4)} · ${hit.passedThreshold ? "命中" : "未过阈值"}
关键词 ${hit.keywordScore.toFixed(4)} · 标签 ${hit.tagScore.toFixed(4)} · 向量 ${hit.vectorScore.toFixed(4)} · 关系 ${hit.edgeScore.toFixed(4)}`, []));
      }
    } else {
      const progress = state.progress, running = progress !== null && (progress.status === "preparing" || progress.status === "running");
      children.push(ui.text("从聊天历史重建记忆：追加或更新，保留现有记忆。关闭此页不会终止后台任务。"), ui.field("memory-rebuild-window", "窗口消息数", state.window, (window) => update({ window })), ui.field("memory-rebuild-from", "开始日期（YYYY-MM-DD，可留空）", state.from, (from) => update({ from })), ui.field("memory-rebuild-to", "结束日期（YYYY-MM-DD，可留空）", state.to, (to) => update({ to })));
      for (const chat of state.listing.chats) children.push(ctx.UI.Row({ key: "memory-rebuild-chat:" + chat.id, fillMaxWidth: true, spacing: 8, verticalAlignment: "center" }, [ctx.UI.Checkbox({ key: "memory-rebuild-check:" + chat.id, checked: state.selected.includes(chat.id), enabled: !state.busy && !running, onCheckedChange: (checked) => update({ selected: checked ? [.../* @__PURE__ */ new Set([...live.current.selected, chat.id])] : live.current.selected.filter((id2) => id2 !== chat.id) }) }), ctx.UI.Column({ weight: 1, spacing: 4 }, [ui.text(chat.title), ui.text(`${chat.messageCount} 条消息`)])]));
      if (state.listing.chats.length === 0) children.push(ui.text("此记忆库暂无绑定聊天"));
      children.push(ui.actions([ui.button("memory-rebuild-start", "开始重建", () => act(async () => {
        const value = live.current, fromInclusive = uiDate(value.from, false), toInclusive = uiDate(value.to, true);
        if (fromInclusive !== null && toInclusive !== null && fromInclusive > toInclusive) throw new Error("开始日期不能晚于结束日期");
        const result = await uiRequest({ action: "startMemoryRebuild", ownerKey, rebuild: { chatIds: [...value.selected], windowMessageCount: uiNumber(value.window, "窗口消息数", 1, Infinity, true), fromInclusive, toInclusive } });
        if (alive.current) update({ progress: result, notice: "后台重建任务已提交" });
      }), !running && state.selected.length > 0), ui.button("memory-rebuild-cancel", "取消重建", () => act(async () => {
        const progress2 = await uiRequest({ action: "cancelMemoryRebuild", ownerKey });
        if (alive.current) update({ progress: progress2 });
      }), running), ui.button("memory-status-refresh", "刷新状态", () => act(status))]));
      if (progress !== null) {
        const labels = { idle: "未开始", preparing: "准备中", running: "重建中", completed: "已完成", cancelled: "已取消", failed: "失败" };
        children.push(ui.text(`${labels[progress.status] ?? progress.status} · 聊天 ${progress.completedChats}/${progress.totalChats} · 窗口 ${progress.completedWindows}/${progress.totalWindows} · 失败 ${progress.failedWindows}`), ui.text(`已处理源消息 ${progress.processedSourceMessages}/${progress.totalSourceMessages} · ${progress.currentChatTitle}`));
        if (progress.lastError !== "") children.push(ui.text(progress.lastError));
      }
    }
  }
  const tabs = [{ id: "automatic", label: "自动提取" }, { id: "search", label: "检索" }, { id: "rebuild", label: "历史重建" }];
  return ui.dialog("memory-settings-root", target.name + " · 记忆设置", ctx.UI.Column({ fillMaxSize: true }, [
    ctx.UI.PrimaryTabRow({ key: "memory-settings-tabs", fillMaxWidth: true, selectedTabIndex: tabs.findIndex((tab) => tab.id === state.tab), tabs: tabs.map((tab) => ctx.UI.Tab({ key: "memory-settings-tabs:" + tab.id, selected: state.tab === tab.id, enabled: !state.busy, onClick: () => update({ tab: tab.id }) }, ctx.UI.Text({ text: tab.label, style: "titleSmall" }))) }),
    ui.list("memory-settings-body", children)
  ]), [ui.button("memory-settings-back", "关闭", close), ...settings2 !== null && config !== null && state.listing !== null ? [ui.primary("memory-settings-save", "保存", save)] : []], close, "large", load);
}

// src/ui-memory-workspace.ts
function filteredMemoryGraph(data, folder, searchIds) {
  const items = data.items.filter((item) => (folder === "*" || (item.folderPath ?? "") === folder) && (searchIds === null || searchIds.includes(item.uuid)));
  const ids = new Set(items.map((item) => item.uuid));
  if (searchIds !== null) {
    const matches = new Set(ids);
    for (const edge of data.graph.edges) {
      if (matches.has(edge.sourceId)) ids.add(edge.targetId);
      if (matches.has(edge.targetId)) ids.add(edge.sourceId);
    }
  }
  return { items, graph: { nodes: data.graph.nodes.filter((node) => ids.has(node.id)), edges: data.graph.edges.filter((edge) => ids.has(edge.sourceId) && ids.has(edge.targetId)) } };
}
function renderMemoryWorkspace(ctx, target, close, instance) {
  const key = instance + ":" + target.ownerKey;
  const { state, live, update } = uiState(ctx, key, { data: null, panel: target.initial === "settings" ? { type: "settings" } : target.initial === "profile" ? { type: "profile", content: "", loaded: false } : null, busy: false, error: "", notice: "", query: "", appliedQuery: "", searchIds: null, folder: "*", inspectorExpanded: false, selected: null, selectedEdge: null, linkSource: null, linking: false, panelSequence: 0 });
  const ui = nativeUi(ctx, !state.busy), ownerKey = target.ownerKey;
  const alive = ctx.useRef(key + ":alive", true);
  ctx.useEffect(key + ":lifetime", () => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);
  async function act(operation) {
    if (live.current.busy || !alive.current) return;
    update({ busy: true, error: "", notice: "" });
    try {
      await operation();
    } catch (error) {
      if (alive.current) update({ error: String(error) });
    } finally {
      if (alive.current) update({ busy: false });
    }
  }
  function open(panel) {
    update({ panel, panelSequence: live.current.panelSequence + 1, error: "" });
  }
  function back() {
    if (live.current.busy) return;
    if (target.initial !== void 0 && target.initial !== "browse" && live.current.data === null) close();
    else update({ panel: null, error: "" });
  }
  async function refresh() {
    const data = await uiRequest({ action: "graph", ownerKey });
    if (alive.current) update({ data, searchIds: null, query: "", appliedQuery: "", selected: null, selectedEdge: null, linkSource: null });
  }
  async function profile() {
    open({ type: "profile", content: "", loaded: false });
    await act(async () => {
      const result = await uiRequest({ action: "readUser", ownerKey });
      if (alive.current) update({ panel: { type: "profile", content: result.content, loaded: true } });
    });
  }
  async function load() {
    if (target.initial === "profile") {
      await profile();
      return;
    }
    if (target.initial === "settings" || live.current.data !== null) return;
    await act(refresh);
  }
  function editMemory(item) {
    open({ type: "memory", draft: item === null ? { originalTitle: null, title: "", content: "", contentType: "text/plain", source: "manual", credibility: "0.5", importance: "0.5", folderPath: live.current.folder === "*" ? "" : live.current.folder, tags: "" } : { originalTitle: item.title, title: item.title, content: item.content, contentType: item.contentType, source: item.source, credibility: String(item.credibility), importance: String(item.importance), folderPath: item.folderPath ?? "", tags: item.tags.map((tag) => tag.name).join(",") } });
  }
  function newLink(sourceTitle = "", targetTitle = "") {
    open({ type: "link", draft: { id: null, sourceTitle, targetTitle, linkType: "related", weight: "1", description: "" } });
  }
  async function editLink(id2) {
    await act(async () => {
      const link = await uiRequest({ action: "readMemoryLink", ownerKey, linkId: id2 });
      if (!alive.current) return;
      const source = live.current.data?.items.find((item) => item.id === link.sourceMemoryId), destination = live.current.data?.items.find((item) => item.id === link.targetMemoryId);
      if (source === void 0 || destination === void 0) throw new Error("记忆关系端点不存在");
      open({ type: "link", draft: { id: id2, sourceTitle: source.title, targetTitle: destination.title, linkType: link.type_, weight: String(link.weight), description: link.description } });
    });
  }
  function select(kind, id2) {
    if (live.current.busy || live.current.panel !== null) return;
    if (kind === "edge") {
      update({ selected: null, selectedEdge: id2 });
      return;
    }
    if (!live.current.linking) {
      update({ selected: id2, selectedEdge: null });
      return;
    }
    const first = live.current.linkSource;
    if (first === null) {
      update({ linkSource: id2 });
      return;
    }
    if (first === id2) return;
    const source = live.current.data?.items.find((item) => item.uuid === first), destination = live.current.data?.items.find((item) => item.uuid === id2);
    if (source === void 0 || destination === void 0) throw new Error("记忆节点不存在");
    newLink(source.title, destination.title);
  }
  function memoryChange(patch) {
    const panel = live.current.panel;
    if (!live.current.busy && panel?.type === "memory") update({ panel: { ...panel, draft: { ...panel.draft, ...patch } } });
  }
  function linkChange(patch) {
    const panel = live.current.panel;
    if (!live.current.busy && panel?.type === "link") update({ panel: { ...panel, draft: { ...panel.draft, ...patch } } });
  }
  async function saveMemory() {
    await act(async () => {
      const panel = live.current.panel;
      if (panel?.type !== "memory") return;
      const draft = panel.draft;
      const data = await uiRequest({ action: "saveMemory", ownerKey, ...draft, credibility: uiNumber(draft.credibility, "可信度", 0, 1), importance: uiNumber(draft.importance, "重要性", 0, 1) });
      if (alive.current) update({ data, panel: null, searchIds: null, query: "", appliedQuery: "", selected: null, notice: "记忆已保存" });
    });
  }
  async function saveLink() {
    await act(async () => {
      const panel = live.current.panel;
      if (panel?.type !== "link") return;
      const draft = panel.draft, weight = uiNumber(draft.weight, "关系强度", 0, 1);
      if (draft.linkType.trim() === "" || draft.sourceTitle === "" || draft.targetTitle === "" || draft.sourceTitle === draft.targetTitle) throw new Error("请选择两条不同记忆并填写关系类型");
      const data = await uiRequest(draft.id === null ? { action: "createLink", ownerKey, sourceTitle: draft.sourceTitle, targetTitle: draft.targetTitle, linkType: draft.linkType, weight, description: draft.description } : { action: "updateLink", ownerKey, linkId: draft.id, linkType: draft.linkType, weight, description: draft.description });
      if (alive.current) update({ data, panel: null, selectedEdge: null, linkSource: null, linking: false, notice: "关系已保存" });
    });
  }
  function panelBody(panel) {
    switch (panel.type) {
      case "memory": {
        const draft = panel.draft;
        return [
          ui.text(draft.originalTitle === null ? "创建记忆" : "编辑记忆", true),
          ui.field("memory-title", "标题 *", draft.title, (title) => memoryChange({ title })),
          ui.field("memory-content", "内容", draft.content, (content) => memoryChange({ content }), 8),
          ui.field("memory-content-type", "内容类型", draft.contentType, (contentType) => memoryChange({ contentType })),
          ui.field("memory-source", "来源", draft.source, (source) => memoryChange({ source })),
          ui.score("memory-credibility", "可信度", draft.credibility, (credibility) => memoryChange({ credibility })),
          ui.score("memory-importance", "重要性", draft.importance, (importance) => memoryChange({ importance })),
          ui.field("memory-folder", "文件夹", draft.folderPath, (folderPath) => memoryChange({ folderPath })),
          ui.field("memory-tags", "标签（逗号分隔）", draft.tags, (tags) => memoryChange({ tags })),
          ui.primary("memory-save", "保存", saveMemory)
        ];
      }
      case "link": {
        const draft = panel.draft, options = (state.data?.items ?? []).map((item) => ({ id: item.title, label: item.title }));
        return [ui.text(draft.id === null ? "创建记忆关系" : "编辑记忆关系", true), ...draft.id === null ? [ui.choose("link-source", "源记忆", draft.sourceTitle, options, (sourceTitle) => linkChange({ sourceTitle })), ui.choose("link-target", "目标记忆", draft.targetTitle, options, (targetTitle) => linkChange({ targetTitle }))] : [ui.text(draft.sourceTitle + " → " + draft.targetTitle)], ui.field("link-type", "关系类型", draft.linkType, (linkType) => linkChange({ linkType })), ui.field("link-weight", "强度（0–1）", draft.weight, (weight) => linkChange({ weight })), ui.field("link-description", "描述", draft.description, (description) => linkChange({ description }), 3), ui.primary("link-save", "保存", saveLink)];
      }
      case "profile":
        return [ui.text("用户资料 · USER.md", true), ...panel.loaded ? [ui.field("memory-profile", "用户资料", panel.content, (content) => update({ panel: { ...panel, content } }), 12), ui.primary("memory-profile-save", "保存", () => act(async () => {
          const current = live.current.panel;
          if (current?.type !== "profile" || !current.loaded) throw new Error("用户资料尚未读取成功");
          await uiRequest({ action: "writeUser", ownerKey, content: current.content });
          if (alive.current) {
            if (live.current.data === null) close();
            else update({ panel: null, notice: "用户资料已保存" });
          }
        }))] : [ui.text(state.busy ? "正在读取用户资料…" : "用户资料读取失败"), ui.button("memory-profile-retry", "重试", profile)]];
      case "import":
        return [ui.text("导入记忆 JSON", true), ui.choose("memory-import-strategy", "相同 UUID 记忆处理", panel.strategy, [{ id: "SKIP", label: "跳过" }, { id: "UPDATE", label: "更新" }, { id: "CREATE_NEW", label: "创建新记忆" }], (strategy) => update({ panel: { ...panel, strategy } })), ui.text("选择记忆库导出的 JSON 备份文件，导入完整记忆、关系、文档分块及 USER.md。导入前建议先导出备份；只修改当前记忆库。"), ui.primary("memory-import-submit", "选择 JSON 文件并导入", () => act(async () => {
          const current = live.current.panel;
          if (current?.type !== "import") return;
          const content = await readUiJson(ctx);
          if (content === null) return;
          const result = await uiRequest({ action: "importMemory", ownerKey, content, strategy: current.strategy });
          await refresh();
          if (alive.current) update({ panel: null, notice: `新增 ${result.result.newMemories} · 更新 ${result.result.updatedMemories} · 跳过 ${result.result.skippedMemories} · 关系 ${result.result.newLinks}` });
        }))];
      case "export":
        return [ui.text("导出 JSON", true), ctx.UI.TextField({ key: "memory-export-content", label: "JSON 内容", value: panel.content, readOnly: true, onValueChange: () => {
        }, fillMaxWidth: true, minLines: 16, maxLines: 24 }), ui.text("包含当前记忆库的完整记录与 USER.md。点击保存文件选择保存位置。"), ui.primary("memory-export-save", "保存文件", () => act(async () => {
          const current = live.current.panel;
          if (current?.type !== "export") return;
          const result = await ctx.saveFile({ suggestedName: current.name, content: current.content, mimeType: "application/json", extensions: ["json"] });
          if (!result.cancelled && alive.current) update({ panel: null, notice: "文件已导出" });
        }))];
      case "delete-memory":
        return [ui.text("确定删除记忆「" + panel.title + "」及关联关系？"), ui.primary("memory-delete-confirm", "确认删除", () => act(async () => {
          const data = await uiRequest({ action: "deleteMemory", ownerKey, title: panel.title });
          if (alive.current) update({ data, panel: null, selected: null, selectedEdge: null, searchIds: null, notice: "记忆已删除" });
        }))];
      case "delete-link":
        return [ui.text("确定删除这条记忆关系？"), ui.primary("link-delete-confirm", "确认删除", () => act(async () => {
          const data = await uiRequest({ action: "deleteLink", ownerKey, linkId: panel.id });
          if (alive.current) update({ data, panel: null, selectedEdge: null, notice: "关系已删除" });
        }))];
    }
  }
  async function search() {
    await act(async () => {
      const query = live.current.query.trim();
      const hits = query === "" ? null : await uiRequest({ action: "searchMemory", ownerKey, query });
      if (alive.current) update({ searchIds: hits?.map((item) => item.uuid) ?? null, appliedQuery: query, selected: null, selectedEdge: null });
    });
  }
  const messages = [];
  if (state.error !== "") messages.push(ctx.UI.Text({ key: "memory-workspace-error", text: state.error, color: ctx.MaterialTheme.colorScheme.error }));
  if (state.notice !== "") messages.push(ui.text(state.notice));
  const layers = [];
  const direct = target.initial !== void 0 && target.initial !== "browse" && state.data === null;
  if (!direct) {
    const data = state.data, narrow = ui.size.width <= 840;
    const projection = data === null ? null : filteredMemoryGraph(data, state.folder, state.searchIds);
    const selected = data?.items.find((item) => item.uuid === state.selected), edge = data?.graph.edges.find((item) => item.id === state.selectedEdge);
    const header = ctx.UI.Row({ fillMaxWidth: true, paddingHorizontal: 12, paddingVertical: 8, spacing: 8, verticalAlignment: "center" }, [
      ui.iconButton("memory-workspace-back", "关闭", "ArrowBack", close),
      ctx.UI.Column({ weight: 1 }, [ctx.UI.Text({ text: "记忆图谱", fontSize: 18, fontWeight: "600" }), ctx.UI.Text({ text: target.name, fontSize: 12, color: ctx.MaterialTheme.colorScheme.onSurfaceVariant, maxLines: 1, overflow: "ellipsis" })]),
      ui.menu("memory-more", "更多操作", "MoreVert", [
        { key: "memory-settings-open", label: "记忆设置", name: "Tune", action: () => open({ type: "settings" }) },
        { key: "memory-refresh", label: "刷新", name: "Refresh", enabled: data !== null, action: () => act(refresh) },
        { key: "memory-import", label: "导入记忆 JSON", name: "Upload", enabled: data !== null, action: () => open({ type: "import", strategy: "SKIP", content: "" }) },
        { key: "memory-export", label: "导出记忆 JSON", name: "Download", enabled: data !== null, action: () => act(async () => {
          const result = await uiRequest({ action: "exportMemory", ownerKey });
          if (alive.current) open({ type: "export", name: "memory-backup.json", content: result.content });
        }) }
      ], true)
    ]);
    let body;
    if (data === null || projection === null) body = ui.list("memory-loading", [...messages, ui.text(state.busy ? "正在读取记忆图谱…" : "记忆图谱未加载成功，未开放写入。"), ui.button("memory-load-retry", "重试", () => act(refresh))]);
    else {
      const folders = [.../* @__PURE__ */ new Set(["", ...data.items.map((item) => item.folderPath ?? "")])].sort();
      const filters = ctx.UI.Column({ fillMaxWidth: true, paddingHorizontal: 12, paddingBottom: 10, spacing: 8 }, [
        ctx.UI.Row({ fillMaxWidth: true, height: 40, verticalAlignment: "center", spacing: 4, modifier: ctx.Modifier.clip({ cornerRadius: 12 }).background(ctx.MaterialTheme.colorScheme.surfaceContainerHigh) }, [ui.iconButton("memory-search", "搜索", "Search", search), ctx.UI.TextField({ key: "memory-search-query", placeholder: "搜索记忆…", decoration: { borderless: true, isDense: true, contentPadding: { horizontal: 4, vertical: 8 } }, style: { fontSize: 14 }, value: state.query, readOnly: state.busy, singleLine: true, weight: 1, onValueChange: (query) => update({ query }) }), ui.iconButton("memory-search-clear", "清除搜索", "Close", () => update({ query: "", appliedQuery: "", searchIds: null }))]),
        ctx.UI.Row({ fillMaxWidth: true, verticalAlignment: "center", spacing: 12 }, [ui.choose("memory-folder-filter", "文件夹", state.folder, [{ id: "*", label: "全部文件夹" }, ...folders.map((folder) => ({ id: folder, label: folder === "" ? "根目录" : folder }))], (folder) => update({ folder, selected: null, selectedEdge: null }), true), ctx.UI.FilterChip({ key: "memory-graph-link-mode", selected: state.linking, enabled: !state.busy && data.items.length > 1, label: ctx.UI.Text({ text: "链接" }), onClick: () => update({ linking: !live.current.linking, linkSource: null }) })]),
        ...state.linking ? [ui.text(state.linkSource === null ? "选择两条记忆以建立链接 · 请选择源记忆" : "请选择目标记忆")] : [],
        ...state.panel === null ? messages : []
      ]);
      const details = [];
      if (selected !== void 0) {
        details.push(ui.text("记忆详情"), ui.text(selected.title, true), ui.text(`${selected.contentType === "text/plain" ? "文本" : selected.contentType} · ${selected.source === "manual" ? "手动记录" : selected.source} · 可信度 ${selected.credibility.toFixed(2)} · 重要性 ${selected.importance.toFixed(2)}`), ui.text(selected.content), ui.actions([ui.button("memory-edit-selected", "编辑", () => editMemory(selected)), ui.button("memory-delete-selected", "删除", () => open({ type: "delete-memory", title: selected.title }))]));
        for (const relation of data.graph.edges.filter((item) => item.sourceId === selected.uuid || item.targetId === selected.uuid)) details.push(ctx.UI.Row({ fillMaxWidth: true, verticalAlignment: "center" }, [ctx.UI.Text({ text: `${relation.label ?? ""} · ${relation.weight.toFixed(2)}`, weight: 1 }), ui.iconButton("link-delete:" + relation.id, "删除关系", "DeleteOutline", () => open({ type: "delete-link", id: relation.id }))]));
      } else if (edge !== void 0) details.push(ui.text("关系详情"), ui.text("记忆关系", true), ui.text(edge.label ?? ""), ui.text("强度 " + edge.weight.toFixed(2)), ui.text(String(edge.metadata.description ?? "")), ui.actions([ui.button("link-edit-selected", "编辑关系", () => editLink(edge.id)), ui.button("link-delete-selected", "删除关系", () => open({ type: "delete-link", id: edge.id }))]));
      const items = projection.items.map((item) => ctx.UI.Row({ key: "memory-select:" + item.uuid, fillMaxWidth: true, verticalAlignment: "center", spacing: 10, padding: 10, onClick: () => select("memory", item.uuid) }, [ctx.UI.Icon({ name: selected?.uuid === item.uuid ? "Check" : "PsychologyOutlined", size: 22 }), ctx.UI.Column({ weight: 1 }, [ctx.UI.Text({ text: item.title, fontSize: 13, fontWeight: "500" }), ctx.UI.Text({ text: item.content.slice(0, 70), fontSize: 12, color: ctx.MaterialTheme.colorScheme.onSurfaceVariant, maxLines: 2, overflow: "ellipsis" })])]));
      const inspector = ctx.UI.Column({ key: "memory-inspector", ...narrow ? { weight: state.inspectorExpanded ? 1 : 2 } : { width: 304, fillMaxHeight: true }, modifier: ctx.Modifier.background(ctx.MaterialTheme.colorScheme.surfaceContainer) }, [ctx.UI.Row({ fillMaxWidth: true, paddingHorizontal: 16, paddingVertical: 8, verticalAlignment: "center" }, [ctx.UI.Icon({ name: "PsychologyOutlined", size: 18 }), ctx.UI.Text({ text: `记忆库 ${projection.items.length}`, fontSize: 14, fontWeight: "600", weight: 1 }), ...narrow ? [ui.button("memory-inspector-toggle", state.inspectorExpanded ? "收起" : "展开", () => update({ inspectorExpanded: !live.current.inspectorExpanded }))] : []]), ui.list("memory-inspector-list", [...details, ...items, ...items.length === 0 ? [ui.text("没有匹配的记忆")] : []])]);
      const canvas = ctx.UI.Box({ key: "memory-canvas", weight: narrow ? 3 : 1, fillMaxWidth: narrow, fillMaxHeight: !narrow }, [
        ...state.panel === null ? [renderMemoryGraph(ctx, key + ":canvas", { data: projection, selected: state.linkSource ?? state.selected, selectedEdge: state.selectedEdge, theme: ctx.Theme.getCurrent() }, select, (error) => update({ error }))] : [],
        ctx.UI.Box({ modifier: ctx.Modifier.align("bottomEnd"), padding: 16 }, ctx.UI.FloatingActionButton({ key: "memory-create", icon: "Add", text: "创建记忆", onClick: () => editMemory(null) }))
      ]);
      body = ctx.UI.Column({ fillMaxSize: true }, [filters, ctx.UI.Box({ weight: 1, fillMaxWidth: true }, [
        (narrow ? ctx.UI.Column : ctx.UI.Row)({ fillMaxSize: true }, [...narrow && state.inspectorExpanded ? [] : [canvas], inspector])
      ])]);
    }
    layers.push(ui.dialog("memory-graph-dialog", "记忆图谱", body, [], close, "graph", void 0, header));
  }
  if (state.panel?.type === "settings") layers.push(renderMemorySettings(ctx, target, back, key + ":settings:" + state.panelSequence));
  else if (state.panel !== null) {
    const panel = state.panel, fields2 = panelBody(panel), titleNode = fields2.shift();
    const submit = fields2.pop();
    const title = panel.type === "memory" ? panel.draft.originalTitle === null ? "创建记忆" : "编辑记忆" : panel.type === "link" ? panel.draft.id === null ? "创建记忆关系" : "编辑记忆关系" : panel.type === "profile" ? "用户资料 · USER.md" : panel.type === "import" ? "导入记忆 JSON" : panel.type === "export" ? "导出 JSON" : panel.type === "delete-memory" ? "删除记忆" : "删除关系";
    const confirmation = panel.type === "delete-memory" || panel.type === "delete-link";
    layers.push(ui.dialog("memory-form-dialog", title, ui.list("memory-native-form", [...messages, ...confirmation && titleNode ? [titleNode] : [], ...fields2]), [ui.button("memory-form-cancel", "取消", back), submit], back, panel.type === "memory" || panel.type === "profile" || panel.type === "export" ? "large" : confirmation || panel.type === "import" ? "compact" : "normal", void 0, void 0, panel.type === "import" ? 360 : void 0));
  }
  return ctx.UI.Box({ key: "memory-workspace-root", fillMaxSize: true, onLoad: load }, layers);
}

// src/ui-management.ts
function cardMemoryTarget(card) {
  if (card.memoryBindingMode === "SHARED") {
    if (card.sharedMemoryId === null) throw new Error("请先为角色绑定共享记忆库");
    return { ownerKey: "shared:" + card.sharedMemoryId, name: card.name };
  }
  return { ownerKey: "character:" + card.id, name: card.name };
}
function chatBindingLabel(selection2, snapshot) {
  if (selection2 === null) return "未绑定角色";
  if (selection2 === "model:chat") return "仅使用模型";
  for (const card of snapshot.cards) if (selection2 === "card:" + card.id) return card.name;
  for (const group of snapshot.groups) if (selection2 === "group:" + group.id) return "群组 · " + group.name;
  return "原角色已失效";
}
function renderManagement(ctx, view) {
  const { state, live, update } = uiState(ctx, "management:" + view, { snapshot: null, busy: false, error: "", notice: "", panel: null, memory: null, navigation: 0, defaultAvatar: null });
  const [managed, setManaged] = ctx.useState("managed-entity", null);
  const sequence = ctx.useRef("management-entity-sequence", 0);
  const recovery = initializationRecoveryUi(ctx, () => {
    void load();
  });
  const ui = nativeUi(ctx, !state.busy);
  async function act(operation) {
    if (live.current.busy) return;
    update({ busy: true, error: "", notice: "" });
    try {
      await operation();
    } catch (error) {
      update({ error: String(error) });
    } finally {
      update({ busy: false });
    }
  }
  async function refresh() {
    update({ snapshot: await uiRequest({ action: "snapshot" }) });
  }
  async function load() {
    await act(async () => {
      try {
        await refresh();
      } catch (error) {
        recovery.reportFailure(error);
        throw error;
      }
    });
    try {
      update({ defaultAvatar: await ToolPkg.readResource("character_default_avatar", "operit-avatar.png") });
    } catch (error) {
      update({ error: String(error) });
    }
  }
  function entity(input) {
    if (live.current.busy) return;
    setManaged({ requestId: "management-" + ++sequence.current, input });
  }
  function finishEntity(result) {
    setManaged(null);
    if (result !== null) void act(refresh);
  }
  function memory(target, initial = "browse") {
    update({ memory: { ...target, initial }, navigation: live.current.navigation + 1, error: "" });
  }
  async function bindings(cardId) {
    await act(async () => {
      update({ panel: { type: "binding", draft: { cardId, query: "", unbound: false, chats: await uiRequest({ action: "listBindableChats" }), selected: [], failures: {}, completed: 0, total: 0 } } });
    });
  }
  function bindingChange(patch) {
    const panel = live.current.panel;
    if (panel?.type !== "binding" || live.current.busy) return;
    update({ panel: { ...panel, draft: { ...panel.draft, ...patch } } });
  }
  async function submitBindings() {
    await act(async () => {
      const panel = live.current.panel;
      if (panel?.type !== "binding") return;
      const draft = panel.draft, selection2 = "card:" + draft.cardId;
      if (!live.current.snapshot?.cards.some((card) => card.id === draft.cardId)) throw new Error("请选择角色卡");
      const ids = draft.selected.filter((id2) => draft.chats.some((chat) => chat.id === id2 && chat.selection !== selection2));
      if (ids.length === 0) throw new Error("请选择要绑定的对话");
      const next = { ...draft, chats: draft.chats.map((chat) => ({ ...chat })), failures: {}, completed: 0, total: ids.length, selected: [...ids] };
      update({ panel: { type: "binding", draft: next } });
      for (const chatId of ids) {
        try {
          const result = await uiRequest({ action: "writeChatBinding", chatId, selection: selection2 });
          if (result.chatId !== chatId || result.selection !== selection2) throw new Error("绑定结果与请求不一致");
          next.chats.find((chat) => chat.id === chatId).selection = selection2;
          next.selected = next.selected.filter((id2) => id2 !== chatId);
        } catch (error) {
          next.failures[chatId] = String(error);
        }
        next.completed++;
        update({ panel: { type: "binding", draft: { ...next } } });
      }
      if (next.selected.length === 0) update({ panel: null, notice: "已绑定 " + ids.length + " 条对话" });
      else update({ error: "部分对话未完成绑定，已保留失败选择，可重试。" });
    });
  }
  function panelBody(panel) {
    switch (panel.type) {
      case "store":
        return [ui.field("store-name", "名称", panel.store.name, (name) => update({ panel: { ...panel, store: { ...panel.store, name } } })), ui.primary("store-save", "保存", () => act(async () => {
          const current = live.current.panel;
          if (current?.type !== "store") return;
          update({ snapshot: await uiRequest({ action: "saveStore", store: current.store, create: current.create }), panel: null });
        }))];
      case "delete-store":
        return [ui.text("删除共享记忆库「" + panel.name + "」？此操作会影响关联角色。"), ui.primary("store-delete-confirm", "确认删除", () => act(async () => update({ snapshot: await uiRequest({ action: "deleteStore", id: panel.id }), panel: null })))];
      case "import":
        return [
          ...panel.entity === "card" ? [ui.choose("import-format", "角色格式", panel.format, [{ id: "operit", label: "Operit JSON" }, { id: "tavern", label: "Tavern JSON" }], (format2) => update({ panel: { ...panel, format: format2 } }))] : [],
          ui.field("import-content", panel.entity === "card" && panel.format === "tavern" ? "粘贴 Tavern JSON" : "粘贴 Operit JSON", panel.content, (content) => update({ panel: { ...panel, content } }), 18),
          ui.text("角色卡支持 Operit 和 Tavern JSON；群组使用 Operit JSON。"),
          ui.primary("import-submit", "导入", () => act(async () => {
            const current = live.current.panel;
            if (current?.type !== "import") return;
            update({ snapshot: await uiRequest(current.entity === "card" ? { action: "importCharacter", format: current.format, content: current.content } : { action: "importGroup", content: current.content }), panel: null, notice: "导入完成" });
          }))
        ];
      case "binding": {
        const draft = panel.draft, snapshot2 = state.snapshot;
        const query = draft.query.trim().toLocaleLowerCase();
        const visible = draft.chats.filter((chat) => (!draft.unbound || chat.selection === null) && (query === "" || [chat.title, chatBindingLabel(chat.selection, snapshot2)].some((text3) => text3.toLocaleLowerCase().includes(query))));
        const selectable = visible.filter((chat) => chat.selection !== "card:" + draft.cardId);
        const all = selectable.length > 0 && selectable.every((chat) => draft.selected.includes(chat.id));
        return [
          ui.text("为已有对话指定角色", true),
          ui.text("绑定后，对话将使用所选角色的设定继续交流。已有角色绑定会被替换。"),
          ui.choose("binding-card", "绑定到角色卡", draft.cardId, snapshot2.cards.map((card) => ({ id: card.id, label: card.name })), (cardId) => bindingChange({ cardId, selected: draft.selected.filter((id2) => draft.chats.some((chat) => chat.id === id2 && chat.selection !== "card:" + cardId)), failures: {} })),
          ui.field("binding-query", "搜索对话或当前角色", draft.query, (query2) => bindingChange({ query: query2 })),
          ctx.UI.Row({ fillMaxWidth: true, verticalAlignment: "center", spacing: 8 }, [
            ctx.UI.Box({ weight: 1 }, ui.choices("binding-filter", "", draft.unbound ? "unbound" : "all", [{ id: "all", label: "全部对话" }, { id: "unbound", label: "未绑定" }], (value) => bindingChange({ unbound: value === "unbound" }))),
            ui.button("binding-select-visible", all ? "取消全选结果" : "全选结果", () => {
              const selected = new Set(draft.selected);
              for (const chat of selectable) {
                if (all) selected.delete(chat.id);
                else selected.add(chat.id);
              }
              bindingChange({ selected: [...selected] });
            }, draft.cardId !== "")
          ]),
          ui.text(state.busy ? `正在绑定 ${draft.completed}/${draft.total}` : `已选择 ${draft.selected.length} 条 · 筛选出 ${visible.length} 条`, false, "binding-status"),
          ...visible.map((chat) => ctx.UI.Row({ key: "binding-chat:" + chat.id, fillMaxWidth: true, verticalAlignment: "center", spacing: 8 }, [ctx.UI.Checkbox({ key: "binding-check:" + chat.id, checked: draft.selected.includes(chat.id), enabled: !state.busy && draft.cardId !== "" && chat.selection !== "card:" + draft.cardId, onCheckedChange: (checked) => bindingChange({ selected: checked ? [.../* @__PURE__ */ new Set([...draft.selected, chat.id])] : draft.selected.filter((id2) => id2 !== chat.id) }) }), ctx.UI.Column({ weight: 1, spacing: 4 }, [ui.text(chat.title + (chat.isCurrent ? " · 当前对话" : ""), true), ui.text(chatBindingLabel(chat.selection, snapshot2) + ` · ${chat.messageCount} 条消息`), ...draft.failures[chat.id] ? [ui.text(draft.failures[chat.id])] : []])])),
          ...visible.length === 0 ? [ui.text("没有符合条件的对话")] : [],
          ui.primary("binding-submit", state.busy ? "绑定中…" : draft.selected.length > 0 ? `绑定 ${draft.selected.length} 条对话` : "绑定对话", submitBindings, draft.cardId !== "" && draft.selected.length > 0)
        ];
      }
    }
  }
  if (recovery.panel !== null) return recovery.panel;
  const snapshot = state.snapshot;
  const children = [];
  if (state.error !== "") children.push(ctx.UI.Text({ key: "management-error", text: state.error, color: ctx.MaterialTheme.colorScheme.error }));
  if (state.notice !== "") children.push(ui.text(state.notice));
  if (snapshot !== null) {
    if (view === "characters") {
      const cards = snapshot.cards.map((card) => {
        const active = "CharacterCard" in snapshot.active && snapshot.active.CharacterCard.id === card.id;
        return ui.tile("card:" + card.id, card.name, card.description, [card.chatModelBindingMode === "FIXED_MODEL" ? "固定模型配置" : "跟随全局模型", card.memoryBindingMode === "SHARED" ? "共享记忆" : "角色记忆", ...snapshot.tags.filter((tag) => card.attachedTagIds.includes(tag.id)).map((tag) => tag.name)], [
          ui.iconButton("card-bind:" + card.id, "绑定对话", "ChatBubbleOutline", () => bindings(card.id)),
          ui.iconButton("card-memory:" + card.id, "打开记忆图谱", "AccountTreeOutlined", () => act(async () => memory(cardMemoryTarget(card)))),
          ui.iconButton("card-user:" + card.id, "编辑用户资料", "AccountBoxOutlined", () => act(async () => memory(cardMemoryTarget(card), "profile"))),
          active ? ui.activePill() : ui.sectionAction("card-activate:" + card.id, "使用", "", () => act(async () => update({ snapshot: await uiRequest({ action: "activate", type: "card", id: card.id }) })))
        ], active, () => entity({ mode: "edit", entity: "card", id: card.id }), card.avatarUri ?? state.defaultAvatar);
      });
      children.push(ui.section("character-section", "角色卡", "BadgeOutlined", [ui.sectionAction("bind-chats", "绑定对话", "ChatBubbleOutline", () => bindings(""), snapshot.cards.length > 0), ui.menu("add-card", "添加", "Add", [{ key: "create-card", label: "新建角色卡", name: "Add", action: () => entity({ mode: "create", entity: "card", id: "" }) }, { key: "import-card", label: "导入角色卡", name: "Upload", action: () => update({ panel: { type: "import", entity: "card", format: "operit", content: "" } }) }])], cards.length === 0 ? [ui.text("还没有角色卡，点击创建添加角色。")] : cards));
      const groups = snapshot.groups.map((group) => {
        const active = "CharacterGroup" in snapshot.active && snapshot.active.CharacterGroup.id === group.id;
        return ui.tile("group:" + group.id, group.name, group.members.map((member2) => snapshot.cards.find((card) => card.id === member2.characterCardId)?.name ?? "已删除角色").join("、"), [`${group.members.length} 个角色`], [active ? ui.activePill() : ui.sectionAction("group-activate:" + group.id, "使用", "", () => act(async () => update({ snapshot: await uiRequest({ action: "activate", type: "group", id: group.id }) })))], active, () => entity({ mode: "edit", entity: "group", id: group.id }), null, "GroupsOutlined");
      });
      children.push(ui.section("group-section", "群组", "GroupsOutlined", [ui.sectionAction("import-group", "导入 JSON", "Upload", () => update({ panel: { type: "import", entity: "group", format: "operit", content: "" } })), ui.sectionAction("create-group", "创建", "Add", () => entity({ mode: "create", entity: "group", id: "" }))], groups.length === 0 ? [ui.text("还没有群组，创建后可编排多个角色参与对话。")] : groups));
    } else {
      let ownerActions2 = function(target, scope = "") {
        if (!snapshot.memoryOwnerKeys.includes(target.ownerKey)) return [ui.sectionAction("library-create:" + target.ownerKey + scope, target.ownerKey.startsWith("character:") ? "新建角色卡记忆库" : "新建共享记忆库", "Add", () => act(async () => update({ snapshot: await uiRequest({ action: "createMemoryLibrary", ownerKey: target.ownerKey }) })))];
        return [ui.iconButton("library-open:" + target.ownerKey + scope, "记忆图谱", "AccountTreeOutlined", () => memory(target)), ui.iconButton("library-user:" + target.ownerKey + scope, "用户资料 · USER.md", "AccountBoxOutlined", () => memory(target, "profile")), ui.iconButton("library-settings:" + target.ownerKey + scope, "记忆设置", "Tune", () => memory(target, "settings"))];
      };
      var ownerActions = ownerActions2;
      const global = [];
      for (const [key, label, description] of [["userDescriptionEnabled", "向模型提供用户资料", "聊天执行时提供用户资料；关闭不会删除 USER.md 或改写历史记录。"], ["autoUpdateEnabled", "自动更新记忆库", "允许自动候选入队和定时提取；关闭不会取消手动历史重建。"]]) global.push(ctx.UI.Row({ fillMaxWidth: true, spacing: 16, paddingVertical: 10, verticalAlignment: "center" }, [ctx.UI.Column({ weight: 1, spacing: 4 }, [ctx.UI.Text({ text: label, fontSize: 14 }), ctx.UI.Text({ text: description, fontSize: 12, color: ctx.MaterialTheme.colorScheme.onSurfaceVariant })]), ctx.UI.Switch({ key: "memory-preference:" + key, checked: snapshot.memoryPreferences[key], enabled: !state.busy, onCheckedChange: (value) => act(async () => {
        const memoryPreferences = await uiRequest({ action: "writeGlobalMemoryPreference", key, value });
        update({ snapshot: { ...live.current.snapshot, memoryPreferences } });
      }) })]));
      children.push(ui.section("global-memory-section", "全局记忆设置", "Tune", [], global));
      const stores = snapshot.stores.map((store) => ui.tile("store:" + store.id, store.name, "", [`${snapshot.cards.filter((card) => card.memoryBindingMode === "SHARED" && card.sharedMemoryId === store.id).length} 个角色绑定`], [...ownerActions2({ ownerKey: "shared:" + store.id, name: store.name }), ui.iconButton("store-edit:" + store.id, "编辑共享记忆库", "EditOutlined", () => update({ panel: { type: "store", create: false, store: { id: store.id, name: store.name } } })), ui.iconButton("store-delete:" + store.id, "删除共享记忆库", "DeleteOutline", () => update({ panel: { type: "delete-store", id: store.id, name: store.name } }))], false, void 0, null, "HubOutlined"));
      children.push(ui.section("shared-memory-section", "共享记忆库", "HubOutlined", [ui.sectionAction("create-store", "创建", "Add", () => update({ panel: { type: "store", create: true, store: { id: "", name: "" } } }))], stores.length === 0 ? [ui.text("当前还没有共享记忆库，创建后可被多个角色卡挂载共享。")] : stores));
      const cards = snapshot.cards.map((card) => {
        const missing = card.memoryBindingMode === "SHARED" && !snapshot.stores.some((store) => store.id === card.sharedMemoryId);
        const target = missing ? null : cardMemoryTarget(card);
        const active = "CharacterCard" in snapshot.active && snapshot.active.CharacterCard.id === card.id;
        return ui.tile("memory-card:" + card.id, card.name, "", [card.memoryBindingMode === "SHARED" ? "共享记忆" : "角色记忆", ...missing ? ["需要重新绑定记忆库"] : target !== null && !snapshot.memoryOwnerKeys.includes(target.ownerKey) ? ["当前角色无记忆库"] : [], ...active ? ["当前使用"] : []], target === null ? [ui.sectionAction("memory-rebind:" + card.id, "重新绑定记忆库", "EditOutlined", () => entity({ mode: "edit", entity: "card", id: card.id }))] : ownerActions2(target, card.memoryBindingMode === "SHARED" ? ":card:" + card.id : ""), active, void 0, card.avatarUri ?? state.defaultAvatar);
      });
      children.push(ui.section("character-memory-section", "角色记忆库", "BadgeOutlined", [], cards));
    }
  } else children.push(ui.text(state.busy ? "正在读取…" : "尚未读取成功，请重试"));
  const overlays = [];
  if (state.panel !== null) {
    const panel = state.panel, fields2 = panelBody(panel), submit = fields2.pop();
    const close = () => {
      if (!live.current.busy) update({ panel: null, error: "" });
    };
    const title = panel.type === "binding" ? "批量绑定对话" : panel.type === "import" ? panel.entity === "card" ? "导入角色卡" : "导入群组" : panel.type === "delete-store" ? "删除共享记忆库" : panel.create ? "创建共享记忆库" : "编辑共享记忆库";
    const messages = state.error === "" ? [] : [ctx.UI.Text({ text: state.error, color: ctx.MaterialTheme.colorScheme.error })];
    const body = panel.type === "binding" ? ctx.UI.Column({ fillMaxSize: true }, [
      ctx.UI.Column({ fillMaxWidth: true, paddingHorizontal: 18, spacing: 6 }, fields2.filter((node) => !String(node.props?.key ?? "").startsWith("binding-chat:") && node.props?.key !== "binding-status")),
      ui.list("binding-chat-list", [...messages, ...fields2.filter((node) => String(node.props?.key ?? "").startsWith("binding-chat:"))])
    ]) : ui.list("management-form", [...messages, ...fields2]);
    const cancel = ui.button("management-back", "取消", close);
    const status = fields2.find((node) => node.props?.key === "binding-status");
    const footer = panel.type !== "binding" ? [cancel, submit] : [ui.size.width <= 600 ? ctx.UI.Column({ fillMaxWidth: true, spacing: 8 }, [status, ctx.UI.Row({ fillMaxWidth: true, horizontalArrangement: "end", verticalAlignment: "center", spacing: 8 }, [cancel, submit])]) : ctx.UI.Row({ fillMaxWidth: true, verticalAlignment: "center", spacing: 8 }, [ctx.UI.Box({ weight: 1 }, status), cancel, submit])];
    overlays.push(ui.dialog("management-dialog", title, body, footer, close, panel.type === "binding" || panel.type === "import" ? "large" : "compact"));
  }
  if (state.memory !== null) overlays.push(renderMemoryWorkspace(ctx, state.memory, () => update({ memory: null }), "management-memory:" + state.navigation));
  if (managed !== null) overlays.push(renderManagedEntityScreen(ctx, managed, finishEntity));
  return ctx.UI.Box({ key: "management-root", fillMaxSize: true, modifier: ui.measure, onLoad: load }, [ctx.UI.Column({ fillMaxSize: true }, [ctx.UI.LazyColumn({ key: "management-list", weight: 1, fillMaxWidth: true, paddingHorizontal: ui.size.width <= 600 ? 12 : 16, paddingTop: 12, paddingBottom: 20 }, children)]), ...overlays]);
}

// src/ui-memory-attachment.ts
function memoryAttachmentOwners(snapshot) {
  return [...snapshot.cards.map((card) => ({ key: "character:" + card.id, name: card.name })), ...snapshot.stores.map((store) => ({ key: "shared:" + store.id, name: store.name }))];
}
function memoryAttachmentContent(owner2, folder, items) {
  if (items.length === 0) return "";
  return `# ${owner2.name} / ${folder === "" ? "/" : folder}

` + items.map((item) => `## ${item.title}

${item.content}`).join("\n\n");
}
function renderMemoryAttachment(ctx) {
  const [presentation] = ctx.useState("presentation", null);
  const session = ctx.useRef("attachment-session", null);
  if (session.current === null) session.current = createUiScreenSession(presentation);
  const current = session.current.currentScreen(), screenSession = session.current;
  if (current.input.mode !== "memory-attachment") throw new Error("The attachment route requires an explicit attachment presentation");
  const input = current.input;
  const ownerPresentation = ctx.useRef("attachment-presentation", JSON.stringify(presentation));
  const { state, live, update } = uiState(ctx, "memory-attachment", { snapshot: null, step: "owners", owner: null, folders: null, folder: null, items: null, busy: false, error: "", finished: false });
  const ui = nativeUi(ctx, !state.busy && !state.finished);
  function assertOwner() {
    const [actual] = ctx.useState("presentation", null);
    if (JSON.stringify(actual) !== ownerPresentation.current) throw new Error("Attachment presentation owner changed");
  }
  assertOwner();
  async function act(operation) {
    assertOwner();
    if (live.current.busy || live.current.finished) return;
    update({ busy: true, error: "" });
    try {
      await operation();
      assertOwner();
    } catch (error) {
      assertOwner();
      update({ error: String(error) });
    } finally {
      update({ busy: false });
    }
  }
  async function content(folder) {
    const value = live.current;
    if (value.owner === null || value.folders === null || !value.folders.includes(folder)) throw new Error("请选择存在的记忆文件夹");
    if (input.folderPath !== null && input.folderPath !== folder) throw new Error("不能更改调用方指定的文件夹");
    update({ step: "content", folder, items: null });
    const items = await uiRequest({ action: "listMemories", ownerKey: value.owner.key });
    assertOwner();
    update({ items: items.filter((item) => (item.folderPath ?? "") === folder) });
  }
  async function folders(owner2) {
    if (input.ownerKey !== null && input.ownerKey !== owner2.key) throw new Error("不能更改调用方指定的记忆库");
    update({ step: "folders", owner: owner2, folders: null, folder: null, items: null });
    const paths = await uiRequest({ action: "listMemoryFolders", ownerKey: owner2.key });
    assertOwner();
    update({ folders: [.../* @__PURE__ */ new Set(["", ...paths])].sort() });
    if (input.folderPath !== null) await content(input.folderPath);
  }
  async function load() {
    await act(async () => {
      const snapshot = await uiRequest({ action: "snapshot" });
      assertOwner();
      update({ snapshot });
      if (input.ownerKey !== null) {
        const owner2 = memoryAttachmentOwners(snapshot).find((owner3) => owner3.key === input.ownerKey);
        if (owner2 === void 0) throw new Error("调用方指定的记忆库不存在");
        await folders(owner2);
      }
    });
  }
  async function retry() {
    if (live.current.snapshot === null || live.current.owner === null) {
      await load();
      return;
    }
    await act(async () => {
      const value = live.current;
      if (value.step === "content" && value.folder !== null && value.folders !== null) await content(value.folder);
      else await folders(value.owner);
    });
  }
  async function confirm() {
    assertOwner();
    const value = live.current;
    if (value.busy || value.finished || value.error !== "" || value.owner === null || value.folder === null || value.items === null) return null;
    const content2 = memoryAttachmentContent(value.owner, value.folder, value.items);
    if (content2 === "") return null;
    const result = await screenSession.completeScreen({ mode: "memory-attachment", ownerKey: value.owner.key, folderPath: value.folder, content: content2 });
    update({ finished: true });
    return JSON.parse(JSON.stringify(result));
  }
  function cancel() {
    assertOwner();
    if (live.current.busy || live.current.finished) return null;
    update({ finished: true });
    return JSON.parse(JSON.stringify(screenSession.cancelScreen()));
  }
  const children = [];
  if (state.error !== "") children.push(ctx.UI.Text({ key: "attachment-error", text: state.error, color: ctx.MaterialTheme.colorScheme.error }), ui.button("attachment-retry", "重试", retry));
  else if (state.busy || state.snapshot === null) children.push(ui.text("正在读取记忆库…"));
  else if (state.step === "owners") {
    const owners = memoryAttachmentOwners(state.snapshot);
    children.push(...owners.map((owner2) => ctx.UI.Row({ key: "attachment-owner:" + owner2.key, fillMaxWidth: true, paddingVertical: 12, spacing: 12, verticalAlignment: "center", onClick: () => act(() => folders(owner2)) }, [ctx.UI.Icon({ name: owner2.key.startsWith("shared:") ? "GroupsOutlined" : "PersonOutline", size: 22 }), ui.text(owner2.name)])));
    if (owners.length === 0) children.push(ui.text("没有可选的记忆库"));
  } else if (state.step === "folders") children.push(...(state.folders ?? []).map((folder) => ctx.UI.Row({ key: "attachment-folder:" + folder, fillMaxWidth: true, paddingVertical: 12, spacing: 12, verticalAlignment: "center", onClick: () => act(() => content(folder)) }, [ctx.UI.Icon({ name: "FolderOutlined", size: 22 }), ui.text(folder === "" ? "/" : folder)])));
  else if (state.owner !== null && state.folder !== null && state.items !== null) children.push(ui.text(memoryAttachmentContent(state.owner, state.folder, state.items) || "当前文件夹没有记忆"));
  const canBack = state.step === "folders" ? input.ownerKey === null : state.step === "content" && input.folderPath === null;
  return ctx.UI.Column({ key: "memory-attachment-root", fillMaxSize: true, onLoad: load }, [ctx.UI.Row({ fillMaxWidth: true, height: 58, paddingHorizontal: 18, spacing: 12, verticalAlignment: "center" }, [...canBack ? [ui.iconButton("attachment-back", "返回", "ArrowBack", () => {
    if (state.step === "content") update({ step: "folders", folder: null, items: null, error: "" });
    else update({ step: "owners", owner: null, folders: null, error: "" });
  })] : [], ctx.UI.Text({ text: state.owner === null ? "记忆附件" : state.owner.name, fontSize: 20, fontWeight: "500" })]), ui.list("attachment-body", children), ctx.UI.Box({ fillMaxWidth: true, padding: 12 }, ui.actions([ui.button("attachment-cancel", "取消", cancel), ...state.step === "content" && state.items !== null && state.items.length > 0 && state.error === "" ? [ui.button("attachment-confirm", "添加记忆附件", confirm)] : []]))]);
}

// src/ui-editor.ts
function renderScreen(ctx, _definition, requiredMode = null, managementView = "characters") {
  return requiredMode === "memory-attachment" ? renderMemoryAttachment(ctx) : renderManagement(ctx, managementView);
}

// src/ui/memory/index.ui.ts
function Screen(ctx) {
  return renderScreen(ctx, definition, null, "memory");
}

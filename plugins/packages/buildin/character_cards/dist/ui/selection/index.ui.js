// @operit-compose-artifact 1
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
var __async = (__this, __arguments, generator) => {
  return new Promise((resolve, reject) => {
    var fulfilled = (value) => {
      try {
        step(generator.next(value));
      } catch (e) {
        reject(e);
      }
    };
    var rejected = (value) => {
      try {
        step(generator.throw(value));
      } catch (e) {
        reject(e);
      }
    };
    var step = (x) => x.done ? resolve(x.value) : Promise.resolve(x.value).then(fulfilled, rejected);
    step((generator = generator.apply(__this, __arguments)).next());
  });
};

// src/ui/selection/index.ui.ts
var index_ui_exports = {};
__export(index_ui_exports, {
  default: () => Screen
});
module.exports = __toCommonJS(index_ui_exports);

// src/validation.ts
function assertObject(value, label) {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new Error(`${label} \u5FC5\u987B\u662F\u5BF9\u8C61`);
}
function assertString(value, label) {
  if (typeof value !== "string") throw new Error(`${label} \u5FC5\u987B\u662F\u5B57\u7B26\u4E32`);
}
function requireId(value, label) {
  assertString(value, label);
  if (value.trim() === "" || value !== value.trim()) throw new Error(`${label} \u65E0\u6548`);
  return value;
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
function requireChatSelection(selection2, cards, groups) {
  const parsed = parseChatSelection(selection2);
  if (parsed.kind === "model") return parsed;
  const records2 = parsed.kind === "card" ? cards : groups;
  let matches = 0;
  for (const record2 of records2) if (record2.id === parsed.id) matches += 1;
  if (matches !== 1) throw new Error("Chat selection does not identify exactly one stored record: " + selection2);
  if (parsed.kind === "group") {
    for (const group of groups) if (group.id === parsed.id && group.members.length === 0) throw new Error("A bound chat group must have actual participants: " + group.id);
  }
  return parsed;
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
function markerVersion(value, path) {
  if (value.version !== 1) throw new Error("Unsupported " + path + " version");
}
function decodeChatMarker(value) {
  const extension = markerObject(value, "chat extension");
  markerVersion(extension, "chat marker");
  const selection2 = requireId(extension.selection, "chat marker selection");
  parseChatSelection(selection2);
  if (Object.prototype.hasOwnProperty.call(extension, "groupId")) throw new Error("Conversation membership is owned by plugin files, not chat extension.groupId");
  return { version: 1, selection: selection2 };
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
function parseMemoryIdentifier(value, path, positive = true) {
  if (typeof value !== "string" || !/^(0|[1-9][0-9]*)$/.test(value)) throw new Error(`${path} must be a canonical decimal string`);
  const id2 = BigInt(value);
  if (id2 > 9223372036854775807n || positive && id2 === 0n) throw new Error(`${path} is outside its declared identity range`);
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
  ttsConfigId: optional(nullable(text)),
  themeConfigId: optional(nullable(text)),
  memoryBindingMode: optional(enumeration("CHARACTER", "SHARED")),
  sharedMemoryId: optional(nullable(text)),
  sharedMemoryMounts: optional(array(mount)),
  toolAccessConfig: optional(toolAccess),
  isDefault: optional(boolean),
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
    messageExtension: required(nullable(markerObject)),
    participantId: required(nullable(nonblank)),
    promptFunctionType: required(enumeration("CHAT", "VOICE")),
    defaultModelBinding: required(shape({ providerId: required(text), modelId: required(text) })),
    defaultTtsConfigId: required(text)
  })(value, "chat.configuration.resolve");
  const request = value;
  if (request.purpose === "display") {
    if (request.participantId !== null || request.messageExtension !== null) throw new Error("Display configuration cannot request an execution participant or message snapshot");
  } else {
    nonblank(request.defaultModelBinding.providerId, "execution default provider");
    nonblank(request.defaultModelBinding.modelId, "execution default model");
    nonblank(request.defaultTtsConfigId, "execution default TTS configuration");
  }
  return request;
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

// src/storage/schema.json
var collections = {
  cards: {
    kind: "object",
    fields: {
      id: {
        kind: "text"
      },
      name: {
        kind: "text"
      },
      description: {
        kind: "text"
      },
      characterSetting: {
        kind: "text"
      },
      openingStatement: {
        kind: "text"
      },
      otherContentChat: {
        kind: "text"
      },
      otherContentVoice: {
        kind: "text"
      },
      avatarUri: {
        kind: "text",
        nullable: true
      },
      attachedTagIds: {
        kind: "array",
        item: {
          kind: "text"
        }
      },
      advancedCustomPrompt: {
        kind: "text"
      },
      marks: {
        kind: "text"
      },
      chatModelBindingMode: {
        kind: "text"
      },
      chatModelId: {
        kind: "text",
        nullable: true
      },
      ttsConfigId: {
        kind: "text",
        nullable: true
      },
      themeConfigId: {
        kind: "text",
        nullable: true
      },
      memoryBindingMode: {
        kind: "text"
      },
      sharedMemoryId: {
        kind: "text",
        nullable: true
      },
      sharedMemoryMounts: {
        kind: "array",
        item: {
          kind: "object",
          fields: {
            sharedMemoryId: {
              kind: "text"
            },
            readable: {
              kind: "boolean"
            },
            writable: {
              kind: "boolean"
            }
          }
        }
      },
      toolAccessConfig: {
        kind: "object",
        fields: {
          enabled: {
            kind: "boolean"
          },
          allowedBuiltinTools: {
            kind: "array",
            item: {
              kind: "text"
            }
          },
          allowedPackages: {
            kind: "array",
            item: {
              kind: "text"
            }
          },
          allowedSkills: {
            kind: "array",
            item: {
              kind: "text"
            }
          },
          allowedMcpServers: {
            kind: "array",
            item: {
              kind: "text"
            }
          }
        }
      },
      isDefault: {
        kind: "boolean"
      },
      createdAt: {
        kind: "integer"
      },
      updatedAt: {
        kind: "integer"
      }
    }
  },
  groups: {
    kind: "object",
    fields: {
      id: {
        kind: "text"
      },
      name: {
        kind: "text"
      },
      description: {
        kind: "text"
      },
      members: {
        kind: "array",
        item: {
          kind: "object",
          fields: {
            characterCardId: {
              kind: "text"
            },
            orderIndex: {
              kind: "integer"
            }
          }
        }
      },
      themeConfigId: {
        kind: "text",
        nullable: true
      },
      createdAt: {
        kind: "integer"
      },
      updatedAt: {
        kind: "integer"
      }
    }
  },
  tags: {
    kind: "object",
    fields: {
      id: {
        kind: "text"
      },
      name: {
        kind: "text"
      },
      description: {
        kind: "text"
      },
      promptContent: {
        kind: "text"
      },
      tagType: {
        kind: "text"
      },
      createdAt: {
        kind: "integer"
      },
      updatedAt: {
        kind: "integer"
      }
    }
  },
  stores: {
    kind: "object",
    fields: {
      id: {
        kind: "text"
      },
      name: {
        kind: "text"
      },
      createdAt: {
        kind: "integer"
      },
      updatedAt: {
        kind: "integer"
      }
    }
  },
  conversationGroups: {
    kind: "object",
    fields: {
      id: {
        kind: "text"
      },
      ownerSelection: {
        kind: "text",
        nullable: true
      },
      name: {
        kind: "text"
      },
      chatIds: {
        kind: "array",
        item: {
          kind: "text"
        }
      },
      displayOrder: {
        kind: "integer"
      },
      pinned: {
        kind: "boolean"
      },
      createdAt: {
        kind: "integer"
      },
      updatedAt: {
        kind: "integer"
      }
    }
  },
  owners: {
    kind: "object",
    fields: {
      ownerKey: {
        kind: "text"
      },
      settings: {
        kind: "object",
        fields: {
          autoSaveIntervalMinutes: {
            kind: "integer"
          },
          nextAutoSaveRunAtMs: {
            kind: "integer"
          },
          memoryExtractionCustomRules: {
            kind: "text"
          },
          profileAutoUpdateEnabled: {
            kind: "boolean"
          },
          profileAutoUpdateLocked: {
            kind: "boolean"
          },
          cloudEmbeddingEnabled: {
            kind: "boolean"
          },
          cloudEmbeddingEndpoint: {
            kind: "text"
          },
          cloudEmbeddingApiKey: {
            kind: "text"
          },
          cloudEmbeddingModel: {
            kind: "text"
          }
        }
      },
      searchConfig: {
        kind: "object",
        fields: {
          scoreMode: {
            kind: "text"
          },
          keywordWeight: {
            kind: "number"
          },
          tagWeight: {
            kind: "number"
          },
          vectorWeight: {
            kind: "number"
          },
          edgeWeight: {
            kind: "number"
          }
        }
      },
      userDocumentPath: {
        kind: "text"
      },
      rebuildProgress: {
        kind: "object",
        fields: {
          status: {
            kind: "text"
          },
          totalChats: {
            kind: "integer"
          },
          completedChats: {
            kind: "integer"
          },
          totalWindows: {
            kind: "integer"
          },
          completedWindows: {
            kind: "integer"
          },
          totalSourceMessages: {
            kind: "integer"
          },
          processedSourceMessages: {
            kind: "integer"
          },
          failedWindows: {
            kind: "integer"
          },
          currentChatTitle: {
            kind: "text"
          },
          lastError: {
            kind: "text"
          }
        }
      },
      rebuildTask: {
        kind: "object",
        fields: {
          id: {
            kind: "text"
          },
          windows: {
            kind: "array",
            item: {
              kind: "object",
              fields: {
                chatId: {
                  kind: "text"
                },
                chatTitle: {
                  kind: "text"
                },
                messages: {
                  kind: "array",
                  item: {
                    kind: "object",
                    fields: {
                      sender: {
                        kind: "text"
                      },
                      content: {
                        kind: "text"
                      },
                      timestamp: {
                        kind: "integer"
                      },
                      variantIndex: {
                        kind: "integer"
                      },
                      variantCount: {
                        kind: "integer"
                      },
                      provider: {
                        kind: "text"
                      },
                      modelName: {
                        kind: "text"
                      }
                    }
                  }
                },
                sourceMessageCount: {
                  kind: "integer"
                }
              }
            }
          },
          nextWindow: {
            kind: "integer"
          }
        },
        nullable: true
      }
    }
  },
  memories: {
    kind: "object",
    fields: {
      ownerKey: {
        kind: "text"
      },
      record: {
        kind: "object",
        fields: {
          id: {
            kind: "text"
          },
          uuid: {
            kind: "text"
          },
          title: {
            kind: "text"
          },
          content: {
            kind: "text"
          },
          contentType: {
            kind: "text"
          },
          source: {
            kind: "text"
          },
          credibility: {
            kind: "number"
          },
          importance: {
            kind: "number"
          },
          documentPath: {
            kind: "text",
            nullable: true
          },
          isDocumentNode: {
            kind: "boolean"
          },
          chunkIndexFilePath: {
            kind: "text",
            nullable: true
          },
          folderPath: {
            kind: "text",
            nullable: true
          },
          createdAt: {
            kind: "integer"
          },
          updatedAt: {
            kind: "integer"
          },
          lastAccessedAt: {
            kind: "integer"
          },
          tags: {
            kind: "array",
            item: {
              kind: "object",
              fields: {
                id: {
                  kind: "text"
                },
                name: {
                  kind: "text"
                }
              }
            }
          },
          properties: {
            kind: "array",
            item: {
              kind: "object",
              fields: {
                id: {
                  kind: "text"
                },
                key: {
                  kind: "text"
                },
                value: {
                  kind: "text"
                }
              }
            }
          }
        }
      }
    }
  },
  links: {
    kind: "object",
    fields: {
      ownerKey: {
        kind: "text"
      },
      record: {
        kind: "object",
        fields: {
          id: {
            kind: "text"
          },
          sourceMemoryId: {
            kind: "text"
          },
          targetMemoryId: {
            kind: "text"
          },
          type_: {
            kind: "text"
          },
          weight: {
            kind: "number"
          },
          description: {
            kind: "text"
          }
        }
      }
    }
  },
  chunks: {
    kind: "object",
    fields: {
      ownerKey: {
        kind: "text"
      },
      record: {
        kind: "object",
        fields: {
          id: {
            kind: "text"
          },
          memoryUuid: {
            kind: "text"
          },
          chunkIndex: {
            kind: "integer"
          },
          content: {
            kind: "text"
          }
        }
      }
    }
  },
  candidates: {
    kind: "object",
    fields: {
      ownerKey: {
        kind: "text"
      },
      record: {
        kind: "object",
        fields: {
          id: {
            kind: "text"
          },
          chatId: {
            kind: "text"
          },
          triggerMessageTimestamp: {
            kind: "integer"
          },
          triggerVariantIndex: {
            kind: "integer"
          },
          createdAt: {
            kind: "integer"
          },
          updatedAt: {
            kind: "integer"
          },
          status: {
            kind: "text"
          },
          attemptCount: {
            kind: "integer"
          },
          lastError: {
            kind: "text"
          },
          sourceType: {
            kind: "text"
          }
        }
      }
    }
  },
  embeddings: {
    kind: "object",
    fields: {
      ownerKey: {
        kind: "text"
      },
      record: {
        kind: "object",
        fields: {
          endpoint: {
            kind: "text"
          },
          model: {
            kind: "text"
          },
          text: {
            kind: "text"
          },
          vector: {
            kind: "vector"
          },
          updatedAt: {
            kind: "integer"
          }
        }
      }
    }
  },
  meta: {
    kind: "object",
    fields: {
      version: {
        kind: "integer"
      },
      nextId: {
        kind: "text"
      },
      active: {
        kind: "object",
        nullable: true,
        fields: {
          CharacterCard: {
            kind: "object",
            optional: true,
            fields: {
              id: {
                kind: "text"
              }
            }
          },
          CharacterGroup: {
            kind: "object",
            optional: true,
            fields: {
              id: {
                kind: "text"
              }
            }
          }
        }
      },
      memoryPreferences: {
        kind: "object",
        fields: {
          userDescriptionEnabled: {
            kind: "boolean"
          },
          autoUpdateEnabled: {
            kind: "boolean"
          }
        },
        optional: true
      },
      migrationStatus: {
        kind: "text",
        optional: true
      },
      migrationSource: {
        kind: "text",
        optional: true
      }
    }
  },
  chat_bindings: {
    kind: "object",
    fields: {
      version: {
        kind: "integer"
      },
      selection: {
        kind: "text"
      }
    }
  },
  chat_drafts: {
    kind: "object",
    fields: {
      version: {
        kind: "integer"
      },
      selection: {
        kind: "text"
      }
    }
  },
  message_snapshots: {
    kind: "object",
    fields: {
      version: {
        kind: "integer"
      },
      selection: {
        kind: "text"
      },
      promptFunctionType: {
        kind: "text"
      },
      primaryOwnerKey: {
        kind: "text"
      },
      profile: {
        kind: "object",
        fields: {
          id: {
            kind: "text"
          },
          name: {
            kind: "text"
          },
          avatarUri: {
            kind: "text",
            nullable: true
          },
          introPrompt: {
            kind: "text"
          },
          userPreferencesText: {
            kind: "text"
          },
          openingStatement: {
            kind: "text"
          },
          modelBinding: {
            kind: "object",
            fields: {
              providerId: {
                kind: "text"
              },
              modelId: {
                kind: "text"
              }
            }
          },
          ttsConfigId: {
            kind: "text"
          },
          toolAccess: {
            kind: "object",
            fields: {
              enabled: {
                kind: "boolean"
              },
              allowedBuiltinTools: {
                kind: "array",
                item: {
                  kind: "text"
                }
              },
              allowedPackages: {
                kind: "array",
                item: {
                  kind: "text"
                }
              },
              allowedSkills: {
                kind: "array",
                item: {
                  kind: "text"
                }
              },
              allowedMcpServers: {
                kind: "array",
                item: {
                  kind: "text"
                }
              }
            }
          },
          resources: {
            kind: "array",
            item: {
              kind: "object",
              fields: {
                key: {
                  kind: "text"
                },
                readable: {
                  kind: "boolean"
                },
                writable: {
                  kind: "boolean"
                }
              }
            }
          }
        }
      },
      participants: {
        kind: "array",
        item: {
          kind: "object",
          fields: {
            id: {
              kind: "text"
            },
            name: {
              kind: "text"
            },
            avatarUri: {
              kind: "text",
              nullable: true
            },
            introPrompt: {
              kind: "text"
            },
            userPreferencesText: {
              kind: "text"
            },
            openingStatement: {
              kind: "text"
            },
            modelBinding: {
              kind: "object",
              fields: {
                providerId: {
                  kind: "text"
                },
                modelId: {
                  kind: "text"
                }
              }
            },
            ttsConfigId: {
              kind: "text"
            },
            toolAccess: {
              kind: "object",
              fields: {
                enabled: {
                  kind: "boolean"
                },
                allowedBuiltinTools: {
                  kind: "array",
                  item: {
                    kind: "text"
                  }
                },
                allowedPackages: {
                  kind: "array",
                  item: {
                    kind: "text"
                  }
                },
                allowedSkills: {
                  kind: "array",
                  item: {
                    kind: "text"
                  }
                },
                allowedMcpServers: {
                  kind: "array",
                  item: {
                    kind: "text"
                  }
                }
              }
            },
            resources: {
              kind: "array",
              item: {
                kind: "object",
                fields: {
                  key: {
                    kind: "text"
                  },
                  readable: {
                    kind: "boolean"
                  },
                  writable: {
                    kind: "boolean"
                  }
                }
              }
            }
          }
        }
      }
    }
  }
};

// src/storage/relational.ts
var schemas = collections;
function quote(name) {
  if (!/^[A-Za-z_][A-Za-z_0-9]*$/.test(name))
    throw new Error("Invalid relational identifier: " + name);
  return '"' + name + '"';
}
function field(path) {
  return path === "" ? "_value" : path;
}
function layout(name, node, child = false) {
  const columns = [
    { name: "_key", affinity: "text", nullable: false },
    { name: "_revision", affinity: "text", nullable: false }
  ];
  if (child)
    columns.push(
      { name: "_root", affinity: "text", nullable: false },
      { name: "_parent", affinity: "text", nullable: false },
      { name: "_position", affinity: "integer", nullable: false },
      { name: "_map_key", affinity: "text", nullable: true }
    );
  const children = [];
  function visit(schema, path, absent) {
    if (schema.optional || schema.nullable)
      columns.push({
        name: field(path) + "__state",
        affinity: "integer",
        nullable: absent
      });
    const nullable2 = absent || schema.optional === true || schema.nullable === true;
    if (schema.kind === "object") {
      for (const [key, value] of Object.entries(schema.fields).sort(
        ([left], [right]) => left < right ? -1 : left > right ? 1 : 0
      ))
        visit(value, path === "" ? key : path + "__" + key, nullable2);
    } else if (schema.kind === "array" || schema.kind === "map") {
      columns.push({
        name: field(path) + "__count",
        affinity: "integer",
        nullable: nullable2
      });
      children.push(layout(name + "_" + field(path), schema.item, true));
    } else
      columns.push({
        name: field(path),
        affinity: schema.kind === "text" ? "text" : schema.kind === "vector" ? "blob" : schema.kind === "boolean" || schema.kind === "integer" ? "integer" : "real",
        nullable: nullable2
      });
  }
  visit(node, "", false);
  return { name, node, table: { name, primaryKey: "_key", columns }, children };
}
var layouts = Object.fromEntries(
  Object.entries(schemas).map(([name, node]) => [name, layout(name, node)])
);
function tables(root) {
  return [root, ...root.children.flatMap(tables)];
}
function childKey(parent, index) {
  return parent.length + ":" + parent + ":" + index;
}
function vector(value) {
  if (!Array.isArray(value))
    throw new Error("Embedding vector must be an array");
  const bytes = new Uint8Array(value.length * 8), view = new DataView(bytes.buffer);
  value.forEach((item, index) => {
    if (typeof item !== "number" || !Number.isFinite(item))
      throw new Error("Invalid embedding component");
    view.setFloat64(index * 8, item, true);
  });
  return bytes;
}
function encode(collection, key, value, revision) {
  const root = layouts[collection];
  if (root === void 0)
    throw new Error("Unknown relational collection: " + collection);
  const rows = new Map(tables(root).map((table) => [table.name, []]));
  function record2(current, identity2, input, extra) {
    const row = { _key: identity2, _revision: revision, ...extra };
    function visit(node, path, item, absent = false) {
      const column = field(path);
      if (node.optional || node.nullable)
        row[column + "__state"] = item === void 0 ? 0 : item === null ? 1 : 2;
      if (item === void 0 && !node.optional && !absent)
        throw new Error(collection + "/" + key + " missing " + path);
      if (item === null && !node.nullable && !absent)
        throw new Error(collection + "/" + key + " null " + path);
      const empty2 = absent || item === void 0 || item === null;
      if (node.kind === "object") {
        if (!empty2 && (typeof item !== "object" || Array.isArray(item)))
          throw new Error("Expected object: " + path);
        if (!empty2) {
          for (const name of Object.keys(item))
            if (!Object.prototype.hasOwnProperty.call(node.fields, name))
              throw new Error("Undeclared stored field: " + path + "." + name);
        }
        for (const [name, schema] of Object.entries(node.fields))
          visit(
            schema,
            path === "" ? name : path + "__" + name,
            empty2 ? void 0 : item[name],
            empty2
          );
      } else if (node.kind === "array" || node.kind === "map") {
        if (empty2) {
          row[column + "__count"] = null;
          return;
        }
        if (node.kind === "array" && !Array.isArray(item))
          throw new Error("Expected array: " + path);
        if (node.kind === "map" && (typeof item !== "object" || Array.isArray(item)))
          throw new Error("Expected map: " + path);
        const child = current.children.find(
          (table) => table.name === current.name + "_" + column
        );
        const items = node.kind === "array" ? item.map(
          (entry, index) => [null, entry]
        ) : Object.entries(item);
        row[column + "__count"] = items.length;
        items.forEach(
          ([mapKey, entry], index) => record2(child, childKey(identity2, index), entry, {
            _root: key,
            _parent: identity2,
            _position: index,
            _map_key: mapKey
          })
        );
      } else if (empty2) row[column] = null;
      else if (node.kind === "vector") row[column] = vector(item);
      else if (node.kind === "boolean") {
        if (typeof item !== "boolean")
          throw new Error("Expected boolean: " + path);
        row[column] = item ? 1 : 0;
      } else if (node.kind === "text") {
        if (typeof item !== "string") throw new Error("Expected text: " + path);
        row[column] = item;
      } else {
        if (typeof item !== "number" || !Number.isFinite(item))
          throw new Error("Expected finite number: " + path);
        if (node.kind === "integer" && !Number.isSafeInteger(item))
          throw new Error("Expected safe integer: " + path);
        row[column] = item;
      }
    }
    visit(current.node, "", input);
    rows.get(current.name).push(row);
  }
  record2(root, key, value, {});
  return rows;
}
function indexRelations(root, rows) {
  const index = /* @__PURE__ */ new Map();
  for (const table of tables(root)) {
    for (const child of table.children) {
      const parents = new Map(
        rows.get(table.name).map((row) => [row._key, []])
      );
      for (const row of rows.get(child.name)) {
        const siblings = parents.get(row._parent);
        if (siblings === void 0)
          throw new Error("Orphan relation: " + child.name);
        siblings.push(row);
      }
      for (const siblings of parents.values())
        siblings.sort(
          (left, right) => Number(left._position) - Number(right._position)
        );
      index.set(child.name, parents);
    }
  }
  return index;
}
function decodeLayout(root, row, rows = /* @__PURE__ */ new Map()) {
  function record2(current, stored) {
    function visit(node, path) {
      const column = field(path);
      if (node.optional || node.nullable) {
        const state = Number(stored[column + "__state"]);
        if (state === 0 && node.optional) return void 0;
        if (state === 1 && node.nullable) return null;
        if (state !== 2) throw new Error("Invalid field presence: " + path);
      }
      if (node.kind === "object") {
        const value2 = {};
        for (const [key, schema] of Object.entries(node.fields)) {
          const item = visit(schema, path === "" ? key : path + "__" + key);
          if (item !== void 0) value2[key] = item;
        }
        return value2;
      }
      if (node.kind === "array" || node.kind === "map") {
        const child = current.children.find(
          (table) => table.name === current.name + "_" + column
        );
        const children = rows.get(child.name).get(stored._key);
        if (children.length !== Number(stored[column + "__count"]))
          throw new Error("Incomplete relation: " + child.name);
        children.forEach((item, index) => {
          if (Number(item._position) !== index || item._key !== childKey(String(stored._key), index) || item._revision !== stored._revision)
            throw new Error("Incomplete ordered relation: " + child.name);
        });
        if (node.kind === "array")
          return children.map((item) => record2(child, item));
        const result = {};
        for (const item of children) {
          if (typeof item._map_key !== "string" || Object.prototype.hasOwnProperty.call(result, item._map_key))
            throw new Error("Invalid map relation");
          Object.defineProperty(result, item._map_key, {
            value: record2(child, item),
            enumerable: true
          });
        }
        return result;
      }
      const value = stored[column];
      if (node.kind === "vector") {
        if (!(value instanceof Uint8Array) && Object.prototype.toString.call(value) !== "[object Uint8Array]")
          throw new Error("Invalid embedding bytes");
        const bytes = value;
        if (bytes.byteLength % 8 !== 0)
          throw new Error("Invalid embedding length");
        const view = new DataView(
          bytes.buffer,
          bytes.byteOffset,
          bytes.byteLength
        );
        return Array.from(
          { length: bytes.byteLength / 8 },
          (_, index) => view.getFloat64(index * 8, true)
        );
      }
      if (node.kind === "boolean") {
        if (value !== 0 && value !== 1 && value !== 0n && value !== 1n)
          throw new Error("Invalid stored boolean");
        return Number(value) === 1;
      }
      if (node.kind === "text") {
        if (typeof value !== "string")
          throw new Error("Invalid stored text: " + path);
        return value;
      }
      if (typeof value !== "number" && typeof value !== "bigint")
        throw new Error("Invalid stored number: " + path);
      const number2 = Number(value);
      if (!Number.isFinite(number2) || node.kind === "integer" && !Number.isSafeInteger(number2))
        throw new Error("Invalid stored number: " + path);
      return number2;
    }
    return visit(current.node, "");
  }
  return record2(root, row);
}
var RelationalDatabase = class _RelationalDatabase {
  /** Retains the generic host SQL handle. */
  constructor(sql) {
    this.sql = sql;
    this.sequence = 0;
    this.executionId = Array.from(
      { length: 4 },
      () => Math.floor(Math.random() * 4294967296).toString(16).padStart(8, "0")
    ).join("");
  }
  /** Opens the sole runtime database and declares its exact typed schema. */
  static open() {
    return __async(this, null, function* () {
      const directory = ToolPkg.getSpaceDataDir();
      if (typeof directory !== "string" || directory.trim() === "")
        throw new Error("Plugin data directory is unavailable");
      const sql = yield Tools.Storage.sqlite.open({
        path: directory + "/characters-relational.sqlite"
      });
      for (const root of Object.values(layouts))
        for (const table of tables(root)) yield sql.defineTable(table.table);
      for (const root of Object.values(layouts))
        for (const child of tables(root).slice(1))
          yield sql.execute(
            "CREATE INDEX IF NOT EXISTS " + quote(child.name + "_root_idx") + " ON " + quote(child.name) + ' ("_root", "_parent", "_position")'
          );
      yield sql.execute(
        'CREATE INDEX IF NOT EXISTS "chat_binding_selection_idx" ON "chat_bindings" ("selection")'
      );
      return new _RelationalDatabase(sql);
    });
  }
  /** Reads a collection as typed records with all child rows from the same guarded operation snapshot. */
  read(collection, key) {
    return __async(this, null, function* () {
      const root = layouts[collection];
      if (root === void 0)
        throw new Error("Unknown relational collection: " + collection);
      const rows = /* @__PURE__ */ new Map();
      for (const table of tables(root))
        rows.set(
          table.name,
          yield this.sql.query(
            "SELECT * FROM " + quote(table.name) + (key === void 0 ? "" : " WHERE " + quote(table === root ? "_key" : "_root") + " = ?") + ' ORDER BY "_key"',
            key === void 0 ? [] : [key]
          )
        );
      const relations = indexRelations(root, rows);
      return rows.get(collection).map((row) => {
        if (typeof row._key !== "string" || typeof row._revision !== "string")
          throw new Error("Invalid stored identity");
        return {
          key: row._key,
          version: row._revision,
          value: decodeLayout(root, row, relations)
        };
      });
    });
  }
  /** Reads one exact root identity and its declared child relations. */
  get(collection, key) {
    return __async(this, null, function* () {
      const rows = yield this.read(collection, key);
      return rows.length === 0 ? null : rows[0];
    });
  }
  /** Writes guarded roots and their relations atomically through the host journal. */
  commit(mutations) {
    return __async(this, null, function* () {
      const version = this.executionId + ":" + ++this.sequence;
      const statements = [];
      for (const mutation of mutations) {
        const root = layouts[mutation.collection];
        if (root === void 0)
          throw new Error(
            "Unknown relational collection: " + mutation.collection
          );
        const { key, expectedVersion } = mutation;
        if (mutation.op === "delete") {
          statements.push({
            sql: "DELETE FROM " + quote(root.name) + ' WHERE "_key" = ? AND "_revision" = ?',
            params: [key, expectedVersion],
            expectedAffectedRows: 1
          });
        } else {
          const encoded = encode(root.name, key, mutation.value, version);
          const row = encoded.get(root.name)[0];
          statements.push(rootWrite(root.name, row, expectedVersion));
          if (expectedVersion !== null)
            statements.push(...deleteRelations(root, key));
          for (const table of tables(root).slice(1)) {
            for (const child of encoded.get(table.name))
              statements.push(insertRow(table.name, child));
          }
        }
        if (mutation.op === "delete")
          statements.push(...deleteRelations(root, key));
      }
      if (statements.length === 0 || statements.length > 1e3) {
        throw new Error(
          "Relational operation exceeds the host transaction statement limit"
        );
      }
      yield this.sql.transaction(statements);
      return { version };
    });
  }
};
function insertRow(table, row) {
  const columns = Object.keys(row);
  return {
    sql: "INSERT INTO " + quote(table) + " (" + columns.map(quote).join(", ") + ") VALUES (" + columns.map(() => "?").join(", ") + ")",
    params: columns.map((column) => row[column]),
    expectedAffectedRows: 1
  };
}
function rootWrite(table, row, expectedVersion) {
  if (expectedVersion === null) return insertRow(table, row);
  const columns = Object.keys(row).filter((column) => column !== "_key");
  return {
    sql: "UPDATE " + quote(table) + " SET " + columns.map((column) => quote(column) + " = ?").join(", ") + ' WHERE "_key" = ? AND "_revision" = ?',
    params: [
      ...columns.map((column) => row[column]),
      row._key,
      expectedVersion
    ],
    expectedAffectedRows: 1
  };
}
function deleteRelations(root, key) {
  return tables(root).slice(1).reverse().map((child) => ({
    sql: "DELETE FROM " + quote(child.name) + ' WHERE "_root" = ?',
    params: [key]
  }));
}

// src/chat-storage.ts
var database = null;
function chatDatabase() {
  if (database === null) database = RelationalDatabase.open();
  return database;
}
function findChatBinding(chatId) {
  return __async(this, null, function* () {
    requireId(chatId, "chat id");
    const row = yield (yield chatDatabase()).get("chat_bindings", chatId);
    return row === null ? null : { chatId, selection: decodeChatMarker(row.value).selection };
  });
}

// src/definition.ts
var definition = { id: "com.operit.character_cards", title: "\u89D2\u8272\u5361", icon: "Badge", order: 150 };
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
    completeScreen(value) {
      return __async(this, null, function* () {
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
                const active = identity2.kind === "model" ? yield callMainDomain("activePrompt.setModel", {}) : identity2.kind === "card" ? yield callMainDomain("activePrompt.setCard", { id: identity2.id }) : yield callMainDomain("activePrompt.setGroup", { id: identity2.id });
                if (!active.active || active.id !== identity2.id || active.type !== (identity2.kind === "model" ? "model" : identity2.kind === "card" ? "character_card" : "character_group")) throw new Error("Main runtime did not confirm the selected global active prompt");
              } else {
                const chatId = selectorTarget.chatId;
                const binding = yield callMainDomain("chat.configuration.binding.write", { chatId, selection: selection2 });
                if (binding.chatId !== chatId || binding.selection !== selection2) throw new Error("Main runtime did not confirm the selected chat binding");
              }
              const complete = session.completeScreen(result);
              completion = { ...complete, value: { selection: selection2, contextKey: selection2 } };
              break;
            }
            case "memory-attachment": {
              const complete = session.completeScreen(result);
              completion = { ...complete, value: { type: "text", name: "\u8BB0\u5FC6\u9644\u4EF6", content: result.content, mediaType: "text/plain" } };
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
      });
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

// src/ui-selector.ts
function selectorSession(presentation) {
  if (presentation === null || presentation.input.mode !== "select") throw new Error("The selection route requires an explicit select presentation");
  const { chatId } = presentation.input;
  if (chatId !== null) chatIdentity(chatId, "selector input.chatId");
  const session = createUiScreenSession(presentation), current = session.currentScreen();
  if (current.requestId === null || current.input.mode !== "select") throw new Error("The selection route requires a real presentation request");
  return { input: { ...current.input, chatId }, session };
}
function readSelectorData(input) {
  return __async(this, null, function* () {
    const [cards, groups] = yield Promise.all([callMainDomain("character.list", {}), callMainDomain("group.list", {})]);
    let selected;
    if (input.chatId === null) selected = encodeSelection(yield callMainDomain("activePrompt.get", {}));
    else {
      const binding = yield findChatBinding(input.chatId);
      selected = binding === null ? "model:chat" : binding.selection;
    }
    if (selected !== null) requireChatSelection(selected, cards, groups);
    const options = [];
    if (input.kind === "card" || input.kind === "all") options.push({ key: "model:chat", token: "model:chat", kind: "model", id: "chat", title: "\u4EC5\u4F7F\u7528\u6A21\u578B", description: "\u4E0D\u4F7F\u7528\u89D2\u8272\u5361\u63D0\u793A\u8BCD\u548C\u89D2\u8272\u8BB0\u5FC6", selection: { Model: { id: "chat" } }, avatar: { type: "model" } });
    if (input.kind === "card" || input.kind === "all") {
      for (const card of cards) {
        let avatar;
        if (card.avatarUri !== null) avatar = { type: "uri", uri: card.avatarUri };
        else avatar = { type: "resource", path: null };
        options.push({
          key: "card:" + card.id,
          token: "card:" + card.id,
          kind: "card",
          id: card.id,
          title: card.name,
          description: card.description,
          selection: { CharacterCard: { id: card.id } },
          avatar
        });
      }
    }
    if (input.kind === "group" || input.kind === "all") for (const group of groups) options.push({
      key: "group:" + group.id,
      token: "group:" + group.id,
      kind: "group",
      id: group.id,
      title: group.name,
      description: group.description,
      selection: { CharacterGroup: { id: group.id } },
      avatar: { type: "group" }
    });
    const keys = /* @__PURE__ */ new Set();
    for (const option of options) {
      if (keys.has(option.key)) throw new Error("Duplicate selector row identity: " + option.key);
      keys.add(option.key);
    }
    return { options, selected };
  });
}
function loadSelectorAvatars(data) {
  return __async(this, null, function* () {
    if (!data.options.some((option) => option.avatar.type === "resource" && option.avatar.path === null)) return data;
    const path = yield ToolPkg.readResource("character_default_avatar", "operit-avatar.png");
    if (path.trim() === "") throw new Error("Declared character default-avatar resource did not resolve");
    return { ...data, options: data.options.map((option) => option.avatar.type === "resource" && option.avatar.path === null ? { ...option, avatar: { type: "resource", path } } : option) };
  });
}
function createSelectorController(input, session, publish, assertOwner) {
  let state = { data: null, loading: true, switchingKey: null, error: "", finished: false }, loading = null;
  function update(changes) {
    state = { ...state, ...changes };
    publish(state);
  }
  function idle() {
    assertOwner();
    if (state.finished) throw new Error("This selector presentation has already finished");
    if (state.switchingKey !== null) throw new Error("A selector binding commit is already running");
  }
  return {
    /** Supplies the authoritative current local state to the renderer. */
    current() {
      return state;
    },
    /** Retains the first real load result or rejection; node rerenders do not initiate implicit source retries. */
    load() {
      assertOwner();
      if (loading === null) loading = readSelectorData(input).then(
        /** Installs genuine persisted records only while this request remains open. */
        (data) => {
          assertOwner();
          if (state.finished) return;
          update({ data, loading: false, error: "" });
          void loadSelectorAvatars(data).then((resolved) => {
            assertOwner();
            if (!state.finished && state.data === data) update({ data: resolved });
          }).catch(() => {
          });
        },
        /** Keeps a real source failure visible and propagates the same original error to the host. */
        (failure) => {
          update({ loading: false, error: String(failure) });
          throw failure;
        }
      );
      return loading;
    },
    /** Matches the original popup: one row click awaits the real switch, then returns an explicit completion. */
    select(key) {
      return __async(this, null, function* () {
        idle();
        if (state.loading || state.data === null) throw new Error("Selector records have not been loaded successfully");
        const options = state.data.options.filter(
          /** Resolves exactly one genuine allowed row rather than a caller-supplied role identity. */
          (option) => option.key === key
        );
        if (options.length !== 1) throw new Error("Selector row is not one actual available choice: " + key);
        update({ switchingKey: key, error: "" });
        try {
          const complete = yield session.completeScreen({ mode: "select", selection: options[0].selection });
          assertOwner();
          update({ finished: true });
          return complete;
        } catch (failure) {
          update({ error: String(failure) });
          throw failure;
        } finally {
          update({ switchingKey: null });
        }
      });
    },
    /** Closes only this request and never commits a staged choice or modifies another selector. */
    cancel() {
      idle();
      const cancel = session.cancelScreen();
      update({ finished: true });
      return cancel;
    }
  };
}
function selectorResultJson(result) {
  switch (result.type) {
    case "toolpkg.presentation.complete":
      return { type: result.type, requestId: result.requestId, value: result.value };
    case "toolpkg.presentation.cancel":
      return { type: result.type, requestId: result.requestId };
  }
}
function selectorAvatar(ctx, option) {
  const props = { width: 28, height: 28, contentScale: "crop", contentDescription: option.title, modifier: ctx.Modifier.clip({ type: "circle" }) };
  switch (option.avatar.type) {
    case "uri":
      return OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:71535",[{ ...props, uri: option.avatar.uri }],(...__operitArgs)=>ctx.UI.Image(...__operitArgs));
    case "resource":
      return option.avatar.path === null ? OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:71651",[{ width: 28, height: 28, contentAlignment: "center" },OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:71717",[{ name: "Person", size: 22, tint: "onSurfaceVariant", contentDescription: option.title }],(...__operitArgs)=>ctx.UI.Icon(...__operitArgs))],(...__operitArgs)=>ctx.UI.Box(...__operitArgs)) : OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:71822",[{ ...props, path: option.avatar.path }],(...__operitArgs)=>ctx.UI.Image(...__operitArgs));
    case "model":
      return OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:71907",[{ width: 28, height: 28, contentAlignment: "center" },OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:71973",[{ name: "SmartToy", size: 22, tint: "onSurfaceVariant", contentDescription: option.title }],(...__operitArgs)=>ctx.UI.Icon(...__operitArgs))],(...__operitArgs)=>ctx.UI.Box(...__operitArgs));
    case "group":
      return OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:72110",[{ width: 28, height: 28, contentAlignment: "center" },OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:72176",[{ name: "Groups", size: 22, tint: "onSurfaceVariant", contentDescription: option.title }],(...__operitArgs)=>ctx.UI.Icon(...__operitArgs))],(...__operitArgs)=>ctx.UI.Box(...__operitArgs));
  }
}
function selectorRow(ctx, option, state, controller) {
  if (state.data === null) throw new Error("A selector row requires its actual loaded catalog");
  const active = option.token === state.data.selected, switching = state.switchingKey === option.key;
  const enabled = state.switchingKey === null && !state.finished, colors = ctx.MaterialTheme.colorScheme;
  const titleColor = active ? colors.primary : enabled ? colors.onSurface : colors.onSurfaceVariant.copy({ alpha: 0.65 });
  const descriptionColor = enabled ? colors.onSurfaceVariant : colors.onSurfaceVariant.copy({ alpha: 0.5 });
  const props = { key: option.key, fillMaxWidth: true, paddingStart: 10, paddingTop: 7, paddingEnd: 8, paddingBottom: 7, verticalAlignment: "center", spacing: 8 };
  const contents = [
    selectorAvatar(ctx, option),
    OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:73100",[{ weight: 1, horizontalAlignment: "start" },[
      OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:73167",[{ text: option.title, style: "bodySmall", maxLines: 1, overflow: "ellipsis", color: titleColor, fontWeight: active ? "700" : "600" }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs)),
      ...option.description === "" ? [] : [OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:73357",[{ text: option.description, style: "labelSmall", maxLines: 1, overflow: "ellipsis", color: descriptionColor }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs))]
    ]],(...__operitArgs)=>ctx.UI.Column(...__operitArgs)),
    OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:73493",[{ width: 20, height: 20, contentAlignment: "center" },switching ? OperitComposeGeneration.leaf(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:73571",[],()=>[{ width: 16, height: 16, strokeWidth: 2 }],(...__operitArgs)=>ctx.UI.CircularProgressIndicator(...__operitArgs)) : OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:73649",[{ name: active ? "Check" : "CircleOutlined", size: active ? 18 : 16, tint: active ? colors.primary : colors.onSurfaceVariant.copy({ alpha: 0.45 }), contentDescription: active ? "\u5F53\u524D\u9009\u4E2D" : "\u672A\u9009\u4E2D" }],(...__operitArgs)=>ctx.UI.Icon(...__operitArgs))],(...__operitArgs)=>ctx.UI.Box(...__operitArgs))
  ];
  if (!enabled) return OperitComposeGeneration.leaf(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:73920",[props,contents],()=>[props,contents],(...__operitArgs)=>ctx.UI.Row(...__operitArgs));
  return OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:73958",[{
    ...props,
    /** Returns the real commit result to the existing generic Compose action channel without a synthetic receiver. */
    onClick: () => __async(null, null, function* () {
      return selectorResultJson(yield controller.select(option.key));
    })
  },contents],(...__operitArgs)=>ctx.UI.Row(...__operitArgs));
}
function selectorView(ctx, input, state, controller) {
  const title = input.kind === "card" ? "\u5207\u6362\u89D2\u8272\u5361" : input.kind === "group" ? "\u5207\u6362\u7FA4\u7EC4" : "\u5207\u6362\u89D2\u8272\u5361\u6216\u7FA4\u7EC4";
  const colors = ctx.MaterialTheme.colorScheme;
  const header = OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:74554",[{ fillMaxWidth: true, paddingStart: 16, paddingTop: 8, paddingEnd: 8, paddingBottom: 4, verticalAlignment: "center" },[
    OperitComposeGeneration.leaf(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:74690",[title],()=>[{ text: title, style: "titleSmall", fontWeight: "700", weight: 1 }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs)),
    ...state.data === null ? [] : [OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:74806",[{ text: state.data.options.length + " \u4E2A", style: "labelSmall", color: colors.onSurfaceVariant }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs)), OperitComposeGeneration.leaf(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:74921",[],()=>[{ width: 2 }],(...__operitArgs)=>ctx.UI.Spacer(...__operitArgs))],
    OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:74955",[{
      key: "selector-close",
      enabled: state.switchingKey === null && !state.finished,
      width: 32,
      height: 32,
      /** Emits the explicit cancellation discriminator, not a route pop that bypasses the plugin session. */
      onClick: () => selectorResultJson(controller.cancel())
    },OperitComposeGeneration.leaf(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:75280",[],()=>[{ name: "Close", size: 18, contentDescription: "\u5173\u95ED" }],(...__operitArgs)=>ctx.UI.Icon(...__operitArgs))],(...__operitArgs)=>ctx.UI.IconButton(...__operitArgs))
  ]],(...__operitArgs)=>ctx.UI.Row(...__operitArgs));
  let body;
  if (state.loading) body = OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:75404",[{ weight: 1, fillMaxWidth: true, contentAlignment: "center" },OperitComposeGeneration.leaf(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:75478",[],()=>[{ width: 20, height: 20, strokeWidth: 2 }],(...__operitArgs)=>ctx.UI.CircularProgressIndicator(...__operitArgs))],(...__operitArgs)=>ctx.UI.Box(...__operitArgs));
  else if (state.data === null) body = OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:75595",[{ key: "selector-load-error", text: state.error, color: colors.error, paddingHorizontal: 16, paddingVertical: 18, weight: 1 }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs));
  else if (state.data.options.length === 0) body = OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:75786",[{ text: input.kind === "group" ? "\u6682\u65E0\u7FA4\u7EC4" : "\u6682\u65E0\u89D2\u8272\u5361", style: "bodySmall", color: colors.onSurfaceVariant, paddingStart: 16, paddingTop: 18, paddingEnd: 16, paddingBottom: 20, weight: 1 }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs));
  else {
    const options = state.data.options;
    const items = ctx.lazyItems(
      { key: "selector-records", itemCount: options.length, itemExtent: 56, initialItemCount: 12 },
      /** Builds one keyed row only when it enters the materialized viewport window. */
      (index) => OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:76316",[{ key: "selector-item:" + options[index].key, fillMaxWidth: true },[
        selectorRow(ctx, options[index], state, controller),
        ...index + 1 === options.length ? [] : [OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:76509",[{ thickness: 1, color: colors.outlineVariant.copy({ alpha: 0.45 }) }],(...__operitArgs)=>ctx.UI.HorizontalDivider(...__operitArgs))]
      ]],(...__operitArgs)=>ctx.UI.Column(...__operitArgs))
    );
    body = OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:76632",[{ key: "selector-records", weight: 1, fillMaxWidth: true, paddingStart: 8, paddingTop: 4, paddingEnd: 8, paddingBottom: 8, ...items }],(...__operitArgs)=>ctx.UI.LazyColumn(...__operitArgs));
  }
  return OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:76799",[{
    key: "character-selection",
    containerColor: "surfaceContainerHigh",
    shape: { cornerRadius: 28 },
    closeOnDismissRequest: false,
    properties: { usePlatformDefaultWidth: false, dismissOnBackPress: state.switchingKey === null, dismissOnClickOutside: state.switchingKey === null },
    /** Loads once through the main domain IPC; a rerender cannot initialize another store or retry a failed read. */
    onLoad: () => controller.load(),
    /** Returns explicit V1 cancellation for host dismissal rather than changing a global active selection. */
    onDismissRequest: () => selectorResultJson(controller.cancel())
  },OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:77450",[{ fillMaxWidth: true, height: 420, modifier: ctx.Modifier.widthIn({ maxWidth: 360 }).heightIn({ minHeight: 420, maxHeight: 420 }) },[
    header,
    OperitComposeGeneration.leaf(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:77615",[],()=>[{ thickness: 1 }],(...__operitArgs)=>ctx.UI.HorizontalDivider(...__operitArgs)),
    body,
    ...state.error === "" || state.data === null ? [] : [OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/selection/index.ui.js:77726",[{ key: "selector-commit-error", text: state.error, color: colors.error, style: "bodySmall", paddingHorizontal: 16, paddingVertical: 8 }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs))]
  ]],(...__operitArgs)=>ctx.UI.Column(...__operitArgs))],(...__operitArgs)=>ctx.UI.Dialog(...__operitArgs));
}
function renderSelectionScreen(ctx) {
  const [presentation] = ctx.useState("presentation", null);
  const [state, setState] = ctx.useState("native-selector-state", { data: null, loading: true, switchingKey: null, error: "", finished: false });
  const stored = ctx.useRef("native-selector-controller", null);
  const identity2 = ctx.useRef("native-selector-request", JSON.stringify(presentation));
  function assertOwner() {
    const [current] = ctx.useState("presentation", null);
    if (identity2.current !== JSON.stringify(current)) throw new Error("A native selector cannot change its owning presentation request");
  }
  assertOwner();
  const parsed = selectorSession(presentation);
  if (stored.current === null) stored.current = createSelectorController(parsed.input, parsed.session, setState, assertOwner);
  return selectorView(ctx, parsed.input, state, stored.current);
}

// src/ui/selection/index.ui.ts
function Screen(ctx) {
  return renderSelectionScreen(ctx);
}

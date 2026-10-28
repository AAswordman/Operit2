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

// src/ui/memory-attachment/index.ui.ts
var index_ui_exports = {};
__export(index_ui_exports, {
  default: () => Screen
});
module.exports = __toCommonJS(index_ui_exports);

// src/avatar-resources.ts
function createAvatarResources(origin) {
  const references = /* @__PURE__ */ new Map();
  const files = /* @__PURE__ */ new Map();
  const remote = /* @__PURE__ */ new Set();
  let defaultPath = null;
  let next = 0;
  function source(path) {
    return __async(this, null, function* () {
      if (path === null) {
        if (defaultPath === null) {
          const pending = ToolPkg.readResource("character_default_avatar", "operit-avatar.png").then((path2) => {
            if (path2.trim() === "") throw new Error("默认头像资源路径不能为空");
            return path2;
          });
          defaultPath = pending;
          pending.catch(() => {
            if (defaultPath === pending) defaultPath = null;
          });
        }
        path = yield defaultPath;
      }
      if (path.trim() === "") throw new Error("头像资源路径不能为空");
      if (/^data:image\//i.test(path)) return path;
      const existing = references.get(path);
      if (existing !== void 0) return existing;
      const reference = "avatars/" + ++next;
      references.set(path, reference);
      files.set(origin + reference, path);
      if (/^https?:\/\//i.test(path)) remote.add(path);
      return reference;
    });
  }
  function intercept(request2) {
    if (remote.has(request2.url) && !request2.isMainFrame && (request2.method === "GET" || request2.method === "HEAD")) return { action: "allow" };
    const path = files.get(request2.url);
    if (path === void 0) return null;
    if (request2.isMainFrame || request2.method !== "GET" && request2.method !== "HEAD") return { action: "block" };
    if (remote.has(path)) return { action: "rewrite", url: path };
    return { action: "respond", response: { statusCode: 200, reasonPhrase: "OK", filePath: path } };
  }
  return { source, intercept };
}

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
function requireId(value, label) {
  assertString(value, label);
  if (value.trim() === "" || value !== value.trim()) throw new Error(`${label} 无效`);
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
    complete(operation, id2) {
      return __async(this, null, function* () {
        assertOwner();
        finish(operation === void 0 ? null : { operation, id: id2 ?? pending.input.id });
        return null;
      });
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
  function act(operation) {
    return __async(this, null, function* () {
      assertOwner();
      if (live.current.busy || live.current.finished) return null;
      update({ busy: true, error: "" });
      try {
        const value = yield operation();
        if (!live.current.finished) assertOwner();
        return value ?? null;
      } catch (error) {
        assertOwner();
        update({ error: String(error) });
      } finally {
        update({ busy: false });
      }
      return null;
    });
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
  function loadThemes() {
    return __async(this, null, function* () {
      try {
        const themes = yield request({ action: "listThemeChoices" });
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
    });
  }
  function loadDefaultAvatar() {
    return __async(this, null, function* () {
      try {
        const avatar = yield ToolPkg.readResource("character_default_avatar", "operit-avatar.png");
        if (avatar.trim() === "" || live.current.finished) return;
        assertOwner();
        update({ avatar });
      } catch {
      }
    });
  }
  function load() {
    return __async(this, null, function* () {
      yield act(() => __async(null, null, function* () {
        const snapshot = yield request({ action: "snapshot" });
        assertOwner();
        const record2 = input.mode === "create" ? input.entity === "card" ? createCharacterDraft() : createGroupDraft() : input.entity === "card" ? snapshot.cards.find((value) => value.id === input.id) : snapshot.groups.find((value) => value.id === input.id);
        if (record2 === void 0) throw new Error("预览实体不存在：" + input.id);
        assertOwner();
        update({ snapshot, tags: copy(snapshot.tags), card: input.entity === "card" ? copy(record2) : null, group: input.entity === "group" ? copy(record2) : null, loading: false });
        if (input.entity === "group" || record2.avatarUri === null) void loadDefaultAvatar();
        if (live.current.editing) void loadThemes();
      }));
      if (live.current.loading) update({ loading: false });
    });
  }
  function close() {
    return __async(this, null, function* () {
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
    });
  }
  function edit() {
    return __async(this, null, function* () {
      assertOwner();
      update({ editing: true, tab: 0 });
      if (live.current.themes === null) void loadThemes();
    });
  }
  function togglePreviewConfiguration() {
    assertOwner();
    const expanded = !live.current.previewConfigurationExpanded;
    update({ previewConfigurationExpanded: expanded });
    if (expanded && (live.current.card ?? live.current.group)?.themeConfigId != null && live.current.themes === null) void loadThemes();
  }
  function save() {
    return __async(this, null, function* () {
      return act(() => __async(null, null, function* () {
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
          snapshot = yield request({ action: "saveCharacter", card: copy(draft), create: input.mode === "create", tagChanges: copy(live.current.tagChanges) });
        } else {
          const draft = group();
          if (draft.name.trim() === "") throw new Error("群组名称不能为空");
          snapshot = yield request({ action: "saveGroup", group: copy(draft), create: input.mode === "create" });
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
      }));
    });
  }
  function stageTag() {
    return __async(this, null, function* () {
      return act(() => __async(null, null, function* () {
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
      }));
    });
  }
  function remove() {
    return __async(this, null, function* () {
      return act(() => __async(null, null, function* () {
        yield request(input.entity === "card" ? { action: "deleteCharacter", id: input.id } : { action: "deleteGroup", id: input.id });
        assertOwner();
        update({ finished: true });
        return lifecycle.complete(input.mode === "preview" ? void 0 : "deleted");
      }));
    });
  }
  function chooseAvatar() {
    return __async(this, null, function* () {
      yield act(() => __async(null, null, function* () {
        const selected = yield ctx.openFilePicker({ picker: "image", allowMultiple: false, mimeTypes: ["image/*"], maxContentBytes: MAX_AVATAR_BYTES });
        assertOwner();
        if (selected.cancelled) return;
        if (selected.files.length !== 1) throw new Error("请选择一个头像文件");
        const contentBase64 = readAvatarImport(selected.files[0]);
        assertOwner();
        const directory = ToolPkg.getConfigDir().replace(/\/$/, "") + "/avatars";
        const created = yield Tools.Files.mkdir(directory, true);
        if (!created.successful) throw new Error(created.details);
        const uri = directory + "/" + Date.now() + "-" + Math.random().toString(36).slice(2) + ".image";
        const written = yield Tools.Files.writeBinary(uri, contentBase64);
        if (!written.successful) throw new Error(written.details);
        assertOwner();
        update({ card: { ...card(), avatarUri: uri } });
      }));
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
    const exportFile = (format2) => __async(null, null, function* () {
      yield act(() => __async(null, null, function* () {
        const content = input.entity === "card" ? yield request({ action: "exportCharacter", id: input.id, format: format2 }) : yield request({ action: "exportGroup", id: input.id });
        assertOwner();
        const name = input.entity === "card" ? card().name : group().name;
        update({ exportContent: content, exportName: name.replace(/[<>:"/\\|?*\u0000-\u001f]/g, "_") + "-" + format2 + ".json" });
      }));
    });
    const writeExport = () => __async(null, null, function* () {
      yield act(() => __async(null, null, function* () {
        const result = yield ctx.saveFile({ suggestedName: live.current.exportName, content: live.current.exportContent, mimeType: "application/json", extensions: ["json"] });
        assertOwner();
        if (result.cancelled) return;
        update({ exporting: false, exportContent: "", exportName: "" });
      }));
    });
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
            ...draft.avatarUri === null ? [] : [standardButton("entity-clear-avatar", "清除", () => __async(null, null, function* () {
              yield act(() => __async(null, null, function* () {
                assertOwner();
                update({ card: { ...card(), avatarUri: null } });
                if (live.current.avatar === null) void loadDefaultAvatar();
              }));
            }))]
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
        if (state.themeError !== "") pages[2].push(ctx.UI.Text({ text: state.themeError, color: colors.error }), standardButton("entity-theme-retry", "重新加载主题", () => __async(null, null, function* () {
          yield act(loadThemes);
        })));
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

// src/ui-editor.ts
function webArguments(value, expected) {
  if (value.length !== 1 || !Array.isArray(value[0]) || value[0].length !== expected) throw new Error(`CharacterMemoryHost expects ${expected} arguments`);
  return value[0];
}
function renderScreen(ctx, definition2, requiredMode = null, managementView = "characters") {
  const controller = ctx.createWebViewController("character-memory-web");
  const [path, setPath] = ctx.useState("character-memory-html", "");
  const [error, setError] = ctx.useState("character-memory-error", "");
  const ready = ctx.useRef("character-memory-ready", false);
  const unsubscribe = ctx.useRef("character-memory-theme-subscription", null);
  const [presentation] = ctx.useState("presentation", null);
  const session = ctx.useRef("character-memory-screen-session", null);
  if (session.current === null) session.current = createUiScreenSession(presentation);
  const screenSession = session.current;
  const [managed, setManaged] = ctx.useState("managed-entity", null);
  const managedLive = ctx.useRef("managed-entity-live", managed);
  managedLive.current = managed;
  const nextManaged = ctx.useRef("managed-entity-counter", 0);
  function finishEntity(result) {
    managedLive.current = null;
    setManaged(null);
    void controller.evaluateJavascript("window.dispatchEvent(new CustomEvent('character-entity-closed', { detail: " + JSON.stringify(result) + " }));").catch((error2) => setError(String(error2)));
  }
  if (requiredMode !== null && screenSession.currentScreen().input.mode !== requiredMode) throw new Error("The attachment route requires an explicit attachment presentation");
  const origin = "https://characters.operit.local/";
  const avatars = ctx.useRef("character-memory-avatar-resources", createAvatarResources(origin)).current;
  function applyTheme(theme) {
    return __async(this, null, function* () {
      if (ready.current) yield controller.evaluateJavascript(`window.applyCharacterMemoryTheme(${JSON.stringify(theme)});`);
    });
  }
  function initialize() {
    return __async(this, null, function* () {
      try {
        controller.addJavascriptInterface("CharacterMemoryHost", {
          /** Loads the initial route, palette and domain aggregate in one existing bridge invocation. */
          loadScreen: (...args) => __async(null, null, function* () {
            webArguments(args, 0);
            const current = screenSession.currentScreen();
            const screen = current.input.mode === "manage" && managementView === "memory" ? { ...current, input: { mode: "manage", view: "memory" } } : current;
            const snapshot = yield ToolPkg.ipc.call("character-memory.request", { action: "snapshot" }, { targetRuntime: "main" });
            ready.current = true;
            return { theme: ctx.Theme.getCurrent(), screen, snapshot };
          }),
          /** Routes all editing operations through this package's main runtime. */
          request: (...args) => __async(null, null, function* () {
            const [request2] = webArguments(args, 1);
            const decoded = record(request2, "character-memory.request");
            return ToolPkg.ipc.call("character-memory.request", decoded, { targetRuntime: "main" });
          }),
          /** Returns an explicit completion action result for this exact presented screen. */
          completeScreen: (...args) => {
            const [result] = webArguments(args, 1);
            const current = screenSession.currentScreen();
            if (current.input.mode === "manage") throw new Error("Management has no presentation completion channel");
            return screenSession.completeScreen(parseScreenResult(result, current.input));
          },
          /** Returns an explicit cancellation action result without changing the selected identity. */
          cancelScreen: (...args) => {
            webArguments(args, 0);
            return screenSession.cancelScreen();
          },
          /** Returns a document resource reference without transporting image bytes through JS. */
          avatarImage: (...args) => __async(null, null, function* () {
            const [uri] = webArguments(args, 1);
            if (uri !== null && typeof uri !== "string") throw new Error("头像路径必须是字符串或 null");
            return avatars.source(uri);
          }),
          /** Opens the same native entity renderer used by chat avatars and the sidebar. */
          openEntity: (...args) => {
            const [mode, entity, id2] = webArguments(args, 3);
            if (screenSession.currentScreen().input.mode !== "manage") throw new Error("Only management can open a local entity dialog");
            if (mode !== "create" && mode !== "edit" && mode !== "preview" || entity !== "card" && entity !== "group" || typeof id2 !== "string" || (mode === "create" ? id2 !== "" : id2.trim() === "")) throw new Error("Invalid native entity dialog request");
            if (managedLive.current !== null) throw new Error("An entity dialog is already open");
            const request2 = { requestId: "management-" + ++nextManaged.current, input: mode === "create" ? { mode, entity, id: "" } : { mode, entity, id: id2 } };
            managedLive.current = request2;
            setManaged(request2);
            return null;
          },
          /** Reads the selected document through its host-provided VFS resource. */
          readImportFile: (...args) => __async(null, null, function* () {
            webArguments(args, 0);
            const selected = yield ctx.openFilePicker({ picker: "document", allowMultiple: false, mimeTypes: ["application/json"] });
            if (selected.cancelled) return { cancelled: true };
            if (selected.files.length !== 1) throw new Error("请选择一个 JSON 文件");
            const file = yield Tools.Files.read(selected.files[0].path);
            const content = file.content.replace(/^\uFEFF/, "");
            if (content.trim() === "") throw new Error("所选 JSON 文件为空");
            return { cancelled: false, content };
          }),
          /** Saves an export through the same host boundary used for file selection. */
          exportFile: (...args) => __async(null, null, function* () {
            const [name, content] = webArguments(args, 2);
            if (typeof name !== "string" || name.trim() === "") throw new Error("导出文件名不能为空");
            if (typeof content !== "string") throw new Error("导出内容必须是字符串");
            return ctx.saveFile({ suggestedName: name, content, mimeType: "application/json", extensions: ["json"] });
          })
        });
        unsubscribe.current = ctx.Theme.subscribe(applyTheme);
        setPath(yield ToolPkg.readResource("character_memory_html", "character-memory.html"));
      } catch (failure) {
        setError(String(failure));
      }
    });
  }
  function dispose() {
    ready.current = false;
    if (unsubscribe.current !== null) {
      unsubscribe.current();
      unsubscribe.current = null;
    }
  }
  return ctx.UI.Box({ fillMaxSize: true, onLoad: initialize }, [path === "" ? ctx.UI.Text({ text: error === "" ? `正在加载${managementView === "memory" ? "记忆" : definition2.title}…` : error }) : ctx.UI.WebView({
    key: "character-memory-web",
    controller,
    fillMaxSize: true,
    url: origin,
    javaScriptEnabled: true,
    domStorageEnabled: true,
    supportZoom: false,
    useWideViewPort: true,
    /** Restricts top-level navigation to the package document. */
    onShouldOverrideUrlLoading: (request2) => request2.url === origin ? { action: "allow" } : { action: "cancel" },
    /** Serves the offline document and its registered images through the resource host. */
    onInterceptRequest: (request2) => avatars.intercept(request2) ?? (request2.url === origin ? { action: "respond", response: { mimeType: "text/html", encoding: "utf-8", statusCode: 200, reasonPhrase: "OK", filePath: path } } : { action: "block" }),
    /** Handles disposal without retaining the screen in the host theme service. */
    onLifecycleEvent: (event) => {
      if (event.type === "Disposed") dispose();
    }
  }), ...managed === null ? [] : [renderManagedEntityScreen(ctx, managed, finishEntity)]]);
}

// src/ui/memory-attachment/index.ui.ts
function Screen(ctx) {
  return renderScreen(ctx, definition, "memory-attachment");
}

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

// src/ui/memory/index.ui.ts
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
            if (path2.trim() === "") throw new Error("\u9ED8\u8BA4\u5934\u50CF\u8D44\u6E90\u8DEF\u5F84\u4E0D\u80FD\u4E3A\u7A7A");
            return path2;
          });
          defaultPath = pending;
          pending.catch(() => {
            if (defaultPath === pending) defaultPath = null;
          });
        }
        path = yield defaultPath;
      }
      if (path.trim() === "") throw new Error("\u5934\u50CF\u8D44\u6E90\u8DEF\u5F84\u4E0D\u80FD\u4E3A\u7A7A");
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
  chatProviderId: optional(nullable(text)),
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
  const request2 = value;
  if (request2.purpose === "display") {
    if (request2.participantId !== null || request2.messageExtension !== null) throw new Error("Display configuration cannot request an execution participant or message snapshot");
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
    isDefault: false,
    createdAt: 0,
    updatedAt: 0
  };
}
function createGroupDraft() {
  return { id: "", name: "", description: "", members: [], themeConfigId: null, createdAt: 0, updatedAt: 0 };
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
      chatProviderId: {
        kind: "text",
        nullable: true,
        nullEncoding: "column"
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
  schema_migrations: {
    kind: "object",
    fields: {
      version: {
        kind: "integer"
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
function field(path) {
  return path === "" ? "_value" : path;
}
function usesPresenceColumn(node) {
  if (node.nullEncoding === "column") {
    if (!node.nullable || node.optional || node.kind === "object" || node.kind === "array" || node.kind === "map")
      throw new Error("Column null encoding requires a required nullable scalar");
    return false;
  }
  return node.optional === true || node.nullable === true;
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
    if (usesPresenceColumn(schema))
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

// src/image-source.ts
function readAvatarImport(path) {
  return __async(this, null, function* () {
    const file = yield Tools.Files.readBinary(path);
    if (file.size > 8 * 1024 * 1024) throw new Error("\u5934\u50CF\u56FE\u7247\u4E0D\u80FD\u8D85\u8FC7 8 MB");
    const b = file.contentBase64;
    if (!(b.startsWith("iVBOR") || b.startsWith("/9j/") || b.startsWith("R0lGOD") || b.startsWith("UklGR") || b.startsWith("Qk"))) {
      throw new Error("\u8BF7\u9009\u62E9 PNG\u3001JPEG\u3001GIF\u3001WebP \u6216 BMP \u56FE\u7247");
    }
    return b;
  });
}
function pickedImagePath(path, platform) {
  const normalized = path.replace(/\\/g, "/");
  if (normalized.startsWith("/app/") || normalized.startsWith("/mnt/")) return normalized;
  if (platform === "windows" && /^[a-z]:\//i.test(normalized)) return `/mnt/windows/${normalized[0].toLowerCase()}/${normalized.slice(3)}`;
  if (!normalized.startsWith("/")) throw new Error("\u9009\u62E9\u5668\u6CA1\u6709\u8FD4\u56DE\u53EF\u8BFB\u53D6\u7684\u6587\u4EF6\u8DEF\u5F84");
  if (platform === "macos" || platform === "linux") return `/mnt/${platform}${normalized}`;
  if (platform === "android") return `/mnt/android/root${normalized}`;
  throw new Error("\u5F53\u524D\u5E73\u53F0\u4E0D\u652F\u6301\u5BFC\u5165\u6B64\u56FE\u7247\u8DEF\u5F84");
}

// src/ui-entity-preview.ts
function renderEntityPreview(ctx, input) {
  const colors = ctx.MaterialTheme.colorScheme;
  const entity = input.card ?? input.group;
  if (entity === null) return [];
  const card = input.card;
  const selectable = (key, value, subdued = false) => OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:54555",[{ key },OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:54590",[{ text: value, fillMaxWidth: true, style: "bodyMedium", color: subdued ? colors.onSurfaceVariant : colors.onSurface }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs))],(...__operitArgs)=>ctx.UI.SelectionContainer(...__operitArgs));
  const pill = (key, label) => OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:54754",[{
    key,
    paddingHorizontal: 10,
    paddingVertical: 5,
    backgroundColor: colors.primaryContainer.copy({ alpha: 0.38 }),
    backgroundShape: { cornerRadius: 8 }
  },OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:54941",[{ text: label, style: "labelMedium", color: colors.onPrimaryContainer }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs))],(...__operitArgs)=>ctx.UI.Box(...__operitArgs));
  const attachedTags = card === null ? [] : input.tags.filter((tag) => card.attachedTagIds.includes(tag.id));
  const identity2 = OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:55158",[{ key: "entity-preview-identity", fillMaxWidth: true, spacing: 14, paddingBottom: 22 },[
    OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:55266",[{ fillMaxWidth: true, spacing: 16, verticalAlignment: "center" },[
      card === null ? OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:55367",[{ width: 72, height: 72, contentAlignment: "center", backgroundColor: colors.primaryContainer.copy({ alpha: 0.48 }), backgroundShape: { cornerRadius: 20 } },OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:55536",[{ name: "GroupsOutlined", size: 34, tint: colors.primary }],(...__operitArgs)=>ctx.UI.Icon(...__operitArgs))],(...__operitArgs)=>ctx.UI.Box(...__operitArgs)) : OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:55611",[{
        key: "entity-avatar",
        ...card.avatarUri === null ? { path: input.avatar ?? void 0 } : { uri: card.avatarUri },
        width: 72,
        height: 72,
        contentScale: "crop",
        contentDescription: card.name,
        modifier: ctx.Modifier.clip({ cornerRadius: 20 })
      }],(...__operitArgs)=>ctx.UI.Image(...__operitArgs)),
      OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:55935",[{ weight: 1, spacing: 6 },[
        OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:55986",[{ key: "entity-preview-name", text: entity.name, style: "headlineSmall", fontSize: 24, fontWeight: "700", fillMaxWidth: true }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs)),
        ...card === null || card.isDefault ? [OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:56173",[{ text: card === null ? input.group.members.length + " \u4F4D\u89D2\u8272" : "\u9ED8\u8BA4\u89D2\u8272\u5361", style: "labelMedium", color: colors.onSurfaceVariant }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs))] : []
      ]],(...__operitArgs)=>ctx.UI.Column(...__operitArgs))
    ]],(...__operitArgs)=>ctx.UI.Row(...__operitArgs)),
    ...entity.description.trim() === "" ? [] : [selectable("entity-preview-description", entity.description, true)],
    ...attachedTags.length === 0 ? [] : [OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:56533",[{ key: "entity-preview-tags", spacing: 6, runSpacing: 6 },attachedTags.map((tag) => pill("entity-preview-tag-" + tag.id, tag.name))],(...__operitArgs)=>ctx.UI.FlowRow(...__operitArgs))]
  ]],(...__operitArgs)=>ctx.UI.Column(...__operitArgs));
  const contentSection = (key, title, value, quote = false) => value.trim() === "" ? [] : [
    OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:56785",[{ key: "entity-preview-" + key, fillMaxWidth: true, paddingBottom: 22 },OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:56882",[{
        fillMaxWidth: true,
        spacing: 10,
        ...quote ? { padding: 18, backgroundColor: colors.primaryContainer.copy({ alpha: 0.22 }), backgroundShape: { cornerRadius: 16 } } : {}
      },[
        OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:57109",[{ fillMaxWidth: true, spacing: 8, verticalAlignment: "center" },[
          ...quote ? [OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:57209",[{ name: "FormatQuote", size: 20, tint: colors.primary }],(...__operitArgs)=>ctx.UI.Icon(...__operitArgs))] : [],
          OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:57295",[{ text: title, style: "titleSmall", fontWeight: "600", color: quote ? colors.primary : colors.onSurfaceVariant, weight: 1 }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs))
        ]],(...__operitArgs)=>ctx.UI.Row(...__operitArgs)),
        selectable("entity-preview-" + key + "-text", value)
      ]],(...__operitArgs)=>ctx.UI.Column(...__operitArgs))],(...__operitArgs)=>ctx.UI.Box(...__operitArgs))
  ];
  const themeLabel = entity.themeConfigId === null ? "\u8DDF\u968F\u5F53\u524D\u4E3B\u9898" : input.themeError !== "" ? "\u4E3B\u9898\u540D\u79F0\u6682\u65F6\u65E0\u6CD5\u8BFB\u53D6" : input.themes === null ? "\u6B63\u5728\u8BFB\u53D6\u4E3B\u9898\u540D\u79F0\u2026" : input.themes.find((theme) => theme.id === entity.themeConfigId)?.label ?? "\u4E3B\u9898\u914D\u7F6E\u5DF2\u4E0D\u53EF\u7528";
  const rows = [];
  const contents = [identity2];
  if (card !== null) {
    contents.push(
      ...contentSection("opening", "\u5F00\u573A\u767D", card.openingStatement, true),
      ...contentSection("setting", "\u89D2\u8272\u8BBE\u5B9A", card.characterSetting),
      ...contentSection("chat", "\u804A\u5929\u9644\u52A0\u5185\u5BB9", card.otherContentChat),
      ...contentSection("voice", "\u8BED\u97F3\u9644\u52A0\u5185\u5BB9", card.otherContentVoice),
      ...contentSection("prompt", "\u9AD8\u7EA7\u81EA\u5B9A\u4E49 Prompt", card.advancedCustomPrompt),
      ...contentSection("notes", "\u5907\u6CE8", card.marks)
    );
    const model = input.snapshot.models.find((model2) => model2.providerId === card.chatProviderId && model2.modelId === card.chatModelId);
    rows.push(
      { key: "model", label: "\u804A\u5929\u6A21\u578B", value: card.chatModelBindingMode === "FIXED_MODEL" ? model === void 0 ? "\u56FA\u5B9A\u6A21\u578B\u5DF2\u4E0D\u53EF\u7528" : model.providerName + " \xB7 " + model.modelId : "\u8DDF\u968F\u5168\u5C40\u6A21\u578B" },
      { key: "theme", label: "\u4E3B\u9898", value: themeLabel },
      { key: "tts", label: "\u8BED\u97F3", value: card.ttsConfigId === null ? "\u8DDF\u968F\u5168\u5C40\u8BED\u97F3\u914D\u7F6E" : input.snapshot.ttsConfigs.find((config) => config.id === card.ttsConfigId)?.name ?? "\u8BED\u97F3\u914D\u7F6E\u5DF2\u4E0D\u53EF\u7528" },
      { key: "memory", label: "\u8BB0\u5FC6\u5E93", value: card.memoryBindingMode === "SHARED" ? input.snapshot.stores.find((store) => store.id === card.sharedMemoryId)?.name ?? "\u5171\u4EAB\u8BB0\u5FC6\u5E93\u5DF2\u4E0D\u53EF\u7528" : "\u89D2\u8272\u72EC\u7ACB\u8BB0\u5FC6\u5E93" }
    );
    if (card.sharedMemoryMounts.length !== 0) rows.push({ key: "mounts", label: "\u5171\u4EAB\u8BB0\u5FC6\u6302\u8F7D", value: card.sharedMemoryMounts.map((mount2) => {
      const name = input.snapshot.stores.find((store) => store.id === mount2.sharedMemoryId)?.name ?? "\u8BB0\u5FC6\u5E93\u5DF2\u4E0D\u53EF\u7528";
      const permissions = [mount2.readable ? "\u53EF\u8BFB" : "", mount2.writable ? "\u53EF\u5199" : ""].filter(Boolean).join("\u3001") || "\u65E0\u8BBF\u95EE\u6743\u9650";
      return name + "\uFF08" + permissions + "\uFF09";
    }).join("\n") });
    const policy = card.toolAccessConfig;
    const toolNames = [
      { ids: policy.allowedBuiltinTools, catalog: input.snapshot.toolCatalog.builtinTools },
      { ids: policy.allowedPackages, catalog: input.snapshot.toolCatalog.packages },
      { ids: policy.allowedSkills, catalog: input.snapshot.toolCatalog.skills },
      { ids: policy.allowedMcpServers, catalog: input.snapshot.toolCatalog.mcpServers }
    ].flatMap((source) => source.ids.map((id2) => source.catalog.find((item) => item.name === id2)?.displayName ?? "\u6765\u6E90\u5DF2\u4E0D\u53EF\u7528"));
    rows.push({ key: "tools", label: "\u5DE5\u5177\u8BBF\u95EE", value: !policy.enabled ? "\u8DDF\u968F\u5168\u5C40\u5DE5\u5177\u914D\u7F6E" : toolNames.length === 0 ? "\u672A\u5141\u8BB8\u4EFB\u4F55\u5DE5\u5177\u6765\u6E90" : toolNames.join("\u3001") });
  } else {
    const members = [...input.group.members].sort((a, b) => a.orderIndex - b.orderIndex);
    contents.push(OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:61116",[{ key: "entity-preview-members", fillMaxWidth: true, spacing: 10, paddingBottom: 22 },[
      OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:61225",[{ text: "\u7EC4\u5185\u89D2\u8272", style: "titleSmall", fontWeight: "600", color: colors.onSurfaceVariant }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs)),
      ...members.length === 0 ? [OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:61381",[{ text: "\u6682\u672A\u6DFB\u52A0\u89D2\u8272", style: "bodyMedium", color: colors.onSurfaceVariant }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs))] : members.map((member2, index) => {
        const record2 = input.snapshot.cards.find((card2) => card2.id === member2.characterCardId);
        return OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:61648",[{ key: "entity-preview-member-" + member2.characterCardId, fillMaxWidth: true, paddingVertical: 10, spacing: 12, verticalAlignment: "center" },[
          OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:61815",[{ text: String(index + 1).padStart(2, "0"), style: "labelMedium", color: colors.onSurfaceVariant }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs)),
          OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:61938",[{ width: 40, height: 40, ...record2?.avatarUri == null ? { path: input.avatar ?? void 0 } : { uri: record2.avatarUri }, contentScale: "crop", contentDescription: record2?.name ?? "\u89D2\u8272\u5DF2\u79FB\u9664", modifier: ctx.Modifier.clip({ cornerRadius: 12 }) }],(...__operitArgs)=>ctx.UI.Image(...__operitArgs)),
          OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:62228",[{ weight: 1, spacing: 4 },[
            OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:62283",[{ text: record2?.name ?? "\u89D2\u8272\u5DF2\u79FB\u9664", style: "titleSmall", fontWeight: "600", fillMaxWidth: true }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs)),
            ...record2?.description.trim() ? [OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:62463",[{ text: record2.description, style: "bodySmall", color: colors.onSurfaceVariant, fillMaxWidth: true }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs))] : []
          ]],(...__operitArgs)=>ctx.UI.Column(...__operitArgs))
        ]],(...__operitArgs)=>ctx.UI.Row(...__operitArgs));
      })
    ]],(...__operitArgs)=>ctx.UI.Column(...__operitArgs)));
    rows.push({ key: "theme", label: "\u4E3B\u9898", value: themeLabel });
  }
  contents.push(OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:62722",[{ key: "entity-preview-config", fillMaxWidth: true, backgroundColor: colors.surfaceContainerLow, backgroundShape: { cornerRadius: 16 } },[
    OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:62880",[{ key: "entity-preview-config-toggle", fillMaxWidth: true, color: "transparent", shape: { cornerRadius: 16 }, onClick: input.onToggleConfiguration },OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:63058",[{ fillMaxWidth: true, padding: 16, spacing: 10, verticalAlignment: "center" },[
        OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:63158",[{ name: "Tune", size: 20, tint: colors.onSurfaceVariant }],(...__operitArgs)=>ctx.UI.Icon(...__operitArgs)),
        OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:63238",[{ weight: 1, spacing: 3 },[
          OperitComposeGeneration.leaf(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:63291",[],()=>[{ text: "\u914D\u7F6E\u6458\u8981", style: "titleSmall", fontWeight: "600" }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs)),
          OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:63392",[{ text: card === null ? "\u4E3B\u9898\u914D\u7F6E" : "\u6A21\u578B\u3001\u8BED\u97F3\u4E0E\u8BB0\u5FC6", style: "bodySmall", color: colors.onSurfaceVariant }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs))
        ]],(...__operitArgs)=>ctx.UI.Column(...__operitArgs)),
        OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:63583",[{ name: input.configurationExpanded ? "ExpandLess" : "ExpandMore", size: 20, tint: colors.onSurfaceVariant }],(...__operitArgs)=>ctx.UI.Icon(...__operitArgs))
      ]],(...__operitArgs)=>ctx.UI.Row(...__operitArgs))],(...__operitArgs)=>ctx.UI.Surface(...__operitArgs)),
    ...input.configurationExpanded ? [OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:63759",[{ key: "entity-preview-config-details", fillMaxWidth: true, paddingStart: 16, paddingEnd: 16, paddingBottom: 18, spacing: 14 },rows.map((row) => OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:63932",[{ key: "entity-preview-config-" + row.key, fillMaxWidth: true, spacing: 4 },[
        OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:64033",[{ text: row.label, style: "labelMedium", color: colors.onSurfaceVariant }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs)),
        selectable("entity-preview-config-" + row.key + "-value", row.value)
      ]],(...__operitArgs)=>ctx.UI.Column(...__operitArgs)))],(...__operitArgs)=>ctx.UI.Column(...__operitArgs))] : []
  ]],(...__operitArgs)=>ctx.UI.Column(...__operitArgs)));
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
    if (live.current.card === null) throw new Error("\u89D2\u8272\u5361\u5C1A\u672A\u52A0\u8F7D");
    return live.current.card;
  }
  function group() {
    if (live.current.group === null) throw new Error("\u7FA4\u7EC4\u5C1A\u672A\u52A0\u8F7D");
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
        const snapshot2 = yield request({ action: "snapshot" });
        assertOwner();
        const record2 = input.mode === "create" ? input.entity === "card" ? createCharacterDraft() : createGroupDraft() : input.entity === "card" ? snapshot2.cards.find((value) => value.id === input.id) : snapshot2.groups.find((value) => value.id === input.id);
        if (record2 === void 0) throw new Error("\u9884\u89C8\u5B9E\u4F53\u4E0D\u5B58\u5728\uFF1A" + input.id);
        assertOwner();
        update({ snapshot: snapshot2, tags: copy(snapshot2.tags), card: input.entity === "card" ? copy(record2) : null, group: input.entity === "group" ? copy(record2) : null, loading: false });
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
        const snapshot2 = live.current.snapshot;
        if (snapshot2 === null) throw new Error("\u9884\u89C8\u6570\u636E\u5C1A\u672A\u52A0\u8F7D");
        update({
          editing: false,
          tab: 0,
          error: "",
          tags: copy(snapshot2.tags),
          tagChanges: { created: [], updated: [], deleted: [] },
          card: input.entity === "card" ? copy(snapshot2.cards.find((value) => value.id === input.id) ?? card()) : null,
          group: input.entity === "group" ? copy(snapshot2.groups.find((value) => value.id === input.id) ?? group()) : null
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
        let snapshot2;
        if (input.entity === "card") {
          const draft = card();
          if (draft.name.trim() === "") throw new Error("\u89D2\u8272\u540D\u79F0\u4E0D\u80FD\u4E3A\u7A7A");
          if (draft.chatModelBindingMode === "FIXED_MODEL" && !live.current.snapshot?.models.some((model) => model.providerId === draft.chatProviderId && model.modelId === draft.chatModelId)) throw new Error("\u8BF7\u9009\u62E9\u6709\u6548\u7684\u56FA\u5B9A\u804A\u5929\u6A21\u578B");
          if (draft.memoryBindingMode === "SHARED" && !live.current.snapshot?.stores.some((store) => store.id === draft.sharedMemoryId)) throw new Error("\u8BF7\u9009\u62E9\u6709\u6548\u7684\u5171\u4EAB\u8BB0\u5FC6\u5E93");
          snapshot2 = yield request({ action: "saveCharacter", card: copy(draft), create: input.mode === "create", tagChanges: copy(live.current.tagChanges) });
        } else {
          const draft = group();
          if (draft.name.trim() === "") throw new Error("\u7FA4\u7EC4\u540D\u79F0\u4E0D\u80FD\u4E3A\u7A7A");
          snapshot2 = yield request({ action: "saveGroup", group: copy(draft), create: input.mode === "create" });
        }
        const previousSnapshot = live.current.snapshot;
        assertOwner();
        update({
          snapshot: snapshot2,
          editing: false,
          tab: 0,
          tags: copy(snapshot2.tags),
          tagChanges: { created: [], updated: [], deleted: [] },
          card: input.entity === "card" ? copy(snapshot2.cards.find((value) => value.id === input.id) ?? card()) : null,
          group: input.entity === "group" ? copy(snapshot2.groups.find((value) => value.id === input.id) ?? group()) : null
        });
        if (input.mode !== "preview") {
          const before = previousSnapshot;
          const saved = input.mode === "create" ? input.entity === "card" ? snapshot2.cards.find((value) => !before.cards.some((old) => old.id === value.id)) : snapshot2.groups.find((value) => !before.groups.some((old) => old.id === value.id)) : input.entity === "card" ? snapshot2.cards.find((value) => value.id === input.id) : snapshot2.groups.find((value) => value.id === input.id);
          if (saved === void 0) throw new Error("\u4FDD\u5B58\u7ED3\u679C\u7F3A\u5C11\u5B9E\u4F53\u6807\u8BC6");
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
        if (draft === null) throw new Error("\u6CA1\u6709\u6807\u7B7E\u8349\u7A3F");
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
          if (draft.name.trim() === "") throw new Error("\u6807\u7B7E\u540D\u79F0\u4E0D\u80FD\u4E3A\u7A7A");
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
        const selected = yield ctx.openFilePicker({ picker: "image", allowMultiple: false, mimeTypes: ["image/*"] });
        assertOwner();
        if (selected.cancelled) return;
        if (selected.files.length !== 1) throw new Error("\u8BF7\u9009\u62E9\u4E00\u4E2A\u5934\u50CF\u6587\u4EF6");
        const platform = (yield Tools.System.terminal.info()).platform, contentBase64 = yield readAvatarImport(pickedImagePath(selected.files[0].path, platform));
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
  const text3 = (value) => OperitComposeGeneration.leaf(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:77947",[value],()=>[{ text: value, fillMaxWidth: true, style: "bodyMedium" }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs));
  const caption = (value) => OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:78047",[{ text: value, fillMaxWidth: true, style: "bodySmall", color: colors.onSurfaceVariant }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs));
  const section = (title, value) => OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:78185",[{ key: "preview-" + title, fillMaxWidth: true, paddingBottom: 12, spacing: 6 },[
    OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:78285",[{ text: title, style: "labelLarge", color: colors.onSurfaceVariant }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs)),
    OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:78372",[{},text3(value)],(...__operitArgs)=>ctx.UI.SelectionContainer(...__operitArgs))
  ]],(...__operitArgs)=>ctx.UI.Column(...__operitArgs));
  const field2 = (key, label, value, apply, maxLines = 1) => OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:78483",[{ fillMaxWidth: true, paddingBottom: 12 },OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:78537",[{
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
    ...maxLines === 1 ? {} : { suffixIconWidth: 36, suffixIconHeight: 36, trailingIcon: OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:78846",[{ width: 36, height: 36, contentAlignment: "center" },OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:78912",[{
      key: "entity-expand-" + key,
      width: 28,
      height: 28,
      contentPadding: { horizontal: 0, vertical: 0 },
      enabled,
      onClick: () => {
        assertOwner();
        update({ expanded: { key, label, value, apply }, fullscreen: true });
      }
    },OperitComposeGeneration.leaf(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:79210",[],()=>[{ name: "Fullscreen", size: 18 }],(...__operitArgs)=>ctx.UI.Icon(...__operitArgs))],(...__operitArgs)=>ctx.UI.IconButton(...__operitArgs))],(...__operitArgs)=>ctx.UI.Box(...__operitArgs)) }
  }],(...__operitArgs)=>ctx.UI.TextField(...__operitArgs))],(...__operitArgs)=>ctx.UI.Box(...__operitArgs));
  const check = (key, label, checked, apply) => OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:79315",[{ key, fillMaxWidth: true, verticalAlignment: "center", spacing: 8 },[OperitComposeGeneration.leaf(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:79397",[checked,enabled,apply],()=>[{ checked, enabled, onCheckedChange: apply }],(...__operitArgs)=>ctx.UI.Checkbox(...__operitArgs)), OperitComposeGeneration.leaf(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:79460",[label],()=>[{ text: label, weight: 1 }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs))]],(...__operitArgs)=>ctx.UI.Row(...__operitArgs));
  const configure = (key, label, value, apply) => OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:79553",[{ fillMaxWidth: true, paddingBottom: 12 },OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:79616",[{ key, fillMaxWidth: true, color: "transparent", onClick: enabled ? apply : void 0 },OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:79730",[{ label, enabled },OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:79772",[{ fillMaxWidth: true, verticalAlignment: "center", spacing: 12 },[OperitComposeGeneration.leaf(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:79850",[value],()=>[{ text: value, weight: 1, maxLines: 1, overflow: "ellipsis", style: "bodyMedium" }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs)), OperitComposeGeneration.leaf(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:79947",[],()=>[{ name: "Tune", size: 24 }],(...__operitArgs)=>ctx.UI.Icon(...__operitArgs))]],(...__operitArgs)=>ctx.UI.Row(...__operitArgs))],(...__operitArgs)=>ctx.UI.InputDecorator(...__operitArgs))],(...__operitArgs)=>ctx.UI.Surface(...__operitArgs))],(...__operitArgs)=>ctx.UI.Box(...__operitArgs));
  const select = (key, label, value, choices, apply, empty2 = "\u4E0D\u7ED1\u5B9A") => configure(key, label, choices.find((option) => option.id === value)?.label ?? (value === null ? empty2 : "\u5DF2\u7ED1\u5B9A\u914D\u7F6E\u4E0D\u5B58\u5728\uFF1A" + value), () => {
    assertOwner();
    update({ picker: { title: "\u9009\u62E9" + label, key, value, choices, apply, query: "" } });
  });
  const binding = (key, title, subtitleOff, subtitleOn, checked, apply, children, switchEnabled = true) => OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:80498",[{ key, fillMaxWidth: true, paddingBottom: 7 },OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:80556",[{ fillMaxWidth: true, modifier: ctx.Modifier.border(1, colors.outlineVariant.copy({ alpha: 0.24 }), { cornerRadius: 12 }), paddingHorizontal: 10, paddingVertical: 6 },OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:80744",[{ fillMaxWidth: true },[
      OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:80790",[{ fillMaxWidth: true, modifier: ctx.Modifier.heightIn({ minHeight: 46 }), verticalAlignment: "center", spacing: 8 },[
        OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:80928",[{ weight: 1, spacing: 1 },[OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:80970",[{ text: title, fontSize: 14, fontWeight: "700", color: enabled && switchEnabled ? colors.onSurface : colors.onSurface.copy({ alpha: 0.46 }) }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs)), OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:81126",[{ text: checked ? subtitleOn : subtitleOff, fontSize: 12, color: enabled && switchEnabled ? colors.onSurfaceVariant : colors.onSurfaceVariant.copy({ alpha: 0.46 }) }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs))]],(...__operitArgs)=>ctx.UI.Column(...__operitArgs)),
        OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:81316",[{ key: key + "-switch", checked, enabled: enabled && switchEnabled, modifier: ctx.Modifier.scale(0.82), onCheckedChange: apply }],(...__operitArgs)=>ctx.UI.Switch(...__operitArgs))
      ]],(...__operitArgs)=>ctx.UI.Row(...__operitArgs)),
      ...checked || !switchEnabled ? [OperitComposeGeneration.leaf(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:81508",[],()=>[{ height: 6 }],(...__operitArgs)=>ctx.UI.Spacer(...__operitArgs)), ...children] : []
    ]],(...__operitArgs)=>ctx.UI.Column(...__operitArgs))],(...__operitArgs)=>ctx.UI.Box(...__operitArgs))],(...__operitArgs)=>ctx.UI.Box(...__operitArgs));
  const body = (key, children, padding = { paddingStart: 18, paddingTop: 10, paddingEnd: 18, paddingBottom: 10 }) => OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:81686",[{ key, fillMaxWidth: true },children.map((child, index) => OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:81764",[{
    key: key + "-item-" + String(child.props?.key ?? index),
    fillMaxWidth: true,
    paddingStart: padding.paddingStart,
    paddingEnd: padding.paddingEnd,
    paddingTop: index === 0 ? padding.paddingTop : 0,
    paddingBottom: index === children.length - 1 ? padding.paddingBottom : 0
  },child],(...__operitArgs)=>ctx.UI.Box(...__operitArgs)))],(...__operitArgs)=>ctx.UI.LazyColumn(...__operitArgs));
  const tabs = (key, labels, selected, apply) => OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:82133",[{
    key,
    fillMaxWidth: true,
    selectedTabIndex: selected,
    tabs: labels.map((label, tab) => OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:82258",[{ key: key + "-" + tab, selected: selected === tab, enabled, onClick: () => {
      assertOwner();
      apply(tab);
    } },OperitComposeGeneration.leaf(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:82395",[label],()=>[{ text: label, style: "titleSmall" }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs))],(...__operitArgs)=>ctx.UI.Tab(...__operitArgs)))
  }],(...__operitArgs)=>ctx.UI.PrimaryTabRow(...__operitArgs));
  const standardButton = (key, label, action, filled = false) => filled ? OperitComposeGeneration.leaf(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:82527",[key,label,enabled,action],()=>[{ key, text: label, enabled, onClick: action }],(...__operitArgs)=>ctx.UI.Button(...__operitArgs)) : OperitComposeGeneration.leaf(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:82591",[key,label,enabled,action],()=>[{ key, text: label, enabled, onClick: action }],(...__operitArgs)=>ctx.UI.TextButton(...__operitArgs));
  function shell(title, child, actions, width = 760, height = 820, plainBody = false, reading = false) {
    return OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:82774",[{ key: "native-entity-panel", ...state.fullscreen ? { fillMaxSize: true } : { width, height }, modifier: ctx.Modifier.clip({ cornerRadius: state.fullscreen ? 0 : 28 }) },[
      OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:82967",[{ key: "entity-header", fillMaxWidth: true, paddingStart: 24, paddingTop: reading ? 18 : 22, paddingEnd: 16, paddingBottom: reading ? 8 : 12, verticalAlignment: "center" },[
        OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:83161",[{ text: title, style: reading ? "labelLarge" : "headlineSmall", fontWeight: reading ? "600" : "700", ...reading ? { color: colors.onSurfaceVariant, fontSize: 14 } : {}, maxLines: 2, overflow: "ellipsis", weight: 1 }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs)),
        OperitComposeGeneration.leaf(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:83399",[enabled,close],()=>[{ key: "entity-close", icon: "Close", enabled, onClick: close }],(...__operitArgs)=>ctx.UI.IconButton(...__operitArgs))
      ]],(...__operitArgs)=>ctx.UI.Row(...__operitArgs)),
      ...state.busy ? [OperitComposeGeneration.leaf(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:83515",[],()=>[{ fillMaxWidth: true }],(...__operitArgs)=>ctx.UI.LinearProgressIndicator(...__operitArgs))] : [],
      ...state.error === "" ? [] : [OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:83613",[{ key: "entity-error", text: state.error, color: colors.error, fillMaxWidth: true, paddingHorizontal: 24, paddingBottom: 12 }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs))],
      OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:83760",[{ key: "entity-content", weight: 1, fillMaxWidth: true, ...plainBody ? {} : { paddingStart: 24, paddingTop: 16, paddingEnd: 24, paddingBottom: 20 } },child],(...__operitArgs)=>ctx.UI.Box(...__operitArgs)),
      OperitComposeGeneration.leaf(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:83936",[actions],()=>[{ key: "entity-footer", fillMaxWidth: true, paddingStart: 24, paddingTop: 0, paddingEnd: 24, paddingBottom: 20, horizontalArrangement: "end", spacing: 8, runSpacing: 8 },actions],(...__operitArgs)=>ctx.UI.FlowRow(...__operitArgs))
    ]],(...__operitArgs)=>ctx.UI.Column(...__operitArgs));
  }
  let panel;
  const cancel = standardButton("entity-cancel", "\u53D6\u6D88", close);
  if (state.expanded !== null) {
    const expanded = state.expanded;
    panel = shell(expanded.label, OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:84333",[{ key: "entity-expanded-input", label: expanded.label, value: expanded.value, fillMaxSize: true, minLines: 16, maxLines: 40, enabled, onValueChange: (value) => {
      assertOwner();
      if (live.current.expanded !== null) update({ expanded: { ...live.current.expanded, value } });
    } }],(...__operitArgs)=>ctx.UI.TextField(...__operitArgs)), [
      cancel,
      standardButton("entity-primary", "\u4FDD\u5B58", () => {
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
    panel = shell(picker.title, OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:85452",[{ fillMaxSize: true },[
      field2("picker-query", "\u641C\u7D22", picker.query, (query) => {
        assertOwner();
        if (live.current.picker !== null) update({ picker: { ...live.current.picker, query } });
      }),
      ...themePending ? [state.themeError === "" ? caption("\u6B63\u5728\u8BFB\u53D6\u4E3B\u9898\u914D\u7F6E\u2026") : OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:85812",[{ fillMaxWidth: true },[OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:85851",[{ text: state.themeError, color: colors.error }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs)), standardButton("entity-picker-retry", "\u91CD\u65B0\u52A0\u8F7D\u4E3B\u9898", loadThemes)]],(...__operitArgs)=>ctx.UI.Column(...__operitArgs))] : [],
      OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:86018",[{ key: "entity-picker-list", weight: 1, fillMaxWidth: true },[
        OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:86108",[{ key: "entity-picker-clear", fillMaxWidth: true, paddingHorizontal: 8, paddingVertical: 9, verticalAlignment: "center", spacing: 10, onClick: () => choose(null) },[OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:86285",[{ name: picker.value === null ? "CheckCircle" : "CircleOutlined", size: 20, tint: picker.value === null ? colors.primary : colors.onSurfaceVariant }],(...__operitArgs)=>ctx.UI.Icon(...__operitArgs)), text3("\u8DDF\u968F\u5168\u5C40 / \u4E0D\u7ED1\u5B9A")]],(...__operitArgs)=>ctx.UI.Row(...__operitArgs)),
        ...choices.map((option) => OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:86541",[{ key: "entity-picker-" + option.id, fillMaxWidth: true, paddingHorizontal: 8, paddingVertical: 9, verticalAlignment: "center", spacing: 10, onClick: () => choose(option.id) },[OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:86730",[{ name: picker.value === option.id ? "CheckCircle" : "CircleOutlined", size: 20, tint: picker.value === option.id ? colors.primary : colors.onSurfaceVariant }],(...__operitArgs)=>ctx.UI.Icon(...__operitArgs)), OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:86903",[{ weight: 1 },[text3(option.label), ...option.subtitle === void 0 ? [] : [caption(option.subtitle)]]],(...__operitArgs)=>ctx.UI.Column(...__operitArgs))]],(...__operitArgs)=>ctx.UI.Row(...__operitArgs)))
      ]],(...__operitArgs)=>ctx.UI.LazyColumn(...__operitArgs))
    ]],(...__operitArgs)=>ctx.UI.Column(...__operitArgs)), [cancel], 580, 560);
  } else if (state.toolDraft !== null && state.snapshot !== null) {
    const draft = state.toolDraft, catalog = state.snapshot.toolCatalog;
    const names = ["\u5185\u7F6E\u5DE5\u5177", "\u63D2\u4EF6", "\u6280\u80FD", "MCP"], keys = ["allowedBuiltinTools", "allowedPackages", "allowedSkills", "allowedMcpServers"];
    const key = keys[state.toolTab], options = [catalog.builtinTools, catalog.packages, catalog.skills, catalog.mcpServers][state.toolTab];
    const children = options.filter((option) => (option.displayName + " " + option.description).toLowerCase().includes(state.toolQuery.toLowerCase())).map((option) => check("entity-tool-" + key + "-" + option.name, option.displayName, draft[key].includes(option.name), (checked) => {
      assertOwner();
      const policy = live.current.toolDraft;
      if (policy !== null) update({ toolDraft: { ...policy, [key]: checked ? [...policy[key], option.name] : policy[key].filter((name) => name !== option.name) } });
    }));
    panel = shell("\u914D\u7F6E\u5141\u8BB8\u4F7F\u7528\u7684\u5DE5\u5177", OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:88119",[{ fillMaxSize: true },[
      tabs("entity-tool-tabs", names, state.toolTab, (toolTab) => update({ toolTab })),
      OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:88252",[{ fillMaxWidth: true, paddingHorizontal: 18, paddingTop: 10 },field2("tool-query", "\u641C\u7D22", state.toolQuery, (toolQuery) => update({ toolQuery }))],(...__operitArgs)=>ctx.UI.Box(...__operitArgs)),
      OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:88426",[{ weight: 1, fillMaxWidth: true },body("entity-tool-list", children)],(...__operitArgs)=>ctx.UI.Box(...__operitArgs))
    ]],(...__operitArgs)=>ctx.UI.Column(...__operitArgs)), [cancel, standardButton("entity-primary", "\u786E\u5B9A", () => {
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
    panel = shell("\u5BFC\u51FA", state.exportContent === "" ? OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:89883",[{ fillMaxWidth: true, spacing: 8 },[standardButton("entity-export-json", "\u5BFC\u51FA JSON", () => exportFile("operit")), ...input.entity === "card" ? [standardButton("entity-export-tavern", "\u5BFC\u51FA\u9152\u9986 JSON", () => exportFile("tavern"))] : []]],(...__operitArgs)=>ctx.UI.Column(...__operitArgs)) : body("entity-export-body", [caption("\u70B9\u51FB\u4FDD\u5B58\u6587\u4EF6\u9009\u62E9\u4FDD\u5B58\u4F4D\u7F6E\u3002"), section("JSON", state.exportContent)]), [cancel, ...state.exportContent === "" ? [] : [standardButton("entity-export-save", "\u4FDD\u5B58\u6587\u4EF6", writeExport, true)]], state.exportContent === "" ? 420 : 760, state.exportContent === "" ? 240 : 620);
  } else if (state.tagDraft !== null) {
    const draft = state.tagDraft;
    const children = state.tagDeleting ? [text3(`\u786E\u5B9A\u5220\u9664\u6807\u7B7E\u201C${draft.name}\u201D\uFF1F`)] : [["name", "\u6807\u7B7E\u540D\u79F0", 1], ["description", "\u63CF\u8FF0", 1], ["promptContent", "\u63D0\u793A\u8BCD\u5185\u5BB9", 6]].map(([key, label, lines]) => field2("tag-" + key, label, draft[key], (value) => {
      assertOwner();
      if (live.current.tagDraft !== null) update({ tagDraft: { ...live.current.tagDraft, [key]: value } });
    }, lines));
    panel = shell(state.tagDeleting ? "\u5220\u9664\u6807\u7B7E" : draft.id === "" ? "\u521B\u5EFA\u6807\u7B7E" : "\u7F16\u8F91\u6807\u7B7E", body("entity-tag-body", children, { paddingStart: 0, paddingTop: 0, paddingEnd: 0, paddingBottom: 0 }), [
      ...draft.id === "" || state.tagDeleting ? [] : [standardButton("entity-tag-delete", "\u5220\u9664", () => {
        assertOwner();
        update({ tagDeleting: true });
      })],
      cancel,
      standardButton("entity-primary", state.tagDeleting ? "\u5220\u9664" : "\u4FDD\u5B58", stageTag, true)
    ], 620, state.tagDeleting ? 240 : 520);
  } else if (state.tagManaging) {
    panel = shell("\u7BA1\u7406\u6807\u7B7E", body("entity-tag-manager", state.tags.map((tag) => OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:91824",[{ key: "entity-tag-row-" + tag.id, fillMaxWidth: true, verticalAlignment: "center" },[
      OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:91929",[{ weight: 1 },check("entity-tag-" + tag.id, tag.name, state.card?.attachedTagIds.includes(tag.id) ?? false, (checked) => changeCard({ attachedTagIds: checked ? [...card().attachedTagIds, tag.id] : card().attachedTagIds.filter((id2) => id2 !== tag.id) }))],(...__operitArgs)=>ctx.UI.Box(...__operitArgs)),
      standardButton("entity-tag-edit-" + tag.id, "\u7F16\u8F91", () => {
        assertOwner();
        update({ tagDraft: copy(tag) });
      }),
      standardButton("entity-tag-remove-" + tag.id, "\u5220\u9664", () => {
        assertOwner();
        update({ tagDraft: copy(tag), tagDeleting: true });
      })
    ]],(...__operitArgs)=>ctx.UI.Row(...__operitArgs))), { paddingStart: 0, paddingTop: 0, paddingEnd: 0, paddingBottom: 0 }), [standardButton("entity-tag-create", "\u521B\u5EFA\u6807\u7B7E", () => {
      assertOwner();
      update({ tagDraft: { id: "", name: "", description: "", promptContent: "", tagType: "CUSTOM" } });
    }), standardButton("entity-cancel", "\u5B8C\u6210", close)], 620, 560);
  } else if (state.deleting) {
    panel = shell("\u786E\u8BA4\u5220\u9664", text3(`\u786E\u5B9A\u5220\u9664\u201C${state.card?.name ?? state.group?.name}\u201D\uFF1F\u6B64\u64CD\u4F5C\u4E0D\u53EF\u64A4\u9500\u3002`), [cancel, standardButton("entity-primary", "\u5220\u9664", remove, true)], 560, 240);
  } else {
    const pages = [[], [], []];
    if (state.loading) pages[0].push(OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:93251",[{ fillMaxSize: true, contentAlignment: "center" },OperitComposeGeneration.leaf(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:93313",[],()=>[{ size: 32 }],(...__operitArgs)=>ctx.UI.LoadingIndicator(...__operitArgs))],(...__operitArgs)=>ctx.UI.Box(...__operitArgs)));
    else if (state.snapshot === null) pages[0].push(standardButton("entity-retry", "\u91CD\u65B0\u52A0\u8F7D", load));
    else if (state.card !== null) {
      const draft = state.card, snapshot2 = state.snapshot;
      if (!state.editing) {
        pages[0].push(...renderEntityPreview(ctx, {
          card: draft,
          group: null,
          snapshot: snapshot2,
          tags: state.tags,
          avatar: state.avatar,
          themes: state.themes,
          themeError: state.themeError,
          configurationExpanded: state.previewConfigurationExpanded,
          onToggleConfiguration: togglePreviewConfiguration
        }));
      } else {
        pages[0].push(
          field2("name", "\u89D2\u8272\u540D\u79F0", draft.name, (name) => changeCard({ name })),
          field2("description", "\u63CF\u8FF0", draft.description, (description) => changeCard({ description })),
          OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:94260",[{ fillMaxWidth: true, paddingBottom: 12 },OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:94314",[{ label: "\u89D2\u8272\u5934\u50CF", enabled },OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:94384",[{ fillMaxWidth: true, verticalAlignment: "center" },[
            OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:94462",[{ key: "entity-edit-avatar", ...draft.avatarUri === null ? { path: state.avatar ?? void 0 } : { uri: draft.avatarUri }, width: 44, height: 44, contentScale: "crop", contentDescription: "\u89D2\u8272\u5934\u50CF", modifier: ctx.Modifier.clip({ type: "circle" }) }],(...__operitArgs)=>ctx.UI.Image(...__operitArgs)),
            OperitComposeGeneration.leaf(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:94752",[],()=>[{ width: 12 }],(...__operitArgs)=>ctx.UI.Spacer(...__operitArgs)),
            OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:94794",[{ text: draft.avatarUri ?? "\u672A\u8BBE\u7F6E", style: "bodyMedium", weight: 1, maxLines: 1, overflow: "ellipsis" }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs)),
            OperitComposeGeneration.leaf(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:94937",[],()=>[{ width: 8 }],(...__operitArgs)=>ctx.UI.Spacer(...__operitArgs)),
            standardButton("entity-choose-avatar", "\u9009\u62E9", chooseAvatar),
            ...draft.avatarUri === null ? [] : [standardButton("entity-clear-avatar", "\u6E05\u9664", () => __async(null, null, function* () {
              yield act(() => __async(null, null, function* () {
                assertOwner();
                update({ card: { ...card(), avatarUri: null } });
                if (live.current.avatar === null) void loadDefaultAvatar();
              }));
            }))]
          ]],(...__operitArgs)=>ctx.UI.Row(...__operitArgs))],(...__operitArgs)=>ctx.UI.InputDecorator(...__operitArgs))],(...__operitArgs)=>ctx.UI.Box(...__operitArgs)),
          field2("characterSetting", "\u89D2\u8272\u8BBE\u5B9A", draft.characterSetting, (characterSetting) => changeCard({ characterSetting }), 6),
          field2("openingStatement", "\u5F00\u573A\u767D", draft.openingStatement, (openingStatement) => changeCard({ openingStatement }), 3),
          OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:95783",[{ fillMaxWidth: true, paddingBottom: 12 },[
            OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:95854",[{ fillMaxWidth: true, verticalAlignment: "center" },[OperitComposeGeneration.leaf(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:95919",[],()=>[{ text: "\u6807\u7B7E", weight: 1, fontWeight: "700" }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs)), OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:95988",[{ key: "entity-tags-manage", enabled, onClick: () => {
              assertOwner();
              update({ tagManaging: true });
            } },OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:96152",[{ spacing: 8, verticalAlignment: "center" },[OperitComposeGeneration.leaf(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:96209",[],()=>[{ name: "Tune", size: 18 }],(...__operitArgs)=>ctx.UI.Icon(...__operitArgs)), OperitComposeGeneration.leaf(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:96250",[],()=>[{ text: "\u7BA1\u7406\u6807\u7B7E" }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs))]],(...__operitArgs)=>ctx.UI.Row(...__operitArgs))],(...__operitArgs)=>ctx.UI.TextButton(...__operitArgs))]],(...__operitArgs)=>ctx.UI.Row(...__operitArgs)),
            OperitComposeGeneration.leaf(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:96318",[],()=>[{ height: 6 }],(...__operitArgs)=>ctx.UI.Spacer(...__operitArgs)),
            state.tags.length === 0 ? caption("\u6682\u65E0\u63D0\u793A\u8BCD\u6807\u7B7E") : OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:96442",[{ spacing: 8, runSpacing: 8 },state.tags.map((tag) => OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:96512",[{ key: "entity-tag-" + tag.id, selected: draft.attachedTagIds.includes(tag.id), enabled, label: OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:96626",[{ text: tag.name, maxLines: 1, overflow: "ellipsis", modifier: ctx.Modifier.widthIn({ maxWidth: 160 }) }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs)), onClick: () => {
              assertOwner();
              changeCard({ attachedTagIds: card().attachedTagIds.includes(tag.id) ? card().attachedTagIds.filter((id2) => id2 !== tag.id) : [...card().attachedTagIds, tag.id] });
            } }],(...__operitArgs)=>ctx.UI.FilterChip(...__operitArgs)))],(...__operitArgs)=>ctx.UI.FlowRow(...__operitArgs))
          ]],(...__operitArgs)=>ctx.UI.Column(...__operitArgs))
        );
        for (const [key, label, lines] of [["otherContentChat", "\u804A\u5929\u9644\u52A0\u5185\u5BB9", 4], ["otherContentVoice", "\u8BED\u97F3\u9644\u52A0\u5185\u5BB9", 4], ["advancedCustomPrompt", "\u9AD8\u7EA7\u81EA\u5B9A\u4E49 Prompt", 4], ["marks", "\u5907\u6CE8", 3]]) pages[1].push(field2(key, label, draft[key], (value) => changeCard({ [key]: value }), lines));
        const tts = snapshot2.ttsConfigs.find((config) => config.id === draft.ttsConfigId);
        const policy = draft.toolAccessConfig, count = policy.allowedBuiltinTools.length + policy.allowedPackages.length + policy.allowedSkills.length + policy.allowedMcpServers.length;
        pages[2].push(
          binding("entity-model", "\u804A\u5929\u6A21\u578B", "\u8DDF\u968F\u5168\u5C40\u6A21\u578B\u914D\u7F6E", "\u4F7F\u7528\u56FA\u5B9A\u6A21\u578B\u914D\u7F6E", draft.chatModelBindingMode === "FIXED_MODEL", (checked) => changeCard({ chatModelBindingMode: checked ? "FIXED_MODEL" : "FOLLOW_GLOBAL", chatModelId: checked ? card().chatModelId : null, chatProviderId: checked ? card().chatProviderId : null }), [select("entity-model-picker", "\u804A\u5929\u6A21\u578B\u914D\u7F6E", draft.chatProviderId === null || draft.chatModelId === null ? null : JSON.stringify([draft.chatProviderId, draft.chatModelId]), snapshot2.models.map((value) => ({ id: JSON.stringify([value.providerId, value.modelId]), label: value.modelId, subtitle: value.providerName })), (selected) => {
            const model = snapshot2.models.find((value) => JSON.stringify([value.providerId, value.modelId]) === selected);
            if (model === void 0) throw new Error("\u8BF7\u9009\u62E9\u6709\u6548\u7684\u63D0\u4F9B\u5546\u4E0E\u6A21\u578B");
            changeCard({ chatProviderId: model.providerId, chatModelId: model.modelId });
          }, "\u8BF7\u9009\u62E9\u804A\u5929\u6A21\u578B")]),
          binding("entity-theme", "\u4E3B\u9898\u914D\u7F6E", "\u4E0D\u6539\u53D8\u5F53\u524D\u72EC\u7ACB\u4E3B\u9898", "\u4F7F\u7528\u89D2\u8272\u5361\u4E3B\u9898\u914D\u7F6E", draft.themeConfigId !== null, (checked) => {
            if (!checked) changeCard({ themeConfigId: null });
            else update({ picker: { title: "\u9009\u62E9\u4E3B\u9898\u914D\u7F6E", key: "theme", value: draft.themeConfigId, choices: state.themes ?? [], apply: (themeConfigId) => changeCard({ themeConfigId }), query: "" } });
          }, [select("entity-theme-picker", "\u4E3B\u9898\u914D\u7F6E", draft.themeConfigId, state.themes ?? [], (themeConfigId) => changeCard({ themeConfigId }))]),
          binding("entity-tts", "TTS \u914D\u7F6E", "\u8DDF\u968F\u5168\u5C40 TTS \u914D\u7F6E", "\u4F7F\u7528\u89D2\u8272\u5361 TTS \u914D\u7F6E", draft.ttsConfigId !== null, (checked) => {
            if (!checked) changeCard({ ttsConfigId: null });
            else update({ picker: { title: "\u9009\u62E9 TTS \u914D\u7F6E", key: "tts", value: draft.ttsConfigId, choices: snapshot2.ttsConfigs.map((config) => ({ id: config.id, label: config.name, subtitle: config.providerType + " \xB7 " + config.voice })), apply: (ttsConfigId) => changeCard({ ttsConfigId }), query: "" } });
          }, [select("entity-tts-picker", "TTS \u914D\u7F6E", draft.ttsConfigId, snapshot2.ttsConfigs.map((config) => ({ id: config.id, label: config.name, subtitle: config.providerType + " \xB7 " + config.voice })), (ttsConfigId) => changeCard({ ttsConfigId }), tts?.name ?? "\u8BF7\u9009\u62E9 TTS \u914D\u7F6E")]),
          binding("entity-memory", "\u8BB0\u5FC6\u7ED1\u5B9A", "\u4F7F\u7528\u89D2\u8272\u8BB0\u5FC6", "\u4F7F\u7528\u5171\u4EAB\u8BB0\u5FC6", draft.memoryBindingMode === "SHARED", (checked) => changeCard({ memoryBindingMode: checked ? "SHARED" : "CHARACTER", sharedMemoryId: checked ? card().sharedMemoryId : null }), [snapshot2.stores.length === 0 ? caption("\u8FD8\u6CA1\u6709\u5171\u4EAB\u8BB0\u5FC6\u5E93") : select("entity-store-picker", "\u5171\u4EAB\u8BB0\u5FC6\u5E93", draft.sharedMemoryId, snapshot2.stores.map((store) => ({ id: store.id, label: store.name })), (sharedMemoryId) => changeCard({ sharedMemoryId }), "\u8BF7\u9009\u62E9\u5171\u4EAB\u8BB0\u5FC6\u5E93")], snapshot2.stores.length !== 0),
          binding("entity-tool", "\u5DE5\u5177\u8BBF\u95EE", "\u8DDF\u968F\u5168\u5C40\u5DE5\u5177\u914D\u7F6E", "\u81EA\u5B9A\u4E49\u5141\u8BB8\u4F7F\u7528\u7684\u5DE5\u5177", policy.enabled, (checked) => changeCard({ toolAccessConfig: { ...card().toolAccessConfig, enabled: checked } }), [configure("entity-tool-configure", "\u914D\u7F6E\u5141\u8BB8\u4F7F\u7528\u7684\u5DE5\u5177", "\u5DF2\u914D\u7F6E " + count + " \u4E2A\u5DE5\u5177\u6765\u6E90", () => {
            assertOwner();
            update({ toolDraft: copy(card().toolAccessConfig), toolTab: 0, toolQuery: "" });
          })])
        );
        if (draft.sharedMemoryMounts.length !== 0) pages[2].push(section("\u5171\u4EAB\u8BB0\u5FC6\u6302\u8F7D", JSON.stringify(draft.sharedMemoryMounts, null, 2)));
        if (state.themeError !== "") pages[2].push(OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:101969",[{ text: state.themeError, color: colors.error }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs)), standardButton("entity-theme-retry", "\u91CD\u65B0\u52A0\u8F7D\u4E3B\u9898", () => __async(null, null, function* () {
          yield act(loadThemes);
        })));
      }
    } else if (state.group !== null && state.snapshot !== null) {
      const draft = state.group, snapshot2 = state.snapshot;
      if (!state.editing) pages[0].push(...renderEntityPreview(ctx, {
        card: null,
        group: draft,
        snapshot: snapshot2,
        tags: state.tags,
        avatar: state.avatar,
        themes: state.themes,
        themeError: state.themeError,
        configurationExpanded: state.previewConfigurationExpanded,
        onToggleConfiguration: togglePreviewConfiguration
      }));
      else {
        pages[0].push(field2("name", "\u7FA4\u7EC4\u540D\u79F0", draft.name, (name) => changeGroup({ name })), field2("description", "\u63CF\u8FF0", draft.description, (description) => changeGroup({ description })), select("entity-theme-picker", "\u4E3B\u9898\u914D\u7F6E", draft.themeConfigId, state.themes ?? [], (themeConfigId) => changeGroup({ themeConfigId })), OperitComposeGeneration.leaf(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:103112",[],()=>[{ text: "\u7EC4\u5185\u89D2\u8272", style: "titleSmall", fontWeight: "700", paddingTop: 8, paddingBottom: 8 }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs)));
        for (const option of snapshot2.cards) {
          const selected = draft.members.some((member2) => member2.characterCardId === option.id);
          pages[0].push(OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:103408",[{ fillMaxWidth: true, paddingBottom: 6 },OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:103461",[{
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
          },OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:104294",[{ fillMaxWidth: true, paddingHorizontal: 12, paddingVertical: 8, spacing: 10, verticalAlignment: "center" },[OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:104415",[{ name: selected ? "CheckCircle" : "CircleOutlined", size: 18, tint: selected ? colors.primary : colors.onSurfaceVariant }],(...__operitArgs)=>ctx.UI.Icon(...__operitArgs)), OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:104552",[{ width: 28, height: 28, ...option.avatarUri === null ? { path: state.avatar ?? void 0 } : { uri: option.avatarUri }, modifier: ctx.Modifier.clip({ type: "circle" }), contentScale: "crop", contentDescription: option.name }],(...__operitArgs)=>ctx.UI.Image(...__operitArgs)), OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:104790",[{ weight: 1 },[OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:104820",[{ text: option.name, style: "bodyMedium", fontWeight: "700", maxLines: 1, overflow: "ellipsis" }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs)), ...option.description.trim() === "" ? [] : [caption(option.description)]]],(...__operitArgs)=>ctx.UI.Column(...__operitArgs))]],(...__operitArgs)=>ctx.UI.Row(...__operitArgs))],(...__operitArgs)=>ctx.UI.Surface(...__operitArgs))],(...__operitArgs)=>ctx.UI.Box(...__operitArgs)));
        }
      }
    }
    const isCard = state.card !== null;
    const reading = !state.editing;
    const page = reading ? body("entity-preview-body", pages[0], { paddingStart: 24, paddingTop: 8, paddingEnd: 24, paddingBottom: 24 }) : isCard ? OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:105260",[{ fillMaxSize: true },[tabs("entity-tab", ["\u57FA\u7840", "\u5185\u5BB9", "\u7ED1\u5B9A"], state.tab, (tab) => update({ tab })), OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:105405",[{ key: "entity-pager", weight: 1, fillMaxWidth: true, currentPage: state.tab, onPageChanged: (tab) => {
      assertOwner();
      if (tab >= 0 && tab < 3) update({ tab });
    } },pages.map((children, index) => body("entity-body-" + index, children))],(...__operitArgs)=>ctx.UI.HorizontalPager(...__operitArgs))]],(...__operitArgs)=>ctx.UI.Column(...__operitArgs)) : body("entity-body", pages[0], { paddingStart: 0, paddingTop: 0, paddingEnd: 0, paddingBottom: 0 });
    const title = state.editing ? input.mode === "create" ? input.entity === "card" ? "\u521B\u5EFA\u89D2\u8272\u5361" : "\u521B\u5EFA\u7FA4\u7EC4" : input.entity === "card" ? "\u7F16\u8F91\u89D2\u8272\u5361" : "\u7F16\u8F91\u7FA4\u7EC4" : input.entity === "card" ? "\u89D2\u8272\u5361" : "\u89D2\u8272\u7FA4\u7EC4";
    panel = shell(title, page, [
      ...state.editing && input.mode !== "create" ? [
        ...state.card?.isDefault ? [] : [standardButton("entity-delete", "\u5220\u9664", () => {
          assertOwner();
          update({ deleting: true });
        })],
        standardButton("entity-export", "\u5BFC\u51FA", () => {
          assertOwner();
          update({ exporting: true });
        })
      ] : [],
      standardButton("entity-cancel", state.editing ? "\u53D6\u6D88" : "\u5173\u95ED", close),
      ...state.snapshot === null || state.finished ? [] : [standardButton("entity-primary", state.editing ? "\u4FDD\u5B58" : "\u7F16\u8F91", state.editing ? save : edit, true)]
    ], reading ? isCard ? 680 : 620 : isCard ? 760 : 620, reading ? 760 : 820, reading || isCard, reading);
  }
  return OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:106909",[{
    key: "character-presentation",
    onLoad: load,
    closeOnDismissRequest: false,
    onDismissRequest: close,
    containerColor: colors.surfaceContainerHigh,
    shape: { cornerRadius: state.fullscreen ? 0 : 28 },
    properties: { fullscreen: state.fullscreen, usePlatformDefaultWidth: false, dismissOnClickOutside: false }
  },panel],(...__operitArgs)=>ctx.UI.Dialog(...__operitArgs));
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
            const snapshot2 = yield ToolPkg.ipc.call("character-memory.request", { action: "snapshot" }, { targetRuntime: "main" });
            ready.current = true;
            return { theme: ctx.Theme.getCurrent(), screen, snapshot: snapshot2 };
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
            if (uri !== null && typeof uri !== "string") throw new Error("\u5934\u50CF\u8DEF\u5F84\u5FC5\u987B\u662F\u5B57\u7B26\u4E32\u6216 null");
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
          /** Saves an export through the same host boundary used for file selection. */
          exportFile: (...args) => __async(null, null, function* () {
            const [name, content] = webArguments(args, 2);
            if (typeof name !== "string" || name.trim() === "") throw new Error("\u5BFC\u51FA\u6587\u4EF6\u540D\u4E0D\u80FD\u4E3A\u7A7A");
            if (typeof content !== "string") throw new Error("\u5BFC\u51FA\u5185\u5BB9\u5FC5\u987B\u662F\u5B57\u7B26\u4E32");
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
  return OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:113670",[{ fillMaxSize: true, onLoad: initialize },[path === "" ? OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:113739",[{ text: error === "" ? `\u6B63\u5728\u52A0\u8F7D${managementView === "memory" ? "\u8BB0\u5FC6" : definition2.title}\u2026` : error }],(...__operitArgs)=>ctx.UI.Text(...__operitArgs)) : OperitComposeGeneration.node(ctx,"com.operit.character_cards:dist/ui/memory/index.ui.js:113887",[{
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
  }],(...__operitArgs)=>ctx.UI.WebView(...__operitArgs)), ...managed === null ? [] : [renderManagedEntityScreen(ctx, managed, finishEntity)]]],(...__operitArgs)=>ctx.UI.Box(...__operitArgs));
}

// src/ui/memory/index.ui.ts
function Screen(ctx) {
  return renderScreen(ctx, definition, null, "memory");
}

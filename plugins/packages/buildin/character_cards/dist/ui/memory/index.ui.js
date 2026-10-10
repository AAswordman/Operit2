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

// src/image-source.ts
var sources = /* @__PURE__ */ new Map();
function imageSource(path) {
  return __async(this, null, function* () {
    if (/^(data:image\/|https?:\/\/)/i.test(path)) return path;
    let pending = sources.get(path);
    if (pending === void 0) {
      pending = read();
      sources.set(path, pending);
      pending.catch(() => sources.delete(path));
    }
    return pending;
    function read() {
      return __async(this, null, function* () {
        const file = yield Tools.Files.readBinary(path);
        if (file.size > 8 * 1024 * 1024) throw new Error("\u5934\u50CF\u56FE\u7247\u4E0D\u80FD\u8D85\u8FC7 8 MB");
        const b = file.contentBase64;
        const mime = b.startsWith("iVBOR") ? "image/png" : b.startsWith("/9j/") ? "image/jpeg" : b.startsWith("R0lGOD") ? "image/gif" : b.startsWith("UklGR") ? "image/webp" : b.startsWith("Qk") ? "image/bmp" : null;
        if (mime === null) throw new Error("\u8BF7\u9009\u62E9 PNG\u3001JPEG\u3001GIF\u3001WebP \u6216 BMP \u56FE\u7247");
        return `data:${mime};base64,${b}`;
      });
    }
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
function text(value, path) {
  if (typeof value !== "string") throw new Error(`${path} must be a string`);
  return value;
}
function identity(value, path) {
  const result = text(value, path);
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
      if (selected !== null && (object.kind === "card" && !("CharacterCard" in selected) || object.kind === "group" && !("CharacterGroup" in selected))) throw new Error("screen.selected does not match screen.kind");
      return { mode: "select", kind: object.kind, selected };
    }
    case "preview":
    case "edit":
      fields(object, ["mode", "entity", "id"], "screen.input");
      return { mode: object.mode, entity: entityKind(object.entity), id: identity(object.id, "screen.id") };
    case "memory-attachment": {
      fields(object, ["mode", "ownerKey", "folderPath"], "screen.input");
      const ownerKey = object.ownerKey === null ? null : identity(object.ownerKey, "screen.ownerKey");
      const folderPath = object.folderPath === null ? null : text(object.folderPath, "screen.folderPath");
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
      if (input.kind === "card" && !("CharacterCard" in result) || input.kind === "group" && !("CharacterGroup" in result)) throw new Error("screen.result.selection does not match screen.kind");
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
      const ownerKey = identity(object.ownerKey, "screen.result.ownerKey"), folderPath = text(object.folderPath, "screen.result.folderPath");
      if (input.ownerKey !== null && ownerKey !== input.ownerKey || input.folderPath !== null && folderPath !== input.folderPath) throw new Error("screen.result does not match the presented attachment scope");
      return { mode: "memory-attachment", ownerKey, folderPath, content: text(object.content, "screen.result.content") };
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
  const parsed = /^(card|group):([^\s]+)$/u.exec(selection2);
  if (parsed === null) throw new Error("Invalid plugin chat selection: " + selection2);
  const kind = parsed[1];
  if (kind !== "card" && kind !== "group") throw new Error("Invalid chat selection kind: " + kind);
  return { kind, id: parsed[2] };
}
function selectionForActive(active) {
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
function text2(value, path) {
  if (typeof value !== "string") throw new Error(`${path} must be a string`);
}
function nonblank(value, path) {
  text2(value, path);
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
var strings = array(text2);
var identifiers = array(nonblank);
var member = shape({ characterCardId: required(nonblank), orderIndex: required(integer) });
var mount = shape({ sharedMemoryId: required(nonblank), readable: required(boolean), writable: required(boolean) });
var toolAccess = shape({ enabled: required(boolean), allowedBuiltinTools: required(strings), allowedPackages: required(strings), allowedSkills: required(strings), allowedMcpServers: required(strings) });
var property = shape({ id: required(propertyIdentifier), key: required(text2), value: required(text2) });
var characterFields = {
  name: optional(nonblank),
  description: optional(text2),
  characterSetting: optional(text2),
  openingStatement: optional(text2),
  otherContentChat: optional(text2),
  otherContentVoice: optional(text2),
  avatarUri: optional(nullable(text2)),
  attachedTagIds: optional(identifiers),
  advancedCustomPrompt: optional(text2),
  marks: optional(text2),
  chatModelBindingMode: optional(enumeration("FOLLOW_GLOBAL", "FIXED_MODEL")),
  chatModelId: optional(nullable(text2)),
  ttsConfigId: optional(nullable(text2)),
  themeConfigId: optional(nullable(text2)),
  memoryBindingMode: optional(enumeration("CHARACTER", "SHARED")),
  sharedMemoryId: optional(nullable(text2)),
  sharedMemoryMounts: optional(array(mount)),
  toolAccessConfig: optional(toolAccess),
  isDefault: optional(boolean),
  createdAt: optional(integer),
  updatedAt: optional(integer)
};
var groupFields = { name: optional(nonblank), description: optional(text2), themeConfigId: optional(nullable(text2)), members: optional(array(member)), createdAt: optional(integer), updatedAt: optional(integer) };
var tagFields = { name: optional(nonblank), description: optional(text2), promptContent: optional(text2), tagType: optional(enumeration("TONE", "CHARACTER", "FUNCTION", "CUSTOM")), createdAt: optional(integer), updatedAt: optional(integer) };
var memoryFields = {
  uuid: optional(text2),
  title: optional(nonblank),
  content: optional(text2),
  contentType: optional(text2),
  source: optional(text2),
  credibility: optional(score),
  importance: optional(score),
  documentPath: optional(nullable(text2)),
  isDocumentNode: optional(boolean),
  chunkIndexFilePath: optional(nullable(text2)),
  folderPath: optional(nullable(text2)),
  createdAt: optional(integer),
  updatedAt: optional(integer),
  lastAccessedAt: optional(integer),
  tags: optional(strings),
  properties: optional(array(property))
};
var settings = shape({
  autoSaveIntervalMinutes: required(nonnegative),
  nextAutoSaveRunAtMs: required(integer),
  memoryExtractionCustomRules: required(text2),
  profileAutoUpdateEnabled: required(boolean),
  profileAutoUpdateLocked: required(boolean),
  cloudEmbeddingEnabled: required(boolean),
  cloudEmbeddingEndpoint: required(text2),
  cloudEmbeddingApiKey: required(text2),
  cloudEmbeddingModel: required(text2)
});
var searchConfig = shape({ scoreMode: required(enumeration("BALANCED", "KEYWORD_FIRST", "SEMANTIC_FIRST")), keywordWeight: required(nonnegative), tagWeight: required(nonnegative), vectorWeight: required(nonnegative), edgeWeight: required(nonnegative) });
var empty = shape({});
var id = shape({ id: required(nonblank) });
var ownerOnly = shape({ ownerKey: required(owner) });
var contentOnly = shape({ content: required(text2) });
var format = enumeration("operit", "tavern");
var memoryQueryRequest = shape({ participantId: required(nonblank), query: required(nonblank), limit: required(nonnegative), snapshotId: required(nullable(nonblank)) });
var validators = {
  "memory.searchWithOptions": shape({ ownerKey: required(owner), query: required(text2), folderPath: required(nullable(text2)), relevanceThreshold: required(nonnegative), createdAtStartMs: required(nullable(integer)), createdAtEndMs: required(nullable(integer)) }),
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
  "character.create": shape({ values: required(shape({ ...characterFields, id: optional(text2), name: required(nonblank) })) }),
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
  "group.create": shape({ values: required(shape({ ...groupFields, id: optional(text2), name: required(nonblank) })) }),
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
  "tag.create": shape({ values: required(shape({ ...tagFields, id: optional(text2), name: required(nonblank) })) }),
  "tag.update": shape({ id: required(nonblank), changes: required(shape(tagFields, true)) }),
  "tag.delete": id,
  "memory.shared.list": empty,
  "memory.shared.create": shape({ name: required(nonblank) }),
  "memory.shared.rename": shape({ id: required(nonblank), name: required(nonblank) }),
  "memory.shared.delete": id,
  "memory.mount": shape({ characterId: required(nonblank), sharedId: required(nonblank), readable: required(boolean), writable: required(boolean) }),
  "memory.unmount": shape({ characterId: required(nonblank), sharedId: required(nonblank) }),
  "memory.user.read": ownerOnly,
  "memory.user.write": shape({ ownerKey: required(owner), content: required(text2) }),
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
  "memory.create": shape({ ownerKey: required(owner), values: required(shape({ ...memoryFields, id: optional(memoryIdentifier), title: required(nonblank), content: required(text2) })) }),
  "memory.update": shape({ ownerKey: required(owner), originalTitle: required(nonblank), expectedId: optional(memoryIdentifier), changes: required(shape(memoryFields, true)) }),
  "memory.delete": shape({ ownerKey: required(owner), id: required(memoryIdentifier) }),
  "memory.move": shape({ ownerKey: required(owner), ids: required(array(memoryIdentifier)), folderPath: required(text2) }),
  "memory.link.create": shape({ ownerKey: required(owner), sourceTitle: required(nonblank), targetTitle: required(nonblank), linkType: required(nonblank), weight: required(score), description: required(text2) }),
  "memory.link.update": shape({ ownerKey: required(owner), linkId: required(memoryIdentifier), changes: required(shape({ linkType: optional(nonblank), weight: optional(score), description: optional(text2) }, true)) }),
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
    defaultModelBinding: required(shape({ providerId: required(text2), modelId: required(text2) })),
    defaultTtsConfigId: required(text2)
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
                const active = identity2.kind === "card" ? yield callMainDomain("activePrompt.setCard", { id: identity2.id }) : yield callMainDomain("activePrompt.setGroup", { id: identity2.id });
                if (!active.active || active.id !== identity2.id || active.type !== (identity2.kind === "card" ? "character_card" : "character_group")) throw new Error("Main runtime did not confirm the selected global active prompt");
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
  if (requiredMode !== null && screenSession.currentScreen().input.mode !== requiredMode) throw new Error("The attachment route requires an explicit attachment presentation");
  const origin = "https://characters.operit.local/";
  function applyTheme(theme) {
    return __async(this, null, function* () {
      if (ready.current) yield controller.evaluateJavascript(`window.applyCharacterMemoryTheme(${JSON.stringify(theme)});`);
    });
  }
  function initialize() {
    return __async(this, null, function* () {
      try {
        controller.addJavascriptInterface("CharacterMemoryHost", {
          /** Marks the document ready and returns the actual host palette. */
          currentTheme: () => {
            ready.current = true;
            return ctx.Theme.getCurrent();
          },
          /** Routes all editing operations through this package's main runtime. */
          request: (...args) => __async(null, null, function* () {
            const [request] = webArguments(args, 1);
            const decoded = record(request, "character-memory.request");
            return ToolPkg.ipc.call("character-memory.request", decoded, { targetRuntime: "main" });
          }),
          /** Returns the readonly plugin screen input and its real host presentation request id. */
          currentScreen: (...args) => {
            webArguments(args, 0);
            const current = screenSession.currentScreen();
            return current.input.mode === "manage" && managementView === "memory" ? { ...current, input: { mode: "manage", view: "memory" } } : current;
          },
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
          /** Reads an image through Files instead of exposing VFS paths to Image.file. */
          avatarImage: (...args) => __async(null, null, function* () {
            const [uri] = webArguments(args, 1);
            if (uri !== null && typeof uri !== "string") throw new Error("\u5934\u50CF\u8DEF\u5F84\u5FC5\u987B\u662F\u5B57\u7B26\u4E32\u6216 null");
            return imageSource(uri === null ? yield ToolPkg.readResource("character_default_avatar", "operit-avatar.png") : uri);
          }),
          /** Imports only a user-selected image to persistent package storage. */
          chooseAvatar: (...args) => __async(null, null, function* () {
            webArguments(args, 0);
            const selected = yield ctx.openFilePicker({ picker: "image", allowMultiple: false, mimeTypes: ["image/*"] });
            if (selected.cancelled) return null;
            if (selected.files.length !== 1) throw new Error("\u8BF7\u9009\u62E9\u4E00\u5F20\u5934\u50CF\u56FE\u7247");
            const platform = (yield Tools.System.terminal.info()).platform;
            const source = yield imageSource(pickedImagePath(selected.files[0].path, platform));
            const directory = ToolPkg.getConfigDir().replace(/\/$/, "") + "/avatars";
            const created = yield Tools.Files.mkdir(directory, true);
            if (!created.successful) throw new Error(created.details);
            const uri = directory + "/" + Date.now() + "-" + Math.random().toString(36).slice(2) + ".image";
            const saved = yield Tools.Files.writeBinary(uri, source.slice(source.indexOf(",") + 1));
            if (!saved.successful) throw new Error(saved.details);
            return { uri, source };
          }),
          /** Saves an export to the VFS path explicitly supplied by the user. */
          exportFile: (...args) => __async(null, null, function* () {
            const [path2, content] = webArguments(args, 2);
            if (typeof path2 !== "string" || path2.trim() === "") throw new Error("\u8BF7\u8F93\u5165\u5BFC\u51FA\u6587\u4EF6\u7684 VFS \u8DEF\u5F84");
            if (typeof content !== "string") throw new Error("\u5BFC\u51FA\u5185\u5BB9\u5FC5\u987B\u662F\u5B57\u7B26\u4E32");
            yield Tools.Files.create(path2, content);
            return true;
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
  return ctx.UI.Box({ fillMaxSize: true, onLoad: initialize }, path === "" ? ctx.UI.Text({ text: error === "" ? `\u6B63\u5728\u52A0\u8F7D${managementView === "memory" ? "\u8BB0\u5FC6" : definition2.title}\u2026` : error }) : ctx.UI.WebView({
    key: "character-memory-web",
    controller,
    fillMaxSize: true,
    url: origin,
    javaScriptEnabled: true,
    domStorageEnabled: true,
    supportZoom: false,
    useWideViewPort: true,
    /** Restricts top-level navigation to the package document. */
    onShouldOverrideUrlLoading: (request) => request.url === origin ? { action: "allow" } : { action: "cancel" },
    /** Serves the single offline asset through the existing cross-platform resource host. */
    onInterceptRequest: (request) => request.url === origin ? { action: "respond", response: { mimeType: "text/html", encoding: "utf-8", statusCode: 200, reasonPhrase: "OK", filePath: path } } : { action: "block" },
    /** Handles disposal without retaining the screen in the host theme service. */
    onLifecycleEvent: (event) => {
      if (event.type === "Disposed") dispose();
    }
  }));
}

// src/ui/memory/index.ui.ts
function Screen(ctx) {
  return renderScreen(ctx, definition, null, "memory");
}

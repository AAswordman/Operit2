// @operit-compose-artifact 4
"use strict";
var __operitUiPrograms_=[{"id":"com.operit.character_cards:dist/ui/chat-input-memory/index.ui.js:93zuy:68751","root":0,"nodes":[{"type":"Icon","props":[2,["name","size","tint"],[[1,0],[0,16],[1,1]]],"children":[0,null],"explicitChildren":false}]},{"id":"com.operit.character_cards:dist/ui/chat-input-memory/index.ui.js:93zuy:68975","root":0,"nodes":[{"type":"Row","props":[2,["key","fillMaxWidth","paddingHorizontal","verticalAlignment","modifier","onClick"],[[1,0],[0,true],[0,12],[0,"center"],[1,1],[1,2]]],"children":[3,[[1,3],[4,1],[4,2],[4,3],[4,4],[4,5]]],"explicitChildren":true},{"type":"Spacer","props":[2,["width"],[[0,12]]],"children":[0,null],"explicitChildren":false},{"type":"Text","props":[2,["text","style","color","maxLines","overflow","weight"],[[1,4],[0,"bodySmall"],[1,5],[0,1],[0,"ellipsis"],[0,1]]],"children":[0,null],"explicitChildren":false},{"type":"Text","props":[2,["text","style","color","fontWeight"],[[1,6],[0,"bodySmall"],[1,7],[0,"600"]]],"children":[0,null],"explicitChildren":false},{"type":"Spacer","props":[2,["width"],[[0,8]]],"children":[0,null],"explicitChildren":false},{"type":"Switch","props":[2,["checked","width","height","onCheckedChange"],[[1,8],[0,40],[0,28],[1,9]]],"children":[0,null],"explicitChildren":false}]},{"id":"com.operit.character_cards:dist/ui/chat-input-memory/index.ui.js:93zuy:69734","root":0,"nodes":[{"type":"Box","props":[2,["key"],[[0,"memory-menu-unavailable"]]],"children":[0,null],"explicitChildren":false}]},{"id":"com.operit.character_cards:dist/ui/chat-input-memory/index.ui.js:93zuy:69817","root":0,"nodes":[{"type":"Box","props":[2,["key","onLoad"],[[0,"memory-menu-loading"],[1,0]]],"children":[0,null],"explicitChildren":false}]},{"id":"com.operit.character_cards:dist/ui/chat-input-memory/index.ui.js:93zuy:69895","root":0,"nodes":[{"type":"Row","props":[2,["key","fillMaxWidth","paddingHorizontal","verticalAlignment","modifier","onClick"],[[0,"memory-section-header"],[0,true],[0,12],[0,"center"],[1,0],[1,1]]],"children":[3,[[1,2],[4,1],[4,2],[4,3],[4,4],[4,5],[4,7],[4,8]]],"explicitChildren":true},{"type":"Spacer","props":[2,["width"],[[0,12]]],"children":[0,null],"explicitChildren":false},{"type":"Text","props":[2,["text","style"],[[0,"记忆"],[0,"bodySmall"]]],"children":[0,null],"explicitChildren":false},{"type":"Spacer","props":[2,["width"],[[0,8]]],"children":[0,null],"explicitChildren":false},{"type":"Text","props":[2,["text","style","color","fontWeight","textAlign","maxLines","overflow","weight"],[[1,3],[0,"bodySmall"],[1,4],[0,"600"],[0,"end"],[0,1],[0,"ellipsis"],[0,1]]],"children":[0,null],"explicitChildren":false},{"type":"IconButton","props":[2,["key","tooltip","width","height","contentPadding","onClick","content"],[[0,"memory-info"],[0,"记忆状态说明"],[0,24],[0,28],[2,["horizontal","vertical"],[[0,0],[0,0]]],[1,5],[4,6]]],"children":[0,null],"explicitChildren":false},{"type":"Icon","props":[2,["name","size","tint"],[[0,"InfoOutline"],[0,16],[1,6]]],"children":[0,null],"explicitChildren":false},{"type":"Spacer","props":[2,["width"],[[0,6]]],"children":[0,null],"explicitChildren":false},{"type":"Icon","props":[2,["name","size","tint"],[[1,7],[0,20],[1,8]]],"children":[0,null],"explicitChildren":false}]},{"id":"com.operit.character_cards:dist/ui/chat-input-memory/index.ui.js:93zuy:71086","root":0,"nodes":[{"type":"Column","props":[2,["key","modifier"],[[0,"memory-section-children"],[1,0]]],"children":[3,[[1,1],[1,2],[4,1]]],"explicitChildren":true},{"type":"Row","props":[2,["key","fillMaxWidth","paddingHorizontal","verticalAlignment","modifier","onClick"],[[0,"memory-manual"],[0,true],[0,12],[0,"center"],[1,3],[1,4]]],"children":[3,[[1,5],[4,2],[4,3]]],"explicitChildren":true},{"type":"Spacer","props":[2,["width"],[[0,12]]],"children":[0,null],"explicitChildren":false},{"type":"Text","props":[2,["text","style","maxLines","overflow","weight","color"],[[1,6],[0,"bodySmall"],[0,2],[0,"ellipsis"],[0,1],[1,7]]],"children":[0,null],"explicitChildren":false}]},{"id":"com.operit.character_cards:dist/ui/chat-input-memory/index.ui.js:93zuy:72117","root":0,"nodes":[{"type":"Column","props":[2,["key","fillMaxWidth"],[[0,"memory-section"],[0,true]]],"children":[1,0],"explicitChildren":true}]}];

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

// src/ui/chat-input-memory/index.ui.ts
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
function assertNumber(value, label, min, max) {
  if (typeof value !== "number" || !Number.isFinite(value) || value < min || value > max) throw new Error(`${label} \u5FC5\u987B\u5728 ${min} \u5230 ${max} \u4E4B\u95F4`);
}
function assertInteger(value, label, min) {
  assertNumber(value, label, min, Number.MAX_SAFE_INTEGER);
  if (!Number.isSafeInteger(value)) throw new Error(`${label} \u5FC5\u987B\u662F\u5B89\u5168\u6574\u6570`);
}
function requireId(value, label) {
  assertString(value, label);
  if (value.trim() === "" || value !== value.trim()) throw new Error(`${label} \u65E0\u6548`);
  return value;
}

// src/chat-bindings.ts
function parseChatSelection(selection) {
  requireId(selection, "chat selection");
  const parsed = /^(card|group|model):([^\s]+)$/u.exec(selection);
  if (parsed === null) throw new Error("Invalid plugin chat selection: " + selection);
  const kind = parsed[1];
  if (kind === "model" && parsed[2] !== "chat") throw new Error("Invalid model selection: " + selection);
  if (kind !== "card" && kind !== "group" && kind !== "model") throw new Error("Invalid chat selection kind: " + kind);
  return { kind, id: parsed[2] };
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
  const selection = requireId(extension.selection, "chat marker selection");
  parseChatSelection(selection);
  if (Object.prototype.hasOwnProperty.call(extension, "groupId")) throw new Error("Conversation membership is owned by plugin files, not chat extension.groupId");
  return { version: 1, selection };
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
function shape(fields, nonempty = false) {
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
  "memory.searchWithOptions": shape({ ownerKey: required(owner), query: required(nonblank), folderPath: required(nullable(text)), relevanceThreshold: required(nonnegative), createdAtStartMs: required(nullable(integer)), createdAtEndMs: required(nullable(integer)) }),
  "memory.chat.list": ownerOnly,
  "memory.chat.pruneMissing": ownerOnly,
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

// src/identity.ts
function uuid() {
  const bytes = [];
  for (let index = 0; index < 16; index += 1) bytes.push(Math.floor(Math.random() * 256));
  bytes[6] = bytes[6] & 15 | 64;
  bytes[8] = bytes[8] & 63 | 128;
  const text2 = bytes.map(
    /** Encodes every byte as two hexadecimal digits. */
    (byte) => byte.toString(16).padStart(2, "0")
  ).join("");
  return text2.slice(0, 8) + "-" + text2.slice(8, 12) + "-" + text2.slice(12, 16) + "-" + text2.slice(16, 20) + "-" + text2.slice(20);
}

// src/storage/merge.json
var merge_default = {
  cards: {
    version: 1,
    catalog: {
      keyPrefix: "record_",
      keySuffix: "",
      identityField: "id",
      nameField: "name",
      ignoredFields: [
        "id",
        "createdAt",
        "updatedAt"
      ],
      retiredKeys: []
    },
    references: [
      {
        key: {
          kind: "records",
          prefix: "record_",
          suffix: ""
        },
        path: [
          {
            kind: "field",
            name: "chatProviderId"
          }
        ],
        target: "runtime/config/preferences/model_configs.preferences.json"
      },
      {
        key: {
          kind: "records",
          prefix: "record_",
          suffix: ""
        },
        path: [
          {
            kind: "field",
            name: "ttsConfigId"
          }
        ],
        target: "runtime/config/preferences/tts_configs.preferences.json"
      },
      {
        key: {
          kind: "records",
          prefix: "record_",
          suffix: ""
        },
        path: [
          {
            kind: "field",
            name: "themeConfigId"
          }
        ],
        target: "runtime/config/preferences/user_preferences.preferences.json"
      },
      {
        key: {
          kind: "records",
          prefix: "record_",
          suffix: ""
        },
        path: [
          {
            kind: "field",
            name: "sharedMemoryId"
          }
        ],
        target: "runtime/plugin_data/space/com.operit.character_cards/stores.preferences.json"
      },
      {
        key: {
          kind: "records",
          prefix: "record_",
          suffix: ""
        },
        path: [
          {
            kind: "field",
            name: "attachedTagIds"
          },
          {
            kind: "array_values"
          }
        ],
        target: "runtime/plugin_data/space/com.operit.character_cards/tags.preferences.json"
      },
      {
        key: {
          kind: "records",
          prefix: "record_",
          suffix: ""
        },
        path: [
          {
            kind: "field",
            name: "sharedMemoryMounts"
          },
          {
            kind: "array_values"
          },
          {
            kind: "field",
            name: "sharedMemoryId"
          }
        ],
        target: "runtime/plugin_data/space/com.operit.character_cards/stores.preferences.json"
      }
    ],
    projection: null
  },
  groups: {
    version: 1,
    catalog: {
      keyPrefix: "record_",
      keySuffix: "",
      identityField: "id",
      nameField: "name",
      ignoredFields: [
        "id",
        "createdAt",
        "updatedAt"
      ],
      retiredKeys: []
    },
    references: [
      {
        key: {
          kind: "records",
          prefix: "record_",
          suffix: ""
        },
        path: [
          {
            kind: "field",
            name: "themeConfigId"
          }
        ],
        target: "runtime/config/preferences/user_preferences.preferences.json"
      },
      {
        key: {
          kind: "records",
          prefix: "record_",
          suffix: ""
        },
        path: [
          {
            kind: "field",
            name: "members"
          },
          {
            kind: "array_values"
          },
          {
            kind: "field",
            name: "characterCardId"
          }
        ],
        target: "runtime/plugin_data/space/com.operit.character_cards/cards.preferences.json"
      }
    ],
    projection: null
  },
  tags: {
    version: 1,
    catalog: {
      keyPrefix: "record_",
      keySuffix: "",
      identityField: "id",
      nameField: "name",
      ignoredFields: [
        "id",
        "createdAt",
        "updatedAt"
      ],
      retiredKeys: []
    },
    references: [],
    projection: null
  },
  stores: {
    version: 1,
    catalog: {
      keyPrefix: "record_",
      keySuffix: "",
      identityField: "id",
      nameField: "name",
      ignoredFields: [
        "id",
        "createdAt",
        "updatedAt"
      ],
      retiredKeys: []
    },
    references: [],
    projection: null
  },
  conversationGroups: {
    version: 1,
    catalog: null,
    references: [
      {
        key: {
          kind: "records",
          prefix: "record_",
          suffix: ""
        },
        path: [
          {
            kind: "field",
            name: "ownerSelection"
          },
          {
            kind: "identity",
            prefix: "card:",
            suffix: ""
          }
        ],
        target: "runtime/plugin_data/space/com.operit.character_cards/cards.preferences.json"
      },
      {
        key: {
          kind: "records",
          prefix: "record_",
          suffix: ""
        },
        path: [
          {
            kind: "field",
            name: "ownerSelection"
          },
          {
            kind: "identity",
            prefix: "group:",
            suffix: ""
          }
        ],
        target: "runtime/plugin_data/space/com.operit.character_cards/groups.preferences.json"
      }
    ],
    projection: null
  },
  meta: {
    version: 1,
    catalog: null,
    references: [
      {
        key: {
          kind: "exact",
          key: "state"
        },
        path: [
          {
            kind: "field",
            name: "active"
          },
          {
            kind: "variant",
            name: "CharacterCard"
          },
          {
            kind: "field",
            name: "id"
          }
        ],
        target: "runtime/plugin_data/space/com.operit.character_cards/cards.preferences.json"
      },
      {
        key: {
          kind: "exact",
          key: "state"
        },
        path: [
          {
            kind: "field",
            name: "active"
          },
          {
            kind: "variant",
            name: "CharacterGroup"
          },
          {
            kind: "field",
            name: "id"
          }
        ],
        target: "runtime/plugin_data/space/com.operit.character_cards/groups.preferences.json"
      }
    ],
    projection: null
  },
  chats: {
    version: 1,
    catalog: null,
    references: [
      {
        key: {
          kind: "records",
          prefix: "binding_",
          suffix: ""
        },
        path: [
          {
            kind: "field",
            name: "selection"
          },
          {
            kind: "identity",
            prefix: "card:",
            suffix: ""
          }
        ],
        target: "runtime/plugin_data/space/com.operit.character_cards/cards.preferences.json"
      },
      {
        key: {
          kind: "records",
          prefix: "draft_",
          suffix: ""
        },
        path: [
          {
            kind: "field",
            name: "selection"
          },
          {
            kind: "identity",
            prefix: "card:",
            suffix: ""
          }
        ],
        target: "runtime/plugin_data/space/com.operit.character_cards/cards.preferences.json"
      },
      {
        key: {
          kind: "records",
          prefix: "binding_",
          suffix: ""
        },
        path: [
          {
            kind: "field",
            name: "selection"
          },
          {
            kind: "identity",
            prefix: "group:",
            suffix: ""
          }
        ],
        target: "runtime/plugin_data/space/com.operit.character_cards/groups.preferences.json"
      },
      {
        key: {
          kind: "records",
          prefix: "draft_",
          suffix: ""
        },
        path: [
          {
            kind: "field",
            name: "selection"
          },
          {
            kind: "identity",
            prefix: "group:",
            suffix: ""
          }
        ],
        target: "runtime/plugin_data/space/com.operit.character_cards/groups.preferences.json"
      }
    ],
    projection: null
  }
};

// src/storage/state.ts
function userDocumentPath(ownerKey) {
  return "owners/" + encodeURIComponent(ownerKey) + "/USER.md";
}

// src/storage/records.ts
var ownerCollections = [
  "memories",
  "links",
  "chunks",
  "candidates",
  "embeddings"
];
function clone(value) {
  if (Array.isArray(value)) return value.map(clone);
  if (value !== null && typeof value === "object")
    return Object.fromEntries(
      Object.entries(value).map(([key, item]) => [key, clone(item)])
    );
  return value;
}
function equal(left, right) {
  if (left === right) return true;
  if (left === null || right === null || typeof left !== "object" || typeof right !== "object")
    return false;
  const keys = Object.keys(left), other = Object.keys(right);
  return Array.isArray(left) === Array.isArray(right) && keys.length === other.length && keys.every(
    (key) => Object.prototype.hasOwnProperty.call(right, key) && equal(
      left[key],
      right[key]
    )
  );
}
function recordKey(value) {
  assertObject(value, "stored record");
  return requireId(value.id, "stored record id");
}
function storedValue(record2) {
  return record2;
}
function records(state) {
  const result = [];
  for (const collection of [
    "cards",
    "groups",
    "tags",
    "stores",
    "conversationGroups"
  ]) {
    for (const record2 of state[collection])
      result.push({
        collection,
        key: record2.id,
        value: storedValue(record2),
        version: null
      });
  }
  for (const owner2 of state.owners) {
    const { memories, links, chunks, candidates, embeddings, ...metadata } = owner2;
    result.push({
      collection: "owners",
      key: owner2.ownerKey,
      value: storedValue(metadata),
      version: null
    });
    for (const collection of ownerCollections)
      for (const record2 of owner2[collection])
        result.push({
          collection,
          key: owner2.ownerKey + "/" + recordKey(storedValue(record2)),
          value: storedValue({ ownerKey: owner2.ownerKey, record: record2 }),
          version: null
        });
  }
  return result;
}
function decodeStateFromRecords(metadata, rows) {
  assertObject(metadata, "character database metadata");
  if (metadata.migrationStatus === "writing" || metadata.legacyImportPending === true)
    throw new Error("Character migration has not completed");
  const state = {
    version: metadata.version,
    active: metadata.active === null ? { Model: { id: "chat" } } : metadata.active,
    memoryPreferences: metadata.memoryPreferences,
    cards: [],
    groups: [],
    tags: [],
    stores: [],
    conversationGroups: [],
    owners: []
  };
  const owners = /* @__PURE__ */ new Map();
  for (const row of rows) {
    switch (row.collection) {
      case "owners": {
        assertObject(row.value, "memory owner metadata");
        const owner2 = {
          ...row.value,
          memories: [],
          links: [],
          chunks: [],
          candidates: [],
          embeddings: []
        };
        if (owner2.ownerKey !== row.key || owners.has(row.key))
          throw new Error("Invalid memory owner record: " + row.key);
        owners.set(row.key, owner2);
        state.owners.push(owner2);
        break;
      }
      case "cards":
      case "groups":
      case "tags":
      case "stores":
      case "conversationGroups":
        state[row.collection].push(row.value);
        break;
    }
  }
  for (const row of rows) {
    if (!ownerCollections.some((collection) => collection === row.collection))
      continue;
    assertObject(row.value, "owner-scoped record");
    const ownerKey = requireId(row.value.ownerKey, "record owner key");
    const owner2 = owners.get(ownerKey);
    if (owner2 === void 0)
      throw new Error("Missing record owner: " + ownerKey);
    const entries = owner2[row.collection];
    if (!Array.isArray(entries))
      throw new Error("Invalid owner collection: " + row.collection);
    if (row.key !== ownerKey + "/" + recordKey(row.value.record)) {
      throw new Error("Invalid owner-scoped record: " + row.key);
    }
    entries.push(row.value.record);
  }
  return state;
}

// src/storage/migrations/legacy-to-v1.ts
function assertMigrationPlan(value) {
  assertObject(value, "character schema migration");
  if (value.from !== "released" || value.to !== 1) throw new Error("Unsupported pending character schema migration");
  if (!["files", "authors", "records", "verify", "cleanup"].some((phase) => phase === value.phase)) throw new Error("Invalid character migration phase");
  assertInteger(value.cursor, "character migration cursor", 0);
  assertObject(value.identities, "character migration identities");
  for (const field of ["files", "directories", "authors", "changes"])
    if (!Array.isArray(value[field])) throw new Error("Invalid character migration work: " + field);
  for (const directory of value.directories) {
    assertObject(directory, "character migration directory");
    for (const field of ["source", "destination"])
      if (typeof directory[field] !== "string" || !/^owners\/(?:character|shared)%3A[^/]+$/u.test(directory[field]))
        throw new Error("Invalid character migration owner directory");
    if (directory.source === directory.destination) throw new Error("Character migration owner directory must change");
  }
  for (const file of value.files) {
    assertObject(file, "character migration file");
    if (typeof file.source !== "string" || typeof file.destination !== "string") throw new Error("Invalid character migration file path");
    const source = file.source, destination = file.destination;
    if (!value.directories.some((directory) => source.startsWith(directory.source + "/") && destination === directory.destination + source.slice(directory.source.length)))
      throw new Error("Character migration file is outside its owner directory");
  }
  for (const group of value.changes)
    if (!Array.isArray(group) || group.length < 1 || group.length > 2) throw new Error("Invalid atomic character migration record change");
  const lengths = { files: value.files.length, authors: value.authors.length, records: value.changes.length, verify: 0, cleanup: value.directories.length };
  if (value.cursor > lengths[value.phase]) throw new Error("Character migration cursor exceeds its work");
}
function completed(result) {
  if (!result.successful) throw new Error(result.details);
}
function absolute(directory, relative) {
  if (relative.split("/").some((part) => part === "" || part === "." || part === ".." || /[\\:\u0000-\u001f]/u.test(part)))
    throw new Error("Invalid character migration resource path: " + relative);
  return directory + "/" + relative;
}
function ownerFiles(directory, source, destination) {
  return __async(this, null, function* () {
    const path = absolute(directory, source);
    if (!(yield Tools.Files.exists(path)).exists) return [];
    const listing = yield Tools.Files.list(path);
    const result = [];
    for (const entry of listing.entries) {
      if (entry.name === "." || entry.name === ".." || /[/\\:\u0000-\u001f]/u.test(entry.name))
        throw new Error("Invalid owner resource entry: " + entry.name);
      const from = source + "/" + entry.name, to = destination + "/" + entry.name;
      if (entry.isDirectory) result.push(...yield ownerFiles(directory, from, to));
      else result.push({ source: from, destination: to });
    }
    return result;
  });
}
function removeDeletedReferences(state) {
  const tags = new Set(state.tags.map((tag) => tag.id));
  const stores = new Set(state.stores.map((store) => store.id));
  const cards = new Set(state.cards.map((card) => card.id));
  for (const card of state.cards) {
    card.attachedTagIds = card.attachedTagIds.filter((id2) => tags.has(id2));
    card.sharedMemoryMounts = card.sharedMemoryMounts.filter((mount2) => stores.has(mount2.sharedMemoryId));
  }
  for (const group of state.groups) group.members = group.members.filter((member2) => cards.has(member2.characterCardId));
  for (const owner2 of state.owners) {
    const memories = new Set(owner2.memories.map((memory) => memory.id));
    const documents2 = new Set(owner2.memories.filter((memory) => memory.isDocumentNode).map((memory) => memory.uuid));
    owner2.links = owner2.links.filter((link) => memories.has(link.sourceMemoryId) && memories.has(link.targetMemoryId));
    owner2.chunks = owner2.chunks.filter((chunk) => documents2.has(chunk.memoryUuid));
  }
}
function prepareVersionOne(storage, metadata, directory) {
  return __async(this, null, function* () {
    const rows = yield migrationRecords(storage);
    const input = clone(rows);
    for (const row of input) {
      if (row.collection === "cards") {
        assertObject(row.value, "legacy card");
        delete row.value.isDefault;
      }
    }
    const inputMetadata = { ...metadata, version: 1 };
    delete inputMetadata.legacyImportPending;
    const state = decodeStateFromRecords(inputMetadata, input);
    removeDeletedReferences(state);
    const before = records(state), identities = {};
    function identity(scope, value) {
      requireId(value, scope + " identity");
      const key = JSON.stringify([scope, value]);
      if (!Object.prototype.hasOwnProperty.call(identities, key)) identities[key] = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/iu.test(value) ? value : uuid();
      return identities[key];
    }
    function reference(scope, value) {
      const key = JSON.stringify([scope, value]);
      if (!Object.prototype.hasOwnProperty.call(identities, key)) throw new Error("Missing migration reference: " + key);
      return identities[key];
    }
    const sourceStoreIds = new Set(state.stores.map((store) => store.id));
    const sourceCardIds = new Set(state.cards.map((card) => card.id));
    const sourceGroupIds = new Set(state.groups.map((group) => group.id));
    for (const collection of configurationNames)
      for (const record2 of state[collection]) record2.id = identity(collection, record2.id);
    function selection(value) {
      const parsed = parseChatSelection(value);
      if (parsed.kind === "card" && sourceCardIds.has(parsed.id)) value = "card:" + reference("cards", parsed.id);
      if (parsed.kind === "group" && sourceGroupIds.has(parsed.id)) value = "group:" + reference("groups", parsed.id);
      return value;
    }
    for (const card of state.cards) {
      card.attachedTagIds = card.attachedTagIds.map((id2) => reference("tags", id2));
      if (card.sharedMemoryId !== null && sourceStoreIds.has(card.sharedMemoryId)) card.sharedMemoryId = reference("stores", card.sharedMemoryId);
      for (const mount2 of card.sharedMemoryMounts) mount2.sharedMemoryId = reference("stores", mount2.sharedMemoryId);
    }
    for (const group of state.groups)
      for (const member2 of group.members) member2.characterCardId = reference("cards", member2.characterCardId);
    for (const group of state.conversationGroups) {
      if (group.ownerSelection !== null && /^(card|group):/u.test(group.ownerSelection)) group.ownerSelection = selection(group.ownerSelection);
    }
    if ("CharacterCard" in state.active && sourceCardIds.has(state.active.CharacterCard.id)) state.active.CharacterCard.id = reference("cards", state.active.CharacterCard.id);
    if ("CharacterGroup" in state.active && sourceGroupIds.has(state.active.CharacterGroup.id)) state.active.CharacterGroup.id = reference("groups", state.active.CharacterGroup.id);
    const directories = [];
    for (const owner2 of state.owners) {
      const old = owner2.ownerKey;
      if (old.startsWith("character:")) owner2.ownerKey = "character:" + reference("cards", old.slice(10));
      else if (old.startsWith("shared:")) owner2.ownerKey = "shared:" + reference("stores", old.slice(7));
      else throw new Error("Invalid migration owner: " + old);
      owner2.userDocumentPath = userDocumentPath(owner2.ownerKey);
      if (old !== owner2.ownerKey) directories.push({ source: "owners/" + encodeURIComponent(old), destination: "owners/" + encodeURIComponent(owner2.ownerKey) });
      const scope = owner2.ownerKey;
      for (const collection of ["memories", "links", "chunks", "candidates", "embeddings"])
        for (const record2 of owner2[collection]) record2.id = identity(scope + "/" + collection, record2.id);
      for (const memory of owner2.memories) {
        memory.uuid = identity(scope + "/memoryUuid", memory.uuid);
        for (const tag of memory.tags) tag.id = identity(scope + "/memoryTags", tag.id);
        for (const property2 of memory.properties) property2.id = identity(scope + "/memoryProperties", property2.id);
      }
      for (const link of owner2.links) {
        link.sourceMemoryId = reference(scope + "/memories", link.sourceMemoryId);
        link.targetMemoryId = reference(scope + "/memories", link.targetMemoryId);
      }
      for (const chunk of owner2.chunks) chunk.memoryUuid = reference(scope + "/memoryUuid", chunk.memoryUuid);
      if (owner2.rebuildTask !== null) owner2.rebuildTask.id = identity(scope + "/rebuildTask", owner2.rebuildTask.id);
    }
    const resourceMoves = [];
    for (const move of directories) {
      resourceMoves.push(move, { source: absolute(directory, move.source), destination: absolute(directory, move.destination) });
    }
    for (const owner2 of state.owners) for (const memory of owner2.memories)
      for (const field of ["documentPath", "chunkIndexFilePath"]) {
        const path = memory[field];
        if (path === null) continue;
        const source = path.startsWith("owners/") ? absolute(directory, path) : path;
        if (!(yield Tools.Files.exists(source)).exists) {
          memory[field] = null;
          continue;
        }
        for (const move of resourceMoves) if (path.startsWith(move.source + "/")) {
          memory[field] = move.destination + path.slice(move.source.length);
          break;
        }
      }
    const after = records(state), changes = [];
    const original = new Map(rows.map((row) => [JSON.stringify([row.collection, row.key]), row]));
    const targets = /* @__PURE__ */ new Set();
    for (let index = 0; index < after.length; index += 1) {
      const from = before[index], to = after[index], key = JSON.stringify([to.collection, to.key]);
      if (targets.has(key)) throw new Error("Duplicate migration destination: " + key);
      targets.add(key);
      const source = original.get(JSON.stringify([from.collection, from.key]));
      if (source === void 0) throw new Error("Missing migration source record");
      if (from.key === to.key) {
        if (!equal(source.value, to.value)) changes.push([{ op: "put", collection: to.collection, key: to.key, value: to.value, expectedValue: source.value }]);
      } else {
        if (original.has(key)) throw new Error("Occupied migration destination: " + key);
        changes.push([
          { op: "put", collection: to.collection, key: to.key, value: to.value, expectedValue: null },
          { op: "delete", collection: from.collection, key: from.key, expectedValue: source.value }
        ]);
      }
    }
    const retained = new Set(before.map((row) => JSON.stringify([row.collection, row.key])));
    for (const row of rows) if (!retained.has(JSON.stringify([row.collection, row.key])))
      changes.push([{ op: "delete", collection: row.collection, key: row.key, expectedValue: row.value }]);
    for (const [key, originalValue] of (yield storage.chats.data()).entries()) {
      const collection = key.startsWith("binding_") ? "chat_bindings" : key.startsWith("draft_") ? "chat_drafts" : null;
      if (collection === null) throw new Error("Unsupported chat preference during migration: " + key);
      const id2 = key.slice(collection === "chat_bindings" ? 8 : 6);
      assertObject(originalValue, collection);
      const value = clone(originalValue);
      value.selection = selection(requireId(value.selection, "stored selection"));
      if (!equal(value, originalValue)) changes.push([{ op: "put", collection, key: id2, value, expectedValue: originalValue }]);
    }
    const files = [];
    for (const move of directories) {
      const entries = yield ownerFiles(directory, move.source, move.destination);
      files.push(...entries);
    }
    const authors = [];
    const cardIds = new Map(before.filter((row) => row.collection === "cards").map((row) => [row.key, reference("cards", row.key)]));
    if ([...cardIds].some(([old, next]) => old !== next)) {
      const chats = yield Tools.Chat.listAll();
      for (const chat of chats.chats) for (let start = 0; start < chat.messageCount; start += 100) {
        const page = yield Tools.Chat.getMessagesRange(chat.id, { order: "asc", start, end: Math.min(chat.messageCount - 1, start + 99) });
        for (const message of page.messages) for (let variantIndex = 0; variantIndex < message.variantCount; variantIndex += 1) {
          const target = { kind: "message", chatId: chat.id, messageTimestamp: message.timestamp, variantIndex };
          const { extension } = yield Tools.Chat.readExtension(target);
          if (extension === null) continue;
          assertObject(extension, "legacy message author");
          if (Object.keys(extension).length !== 1) throw new Error("Invalid legacy message author");
          const participant = requireId(extension.participantId, "legacy participant id"), next = cardIds.get(participant);
          if (next !== void 0 && next !== participant) authors.push({ target, before: participant, after: next });
        }
      }
    }
    return { from: "released", to: 1, phase: "files", cursor: 0, active: storedValue(state.active), identities, files, directories, authors, changes };
  });
}
function verifyFile(directory, file) {
  return __async(this, null, function* () {
    const source = yield Tools.Files.readBinary(absolute(directory, file.source));
    const target = yield Tools.Files.readBinary(absolute(directory, file.destination));
    if (source.size !== target.size || source.contentBase64 !== target.contentBase64)
      throw new Error("Character migration resource verification failed: " + file.destination);
  });
}
function migrationRecords(storage) {
  return __async(this, null, function* () {
    const rows = [], configuration = yield storage.data();
    for (const name of configurationNames) for (const [key, value] of configuration[name].entries()) {
      if (!key.startsWith(recordPrefix)) throw new Error("Unknown character preference during migration: " + key);
      rows.push({ collection: name, key: key.slice(recordPrefix.length), value, version: null });
    }
    for (const collection of ["owners", "memories", "links", "chunks", "candidates", "embeddings"])
      for (const entry of yield storage.memory.read(collection)) rows.push({ collection, ...entry });
    return rows;
  });
}
function applyChanges(storage, changes) {
  return __async(this, null, function* () {
    const first = changes[0];
    if (first === void 0) throw new Error("Empty character migration unit");
    if (changes.some((change) => change.collection !== first.collection)) throw new Error("Migration unit spans storage owners");
    const collection = first.collection;
    const name = configurationNames.find((name2) => name2 === collection);
    const store = name === void 0 ? collection === "chat_bindings" || collection === "chat_drafts" ? storage.chats : null : storage.configuration[name];
    if (store !== null) {
      yield store.edit((preferences) => {
        const prefix = collection === "chat_bindings" ? "binding_" : collection === "chat_drafts" ? "draft_" : recordPrefix;
        for (const change of changes) {
          const key = prefix + change.key, value = preferences.get(key);
          if (change.op === "put" && equal(value, change.value) || change.op === "delete" && value === void 0) continue;
          if (change.expectedValue === null ? value !== void 0 : !equal(value, change.expectedValue)) throw new Error("Character migration source changed: " + collection + "/" + change.key);
          if (change.op === "put") preferences.set(key, change.value);
          else preferences.remove(key);
        }
      });
      return;
    }
    const mutations = [];
    for (const change of changes) {
      const entry = yield storage.memory.get(collection, change.key);
      if (change.op === "put" && entry !== null && equal(entry.value, change.value) || change.op === "delete" && entry === null) continue;
      if (change.expectedValue === null ? entry !== null : entry === null || !equal(entry.value, change.expectedValue)) throw new Error("Character memory migration source changed: " + change.key);
      mutations.push(change.op === "put" ? { op: "put", collection, key: change.key, value: change.value, expectedVersion: entry === null ? null : entry.version } : { op: "delete", collection, key: change.key, expectedVersion: entry === null ? null : entry.version });
    }
    if (mutations.length !== 0) yield storage.memory.commit(mutations);
  });
}
function migrateLegacyToV1(storage, preferences, directory, source) {
  return __async(this, null, function* () {
    const metadata = clone(source);
    let plan;
    const persisted = (yield storage.migration.data()).get("plan");
    if (persisted === void 0) plan = yield prepareVersionOne(storage, metadata, directory);
    else {
      assertMigrationPlan(persisted);
      plan = clone(persisted);
    }
    let previous = persisted === void 0 ? void 0 : storedValue(persisted);
    function publish() {
      return __async(this, null, function* () {
        yield storage.migration.edit((work) => {
          if (!equal(work.get("plan"), previous)) throw new Error("Character migration plan changed concurrently");
          work.set("plan", storedValue(clone(plan)));
        });
        previous = storedValue(clone(plan));
      });
    }
    if (persisted === void 0) yield publish();
    while (true) {
      switch (plan.phase) {
        case "files":
          for (; plan.cursor < plan.files.length; ) {
            const file = plan.files[plan.cursor], destination = absolute(directory, file.destination);
            const exists = yield Tools.Files.exists(destination);
            if (!exists.exists) {
              completed(yield Tools.Files.mkdir(destination.slice(0, destination.lastIndexOf("/")), true));
              completed(yield Tools.Files.copy(absolute(directory, file.source), destination, false));
            } else yield verifyFile(directory, file);
            plan.cursor += 1;
            yield publish();
          }
          for (const move of plan.directories) {
            const document2 = absolute(directory, move.destination + "/USER.md");
            if (!(yield Tools.Files.exists(document2)).exists) {
              completed(yield Tools.Files.mkdir(document2.slice(0, document2.lastIndexOf("/")), true));
              completed(yield Tools.Files.write(document2, ""));
            }
          }
          plan.phase = "authors";
          plan.cursor = 0;
          yield publish();
          break;
        case "authors":
          for (; plan.cursor < plan.authors.length; ) {
            const author = plan.authors[plan.cursor], { extension } = yield Tools.Chat.readExtension(author.target);
            if (extension === null || Object.keys(extension).length !== 1 || extension.participantId !== author.before && extension.participantId !== author.after)
              throw new Error("Message author changed during character migration");
            if (extension.participantId === author.before) yield Tools.Chat.writeExtension(author.target, { participantId: author.after });
            plan.cursor += 1;
            yield publish();
          }
          plan.phase = "records";
          plan.cursor = 0;
          yield publish();
          break;
        case "records":
          while (plan.cursor < plan.changes.length) {
            const batch = [], collection = plan.changes[plan.cursor][0].collection;
            let cursor = plan.cursor;
            while (cursor < plan.changes.length && plan.changes[cursor][0].collection === collection && batch.length + plan.changes[cursor].length <= 1e3) {
              batch.push(...plan.changes[cursor]);
              cursor += 1;
            }
            yield applyChanges(storage, batch);
            plan.cursor = cursor;
            yield publish();
          }
          plan.phase = "cleanup";
          plan.cursor = 0;
          yield publish();
          break;
        // Retains the persisted phase name so interrupted released migrations can continue.
        case "verify":
          plan.phase = "cleanup";
          plan.cursor = 0;
          yield publish();
          break;
        case "cleanup":
          for (; plan.cursor < plan.directories.length; ) {
            const source2 = absolute(directory, plan.directories[plan.cursor].source);
            if ((yield Tools.Files.exists(source2)).exists) {
              completed(yield Tools.Files.deleteFile(source2, true));
            }
            plan.cursor += 1;
            yield publish();
          }
          metadata.version = 1;
          metadata.active = plan.active;
          delete metadata.legacyImportPending;
          preferences.set("state", metadata);
          return;
        default:
          throw new Error("Invalid character migration phase");
      }
    }
  });
}

// src/storage/schema.ts
function migrateCharacterSchema(storage, preferences, directory) {
  return __async(this, null, function* () {
    const metadata = preferences.get("state");
    if (metadata === void 0) return;
    assertObject(metadata, "character schema metadata");
    if (metadata.migrationStatus === "writing") throw new Error("Character migration has not completed: legacy import is still writing");
    if (metadata.version !== 1) throw new Error("Unsupported character schema version: " + metadata.version);
    yield migrateLegacyToV1(storage, preferences, directory, metadata);
  });
}

// src/storage/memory-merge.json
var memory_merge_default = {
  version: 4,
  maxEntities: 1,
  maxReferences: 48,
  entities: [
    {
      collection: "owners",
      identityColumn: "ownerKey",
      nameColumn: null,
      ignoredColumns: [],
      keyEncoding: "slash_root",
      matchMode: "identity",
      keyReferences: [
        {
          prefix: "character:",
          target: {
            kind: "preference",
            path: "runtime/plugin_data/space/com.operit.character_cards/cards.preferences.json"
          }
        },
        {
          prefix: "shared:",
          target: {
            kind: "preference",
            path: "runtime/plugin_data/space/com.operit.character_cards/stores.preferences.json"
          }
        }
      ],
      childCollections: [
        "memories",
        "links",
        "chunks",
        "candidates",
        "embeddings"
      ],
      references: [
        {
          collection: "owners",
          column: "ownerKey",
          target: {
            kind: "preference",
            path: "runtime/plugin_data/space/com.operit.character_cards/cards.preferences.json"
          },
          keyPrefix: "character:",
          keySuffix: "",
          valueEncoding: "direct"
        },
        {
          collection: "owners",
          column: "ownerKey",
          target: {
            kind: "preference",
            path: "runtime/plugin_data/space/com.operit.character_cards/stores.preferences.json"
          },
          keyPrefix: "shared:",
          keySuffix: "",
          valueEncoding: "direct"
        },
        {
          collection: "owners",
          column: "userDocumentPath",
          target: {
            kind: "collection",
            collection: "owners"
          },
          keyPrefix: "owners/",
          keySuffix: "/USER.md",
          valueEncoding: "uri_component"
        },
        {
          collection: "memories",
          column: "ownerKey",
          target: {
            kind: "preference",
            path: "runtime/plugin_data/space/com.operit.character_cards/cards.preferences.json"
          },
          keyPrefix: "character:",
          keySuffix: "",
          valueEncoding: "direct"
        },
        {
          collection: "memories",
          column: "ownerKey",
          target: {
            kind: "preference",
            path: "runtime/plugin_data/space/com.operit.character_cards/stores.preferences.json"
          },
          keyPrefix: "shared:",
          keySuffix: "",
          valueEncoding: "direct"
        },
        {
          collection: "links",
          column: "ownerKey",
          target: {
            kind: "preference",
            path: "runtime/plugin_data/space/com.operit.character_cards/cards.preferences.json"
          },
          keyPrefix: "character:",
          keySuffix: "",
          valueEncoding: "direct"
        },
        {
          collection: "links",
          column: "ownerKey",
          target: {
            kind: "preference",
            path: "runtime/plugin_data/space/com.operit.character_cards/stores.preferences.json"
          },
          keyPrefix: "shared:",
          keySuffix: "",
          valueEncoding: "direct"
        },
        {
          collection: "chunks",
          column: "ownerKey",
          target: {
            kind: "preference",
            path: "runtime/plugin_data/space/com.operit.character_cards/cards.preferences.json"
          },
          keyPrefix: "character:",
          keySuffix: "",
          valueEncoding: "direct"
        },
        {
          collection: "chunks",
          column: "ownerKey",
          target: {
            kind: "preference",
            path: "runtime/plugin_data/space/com.operit.character_cards/stores.preferences.json"
          },
          keyPrefix: "shared:",
          keySuffix: "",
          valueEncoding: "direct"
        },
        {
          collection: "candidates",
          column: "ownerKey",
          target: {
            kind: "preference",
            path: "runtime/plugin_data/space/com.operit.character_cards/cards.preferences.json"
          },
          keyPrefix: "character:",
          keySuffix: "",
          valueEncoding: "direct"
        },
        {
          collection: "candidates",
          column: "ownerKey",
          target: {
            kind: "preference",
            path: "runtime/plugin_data/space/com.operit.character_cards/stores.preferences.json"
          },
          keyPrefix: "shared:",
          keySuffix: "",
          valueEncoding: "direct"
        },
        {
          collection: "embeddings",
          column: "ownerKey",
          target: {
            kind: "preference",
            path: "runtime/plugin_data/space/com.operit.character_cards/cards.preferences.json"
          },
          keyPrefix: "character:",
          keySuffix: "",
          valueEncoding: "direct"
        },
        {
          collection: "embeddings",
          column: "ownerKey",
          target: {
            kind: "preference",
            path: "runtime/plugin_data/space/com.operit.character_cards/stores.preferences.json"
          },
          keyPrefix: "shared:",
          keySuffix: "",
          valueEncoding: "direct"
        }
      ]
    }
  ]
};

// src/storage/memory.ts
var characterMerge = memory_merge_default;
var CharacterMemoryDatabase = class _CharacterMemoryDatabase {
  /** Retains one authenticated object handle without declaring business SQL tables. */
  constructor(database2) {
    this.database = database2;
  }
  /** Opens the sole synchronized character namespace independently of plugin installation scope. */
  static open() {
    return __async(this, null, function* () {
      const directory = ToolPkg.getSpaceDataDir();
      if (typeof directory !== "string" || directory.trim() === "")
        throw new Error("Plugin data directory is unavailable");
      const database2 = new _CharacterMemoryDatabase(yield Tools.Storage.objects.open({ path: directory + "/memory.sqlite", merge: characterMerge }));
      return database2;
    });
  }
  /** Closes the retained memory engine handle. */
  close() {
    return this.database.close();
  }
  /** Reads complete JSON values in primary-key order using bounded host pages. */
  read(collection) {
    return __async(this, null, function* () {
      const result = [];
      const source = this.database.collection(collection);
      let after = null;
      for (; ; ) {
        const page = yield source.list(after, 1e3);
        result.push(...page);
        if (page.length < 1e3) return result;
        after = page[page.length - 1].key;
      }
    });
  }
  /** Reads one exact document and its host-issued concurrency version. */
  get(collection, key) {
    return this.database.collection(collection).get(key);
  }
  /** Reads selected identities in one host snapshot without scanning their collections. */
  getMany(collection, keys) {
    return this.database.collection(collection).getMany([...keys]);
  }
  /** Publishes complete changed JSON documents and memory objects in one guarded transaction. */
  commit(mutations) {
    return this.database.commit(mutations);
  }
};

// src/storage/preferences.ts
var merge = merge_default;
var configurationNames = ["cards", "groups", "tags", "stores", "conversationGroups"];
var recordPrefix = "record_";
var CharacterStorage = class _CharacterStorage {
  /** Retains explicit storage owners without emulating a cross-engine transaction. */
  constructor(configuration, metadata, chats, migration, memory) {
    this.configuration = configuration;
    this.metadata = metadata;
    this.chats = chats;
    this.migration = migration;
    this.memory = memory;
  }
  /** Opens JSON configuration owners and upgrades the plugin schema before business reads. */
  static open() {
    return __async(this, null, function* () {
      const directory = ToolPkg.getSpaceDataDir();
      if (typeof directory !== "string" || directory.trim() === "") throw new Error("Plugin data directory is unavailable");
      const handles = [];
      let memory;
      try {
        function open(name) {
          return __async(this, null, function* () {
            const store = yield Tools.Storage.dataStore.open({ path: directory + "/" + name + ".preferences.json", merge: merge[name] });
            handles.push(store);
            return store;
          });
        }
        const configuration = {};
        for (const name of configurationNames) configuration[name] = yield open(name);
        const metadata = yield open("meta"), chats = yield open("chats");
        const migration = yield Tools.Storage.dataStore.open({ path: directory + "/migration.preferences.json" });
        handles.push(migration);
        memory = yield CharacterMemoryDatabase.open();
        const storage = new _CharacterStorage(configuration, metadata, chats, migration, memory);
        metadata.withSchema(1, (version, preferences) => __async(null, null, function* () {
          switch (version) {
            case 0:
              yield migrateCharacterSchema(storage, preferences, directory);
              break;
            default:
              throw new Error("Missing character schema migration: " + version);
          }
        }));
        const current = yield metadata.data();
        const state = current.get("state");
        if (state !== void 0 && state.legacyImportPending === true) yield metadata.edit((preferences) => migrateCharacterSchema(storage, preferences, directory));
        if ((yield migration.data()).contains("plan")) yield migration.edit((preferences) => {
          preferences.remove("plan");
        });
        return storage;
      } catch (error) {
        const closed = yield Promise.allSettled([...handles.map((handle) => handle.close()), ...memory === void 0 ? [] : [memory.close()]]);
        const failures = closed.filter((result) => result.status === "rejected").map((result) => result.reason);
        if (failures.length !== 0) throw Object.assign(new Error("Character storage open and handle cleanup failed"), { cause: error, cleanupErrors: failures });
        throw error;
      }
    });
  }
  /** Reads each configuration owner's immutable snapshot without loading memory objects. */
  data() {
    return __async(this, null, function* () {
      const result = {};
      for (const name of configurationNames) result[name] = yield this.configuration[name].data();
      return result;
    });
  }
};

// src/chat-storage.ts
var database = null;
function chatDatabase() {
  if (database === null) database = CharacterStorage.open();
  return database;
}
function findChatBinding(chatId) {
  return __async(this, null, function* () {
    requireId(chatId, "chat id");
    const value = (yield (yield chatDatabase()).chats.data()).get("binding_" + chatId);
    return value === void 0 ? null : { chatId, selection: decodeChatMarker(value).selection };
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
function chatIdentity(value, label) {
  if (typeof value !== "string" || value.trim() === "") throw new Error(label + " must be a nonblank string");
  return value;
}

// src/ui-memory-menu.ts
function request(input) {
  return ToolPkg.ipc.call("character-memory.request", input, { targetRuntime: "main" });
}
function memorySummary(preferences) {
  if (preferences.userDescriptionEnabled) return preferences.autoUpdateEnabled ? "\u7528\u6237\u8D44\u6599 \xB7 \u8BB0\u5FC6\u5E93\u66F4\u65B0" : "\u7528\u6237\u8D44\u6599";
  return preferences.autoUpdateEnabled ? "\u8BB0\u5FC6\u5E93\u66F4\u65B0" : "\u5173";
}
function memoryQueueDescription(queue) {
  if (queue === null) return "\u6682\u65E0\u961F\u5217\u72B6\u6001\u3002\u81EA\u52A8\u63D0\u53D6\u4F1A\u5B9A\u671F\u68C0\u67E5\u5F53\u524D\u8BB0\u5FC6\u5E93\u3002";
  return `\u5F85\u5904\u7406 ${queue.pendingCandidates} \u6761 \xB7 ${queue.pendingChats} \u4E2A\u804A\u5929
\u5904\u7406\u4E2D ${queue.processingCandidates} \xB7 \u5931\u8D25 ${queue.failedCandidates}
\u4E0B\u6B21\u68C0\u67E5\uFF1A\u7EA6 ${queue.minutesUntilNextRun} \u5206\u949F\u540E

\u81EA\u52A8\u68C0\u67E5\u9700\u8981\u81F3\u5C11 5 \u4E2A\u5019\u9009\uFF1B\u4E0D\u8DB3\u65F6\u7EE7\u7EED\u7B49\u5F85\u3002\u624B\u52A8\u66F4\u65B0\u4E0D\u53D7\u6B64\u95E8\u69DB\u9650\u5236\u3002` + (queue.lastError === "" ? "" : "\n\n" + queue.lastError);
}
function renderMemoryMenu(ctx) {
  const [chatId] = ctx.useState("chatId", null);
  if (chatId !== null) chatIdentity(chatId, "memory menu chatId");
  const owner2 = ctx.useRef("memory-menu-chat", chatId);
  const [available, setAvailable] = ctx.useState("memory-menu-available", null);
  const [preferences, setPreferences] = ctx.useState("memory-menu-preferences", null);
  const [, setQueue] = ctx.useState("memory-menu-queue", null);
  const [expanded, setExpanded] = ctx.useState("memory-menu-expanded", false);
  const [busy, setBusy] = ctx.useState("memory-menu-busy", false);
  const liveBusy = ctx.useRef("memory-menu-busy-live", false);
  const started = ctx.useRef("memory-menu-started", false);
  const colors = ctx.MaterialTheme.colorScheme;
  function assertOwner() {
    const [current] = ctx.useState("chatId", null);
    if (current !== owner2.current) throw new Error("Memory menu chat owner changed");
  }
  assertOwner();
  function refreshQueue() {
    return __async(this, null, function* () {
      assertOwner();
      if (chatId === null) return;
      const value = yield request({ action: "readChatMemoryAutoSaveStatus", chatId });
      assertOwner();
      setQueue(value);
    });
  }
  function load() {
    return __async(this, null, function* () {
      assertOwner();
      if (started.current) return;
      started.current = true;
      if (chatId !== null) {
        const binding = yield findChatBinding(chatId);
        assertOwner();
        const hasCharacter = binding !== null && parseChatSelection(binding.selection).kind === "card";
        setAvailable(hasCharacter);
        if (!hasCharacter) return;
      }
      const value = yield request({ action: "readGlobalMemoryPreferences" });
      assertOwner();
      setPreferences(value);
      yield refreshQueue();
    });
  }
  function showInfo() {
    return __async(this, null, function* () {
      yield refreshQueue();
      assertOwner();
      const [current] = ctx.useState("memory-menu-queue", null);
      return { type: "toolpkg.ui.present", routeId: `toolpkg:${definition.id}:ui:memory-status`, input: { description: memoryQueueDescription(current) } };
    });
  }
  function toggle(key) {
    return __async(this, null, function* () {
      assertOwner();
      const [current] = ctx.useState("memory-menu-preferences", null);
      if (current === null) throw new Error("Memory preferences are not loaded");
      const value = yield request({ action: "writeGlobalMemoryPreference", key, value: !current[key] });
      assertOwner();
      setPreferences(value);
    });
  }
  function manual() {
    return __async(this, null, function* () {
      assertOwner();
      if (liveBusy.current || chatId === null) return null;
      liveBusy.current = true;
      setBusy(true);
      try {
        yield request({ action: "updateCurrentChatMemory", chatId });
        assertOwner();
        yield refreshQueue();
        return { type: "toolpkg.ui.message", message: "\u8BB0\u5FC6\u63D0\u53D6\u5B8C\u6210" };
      } catch (error) {
        assertOwner();
        return { type: "toolpkg.ui.message", message: "\u8BB0\u5FC6\u63D0\u53D6\u5931\u8D25\uFF1A" + String(error) };
      } finally {
        liveBusy.current = false;
        assertOwner();
        setBusy(false);
      }
    });
  }
  function icon(name, color = colors.onSurfaceVariant) {
    return OperitComposeComposition.evaluate(ctx,"com.operit.character_cards:dist/ui/chat-input-memory/index.ui.js:68751",__operitUiPrograms_[0],[name,color]);
  }
  function switchRow(key, name, title) {
    if (preferences === null) throw new Error("Memory preferences are not loaded");
    const checked = preferences[key];
    return OperitComposeComposition.evaluate(ctx,"com.operit.character_cards:dist/ui/chat-input-memory/index.ui.js:68975",__operitUiPrograms_[1],["memory-" + key,({__modifierOps:OperitComposeNodes.modifierBase(ctx.Modifier).concat([{name:"heightIn",args:[{ minHeight: 32 }]}])}),() => toggle(key),icon(name, checked ? colors.primary : colors.onSurfaceVariant),title,colors.onSurface,checked ? "\u5F00" : "\u5173",colors.primary,checked,() => toggle(key)]);
  }
  if (available === false) return OperitComposeComposition.evaluate(ctx,"com.operit.character_cards:dist/ui/chat-input-memory/index.ui.js:69734",__operitUiPrograms_[2],[]);
  if (preferences === null) return OperitComposeComposition.evaluate(ctx,"com.operit.character_cards:dist/ui/chat-input-memory/index.ui.js:69817",__operitUiPrograms_[3],[load]);
  const children = [OperitComposeComposition.evaluate(ctx,"com.operit.character_cards:dist/ui/chat-input-memory/index.ui.js:69895",__operitUiPrograms_[4],[({__modifierOps:OperitComposeNodes.modifierBase(ctx.Modifier).concat([{name:"heightIn",args:[{ minHeight: 36 }]}])}),() => {
      assertOwner();
      setExpanded(!expanded);
    },icon("DataObjectOutlined", colors.onSurfaceVariant.copy({ alpha: 0.7 })),memorySummary(preferences),colors.primary,showInfo,colors.onSurfaceVariant.copy({ alpha: 0.7 }),expanded ? "KeyboardArrowUp" : "KeyboardArrowDown",colors.onSurfaceVariant])];
  if (expanded) children.push(OperitComposeComposition.evaluate(ctx,"com.operit.character_cards:dist/ui/chat-input-memory/index.ui.js:71086",__operitUiPrograms_[5],[({__modifierOps:OperitComposeNodes.modifierBase(ctx.Modifier).concat([{name:"padding",args:[12,0,8,4]}])}),switchRow("userDescriptionEnabled", "AssignmentIndOutlined", "\u63D0\u4F9B\u7528\u6237\u8D44\u6599"),switchRow("autoUpdateEnabled", preferences.autoUpdateEnabled ? "Save" : "SaveOutlined", "\u81EA\u52A8\u66F4\u65B0\u8BB0\u5FC6\u5E93"),({__modifierOps:OperitComposeNodes.modifierBase(ctx.Modifier).concat([{name:"heightIn",args:[{ minHeight: 32 }]}])}),!busy && chatId !== null ? manual : void 0,icon("SaveOutlined"),busy ? "\u6B63\u5728\u66F4\u65B0\u8BB0\u5FC6\u2026" : "\u624B\u52A8\u66F4\u65B0\u8BB0\u5FC6",!busy && chatId !== null ? colors.onSurface : colors.onSurfaceVariant.copy({ alpha: 0.45 })]));
  return OperitComposeComposition.evaluate(ctx,"com.operit.character_cards:dist/ui/chat-input-memory/index.ui.js:72117",__operitUiPrograms_[6],[children]);
}

// src/ui/chat-input-memory/index.ui.ts
function Screen(ctx) {
  return renderMemoryMenu(ctx);
}

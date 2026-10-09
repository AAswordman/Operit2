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

// src/main.ts
var main_exports = {};
__export(main_exports, {
  activePromptActivateForChatApi: () => activePromptActivateForChatApi,
  activePromptGetApi: () => activePromptGetApi,
  activePromptResolvedCardApi: () => activePromptResolvedCardApi,
  activePromptSetCardApi: () => activePromptSetCardApi,
  activePromptSetGroupApi: () => activePromptSetGroupApi,
  attachmentScreen: () => attachmentScreen,
  beforeChatCreate: () => beforeChatCreate,
  characterCombineApi: () => characterCombineApi,
  characterCreateApi: () => characterCreateApi,
  characterDeleteApi: () => characterDeleteApi,
  characterExportApi: () => characterExportApi,
  characterExportBackupApi: () => characterExportBackupApi,
  characterGetApi: () => characterGetApi,
  characterImportApi: () => characterImportApi,
  characterImportBackupApi: () => characterImportBackupApi,
  characterListApi: () => characterListApi,
  characterResetDefaultApi: () => characterResetDefaultApi,
  characterSetActiveApi: () => characterSetActiveApi,
  characterUpdateApi: () => characterUpdateApi,
  chatConfigurationBindingDeleteApi: () => chatConfigurationBindingDeleteApi,
  chatConfigurationBindingReadApi: () => chatConfigurationBindingReadApi,
  chatConfigurationBindingWriteApi: () => chatConfigurationBindingWriteApi,
  chatConfigurationResolveApi: () => chatConfigurationResolveApi,
  chatContextActionsApi: () => chatContextActionsApi,
  chatListSectionsApi: () => chatListSectionsApi,
  conversationGroupCreateApi: () => conversationGroupCreateApi,
  conversationGroupDeleteApi: () => conversationGroupDeleteApi,
  conversationGroupListApi: () => conversationGroupListApi,
  conversationGroupMoveChatApi: () => conversationGroupMoveChatApi,
  conversationGroupReorderApi: () => conversationGroupReorderApi,
  conversationGroupUpdateApi: () => conversationGroupUpdateApi,
  groupCreateApi: () => groupCreateApi,
  groupDeleteApi: () => groupDeleteApi,
  groupDuplicateApi: () => groupDuplicateApi,
  groupExecutionScreen: () => groupExecutionScreen,
  groupExportApi: () => groupExportApi,
  groupExportBackupApi: () => groupExportBackupApi,
  groupGetApi: () => groupGetApi,
  groupImportApi: () => groupImportApi,
  groupImportBackupApi: () => groupImportBackupApi,
  groupListApi: () => groupListApi,
  groupSetActiveApi: () => groupSetActiveApi,
  groupUpdateApi: () => groupUpdateApi,
  memoryCandidateEnqueueApi: () => memoryCandidateEnqueueApi,
  memoryCategorizeApi: () => memoryCategorizeApi,
  memoryChatListApi: () => memoryChatListApi,
  memoryChatUpdateApi: () => memoryChatUpdateApi,
  memoryCreateApi: () => memoryCreateApi,
  memoryDeleteApi: () => memoryDeleteApi,
  memoryEmbeddingsRebuildApi: () => memoryEmbeddingsRebuildApi,
  memoryExportApi: () => memoryExportApi,
  memoryGetApi: () => memoryGetApi,
  memoryGraphApi: () => memoryGraphApi,
  memoryImportApi: () => memoryImportApi,
  memoryLinkCreateApi: () => memoryLinkCreateApi,
  memoryLinkDeleteApi: () => memoryLinkDeleteApi,
  memoryLinkUpdateApi: () => memoryLinkUpdateApi,
  memoryListApi: () => memoryListApi,
  memoryMountApi: () => memoryMountApi,
  memoryMoveApi: () => memoryMoveApi,
  memoryRebuildCancelApi: () => memoryRebuildCancelApi,
  memoryRebuildProgressApi: () => memoryRebuildProgressApi,
  memoryRebuildStartApi: () => memoryRebuildStartApi,
  memoryResolveOwnerApi: () => memoryResolveOwnerApi,
  memorySearchApi: () => memorySearchApi,
  memorySearchConfigReadApi: () => memorySearchConfigReadApi,
  memorySearchConfigWriteApi: () => memorySearchConfigWriteApi,
  memorySearchWithOptionsApi: () => memorySearchWithOptionsApi,
  memorySettingsReadApi: () => memorySettingsReadApi,
  memorySettingsWriteApi: () => memorySettingsWriteApi,
  memorySharedCreateApi: () => memorySharedCreateApi,
  memorySharedDeleteApi: () => memorySharedDeleteApi,
  memorySharedListApi: () => memorySharedListApi,
  memorySharedRenameApi: () => memorySharedRenameApi,
  memoryUnmountApi: () => memoryUnmountApi,
  memoryUpdateApi: () => memoryUpdateApi,
  memoryUserPathApi: () => memoryUserPathApi,
  memoryUserReadApi: () => memoryUserReadApi,
  memoryUserWriteApi: () => memoryUserWriteApi,
  onActivePromptCommand: () => onActivePromptCommand,
  onCharacterCommand: () => onCharacterCommand,
  onGroupCommand: () => onGroupCommand,
  onGroupInputSubmit: () => onGroupInputSubmit,
  onMemoryCommand: () => onMemoryCommand,
  onMemoryInterval: () => onMemoryInterval,
  onMemoryMessagePersisted: () => onMemoryMessagePersisted,
  onServiceInitialize: () => onServiceInitialize,
  onTagCommand: () => onTagCommand,
  parseActivePromptCommand: () => parseActivePromptCommand,
  parseCharacterCommand: () => parseCharacterCommand,
  parseGroupCommand: () => parseGroupCommand,
  parseMemoryCommand: () => parseMemoryCommand,
  parseTagCommand: () => parseTagCommand,
  registerChatConfigurationApis: () => registerChatConfigurationApis,
  registerDomainApis: () => registerDomainApis,
  registerDomainCommands: () => registerDomainCommands,
  registerMemoryJobHooks: () => registerMemoryJobHooks,
  registerToolPkg: () => registerToolPkg,
  screen: () => screen,
  selectionScreen: () => selectionScreen,
  sidebarScreen: () => sidebarScreen,
  snapshotApi: () => snapshotApi,
  tagCreateApi: () => tagCreateApi,
  tagDeleteApi: () => tagDeleteApi,
  tagGetApi: () => tagGetApi,
  tagListApi: () => tagListApi,
  tagUpdateApi: () => tagUpdateApi,
  toolCallPolicy: () => toolCallPolicy,
  toolPromptPolicy: () => toolPromptPolicy
});
module.exports = __toCommonJS(main_exports);

// src/presentation.ts
function fields(value, keys, path) {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new Error(`${path} must be an object`);
  const object3 = value, actual = Object.keys(object3);
  if (actual.length !== keys.length || actual.some(
    /** Compares exact field identities rather than interpreting string fragments. */
    (key) => keys.indexOf(key) < 0
  )) throw new Error(`${path} must have exactly ${keys.join(", ")}`);
  return object3;
}
function text(value, path) {
  if (typeof value !== "string") throw new Error(`${path} must be a string`);
  return value;
}
function identity(value, path) {
  const result2 = text(value, path);
  if (result2.trim() === "") throw new Error(`${path} must not be blank`);
  return result2;
}
function entityKind(value) {
  if (value !== "card" && value !== "group") throw new Error("screen.entity must be card or group");
  return value;
}
function selection(value) {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new Error("screen.selection must be an active prompt object");
  const keys = Object.keys(value);
  if (keys.length !== 1) throw new Error("screen.selection must have one active prompt discriminator");
  const object3 = value;
  switch (keys[0]) {
    case "CharacterCard":
      return { CharacterCard: { id: identity(fields(object3.CharacterCard, ["id"], "screen.selection.CharacterCard").id, "screen.selection.id") } };
    case "CharacterGroup":
      return { CharacterGroup: { id: identity(fields(object3.CharacterGroup, ["id"], "screen.selection.CharacterGroup").id, "screen.selection.id") } };
    default:
      throw new Error("screen.selection has an unknown active prompt discriminator");
  }
}
function parseScreenInput(value) {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new Error("screen.input must be an object");
  const object3 = value;
  switch (object3.mode) {
    case "manage":
      fields(object3, ["mode"], "screen.input");
      return { mode: "manage" };
    case "select": {
      fields(object3, ["mode", "kind", "selected"], "screen.input");
      if (object3.kind !== "card" && object3.kind !== "group" && object3.kind !== "all") throw new Error("screen.kind must be card, group, or all");
      const selected = object3.selected === null ? null : selection(object3.selected);
      if (selected !== null && (object3.kind === "card" && !("CharacterCard" in selected) || object3.kind === "group" && !("CharacterGroup" in selected))) throw new Error("screen.selected does not match screen.kind");
      return { mode: "select", kind: object3.kind, selected };
    }
    case "preview":
    case "edit":
      fields(object3, ["mode", "entity", "id"], "screen.input");
      return { mode: object3.mode, entity: entityKind(object3.entity), id: identity(object3.id, "screen.id") };
    case "memory-attachment": {
      fields(object3, ["mode", "ownerKey", "folderPath"], "screen.input");
      const ownerKey = object3.ownerKey === null ? null : identity(object3.ownerKey, "screen.ownerKey");
      const folderPath = object3.folderPath === null ? null : text(object3.folderPath, "screen.folderPath");
      if (ownerKey === null && folderPath !== null) throw new Error("screen.folderPath requires an explicitly selected ownerKey");
      return { mode: "memory-attachment", ownerKey, folderPath };
    }
    default:
      throw new Error("screen.input has an unknown mode");
  }
}
function parseScreenResult(value, input) {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new Error("screen.result must be an object");
  const object3 = value;
  if (object3.mode !== input.mode) throw new Error("screen.result.mode does not match the active presentation");
  switch (input.mode) {
    case "select": {
      fields(object3, ["mode", "selection"], "screen.result");
      const result2 = selection(object3.selection);
      if (input.kind === "card" && !("CharacterCard" in result2) || input.kind === "group" && !("CharacterGroup" in result2)) throw new Error("screen.result.selection does not match screen.kind");
      return { mode: "select", selection: result2 };
    }
    case "preview":
    case "edit": {
      fields(object3, input.mode === "edit" ? ["mode", "entity", "id", "operation"] : ["mode", "entity", "id"], "screen.result");
      if (object3.entity !== input.entity || object3.id !== input.id) throw new Error("screen.result does not match the presented entity");
      if (input.mode === "preview") return { mode: "preview", entity: input.entity, id: input.id };
      if (object3.operation !== "saved" && object3.operation !== "deleted") throw new Error("screen.result.operation must be saved or deleted");
      return { mode: "edit", entity: input.entity, id: input.id, operation: object3.operation };
    }
    case "memory-attachment": {
      fields(object3, ["mode", "ownerKey", "folderPath", "content"], "screen.result");
      const ownerKey = identity(object3.ownerKey, "screen.result.ownerKey"), folderPath = text(object3.folderPath, "screen.result.folderPath");
      if (input.ownerKey !== null && ownerKey !== input.ownerKey || input.folderPath !== null && folderPath !== input.folderPath) throw new Error("screen.result does not match the presented attachment scope");
      return { mode: "memory-attachment", ownerKey, folderPath, content: text(object3.content, "screen.result.content") };
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
      const active = activeRequest(), result2 = parseScreenResult(value, active.input);
      finished = true;
      return { type: "toolpkg.presentation.complete", requestId: active.requestId, value: result2 };
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
function createTagDraft() {
  return { name: "", description: "", promptContent: "", tagType: "CUSTOM" };
}

// src/validation.ts
function requireDecimal(value, label, positive = false) {
  if (typeof value !== "string" || !/^(0|[1-9][0-9]*)$/.test(value)) throw new Error(`${label} must be a canonical decimal string`);
  const integer3 = BigInt(value);
  if (integer3 > 9223372036854775807n || positive && integer3 === 0n) throw new Error(`${label} is outside its i64 range`);
  return value;
}
function assertObject(value, label) {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new Error(`${label} \u5FC5\u987B\u662F\u5BF9\u8C61`);
}
function assertString(value, label) {
  if (typeof value !== "string") throw new Error(`${label} \u5FC5\u987B\u662F\u5B57\u7B26\u4E32`);
}
function assertBoolean(value, label) {
  if (typeof value !== "boolean") throw new Error(`${label} \u5FC5\u987B\u662F\u5E03\u5C14\u503C`);
}
function assertNumber(value, label, min, max) {
  if (typeof value !== "number" || !Number.isFinite(value) || value < min || value > max) throw new Error(`${label} \u5FC5\u987B\u5728 ${min} \u5230 ${max} \u4E4B\u95F4`);
}
function assertInteger(value, label, min) {
  assertNumber(value, label, min, Number.MAX_SAFE_INTEGER);
  if (!Number.isSafeInteger(value)) throw new Error(`${label} \u5FC5\u987B\u662F\u5B89\u5168\u6574\u6570`);
}
function assertNullableString(value, label) {
  if (value !== null) assertString(value, label);
}
function assertStrings(value, label) {
  if (!Array.isArray(value)) throw new Error(`${label} \u5FC5\u987B\u662F\u5B57\u7B26\u4E32\u6570\u7EC4`);
  for (const item of value) assertString(item, label);
}
function requireId(value, label) {
  assertString(value, label);
  if (value.trim() === "" || value !== value.trim()) throw new Error(`${label} \u65E0\u6548`);
  return value;
}
function requireName(value, label) {
  assertString(value, label);
  const name = value.trim();
  if (name === "") throw new Error(`${label} \u4E0D\u80FD\u4E3A\u7A7A`);
  return name;
}
function assertConversationGroupScope(value) {
  if (value !== null) requireId(value, "conversation group scope");
}
function assertConversationGroup(value) {
  assertObject(value, "conversation group");
  requireDecimal(value.id, "conversation group id", true);
  assertConversationGroupScope(value.ownerSelection);
  const name = requireName(value.name, "conversation group name");
  if (name !== value.name) throw new Error("Conversation group names must be stored in their trimmed form");
  assertBoolean(value.pinned, "conversation group pinned");
  assertStrings(value.chatIds, "conversation group chats");
  for (const field of ["displayOrder", "createdAt", "updatedAt"]) assertInteger(value[field], field, 0);
  const chats = /* @__PURE__ */ new Set();
  for (const chatId of value.chatIds) {
    requireId(chatId, "conversation group chat id");
    if (chats.has(chatId)) throw new Error("Duplicate chat within a conversation group: " + chatId);
    chats.add(chatId);
  }
}
function assertConversationGroups(value) {
  if (!Array.isArray(value)) throw new Error("conversationGroups must be an array");
  const ids = /* @__PURE__ */ new Set(), names = /* @__PURE__ */ new Set(), orders = /* @__PURE__ */ new Set(), chats = /* @__PURE__ */ new Set();
  const scopeCounts = /* @__PURE__ */ new Map();
  for (const group of value) {
    assertConversationGroup(group);
    if (ids.has(group.id)) throw new Error("Duplicate conversation group id: " + group.id);
    ids.add(group.id);
    const nameKey = JSON.stringify([group.ownerSelection, group.name]);
    if (names.has(nameKey)) throw new Error("Conversation group name already exists in this scope: " + group.name);
    names.add(nameKey);
    const orderKey = JSON.stringify([group.ownerSelection, group.displayOrder]);
    if (orders.has(orderKey)) throw new Error("Duplicate conversation group order in one scope");
    orders.add(orderKey);
    const count = scopeCounts.get(group.ownerSelection);
    scopeCounts.set(group.ownerSelection, count === void 0 ? 1 : count + 1);
    for (const chatId of group.chatIds) {
      if (chats.has(chatId)) throw new Error("Chat belongs to more than one conversation group: " + chatId);
      chats.add(chatId);
    }
  }
  for (const group of value) {
    assertConversationGroup(group);
    const count = scopeCounts.get(group.ownerSelection);
    if (count === void 0 || group.displayOrder >= count) throw new Error("Conversation group order must be a complete zero-based scoped sequence");
  }
}
function assertConversationGroupChanges(value) {
  assertObject(value, "conversation group changes");
  const keys = Object.keys(value);
  if (keys.length === 0) throw new Error("Conversation group changes must contain at least one field");
  for (const key of keys) {
    switch (key) {
      case "name":
        requireName(value.name, "conversation group name");
        break;
      case "pinned":
        assertBoolean(value.pinned, "conversation group pinned");
        break;
      default:
        throw new Error("Unsupported conversation group edit: " + key);
    }
  }
}
function assertToolAccess(value) {
  assertObject(value, "\u5DE5\u5177\u8BBF\u95EE\u914D\u7F6E");
  assertBoolean(value.enabled, "\u5DE5\u5177\u8BBF\u95EE\u5F00\u5173");
  for (const field of ["allowedBuiltinTools", "allowedPackages", "allowedSkills", "allowedMcpServers"]) assertStrings(value[field], field);
}
function assertCard(value) {
  assertObject(value, "\u89D2\u8272\u5361");
  for (const field of ["id", "name", "description", "characterSetting", "openingStatement", "otherContentChat", "otherContentVoice", "advancedCustomPrompt", "marks", "chatModelBindingMode", "memoryBindingMode"]) assertString(value[field], field);
  for (const field of ["avatarUri", "chatModelId", "ttsConfigId", "themeConfigId", "sharedMemoryId"]) assertNullableString(value[field], field);
  if (value.themeConfigId !== null) requireId(value.themeConfigId, "\u89D2\u8272\u4E3B\u9898\u914D\u7F6E");
  assertStrings(value.attachedTagIds, "\u89D2\u8272\u6807\u7B7E");
  assertBoolean(value.isDefault, "\u9ED8\u8BA4\u89D2\u8272\u6807\u8BC6");
  assertInteger(value.createdAt, "\u89D2\u8272\u521B\u5EFA\u65F6\u95F4", 0);
  assertInteger(value.updatedAt, "\u89D2\u8272\u4FEE\u6539\u65F6\u95F4", 0);
  assertToolAccess(value.toolAccessConfig);
  if (!Array.isArray(value.sharedMemoryMounts)) throw new Error("\u5171\u4EAB\u8BB0\u5FC6\u6302\u8F7D\u5FC5\u987B\u662F\u6570\u7EC4");
  for (const mount2 of value.sharedMemoryMounts) {
    assertObject(mount2, "\u5171\u4EAB\u8BB0\u5FC6\u6302\u8F7D");
    requireId(mount2.sharedMemoryId, "\u5171\u4EAB\u8BB0\u5FC6\u6302\u8F7D\u6807\u8BC6");
    assertBoolean(mount2.readable, "\u5171\u4EAB\u8BB0\u5FC6\u8BFB\u53D6\u5F00\u5173");
    assertBoolean(mount2.writable, "\u5171\u4EAB\u8BB0\u5FC6\u5199\u5165\u5F00\u5173");
  }
}
function assertGroupValues(value) {
  assertObject(value, "\u89D2\u8272\u7FA4\u7EC4");
  for (const field of ["id", "name", "description"]) assertString(value[field], field);
  assertNullableString(value.themeConfigId, "group.themeConfigId");
  if (value.themeConfigId !== null) requireId(value.themeConfigId, "\u7FA4\u7EC4\u4E3B\u9898\u914D\u7F6E");
  if (!Array.isArray(value.members)) throw new Error("\u7FA4\u7EC4\u6210\u5458\u5FC5\u987B\u662F\u6570\u7EC4");
  for (const member2 of value.members) {
    assertObject(member2, "\u7FA4\u7EC4\u6210\u5458");
    requireId(member2.characterCardId, "\u7FA4\u7EC4\u89D2\u8272\u6807\u8BC6");
    assertInteger(member2.orderIndex, "\u7FA4\u7EC4\u6210\u5458\u987A\u5E8F", -2147483648);
    if (member2.orderIndex > 2147483647) throw new Error("\u7FA4\u7EC4\u6210\u5458\u987A\u5E8F\u8D85\u51FA i32 \u8303\u56F4");
  }
}
function assertGroup(value) {
  assertObject(value, "\u89D2\u8272\u7FA4\u7EC4");
  const record2 = value;
  assertGroupValues(value);
  assertInteger(record2.createdAt, "\u7FA4\u7EC4\u521B\u5EFA\u65F6\u95F4", 0);
  assertInteger(record2.updatedAt, "\u7FA4\u7EC4\u4FEE\u6539\u65F6\u95F4", 0);
}
function assertTagValues(value) {
  assertObject(value, "\u6807\u7B7E");
  for (const field of ["name", "description", "promptContent"]) assertString(value[field], field);
  if (value.tagType !== "TONE" && value.tagType !== "CHARACTER" && value.tagType !== "FUNCTION" && value.tagType !== "CUSTOM") throw new Error("\u6807\u7B7E\u7C7B\u578B\u65E0\u6548");
}
function assertTag(value) {
  assertObject(value, "\u6807\u7B7E");
  const record2 = value;
  assertTagValues(value);
  requireId(record2.id, "\u6807\u7B7E\u6807\u8BC6");
  assertInteger(record2.createdAt, "\u6807\u7B7E\u521B\u5EFA\u65F6\u95F4", 0);
  assertInteger(record2.updatedAt, "\u6807\u7B7E\u4FEE\u6539\u65F6\u95F4", 0);
}
function assertMemoryValues(value) {
  assertObject(value, "\u8BB0\u5FC6");
  for (const field of ["title", "content", "contentType", "source"]) assertString(value[field], field);
  assertNumber(value.credibility, "\u53EF\u4FE1\u5EA6", 0, 1);
  assertNumber(value.importance, "\u91CD\u8981\u6027", 0, 1);
  assertNullableString(value.folderPath, "\u8BB0\u5FC6\u6587\u4EF6\u5939");
  assertStrings(value.tags, "\u8BB0\u5FC6\u6807\u7B7E");
}
function assertMemory(value) {
  assertObject(value, "\u8BB0\u5FC6\u8BB0\u5F55");
  requireDecimal(value.id, "\u8BB0\u5FC6\u6570\u636E\u5E93\u6807\u8BC6", true);
  requireId(value.uuid, "\u8BB0\u5FC6 UUID");
  for (const field of ["title", "content", "contentType", "source"]) assertString(value[field], field);
  assertNumber(value.credibility, "\u53EF\u4FE1\u5EA6", 0, 1);
  assertNumber(value.importance, "\u91CD\u8981\u6027", 0, 1);
  for (const field of ["documentPath", "chunkIndexFilePath", "folderPath"]) assertNullableString(value[field], field);
  assertBoolean(value.isDocumentNode, "\u6587\u6863\u8282\u70B9\u6807\u8BC6");
  for (const field of ["createdAt", "updatedAt", "lastAccessedAt"]) assertInteger(value[field], field, 0);
  if (!Array.isArray(value.tags) || !Array.isArray(value.properties)) throw new Error("\u8BB0\u5FC6\u6807\u7B7E\u548C\u5C5E\u6027\u5FC5\u987B\u662F\u6570\u7EC4");
  for (const tag of value.tags) {
    assertObject(tag, "\u8BB0\u5FC6\u6807\u7B7E");
    requireDecimal(tag.id, "\u8BB0\u5FC6\u6807\u7B7E\u6807\u8BC6");
    assertString(tag.name, "\u8BB0\u5FC6\u6807\u7B7E\u540D\u79F0");
  }
  for (const property2 of value.properties) {
    assertObject(property2, "\u8BB0\u5FC6\u5C5E\u6027");
    requireDecimal(property2.id, "\u8BB0\u5FC6\u5C5E\u6027\u6807\u8BC6");
    assertString(property2.key, "\u8BB0\u5FC6\u5C5E\u6027\u952E");
    assertString(property2.value, "\u8BB0\u5FC6\u5C5E\u6027\u503C");
  }
}
function assertMemoryChatMessage(value) {
  assertObject(value, "chat message");
  assertInteger(value.timestamp, "message timestamp", 0);
  for (const key of ["sender", "content", "provider", "modelName"]) assertString(value[key], key);
  assertInteger(value.variantIndex, "message variant index", 0);
  assertInteger(value.variantCount, "message variant count", 1);
  if (value.variantIndex > 2147483647 || value.variantCount > 2147483647 || value.variantIndex >= value.variantCount) throw new Error("Invalid persisted message variant identity");
}
function assertGraph(value) {
  assertObject(value, "\u8BB0\u5FC6\u56FE\u8C31");
  if (!Array.isArray(value.nodes) || !Array.isArray(value.edges)) throw new Error("\u8BB0\u5FC6\u56FE\u8C31\u8282\u70B9\u548C\u5173\u7CFB\u5FC5\u987B\u662F\u6570\u7EC4");
  for (const node of value.nodes) {
    assertObject(node, "\u8BB0\u5FC6\u56FE\u8C31\u8282\u70B9");
    requireId(node.id, "\u56FE\u8C31\u8282\u70B9 UUID");
    assertString(node.label, "\u56FE\u8C31\u8282\u70B9\u6807\u9898");
    assertInteger(node.color, "\u56FE\u8C31\u8282\u70B9\u989C\u8272", 0);
    assertMetadata(node.metadata);
  }
  for (const edge of value.edges) {
    assertObject(edge, "\u8BB0\u5FC6\u56FE\u8C31\u5173\u7CFB");
    requireDecimal(edge.id, "\u56FE\u8C31\u5173\u7CFB\u6807\u8BC6", true);
    requireId(edge.sourceId, "\u56FE\u8C31\u6E90\u8282\u70B9 UUID");
    requireId(edge.targetId, "\u56FE\u8C31\u76EE\u6807\u8282\u70B9 UUID");
    assertNullableString(edge.label, "\u56FE\u8C31\u5173\u7CFB\u6807\u9898");
    assertNumber(edge.weight, "\u5173\u7CFB\u5F3A\u5EA6", 0, 1);
    assertBoolean(edge.isCrossFolderLink, "\u8DE8\u6587\u4EF6\u5939\u5173\u7CFB\u6807\u8BC6");
    assertMetadata(edge.metadata);
  }
}
function assertMetadata(value) {
  assertObject(value, "\u56FE\u8C31\u5143\u6570\u636E");
  for (const item of Object.values(value)) assertString(item, "\u56FE\u8C31\u5143\u6570\u636E\u503C");
}
function assertSearchOptions(value) {
  assertObject(value, "\u8BB0\u5FC6\u641C\u7D22\u53C2\u6570");
  assertString(value.ownerKey, "\u8BB0\u5FC6\u5E93\u6807\u8BC6");
  assertString(value.query, "\u641C\u7D22\u6587\u672C");
  assertNullableString(value.folderPath, "\u641C\u7D22\u6587\u4EF6\u5939");
  assertNumber(value.relevanceThreshold, "\u641C\u7D22\u9608\u503C", 0, Number.MAX_VALUE);
  for (const field of ["createdAtStartMs", "createdAtEndMs"]) if (value[field] !== null) assertInteger(value[field], field, 0);
  if (typeof value.createdAtStartMs === "number" && typeof value.createdAtEndMs === "number" && value.createdAtStartMs > value.createdAtEndMs) throw new Error("\u641C\u7D22\u8D77\u59CB\u65F6\u95F4\u4E0D\u80FD\u665A\u4E8E\u7ED3\u675F\u65F6\u95F4");
}
function assertMemorySettings(value) {
  assertObject(value, "\u8BB0\u5FC6\u8BBE\u7F6E");
  assertInteger(value.autoSaveIntervalMinutes, "\u81EA\u52A8\u4FDD\u5B58\u95F4\u9694", 1);
  if (value.autoSaveIntervalMinutes > 30) throw new Error("\u81EA\u52A8\u4FDD\u5B58\u95F4\u9694\u5FC5\u987B\u5728 1 \u5230 30 \u5206\u949F\u4E4B\u95F4");
  assertInteger(value.nextAutoSaveRunAtMs, "\u4E0B\u6B21\u81EA\u52A8\u4FDD\u5B58\u65F6\u95F4", 0);
  for (const field of ["memoryExtractionCustomRules", "cloudEmbeddingEndpoint", "cloudEmbeddingApiKey", "cloudEmbeddingModel"]) assertString(value[field], field);
  for (const field of ["profileAutoUpdateEnabled", "profileAutoUpdateLocked", "cloudEmbeddingEnabled"]) assertBoolean(value[field], field);
}
function assertSearchConfig(value) {
  assertObject(value, "\u8BB0\u5FC6\u641C\u7D22\u8BBE\u7F6E");
  if (value.scoreMode !== "BALANCED" && value.scoreMode !== "KEYWORD_FIRST" && value.scoreMode !== "SEMANTIC_FIRST") throw new Error("\u8BB0\u5FC6\u8BC4\u5206\u6A21\u5F0F\u65E0\u6548");
  for (const field of ["keywordWeight", "tagWeight", "vectorWeight", "edgeWeight"]) assertNumber(value[field], field, 0, Number.MAX_VALUE);
}
function normalizeNames(values) {
  const result2 = [];
  const seen = /* @__PURE__ */ new Set();
  for (const value of values) {
    const normalized = value.trim();
    if (normalized === "" || seen.has(normalized)) continue;
    seen.add(normalized);
    result2.push(normalized);
  }
  return result2;
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
function requireChatSelection(selection2, cards, groups) {
  const parsed = parseChatSelection(selection2);
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
  const result2 = {};
  for (const [key, item] of Object.entries(value)) Object.defineProperty(result2, key, { value: markerValue(item, path + "." + key), enumerable: true, configurable: true, writable: true });
  return result2;
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
function encodeChatMarker(selection2, extension) {
  parseChatSelection(selection2);
  if (extension === null) return { version: 1, selection: selection2 };
  decodeChatMarker(extension);
  return { ...markerObject(extension, "chat extension"), version: 1, selection: selection2 };
}
function policyNames(value, path) {
  if (!Array.isArray(value)) throw new Error(path + " must be an array");
  const names = [], seen = /* @__PURE__ */ new Set();
  for (const item of value) {
    const name = requireId(item, path);
    if (seen.has(name)) throw new Error("Duplicate " + path + ": " + name);
    seen.add(name);
    names.push(name);
  }
  return names;
}
function markerProfile(value) {
  assertObject(value, "message participant");
  const id2 = requireId(value.id, "message participant id"), name = requireId(value.name, "message participant name");
  assertNullableString(value.avatarUri, "message participant avatar");
  assertString(value.introPrompt, "message participant introPrompt");
  assertString(value.userPreferencesText, "message participant userPreferencesText");
  assertString(value.openingStatement, "message participant openingStatement");
  assertObject(value.modelBinding, "message model binding");
  const providerId = requireId(value.modelBinding.providerId, "message model provider"), modelId = requireId(value.modelBinding.modelId, "message model id");
  const ttsConfigId = requireId(value.ttsConfigId, "message TTS config");
  assertObject(value.toolAccess, "message tool policy");
  assertBoolean(value.toolAccess.enabled, "message tool policy enabled");
  const toolAccess2 = {
    enabled: value.toolAccess.enabled,
    allowedBuiltinTools: policyNames(value.toolAccess.allowedBuiltinTools, "message builtin tools"),
    allowedPackages: policyNames(value.toolAccess.allowedPackages, "message packages"),
    allowedSkills: policyNames(value.toolAccess.allowedSkills, "message skills"),
    allowedMcpServers: policyNames(value.toolAccess.allowedMcpServers, "message MCP servers")
  };
  if (!Array.isArray(value.resources)) throw new Error("Message resources must be an array");
  const resources = [], keys = /* @__PURE__ */ new Set();
  for (const resource of value.resources) {
    assertObject(resource, "message resource");
    const key = requireId(resource.key, "message resource key");
    if (keys.has(key)) throw new Error("Duplicate message resource: " + key);
    keys.add(key);
    assertBoolean(resource.readable, "message resource readable");
    assertBoolean(resource.writable, "message resource writable");
    resources.push({ key, readable: resource.readable, writable: resource.writable });
  }
  return {
    id: id2,
    name,
    avatarUri: value.avatarUri,
    introPrompt: value.introPrompt,
    userPreferencesText: value.userPreferencesText,
    openingStatement: value.openingStatement,
    modelBinding: { providerId, modelId },
    ttsConfigId,
    toolAccess: toolAccess2,
    resources
  };
}
function decodeMessageMarker(value) {
  const extension = markerObject(value, "message extension");
  markerVersion(extension, "message marker");
  const selection2 = requireId(extension.selection, "message marker selection"), parsed = parseChatSelection(selection2);
  if (extension.promptFunctionType !== "CHAT" && extension.promptFunctionType !== "VOICE") throw new Error("Invalid message marker prompt function");
  if (!Array.isArray(extension.participants) || extension.participants.length === 0) throw new Error("Message marker requires complete participants");
  const participants = extension.participants.map(
    /** Constructs each complete saved participant independently from current domain records. */
    (item) => markerProfile(item)
  ), ids = /* @__PURE__ */ new Set();
  for (const participant of participants) {
    if (ids.has(participant.id)) throw new Error("Duplicate message participant: " + participant.id);
    ids.add(participant.id);
  }
  const profile = markerProfile(extension.profile), matches = participants.filter(
    /** Requires the original execution participant to be one exact complete saved participant. */
    (participant) => participant.id === profile.id
  );
  if (matches.length !== 1 || JSON.stringify(matches[0]) !== JSON.stringify(profile)) throw new Error("Message execution profile does not match its saved participant snapshot");
  if (parsed.kind === "card" && (participants.length !== 1 || profile.id !== parsed.id)) throw new Error("Message card selection does not match its saved participant");
  const primaryOwnerKey = requireId(extension.primaryOwnerKey, "message primary memory owner");
  if (!/^(character|shared):[^:\s]+$/u.test(primaryOwnerKey)) throw new Error("Invalid message primary memory owner");
  if (!profile.resources.some(
    /** Requires the primary memory destination to be permitted by this exact saved execution profile. */
    (resource) => resource.key === primaryOwnerKey && resource.writable
  )) throw new Error("Message primary memory owner is not writable in its saved profile");
  return { version: 1, selection: selection2, promptFunctionType: extension.promptFunctionType, primaryOwnerKey, profile, participants };
}
function encodeMessageMarker(marker) {
  const validated = decodeMessageMarker(marker);
  return markerObject(validated, "message extension");
}

// src/group-execution/display.ts
function identity2(card) {
  if (card.id.trim().length === 0 || card.name.trim().length === 0) throw new Error("Display participant identity is empty");
  return { id: card.id, name: card.name, avatarUri: card.avatarUri };
}
function orderedGroupParticipants(group, service) {
  return __async(this, null, function* () {
    if (group.members.length === 0) throw new Error("Selected group has no participants: " + group.id);
    const members2 = [...group.members].sort(
      /** Uses the saved order without choosing an execution participant. */
      (left, right) => left.orderIndex - right.orderIndex
    );
    const ids = /* @__PURE__ */ new Set(), participants = [];
    for (const member2 of members2) {
      if (ids.has(member2.characterCardId)) throw new Error("Duplicate group participant: " + member2.characterCardId);
      ids.add(member2.characterCardId);
      const card = yield service.dispatchDomain("character.get", { id: member2.characterCardId });
      if (card.id !== member2.characterCardId) throw new Error("Group participant read returned a different identity");
      participants.push(identity2(card));
    }
    return participants;
  });
}
function resolveChatDisplay(request, service, executeInitialMessage) {
  return __async(this, null, function* () {
    if (request.purpose !== "display" || request.participantId !== null || request.messageExtension !== null) throw new Error("Display configuration cannot request an execution participant or message snapshot");
    const marker = decodeChatMarker(request.chatExtension), selection2 = parseChatSelection(marker.selection);
    if (selection2.kind === "group") {
      const group = yield service.dispatchDomain("group.get", { id: selection2.id });
      if (group.id !== selection2.id || group.name.trim().length === 0) throw new Error("Display group identity is invalid");
      return { contextKey: marker.selection, identity: { title: group.name, avatarUri: null }, participants: yield orderedGroupParticipants(group, service), initialMessages: [] };
    }
    const card = yield service.dispatchDomain("character.get", { id: selection2.id });
    if (card.id !== selection2.id) throw new Error("Display character read returned a different identity");
    const result2 = { contextKey: marker.selection, identity: { title: card.name, avatarUri: card.avatarUri }, participants: [identity2(card)], initialMessages: [] };
    if (card.openingStatement.trim().length !== 0) {
      const execution = yield executeInitialMessage({ ...request, purpose: "execution", participantId: card.id }, service);
      if (execution.profile.id !== card.id) throw new Error("Initial message author does not match its explicit participant");
      result2.initialMessages.push({ content: card.openingStatement, displayName: execution.profile.name, messageExtension: execution.messageExtension });
    }
    return result2;
  });
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
function shape(fields5, nonempty = false) {
  return (value, path) => {
    const object3 = record(value, path);
    const keys = Object.keys(object3);
    if (nonempty && keys.length === 0) throw new Error(`${path} must contain at least one field`);
    for (const key of keys) {
      if (!Object.prototype.hasOwnProperty.call(fields5, key)) throw new Error(`${path}.${key} is not a supported field`);
      fields5[key].validate(object3[key], `${path}.${key}`);
    }
    for (const key of Object.keys(fields5)) {
      if (fields5[key].required && !Object.prototype.hasOwnProperty.call(object3, key)) throw new Error(`${path}.${key} is required`);
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
var validators = {
  "conversation-group.list": shape({ ownerSelection: required(nullable(nonblank)) }),
  "conversation-group.create": shape({ ownerSelection: required(nullable(nonblank)), name: required(nonblank), pinned: required(boolean) }),
  "conversation-group.update": shape({ id: required(memoryIdentifier), changes: required(shape({ name: optional(nonblank), pinned: optional(boolean) }, true)) }),
  "conversation-group.delete": shape({ id: required(memoryIdentifier) }),
  "conversation-group.moveChat": shape({ chatId: required(nonblank), groupId: required(nullable(memoryIdentifier)), ownerSelection: required(nullable(nonblank)) }),
  "conversation-group.reorder": shape({ ownerSelection: required(nullable(nonblank)), ids: required(array(memoryIdentifier)) }),
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
function parseJsonObject(raw, path) {
  let value;
  try {
    value = JSON.parse(raw);
  } catch (error) {
    throw new Error(`${path}: invalid JSON: ${error instanceof Error ? error.message : String(error)}`);
  }
  return record(value, path);
}
function parseCharacterChanges(value) {
  shape(characterFields, true)(value, "character changes");
  return value;
}
function parseGroupChanges(value) {
  shape(groupFields, true)(value, "group changes");
  return value;
}
function parseTagChanges(value) {
  shape(tagFields, true)(value, "tag changes");
  return value;
}
function jsonValue(value, path = "result") {
  if (value === null || typeof value === "string" || typeof value === "boolean") return value;
  if (typeof value === "number") {
    number(value, path);
    return value;
  }
  if (Array.isArray(value)) {
    return value.map((item, index) => jsonValue(item, `${path}[${index}]`));
  }
  const object3 = record(value, path);
  const result2 = {};
  for (const key of Object.keys(object3)) result2[key] = jsonValue(object3[key], `${path}.${key}`);
  return result2;
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
function chatRecord(records2, id2, kind) {
  const matches = records2.filter(
    /** Matches the supplied identifier without interpreting its spelling. */
    (item) => item.id === id2
  );
  if (matches.length !== 1) throw new Error(
    matches.length === 0 ? kind + " not found: " + id2 : "Duplicate " + kind + ": " + id2
  );
  return matches[0];
}
function chatParticipant(card, request, directory, service) {
  return __async(this, null, function* () {
    nonblank(card.name, "profile.name");
    let modelBinding;
    switch (card.chatModelBindingMode) {
      case "FOLLOW_GLOBAL":
        modelBinding = { ...request.defaultModelBinding };
        break;
      case "FIXED_MODEL": {
        nonblank(card.chatModelId, "profile.fixedModelId");
        const candidates = directory.models.filter(
          /** Resolves the legacy record's deliberately provider-free model selection exactly. */
          (model) => model.modelId === card.chatModelId
        );
        if (candidates.length !== 1) throw new Error(
          candidates.length === 0 ? "Fixed model unavailable: " + card.chatModelId : "Fixed model ambiguous: " + card.chatModelId
        );
        modelBinding = { providerId: candidates[0].providerId, modelId: candidates[0].modelId };
        break;
      }
      default:
        throw new Error("Invalid model binding mode: " + card.chatModelBindingMode);
    }
    const resolvedModels = directory.models.filter(
      /** Requires the actual configured provider/model pair. */
      (model) => model.providerId === modelBinding.providerId && model.modelId === modelBinding.modelId
    );
    if (resolvedModels.length !== 1) throw new Error("Model binding unavailable or duplicated: " + modelBinding.providerId + "/" + modelBinding.modelId);
    const ttsConfigId = card.ttsConfigId === null ? request.defaultTtsConfigId : card.ttsConfigId;
    nonblank(ttsConfigId, "profile.ttsConfigId");
    chatRecord(directory.ttsConfigs, ttsConfigId, "TTS configuration");
    let primary;
    switch (card.memoryBindingMode) {
      case "CHARACTER":
        primary = "character:" + card.id;
        break;
      case "SHARED":
        nonblank(card.sharedMemoryId, "profile.sharedMemoryId");
        chatRecord(directory.stores, card.sharedMemoryId, "shared memory");
        primary = "shared:" + card.sharedMemoryId;
        break;
      default:
        throw new Error("Invalid memory binding mode: " + card.memoryBindingMode);
    }
    const routes = /* @__PURE__ */ new Map();
    routes.set(primary, { key: primary, readable: true, writable: true });
    for (const mount2 of card.sharedMemoryMounts) {
      chatRecord(directory.stores, mount2.sharedMemoryId, "mounted shared memory");
      const key = "shared:" + mount2.sharedMemoryId;
      const existing = routes.get(key);
      routes.set(key, { key, readable: mount2.readable || existing?.readable === true, writable: mount2.writable || existing?.writable === true });
    }
    const prompt = yield service.dispatchDomain("character.combine", { id: card.id, promptFunctionType: request.promptFunctionType, additionalTagIds: [] });
    const document = yield service.dispatchDomain("memory.user.read", { ownerKey: primary });
    return {
      id: card.id,
      name: card.name,
      avatarUri: card.avatarUri,
      introPrompt: prompt.prompt,
      userPreferencesText: document.content,
      openingStatement: card.openingStatement,
      modelBinding,
      ttsConfigId,
      toolAccess: {
        ...card.toolAccessConfig,
        allowedBuiltinTools: [...card.toolAccessConfig.allowedBuiltinTools],
        allowedPackages: [...card.toolAccessConfig.allowedPackages],
        allowedSkills: [...card.toolAccessConfig.allowedSkills],
        allowedMcpServers: [...card.toolAccessConfig.allowedMcpServers]
      },
      resources: [...routes.values()]
    };
  });
}
function resolveChatConfiguration(request, service) {
  return __async(this, null, function* () {
    if (request.purpose === "display") return resolveChatDisplay(request, service, resolveChatExecution);
    return resolveChatExecution(request, service);
  });
}
function resolveChatExecution(request, service) {
  return __async(this, null, function* () {
    if (request.purpose !== "execution") throw new Error("Execution configuration requires purpose execution");
    if (request.messageExtension !== null) {
      const marker2 = decodeMessageMarker(request.messageExtension);
      if (request.participantId !== null && request.participantId !== marker2.profile.id) throw new Error("Historical execution participant does not match the saved message snapshot");
      return { contextKey: marker2.selection, profile: marker2.profile, participants: marker2.participants, messageExtension: markerObject(request.messageExtension, "historical message extension") };
    }
    const marker = decodeChatMarker(request.chatExtension);
    const directory = yield service.snapshot();
    const selection2 = requireChatSelection(marker.selection, directory.cards, directory.groups);
    let cards;
    if (selection2.kind === "card") cards = [chatRecord(directory.cards, selection2.id, "character")];
    else {
      const group = chatRecord(directory.groups, selection2.id, "group");
      const members2 = [...group.members].sort(
        /** Preserves the persisted participant order without planning turns in Core. */
        (left, right) => left.orderIndex - right.orderIndex
      );
      if (members2.length === 0) throw new Error("Selected group has no participants: " + group.id);
      const ids = /* @__PURE__ */ new Set();
      cards = members2.map(
        /** Rejects duplicate or deleted group members before producing any execution profile. */
        (member2) => {
          if (ids.has(member2.characterCardId)) throw new Error("Duplicate group participant: " + member2.characterCardId);
          ids.add(member2.characterCardId);
          return chatRecord(directory.cards, member2.characterCardId, "group participant");
        }
      );
    }
    if (request.participantId === null && cards.length !== 1) throw new Error("A planned conversation requires an explicit execution participant");
    if (request.participantId !== null) chatRecord(cards, request.participantId, "selected participant");
    const participants = [];
    for (const card of cards) participants.push(yield chatParticipant(card, request, directory, service));
    let profile;
    if (request.participantId !== null) profile = chatRecord(participants, request.participantId, "selected participant");
    else {
      if (participants.length !== 1) throw new Error("A planned conversation requires an explicit execution participant");
      profile = participants[0];
    }
    const owner2 = yield service.dispatchDomain("memory.resolveOwner", { characterId: profile.id });
    const primaryOwnerKey = (owner2.kind === "CHARACTER" ? "character:" : "shared:") + owner2.id;
    const messageExtension = encodeMessageMarker({ version: 1, selection: marker.selection, promptFunctionType: request.promptFunctionType, primaryOwnerKey, profile, participants });
    return { contextKey: marker.selection, profile, participants, messageExtension };
  });
}

// src/memory-jobs/prompt-template.ts
var extractionTemplate = '\u4F60\u8981\u4ECE\u5BF9\u8BDD\u4E2D\u6784\u5EFA\u957F\u671F\u8BB0\u5FC6\u56FE\u8C31\u3002\r\n\r\n$duplicatesPromptPart\r\n$existingMemoriesPrompt\r\n$existingFoldersPrompt\r\n\r\n\u3010\u5199\u5165\u524D\u5148\u8FC7\u7B5B\u3011\r\n- \u53EA\u8BB0\u5F55"\u7528\u6237\u7279\u5F02\u4E14\u53EF\u590D\u7528"\u7684\u4FE1\u606F\uFF1A\u7A33\u5B9A\u504F\u597D\u3001\u7EA6\u675F\u3001\u5DF2\u786E\u8BA4\u51B3\u7B56\u3001\u53CD\u590D\u9519\u8BEF\u3001\u9879\u76EE\u4E8B\u5B9E\u3001\u957F\u671F\u4E16\u754C\u89C2\u4E2D\u7684\u7A33\u5B9A\u8BBE\u5B9A\u3002\r\n- \u4E0D\u8BB0\u5F55\u5E38\u8BC6/\u516C\u5F00\u5B9A\u4E49\uFF08\u5982"TS\u662F\u4EC0\u4E48""Node\u662F\u4EC0\u4E48""\u78C1\u504F\u89D2\u662F\u4EC0\u4E48"\uFF09\u3002\r\n- \u4E0D\u8BB0\u5F55\u672A\u6765\u63A8\u6D4B\u9879\uFF1A\u4E0B\u4E00\u6B65\u5EFA\u8BAE\u3001TODO\u3001\u6682\u5B9A\u8BA1\u5212\u3002\r\n- \u82E5\u6CA1\u6709\u957F\u671F\u4EF7\u503C\u4FE1\u53F7\uFF0C\u76F4\u63A5\u8FD4\u56DE `{}`\u3002\r\n\r\n\u3010\u62BD\u53D6\u7B56\u7565\u3011\r\n- \u63D0\u4F9B\u7684\u5DF2\u6709\u8BB0\u5FC6\u53EA\u662F\u68C0\u7D22\u7EBF\u7D22\uFF0C\u4E0D\u662F\u4E8B\u5B9E\u8BC1\u636E\uFF1B\u53EA\u6709\u5BF9\u8BDD\u660E\u786E\u8BC1\u660E\u4E3B\u4F53\u548C\u4E8B\u5B9E\u76F8\u540C\u65F6\u624D\u53EF `update`\u3001`merge` \u6216\u8FDE\u8FB9\uFF0C\u5426\u5219\u5FFD\u7565\u8BE5\u5019\u9009\u3002\r\n- \u4F18\u5148 `update` / `merge`\uFF0C\u5176\u6B21\u624D\u662F `new`\u3002\r\n- `new` \u4EC5\u5728\u786E\u5B9E\u65B0\u589E\u6982\u5FF5\u65F6\u4F7F\u7528\uFF08\u6700\u591A 5 \u6761\uFF09\u3002\r\n- \u957F\u671F\u5C0F\u8BF4/\u4E16\u754C\u89C2\u573A\u666F\u4E2D\uFF0C\u53CD\u590D\u51FA\u73B0\u4E14\u5F71\u54CD\u8FDE\u7EED\u6027\u7684\u89D2\u8272\u3001\u5730\u70B9\u3001\u7EC4\u7EC7\u3001\u89C4\u5219\u3001\u65F6\u95F4\u7EBF\u53EF\u4EE5\u5165\u5E93\u3002\r\n- \u82E5\u6838\u5FC3\u662F"\u66F4\u65B0\u65E7\u6982\u5FF5"\uFF0C`main` \u5FC5\u987B\u4E3A `null`\uFF0C\u53EA\u7528 `update`\u3002\r\n- \u5982\u679C\u53EA\u662F\u5BF9\u5DF2\u6709\u8BB0\u5FC6\u7684\u6539\u5199\uFF08\u540C\u4E3B\u4F53 + \u540C\u52A8\u4F5C + \u540C\u7ED3\u679C\uFF09\uFF0C\u6309\u91CD\u590D\u5904\u7406\uFF1A\u4F18\u5148 `update`/`merge`\uFF0C\u4E0D\u8981\u518D `new`\u3002\r\n- \u5982\u679C `main` \u4E0E\u5DF2\u6709\u8BB0\u5FC6\u5728\u8BED\u4E49\u4E0A\u662F\u540C\u4E00\u4E8B\u4EF6\uFF0C`main` \u8BBE\u4E3A `null`\uFF0C\u6539\u7528 `update` \u6216 `merge`\u3002\r\n- \u5982\u679C\u5F53\u524D\u8F6E\u7684\u5927\u90E8\u5206\u4E8B\u5B9E\u5DF2\u88AB\u5DF2\u6709\u8BB0\u5FC6\u8986\u76D6\uFF0C\u4E0D\u8981\u518D\u521B\u5EFA\u5E73\u884C `new`\uFF0C\u4F18\u5148\u7ED9\u51FA\u4E00\u6B21 `update` \u6216\u4E00\u6B21 `merge`\u3002\r\n- \u5728\u6709\u660E\u786E\u91CD\u590D\u8BC1\u636E\u65F6\u7EE7\u7EED `new` \u89C6\u4E3A\u4E0D\u5408\u683C\u8F93\u51FA\u3002\r\n- \u63D0\u4F9B\u7ED9\u4F60\u7684\u5DF2\u6709\u8BB0\u5FC6\u6837\u672C\u662F\u53EF\u64CD\u4F5C\u5BF9\u8C61\uFF1A\u5373\u4F7F\u672C\u8F6E\u6CA1\u6709 `new`\uFF0C\u4E5F\u53EF\u4EE5\u76F4\u63A5\u5BF9\u8FD9\u4E9B\u5DF2\u6709\u8BB0\u5FC6\u505A `update`\u3001`merge`\u3001`links`\u3002\r\n\r\n\u3010\u8BED\u6C14\u7B56\u7565\u3011\r\n- \u8BED\u6C14\u53EF\u6839\u636E\u573A\u666F\u53D8\u5316\uFF08\u6280\u672F\u3001\u65E5\u5E38\u804A\u5929\u3001\u5C0F\u8BF4\u521B\u4F5C\uFF09\uFF0C\u4F46\u53EA\u80FD\u6539\u53D8\u8868\u8FBE\u65B9\u5F0F\uFF0C\u4E0D\u80FD\u6539\u53D8\u5165\u5E93\u6807\u51C6\u3002\r\n- \u7ED3\u6784\u548C\u4E8B\u5B9E\u5FC5\u987B\u7A33\u5B9A\uFF1A\u8BED\u6C14\u53D8\u5316\u4E0D\u7B49\u4E8E\u653E\u5BBD\u7B5B\u9009\u3002\r\n- \u4E0D\u80FD\u56E0\u4E3A\u8BED\u6C14\u81EA\u7136\u5316\u5C31\u8BB0\u5F55\u5E38\u8BC6\u6216\u672A\u6765\u8BA1\u5212\u3002\r\n- \u6807\u9898\u4FDD\u6301\u7B80\u6D01\u5E76\u805A\u7126\u4E8B\u4EF6\uFF0C\u5185\u5BB9\u5728\u53EF\u8BFB\u7684\u524D\u63D0\u4E0B\u8D34\u5408\u573A\u666F\u8BED\u6C14\u3002\r\n\r\n\u3010\u6807\u9898\u4E0E\u5185\u5BB9\u5199\u6CD5\u3011\r\n- `main` \u6807\u9898\u4F18\u5148\u5199\u4E8B\u4EF6\uFF0C\u4E0D\u5199\u5B9A\u4E49\u3002\r\n- \u63A8\u8350\u6807\u9898\u6A21\u677F\uFF1A\r\n  - \u4E8B\u4EF6\uFF1A`[\u9886\u57DF] \u4E8B\u4EF6\uFF1A\u52A8\u4F5C + \u7ED3\u679C`\r\n  - \u4E16\u754C\u89C2\u5B9E\u4F53\uFF1A`\u5B9E\u4F53\uFF1A\u540D\u79F0\uFF08\u8EAB\u4EFD/\u7C7B\u578B\uFF09`\r\n- \u4E0D\u63A8\u8350\u6807\u9898\uFF1A`X\u662F\u4EC0\u4E48`\u3001`X\u7684\u5B9A\u4E49`\u3001\u767E\u79D1\u5F0F\u6CDB\u6807\u9898\u3002\r\n- \u5185\u5BB9\u53EA\u5199"\u5DF2\u53D1\u751F\u4E8B\u5B9E + \u5F53\u524D\u5DF2\u786E\u8BA4\u72B6\u6001"\u3002\r\n- \u5185\u5BB9\u7981\u6B62\u5199\u672A\u6765\u52A8\u4F5C\u3001TODO\u3001\u63A8\u6D4B\u6027\u8BA1\u5212\u3002\r\n$memoryExtractionCustomRulesInstruction\r\n\r\n\u3010\u8FDE\u63A5\u5173\u7CFB\u89C4\u5219\u3011\r\n- \u53EA\u6709\u5F53\u5BF9\u8BDD\u91CC\u6709\u660E\u786E\u8BC1\u636E\u65F6\u624D\u5EFA\u8FB9\u3002\r\n- \u63A8\u8350\u5173\u7CFB\u7C7B\u578B\uFF1A\r\n  - \u4E8B\u4EF6\u6D41\u7A0B\uFF1A`FOLLOWS`\u3001`CORRECTS`\u3001`UPDATES`\r\n  - \u53C2\u4E0E\u4E0E\u4E0A\u4E0B\u6587\uFF1A`INVOLVES`\u3001`HAPPENS_AT`\r\n  - \u4E16\u754C\u89C2\u7ED3\u6784\uFF1A`PART_OF`\u3001`ALLIED_WITH`\u3001`OPPOSES`\r\n- \u4E0D\u80FD\u4EC5\u51ED"\u540C\u6BB5\u63D0\u5230\u8FC7"\u5C31\u8FDE\u8FB9\u3002\r\n- \u62FF\u4E0D\u51C6\u5C31\u4E0D\u8FDE\u3002\r\n- \u5EFA\u8FB9\u8303\u56F4\u4E0D\u5E94\u53EA\u9650\u4E8E\u672C\u8F6E\u65B0\u8F93\u51FA\uFF1B\u5982\u679C"\u5DF2\u6709\u6837\u672C\u8BB0\u5FC6"\u4E0E\u672C\u8F6E\u4E8B\u4EF6/\u5B9E\u4F53\u5173\u7CFB\u660E\u786E\uFF0C\u4E5F\u5E94\u4E3B\u52A8\u5EFA\u8FB9\u3002\r\n- \u8F93\u51FA\u524D\u8BF7\u5728\u5168\u91CF\u5BF9\u8C61\u4E0A\u505A\u4E24\u4E24\u5173\u7CFB\u68C0\u67E5\uFF1A`main`\u3001`new`\u3001`update` \u76EE\u6807\u3001\u4EE5\u53CA\u63D0\u4F9B\u7684\u5DF2\u6709\u8BB0\u5FC6\uFF1B\u51E1\u6709\u660E\u786E\u8BC1\u636E\u90FD\u5E94\u5EFA\u8FB9\u3002\r\n\r\n\u3010\u793A\u4F8B\uFF08\u5FC5\u987B\u9075\u5FAA\uFF09\u3011\r\n- \u4EC5\u5728\u95EE\u7B54\u5E38\u8BC6\uFF08\u5982"\u78C1\u504F\u89D2\u662F\u4EC0\u4E48"\uFF09\u4E14\u65E0\u7528\u6237\u7279\u5F02\u4FE1\u53F7\uFF1A\u8FD4\u56DE `{}`\u3002\r\n- \u4EC5\u89E3\u91CA TS/Node \u7B49\u516C\u5F00\u5B9A\u4E49\uFF1A\u8FD4\u56DE `{}`\u3002\r\n- \u95F2\u804A\u4F46\u6709\u5B9E\u9645\u4EA4\u6D41\u5185\u5BB9\uFF1A\u538B\u7F29\u6210\u4E00\u6761\u4E8B\u4EF6\u578B `main` \u8BB0\u5F55\uFF0C\u4E0D\u62C6\u6280\u672F\u7EC6\u8282\u3002\r\n- \u53EA\u6709\u7A7A\u6CDB\u5BD2\u6684\uFF08\u5982\u4EC5"\u4F60\u597D/\u5728\u5417"\uFF09\uFF1A\u8FD4\u56DE `{}`\u3002\r\n- \u672C\u8F6E\u51FA\u73B0"\u7528\u6237\u72AF\u9519\u5E76\u88AB\u7EA0\u6B63"\uFF1A\u4F5C\u4E3A\u4E8B\u4EF6\u5199\u5165 `main`\u3002\r\n- \u957F\u671F\u5C0F\u8BF4/\u4E16\u754C\u89C2\u8BA8\u8BBA\uFF1A\u53CD\u590D\u51FA\u73B0\u4E14\u5F71\u54CD\u8FDE\u7EED\u6027\u7684\u89D2\u8272\u3001\u5730\u540D\u3001\u7EC4\u7EC7\u3001\u89C4\u5219\u3001\u65F6\u95F4\u7EBF\u5E94\u5165\u5E93\uFF0C\u6309\u9700\u4F7F\u7528 `new`/`links`\u3002\r\n- \u4EC5\u89E3\u91CA\u533B\u7597\u5B9A\u4E49\uFF08\u5982"\u6D41\u611F\u662F\u4EC0\u4E48"\uFF09\uFF1A\u8FD4\u56DE `{}`\u3002\r\n- \u4EC5\u89E3\u91CA\u91D1\u878D\u5B9A\u4E49\uFF08\u5982"ETF\u662F\u4EC0\u4E48"\uFF09\uFF1A\u8FD4\u56DE `{}`\u3002\r\n- \u9879\u76EE\u672C\u8F6E\u6709\u660E\u786E\u8FDB\u5C55\uFF08\u4FEE\u590D\u5B8C\u6210/\u6458\u8981\u5B8C\u6210/\u4EFB\u52A1\u7EC8\u6B62\uFF09\uFF1A\u5199\u4E00\u6761\u4E8B\u4EF6\u578B `main`\u3002\r\n- \u53CD\u590D\u89E3\u91CA\u4F46\u6CA1\u6709\u65B0\u8FDB\u5C55/\u65B0\u51B3\u7B56\uFF1A\u8FD4\u56DE `{}`\u3002\r\n- \u4E16\u754C\u89C2\u8BBE\u5B9A\u53D1\u751F\u53D8\u5316\uFF08\u5173\u7CFB/\u5F52\u5C5E\u53D8\u66F4\uFF09\uFF1A\u4F18\u5148 `update`\uFF0C\u5E76\u5728\u8BC1\u636E\u660E\u786E\u65F6\u8FDE `UPDATES` / `PART_OF`\u3002\r\n- \u672C\u8F6E\u53EA\u662F\u91CD\u8FF0\u5DF2\u5B58\u5728\u4E8B\u4EF6\uFF1A\u4F18\u5148 `update`/`merge`\uFF0C\u4E0D\u8981 `new`\u3002\r\n- \u4E8B\u4EF6\u91CC\u660E\u786E\u51FA\u73B0\u53C2\u4E0E\u8005/\u5DE5\u5177\u5305\u4E14\u5173\u7CFB\u6E05\u6670\uFF1A\u8865\u5145 `INVOLVES` \u94FE\u63A5\u3002\r\n- \u672C\u8F6E\u786E\u8BA4\u4E86"\u5DF2\u6709\u6837\u672C\u8BB0\u5FC6"\u548C\u5176\u4ED6\u8BB0\u5FC6\u7684\u660E\u786E\u5173\u7CFB\uFF1A\u5373\u4F7F\u6CA1\u6709 `new`\uFF0C\u4E5F\u5E94\u5728 `links` \u4E2D\u4F53\u73B0\u3002\r\n\r\n\u3010\u8F93\u51FA\u683C\u5F0F\uFF08\u4E25\u683CJSON\uFF09\u3011\r\n- \u9664\u8FD4\u56DE `{}` \u5916\uFF0C\u5FC5\u987B\u5305\u542B `main`\u3001`new`\u3001`update`\u3001`merge`\u3001`links`$profileOptionalKey\uFF1B\u6570\u7EC4\u4E2D\u7684\u6BCF\u4E00\u9879\u5FC5\u987B\u662F\u5177\u540D\u5BF9\u8C61\uFF0C\u7981\u6B62\u4F4D\u7F6E\u6570\u7EC4\u3002\r\n- `main`\uFF1A`null` \u6216 `{"title":"...","content":"...","tags":["..."],"folder_path":"..."}`\u3002\r\n- `new`\uFF1A`[{"title":"...","content":"...","tags":["..."],"folder_path":"...","alias_for":null}, ...]`\u3002\r\n- `update`\uFF1A`[{"title":"...","content":"\u65B0\u5B8C\u6574\u5185\u5BB9","reason":"...","credibility":null,"importance":null}, ...]`\u3002\r\n- `merge`\uFF1A`[{"source_titles":["A","B"],"title":"...","content":"...","tags":["..."],"folder_path":"...","reason":"..."}, ...]`\u3002\r\n- `links`\uFF1A`[{"source":"...","target":"...","type":"\u5927\u5199\u4E0B\u5212\u7EBF\u5173\u7CFB","description":"...","weight":0.0}, ...]`\u3002\r\n- \u53EF\u4FE1\u5EA6\u3001\u91CD\u8981\u6027\u3001\u6743\u91CD\u5FC5\u987B\u662F 0.0 \u5230 1.0 \u7684 JSON \u6570\u5B57\uFF1B\u53EF\u4FE1\u5EA6\u3001\u91CD\u8981\u6027\u548C `alias_for` \u53EF\u7528 JSON `null`\u3002\r\n$profileMarkdownSchemaLine\r\n\r\n$profileUpdateInstruction\r\n\r\n\u53EA\u8FD4\u56DE\u5408\u6CD5 JSON \u5BF9\u8C61\uFF0C\u4E0D\u8981\u8F93\u51FA\u5176\u4ED6\u5185\u5BB9\u3002\r\n';

// src/chat-extensions.ts
function readChatExtensionBinding(chatId) {
  return __async(this, null, function* () {
    requireId(chatId, "chat id");
    const extension = yield Tools.Chat.readExtension({ kind: "chat", chatId });
    if (extension === null) throw new Error("Chat has no character selection in this plugin's namespace: " + chatId);
    const marker = decodeChatMarker(extension);
    return { chatId, selection: marker.selection };
  });
}
function readMessageExtensionMarker(chatId, messageTimestamp, variantIndex) {
  return __async(this, null, function* () {
    requireId(chatId, "chat id");
    assertInteger(messageTimestamp, "message timestamp", 0);
    assertInteger(variantIndex, "message variant index", 0);
    if (variantIndex > 2147483647) throw new Error("Message variant index exceeds the host's i32 range");
    const extension = yield Tools.Chat.readExtension({ kind: "message", chatId, messageTimestamp, variantIndex });
    if (extension === null) throw new Error("Message revision has no saved character identity in this plugin's namespace: " + chatId + "/" + messageTimestamp + "/" + variantIndex);
    return decodeMessageMarker(extension);
  });
}
function listChatExtensionBindings() {
  return __async(this, null, function* () {
    const result2 = yield Tools.Chat.listAll();
    assertInteger(result2.totalCount, "all chat count", 0);
    if (!Array.isArray(result2.chats) || result2.chats.length !== result2.totalCount) throw new Error("Chat.listAll did not return its complete declared records");
    const bindings = [], ids = /* @__PURE__ */ new Set();
    for (const chat of result2.chats) {
      const chatId = requireId(chat.id, "host chat id");
      if (ids.has(chatId)) throw new Error("Duplicate host chat identity: " + chatId);
      ids.add(chatId);
      const extension = yield Tools.Chat.readExtension({ kind: "chat", chatId });
      if (extension === null) continue;
      const marker = decodeChatMarker(extension);
      bindings.push({ chatId, selection: marker.selection });
    }
    return bindings;
  });
}
function writeChatExtensionBinding(binding) {
  return __async(this, null, function* () {
    requireId(binding.chatId, "chat id");
    const target = { kind: "chat", chatId: binding.chatId };
    const existing = yield Tools.Chat.readExtension(target), value = encodeChatMarker(binding.selection, existing === null ? null : markerObject(existing, "existing chat extension"));
    const stored = yield Tools.Chat.writeExtension(target, value), marker = decodeChatMarker(stored);
    if (marker.selection !== binding.selection) throw new Error("Stored chat extension selection differs from the submitted value");
    return { chatId: binding.chatId, selection: marker.selection };
  });
}
function deleteChatExtensionBinding(chatId) {
  return __async(this, null, function* () {
    requireId(chatId, "chat id");
    const deleted = yield Tools.Chat.deleteExtension({ kind: "chat", chatId });
    assertBoolean(deleted, "chat extension deleted");
    return { chatId, deleted };
  });
}

// src/memory-jobs/chat.ts
function assertChat(value) {
  assertObject(value, "chat");
  requireId(value.id, "chat id");
  assertString(value.title, "chat title");
  for (const key of ["messageCount", "inputTokens", "outputTokens"]) assertInteger(value[key], key, 0);
  for (const key of ["createdAt", "updatedAt"]) assertString(value[key], key);
  assertBoolean(value.isCurrent, "current chat");
}
function requireChat(chatId) {
  return __async(this, null, function* () {
    requireId(chatId, "chat id");
    const result2 = yield Tools.Chat.findChat({ query: chatId, match: "exact", index: 0 });
    assertInteger(result2.matchedCount, "matched chat count", 1);
    assertChat(result2.chat);
    if (result2.matchedCount !== 1 || result2.chat.id !== chatId) throw new Error("Chat lookup does not identify exactly one host record: " + chatId);
    return result2.chat;
  });
}
function chatParticipants(chatId, repository) {
  return __async(this, null, function* () {
    const binding = yield repository.readChatBinding(chatId), cards = yield repository.listCharacters(), groups = yield repository.listGroups();
    const selection2 = requireChatSelection(binding.selection, cards, groups);
    if (selection2.kind === "card") return [yield repository.getCharacter(selection2.id)];
    const group = yield repository.getGroup(selection2.id);
    if (group.members.length === 0) throw new Error("Selected chat group has no participants: " + group.id);
    const members2 = [...group.members].sort(
      /** Preserves the exact stored participant order. */
      (left, right) => left.orderIndex - right.orderIndex
    );
    const result2 = [];
    for (const member2 of members2) result2.push(yield repository.getCharacter(member2.characterCardId));
    return result2;
  });
}
function primaryOwner(card, repository) {
  return __async(this, null, function* () {
    switch (card.memoryBindingMode) {
      case "CHARACTER":
        return "character:" + card.id;
      case "SHARED": {
        const id2 = requireId(card.sharedMemoryId, "primary shared library");
        yield repository.getStore(id2);
        return "shared:" + id2;
      }
      default:
        throw new Error("Invalid memory binding mode: " + card.memoryBindingMode);
    }
  });
}
function requireChatOwner(chatId, ownerKey, repository) {
  return __async(this, null, function* () {
    yield repository.readMemorySpace(ownerKey);
    const cards = yield chatParticipants(chatId, repository);
    for (const card of cards) {
      if ((yield primaryOwner(card, repository)) === ownerKey) return;
      for (const mount2 of card.sharedMemoryMounts) if (mount2.writable && "shared:" + mount2.sharedMemoryId === ownerKey) {
        yield repository.getStore(mount2.sharedMemoryId);
        return;
      }
    }
    throw new Error("Chat is not writable for this memory owner: " + chatId + " / " + ownerKey);
  });
}
function requireMessageOwner(chatId, message, ownerKey, repository) {
  return __async(this, null, function* () {
    assertMemoryChatMessage(message);
    yield repository.readMemorySpace(ownerKey);
    const marker = yield readMessageExtensionMarker(chatId, message.timestamp, message.variantIndex);
    if (!marker.profile.resources.some(
      /** Checks only this saved participant's explicit writable resource grants. */
      (resource) => resource.key === ownerKey && resource.writable
    )) throw new Error("Saved message identity is not writable for this memory owner: " + ownerKey);
    return marker;
  });
}
function requireMessagesOwner(chatId, messages, ownerKey, repository) {
  return __async(this, null, function* () {
    yield repository.readMemorySpace(ownerKey);
    let permitted = false;
    for (const message of messages) {
      if (message.sender !== "ai" && message.sender !== "assistant") continue;
      const marker = yield readMessageExtensionMarker(chatId, message.timestamp, message.variantIndex);
      if (marker.profile.resources.some(
        /** Requires an actual saved writable grant without deriving identity from a display name. */
        (resource) => resource.key === ownerKey && resource.writable
      )) permitted = true;
    }
    if (!permitted) throw new Error("History has no saved assistant identity writable for this memory owner: " + ownerKey);
  });
}
function listMemoryChats(ownerKey, repository) {
  return __async(this, null, function* () {
    yield repository.readMemorySpace(ownerKey);
    const bindings = yield repository.listChatBindings(), selected = /* @__PURE__ */ new Set();
    for (const binding of bindings) {
      const cards = yield chatParticipants(binding.chatId, repository);
      for (const card of cards) {
        if ((yield primaryOwner(card, repository)) === ownerKey) selected.add(binding.chatId);
        for (const mount2 of card.sharedMemoryMounts) if (mount2.writable && "shared:" + mount2.sharedMemoryId === ownerKey) selected.add(binding.chatId);
      }
    }
    const chats = [];
    for (const id2 of selected) chats.push(yield requireChat(id2));
    return chats.sort(
      /** Keeps the host directory's updated-at descending presentation without limiting the bound records. */
      (left, right) => {
        const leftTime = Number(left.updatedAt), rightTime = Number(right.updatedAt);
        assertInteger(leftTime, "chat updated timestamp", 0);
        assertInteger(rightTime, "chat updated timestamp", 0);
        return rightTime - leftTime;
      }
    );
  });
}
function readMessages(chatId) {
  return __async(this, null, function* () {
    const chat = yield requireChat(chatId);
    if (chat.messageCount > 2147483647) throw new Error("Chat message count exceeds the host's supported index range");
    const end = chat.messageCount === 0 ? 0 : chat.messageCount - 1;
    const result2 = yield Tools.Chat.getMessagesRange(chatId, { order: "asc", start: 0, end });
    if (result2.order !== "asc" || result2.start !== 0 || result2.end !== end || result2.limit !== end + 1) throw new Error("Chat message result does not match the requested source range");
    if (result2.chatId !== chatId || !Array.isArray(result2.messages)) throw new Error("Chat message result has a different identity or missing records");
    if (result2.messages.length > chat.messageCount) throw new Error("Chat history changed while reading the planned source range");
    const messages = [], timestamps = /* @__PURE__ */ new Set();
    for (const message of result2.messages) {
      assertMemoryChatMessage(message);
      if (timestamps.has(message.timestamp)) throw new Error("Chat range returned duplicate selected message revisions");
      timestamps.add(message.timestamp);
      messages.push({ sender: message.sender, content: message.content, timestamp: message.timestamp, variantIndex: message.variantIndex, variantCount: message.variantCount, provider: message.provider, modelName: message.modelName });
    }
    const current = yield requireChat(chatId);
    if (current.messageCount !== chat.messageCount || current.updatedAt !== chat.updatedAt) throw new Error("Chat history changed while reading the planned source range");
    return messages.sort(
      /** Orders genuine message timestamps with stable source ordering. */
      (left, right) => left.timestamp - right.timestamp
    );
  });
}
function planWindows(chat, messages, options) {
  const size = Math.max(8, Math.min(48, options.windowMessageCount)), windows = [];
  let source = [], context = [], user = null;
  function emit() {
    if (source.length === 0) return;
    windows.push({ chatId: chat.id, chatTitle: chat.title, messages: [...context, ...source], sourceMessageCount: source.length });
    source = [];
    context = [];
  }
  for (const message of messages) {
    if (options.fromInclusive !== null && message.timestamp < options.fromInclusive) continue;
    if (options.toInclusive !== null && message.timestamp > options.toInclusive) continue;
    if (message.content.trim() === "") continue;
    switch (message.sender) {
      case "user":
        if (source.length >= size - 1) emit();
        user = message;
        source.push(message);
        break;
      case "ai":
      case "assistant":
        if (user === null) continue;
        if (source.length >= size) {
          emit();
          context.push(user);
        }
        source.push(message);
        break;
    }
  }
  emit();
  return windows;
}

// src/memory-jobs/analysis.ts
function parseModelJson(text3) {
  assertString(text3, "model output");
  const trimmed = text3.trim();
  const fence = /^```(?:json)?\s*\r?\n([\s\S]*?)\r?\n```\s*$/u.exec(trimmed);
  return JSON.parse(fence === null ? trimmed : fence[1]);
}
function entity(value, alias) {
  assertObject(value, "analysis entity");
  requireName(value.title, "entity title");
  assertString(value.content, "entity content");
  assertStrings(value.tags, "entity tags");
  assertString(value.folder_path, "entity folder");
  if (alias && value.alias_for !== void 0 && value.alias_for !== null) requireName(value.alias_for, "alias target");
}
function newEntity(value) {
  entity(value, true);
}
function update(value) {
  assertObject(value, "memory update");
  requireName(value.title, "update title");
  assertString(value.content, "update content");
  assertString(value.reason, "update reason");
  for (const key of ["credibility", "importance"]) if (value[key] !== void 0 && value[key] !== null) assertNumber(value[key], key, 0, 1);
}
function merge(value) {
  assertObject(value, "memory merge");
  assertStrings(value.source_titles, "merge sources");
  if (value.source_titles.length === 0) throw new Error("Memory merge requires source records");
  entity(value, false);
  assertString(value.reason, "merge reason");
}
function link(value) {
  assertObject(value, "memory relationship");
  for (const key of ["source", "target", "type"]) requireName(value[key], key);
  assertString(value.description, "relationship description");
  assertNumber(value.weight, "relationship weight", 0, 1);
}
function objectArray(value, label, check) {
  if (!Array.isArray(value)) throw new Error("Memory analysis requires a named object array: " + label);
  const result2 = [];
  for (const item of value) {
    check(item);
    result2.push(item);
  }
  return result2;
}
function analysis(value) {
  assertObject(value, "memory analysis");
  if (Object.keys(value).length === 0) return null;
  let main = null;
  if (value.main !== null) {
    entity(value.main, false);
    main = value.main;
  }
  const result2 = {
    main,
    new: objectArray(value.new, "new", newEntity),
    update: objectArray(value.update, "update", update),
    merge: objectArray(value.merge, "merge", merge),
    links: objectArray(value.links, "links", link)
  };
  if (result2.new.length > 5) throw new Error("Memory extraction exceeds the five-entity gate");
  if (Object.prototype.hasOwnProperty.call(value, "profile_markdown")) {
    if (value.profile_markdown === null) result2.profile_markdown = null;
    else {
      assertString(value.profile_markdown, "profile Markdown");
      result2.profile_markdown = value.profile_markdown;
    }
  }
  return result2;
}
function cleanMessage(message) {
  let content = message.content.replace(/<think(?:ing)?\b[^>]*>[\s\S]*?<\/think(?:ing)?>/giu, "").replace(/<tool_result(?:_[A-Za-z0-9_]+)?\b[^>]*>[\s\S]*?<\/tool_result(?:_[A-Za-z0-9_]+)?>/giu, "[\u5DE5\u5177\u7ED3\u679C\u5DF2\u7701\u7565]");
  if (message.sender === "user") content = content.replace(/<memory\b[^>]*>[\s\S]*?<\/memory>/giu, "");
  return { ...message, content: content.trim() };
}
function memoryByTitle(memories, title) {
  const matches = memories.filter(
    /** Matches the complete authored title without substring inference. */
    (memory) => memory.title === title
  );
  if (matches.length !== 1) throw new Error("Memory title does not identify exactly one record: " + title);
  return matches[0];
}
function extractionPrompt(candidates, folders, settings2, profile) {
  const existing = candidates.length === 0 ? "\u6CA1\u6709\u5DF2\u6709\u8BB0\u5FC6\u3002" : "\u5DF2\u6709\u8BB0\u5FC6\uFF08\u4EC5\u662F\u68C0\u7D22\u7EBF\u7D22\uFF0C\u4E0D\u662F\u4E8B\u5B9E\u8BC1\u636E\uFF09\uFF1A\n" + candidates.map(
    /** Preserves the title and a bounded real-record content sample. */
    (memory) => "- " + JSON.stringify(memory.title) + ": " + [...memory.content.replace(/\n/g, " ")].slice(0, 150).join("")
  ).join("\n");
  const enabled = settings2.profileAutoUpdateEnabled && !settings2.profileAutoUpdateLocked;
  const profileInstruction = enabled ? "\u3010\u5F53\u524D\u8BB0\u5FC6\u7A7A\u95F4\u8D44\u6599\u3011\n<user_profile_document>\n" + profile + "\n</user_profile_document>\n\u7A33\u5B9A\u7528\u6237\u504F\u597D\u3001\u7EA6\u675F\u3001\u8EAB\u4EFD\u4E8B\u5B9E\u6216\u4EA4\u6D41\u65B9\u5F0F\u88AB\u786E\u8BA4\u65F6\uFF0C\u4FDD\u7559\u5DF2\u6709\u6709\u6548 Markdown\uFF0C\u8FD4\u56DE\u5B8C\u6574\u66FF\u6362 profile_markdown\uFF1B\u6CA1\u6709\u5145\u5206\u4F9D\u636E\u8FD4\u56DE null\uFF0C\u4E0D\u8BB0\u5F55\u4E34\u65F6\u8BF7\u6C42\u6216\u5E38\u8BC6\u3002" : "";
  const custom = "\u3010\u7528\u6237\u6307\u5B9A\u7684\u8BB0\u5FC6\u63D0\u53D6\u9644\u52A0\u89C4\u5219\u3011\n<memory_extraction_custom_rules>\n" + settings2.memoryExtractionCustomRules + "\n</memory_extraction_custom_rules>\n\u7B5B\u9009\u3001\u8BC1\u636E\u8981\u6C42\u548C\u4E25\u683C JSON \u8F93\u51FA\u534F\u8BAE\u4ECD\u7136\u5FC5\u987B\u9075\u5B88\u3002";
  const fields5 = [
    ["$duplicatesPromptPart", ""],
    ["$existingMemoriesPrompt", existing],
    ["$existingFoldersPrompt", "\u5DF2\u6709\u6587\u4EF6\u5939\uFF1A" + folders.join(", ")],
    ["$profileOptionalKey", enabled ? "\u3001profile_markdown" : ""],
    ["$profileMarkdownSchemaLine", enabled ? "- profile_markdown\uFF1A\u5B8C\u6574\u66FF\u6362 Markdown\uFF0C\u672A\u66F4\u65B0\u65F6\u4E3A null\u3002" : ""],
    ["$profileUpdateInstruction", profileInstruction],
    ["$memoryExtractionCustomRulesInstruction", custom]
  ];
  let prompt = extractionTemplate;
  for (const [token, content] of fields5) prompt = prompt.split(token).join(content);
  return prompt;
}
function mergeMemories(ownerKey, item, repository) {
  return __async(this, null, function* () {
    const space = yield repository.readMemorySpace(ownerKey), sources = item.source_titles.map(
      /** Requires every stated source before starting the merge. */
      (title) => memoryByTitle(space.memories, title)
    );
    const ids = /* @__PURE__ */ new Set();
    for (const source of sources) {
      if (ids.has(source.id)) throw new Error("Duplicate memory merge source");
      ids.add(source.id);
    }
    const primary = sources[0], redundant = sources.slice(1);
    if (sources.some(
      /** Forbids implicit destruction or reassignment of document chunks during a textual merge. */
      (source) => source.isDocumentNode
    )) throw new Error("Document memories require explicit document composition, not a textual AI merge");
    let credibility = 0, importance = 0;
    for (const source of sources) {
      credibility = Math.max(credibility, source.credibility);
      importance = Math.max(importance, source.importance);
    }
    for (const link2 of space.links) {
      if (ids.has(link2.sourceMemoryId)) link2.sourceMemoryId = primary.id;
      if (ids.has(link2.targetMemoryId)) link2.targetMemoryId = primary.id;
    }
    space.links = space.links.filter(
      /** Removes only self-links created by combining two original endpoints. */
      (link2) => link2.sourceMemoryId !== link2.targetMemoryId
    );
    const redundantIds = /* @__PURE__ */ new Set();
    for (const memory of redundant) redundantIds.add(memory.id);
    space.memories = space.memories.filter(
      /** Retains the primary complete record and all unrelated full records. */
      (memory) => !redundantIds.has(memory.id)
    );
    yield repository.writeMemorySpace(ownerKey, space);
    return repository.saveMemory(ownerKey, {
      ...primary,
      title: item.title,
      content: item.content,
      folderPath: item.folder_path,
      contentType: "text",
      source: "memory_analysis",
      credibility,
      importance,
      tags: normalizeNames(item.tags).map(
        /** Initializes explicit editable tags without numeric identity loss. */
        (name, index) => ({ id: String(index + 1), name })
      )
    });
  });
}
function applyAnalysis(ownerKey, result2, repository) {
  return __async(this, null, function* () {
    const report = { ownerKey, created: 0, updated: 0, merged: 0, links: 0, profileUpdated: false };
    if (result2 === null) return report;
    const aliases = /* @__PURE__ */ new Map();
    for (const item of result2.merge) {
      const memory = yield mergeMemories(ownerKey, item, repository);
      aliases.set(memory.title, memory);
      report.merged += 1;
    }
    for (const item of result2.update) {
      const original = memoryByTitle(yield repository.listMemories(ownerKey), item.title);
      const memory = { ...original, content: item.content };
      if (item.credibility !== void 0 && item.credibility !== null) memory.credibility = item.credibility;
      if (item.importance !== void 0 && item.importance !== null) memory.importance = item.importance;
      const saved = yield repository.saveMemory(ownerKey, memory);
      aliases.set(saved.title, saved);
      report.updated += 1;
    }
    if (result2.main !== null) {
      const memories = yield repository.listMemories(ownerKey), matches = memories.filter(
        /** Matches a main event's exact existing title. */
        (memory) => memory.title === result2.main?.title
      );
      if (matches.length > 1) throw new Error("Main event title is ambiguous");
      if (matches.length === 1) {
        const saved = yield repository.saveMemory(ownerKey, { ...matches[0], content: result2.main.content });
        aliases.set(saved.title, saved);
        report.updated += 1;
      } else {
        const saved = yield repository.createMemory(ownerKey, { title: result2.main.title, content: result2.main.content, contentType: "text", source: "memory_analysis", credibility: 1, importance: 0.8, folderPath: result2.main.folder_path, tags: result2.main.tags });
        aliases.set(saved.title, saved);
        report.created += 1;
      }
    }
    for (const item of result2.new) {
      if (item.alias_for !== void 0 && item.alias_for !== null && item.alias_for.trim() !== "") {
        const target = aliases.get(item.alias_for);
        const actual = target === void 0 ? memoryByTitle(yield repository.listMemories(ownerKey), item.alias_for) : target;
        aliases.set(item.title, actual);
      } else {
        const saved = yield repository.createMemory(ownerKey, { title: item.title, content: item.content, contentType: "text", source: "memory_analysis", credibility: 0.5, importance: 0.5, folderPath: item.folder_path, tags: item.tags });
        aliases.set(item.title, saved);
        report.created += 1;
      }
    }
    for (const item of result2.links) {
      const memories = yield repository.listMemories(ownerKey), sourceAlias = aliases.get(item.source), targetAlias = aliases.get(item.target);
      const source = sourceAlias === void 0 ? memoryByTitle(memories, item.source) : sourceAlias, target = targetAlias === void 0 ? memoryByTitle(memories, item.target) : targetAlias;
      yield repository.createLink(ownerKey, { sourceMemoryId: source.id, targetMemoryId: target.id, type_: item.type, description: item.description, weight: item.weight });
      report.links += 1;
    }
    if (result2.profile_markdown !== void 0 && result2.profile_markdown !== null) {
      const settings2 = yield repository.readMemorySettings(ownerKey);
      if (!settings2.profileAutoUpdateEnabled || settings2.profileAutoUpdateLocked) throw new Error("Model returned a profile update forbidden by the owner settings");
      yield repository.writeUser(ownerKey, result2.profile_markdown);
      report.profileUpdated = true;
    }
    return report;
  });
}
function extractMessages(ownerKey, messages, repository) {
  return __async(this, null, function* () {
    const history = messages.map(cleanMessage).filter(
      /** Selects actual conversation content, excluding unsupported senders and blank records. */
      (message) => (message.sender === "user" || message.sender === "ai" || message.sender === "assistant") && message.content !== ""
    );
    const user = [...history].reverse().find(
      /** Requires genuine user context rather than analyzing an orphan assistant reply. */
      (message) => message.sender === "user"
    );
    if (user === void 0) throw new Error("Memory extraction requires user context");
    const space = yield repository.readMemorySpace(ownerKey), document = yield repository.readUser(ownerKey);
    const query = history.slice(-12).map(
      /** Builds a bounded candidate query from genuine recent conversation content. */
      (message) => [...message.content].slice(0, 800).join("")
    ).join("\n");
    const candidates = yield repository.searchMemories({ ownerKey, query, folderPath: null, relevanceThreshold: 0, createdAtStartMs: null, createdAtEndMs: null });
    const folders = yield repository.listMemoryFolders(ownerKey), prompt = extractionPrompt(candidates.slice(0, 15), folders, space.settings, document.content);
    const response = yield Tools.Chat.call({ functionType: "MEMORY", turns: [{ kind: "SYSTEM", content: prompt }, { kind: "USER", content: "\u5BF9\u8BDD\u8BB0\u5F55\uFF1A\n" + history.map(
      /** Retains the real user and assistant sequence in the selected extraction window. */
      (message) => message.sender + ": " + message.content
    ).join("\n\n") }], recordTokenUsage: true, enableThinking: false });
    return applyAnalysis(ownerKey, analysis(parseModelJson(response.text)), repository);
  });
}
function updateChatMemory(ownerKey, chatId, repository) {
  return __async(this, null, function* () {
    yield requireChat(chatId);
    const messages = yield readMessages(chatId), assistant2 = [...messages].reverse().find(
      /** Selects the last real finalized assistant reply for the manual update. */
      (message) => (message.sender === "ai" || message.sender === "assistant") && message.content.trim() !== ""
    );
    if (assistant2 === void 0) throw new Error("Manual memory update requires an assistant reply");
    yield requireMessageOwner(chatId, assistant2, ownerKey, repository);
    return extractMessages(ownerKey, messages.filter(
      /** Bounds the manual analysis to real messages at or before that reply. */
      (message) => message.timestamp <= assistant2.timestamp
    ).slice(-10), repository);
  });
}
function autoCategorizeMemory(ownerKey, repository) {
  return __async(this, null, function* () {
    const memories = (yield repository.listMemories(ownerKey)).filter(
      /** Selects only records genuinely stored in the root folder. */
      (memory) => memory.folderPath === null || memory.folderPath === ""
    ), folders = yield repository.listMemoryFolders(ownerKey);
    let changed = 0;
    for (let index = 0; index < memories.length; index += 10) {
      const batch = memories.slice(index, index + 10), digest = batch.map(
        /** Supplies real titles and a bounded full-record content preview. */
        (memory) => "- title: " + memory.title + ", content: " + [...memory.content].slice(0, 100).join("")
      ).join("\n");
      const response = yield Tools.Chat.call({ functionType: "MEMORY", turns: [{ kind: "SYSTEM", content: "\u4F60\u662F\u77E5\u8BC6\u5206\u7C7B\u4E13\u5BB6\u3002\u6839\u636E\u8BB0\u5FC6\u5185\u5BB9\uFF0C\u4E3A\u6BCF\u6761\u8BB0\u5FC6\u5206\u914D\u5408\u9002\u7684\u6587\u4EF6\u5939\u8DEF\u5F84\u3002\n\u5DF2\u6709\u6587\u4EF6\u5939\uFF1A" + folders.join(", ") + '\n\u4F18\u5148\u4F7F\u7528\u5DF2\u6709\u6587\u4EF6\u5939\uFF0C\u5FC5\u8981\u65F6\u521B\u5EFA\u65B0\u6587\u4EF6\u5939\u3002\u4EC5\u8FD4\u56DE\u4E25\u683C JSON \u6570\u7EC4 [{"title":"\u8BB0\u5FC6\u6807\u9898","folder":"\u6587\u4EF6\u5939\u8DEF\u5F84"}]\u3002\n\u8BB0\u5FC6\u5217\u8868\uFF1A\n' + digest }, { kind: "USER", content: "\u8BF7\u4E3A\u8FD9\u4E9B\u8BB0\u5FC6\u5206\u7C7B\u3002" }], recordTokenUsage: true, enableThinking: false });
      const rows = parseModelJson(response.text);
      if (!Array.isArray(rows) || rows.length !== batch.length) throw new Error("Categorization must return exactly one folder for each supplied memory");
      const seen = /* @__PURE__ */ new Set();
      for (const row of rows) {
        assertObject(row, "category");
        const title = requireName(row.title, "category title");
        assertString(row.folder, "category folder");
        if (seen.has(title)) throw new Error("Duplicate category title: " + title);
        seen.add(title);
        const memory = memoryByTitle(batch, title);
        yield repository.moveMemories(ownerKey, [memory.id], row.folder);
        changed += 1;
      }
    }
    return changed;
  });
}

// src/memory-jobs/embeddings.ts
function assertEmbedding(value) {
  assertObject(value, "embedding");
  requireName(value.endpoint, "embedding endpoint");
  requireName(value.model, "embedding model");
  assertString(value.text, "embedding text");
  assertInteger(value.updatedAt, "embedding timestamp", 0);
  if (!Array.isArray(value.vector) || value.vector.length === 0) throw new Error("Embedding must contain a nonempty vector");
  let norm = 0;
  for (const component of value.vector) {
    assertNumber(component, "embedding component", -Number.MAX_VALUE, Number.MAX_VALUE);
    norm += component * component;
  }
  if (!Number.isFinite(norm) || norm === 0) throw new Error("Embedding vector has an invalid norm");
}
function embeddingSource(settings2) {
  if (!settings2.cloudEmbeddingEnabled) throw new Error("Cloud embedding is disabled");
  const endpoint = requireName(settings2.cloudEmbeddingEndpoint, "embedding endpoint").replace(/\/+$/, ""), model = requireName(settings2.cloudEmbeddingModel, "embedding model");
  if (!/^https?:\/\//u.test(endpoint)) throw new Error("Embedding endpoint must be an absolute HTTP URL");
  return { endpoint, model };
}
function requestEmbedding(text3, settings2) {
  return __async(this, null, function* () {
    const source = embeddingSource(settings2), headers = { "Content-Type": "application/json" };
    if (settings2.cloudEmbeddingApiKey !== "") headers.Authorization = "Bearer " + settings2.cloudEmbeddingApiKey;
    const response = yield Tools.Net.http({
      url: source.endpoint.endsWith("/embeddings") ? source.endpoint : source.endpoint + "/embeddings",
      method: "POST",
      headers,
      body: JSON.stringify({ input: text3, model: source.model, encoding_format: "float" }),
      connect_timeout: 30,
      read_timeout: 60,
      follow_redirects: true,
      ignore_ssl: false,
      responseType: "text",
      validateStatus: false
    });
    if (response.statusCode < 200 || response.statusCode >= 300) throw new Error("Embedding request failed: HTTP " + response.statusCode + " " + response.content);
    assertString(response.content, "embedding response");
    const payload = JSON.parse(response.content);
    assertObject(payload, "embedding response");
    if (!Array.isArray(payload.data) || payload.data.length !== 1) throw new Error("Embedding response must contain exactly one input vector");
    const record2 = payload.data[0];
    assertObject(record2, "embedding response item");
    const embedding = { ...source, text: text3, vector: record2.embedding, updatedAt: Date.now() };
    assertEmbedding(embedding);
    return embedding;
  });
}
function embeddingFor(space, text3) {
  return __async(this, null, function* () {
    assertString(text3, "embedding input");
    const source = embeddingSource(space.settings);
    const matches = space.embeddings.filter(
      /** Matches the complete provider source and input without hashing away record provenance. */
      (item) => item.endpoint === source.endpoint && item.model === source.model && item.text === text3
    );
    if (matches.length > 1) throw new Error("Duplicate stored embedding input");
    if (matches.length === 1) {
      assertEmbedding(matches[0]);
      return matches[0].vector;
    }
    const computed = yield requestEmbedding(text3, space.settings);
    space.embeddings.push(computed);
    return computed.vector;
  });
}
function cosineSimilarity(left, right) {
  if (left.length === 0 || left.length !== right.length) throw new Error("Embedding dimensions do not match");
  let dot = 0, leftNorm = 0, rightNorm = 0;
  for (let index = 0; index < left.length; index += 1) {
    dot += left[index] * right[index];
    leftNorm += left[index] ** 2;
    rightNorm += right[index] ** 2;
  }
  if (leftNorm === 0 || rightNorm === 0) throw new Error("Embedding norm must be positive");
  return Math.max(-1, Math.min(1, dot / Math.sqrt(leftNorm * rightNorm)));
}
function rebuildMemoryEmbeddings(ownerKey, repository) {
  return __async(this, null, function* () {
    const space = yield repository.readMemorySpace(ownerKey);
    embeddingSource(space.settings);
    const texts = /* @__PURE__ */ new Set();
    for (const memory of space.memories) texts.add(memory.isDocumentNode ? memory.title : memory.content);
    for (const chunk of space.chunks) texts.add(chunk.content);
    const computed = [];
    for (const text3 of texts) computed.push(yield requestEmbedding(text3, space.settings));
    space.embeddings = computed;
    yield repository.writeMemorySpace(ownerKey, space);
    return space.memories.length;
  });
}

// src/memory-jobs/scheduler.ts
function errorText(error) {
  if (error !== null && typeof error === "object" && "message" in error && typeof error.message === "string") return error.message;
  return String(error);
}
var MemoryJobStatusError = class extends Error {
  /** Exposes the original execution error together with the status write that also failed. */
  constructor(executionError, statusPublicationError) {
    super("Memory job failed: " + errorText(executionError) + "; status publication also failed: " + errorText(statusPublicationError));
    this.executionError = executionError;
    this.statusPublicationError = statusPublicationError;
    this.name = "MemoryJobStatusError";
  }
};
function propagateJobFailure(error, publishStatus) {
  return __async(this, null, function* () {
    try {
      yield publishStatus();
    } catch (statusError) {
      throw new MemoryJobStatusError(error, statusError);
    }
    throw error;
  });
}
function enqueueCandidate(input, repository) {
  return __async(this, null, function* () {
    yield requireChat(input.chatId);
    assertInteger(input.timestamp, "candidate message timestamp", 0);
    assertInteger(input.variantIndex, "candidate variant index", 0);
    if (input.variantIndex > 2147483647) throw new Error("Candidate variant exceeds the host range");
    const messages = yield readMessages(input.chatId), matches = messages.filter(
      /** Resolves the message by its genuine persisted timestamp. */
      (message2) => message2.timestamp === input.timestamp && message2.variantIndex === input.variantIndex
    );
    if (matches.length !== 1) throw new Error("Candidate does not identify exactly one source message");
    const message = matches[0];
    switch (input.sourceType) {
      case "reply_finalized_auto":
        if (message.sender !== "ai" && message.sender !== "assistant") throw new Error("Automatic candidate requires an assistant reply");
        break;
      case "selected_user_message":
        if (message.sender !== "user") throw new Error("Selected candidate requires a user message");
        break;
      default:
        throw new Error("Unknown candidate source");
    }
    if (message.content.trim() === "") throw new Error("Memory candidate source is blank");
    const owners = /* @__PURE__ */ new Set();
    if (input.sourceType === "reply_finalized_auto") {
      const marker = yield readMessageExtensionMarker(input.chatId, message.timestamp, message.variantIndex);
      const ownerKey = input.ownerKey === null ? marker.primaryOwnerKey : input.ownerKey;
      if (!marker.profile.resources.some(
        /** Authorizes only the exact submitted resource through the already-read historical snapshot. */
        (resource) => resource.key === ownerKey && resource.writable
      )) throw new Error("Saved candidate identity cannot write this memory owner: " + ownerKey);
      yield repository.readMemorySpace(ownerKey);
      owners.add(ownerKey);
    } else if (input.ownerKey !== null) {
      yield requireChatOwner(input.chatId, input.ownerKey, repository);
      owners.add(input.ownerKey);
    } else {
      const participants = yield chatParticipants(input.chatId, repository);
      if (participants.length !== 1) throw new Error("A group selected-user candidate requires an explicit owner");
      owners.add(yield primaryOwner(participants[0], repository));
    }
    const result2 = { owners: [...owners], candidateIds: [] }, now = Date.now();
    for (const ownerKey of owners) {
      const space = yield repository.readMemorySpace(ownerKey), previous = space.candidates.filter(
        /** Uses the full owner-scoped source identity for genuine hook idempotency. */
        (candidate) => candidate.chatId === input.chatId && candidate.triggerMessageTimestamp === input.timestamp && candidate.triggerVariantIndex === input.variantIndex && candidate.sourceType === input.sourceType
      );
      if (previous.length > 1) throw new Error("Duplicate persisted memory candidate source");
      if (previous.length === 1) {
        result2.candidateIds.push(previous[0].id);
        continue;
      }
      const id2 = yield repository.allocateRecordId();
      space.candidates.push({ id: id2, chatId: input.chatId, triggerMessageTimestamp: input.timestamp, triggerVariantIndex: input.variantIndex, createdAt: now, updatedAt: now, status: "pending", attemptCount: 0, lastError: "", sourceType: input.sourceType });
      if (space.settings.autoSaveIntervalMinutes > 0 && space.settings.nextAutoSaveRunAtMs === 0) space.settings.nextAutoSaveRunAtMs = now + space.settings.autoSaveIntervalMinutes * 6e4;
      yield repository.writeMemorySpace(ownerKey, space);
      result2.candidateIds.push(id2);
    }
    return result2;
  });
}
function startMemoryRebuild(ownerKey, rebuild, repository) {
  return __async(this, null, function* () {
    assertObject(rebuild, "memory rebuild");
    assertStrings(rebuild.chatIds, "rebuild chat ids");
    assertInteger(rebuild.windowMessageCount, "rebuild window size", 1);
    for (const key of ["fromInclusive", "toInclusive"]) if (rebuild[key] !== null) assertInteger(rebuild[key], key, 0);
    if (rebuild.fromInclusive !== null && rebuild.toInclusive !== null && rebuild.fromInclusive > rebuild.toInclusive) throw new Error("Rebuild start must not follow its end");
    if (rebuild.chatIds.length === 0) throw new Error("Rebuild requires selected chats");
    const space = yield repository.readMemorySpace(ownerKey);
    if (space.rebuildProgress.status === "preparing" || space.rebuildProgress.status === "running") throw new Error("Memory rebuild is already active");
    const task = { id: yield repository.allocateRecordId(), windows: [], nextWindow: 0 }, ids = /* @__PURE__ */ new Set();
    for (const id2 of rebuild.chatIds) {
      requireId(id2, "rebuild chat id");
      if (ids.has(id2)) throw new Error("Duplicate selected rebuild chat: " + id2);
      ids.add(id2);
      const chat = yield requireChat(id2), messages = yield readMessages(id2);
      yield requireMessagesOwner(id2, messages, ownerKey, repository);
      const windows = planWindows(chat, messages, rebuild);
      for (const window of windows) yield requireMessagesOwner(id2, window.messages, ownerKey, repository);
      task.windows.push(...windows);
    }
    if (task.windows.length === 0) throw new Error("Selected histories have no user context in the specified range");
    let totalSourceMessages = 0;
    const plannedChats = /* @__PURE__ */ new Set();
    for (const window of task.windows) {
      totalSourceMessages += window.sourceMessageCount;
      plannedChats.add(window.chatId);
    }
    space.rebuildTask = task;
    space.rebuildProgress = {
      status: "preparing",
      totalChats: plannedChats.size,
      completedChats: 0,
      totalWindows: task.windows.length,
      completedWindows: 0,
      totalSourceMessages,
      processedSourceMessages: 0,
      failedWindows: 0,
      currentChatTitle: "",
      lastError: ""
    };
    yield repository.writeMemorySpace(ownerKey, space);
    return space.rebuildProgress;
  });
}
function processRebuildWindow(ownerKey, taskId, owner2) {
  return __async(this, null, function* () {
    try {
      yield owner2.run(
        /** Keeps one model result, graph mutation and progress checkpoint in the same publication. */
        (repository) => __async(null, null, function* () {
          const space = yield repository.readMemorySpace(ownerKey), task = space.rebuildTask;
          if (task === null || task.id !== taskId) throw new Error("Rebuild task identity changed");
          if (space.rebuildProgress.status === "cancelled") return;
          if (space.rebuildProgress.status !== "preparing" && space.rebuildProgress.status !== "running") throw new Error("Rebuild task is not runnable");
          const window = task.windows[task.nextWindow];
          if (window === void 0) throw new Error("Rebuild cursor has no source window");
          const currentMessages = yield readMessages(window.chatId);
          for (const source of window.messages) {
            const present = currentMessages.filter(
              /** Matches the exact planned revision before trusting its original extraction content. */
              (message) => message.timestamp === source.timestamp && message.variantIndex === source.variantIndex
            );
            if (present.length !== 1 || JSON.stringify(present[0]) !== JSON.stringify(source)) throw new Error("A planned rebuild source message revision changed or was deleted");
          }
          yield requireMessagesOwner(window.chatId, window.messages, ownerKey, repository);
          yield extractMessages(ownerKey, window.messages, repository);
          const updated = yield repository.readMemorySpace(ownerKey), savedTask = updated.rebuildTask;
          if (savedTask === null || savedTask.id !== taskId || savedTask.nextWindow !== task.nextWindow) throw new Error("Rebuild cursor changed within execution");
          savedTask.nextWindow += 1;
          updated.rebuildProgress.completedWindows += 1;
          updated.rebuildProgress.processedSourceMessages += window.sourceMessageCount;
          updated.rebuildProgress.currentChatTitle = window.chatTitle;
          const next = savedTask.windows[savedTask.nextWindow];
          if (next === void 0 || next.chatId !== window.chatId) updated.rebuildProgress.completedChats += 1;
          updated.rebuildProgress.status = next === void 0 ? "completed" : "running";
          yield repository.writeMemorySpace(ownerKey, updated);
        })
      );
    } catch (error) {
      yield propagateJobFailure(
        error,
        /** Persists terminal metadata through the same authoritative queue without retrying the failed work. */
        () => owner2.run(
          /** Records the real terminal failure after the unsuccessful operation has published no edits. */
          (repository) => __async(null, null, function* () {
            const space = yield repository.readMemorySpace(ownerKey);
            if (space.rebuildTask === null || space.rebuildTask.id !== taskId) throw new Error("Failed rebuild task identity changed");
            if (space.rebuildProgress.status !== "cancelled") {
              space.rebuildProgress.status = "failed";
              space.rebuildProgress.failedWindows += 1;
              space.rebuildProgress.lastError = errorText(error);
              yield repository.writeMemorySpace(ownerKey, space);
            }
          })
        )
      );
    }
  });
}
function processCandidates(ownerKey, candidates, owner2) {
  return __async(this, null, function* () {
    const ids = /* @__PURE__ */ new Set();
    for (const candidate of candidates) ids.add(candidate.id);
    const chatId = candidates[0].chatId;
    try {
      yield owner2.run(
        /** Validates the current chat owner and applies a genuine selected/automatic message batch. */
        (repository) => __async(null, null, function* () {
          yield requireChat(chatId);
          let messages = yield readMessages(chatId);
          for (const candidate of candidates) {
            const source = messages.filter(
              /** Revalidates each claimed source identity before any analysis or destructive queue update. */
              (message) => message.timestamp === candidate.triggerMessageTimestamp && message.variantIndex === candidate.triggerVariantIndex
            );
            if (source.length !== 1) throw new Error("A claimed candidate source no longer identifies exactly one message: " + candidate.id);
            const sender = source[0].sender;
            if (candidate.sourceType === "selected_user_message" ? sender !== "user" : sender !== "ai" && sender !== "assistant") throw new Error("A claimed candidate source has changed sender: " + candidate.id);
            if (candidate.sourceType === "reply_finalized_auto") yield requireMessageOwner(chatId, source[0], ownerKey, repository);
            else yield requireChatOwner(chatId, ownerKey, repository);
          }
          if (candidates.every(
            /** Recognizes only the exact declared selected-user source type. */
            (candidate) => candidate.sourceType === "selected_user_message"
          )) {
            const timestamps = /* @__PURE__ */ new Set();
            for (const candidate of candidates) timestamps.add(candidate.triggerMessageTimestamp);
            messages = messages.filter(
              /** Selects the genuinely persisted requested user messages. */
              (message) => message.sender === "user" && timestamps.has(message.timestamp)
            );
            if (messages.length !== timestamps.size) throw new Error("A selected candidate message no longer exists");
          } else {
            let lastTimestamp = 0;
            for (const candidate of candidates) lastTimestamp = Math.max(lastTimestamp, candidate.triggerMessageTimestamp);
            messages = messages.filter(
              /** Bounds automatic extraction to actual source records at the final candidate. */
              (message) => message.timestamp <= lastTimestamp
            ).slice(-48);
          }
          yield extractMessages(ownerKey, messages, repository);
          const space = yield repository.readMemorySpace(ownerKey);
          space.candidates = space.candidates.filter(
            /** Removes only the explicitly claimed batch after successful domain mutations. */
            (candidate) => !ids.has(candidate.id)
          );
          yield repository.writeMemorySpace(ownerKey, space);
        })
      );
    } catch (error) {
      yield propagateJobFailure(
        error,
        /** Persists terminal metadata through the same authoritative queue without retrying the failed work. */
        () => owner2.run(
          /** Persists a real failed state without automatic retries or successful empty output. */
          (repository) => __async(null, null, function* () {
            const space = yield repository.readMemorySpace(ownerKey);
            for (const candidate of space.candidates) if (ids.has(candidate.id)) {
              candidate.status = "failed";
              candidate.lastError = errorText(error);
              candidate.updatedAt = Date.now();
            }
            yield repository.writeMemorySpace(ownerKey, space);
          })
        )
      );
    }
  });
}
var MemoryJobRunner = class {
  /** Receives the same sole repository owner used by UI, commands and public APIs. */
  constructor(owner2) {
    this.owner = owner2;
    this.busy = false;
    this.waiters = [];
  }
  /** Serializes genuine interval invocations without launching overlapping extraction batches. */
  acquire() {
    return __async(this, null, function* () {
      if (!this.busy) {
        this.busy = true;
        return;
      }
      yield new Promise(
        /** Parks only this invocation until the previous interval work completes. */
        (resolve) => this.waiters.push(resolve)
      );
    });
  }
  /** Releases one waiting interval while retaining the same runtime owner. */
  release() {
    const next = this.waiters.shift();
    if (next === void 0) this.busy = false;
    else next();
  }
  /** Advances actual rebuild windows and due pending candidates through existing interval hooks. */
  tick() {
    return __async(this, null, function* () {
      yield this.acquire();
      try {
        const keys = yield this.owner.run(
          /** Enumerates only genuine initialized memory spaces. */
          (repository) => repository.listMemoryOwnerKeys()
        ), report = { owners: keys.length, rebuildWindows: 0, candidateBatches: 0 };
        for (const ownerKey of keys) {
          const taskId = yield this.owner.run(
            /** Reads a persisted runnable rebuild identity without fabricating a job. */
            (repository) => __async(this, null, function* () {
              const space = yield repository.readMemorySpace(ownerKey);
              return space.rebuildTask !== null && (space.rebuildProgress.status === "preparing" || space.rebuildProgress.status === "running") ? space.rebuildTask.id : null;
            })
          );
          if (taskId !== null) {
            yield processRebuildWindow(ownerKey, taskId, this.owner);
            report.rebuildWindows += 1;
          }
          const batches = yield this.owner.run(
            /** Claims only due pending records; failed attempts are never automatically retried. */
            (repository) => __async(this, null, function* () {
              const space = yield repository.readMemorySpace(ownerKey), now = Date.now();
              if (space.settings.autoSaveIntervalMinutes === 0 || now < space.settings.nextAutoSaveRunAtMs) return [];
              const pending = space.candidates.filter(
                /** Selects genuinely pending records, not terminal failures. */
                (candidate) => candidate.status === "pending"
              );
              const batches2 = [];
              if (pending.length >= 5) {
                const grouped = /* @__PURE__ */ new Map();
                for (const candidate of pending) {
                  const list = grouped.get(candidate.chatId);
                  if (list === void 0) grouped.set(candidate.chatId, [candidate]);
                  else list.push(candidate);
                }
                for (const records2 of grouped.values()) {
                  const capped = records2.slice(0, 20);
                  for (const sourceType of ["selected_user_message", "reply_finalized_auto"]) {
                    const batch = capped.filter(
                      /** Preserves the original selected/automatic source partition. */
                      (candidate) => candidate.sourceType === sourceType
                    );
                    if (batch.length !== 0) batches2.push(batch);
                  }
                }
              }
              space.settings.nextAutoSaveRunAtMs = now + space.settings.autoSaveIntervalMinutes * 6e4;
              yield repository.writeMemorySpace(ownerKey, space);
              return batches2;
            })
          );
          for (const batch of batches) {
            yield this.owner.run(
              /** Claims only the next actual batch so a failure leaves later sources genuinely pending. */
              (repository) => __async(this, null, function* () {
                const space = yield repository.readMemorySpace(ownerKey), ids = /* @__PURE__ */ new Set();
                for (const candidate of batch) ids.add(candidate.id);
                let count = 0;
                for (const candidate of space.candidates) if (ids.has(candidate.id)) {
                  if (candidate.status !== "pending") throw new Error("Candidate is no longer pending: " + candidate.id);
                  candidate.status = "processing";
                  candidate.attemptCount += 1;
                  candidate.updatedAt = Date.now();
                  count += 1;
                }
                if (count !== batch.length) throw new Error("A selected candidate no longer exists");
                yield repository.writeMemorySpace(ownerKey, space);
              })
            );
            yield processCandidates(ownerKey, batch, this.owner);
            report.candidateBatches += 1;
          }
        }
        return report;
      } finally {
        this.release();
      }
    });
  }
};

// src/storage/state.ts
function createDefaultCharacter(now) {
  return { ...createCharacterDraft(), id: "default", name: "\u9ED8\u8BA4\u89D2\u8272", description: "\u901A\u7528\u52A9\u624B", characterSetting: "\u4F60\u662F\u4E00\u4E2A\u4E50\u4E8E\u52A9\u4EBA\u3001\u8BDA\u5B9E\u4E14\u4E25\u8C28\u7684\u52A9\u624B\u3002", isDefault: true, createdAt: now, updatedAt: now };
}
function userDocumentPath(ownerKey) {
  return "owners/" + encodeURIComponent(ownerKey) + "/USER.md";
}
function createMemorySpace(ownerKey) {
  return {
    ownerKey,
    memories: [],
    links: [],
    chunks: [],
    candidates: [],
    userDocumentPath: userDocumentPath(ownerKey),
    settings: { autoSaveIntervalMinutes: 5, nextAutoSaveRunAtMs: 0, memoryExtractionCustomRules: "", profileAutoUpdateEnabled: true, profileAutoUpdateLocked: false, cloudEmbeddingEnabled: false, cloudEmbeddingEndpoint: "", cloudEmbeddingApiKey: "", cloudEmbeddingModel: "" },
    searchConfig: { scoreMode: "BALANCED", keywordWeight: 10, tagWeight: 0, vectorWeight: 0, edgeWeight: 0.4 },
    rebuildTask: null,
    embeddings: [],
    rebuildProgress: { status: "idle", totalChats: 0, completedChats: 0, totalWindows: 0, completedWindows: 0, totalSourceMessages: 0, processedSourceMessages: 0, failedWindows: 0, currentChatTitle: "", lastError: "" }
  };
}
function createInitialState(now) {
  return { version: 3, conversationGroups: [], nextId: "1", cards: [createDefaultCharacter(now)], groups: [], tags: [], stores: [], active: { CharacterCard: { id: "default" } }, owners: [createMemorySpace("character:default")] };
}
function array2(value, label) {
  if (!Array.isArray(value)) throw new Error(label + " must be an array");
}
function identities(records2, label) {
  const ids = /* @__PURE__ */ new Set();
  for (const record2 of records2) {
    assertObject(record2, label);
    const id2 = requireId(record2.id, label + " id");
    if (ids.has(id2)) throw new Error(label + " has duplicate id: " + id2);
    ids.add(id2);
  }
  return ids;
}
function assertCharacterState(value) {
  assertObject(value, "character state");
  if (value.version !== 3) throw new Error("Unsupported character state file version");
  const fields5 = /* @__PURE__ */ new Set(["version", "nextId", "cards", "groups", "tags", "stores", "active", "owners", "conversationGroups"]);
  for (const key of Object.keys(value)) if (!fields5.has(key)) throw new Error("Unsupported character state field: " + key);
  const nextId = BigInt(requireDecimal(value.nextId, "nextId", true));
  assertConversationGroups(value.conversationGroups);
  for (const group of value.conversationGroups) if (BigInt(group.id) >= nextId) throw new Error("nextId does not exceed the conversation group identity");
  const cards = value.cards, groups = value.groups, tags = value.tags, stores = value.stores, owners = value.owners;
  array2(cards, "cards");
  array2(groups, "groups");
  array2(tags, "tags");
  array2(stores, "stores");
  array2(owners, "owners");
  const cardIds = identities(cards, "cards"), groupIds = identities(groups, "groups"), tagIds = identities(tags, "tags"), storeIds = identities(stores, "stores");
  let defaultCount = 0;
  for (const card of cards) {
    assertCard(card);
    requireId(card.id, "card id");
    if (card.isDefault) defaultCount += 1;
    for (const id2 of card.attachedTagIds) if (!tagIds.has(id2)) throw new Error("Missing attached tag: " + id2);
    if (card.chatModelBindingMode !== "FOLLOW_GLOBAL" && card.chatModelBindingMode !== "FIXED_MODEL") throw new Error("Invalid model binding mode");
    if (card.chatModelBindingMode === "FIXED_MODEL") requireId(card.chatModelId, "fixed model id");
    if (card.memoryBindingMode !== "CHARACTER" && card.memoryBindingMode !== "SHARED") throw new Error("Invalid memory binding mode");
    if (card.memoryBindingMode === "SHARED" && !storeIds.has(requireId(card.sharedMemoryId, "shared memory id"))) throw new Error("Missing primary shared library");
    for (const mount2 of card.sharedMemoryMounts) if (!storeIds.has(mount2.sharedMemoryId)) throw new Error("Missing shared memory mount: " + mount2.sharedMemoryId);
  }
  if (defaultCount !== 1) throw new Error("Character state must contain exactly one default character");
  for (const group of groups) {
    assertGroup(group);
    const members2 = /* @__PURE__ */ new Set();
    for (const member2 of group.members) {
      if (!cardIds.has(member2.characterCardId)) throw new Error("Missing group member: " + member2.characterCardId);
      if (members2.has(member2.characterCardId)) throw new Error("Duplicate group member: " + member2.characterCardId);
      members2.add(member2.characterCardId);
    }
  }
  for (const tag of tags) assertTag(tag);
  for (const store of stores) {
    assertObject(store, "store");
    assertString(store.name, "store name");
    assertInteger(store.createdAt, "store createdAt", 0);
    assertInteger(store.updatedAt, "store updatedAt", 0);
  }
  assertObject(value.active, "active");
  const activeKeys = Object.keys(value.active);
  if (activeKeys.length !== 1) throw new Error("Active prompt must select exactly one entity");
  const activeKey = activeKeys[0];
  if (activeKey !== "CharacterCard" && activeKey !== "CharacterGroup") throw new Error("Invalid active prompt kind");
  const selected = value.active[activeKey];
  assertObject(selected, "active selection");
  const activeId = requireId(selected.id, "active id");
  if (!(activeKey === "CharacterCard" ? cardIds : groupIds).has(activeId)) throw new Error("Active prompt references a missing entity: " + activeId);
  const expectedOwners = /* @__PURE__ */ new Set();
  for (const id2 of cardIds) expectedOwners.add("character:" + id2);
  for (const id2 of storeIds) expectedOwners.add("shared:" + id2);
  const actualOwners = /* @__PURE__ */ new Set();
  for (const owner2 of owners) {
    assertObject(owner2, "memory space");
    const ownerKey = requireId(owner2.ownerKey, "owner key");
    if (!expectedOwners.has(ownerKey) || actualOwners.has(ownerKey)) throw new Error("Invalid or duplicate memory owner: " + ownerKey);
    actualOwners.add(ownerKey);
    assertMemorySpace(owner2);
    if (owner2.rebuildTask !== null && BigInt(owner2.rebuildTask.id) >= nextId) throw new Error("nextId does not exceed the rebuild task identity");
    for (const field of ["memories", "links", "chunks", "candidates"]) {
      for (const record2 of owner2[field]) if (BigInt(record2.id) >= nextId) throw new Error("nextId does not exceed stored identity: " + record2.id);
    }
  }
  if (actualOwners.size !== expectedOwners.size) throw new Error("Character state is missing an owner memory space");
}
function assertMemorySpace(value) {
  assertObject(value, "memory space");
  requireId(value.ownerKey, "memory owner");
  array2(value.memories, "memories");
  array2(value.links, "links");
  array2(value.chunks, "chunks");
  array2(value.candidates, "candidates");
  const memoryIds = identities(value.memories, "memories");
  identities(value.links, "links");
  identities(value.chunks, "chunks");
  identities(value.candidates, "candidates");
  const memoriesByUuid = /* @__PURE__ */ new Map();
  for (const memory of value.memories) {
    assertMemory(memory);
    if (memoriesByUuid.has(memory.uuid)) throw new Error("Duplicate memory UUID: " + memory.uuid);
    memoriesByUuid.set(memory.uuid, memory);
  }
  for (const link2 of value.links) {
    assertObject(link2, "link");
    requireDecimal(link2.id, "link id", true);
    const source = requireDecimal(link2.sourceMemoryId, "source memory id", true), target = requireDecimal(link2.targetMemoryId, "target memory id", true);
    if (!memoryIds.has(source) || !memoryIds.has(target)) throw new Error("Relationship references a missing memory: " + link2.id);
    if (source === target) throw new Error("Relationship cannot point to itself");
    assertString(link2.type_, "link type");
    assertNumber(link2.weight, "link weight", 0, 1);
    assertString(link2.description, "link description");
  }
  const indices = /* @__PURE__ */ new Set();
  for (const chunk of value.chunks) {
    assertObject(chunk, "chunk");
    requireDecimal(chunk.id, "chunk id", true);
    const memoryUuid = requireId(chunk.memoryUuid, "chunk memory UUID");
    assertInteger(chunk.chunkIndex, "chunk index", 0);
    assertString(chunk.content, "chunk content");
    const memory = memoriesByUuid.get(memoryUuid);
    if (memory === void 0 || !memory.isDocumentNode) throw new Error("Chunk references a missing document memory");
    const identity4 = JSON.stringify([chunk.memoryUuid, chunk.chunkIndex]);
    if (indices.has(identity4)) throw new Error("Duplicate document chunk index");
    indices.add(identity4);
  }
  const candidateSources = /* @__PURE__ */ new Set();
  for (const candidate of value.candidates) {
    assertObject(candidate, "candidate");
    requireDecimal(candidate.id, "candidate id", true);
    requireId(candidate.chatId, "candidate chat id");
    for (const field of ["triggerMessageTimestamp", "triggerVariantIndex", "createdAt", "updatedAt", "attemptCount"]) assertInteger(candidate[field], field, 0);
    for (const field of ["status", "lastError", "sourceType"]) assertString(candidate[field], field);
    if (candidate.status !== "pending" && candidate.status !== "processing" && candidate.status !== "failed") throw new Error("Invalid candidate status");
    if (candidate.sourceType !== "reply_finalized_auto" && candidate.sourceType !== "selected_user_message") throw new Error("Invalid candidate source type");
    if (typeof candidate.triggerVariantIndex !== "number" || candidate.triggerVariantIndex > 2147483647) throw new Error("Candidate message variant exceeds the host range");
    const sourceKey = JSON.stringify([candidate.chatId, candidate.triggerMessageTimestamp, candidate.triggerVariantIndex, candidate.sourceType]);
    if (candidateSources.has(sourceKey)) throw new Error("Duplicate persisted candidate source");
    candidateSources.add(sourceKey);
  }
  assertMemorySettings(value.settings);
  assertSearchConfig(value.searchConfig);
  assertString(value.userDocumentPath, "USER.md path");
  if (value.userDocumentPath !== userDocumentPath(requireId(value.ownerKey, "owner key"))) throw new Error("Invalid owner USER.md path");
  assertObject(value.rebuildProgress, "rebuild progress");
  for (const field of ["status", "currentChatTitle", "lastError"]) assertString(value.rebuildProgress[field], field);
  for (const field of ["totalChats", "completedChats", "totalWindows", "completedWindows", "totalSourceMessages", "processedSourceMessages", "failedWindows"]) assertInteger(value.rebuildProgress[field], field, 0);
  if (!["idle", "preparing", "running", "completed", "failed", "cancelled"].some(
    /** Accepts only the declared durable task lifecycle states. */
    (status2) => value.rebuildProgress !== null && typeof value.rebuildProgress === "object" && "status" in value.rebuildProgress && value.rebuildProgress.status === status2
  )) throw new Error("Invalid memory rebuild status");
  if (value.rebuildTask !== null) {
    assertObject(value.rebuildTask, "rebuild task");
    requireDecimal(value.rebuildTask.id, "rebuild task id", true);
    array2(value.rebuildTask.windows, "rebuild windows");
    assertInteger(value.rebuildTask.nextWindow, "rebuild cursor", 0);
    if (value.rebuildTask.windows.length === 0 || value.rebuildTask.nextWindow > value.rebuildTask.windows.length) throw new Error("Invalid rebuild cursor or source plan");
    let sourceTotal = 0, processedTotal = 0, completedChats = 0, windowIndex = 0;
    const chats = /* @__PURE__ */ new Set(), finishedChats = /* @__PURE__ */ new Set();
    let previousChat = null;
    for (const window of value.rebuildTask.windows) {
      assertObject(window, "rebuild window");
      requireId(window.chatId, "rebuild chat id");
      assertString(window.chatTitle, "rebuild chat title");
      assertInteger(window.sourceMessageCount, "source message count", 1);
      array2(window.messages, "rebuild messages");
      if (window.messages.length < window.sourceMessageCount) throw new Error("Rebuild source count exceeds the stored messages");
      sourceTotal += window.sourceMessageCount;
      if (windowIndex < value.rebuildTask.nextWindow) processedTotal += window.sourceMessageCount;
      const chatId = requireId(window.chatId, "window chat id");
      if (previousChat !== null && previousChat !== chatId) {
        finishedChats.add(previousChat);
        if (finishedChats.has(chatId)) throw new Error("Rebuild plan repeats a completed chat block");
        if (windowIndex <= value.rebuildTask.nextWindow) completedChats += 1;
      }
      previousChat = chatId;
      chats.add(chatId);
      windowIndex += 1;
      for (const message of window.messages) {
        assertMemoryChatMessage(message);
        if (message.sender !== "user" && message.sender !== "ai" && message.sender !== "assistant") throw new Error("Rebuild window has an unsupported source sender");
        assertString(message.content, "source message content");
        if (message.content.trim() === "") throw new Error("Rebuild window contains a blank source message");
      }
    }
    if (value.rebuildTask.nextWindow === value.rebuildTask.windows.length) completedChats += 1;
    if (value.rebuildProgress.totalWindows !== value.rebuildTask.windows.length || value.rebuildProgress.completedWindows !== value.rebuildTask.nextWindow) throw new Error("Rebuild plan and progress disagree");
    if (value.rebuildProgress.totalSourceMessages !== sourceTotal || value.rebuildProgress.processedSourceMessages !== processedTotal || value.rebuildProgress.totalChats !== chats.size || value.rebuildProgress.completedChats !== completedChats) throw new Error("Rebuild counters disagree with the actual source plan");
    if (value.rebuildProgress.status === "completed" && value.rebuildTask.nextWindow !== value.rebuildTask.windows.length) throw new Error("Completed rebuild has unprocessed source windows");
    if (value.rebuildProgress.status === "preparing" && value.rebuildTask.nextWindow !== 0) throw new Error("Preparing rebuild already has processed source windows");
  } else if (value.rebuildProgress.status !== "idle") throw new Error("Rebuild progress is missing its source plan");
  array2(value.embeddings, "embeddings");
  const embeddingKeys = /* @__PURE__ */ new Set();
  for (const item of value.embeddings) {
    assertEmbedding(item);
    const key = JSON.stringify([item.endpoint, item.model, item.text]);
    if (embeddingKeys.has(key)) throw new Error("Duplicate stored embedding input");
    embeddingKeys.add(key);
  }
}
function decodeCharacterState(content) {
  assertString(content, "state file content");
  const value = JSON.parse(content);
  assertCharacterState(value);
  return value;
}
function copyState(state) {
  return decodeCharacterState(JSON.stringify(state));
}

// src/backup.ts
function decodeCharacterBackup(content) {
  assertString(content, "character backup");
  const document = JSON.parse(content);
  assertObject(document, "character backup");
  if (document.version !== 2) throw new Error("Unsupported plugin character backup version");
  assertConversationGroups(document.conversationGroups);
  if (!Array.isArray(document.characterCards) || !Array.isArray(document.promptTags)) throw new Error("Character backup requires complete characterCards and promptTags arrays");
  for (const card of document.characterCards) assertCard(card);
  for (const tag of document.promptTags) assertTag(tag);
  return { version: 2, characterCards: document.characterCards, promptTags: document.promptTags, conversationGroups: document.conversationGroups };
}
function decodeGroupBackup(content) {
  assertString(content, "group backup");
  const document = JSON.parse(content);
  assertObject(document, "group backup");
  if (!Array.isArray(document.characterGroups)) throw new Error("Group backup requires complete characterGroups records");
  for (const group of document.characterGroups) assertGroup(group);
  return { characterGroups: document.characterGroups };
}
function exportCharacterBackup(repository) {
  return __async(this, null, function* () {
    const characterCards = yield repository.listCharacters(), promptTags = yield repository.listTags(), conversationGroups = yield repository.readConversationGroupsForBackup();
    assertConversationGroups(conversationGroups);
    return JSON.stringify({ version: 2, characterCards, promptTags, conversationGroups }, null, 2);
  });
}
function exportGroupBackup(repository) {
  return __async(this, null, function* () {
    return JSON.stringify({ characterGroups: yield repository.listGroups() }, null, 2);
  });
}
function decodeMemoryBackup(content) {
  assertString(content, "memory backup");
  const document = JSON.parse(content);
  assertObject(document, "memory backup");
  if (document.version !== "2.0") throw new Error("Unsupported memory backup version; this plugin uses full-record version 2.0");
  assertInteger(document.exportDate, "exportDate", 0);
  assertMemorySpace(document.space);
  assertString(document.userMarkdown, "USER.md backup content");
  return { version: "2.0", exportDate: document.exportDate, space: document.space, userMarkdown: document.userMarkdown };
}
function exportMemoryBackup(ownerKey, repository) {
  return __async(this, null, function* () {
    const backup = { version: "2.0", exportDate: Date.now(), space: yield repository.readMemorySpace(ownerKey), userMarkdown: (yield repository.readUser(ownerKey)).content };
    return JSON.stringify(backup, null, 2);
  });
}

// src/serialization.ts
function decodeTavernData(value) {
  assertObject(value, "\u9152\u9986\u89D2\u8272\u6570\u636E");
  const data = {
    name: "",
    description: "",
    personality: "",
    first_mes: "",
    avatar: "",
    mes_example: "",
    scenario: "",
    creator_notes: "",
    system_prompt: "",
    post_history_instructions: "",
    alternate_greetings: [],
    tags: [],
    creator: "",
    character_version: "",
    extensions: null,
    character_book: null,
    ...value
  };
  for (const field of ["name", "description", "personality", "first_mes", "avatar", "mes_example", "scenario", "creator_notes", "system_prompt", "post_history_instructions", "creator", "character_version"]) assertString(data[field], `data.${field}`);
  assertStrings(data.alternate_greetings, "\u5907\u7528\u5F00\u573A\u767D");
  assertStrings(data.tags, "\u9152\u9986\u6807\u7B7E");
  if (data.extensions !== null) assertObject(data.extensions, "\u9152\u9986\u6269\u5C55");
  if (data.character_book !== null) assertObject(data.character_book, "\u9152\u9986\u4E16\u754C\u4E66");
  return data;
}
function labeledBlock(label, value) {
  const content = value.trim();
  return content === "" ? "" : `${label}
${content}`;
}
function joinBlocks(values) {
  const parts = [];
  for (const value of values) if (value.trim() !== "") parts.push(value.trim());
  return parts.join("\n\n");
}
function stageImportedTag(values, tags, changes2) {
  assertTagValues(values);
  const prompt = values.promptContent.trim();
  for (const tag of tags) if (tag.promptContent.trim() === prompt) return tag.id;
  for (const draft of changes2.created) if (draft.values.promptContent.trim() === prompt) return draft.draftId;
  const draftId = `import-tag:${changes2.created.length}`;
  changes2.created.push({ draftId, values: { ...values, name: requireName(values.name, "\u5BFC\u5165\u6807\u7B7E\u540D\u79F0") } });
  return draftId;
}
function canonicalTag(tags, id2) {
  requireId(id2, "\u89D2\u8272\u6807\u7B7E\u6807\u8BC6");
  for (const tag of tags) if (tag.id === id2) return tag;
  throw new Error(`\u5BFC\u5165\u89D2\u8272\u5F15\u7528\u7684\u6807\u7B7E\u4E0D\u5B58\u5728\uFF1A${id2}`);
}
function decodeOperitExtension(value, tags, changes2) {
  assertObject(value, "Operit \u9152\u9986\u6269\u5C55");
  const extension = { schema: "operit_character_card_v1", ...value };
  assertString(extension.schema, "Operit \u6269\u5C55 schema");
  assertObject(extension.character_card, "Operit \u6269\u5C55\u89D2\u8272\u5361");
  const payload = { ...createCharacterDraft(), attachedTags: [], ...extension.character_card };
  assertStrings(payload.attachedTagIds, "\u6269\u5C55\u89D2\u8272\u6807\u7B7E\u6807\u8BC6");
  if (!Array.isArray(payload.attachedTags)) throw new Error("\u6269\u5C55\u89D2\u8272\u6807\u7B7E\u5FC5\u987B\u662F\u6570\u7EC4");
  const attachedIds = [];
  if (payload.attachedTags.length !== 0) {
    for (const value2 of payload.attachedTags) {
      assertObject(value2, "\u6269\u5C55\u6807\u7B7E");
      const tag = { id: "", name: "", description: "", promptContent: "", tagType: "CUSTOM", ...value2 };
      assertTagValues(tag);
      if (tag.promptContent.trim() === "") continue;
      attachedIds.push(stageImportedTag(tag, tags, changes2));
    }
  } else {
    for (const id2 of payload.attachedTagIds) attachedIds.push(canonicalTag(tags, id2).id);
  }
  const card = { ...payload, id: "", isDefault: false, createdAt: 0, updatedAt: 0, attachedTagIds: normalizeNames(attachedIds) };
  assertCard(card);
  return card;
}
function stageWorldBook(data, tags, changes2) {
  if (data.character_book === null) return null;
  const book = { entries: [], ...data.character_book };
  if (!Array.isArray(book.entries)) throw new Error("\u4E16\u754C\u4E66 entries \u5FC5\u987B\u662F\u6570\u7EC4");
  const parts = [];
  for (const value of book.entries) {
    assertObject(value, "\u4E16\u754C\u4E66\u6761\u76EE");
    const entry = { name: "", content: "", ...value };
    assertString(entry.name, "\u4E16\u754C\u4E66\u6761\u76EE\u540D\u79F0");
    assertString(entry.content, "\u4E16\u754C\u4E66\u6761\u76EE\u5185\u5BB9");
    if (entry.content.trim() !== "") parts.push(`[${entry.name}]
${entry.content}`);
  }
  const content = parts.join("\n\n").trim();
  if (content === "") return null;
  return stageImportedTag({ name: `\u4E16\u754C\u4E66: ${data.name}`, description: `\u4E3A\u89D2\u8272'${data.name}'\u81EA\u52A8\u751F\u6210\u7684\u4E16\u754C\u4E66\u3002`, promptContent: content, tagType: "FUNCTION" }, tags, changes2);
}
function convertStandardTavern(data, spec, version) {
  const card = createCharacterDraft();
  card.name = data.name;
  if (data.tags.length !== 0) card.description = `\u6807\u7B7E\uFF1A${data.tags.slice(0, 5).join(", ")}${data.tags.length > 5 ? `\u7B49${data.tags.length}` : ""}`;
  card.characterSetting = joinBlocks([
    labeledBlock("\u89D2\u8272\u63CF\u8FF0\uFF1A", data.description),
    labeledBlock("\u6027\u683C\u7279\u5F81\uFF1A", data.personality),
    labeledBlock("\u573A\u666F\u8BBE\u5B9A\uFF1A", data.scenario)
  ]);
  let greetings = "";
  if (data.alternate_greetings.length !== 0) {
    greetings = "\u5907\u7528\u95EE\u5019\u8BED\uFF1A\n";
    for (let index = 0; index < data.alternate_greetings.length; index += 1) greetings += `${index + 1}. ${data.alternate_greetings[index]}
`;
  }
  card.otherContentChat = joinBlocks([
    labeledBlock("\u5BF9\u8BDD\u793A\u4F8B\uFF1A", data.mes_example),
    labeledBlock("\u7CFB\u7EDF\u63D0\u793A\u8BCD\uFF1A", data.system_prompt),
    labeledBlock("\u5386\u53F2\u6307\u4EE4\uFF1A", data.post_history_instructions),
    greetings
  ]);
  if (data.extensions !== null) {
    const extensions = { depth_prompt: null, ...data.extensions };
    if (extensions.depth_prompt !== null) {
      assertObject(extensions.depth_prompt, "\u6DF1\u5EA6\u63D0\u793A\u8BCD");
      const depth = { prompt: "", ...extensions.depth_prompt };
      assertString(depth.prompt, "\u6DF1\u5EA6\u63D0\u793A\u8BCD\u5185\u5BB9");
      card.advancedCustomPrompt = labeledBlock("\u6DF1\u5EA6\u63D0\u793A\u8BCD\uFF1A", depth.prompt);
    }
  }
  card.openingStatement = data.first_mes;
  if (data.avatar.trim() !== "") card.avatarUri = data.avatar;
  let marks = "\u6765\u6E90\uFF1A\u9152\u9986\u89D2\u8272\u5361\n";
  if (data.creator.trim() !== "") marks += `\u4F5C\u8005\uFF1A${data.creator}
`;
  if (data.creator_notes.trim() !== "") marks += `\u4F5C\u8005\u5907\u6CE8\uFF1A

${data.creator_notes}

`;
  if (data.character_version.trim() !== "") marks += `\u7248\u672C\uFF1A${data.character_version}
`;
  if (data.tags.length !== 0) marks += `\u539F\u59CB\u6807\u7B7E\uFF1A${data.tags.join(", ")}
`;
  if (spec.trim() !== "") marks += `\u683C\u5F0F\uFF1A${spec}${version.trim() !== "" ? ` v${version}` : ""}
`;
  card.marks = marks.trim();
  return card;
}
function decodeCharacterImport(content, format2, tags) {
  assertString(content, "\u89D2\u8272\u5361\u5BFC\u5165\u5185\u5BB9");
  const document = JSON.parse(content);
  const tagChanges = { created: [], updated: [], deleted: [] };
  if (format2 === "operit") {
    assertCard(document);
    for (const id2 of document.attachedTagIds) canonicalTag(tags, id2);
    return { card: { ...document, id: "", isDefault: false, createdAt: 0, updatedAt: 0 }, tagChanges };
  }
  if (format2 !== "tavern") throw new Error("\u89D2\u8272\u5361\u5BFC\u5165\u683C\u5F0F\u65E0\u6548");
  assertObject(document, "\u9152\u9986\u89D2\u8272\u5361");
  const envelope = { spec: "", spec_version: "", ...document };
  assertString(envelope.spec, "\u9152\u9986 spec");
  assertString(envelope.spec_version, "\u9152\u9986 spec_version");
  const data = decodeTavernData(envelope.data);
  requireName(data.name, "\u89D2\u8272\u5361\u540D\u79F0");
  let card;
  if (data.extensions !== null && Object.prototype.hasOwnProperty.call(data.extensions, "operit") && data.extensions.operit !== null) card = decodeOperitExtension(data.extensions.operit, tags, tagChanges);
  else card = convertStandardTavern(data, envelope.spec, envelope.spec_version);
  const worldBookTag = stageWorldBook(data, tags, tagChanges);
  if (worldBookTag !== null) card.attachedTagIds = normalizeNames([...card.attachedTagIds, worldBookTag]);
  assertCard(card);
  return { card, tagChanges };
}
function encodeCharacterExport(card, format2, tags) {
  assertCard(card);
  if (format2 === "operit") return JSON.stringify(card, null, 2);
  if (format2 !== "tavern") throw new Error("\u89D2\u8272\u5361\u5BFC\u51FA\u683C\u5F0F\u65E0\u6548");
  const attachedTags = [];
  const tagNames = [];
  for (const id2 of card.attachedTagIds) {
    const tag = canonicalTag(tags, id2);
    attachedTags.push({ id: tag.id, name: tag.name, description: tag.description, promptContent: tag.promptContent, tagType: tag.tagType });
    tagNames.push(tag.name);
  }
  const avatar = card.avatarUri === null ? "" : card.avatarUri;
  const document = {
    spec: "chara_card_v2",
    spec_version: "2.0",
    data: {
      name: card.name,
      description: card.description,
      personality: "",
      first_mes: card.openingStatement,
      avatar,
      mes_example: card.otherContentChat,
      scenario: "",
      creator_notes: card.marks,
      system_prompt: card.characterSetting,
      post_history_instructions: card.advancedCustomPrompt,
      alternate_greetings: [],
      tags: tagNames,
      creator: "",
      character_version: "",
      character_book: null,
      extensions: {
        chub: null,
        depth_prompt: null,
        operit: {
          schema: "operit_character_card_v1",
          character_card: {
            name: card.name,
            description: card.description,
            characterSetting: card.characterSetting,
            openingStatement: card.openingStatement,
            otherContent: card.otherContentChat,
            otherContentChat: card.otherContentChat,
            otherContentVoice: card.otherContentVoice,
            avatarUri: card.avatarUri,
            attachedTagIds: [...card.attachedTagIds],
            attachedTags,
            advancedCustomPrompt: card.advancedCustomPrompt,
            marks: card.marks,
            chatModelBindingMode: card.chatModelBindingMode,
            chatModelId: card.chatModelId,
            ttsConfigId: card.ttsConfigId,
            themeConfigId: card.themeConfigId,
            memoryBindingMode: card.memoryBindingMode,
            sharedMemoryId: card.sharedMemoryId,
            sharedMemoryMounts: card.sharedMemoryMounts,
            toolAccessConfig: card.toolAccessConfig
          }
        }
      }
    }
  };
  return JSON.stringify(document, null, 2);
}
function decodeGroupImport(content) {
  assertString(content, "\u7FA4\u7EC4\u5BFC\u5165\u5185\u5BB9");
  const document = JSON.parse(content);
  assertGroup(document);
  return { ...document, id: "", createdAt: 0, updatedAt: 0 };
}

// src/operations.ts
function findMemory(items, title) {
  const name = requireName(title, "\u8BB0\u5FC6\u6807\u9898");
  let found;
  for (const memory of items) {
    if (memory.title !== name) continue;
    if (found !== void 0) throw new Error(`\u8BB0\u5FC6\u6807\u9898\u4E0D\u552F\u4E00\uFF1A${name}`);
    found = memory;
  }
  if (found === void 0) throw new Error(`\u8BB0\u5FC6\u4E0D\u5B58\u5728\uFF1A${name}`);
  return found;
}
function deleteCharacter(id2, host) {
  return __async(this, null, function* () {
    requireId(id2, "\u89D2\u8272\u5361\u6807\u8BC6");
    const card = yield host.getCharacter(id2);
    assertCard(card);
    if (card.isDefault) throw new Error("\u9ED8\u8BA4\u89D2\u8272\u5361\u4E0D\u80FD\u5220\u9664");
    yield host.deleteCharacter(id2);
    return { id: id2, deleted: true };
  });
}
function updateLink(input, host) {
  return __async(this, null, function* () {
    yield requireOwner(input.ownerKey, host);
    const original = yield host.readMemoryLink(input.ownerKey, input.linkId);
    const changes2 = {};
    if (Object.prototype.hasOwnProperty.call(input.changes, "linkType")) changes2.type_ = requireName(input.changes.linkType, "\u5173\u7CFB\u7C7B\u578B");
    if (Object.prototype.hasOwnProperty.call(input.changes, "weight")) {
      assertNumber(input.changes.weight, "\u5173\u7CFB\u5F3A\u5EA6", 0, 1);
      changes2.weight = input.changes.weight;
    }
    if (Object.prototype.hasOwnProperty.call(input.changes, "description")) changes2.description = input.changes.description;
    const value = { ...original, ...changes2 };
    const link2 = yield host.updateLink(input.ownerKey, original.id, { type_: value.type_, weight: value.weight, description: value.description });
    return { ownerKey: input.ownerKey, link: link2 };
  });
}
function activateForChat(input, host) {
  return __async(this, null, function* () {
    if (input.characterGroupId !== null && input.characterCardName !== null) throw new Error("Chat activation cannot select both a card and a group");
    if (input.characterGroupId !== null && input.characterGroupId.trim() !== "") yield activate("group", input.characterGroupId.trim(), host);
    else if (input.characterCardName !== null && input.characterCardName.trim() !== "") {
      const name = input.characterCardName.trim(), cards = yield host.listCharacters();
      let target;
      for (const card of cards) if (card.name === name) {
        if (target !== void 0) throw new Error("\u804A\u5929\u89D2\u8272\u540D\u79F0\u4E0D\u552F\u4E00");
        target = card;
      }
      if (target === void 0) throw new Error(`\u804A\u5929\u89D2\u8272\u4E0D\u5B58\u5728\uFF1A${name}`);
      yield activate("card", target.id, host);
    } else throw new Error("Chat activation requires an explicit character or group selection");
    return { characterCardName: input.characterCardName, characterGroupId: input.characterGroupId, updated: true };
  });
}
var handlers = {
  /** Reads exact opaque-scope manual metadata from the same repository as the editor. */
  "conversation-group.list": (input, repository) => repository.listConversationGroups(input.ownerSelection),
  /** Creates a real empty manual group independently from role composition. */
  "conversation-group.create": (input, repository) => repository.createConversationGroup(input),
  /** Writes only explicit name or pin metadata without moving a chat. */
  "conversation-group.update": (input, repository) => repository.updateConversationGroup(input.id, input.changes),
  /** Releases members without invoking any host chat deletion. */
  "conversation-group.delete": (input, repository) => repository.deleteConversationGroup(input.id),
  /** Verifies genuine host chat existence before committing one explicit membership transfer. */
  "conversation-group.moveChat": (input, repository) => __async(null, null, function* () {
    yield requireChat(input.chatId);
    return repository.moveConversationGroupChat(input.chatId, input.groupId, input.ownerSelection);
  }),
  /** Commits a complete scoped order without mutating another section. */
  "conversation-group.reorder": (input, repository) => repository.reorderConversationGroups(input.ownerSelection, input.ids),
  /** Keeps full-filter searches and provider embedding persistence on the same serialized service. */
  "memory.searchWithOptions": (input, repository) => __async(null, null, function* () {
    return { ownerKey: input.ownerKey, items: yield searchMemories(input, repository) };
  }),
  /** Lists actual owner-bound generic chat summaries. */
  "memory.chat.list": (input, repository) => listMemoryChats(input.ownerKey, repository),
  /** Extracts real chat messages through the configured functional MEMORY model. */
  "memory.chat.update": (input, repository) => updateChatMemory(input.ownerKey, input.chatId, repository),
  /** Performs genuine ten-record functional categorization in the same operation. */
  "memory.categorize": (input, repository) => autoCategorizeMemory(input.ownerKey, repository),
  /** Stores a complete requested rebuild plan for real interval execution. */
  "memory.rebuild.start": (input, repository) => startMemoryRebuild(input.ownerKey, input.rebuild, repository),
  /** Reads the real durable job counters. */
  "memory.rebuild.progress": (input, repository) => repository.readMemoryRebuildProgress(input.ownerKey),
  /** Records an explicit cancellation before another source window may be processed. */
  "memory.rebuild.cancel": (input, repository) => __async(null, null, function* () {
    yield repository.cancelMemoryRebuild(input.ownerKey);
    return repository.readMemoryRebuildProgress(input.ownerKey);
  }),
  /** Rebuilds actual configured provider embeddings using generic HTTP. */
  "memory.embeddings.rebuild": (input, repository) => rebuildMemoryEmbeddings(input.ownerKey, repository),
  /** Enqueues only actual persisted message sources and genuine bound owners. */
  "memory.candidate.enqueue": (input, repository) => enqueueCandidate(input, repository),
  /** Resolves binding, catalogs, prompt and USER.md from one serialized repository operation. */
  "chat.configuration.resolve": (input, repository) => resolveChatConfiguration(input, {
    /** Reads the same in-flight directory without reacquiring the publisher. */
    snapshot: () => snapshot(repository),
    /** Executes nested resolution reads on this exact snapshot without another service. */
    dispatchDomain: (operation, payload) => executeDomain(operation, payload, repository)
  }),
  /** Requires the chat's persisted selection rather than global active state. */
  "chat.configuration.binding.read": (input, repository) => repository.readChatBinding(input.chatId),
  /** Stages a genuine selector submission in the shared publication. */
  "chat.configuration.binding.write": (input, repository) => __async(null, null, function* () {
    yield requireChat(input.chatId);
    return repository.writeChatBinding(input);
  }),
  /** Deletes the requested existing binding through the same file operation. */
  "chat.configuration.binding.delete": (input, repository) => repository.deleteChatBinding(input.chatId),
  /** Reads the complete real editor directories. */
  snapshot: (_input, host) => snapshot(host),
  /** Reads complete canonical character cards. */
  "character.list": (_input, host) => __async(null, null, function* () {
    const cards = yield host.listCharacters();
    for (const card of cards) assertCard(card);
    return cards;
  }),
  /** Requires the selected canonical character. */
  "character.get": (input, host) => __async(null, null, function* () {
    const card = yield host.getCharacter(input.id);
    assertCard(card);
    return card;
  }),
  /** Initializes an explicit new form and persists all provided character fields. */
  "character.create": (input, host) => saveCharacter({ ...createCharacterDraft(), ...input.values }, true, { created: [], updated: [], deleted: [] }, host),
  /** Merges an explicit edit into the complete canonical record before one manager write. */
  "character.update": (input, host) => __async(null, null, function* () {
    const original = yield host.getCharacter(input.id);
    return saveCharacter({ ...original, ...input.changes, id: original.id }, false, { created: [], updated: [], deleted: [] }, host);
  }),
  /** Deletes an existing non-default character using the canonical lifecycle. */
  "character.delete": (input, host) => deleteCharacter(input.id, host),
  /** Activates the selected canonical character. */
  "character.setActive": (input, host) => __async(null, null, function* () {
    yield activate("card", input.id, host);
    return { type: "character_card", id: input.id, active: true };
  }),
  /** Composes the actual canonical prompt using the explicitly selected function and tags. */
  "character.combine": (input, host) => __async(null, null, function* () {
    const options = { promptFunctionType: "CHAT", additionalTagIds: [], ...input };
    const prompt = yield combinePrompts(input.id, options.additionalTagIds, options.promptFunctionType, host);
    return { id: input.id, promptFunctionType: options.promptFunctionType, additionalTagIds: options.additionalTagIds, prompt };
  }),
  /** Resets the canonical default character through its established manager operation. */
  "character.resetDefault": (_input, host) => __async(null, null, function* () {
    const card = yield host.resetDefaultCharacter();
    assertCard(card);
    return { defaultCharacterReset: true };
  }),
  /** Serializes the plugin-owned character interchange representation. */
  "character.export": (input, host) => __async(null, null, function* () {
    const card = yield host.getCharacter(input.id), tags = yield host.listTags();
    return { id: input.id, format: input.format, content: encodeCharacterExport(card, input.format, tags) };
  }),
  /** Parses and stages interchange imports in the plugin before full canonical persistence. */
  "character.import": (input, host) => __async(null, null, function* () {
    const plan = decodeCharacterImport(input.content, input.format, yield host.listTags());
    return saveCharacter(plan.card, true, plan.tagChanges, host);
  }),
  /** Exports the actual canonical records using the original manager backup format. */
  "character.exportBackup": (_input, host) => __async(null, null, function* () {
    return { content: yield exportCharacterBackup(host) };
  }),
  /** Validates the backup in the plugin before invoking canonical identity-preserving restore. */
  "character.importBackup": (input, host) => __async(null, null, function* () {
    const document = decodeCharacterBackup(input.content);
    const result2 = yield host.restoreCharacters(document.characterCards, document.promptTags);
    yield host.restoreConversationGroups(document.conversationGroups);
    return result2;
  }),
  /** Reads complete canonical groups. */
  "group.list": (_input, host) => __async(null, null, function* () {
    const groups = yield host.listGroups();
    for (const group of groups) assertGroup(group);
    return groups;
  }),
  /** Requires the selected canonical group. */
  "group.get": (input, host) => __async(null, null, function* () {
    const group = yield host.getGroup(input.id);
    assertGroup(group);
    return group;
  }),
  /** Persists every field of an explicit new group. */
  "group.create": (input, host) => saveGroup({ ...createGroupDraft(), ...input.values }, true, host),
  /** Merges an explicit edit into the full canonical group before saving. */
  "group.update": (input, host) => __async(null, null, function* () {
    const original = yield host.getGroup(input.id);
    return saveGroup({ ...original, ...input.changes, id: original.id }, false, host);
  }),
  /** Deletes the group through the plugin repository lifecycle. */
  "group.delete": (input, host) => __async(null, null, function* () {
    yield host.getGroup(input.id);
    yield host.deleteGroup(input.id);
    return { id: input.id, deleted: true };
  }),
  /** Activates the selected real group. */
  "group.setActive": (input, host) => __async(null, null, function* () {
    yield activate("group", input.id, host);
    return { type: "character_group", id: input.id, active: true };
  }),
  /** Copies the full canonical group's editable state into an intentional new group. */
  "group.duplicate": (input, host) => __async(null, null, function* () {
    const group = yield host.getGroup(input.id);
    const options = { newName: group.name, ...input };
    return saveGroup({ ...group, id: "", name: requireName(options.newName, "\u65B0\u7FA4\u7EC4\u540D\u79F0") }, true, host);
  }),
  /** Exports the complete canonical group serde record. */
  "group.export": (input, host) => __async(null, null, function* () {
    const group = yield host.getGroup(input.id);
    assertGroup(group);
    return { id: input.id, content: JSON.stringify(group, null, 2) };
  }),
  /** Parses and validates a full imported group before canonical creation. */
  "group.import": (input, host) => saveGroup(decodeGroupImport(input.content), true, host),
  /** Exports the actual canonical group backup shape. */
  "group.exportBackup": (_input, host) => __async(null, null, function* () {
    return { content: yield exportGroupBackup(host) };
  }),
  /** Validates the group backup before canonical identity-preserving restoration. */
  "group.importBackup": (input, host) => host.restoreGroups(decodeGroupBackup(input.content).characterGroups),
  /** Reads the canonical externally tagged active prompt. */
  "activePrompt.get": (_input, host) => host.readActive(),
  /** Activates the selected canonical character. */
  "activePrompt.setCard": (input, host) => __async(null, null, function* () {
    yield activate("card", input.id, host);
    return { type: "character_card", id: input.id, active: true };
  }),
  /** Activates the selected canonical group. */
  "activePrompt.setGroup": (input, host) => __async(null, null, function* () {
    yield activate("group", input.id, host);
    return { type: "character_group", id: input.id, active: true };
  }),
  /** Applies the plugin's explicit chat-binding selection logic. */
  "activePrompt.activateForChat": (input, host) => activateForChat(input, host),
  /** Resolves the real next-send actor under the canonical active card or group selection. */
  "activePrompt.resolvedCard": (_input, host) => __async(null, null, function* () {
    const active = yield host.readActive();
    if (!("CharacterCard" in active)) throw new Error("An active group requires an explicit execution participant");
    return { id: active.CharacterCard.id };
  }),
  /** Reads complete canonical tags. */
  "tag.list": (_input, host) => __async(null, null, function* () {
    const tags = yield host.listTags();
    for (const tag of tags) assertTag(tag);
    return tags;
  }),
  /** Requires an existing complete canonical tag. */
  "tag.get": (input, host) => __async(null, null, function* () {
    const tag = yield host.getTag(input.id);
    assertTag(tag);
    return tag;
  }),
  /** Initializes a new tag form and writes every provided editable field. */
  "tag.create": (input, host) => saveTag({ id: "", ...createTagDraft(), ...input.values }, true, host),
  /** Writes every editable field by merging the patch into the complete canonical tag. */
  "tag.update": (input, host) => __async(null, null, function* () {
    const original = yield host.getTag(input.id);
    return saveTag({ ...original, ...input.changes, id: original.id }, false, host);
  }),
  /** Deletes an existing canonical prompt tag. */
  "tag.delete": (input, host) => __async(null, null, function* () {
    yield host.getTag(input.id);
    yield host.deleteTag(input.id);
    return { id: input.id, deleted: true };
  }),
  /** Reads complete canonical shared stores. */
  "memory.shared.list": (_input, host) => host.listStores(),
  /** Creates the canonical shared store. */
  "memory.shared.create": (input, host) => saveStore({ id: "", name: input.name }, true, host),
  /** Renames the selected canonical shared store. */
  "memory.shared.rename": (input, host) => saveStore({ id: input.id, name: input.name }, false, host),
  /** Removes the shared store and counts the actual canonical character bindings affected. */
  "memory.shared.delete": (input, host) => __async(null, null, function* () {
    const cards = yield host.listCharacters();
    let cleanedCharacters = 0;
    for (const card of cards) {
      let affected = card.sharedMemoryId === input.id;
      for (const mount2 of card.sharedMemoryMounts) if (mount2.sharedMemoryId === input.id) affected = true;
      if (affected) cleanedCharacters += 1;
    }
    yield host.deleteStore(input.id);
    return { sharedId: input.id, deleted: true, cleanedCharacters };
  }),
  /** Saves an explicit shared-store mount through the full canonical character writer. */
  "memory.mount": (input, host) => __async(null, null, function* () {
    yield host.getStore(input.sharedId);
    const original = yield host.getCharacter(input.characterId);
    const mount2 = { sharedMemoryId: input.sharedId, readable: input.readable, writable: input.writable };
    const mounts = [];
    for (const value of original.sharedMemoryMounts) if (value.sharedMemoryId !== input.sharedId) mounts.push(value);
    mounts.push(mount2);
    yield saveCharacter({ ...original, sharedMemoryMounts: mounts }, false, { created: [], updated: [], deleted: [] }, host);
    return { characterId: input.characterId, sharedId: input.sharedId, mount: mount2, mounted: true };
  }),
  /** Removes only the explicitly selected mount through a full canonical character write. */
  "memory.unmount": (input, host) => __async(null, null, function* () {
    const original = yield host.getCharacter(input.characterId), mounts = [];
    for (const mount2 of original.sharedMemoryMounts) if (mount2.sharedMemoryId !== input.sharedId) mounts.push(mount2);
    yield saveCharacter({ ...original, sharedMemoryMounts: mounts }, false, { created: [], updated: [], deleted: [] }, host);
    return { characterId: input.characterId, sharedId: input.sharedId, unmounted: true };
  }),
  /** Reads the selected real USER.md contents. */
  "memory.user.read": (input, host) => __async(null, null, function* () {
    yield requireOwner(input.ownerKey, host);
    return host.readUser(input.ownerKey);
  }),
  /** Writes owner-scoped USER.md without touching preference files. */
  "memory.user.write": (input, host) => __async(null, null, function* () {
    yield requireOwner(input.ownerKey, host);
    yield host.writeUser(input.ownerKey, input.content);
    return { ownerKey: input.ownerKey, contentLength: input.content.length, updated: true };
  }),
  /** Reads the real plugin-owned USER.md path accepted by Files. */
  "memory.user.path": (input, host) => __async(null, null, function* () {
    yield requireOwner(input.ownerKey, host);
    return { ownerKey: input.ownerKey, path: yield host.readUserPath(input.ownerKey) };
  }),
  /** Resolves the primary canonical memory binding to its real owner enum. */
  "memory.resolveOwner": (input, host) => __async(null, null, function* () {
    return parseOwner(yield resolveMemoryOwner(input.characterId, host));
  }),
  /** Reads complete canonical owner settings. */
  "memory.settings.read": (input, host) => __async(null, null, function* () {
    yield requireOwner(input.ownerKey, host);
    return host.readMemorySettings(input.ownerKey);
  }),
  /** Writes complete canonical owner settings. */
  "memory.settings.write": (input, host) => writeMemorySettings(input.ownerKey, input.settings, host),
  /** Reads the complete persisted search configuration. */
  "memory.searchConfig.read": (input, host) => __async(null, null, function* () {
    yield requireOwner(input.ownerKey, host);
    return host.readMemorySearchConfig(input.ownerKey);
  }),
  /** Writes every persisted search configuration field. */
  "memory.searchConfig.write": (input, host) => writeMemorySearchConfig(input.ownerKey, input.config, host),
  /** Reads the actual repository graph with canonical UUID nodes. */
  "memory.graph": (input, host) => __async(null, null, function* () {
    return { ownerKey: input.ownerKey, graph: (yield graph(input.ownerKey, host)).graph };
  }),
  /** Reads complete repository memories in the selected owner scope. */
  "memory.list": (input, host) => __async(null, null, function* () {
    return { ownerKey: input.ownerKey, items: yield listMemories(input.ownerKey, host) };
  }),
  /** Searches complete repository records with the original explicit editor search filters. */
  "memory.search": (input, host) => __async(null, null, function* () {
    return { ownerKey: input.ownerKey, query: input.query, items: yield searchMemories({ ...input, folderPath: null, relevanceThreshold: 0, createdAtStartMs: null, createdAtEndMs: null }, host) };
  }),
  /** Resolves an exact title to one complete canonical memory. */
  "memory.get": (input, host) => __async(null, null, function* () {
    return { ownerKey: input.ownerKey, item: findMemory(yield listMemories(input.ownerKey, host), input.title) };
  }),
  /** Initializes an intentional full new record and writes all memory metadata in the plugin snapshot. */
  "memory.create": (input, host) => __async(null, null, function* () {
    yield requireOwner(input.ownerKey, host);
    const values = {
      id: "",
      uuid: "",
      contentType: "text",
      source: "",
      credibility: 0.5,
      importance: 0.5,
      documentPath: null,
      isDocumentNode: false,
      chunkIndexFilePath: null,
      folderPath: null,
      createdAt: 0,
      updatedAt: 0,
      lastAccessedAt: 0,
      tags: [],
      properties: [],
      ...input.values
    };
    assertStrings(values.tags, "\u8BB0\u5FC6\u6807\u7B7E");
    const tags = [];
    for (const name of normalizeNames(values.tags)) tags.push({ id: String(tags.length + 1), name });
    const memory = { ...values, id: "", uuid: "", title: requireName(values.title, "\u8BB0\u5FC6\u6807\u9898"), tags };
    const item = yield host.saveMemory(input.ownerKey, memory);
    assertMemory(item);
    return { ownerKey: input.ownerKey, item, created: true };
  }),
  /** Merges every supplied memory field into its full canonical record while preserving untouched metadata. */
  "memory.update": (input, host) => __async(null, null, function* () {
    const original = findMemory(yield listMemories(input.ownerKey, host), input.originalTitle);
    if (input.expectedId !== void 0 && input.expectedId !== original.id) throw new Error("Memory identity changed since editing began");
    const changes2 = { ...input.changes };
    const tags = [];
    if (Object.prototype.hasOwnProperty.call(changes2, "tags")) {
      assertStrings(changes2.tags, "\u8BB0\u5FC6\u6807\u7B7E");
      for (const name of normalizeNames(changes2.tags)) tags.push({ id: String(tags.length + 1), name });
    } else for (const tag of original.tags) tags.push(tag);
    const memory = { ...original, ...changes2, id: original.id, tags };
    assertMemory(memory);
    const item = yield host.saveMemory(input.ownerKey, memory);
    assertMemory(item);
    return { ownerKey: input.ownerKey, item, updated: true };
  }),
  /** Deletes the exact owner-scoped canonical memory id. */
  "memory.delete": (input, host) => __async(null, null, function* () {
    yield requireOwner(input.ownerKey, host);
    yield host.deleteMemory(input.ownerKey, input.id);
    return { ownerKey: input.ownerKey, id: input.id, deleted: true };
  }),
  /** Moves the explicitly selected ids using the canonical repository's bulk operation. */
  "memory.move": (input, host) => __async(null, null, function* () {
    const items = yield listMemories(input.ownerKey, host), existing = /* @__PURE__ */ new Set();
    for (const item of items) existing.add(item.id);
    const selected = /* @__PURE__ */ new Set();
    for (const id2 of input.ids) {
      if (!existing.has(id2)) throw new Error(`\u8BB0\u5FC6\u4E0D\u5B58\u5728\uFF1A${id2}`);
      selected.add(id2);
    }
    yield host.moveMemories(input.ownerKey, [...selected], input.folderPath);
    return { ownerKey: input.ownerKey, ids: [...selected], folder: input.folderPath, moved: selected.size };
  }),
  /** Resolves exact source and target records before creating a canonical relationship. */
  "memory.link.create": (input, host) => __async(null, null, function* () {
    const items = yield listMemories(input.ownerKey, host), source = findMemory(items, input.sourceTitle), target = findMemory(items, input.targetTitle);
    if (source.id === target.id) throw new Error("\u4E0D\u80FD\u521B\u5EFA\u8BB0\u5FC6\u81EA\u8EAB\u5173\u7CFB");
    const link2 = yield host.createLink(input.ownerKey, { sourceMemoryId: source.id, targetMemoryId: target.id, type_: requireName(input.linkType, "\u5173\u7CFB\u7C7B\u578B"), weight: input.weight, description: input.description });
    return { ownerKey: input.ownerKey, link: link2 };
  }),
  /** Applies a full canonical relationship edit to the selected id. */
  "memory.link.update": (input, host) => updateLink(input, host),
  /** Deletes the exact owner-scoped canonical relationship id. */
  "memory.link.delete": (input, host) => __async(null, null, function* () {
    yield requireOwner(input.ownerKey, host);
    yield host.deleteLink(input.ownerKey, input.linkId);
    return { ownerKey: input.ownerKey, linkId: input.linkId, deleted: true };
  }),
  /** Serializes the repository's portable backup format in the plugin. */
  "memory.export": (input, host) => __async(null, null, function* () {
    yield requireOwner(input.ownerKey, host);
    return { ownerKey: input.ownerKey, content: yield exportMemoryBackup(input.ownerKey, host) };
  }),
  /** Validates the complete portable backup before notification-preserving canonical restoration. */
  "memory.import": (input, host) => __async(null, null, function* () {
    yield requireOwner(input.ownerKey, host);
    const document = decodeMemoryBackup(input.content);
    const result2 = yield host.importMemorySpace(input.ownerKey, document.space, input.strategy);
    yield host.writeUser(input.ownerKey, document.userMarkdown);
    return { ownerKey: input.ownerKey, result: result2 };
  })
};
function executeDomain(operation, input, host) {
  return handlers[operation](parseDomainPayload(operation, input), host);
}

// src/selection-application.ts
function copyReferences(references) {
  parseChatSelection(references.selection);
  if (references.themeConfigId !== null) requireId(references.themeConfigId, "\u4E3B\u9898\u914D\u7F6E\u6807\u8BC6");
  if (references.ttsConfigId !== null) requireId(references.ttsConfigId, "TTS \u914D\u7F6E\u6807\u8BC6");
  return Object.freeze({ selection: references.selection, themeConfigId: references.themeConfigId, ttsConfigId: references.ttsConfigId });
}
function readSelectionReferences(repository, selection2) {
  return __async(this, null, function* () {
    const parsed = parseChatSelection(selection2);
    switch (parsed.kind) {
      case "card": {
        const card = yield repository.getCharacter(parsed.id);
        assertCard(card);
        if (card.id !== parsed.id) throw new Error("\u89D2\u8272\u5361\u8BFB\u53D6\u7ED3\u679C\u4E0E\u9009\u62E9\u6807\u8BC6\u4E0D\u4E00\u81F4\uFF1A" + selection2);
        return copyReferences({ selection: selection2, themeConfigId: card.themeConfigId, ttsConfigId: card.ttsConfigId });
      }
      case "group": {
        const group = yield repository.getGroup(parsed.id);
        assertGroup(group);
        if (group.id !== parsed.id) throw new Error("\u7FA4\u7EC4\u8BFB\u53D6\u7ED3\u679C\u4E0E\u9009\u62E9\u6807\u8BC6\u4E0D\u4E00\u81F4\uFF1A" + selection2);
        return copyReferences({ selection: selection2, themeConfigId: group.themeConfigId, ttsConfigId: null });
      }
    }
  });
}
function describeConfirmed(steps) {
  return steps.map(
    /** Distinguishes the selection owner and each independent configuration owner's actual acknowledgement. */
    (step) => {
      switch (step.type) {
        case "selection":
          return "\u9009\u62E9\u5DF2\u63D0\u4EA4\uFF08" + step.selection + "\uFF09";
        case "theme":
          return "\u4E3B\u9898\u5DF2\u786E\u8BA4\u5E94\u7528\uFF08" + step.id + "\uFF09";
        case "tts":
          return "TTS \u5DF2\u786E\u8BA4\u5E94\u7528\uFF08" + step.id + "\uFF09";
      }
    }
  ).join("\uFF1B");
}
var SelectionApplicationFailure = class extends Error {
  /** Exposes confirmed progress and the unverified failed write on the same visible selector error. */
  constructor(completed2, failedStep, cause) {
    const label = failedStep.type === "theme" ? "\u4E3B\u9898" : "TTS";
    super(describeConfirmed(completed2) + "\uFF1B" + label + "\u5E94\u7528\u5931\u8D25\uFF08" + failedStep.id + "\uFF09\uFF0C\u8BE5\u914D\u7F6E\u7684\u5B9E\u9645\u5B58\u50A8\u72B6\u6001\u5F85\u6838\u5BF9\u3002\u672A\u81EA\u52A8\u64A4\u9500\u6216\u91CD\u8BD5\u3002\u539F\u59CB\u9519\u8BEF\uFF1A" + String(cause));
    this.name = "SelectionApplicationFailure";
    this.cause = cause;
    this.completed = Object.freeze(completed2.map(
      /** Copies acknowledgement metadata so a caught error cannot rewrite the recorded application history. */
      (step) => Object.freeze({ ...step })
    ));
    this.failedStep = Object.freeze({ ...failedStep });
  }
};
function applyConfigurations(references, access) {
  return __async(this, null, function* () {
    const completed2 = [{ type: "selection", selection: references.selection }];
    if (references.themeConfigId !== null) {
      const id2 = references.themeConfigId;
      try {
        yield access.applyTheme(id2);
      } catch (failure2) {
        throw new SelectionApplicationFailure(completed2, { type: "theme", id: id2 }, failure2);
      }
      completed2.push({ type: "theme", id: id2 });
    }
    if (references.ttsConfigId !== null) {
      const id2 = references.ttsConfigId;
      try {
        yield access.applyTts(id2);
      } catch (failure2) {
        throw new SelectionApplicationFailure(completed2, { type: "tts", id: id2 }, failure2);
      }
      completed2.push({ type: "tts", id: id2 });
    }
  });
}
function createSelectionApplication(access) {
  let busy = false;
  const waiters = [];
  function acquire() {
    return __async(this, null, function* () {
      if (!busy) {
        busy = true;
        return;
      }
      yield new Promise(
        /** Retains only the continuation for a genuinely distinct waiting operation. */
        (resolve) => waiters.push(resolve)
      );
    });
  }
  function release() {
    const next = waiters.shift();
    if (next === void 0) busy = false;
    else next();
  }
  return {
    /** Performs all explicit-reference validation before committing the selection, then awaits every requested apply. */
    commit(prepareAndCommit) {
      return __async(this, null, function* () {
        yield acquire();
        const validated = /* @__PURE__ */ new WeakSet();
        let validations = 0;
        try {
          const committed = yield prepareAndCommit(
            /** Issues one immutable validation receipt inside the selection owner's real record snapshot. */
            (references) => __async(null, null, function* () {
              validations += 1;
              if (validations !== 1) throw new Error("A selection operation must validate exactly one reference plan");
              const checked = copyReferences(references);
              if (checked.themeConfigId !== null) yield access.validateTheme(checked.themeConfigId);
              if (checked.ttsConfigId !== null) yield access.validateTts(checked.ttsConfigId);
              validated.add(checked);
              return checked;
            })
          );
          if (!validated.has(committed.references)) throw new Error("Selection commit did not retain its exact validated reference plan");
          yield applyConfigurations(committed.references, access);
          return committed.value;
        } finally {
          release();
        }
      });
    }
  };
}
function isSelectionOperation(operation) {
  switch (operation) {
    case "character.setActive":
    case "group.setActive":
    case "activePrompt.setCard":
    case "activePrompt.setGroup":
    case "activePrompt.activateForChat":
    case "chat.configuration.binding.write":
      return true;
    default:
      return false;
  }
}
function isSelectionRequest(request) {
  return request.action === "activate" || request.action === "writeChatBinding";
}
function editorSelection(request) {
  if (request.action === "writeChatBinding") return request.selection;
  switch (request.type) {
    case "card":
      return "card:" + request.id;
    case "group":
      return "group:" + request.id;
    default:
      throw new Error("Editor activation requires an explicit card or group type");
  }
}
function domainSelection(operation, input, repository) {
  return __async(this, null, function* () {
    switch (operation) {
      case "character.setActive":
      case "activePrompt.setCard":
        return "card:" + parseDomainPayload("activePrompt.setCard", input).id;
      case "group.setActive":
      case "activePrompt.setGroup":
        return "group:" + parseDomainPayload("activePrompt.setGroup", input).id;
      case "chat.configuration.binding.write":
        return parseDomainPayload("chat.configuration.binding.write", input).selection;
      case "activePrompt.activateForChat": {
        const parsed = parseDomainPayload("activePrompt.activateForChat", input);
        if (parsed.characterGroupId !== null && parsed.characterCardName !== null) throw new Error("Chat activation cannot select both a card and a group");
        if (parsed.characterGroupId !== null) return "group:" + parsed.characterGroupId.trim();
        if (parsed.characterCardName === null) throw new Error("Chat activation requires an explicit character or group selection");
        const name = parsed.characterCardName.trim(), matches = (yield repository.listCharacters()).filter(
          /** Matches the established exact character-name operation without inventing a replacement selection. */
          (card) => card.name === name
        );
        if (matches.length !== 1) throw new Error("Chat character name must identify exactly one persisted record: " + name);
        return "card:" + matches[0].id;
      }
    }
  });
}
function assertThemeChoices(value) {
  if (!Array.isArray(value)) throw new Error("\u4E3B\u9898\u9009\u62E9\u76EE\u5F55\u5FC5\u987B\u662F\u6570\u7EC4");
  const ids = /* @__PURE__ */ new Set();
  for (const choice of value) {
    assertObject(choice, "\u4E3B\u9898\u9009\u62E9\u9879");
    const id2 = requireId(choice.id, "\u4E3B\u9898\u914D\u7F6E ID");
    assertString(choice.label, "\u4E3B\u9898\u914D\u7F6E\u540D\u79F0");
    if (choice.label.trim() === "") throw new Error("\u4E3B\u9898\u914D\u7F6E\u540D\u79F0\u65E0\u6548");
    if (ids.has(id2)) throw new Error("\u4E3B\u9898\u914D\u7F6E ID \u91CD\u590D\uFF1A" + id2);
    ids.add(id2);
  }
}
function readThemeChoices(access) {
  return __async(this, null, function* () {
    const choices = yield access.readThemeChoices();
    assertThemeChoices(choices);
    return choices.map(
      /** Keeps only private presentation fields, never an independent configuration's stored theme values. */
      (choice) => ({ id: choice.id, label: choice.label })
    );
  });
}

// src/selection-settings.ts
var configured = null;
function connectIndependentConfigurations(access) {
  if (configured !== null) throw new Error("Independent configuration access has already been connected");
  configured = access;
}
function requiredAccess() {
  if (configured === null) throw new Error("Independent Theme/TTS configuration capabilities have not been connected");
  return configured;
}
var independentConfigurations = {
  /** Requires an actual theme directory reader; a missing source cannot become an empty successful chooser. */
  readThemeChoices: () => requiredAccess().readThemeChoices(),
  /** Requires a real theme existence validator before any selection write. */
  validateTheme: (id2) => requiredAccess().validateTheme(id2),
  /** Requires a real speech existence validator before any selection write. */
  validateTts: (id2) => requiredAccess().validateTts(id2),
  /** Awaits the actual theme owner's acknowledgement without synthesizing a receipt. */
  applyTheme: (id2) => requiredAccess().applyTheme(id2),
  /** Awaits the actual speech owner's acknowledgement without applying a different config on rejection. */
  applyTts: (id2) => requiredAccess().applyTts(id2)
};
function configurationRecords(values, label) {
  if (!Array.isArray(values)) throw new Error(label + "\u76EE\u5F55\u5FC5\u987B\u662F\u6570\u7EC4");
  const ids = /* @__PURE__ */ new Set();
  for (const value of values) {
    assertObject(value, label + "\u8BB0\u5F55");
    const id2 = requireId(value.id, label + " ID");
    if (ids.has(id2)) throw new Error(label + " ID \u91CD\u590D\uFF1A" + id2);
    ids.add(id2);
  }
  return values;
}
function requireConfiguration(values, id2, label) {
  requireId(id2, label + "\u5F15\u7528 ID");
  const records2 = configurationRecords(values, label);
  const matches = records2.filter(
    /** Resolves an exact ID, not a role-prefixed target, configured global or inferred participant. */
    (config) => config.id === id2
  );
  if (matches.length !== 1) throw new Error(label + "\u4E0D\u5B58\u5728\uFF1A" + id2);
  return matches[0];
}
function createSettingsConfigurationAccess(readSettings) {
  return {
    /** Projects actual named Theme configs into the plugin's one private picker row contract. */
    readThemeChoices() {
      return __async(this, null, function* () {
        const configs = configurationRecords(yield readSettings().listThemeConfigs(), "\u4E3B\u9898\u914D\u7F6E");
        return configs.map(
          /** Preserves the actual independent ID and display name without storing or rewriting its snapshot. */
          (config) => {
            assertString(config.name, "\u4E3B\u9898\u914D\u7F6E\u540D\u79F0");
            if (config.name.trim() === "") throw new Error("\u4E3B\u9898\u914D\u7F6E\u540D\u79F0\u4E0D\u80FD\u4E3A\u7A7A\uFF1A" + config.id);
            return { id: config.id, label: config.name };
          }
        );
      });
    },
    /** Verifies a genuine independent Theme ID before any active selection or chat namespace write. */
    validateTheme(id2) {
      return __async(this, null, function* () {
        requireConfiguration(yield readSettings().listThemeConfigs(), id2, "\u4E3B\u9898\u914D\u7F6E");
      });
    },
    /** Verifies a genuine independent TTS ID before any active selection or chat namespace write. */
    validateTts(id2) {
      return __async(this, null, function* () {
        requireConfiguration(yield readSettings().listTtsConfigs(), id2, "TTS \u914D\u7F6E");
      });
    },
    /** Awaits the canonical Theme transaction and validates its acknowledgement of this exact independent config. */
    applyTheme(id2) {
      return __async(this, null, function* () {
        requireId(id2, "\u4E3B\u9898\u914D\u7F6E\u5F15\u7528 ID");
        const applied = yield readSettings().applyThemeConfig(id2);
        assertObject(applied, "\u5DF2\u5E94\u7528\u4E3B\u9898\u914D\u7F6E");
        if (applied.id !== id2) throw new Error("\u4E3B\u9898\u914D\u7F6E\u5E94\u7528\u8FD4\u56DE\u7684 ID \u4E0E\u8BF7\u6C42\u4E0D\u4E00\u81F4\uFF1A" + id2);
      });
    },
    /** Awaits the canonical speech write and actual current-ID read; failures remain visible partial application errors. */
    applyTts(id2) {
      return __async(this, null, function* () {
        requireId(id2, "TTS \u914D\u7F6E\u5F15\u7528 ID");
        const applied = yield readSettings().setCurrentTtsConfigId(id2);
        if (applied !== id2) throw new Error("TTS \u914D\u7F6E\u63D0\u4EA4\u8FD4\u56DE\u7684 ID \u4E0E\u8BF7\u6C42\u4E0D\u4E00\u81F4\uFF1A" + id2);
        const current = yield readSettings().getCurrentTtsConfigId();
        if (current !== id2) throw new Error("TTS \u5F53\u524D\u914D\u7F6E\u8BFB\u53D6\u7ED3\u679C\u672A\u786E\u8BA4\u8BF7\u6C42\u7684 ID\uFF1A" + id2);
      });
    }
  };
}
function registerSelectionSettingsAccess() {
  connectIndependentConfigurations(createSettingsConfigurationAccess(
    /** Resolves the canonical Settings capability only when a real directory read or application is requested. */
    () => Tools.SoftwareSettings
  ));
}

// src/service.ts
function createCharacterCardsService(owner2, configurations = independentConfigurations) {
  const jobs = new MemoryJobRunner(owner2), application = createSelectionApplication(configurations);
  function dispatchRequest(request) {
    if (request.action === "listThemeChoices") return readThemeChoices(configurations);
    if (!isSelectionRequest(request)) return owner2.run(
      /** Executes ordinary editor operations against this same serialized snapshot. */
      (repository) => dispatch(request, repository)
    );
    return application.commit(
      /** Publishes the actual selection once; independent preference writes are not included in this file publication. */
      (validate) => owner2.run(
        /** Prevalidates the chosen complete record inside the same immutable snapshot used by its actual mutation. */
        (repository) => __async(null, null, function* () {
          const references = yield validate(yield readSelectionReferences(repository, editorSelection(request)));
          return { references, value: yield dispatch(request, repository) };
        })
      )
    );
  }
  return {
    /** Runs the private job scheduler without constructing another store or business service. */
    runMemoryJobs: () => jobs.tick(),
    /** Preserves action-specific results through the shared service dispatcher. */
    dispatch: dispatchRequest,
    /** Reads one consistent domain snapshot through the same operation queue. */
    snapshot: () => owner2.run(snapshot),
    /** Keeps every real selection alias and sidebar binding on the same awaited application sequence. */
    dispatchDomain(operation, input) {
      if (!isSelectionOperation(operation)) return owner2.run(
        /** Runs non-selection APIs without touching independent current configurations. */
        (repository) => dispatchDomain(operation, input, repository)
      );
      return application.commit(
        /** Returns a confirmed owner write before applying any independent preferences. */
        (validate) => owner2.run(
          /** Validates the actual domain request and all record references before writing the selected actor. */
          (repository) => __async(null, null, function* () {
            const selection2 = yield domainSelection(operation, input, repository);
            const references = yield validate(yield readSelectionReferences(repository, selection2));
            return { references, value: yield dispatchDomain(operation, input, repository) };
          })
        )
      );
    }
  };
}
function records(value, check, label) {
  if (!Array.isArray(value)) throw new Error(`${label} \u8FD4\u56DE\u683C\u5F0F\u4E0D\u6B63\u786E`);
  for (const record2 of value) check(record2);
  return value;
}
function assertStore(value) {
  assertObject(value, "\u5171\u4EAB\u8BB0\u5FC6\u5E93");
  requireId(value.id, "\u5171\u4EAB\u8BB0\u5FC6\u5E93\u6807\u8BC6");
  assertString(value.name, "\u5171\u4EAB\u8BB0\u5FC6\u5E93\u540D\u79F0");
  assertInteger(value.createdAt, "\u8BB0\u5FC6\u5E93\u521B\u5EFA\u65F6\u95F4", 0);
  assertInteger(value.updatedAt, "\u8BB0\u5FC6\u5E93\u4FEE\u6539\u65F6\u95F4", 0);
}
function assertActive(value) {
  assertObject(value, "\u5F53\u524D\u63D0\u793A\u8BCD");
  const keys = Object.keys(value);
  if (keys.length !== 1 || keys[0] !== "CharacterCard" && keys[0] !== "CharacterGroup") throw new Error("\u5F53\u524D\u63D0\u793A\u8BCD\u7C7B\u578B\u65E0\u6548");
  const selection2 = value[keys[0]];
  assertObject(selection2, "\u5F53\u524D\u63D0\u793A\u8BCD\u9009\u62E9");
  requireId(selection2.id, "\u5F53\u524D\u63D0\u793A\u8BCD\u6807\u8BC6");
}
function snapshot(host) {
  return __async(this, null, function* () {
    const [cards, groups, stores, tags, active, models, ttsConfigs, toolCatalog] = yield Promise.all([
      host.listCharacters(),
      host.listGroups(),
      host.listStores(),
      host.listTags(),
      host.readActive(),
      host.listModels(),
      host.listTtsConfigs(),
      host.readToolCatalog()
    ]);
    records(cards, assertCard, "\u89D2\u8272\u5361\u5217\u8868");
    records(groups, assertGroup, "\u89D2\u8272\u7EC4\u5217\u8868");
    records(stores, assertStore, "\u5171\u4EAB\u8BB0\u5FC6\u5E93\u5217\u8868");
    records(tags, assertTag, "\u6807\u7B7E\u5217\u8868");
    assertActive(active);
    if (!Array.isArray(models) || !Array.isArray(ttsConfigs)) throw new Error("\u6A21\u578B\u6216 TTS \u76EE\u5F55\u8FD4\u56DE\u683C\u5F0F\u4E0D\u6B63\u786E");
    assertObject(toolCatalog, "\u5DE5\u5177\u6765\u6E90\u76EE\u5F55");
    for (const field of ["builtinTools", "packages", "skills", "mcpServers"]) if (!Array.isArray(toolCatalog[field])) throw new Error(`\u5DE5\u5177\u6765\u6E90\u76EE\u5F55\u7F3A\u5C11 ${field}`);
    return { cards, groups, stores, tags, active, models, ttsConfigs, toolCatalog };
  });
}
function findRecord(values, id2, label) {
  let found;
  for (const value of values) {
    if (value.id !== id2) continue;
    if (found !== void 0) throw new Error(`${label} \u6807\u8BC6\u91CD\u590D\uFF1A${id2}`);
    found = value;
  }
  if (found === void 0) throw new Error(`${label} \u4E0D\u5B58\u5728\uFF1A${id2}`);
  return found;
}
function prepareTagChanges(attachedIds, changes2, tags) {
  assertStrings(attachedIds, "\u89D2\u8272\u6807\u7B7E");
  assertObject(changes2, "\u6807\u7B7E\u4FEE\u6539");
  if (!Array.isArray(changes2.created) || !Array.isArray(changes2.updated)) throw new Error("\u6807\u7B7E\u4FEE\u6539\u683C\u5F0F\u65E0\u6548");
  assertStrings(changes2.deleted, "\u5220\u9664\u6807\u7B7E\u6807\u8BC6");
  const created = [];
  const updated = [];
  const deleted = /* @__PURE__ */ new Set();
  const changedIds = /* @__PURE__ */ new Set();
  const draftIds = /* @__PURE__ */ new Set();
  for (const id2 of changes2.deleted) {
    requireId(id2, "\u5220\u9664\u6807\u7B7E\u6807\u8BC6");
    findRecord(tags, id2, "\u6807\u7B7E");
    if (deleted.has(id2)) throw new Error("\u6807\u7B7E\u5220\u9664\u9879\u91CD\u590D");
    deleted.add(id2);
  }
  for (const draft of changes2.created) {
    assertObject(draft, "\u65B0\u6807\u7B7E");
    const draftId = requireId(draft.draftId, "\u65B0\u6807\u7B7E\u8349\u7A3F\u6807\u8BC6");
    if (draftIds.has(draftId)) throw new Error("\u6807\u7B7E\u8349\u7A3F\u6807\u8BC6\u91CD\u590D");
    for (const tag of tags) if (tag.id === draftId) throw new Error("\u6807\u7B7E\u8349\u7A3F\u6807\u8BC6\u4E0E\u5DF2\u6709\u6807\u7B7E\u51B2\u7A81");
    assertTagValues(draft.values);
    const values = normalizedTagValues(draft.values);
    draftIds.add(draftId);
    created.push({ draftId, values });
  }
  for (const edit of changes2.updated) {
    assertTagValues(edit);
    const id2 = requireId(edit.id, "\u4FEE\u6539\u6807\u7B7E\u6807\u8BC6");
    if (changedIds.has(id2) || deleted.has(id2)) throw new Error("\u540C\u4E00\u6807\u7B7E\u5B58\u5728\u51B2\u7A81\u4FEE\u6539");
    const original = findRecord(tags, id2, "\u6807\u7B7E");
    changedIds.add(id2);
    updated.push({ ...original, ...normalizedTagValues(edit) });
  }
  const projected = [];
  for (const tag of tags) {
    if (deleted.has(tag.id)) continue;
    let current = tag;
    for (const update2 of updated) if (update2.id === tag.id) current = update2;
    projected.push({ id: tag.id, name: current.name.trim() });
  }
  for (const draft of created) projected.push({ id: draft.draftId, name: draft.values.name });
  for (const draft of created) for (const tag of projected) if (tag.id !== draft.draftId && tag.name === draft.values.name) throw new Error(`\u6807\u7B7E\u540D\u79F0\u5DF2\u5B58\u5728\uFF1A${draft.values.name}`);
  for (const update2 of updated) {
    const original = findRecord(tags, update2.id, "\u6807\u7B7E");
    if (original.name.trim() === update2.name) continue;
    for (const tag of projected) if (tag.id !== update2.id && tag.name === update2.name) throw new Error(`\u6807\u7B7E\u540D\u79F0\u5DF2\u5B58\u5728\uFF1A${update2.name}`);
  }
  const attached = [];
  const attachedSet = /* @__PURE__ */ new Set();
  for (const id2 of attachedIds) {
    requireId(id2, "\u89D2\u8272\u6807\u7B7E\u6807\u8BC6");
    if (deleted.has(id2)) continue;
    if (!draftIds.has(id2)) findRecord(tags, id2, "\u89D2\u8272\u6807\u7B7E");
    if (attachedSet.has(id2)) continue;
    attachedSet.add(id2);
    attached.push(id2);
  }
  return { created, updated, deleted: [...deleted], attachedIds: attached };
}
function normalizedTagValues(values) {
  return { name: requireName(values.name, "\u6807\u7B7E\u540D\u79F0"), description: values.description.trim(), promptContent: values.promptContent, tagType: values.tagType };
}
function commitTagChanges(plan, host) {
  return __async(this, null, function* () {
    const resolved = /* @__PURE__ */ new Map();
    for (const draft of plan.created) {
      const created = yield host.createTag(draft.values);
      assertTag(created);
      resolved.set(draft.draftId, created.id);
    }
    for (const tag of plan.updated) {
      const saved = yield host.updateTag(tag);
      assertTag(saved);
    }
    for (const id2 of plan.deleted) yield host.deleteTag(id2);
    const ids = [];
    for (const id2 of plan.attachedIds) {
      if (resolved.has(id2)) {
        const canonicalId = resolved.get(id2);
        if (canonicalId === void 0) throw new Error("\u65B0\u6807\u7B7E\u6CA1\u6709\u8FD4\u56DE canonical \u6807\u8BC6");
        ids.push(canonicalId);
      } else ids.push(id2);
    }
    return normalizeNames(ids);
  });
}
function normalizedCharacter(card) {
  if (card.chatModelBindingMode !== "FOLLOW_GLOBAL" && card.chatModelBindingMode !== "FIXED_MODEL") throw new Error("\u804A\u5929\u6A21\u578B\u7ED1\u5B9A\u6A21\u5F0F\u65E0\u6548");
  if (card.memoryBindingMode !== "CHARACTER" && card.memoryBindingMode !== "SHARED") throw new Error("\u8BB0\u5FC6\u7ED1\u5B9A\u6A21\u5F0F\u65E0\u6548");
  if (card.chatModelBindingMode === "FIXED_MODEL") requireId(card.chatModelId, "\u56FA\u5B9A\u804A\u5929\u6A21\u578B");
  if (card.memoryBindingMode === "SHARED") requireId(card.sharedMemoryId, "\u5171\u4EAB\u8BB0\u5FC6\u5E93");
  if (card.ttsConfigId !== null) requireId(card.ttsConfigId, "TTS \u914D\u7F6E");
  if (card.themeConfigId !== null) requireId(card.themeConfigId, "\u4E3B\u9898\u914D\u7F6E");
  const policy = card.toolAccessConfig;
  if (policy.enabled && (policy.allowedPackages.length !== 0 || policy.allowedSkills.length !== 0 || policy.allowedMcpServers.length !== 0) && !new Set(normalizeNames(policy.allowedBuiltinTools)).has("use_package")) throw new Error("\u9009\u62E9\u5916\u90E8\u5DE5\u5177\u6765\u6E90\u65F6\u5FC5\u987B\u5141\u8BB8 use_package");
  const mounts = [];
  const mountIds = /* @__PURE__ */ new Set();
  for (const mount2 of card.sharedMemoryMounts) {
    if (mountIds.has(mount2.sharedMemoryId)) throw new Error("\u5171\u4EAB\u8BB0\u5FC6\u6302\u8F7D\u91CD\u590D");
    mountIds.add(mount2.sharedMemoryId);
    mounts.push({ sharedMemoryId: mount2.sharedMemoryId, readable: mount2.readable, writable: mount2.writable });
  }
  return {
    ...card,
    name: requireName(card.name, "\u89D2\u8272\u5361\u540D\u79F0"),
    description: card.description.trim(),
    attachedTagIds: [...card.attachedTagIds],
    sharedMemoryMounts: mounts,
    toolAccessConfig: {
      enabled: card.toolAccessConfig.enabled,
      allowedBuiltinTools: normalizeNames(card.toolAccessConfig.allowedBuiltinTools),
      allowedPackages: normalizeNames(card.toolAccessConfig.allowedPackages),
      allowedSkills: normalizeNames(card.toolAccessConfig.allowedSkills),
      allowedMcpServers: normalizeNames(card.toolAccessConfig.allowedMcpServers)
    }
  };
}
function saveCharacter(card, create, tagChanges, host) {
  return __async(this, null, function* () {
    assertCard(card);
    assertBoolean(create, "\u89D2\u8272\u65B0\u5EFA\u6807\u8BC6");
    let value = normalizedCharacter(card);
    if (create) {
      if (value.isDefault) throw new Error("\u4E0D\u80FD\u65B0\u5EFA\u9ED8\u8BA4\u89D2\u8272\u5361");
      value = { ...value, id: "", createdAt: 0, updatedAt: 0 };
    } else {
      const id2 = requireId(value.id, "\u89D2\u8272\u5361\u6807\u8BC6");
      const original = yield host.getCharacter(id2);
      assertCard(original);
      value = { ...value, id: original.id, isDefault: original.isDefault, createdAt: original.createdAt };
    }
    const [tags, stores, cards] = yield Promise.all([host.listTags(), host.listStores(), host.listCharacters()]);
    records(cards, assertCard, "\u89D2\u8272\u5361\u5217\u8868");
    let retainedName = false;
    if (!create) {
      for (const existing of cards) if (existing.id === value.id && existing.name.trim() === value.name) retainedName = true;
    }
    if (!retainedName) {
      for (const existing of cards) if (existing.id !== value.id && existing.name.trim() === value.name) throw new Error(`\u89D2\u8272\u5361\u540D\u79F0\u5DF2\u5B58\u5728\uFF1A${value.name}`);
    }
    if (value.ttsConfigId !== null) {
      const configs = yield host.listTtsConfigs();
      findRecord(configs, value.ttsConfigId, "TTS \u914D\u7F6E");
    }
    records(tags, assertTag, "\u6807\u7B7E\u5217\u8868");
    records(stores, assertStore, "\u5171\u4EAB\u8BB0\u5FC6\u5E93\u5217\u8868");
    const plan = prepareTagChanges(value.attachedTagIds, tagChanges, tags);
    if (value.memoryBindingMode === "SHARED") findRecord(stores, requireId(value.sharedMemoryId, "\u5171\u4EAB\u8BB0\u5FC6\u5E93\u6807\u8BC6"), "\u5171\u4EAB\u8BB0\u5FC6\u5E93");
    for (const mount2 of value.sharedMemoryMounts) findRecord(stores, mount2.sharedMemoryId, "\u5171\u4EAB\u8BB0\u5FC6\u5E93");
    value = { ...value, attachedTagIds: yield commitTagChanges(plan, host) };
    const saved = create ? yield host.createCharacter(value) : yield host.updateCharacter(value);
    assertCard(saved);
    requireId(saved.id, "\u4FDD\u5B58\u7684\u89D2\u8272\u5361\u6807\u8BC6");
    return saved;
  });
}
function saveGroup(group, create, host) {
  return __async(this, null, function* () {
    assertGroupValues(group);
    assertBoolean(create, "\u7FA4\u7EC4\u65B0\u5EFA\u6807\u8BC6");
    const name = requireName(group.name, "\u89D2\u8272\u7EC4\u540D\u79F0");
    const cards = records(yield host.listCharacters(), assertCard, "\u89D2\u8272\u5361\u5217\u8868");
    const members2 = [];
    for (const member2 of group.members) {
      findRecord(cards, member2.characterCardId, "\u7FA4\u7EC4\u89D2\u8272");
      members2.push({ characterCardId: member2.characterCardId, orderIndex: member2.orderIndex });
    }
    let value;
    if (create) value = { id: "", name, description: group.description.trim(), members: members2, themeConfigId: group.themeConfigId, createdAt: 0, updatedAt: 0 };
    else {
      const original = yield host.getGroup(requireId(group.id, "\u7FA4\u7EC4\u6807\u8BC6"));
      assertGroup(original);
      value = { ...original, name, description: group.description.trim(), members: members2, themeConfigId: group.themeConfigId };
    }
    const saved = create ? yield host.createGroup(value) : yield host.updateGroup(value);
    assertGroup(saved);
    return saved;
  });
}
function saveTag(tag, create, host) {
  return __async(this, null, function* () {
    assertTagValues(tag);
    assertBoolean(create, "\u6807\u7B7E\u65B0\u5EFA\u6807\u8BC6");
    const values = normalizedTagValues(tag);
    let saved;
    if (create) saved = yield host.createTag(values);
    else {
      const tags = records(yield host.listTags(), assertTag, "\u6807\u7B7E\u5217\u8868");
      const original = findRecord(tags, requireId(tag.id, "\u6807\u7B7E\u6807\u8BC6"), "\u6807\u7B7E");
      saved = yield host.updateTag({ ...original, ...values });
    }
    assertTag(saved);
    return saved;
  });
}
function saveStore(store, create, host) {
  return __async(this, null, function* () {
    assertObject(store, "\u5171\u4EAB\u8BB0\u5FC6\u5E93");
    assertBoolean(create, "\u8BB0\u5FC6\u5E93\u65B0\u5EFA\u6807\u8BC6");
    const name = requireName(store.name, "\u5171\u4EAB\u8BB0\u5FC6\u5E93\u540D\u79F0");
    let saved;
    if (create) saved = yield host.createStore(name);
    else {
      const id2 = requireId(store.id, "\u5171\u4EAB\u8BB0\u5FC6\u5E93\u6807\u8BC6");
      findRecord(records(yield host.listStores(), assertStore, "\u5171\u4EAB\u8BB0\u5FC6\u5E93\u5217\u8868"), id2, "\u5171\u4EAB\u8BB0\u5FC6\u5E93");
      saved = yield host.renameStore(id2, name);
    }
    assertStore(saved);
    return saved;
  });
}
function activate(type, id2, host) {
  return __async(this, null, function* () {
    requireId(id2, "\u5F53\u524D\u63D0\u793A\u8BCD\u6807\u8BC6");
    if (type === "card") {
      assertCard(yield host.getCharacter(id2));
      yield host.writeActive({ CharacterCard: { id: id2 } });
    } else if (type === "group") {
      assertGroup(yield host.getGroup(id2));
      yield host.writeActive({ CharacterGroup: { id: id2 } });
    } else throw new Error("\u5F53\u524D\u63D0\u793A\u8BCD\u7C7B\u578B\u65E0\u6548");
  });
}
function parseOwner(ownerKey) {
  assertString(ownerKey, "\u8BB0\u5FC6\u5E93\u6807\u8BC6");
  const parts = ownerKey.split(":");
  if (parts.length !== 2 || !/^[^:\s]+$/.test(parts[1])) throw new Error("\u8BB0\u5FC6\u5E93\u6807\u8BC6\u683C\u5F0F\u65E0\u6548");
  if (parts[0] === "character") return { kind: "CHARACTER", id: parts[1] };
  if (parts[0] === "shared") return { kind: "SHARED", id: parts[1] };
  throw new Error("\u8BB0\u5FC6\u5E93\u547D\u540D\u7A7A\u95F4\u65E0\u6548");
}
function requireOwner(ownerKey, host) {
  return __async(this, null, function* () {
    const owner2 = parseOwner(ownerKey);
    if (owner2.kind === "CHARACTER") assertCard(yield host.getCharacter(owner2.id));
    else findRecord(records(yield host.listStores(), assertStore, "\u5171\u4EAB\u8BB0\u5FC6\u5E93\u5217\u8868"), owner2.id, "\u5171\u4EAB\u8BB0\u5FC6\u5E93");
    return owner2;
  });
}
function resolveMemoryOwner(characterId, host) {
  return __async(this, null, function* () {
    const card = yield host.getCharacter(requireId(characterId, "\u89D2\u8272\u5361\u6807\u8BC6"));
    assertCard(card);
    let ownerKey;
    if (card.memoryBindingMode === "CHARACTER") ownerKey = `character:${card.id}`;
    else if (card.memoryBindingMode === "SHARED") ownerKey = `shared:${requireId(card.sharedMemoryId, "\u5171\u4EAB\u8BB0\u5FC6\u5E93\u7ED1\u5B9A")}`;
    else throw new Error("\u8BB0\u5FC6\u7ED1\u5B9A\u6A21\u5F0F\u65E0\u6548");
    yield requireOwner(ownerKey, host);
    return ownerKey;
  });
}
function listMemories(ownerKey, host) {
  return __async(this, null, function* () {
    yield requireOwner(ownerKey, host);
    return records(yield host.listMemories(ownerKey), assertMemory, "\u8BB0\u5FC6\u5217\u8868");
  });
}
function graph(ownerKey, host) {
  return __async(this, null, function* () {
    yield requireOwner(ownerKey, host);
    const [value, items] = yield Promise.all([host.readMemoryGraph(ownerKey), host.listMemories(ownerKey)]);
    assertGraph(value);
    records(items, assertMemory, "\u8BB0\u5FC6\u5217\u8868");
    return { graph: value, items };
  });
}
function memoryByTitle2(items, title) {
  const name = requireName(title, "\u8BB0\u5FC6\u6807\u9898");
  let found;
  for (const item of items) {
    if (item.title !== name) continue;
    if (found !== void 0) throw new Error(`\u8BB0\u5FC6\u6807\u9898\u4E0D\u552F\u4E00\uFF0C\u8BF7\u4F7F\u7528\u6570\u636E\u5E93\u6807\u8BC6\u7F16\u8F91\uFF1A${name}`);
    found = item;
  }
  if (found === void 0) throw new Error(`\u8BB0\u5FC6\u4E0D\u5B58\u5728\uFF1A${name}`);
  return found;
}
function normalizeFolderPath(value) {
  if (value === null) return null;
  const parts = [];
  for (const part of value.replace(/\\/g, "/").split("/")) {
    const name = part.trim();
    if (name !== "") parts.push(name);
  }
  return parts.length === 0 ? null : parts.join("/");
}
function saveMemory(ownerKey, values, id2, host) {
  return __async(this, null, function* () {
    assertMemoryValues(values);
    yield requireOwner(ownerKey, host);
    const normalized = { ...values, title: requireName(values.title, "\u8BB0\u5FC6\u6807\u9898"), contentType: values.contentType.trim(), source: values.source.trim(), folderPath: normalizeFolderPath(values.folderPath), tags: normalizeNames(values.tags) };
    let saved;
    if (id2 === null) saved = yield host.createMemory(ownerKey, normalized);
    else {
      requireDecimal(id2, "\u8BB0\u5FC6\u8BB0\u5F55\u6807\u8BC6", true);
      saved = yield host.updateMemory(ownerKey, id2, normalized);
    }
    assertMemory(saved);
    return saved;
  });
}
function searchMemories(options, host) {
  return __async(this, null, function* () {
    assertSearchOptions(options);
    yield requireOwner(options.ownerKey, host);
    return records(yield host.searchMemories({ ...options, folderPath: normalizeFolderPath(options.folderPath) }), assertMemory, "\u8BB0\u5FC6\u641C\u7D22");
  });
}
function combinePrompts(characterId, additionalTagIds, promptFunctionType, host) {
  return __async(this, null, function* () {
    const card = yield host.getCharacter(requireId(characterId, "\u89D2\u8272\u5361\u6807\u8BC6"));
    assertCard(card);
    assertStrings(additionalTagIds, "\u9644\u52A0\u6807\u7B7E");
    if (promptFunctionType !== "CHAT" && promptFunctionType !== "VOICE") throw new Error("\u63D0\u793A\u8BCD\u529F\u80FD\u7C7B\u578B\u65E0\u6548");
    const tags = records(yield host.listTags(), assertTag, "\u6807\u7B7E\u5217\u8868");
    const parts = [];
    const setting = card.characterSetting.trim();
    if (setting !== "") parts.push(setting);
    const otherContent = (promptFunctionType === "CHAT" ? card.otherContentChat : card.otherContentVoice).trim();
    if (otherContent !== "") parts.push(otherContent);
    const ids = normalizeNames([...card.attachedTagIds, ...additionalTagIds]);
    for (const id2 of ids) {
      const content = findRecord(tags, id2, "\u63D0\u793A\u8BCD\u6807\u7B7E").promptContent.trim();
      if (content !== "") parts.push(content);
    }
    const advanced = card.advancedCustomPrompt.trim();
    if (advanced !== "") parts.push(advanced);
    return parts.join("\n\n").trim();
  });
}
function writeMemorySettings(ownerKey, settings2, host) {
  return __async(this, null, function* () {
    assertMemorySettings(settings2);
    yield requireOwner(ownerKey, host);
    yield host.writeMemorySettings(ownerKey, { ...settings2 });
    const saved = yield host.readMemorySettings(ownerKey);
    assertMemorySettings(saved);
    return saved;
  });
}
function writeMemorySearchConfig(ownerKey, config, host) {
  return __async(this, null, function* () {
    assertSearchConfig(config);
    yield requireOwner(ownerKey, host);
    yield host.writeMemorySearchConfig(ownerKey, { ...config });
    const saved = yield host.readMemorySearchConfig(ownerKey);
    assertSearchConfig(saved);
    return saved;
  });
}
function dispatch(request, host) {
  return __async(this, null, function* () {
    assertObject(request, "\u89D2\u8272\u5361\u8BF7\u6C42");
    switch (request.action) {
      case "snapshot":
        return snapshot(host);
      case "listThemeChoices":
        return readThemeChoices(independentConfigurations);
      case "listCharacters":
        return records(yield host.listCharacters(), assertCard, "\u89D2\u8272\u5361\u5217\u8868");
      case "getCharacter": {
        const card = yield host.getCharacter(requireId(request.id, "\u89D2\u8272\u5361\u6807\u8BC6"));
        assertCard(card);
        return card;
      }
      case "saveCharacter":
        yield saveCharacter(request.card, request.create, request.tagChanges, host);
        return snapshot(host);
      case "deleteCharacter": {
        const id2 = requireId(request.id, "\u89D2\u8272\u5361\u6807\u8BC6");
        const card = yield host.getCharacter(id2);
        assertCard(card);
        if (card.isDefault) throw new Error("\u9ED8\u8BA4\u89D2\u8272\u5361\u4E0D\u80FD\u5220\u9664");
        yield host.deleteCharacter(id2);
        return snapshot(host);
      }
      case "activate":
        yield activate(request.type, request.id, host);
        return snapshot(host);
      case "readActive": {
        const active = yield host.readActive();
        assertActive(active);
        return active;
      }
      case "readChatBinding":
        return executeDomain("chat.configuration.binding.read", { chatId: request.chatId }, host);
      case "writeChatBinding":
        return executeDomain("chat.configuration.binding.write", { chatId: request.chatId, selection: request.selection }, host);
      case "deleteChatBinding":
        return executeDomain("chat.configuration.binding.delete", { chatId: request.chatId }, host);
      case "listConversationGroups":
        return executeDomain("conversation-group.list", { ownerSelection: request.ownerSelection }, host);
      case "createConversationGroup":
        return executeDomain("conversation-group.create", request.values, host);
      case "updateConversationGroup":
        return executeDomain("conversation-group.update", { id: request.id, changes: request.changes }, host);
      case "deleteConversationGroup":
        return executeDomain("conversation-group.delete", { id: request.id }, host);
      case "moveConversationGroupChat":
        return executeDomain("conversation-group.moveChat", { chatId: request.chatId, groupId: request.groupId, ownerSelection: request.ownerSelection }, host);
      case "reorderConversationGroups":
        return executeDomain("conversation-group.reorder", { ownerSelection: request.ownerSelection, ids: request.ids }, host);
      case "listGroups":
        return records(yield host.listGroups(), assertGroup, "\u89D2\u8272\u7EC4\u5217\u8868");
      case "getGroup": {
        const group = yield host.getGroup(requireId(request.id, "\u7FA4\u7EC4\u6807\u8BC6"));
        assertGroup(group);
        return group;
      }
      case "saveGroup":
        yield saveGroup(request.group, request.create, host);
        return snapshot(host);
      case "deleteGroup":
        yield host.deleteGroup(requireId(request.id, "\u7FA4\u7EC4\u6807\u8BC6"));
        return snapshot(host);
      case "listTags":
        return records(yield host.listTags(), assertTag, "\u6807\u7B7E\u5217\u8868");
      case "saveTag":
        return saveTag(request.tag, request.create, host);
      case "deleteTag":
        yield host.deleteTag(requireId(request.id, "\u6807\u7B7E\u6807\u8BC6"));
        return snapshot(host);
      case "listStores":
        return records(yield host.listStores(), assertStore, "\u5171\u4EAB\u8BB0\u5FC6\u5E93\u5217\u8868");
      case "saveStore":
        yield saveStore(request.store, request.create, host);
        return snapshot(host);
      case "deleteStore":
        yield host.deleteStore(requireId(request.id, "\u5171\u4EAB\u8BB0\u5FC6\u5E93\u6807\u8BC6"));
        return snapshot(host);
      case "listModels":
        return host.listModels();
      case "listTtsConfigs":
        return host.listTtsConfigs();
      case "readToolCatalog":
        return host.readToolCatalog();
      case "resolveMemoryOwner":
        return resolveMemoryOwner(request.characterId, host);
      case "readUser": {
        yield requireOwner(request.ownerKey, host);
        const user = yield host.readUser(request.ownerKey);
        assertObject(user, "\u7528\u6237\u8D44\u6599");
        assertString(user.content, "\u7528\u6237\u8D44\u6599\u5185\u5BB9");
        if (user.ownerKey !== request.ownerKey) throw new Error("\u7528\u6237\u8D44\u6599 owner \u4E0E\u8BF7\u6C42\u4E0D\u4E00\u81F4");
        return user;
      }
      case "writeUser":
        yield requireOwner(request.ownerKey, host);
        assertString(request.content, "\u7528\u6237\u8D44\u6599\u5185\u5BB9");
        yield host.writeUser(request.ownerKey, request.content);
        return { saved: true };
      case "graph":
        return graph(request.ownerKey, host);
      case "listMemories":
        return listMemories(request.ownerKey, host);
      case "searchMemory":
        return searchMemories({ ownerKey: request.ownerKey, query: request.query, folderPath: null, relevanceThreshold: 0, createdAtStartMs: null, createdAtEndMs: null }, host);
      case "searchMemories":
        return searchMemories(request.options, host);
      case "saveMemory": {
        assertString(request.tags, "\u8BB0\u5FC6\u6807\u7B7E");
        assertString(request.folderPath, "\u8BB0\u5FC6\u6587\u4EF6\u5939");
        const values = { title: request.title, content: request.content, contentType: request.contentType, source: request.source, credibility: request.credibility, importance: request.importance, folderPath: request.folderPath, tags: normalizeNames(request.tags.split(",")) };
        const id2 = request.originalTitle === null ? null : memoryByTitle2(yield listMemories(request.ownerKey, host), request.originalTitle).id;
        yield saveMemory(request.ownerKey, values, id2, host);
        return graph(request.ownerKey, host);
      }
      case "deleteMemory": {
        const memory = memoryByTitle2(yield listMemories(request.ownerKey, host), request.title);
        yield host.deleteMemory(request.ownerKey, memory.id);
        return graph(request.ownerKey, host);
      }
      case "createLink": {
        assertNumber(request.weight, "\u5173\u7CFB\u5F3A\u5EA6", 0, 1);
        assertString(request.description, "\u5173\u7CFB\u63CF\u8FF0");
        const type_ = requireName(request.linkType, "\u5173\u7CFB\u7C7B\u578B");
        const items = yield listMemories(request.ownerKey, host);
        const source = memoryByTitle2(items, request.sourceTitle), target = memoryByTitle2(items, request.targetTitle);
        if (source.id === target.id) throw new Error("\u4E0D\u80FD\u521B\u5EFA\u8BB0\u5FC6\u81EA\u8EAB\u5173\u7CFB");
        yield host.createLink(request.ownerKey, { sourceMemoryId: source.id, targetMemoryId: target.id, type_, weight: request.weight, description: request.description });
        return graph(request.ownerKey, host);
      }
      case "deleteLink":
        yield requireOwner(request.ownerKey, host);
        requireDecimal(request.linkId, "\u8BB0\u5FC6\u5173\u7CFB\u6807\u8BC6", true);
        yield host.deleteLink(request.ownerKey, request.linkId);
        return graph(request.ownerKey, host);
      case "updateLink": {
        yield requireOwner(request.ownerKey, host);
        requireDecimal(request.linkId, "\u8BB0\u5FC6\u5173\u7CFB\u6807\u8BC6", true);
        assertNumber(request.weight, "\u5173\u7CFB\u5F3A\u5EA6", 0, 1);
        assertString(request.description, "\u5173\u7CFB\u63CF\u8FF0");
        yield host.updateLink(request.ownerKey, request.linkId, { type_: requireName(request.linkType, "\u5173\u7CFB\u7C7B\u578B"), weight: request.weight, description: request.description });
        return graph(request.ownerKey, host);
      }
      case "readMemorySettings": {
        yield requireOwner(request.ownerKey, host);
        const settings2 = yield host.readMemorySettings(request.ownerKey);
        assertMemorySettings(settings2);
        return settings2;
      }
      case "writeMemorySettings":
        return writeMemorySettings(request.ownerKey, request.settings, host);
      case "readMemorySearchConfig": {
        yield requireOwner(request.ownerKey, host);
        const config = yield host.readMemorySearchConfig(request.ownerKey);
        assertSearchConfig(config);
        return config;
      }
      case "writeMemorySearchConfig":
        return writeMemorySearchConfig(request.ownerKey, request.config, host);
      case "combinePrompts":
        return combinePrompts(request.characterId, request.additionalTagIds, request.promptFunctionType, host);
      case "importCharacter": {
        const tags = records(yield host.listTags(), assertTag, "\u6807\u7B7E\u5217\u8868");
        const plan = decodeCharacterImport(request.content, request.format, tags);
        yield saveCharacter(plan.card, true, plan.tagChanges, host);
        return snapshot(host);
      }
      case "exportCharacter": {
        const card = yield host.getCharacter(requireId(request.id, "\u89D2\u8272\u5361\u6807\u8BC6"));
        assertCard(card);
        const tags = records(yield host.listTags(), assertTag, "\u6807\u7B7E\u5217\u8868");
        return encodeCharacterExport(card, request.format, tags);
      }
      case "importGroup":
        yield saveGroup(decodeGroupImport(request.content), true, host);
        return snapshot(host);
      case "exportGroup": {
        const group = yield host.getGroup(requireId(request.id, "\u7FA4\u7EC4\u6807\u8BC6"));
        assertGroup(group);
        return JSON.stringify(group, null, 2);
      }
      case "listMemoryFolders":
        yield requireOwner(request.ownerKey, host);
        return host.listMemoryFolders(request.ownerKey);
      case "readMemoryAutoSaveStatus":
        yield requireOwner(request.ownerKey, host);
        return host.readMemoryAutoSaveStatus(request.ownerKey);
      case "listMemoryChats":
        return executeDomain("memory.chat.list", { ownerKey: request.ownerKey }, host);
      case "updateChatMemory":
        return executeDomain("memory.chat.update", { ownerKey: request.ownerKey, chatId: request.chatId }, host);
      case "autoCategorizeMemory":
        return executeDomain("memory.categorize", { ownerKey: request.ownerKey }, host);
      case "startMemoryRebuild":
        return executeDomain("memory.rebuild.start", { ownerKey: request.ownerKey, rebuild: request.rebuild }, host);
      case "readMemoryRebuildProgress":
        return executeDomain("memory.rebuild.progress", { ownerKey: request.ownerKey }, host);
      case "cancelMemoryRebuild":
        return executeDomain("memory.rebuild.cancel", { ownerKey: request.ownerKey }, host);
      case "rebuildMemoryEmbeddings":
        return executeDomain("memory.embeddings.rebuild", { ownerKey: request.ownerKey }, host);
      case "exportMemory":
        yield requireOwner(request.ownerKey, host);
        return { ownerKey: request.ownerKey, content: yield exportMemoryBackup(request.ownerKey, host) };
      case "importMemory": {
        yield requireOwner(request.ownerKey, host);
        if (request.strategy !== "SKIP" && request.strategy !== "UPDATE" && request.strategy !== "CREATE_NEW") throw new Error("\u8BB0\u5FC6\u5BFC\u5165\u7B56\u7565\u65E0\u6548");
        const document = decodeMemoryBackup(request.content);
        const result2 = yield host.importMemorySpace(request.ownerKey, document.space, request.strategy);
        yield host.writeUser(request.ownerKey, document.userMarkdown);
        return { ownerKey: request.ownerKey, result: result2 };
      }
      default: {
        const action = request;
        throw new Error(`\u672A\u77E5\u89D2\u8272\u5361\u8BF7\u6C42\uFF1A${JSON.stringify(action)}`);
      }
    }
  });
}
function dispatchDomain(operation, input, host) {
  return executeDomain(operation, input, host);
}

// src/memory-search.ts
function tokens(text3) {
  const words = text3.toLocaleLowerCase().match(/[\p{L}\p{N}_]+/gu);
  const result2 = [];
  if (words === null) return result2;
  for (const word of words) {
    if (/^[\p{Script=Han}]+$/u.test(word)) {
      const characters = [...word];
      for (let index = 0; index < characters.length; index += 1) {
        result2.push(characters[index]);
        if (index + 1 < characters.length) result2.push(characters[index] + characters[index + 1]);
      }
    } else result2.push(word);
  }
  return result2;
}
function counts(values) {
  const result2 = /* @__PURE__ */ new Map();
  for (const value of values) {
    const previous = result2.get(value);
    result2.set(value, previous === void 0 ? 1 : previous + 1);
  }
  return result2;
}
function matchesFilters(memory, options) {
  if (options.folderPath !== null && memory.folderPath !== options.folderPath && !(memory.folderPath !== null && memory.folderPath.startsWith(options.folderPath + "/"))) return false;
  if (options.createdAtStartMs !== null && memory.createdAt < options.createdAtStartMs) return false;
  if (options.createdAtEndMs !== null && memory.createdAt > options.createdAtEndMs) return false;
  return true;
}
function searchMemorySpace(space, options) {
  return __async(this, null, function* () {
    const config = space.searchConfig;
    const memories = space.memories.filter(
      /** Keeps complete records under the caller's explicit filters. */
      (memory) => matchesFilters(memory, options)
    );
    if (options.query.trim() === "*") return memories;
    const query = [...new Set(tokens(options.query))];
    if (query.length === 0) throw new Error("Memory search query has no searchable tokens");
    const index = memories.map(
      /** Builds frequencies from the complete title, content and document chunks. */
      (memory) => {
        const chunks = space.chunks.filter(
          /** Selects chunks by the actual persisted UUID, never graph labels. */
          (chunk) => chunk.memoryUuid === memory.uuid
        );
        const text3 = memory.title + "\n" + memory.content + "\n" + chunks.map(
          /** Retains every document chunk's searchable content. */
          (chunk) => chunk.content
        ).join("\n");
        const values = tokens(text3);
        return { memory, length: values.length, frequencies: counts(values), tags: new Set(memory.tags.flatMap(
          /** Indexes real memory tag names independently from content. */
          (tag) => tokens(tag.name)
        )) };
      }
    );
    if (index.length === 0) return [];
    const average = index.reduce(
      /** Accumulates actual token lengths for length normalization. */
      (total, document) => total + document.length,
      0
    ) / index.length;
    const lexical = /* @__PURE__ */ new Map(), tagScores = /* @__PURE__ */ new Map();
    let maximum = 0;
    for (const document of index) {
      let bm25 = 0, hits = 0;
      for (const token of query) {
        const df = index.filter(
          /** Counts documents with this exact token. */
          (candidate) => candidate.frequencies.has(token)
        ).length;
        const frequency = document.frequencies.get(token);
        if (frequency !== void 0) {
          const idf = Math.log(1 + (index.length - df + 0.5) / (df + 0.5));
          const lengthFactor = average === 0 ? 1 : document.length / average;
          bm25 += idf * frequency * 2.2 / (frequency + 1.2 * (0.25 + 0.75 * lengthFactor));
        }
        if (document.tags.has(token)) hits += 1;
      }
      lexical.set(document.memory.id, bm25);
      tagScores.set(document.memory.id, hits / query.length);
      maximum = Math.max(maximum, bm25);
    }
    const semantic = /* @__PURE__ */ new Map();
    if (config.vectorWeight > 0) {
      const queryVector = yield embeddingFor(space, options.query);
      for (const document of index) {
        const memory = document.memory;
        const vector = yield embeddingFor(space, memory.isDocumentNode ? memory.title : memory.content);
        let similarity = Math.max(0, cosineSimilarity(queryVector, vector));
        if (memory.isDocumentNode) {
          for (const chunk of space.chunks) if (chunk.memoryUuid === memory.uuid) similarity = Math.max(similarity, cosineSimilarity(queryVector, yield embeddingFor(space, chunk.content)));
        }
        semantic.set(memory.id, similarity);
      }
    }
    let keywordMultiplier, semanticMultiplier, edgeMultiplier;
    switch (config.scoreMode) {
      case "BALANCED":
        keywordMultiplier = 1;
        semanticMultiplier = 1;
        edgeMultiplier = 1;
        break;
      case "KEYWORD_FIRST":
        keywordMultiplier = 1.3;
        semanticMultiplier = 0.8;
        edgeMultiplier = 0.9;
        break;
      case "SEMANTIC_FIRST":
        keywordMultiplier = 0.8;
        semanticMultiplier = 1.3;
        edgeMultiplier = 1.1;
        break;
      default:
        throw new Error("Invalid memory scoring mode");
    }
    const keywordWeight = config.keywordWeight * keywordMultiplier, tagWeight = config.tagWeight * keywordMultiplier, vectorWeight = config.vectorWeight * semanticMultiplier, edgeWeight = config.edgeWeight * edgeMultiplier;
    const scores = [];
    for (const document of index) {
      const lexicalValue = lexical.get(document.memory.id), tagValue = tagScores.get(document.memory.id);
      if (lexicalValue === void 0 || tagValue === void 0) throw new Error("Search index is inconsistent");
      const keyword = maximum === 0 ? 0 : lexicalValue / maximum;
      let edge = 0;
      for (const link2 of space.links) {
        let other;
        if (link2.sourceMemoryId === document.memory.id) other = link2.targetMemoryId;
        else if (link2.targetMemoryId === document.memory.id) other = link2.sourceMemoryId;
        else continue;
        const otherLexical = lexical.get(other);
        if (otherLexical !== void 0 && maximum !== 0) edge = Math.max(edge, link2.weight * otherLexical / maximum);
      }
      const denominator = keywordWeight + tagWeight + vectorWeight + edgeWeight;
      if (denominator === 0) throw new Error("Memory search has no enabled scoring weights");
      let vector = 0;
      if (vectorWeight > 0) {
        const value = semantic.get(document.memory.id);
        if (value === void 0) throw new Error("Semantic search index is inconsistent");
        vector = value;
      }
      const score2 = (keyword * keywordWeight + tagValue * tagWeight + vector * vectorWeight + edge * edgeWeight) / denominator;
      if (score2 > 0 && score2 >= options.relevanceThreshold) scores.push({ memory: document.memory, score: score2 });
    }
    scores.sort(
      /** Orders actual ranked hits with a stable lossless identity tie breaker. */
      (left, right) => right.score - left.score || (BigInt(left.memory.id) < BigInt(right.memory.id) ? -1 : BigInt(left.memory.id) > BigInt(right.memory.id) ? 1 : 0)
    );
    return scores.map(
      /** Returns full records rather than a lossy search projection. */
      (entry) => entry.memory
    );
  });
}

// src/repository.ts
function copy(value) {
  return JSON.parse(JSON.stringify(value));
}
function get(records2, id2, label) {
  requireId(id2, label + " id");
  const recordsWithId = records2.filter(
    /** Resolves the exact identity rather than guessing names. */
    (record2) => record2.id === id2
  );
  if (recordsWithId.length !== 1) throw new Error(label + " does not identify exactly one record: " + id2);
  return recordsWithId[0];
}
function uniqueName(records2, name, id2) {
  for (const record2 of records2) if (record2.id !== id2 && record2.name === name) throw new Error("Name already exists: " + name);
}
function folder(value) {
  if (value === null) return null;
  const parts = value.replace(/\\/g, "/").split("/").map(
    /** Trims individual folder names using the plugin's slash-delimited format. */
    (part) => part.trim()
  ).filter(
    /** Removes separators, not missing persisted data. */
    (part) => part !== ""
  );
  for (const part of parts) if (part === "." || part === "..") throw new Error("Relative folder traversal is not allowed");
  return parts.length === 0 ? null : parts.join("/");
}
function uuid() {
  const bytes = [];
  for (let index = 0; index < 16; index += 1) bytes.push(Math.floor(Math.random() * 256));
  bytes[6] = bytes[6] & 15 | 64;
  bytes[8] = bytes[8] & 63 | 128;
  const text3 = bytes.map(
    /** Encodes every byte as two hexadecimal digits. */
    (byte) => byte.toString(16).padStart(2, "0")
  ).join("");
  return text3.slice(0, 8) + "-" + text3.slice(8, 12) + "-" + text3.slice(12, 16) + "-" + text3.slice(16, 20) + "-" + text3.slice(20);
}
var RepositorySession = class {
  /** Receives the operation snapshot and explicitly connected external directory readers. */
  constructor(state, directory, directories = null) {
    this.state = state;
    this.directory = directory;
    this.directories = directories;
    this.documents = /* @__PURE__ */ new Map();
  }
  /** Exposes staged owner documents to the sole file publisher without creating another persistent copy. */
  documentWrites() {
    return [...this.documents.values()].map(
      /** Copies the exact staged write envelope. */
      (document) => ({ ...document })
    );
  }
  /** Initializes a real owner document only during explicit owner creation. */
  addSpace(ownerKey) {
    const space = createMemorySpace(ownerKey);
    this.state.owners.push(space);
    this.documents.set(ownerKey, { ownerKey, path: this.directory + "/" + space.userDocumentPath, content: "", newDocument: true });
  }
  /** Allocates lossless plugin-local decimal identities without converting them into Number. */
  allocate() {
    const id2 = requireDecimal(this.state.nextId, "nextId", true), next = BigInt(id2) + 1n;
    if (next > 9223372036854775807n) throw new Error("Plugin identity space is exhausted");
    this.state.nextId = next.toString();
    return id2;
  }
  /** Advances the counter for an explicitly imported full identity. */
  reserve(id2) {
    const value = BigInt(requireDecimal(id2, "imported id", true));
    if (value >= BigInt(this.state.nextId)) {
      if (value === 9223372036854775807n) throw new Error("Imported identity exhausts the plugin identity space");
      this.state.nextId = (value + 1n).toString();
    }
  }
  /** Reserves numeric suffixes from this plugin's explicit entity identity format. */
  reserveEntity(id2) {
    const match = /^(character|group|tag|shared)-([1-9][0-9]*)$/.exec(id2);
    if (match !== null) this.reserve(match[2]);
  }
  /** Requires an actual owner and its existing complete space. */
  space(ownerKey) {
    const parts = ownerKey.split(":");
    if (parts.length !== 2) throw new Error("Invalid memory owner namespace");
    if (parts[0] === "character") get(this.state.cards, parts[1], "character");
    else if (parts[0] === "shared") get(this.state.stores, parts[1], "shared store");
    else throw new Error("Invalid memory owner namespace");
    const spaces = this.state.owners.filter(
      /** Selects the persisted exact owner key. */
      (space) => space.ownerKey === ownerKey
    );
    if (spaces.length !== 1) throw new Error("Missing or duplicate memory space: " + ownerKey);
    return spaces[0];
  }
  /** Resolves only explicitly connected real external directory readers. */
  external() {
    if (this.directories === null) throw new Error("External model, TTS and full tool-source directory readers have not been connected");
    return this.directories;
  }
  /** Lists complete stored characters. */
  listCharacters() {
    return __async(this, null, function* () {
      return copy(this.state.cards);
    });
  }
  /** Reads one complete stored character. */
  getCharacter(id2) {
    return __async(this, null, function* () {
      return copy(get(this.state.cards, id2, "character"));
    });
  }
  /** Creates all character fields and its genuine initialized memory space. */
  createCharacter(card) {
    return __async(this, null, function* () {
      assertCard(card);
      const name = requireName(card.name, "character name");
      uniqueName(this.state.cards, name, "");
      if (card.isDefault) throw new Error("Only the installed product default can be a default character");
      const now = Date.now(), saved = { ...copy(card), id: "character-" + this.allocate(), name, createdAt: now, updatedAt: now };
      this.state.cards.push(saved);
      this.addSpace("character:" + saved.id);
      return copy(saved);
    });
  }
  /** Writes the full editable character while preserving its stable creation identity. */
  updateCharacter(card) {
    return __async(this, null, function* () {
      assertCard(card);
      const original = get(this.state.cards, card.id, "character"), name = requireName(card.name, "character name");
      uniqueName(this.state.cards, name, card.id);
      const saved = { ...copy(card), name, isDefault: original.isDefault, createdAt: original.createdAt, updatedAt: Date.now() };
      Object.assign(original, saved);
      return copy(original);
    });
  }
  /** Removes a non-active non-default character and all of its owned records. */
  deleteCharacter(id2) {
    return __async(this, null, function* () {
      const card = get(this.state.cards, id2, "character");
      if (card.isDefault) throw new Error("The default character cannot be deleted");
      if ("CharacterCard" in this.state.active && this.state.active.CharacterCard.id === id2) throw new Error("Select another character before deleting the active character");
      yield this.requireUnboundSelection("card:" + id2);
      for (const group of this.state.groups) for (const member2 of group.members) if (member2.characterCardId === id2) yield this.requireUnboundSelection("group:" + group.id);
      this.state.cards.splice(this.state.cards.indexOf(card), 1);
      this.documents.delete("character:" + id2);
      this.state.owners = this.state.owners.filter(
        /** Removes this exact deleted owner's full space. */
        (space) => space.ownerKey !== "character:" + id2
      );
      for (const group of this.state.groups) {
        const members2 = group.members.filter(
          /** Removes the deleted actor from actual group memberships. */
          (member2) => member2.characterCardId !== id2
        );
        if (members2.length !== group.members.length) {
          group.members = members2;
          group.updatedAt = Date.now();
        }
      }
    });
  }
  /** Performs the explicit reset command against the genuine product default. */
  resetDefaultCharacter() {
    return __async(this, null, function* () {
      const defaults = this.state.cards.filter(
        /** Identifies the one persisted default character. */
        (card) => card.isDefault
      );
      if (defaults.length !== 1) throw new Error("The product default character is missing or ambiguous");
      const original = defaults[0], reset = createDefaultCharacter(Date.now());
      reset.id = original.id;
      reset.createdAt = original.createdAt;
      Object.assign(original, reset);
      return copy(original);
    });
  }
  /** Lists full manual metadata in the exact supplied scope and persisted display order. */
  listConversationGroups(ownerSelection) {
    return __async(this, null, function* () {
      assertConversationGroupScope(ownerSelection);
      const groups = this.state.conversationGroups.filter(
        /** Uses opaque scope equality without reading Core role, workspace or group fields. */
        (group) => group.ownerSelection === ownerSelection
      ).sort(
        /** Preserves the complete persisted scoped order. */
        (left, right) => left.displayOrder - right.displayOrder
      );
      return copy(groups);
    });
  }
  /** Reads every scoped manual record without dropping memberships or replacing opaque tokens. */
  readConversationGroupsForBackup() {
    return __async(this, null, function* () {
      return copy(this.state.conversationGroups);
    });
  }
  /** Creates a real empty group with explicit scope, pin state, identity and timestamps. */
  createConversationGroup(values) {
    return __async(this, null, function* () {
      assertConversationGroupScope(values.ownerSelection);
      assertBoolean(values.pinned, "conversation group pinned");
      const name = requireName(values.name, "conversation group name"), groups = yield this.listConversationGroups(values.ownerSelection);
      uniqueName(groups, name, "");
      const now = Date.now();
      const created = { id: this.allocate(), ownerSelection: values.ownerSelection, name, chatIds: [], displayOrder: groups.length, pinned: values.pinned, createdAt: now, updatedAt: now };
      this.state.conversationGroups.push(created);
      return copy(created);
    });
  }
  /** Applies only explicit editable metadata while keeping complete membership and creation fields. */
  updateConversationGroup(id2, changes2) {
    return __async(this, null, function* () {
      assertConversationGroupChanges(changes2);
      const original = get(this.state.conversationGroups, requireDecimal(id2, "conversation group id", true), "conversation group");
      const updated = { ...copy(original), ...copy(changes2) };
      updated.name = requireName(updated.name, "conversation group name");
      uniqueName(yield this.listConversationGroups(original.ownerSelection), updated.name, original.id);
      updated.updatedAt = Date.now();
      Object.assign(original, updated);
      return copy(original);
    });
  }
  /** Removes only this group's metadata, releases its chats and compacts its own scoped order. */
  deleteConversationGroup(id2) {
    return __async(this, null, function* () {
      const original = get(this.state.conversationGroups, requireDecimal(id2, "conversation group id", true), "conversation group");
      const result2 = { id: id2, deleted: true, releasedChatIds: copy(original.chatIds) };
      this.state.conversationGroups.splice(this.state.conversationGroups.indexOf(original), 1);
      const groups = yield this.listConversationGroups(original.ownerSelection), now = Date.now();
      for (let index = 0; index < groups.length; index += 1) {
        const record2 = get(this.state.conversationGroups, groups[index].id, "conversation group");
        if (record2.displayOrder !== index) {
          record2.displayOrder = index;
          record2.updatedAt = now;
        }
      }
      return result2;
    });
  }
  /** Transfers one membership atomically across the private snapshot without changing any chat binding. */
  moveConversationGroupChat(chatId, groupId, ownerSelection) {
    return __async(this, null, function* () {
      requireId(chatId, "conversation chat id");
      assertConversationGroupScope(ownerSelection);
      const previous = this.state.conversationGroups.filter(
        /** Locates only the genuine stored membership of this exact chat. */
        (group) => group.chatIds.indexOf(chatId) !== -1
      );
      if (previous.length > 1) throw new Error("Chat belongs to multiple manual groups: " + chatId);
      const target = groupId === null ? null : get(this.state.conversationGroups, requireDecimal(groupId, "conversation group id", true), "conversation group");
      if (target !== null && target.ownerSelection !== ownerSelection) throw new Error("Target conversation group has a different scope");
      if (target === null && previous.length === 1 && previous[0].ownerSelection !== ownerSelection) throw new Error("Cannot unassign a conversation group from another scope");
      const previousGroupId = previous.length === 0 ? null : previous[0].id;
      if (previousGroupId === groupId) return { chatId, previousGroupId, groupId };
      const now = Date.now();
      if (previous.length === 1) {
        previous[0].chatIds.splice(previous[0].chatIds.indexOf(chatId), 1);
        previous[0].updatedAt = now;
      }
      if (target !== null) {
        target.chatIds.push(chatId);
        target.updatedAt = now;
      }
      return { chatId, previousGroupId, groupId };
    });
  }
  /** Requires the caller's complete scoped permutation before changing any persisted ordering. */
  reorderConversationGroups(ownerSelection, ids) {
    return __async(this, null, function* () {
      assertConversationGroupScope(ownerSelection);
      const groups = yield this.listConversationGroups(ownerSelection), supplied = /* @__PURE__ */ new Set();
      if (ids.length !== groups.length) throw new Error("Reorder requires all conversation groups in the requested scope");
      for (const id2 of ids) {
        requireDecimal(id2, "conversation group id", true);
        if (supplied.has(id2)) throw new Error("Reorder contains a duplicate conversation group");
        supplied.add(id2);
        const group = get(this.state.conversationGroups, id2, "conversation group");
        if (group.ownerSelection !== ownerSelection) throw new Error("Reorder contains a conversation group from another scope");
      }
      const now = Date.now();
      for (let index = 0; index < ids.length; index += 1) {
        const group = get(this.state.conversationGroups, ids[index], "conversation group");
        if (group.displayOrder !== index) {
          group.displayOrder = index;
          group.updatedAt = now;
        }
      }
      return this.listConversationGroups(ownerSelection);
    });
  }
  /** Restores an explicit full manual-group backup, preserving every field and reserving lossless IDs. */
  restoreConversationGroups(groups) {
    return __async(this, null, function* () {
      assertConversationGroups(groups);
      for (const group of groups) this.reserve(group.id);
      this.state.conversationGroups = copy(groups);
    });
  }
  /** Lists complete groups. */
  listGroups() {
    return __async(this, null, function* () {
      return copy(this.state.groups);
    });
  }
  /** Reads the exact stored group. */
  getGroup(id2) {
    return __async(this, null, function* () {
      return copy(get(this.state.groups, id2, "group"));
    });
  }
  /** Stores a complete newly created group. */
  createGroup(group) {
    return __async(this, null, function* () {
      assertGroup(group);
      const name = requireName(group.name, "group name");
      uniqueName(this.state.groups, name, "");
      const now = Date.now(), saved = { ...copy(group), id: "group-" + this.allocate(), name, createdAt: now, updatedAt: now };
      this.state.groups.push(saved);
      return copy(saved);
    });
  }
  /** Writes every supplied group field. */
  updateGroup(group) {
    return __async(this, null, function* () {
      assertGroup(group);
      const original = get(this.state.groups, group.id, "group"), name = requireName(group.name, "group name");
      uniqueName(this.state.groups, name, group.id);
      Object.assign(original, copy(group), { name, createdAt: original.createdAt, updatedAt: Date.now() });
      return copy(original);
    });
  }
  /** Deletes only a group that is not selected as active. */
  deleteGroup(id2) {
    return __async(this, null, function* () {
      const group = get(this.state.groups, id2, "group");
      if ("CharacterGroup" in this.state.active && this.state.active.CharacterGroup.id === id2) throw new Error("Select another prompt before deleting the active group");
      yield this.requireUnboundSelection("group:" + id2);
      this.state.groups.splice(this.state.groups.indexOf(group), 1);
    });
  }
  /** Lists full tags. */
  listTags() {
    return __async(this, null, function* () {
      return copy(this.state.tags);
    });
  }
  /** Reads one full tag. */
  getTag(id2) {
    return __async(this, null, function* () {
      return copy(get(this.state.tags, id2, "tag"));
    });
  }
  /** Writes a new tag with all editable values. */
  createTag(values) {
    return __async(this, null, function* () {
      assertTagValues(values);
      const name = requireName(values.name, "tag name");
      uniqueName(this.state.tags, name, "");
      const now = Date.now(), saved = { ...copy(values), name, id: "tag-" + this.allocate(), createdAt: now, updatedAt: now };
      this.state.tags.push(saved);
      return copy(saved);
    });
  }
  /** Writes a full existing tag. */
  updateTag(tag) {
    return __async(this, null, function* () {
      assertTag(tag);
      const original = get(this.state.tags, tag.id, "tag"), name = requireName(tag.name, "tag name");
      uniqueName(this.state.tags, name, tag.id);
      Object.assign(original, copy(tag), { name, createdAt: original.createdAt, updatedAt: Date.now() });
      return copy(original);
    });
  }
  /** Deletes a tag and its attachments within this same private snapshot. */
  deleteTag(id2) {
    return __async(this, null, function* () {
      const tag = get(this.state.tags, id2, "tag");
      this.state.tags.splice(this.state.tags.indexOf(tag), 1);
      for (const card of this.state.cards) {
        const attached = card.attachedTagIds.filter(
          /** Removes only this exact deleted tag reference. */
          (tagId) => tagId !== id2
        );
        if (attached.length !== card.attachedTagIds.length) {
          card.attachedTagIds = attached;
          card.updatedAt = Date.now();
        }
      }
    });
  }
  /** Lists real shared libraries. */
  listStores() {
    return __async(this, null, function* () {
      return copy(this.state.stores);
    });
  }
  /** Reads one exact shared library. */
  getStore(id2) {
    return __async(this, null, function* () {
      return copy(get(this.state.stores, id2, "shared store"));
    });
  }
  /** Creates the shared library and all of its real initial settings. */
  createStore(name) {
    return __async(this, null, function* () {
      name = requireName(name, "library name");
      uniqueName(this.state.stores, name, "");
      const now = Date.now();
      const store = { id: "shared-" + this.allocate(), name, createdAt: now, updatedAt: now };
      this.state.stores.push(store);
      this.addSpace("shared:" + store.id);
      return copy(store);
    });
  }
  /** Renames one stored shared library. */
  renameStore(id2, name) {
    return __async(this, null, function* () {
      const store = get(this.state.stores, id2, "shared store");
      name = requireName(name, "library name");
      uniqueName(this.state.stores, name, id2);
      store.name = name;
      store.updatedAt = Date.now();
      return copy(store);
    });
  }
  /** Deletes the library and explicitly detaches all existing bindings and mounts. */
  deleteStore(id2) {
    return __async(this, null, function* () {
      const store = get(this.state.stores, id2, "shared store");
      this.state.stores.splice(this.state.stores.indexOf(store), 1);
      this.documents.delete("shared:" + id2);
      this.state.owners = this.state.owners.filter(
        /** Removes the deleted library's owned data, not another namespace. */
        (space) => space.ownerKey !== "shared:" + id2
      );
      for (const card of this.state.cards) {
        let changed = false;
        if (card.sharedMemoryId === id2) {
          card.sharedMemoryId = null;
          card.memoryBindingMode = "CHARACTER";
          changed = true;
        }
        const mounts = card.sharedMemoryMounts.filter(
          /** Detaches only this deleted library. */
          (mount2) => mount2.sharedMemoryId !== id2
        );
        if (mounts.length !== card.sharedMemoryMounts.length) {
          card.sharedMemoryMounts = mounts;
          changed = true;
        }
        if (changed) card.updatedAt = Date.now();
      }
    });
  }
  /** Delegates only to a genuinely supplied external model reader. */
  listModels() {
    return this.external().listModels();
  }
  /** Delegates only to a genuinely supplied external speech reader. */
  listTtsConfigs() {
    return this.external().listTtsConfigs();
  }
  /** Delegates only to a genuinely supplied complete tool-source reader. */
  readToolCatalog() {
    return this.external().readToolCatalog();
  }
  /** Reads the exact active selection. */
  readActive() {
    return __async(this, null, function* () {
      return copy(this.state.active);
    });
  }
  /** Writes an explicitly selected existing active entity. */
  writeActive(active) {
    return __async(this, null, function* () {
      if ("CharacterCard" in active) get(this.state.cards, active.CharacterCard.id, "character");
      else get(this.state.groups, active.CharacterGroup.id, "group");
      this.state.active = copy(active);
    });
  }
  /** Rejects deletion using only selections on existing authoritative host records. */
  requireUnboundSelection(selection2) {
    return __async(this, null, function* () {
      for (const binding of yield this.listChatBindings()) if (binding.selection === selection2) throw new Error("Rebind or delete the chat extension before deleting its selection: " + binding.chatId);
    });
  }
  /** Enumerates only real host records and their authenticated plugin extension markers. */
  listChatBindings() {
    return __async(this, null, function* () {
      return listChatExtensionBindings();
    });
  }
  /** Reads the sole record-extension source and validates its actual stored domain reference. */
  readChatBinding(chatId) {
    return __async(this, null, function* () {
      const binding = yield readChatExtensionBinding(chatId);
      requireChatSelection(binding.selection, this.state.cards, this.state.groups);
      return binding;
    });
  }
  /** Writes an explicit selector submission solely to the authenticated host record namespace. */
  writeChatBinding(binding) {
    return __async(this, null, function* () {
      requireChatSelection(binding.selection, this.state.cards, this.state.groups);
      return writeChatExtensionBinding(binding);
    });
  }
  /** Deletes only the authenticated record namespace without touching manual membership or file state. */
  deleteChatBinding(chatId) {
    return __async(this, null, function* () {
      return deleteChatExtensionBinding(chatId);
    });
  }
  /** Reads the actual owner USER.md file or the edit staged within this same operation. */
  readUser(ownerKey) {
    return __async(this, null, function* () {
      const path = yield this.readUserPath(ownerKey), document = this.documents.get(ownerKey);
      if (document !== void 0) return { ownerKey, content: document.content };
      const result2 = yield Tools.Files.read(path);
      assertString(result2.content, "USER.md content");
      return { ownerKey, content: result2.content };
    });
  }
  /** Requires the existing document before staging a save; missing files are never repaired. */
  writeUser(ownerKey, content) {
    return __async(this, null, function* () {
      assertString(content, "USER.md");
      yield this.readUser(ownerKey);
      const document = this.documents.get(ownerKey);
      this.documents.set(ownerKey, { ownerKey, path: yield this.readUserPath(ownerKey), content, newDocument: document !== void 0 && document.newDocument });
    });
  }
  /** Returns a genuine file path accepted by Tools.Files.read/write. */
  readUserPath(ownerKey) {
    return __async(this, null, function* () {
      return this.directory + "/" + this.space(ownerKey).userDocumentPath;
    });
  }
  /** Lists only owners that genuinely exist in this plugin snapshot. */
  listMemoryOwnerKeys() {
    return __async(this, null, function* () {
      return this.state.owners.map(
        /** Preserves each actual initialized namespace. */
        (space) => space.ownerKey
      );
    });
  }
  /** Allocates a lossless identity for a private persisted job or candidate. */
  allocateRecordId() {
    return __async(this, null, function* () {
      return this.allocate();
    });
  }
  /** Stores a complete private owner record without shadowing its USER.md content. */
  writeMemorySpace(ownerKey, space) {
    return __async(this, null, function* () {
      const original = this.space(ownerKey);
      assertMemorySpace(space);
      if (space.ownerKey !== ownerKey || space.userDocumentPath !== original.userDocumentPath) throw new Error("Memory owner identity or USER.md path cannot change");
      for (const records2 of [space.memories, space.links, space.chunks, space.candidates]) for (const record2 of records2) this.reserve(record2.id);
      if (space.rebuildTask !== null) this.reserve(space.rebuildTask.id);
      Object.assign(original, copy(space));
    });
  }
  /** Reads the complete owner space for full backups. */
  readMemorySpace(ownerKey) {
    return __async(this, null, function* () {
      return copy(this.space(ownerKey));
    });
  }
  /** Lists full memory records, including every stored document node. */
  listMemories(ownerKey) {
    return __async(this, null, function* () {
      return copy(this.space(ownerKey).memories);
    });
  }
  /** Lists root and nested folders from actual full records. */
  listMemoryFolders(ownerKey) {
    return __async(this, null, function* () {
      const folders = /* @__PURE__ */ new Set();
      for (const memory of this.space(ownerKey).memories) {
        if (memory.folderPath === null) {
          folders.add("");
          continue;
        }
        const parts = memory.folderPath.split("/");
        for (let length = 1; length <= parts.length; length += 1) folders.add(parts.slice(0, length).join("/"));
      }
      return [...folders].sort();
    });
  }
  /** Builds a faithful UUID graph from full stored records without dropping malformed links. */
  readMemoryGraph(ownerKey) {
    return __async(this, null, function* () {
      const space = this.space(ownerKey), graph2 = { nodes: [], edges: [] };
      for (const memory of space.memories) {
        let color = 4292072403;
        if (memory.isDocumentNode) color = 4287985101;
        else if (memory.tags.length > 0 && memory.tags[0].name === "Person") color = 4286695300;
        else if (memory.tags.length > 0 && memory.tags[0].name === "Concept") color = 4284790262;
        graph2.nodes.push({ id: memory.uuid, label: memory.title, color, metadata: { memoryId: memory.id } });
      }
      for (const link2 of space.links) {
        const source = get(space.memories, link2.sourceMemoryId, "source memory"), target = get(space.memories, link2.targetMemoryId, "target memory");
        graph2.edges.push({ id: link2.id, sourceId: source.uuid, targetId: target.uuid, label: link2.type_, weight: link2.weight, metadata: { description: link2.description }, isCrossFolderLink: source.folderPath !== target.folderPath });
      }
      return graph2;
    });
  }
  /** Searches real full records using the local lexical/tag/graph algorithm. */
  searchMemories(options) {
    return __async(this, null, function* () {
      assertSearchOptions(options);
      return copy(yield searchMemorySpace(this.space(options.ownerKey), options));
    });
  }
  /** Creates an intentional new non-document memory. */
  createMemory(ownerKey, values) {
    return __async(this, null, function* () {
      assertMemoryValues(values);
      const now = Date.now();
      return this.saveMemory(ownerKey, { ...values, id: "", uuid: "", title: requireName(values.title, "memory title"), folderPath: folder(values.folderPath), documentPath: null, isDocumentNode: false, chunkIndexFilePath: null, createdAt: now, updatedAt: now, lastAccessedAt: now, tags: normalizeNames(values.tags).map(
        /** Initializes explicit new tag records using decimal identities. */
        (name, index) => ({ id: String(index + 1), name })
      ), properties: [] });
    });
  }
  /** Saves the entire memory record and preserves all fields not explicitly changed. */
  saveMemory(ownerKey, memory) {
    return __async(this, null, function* () {
      const space = this.space(ownerKey), now = Date.now(), incoming = copy(memory);
      incoming.title = requireName(incoming.title, "memory title");
      incoming.folderPath = folder(incoming.folderPath);
      if (incoming.id === "") {
        incoming.id = this.allocate();
        if (incoming.uuid === "") incoming.uuid = uuid();
        incoming.createdAt = now;
        incoming.updatedAt = now;
        incoming.lastAccessedAt = now;
        assertMemory(incoming);
        uniqueName(space.memories.map(
          /** Checks title uniqueness for explicit creation. */
          (item) => ({ id: item.id, name: item.title })
        ), incoming.title, incoming.id);
        for (const item of space.memories) if (item.uuid === incoming.uuid) throw new Error("Memory UUID already exists");
        space.memories.push(incoming);
        return copy(incoming);
      }
      requireDecimal(incoming.id, "memory id", true);
      const original = get(space.memories, incoming.id, "memory");
      if (incoming.uuid !== original.uuid) throw new Error("An existing memory UUID is immutable");
      incoming.createdAt = original.createdAt;
      incoming.updatedAt = now;
      assertMemory(incoming);
      uniqueName(space.memories.map(
        /** Checks title uniqueness while retaining the current memory identity. */
        (item) => ({ id: item.id, name: item.title })
      ), incoming.title, incoming.id);
      Object.assign(original, incoming);
      return copy(original);
    });
  }
  /** Updates editable fields without projecting away document metadata or properties. */
  updateMemory(ownerKey, id2, values) {
    return __async(this, null, function* () {
      assertMemoryValues(values);
      const original = get(this.space(ownerKey).memories, id2, "memory");
      return this.saveMemory(ownerKey, { ...copy(original), ...values, tags: normalizeNames(values.tags).map(
        /** Allocates the explicit replacement tag list without changing the memory identity. */
        (name, index) => ({ id: String(index + 1), name })
      ) });
    });
  }
  /** Deletes a memory and all referencing chunks and links in this operation. */
  deleteMemory(ownerKey, id2) {
    return __async(this, null, function* () {
      requireDecimal(id2, "memory id", true);
      const space = this.space(ownerKey), memory = get(space.memories, id2, "memory");
      space.memories.splice(space.memories.indexOf(memory), 1);
      space.chunks = space.chunks.filter(
        /** Removes the deleted document's exact UUID chunks. */
        (chunk) => chunk.memoryUuid !== memory.uuid
      );
      space.links = space.links.filter(
        /** Removes links with either exact deleted endpoint. */
        (link2) => link2.sourceMemoryId !== id2 && link2.targetMemoryId !== id2
      );
    });
  }
  /** Moves only explicitly selected existing memories. */
  moveMemories(ownerKey, ids, folderPath) {
    return __async(this, null, function* () {
      const space = this.space(ownerKey), destination = folder(folderPath), selected = new Set(ids);
      for (const id2 of selected) {
        requireDecimal(id2, "memory id", true);
        const memory = get(space.memories, id2, "memory");
        memory.folderPath = destination;
        memory.updatedAt = Date.now();
      }
    });
  }
  /** Lists full relationships including descriptions. */
  listMemoryLinks(ownerKey) {
    return __async(this, null, function* () {
      return copy(this.space(ownerKey).links);
    });
  }
  /** Reads one exact full relationship. */
  readMemoryLink(ownerKey, id2) {
    return __async(this, null, function* () {
      requireDecimal(id2, "link id", true);
      return copy(get(this.space(ownerKey).links, id2, "link"));
    });
  }
  /** Creates a relationship only between existing distinct owner-scoped memories. */
  createLink(ownerKey, values) {
    return __async(this, null, function* () {
      const space = this.space(ownerKey);
      get(space.memories, values.sourceMemoryId, "source memory");
      get(space.memories, values.targetMemoryId, "target memory");
      if (values.sourceMemoryId === values.targetMemoryId) throw new Error("Self relationships are not allowed");
      assertNumber(values.weight, "link weight", 0, 1);
      assertString(values.description, "link description");
      const type_ = requireName(values.type_, "link type");
      const saved = { ...copy(values), type_, id: this.allocate() };
      space.links.push(saved);
      return copy(saved);
    });
  }
  /** Writes every editable field of one exact relationship. */
  updateLink(ownerKey, id2, values) {
    return __async(this, null, function* () {
      const link2 = get(this.space(ownerKey).links, requireDecimal(id2, "link id", true), "link");
      assertNumber(values.weight, "link weight", 0, 1);
      assertString(values.description, "link description");
      Object.assign(link2, copy(values), { type_: requireName(values.type_, "link type") });
      return copy(link2);
    });
  }
  /** Removes one exact relationship. */
  deleteLink(ownerKey, id2) {
    return __async(this, null, function* () {
      const links = this.space(ownerKey).links, link2 = get(links, requireDecimal(id2, "link id", true), "link");
      links.splice(links.indexOf(link2), 1);
    });
  }
  /** Reads complete persisted memory settings. */
  readMemorySettings(ownerKey) {
    return __async(this, null, function* () {
      return copy(this.space(ownerKey).settings);
    });
  }
  /** Writes complete memory settings without silently activating unsupported embedding. */
  writeMemorySettings(ownerKey, settings2) {
    return __async(this, null, function* () {
      assertMemorySettings(settings2);
      this.space(ownerKey).settings = copy(settings2);
    });
  }
  /** Reads complete persisted search configuration. */
  readMemorySearchConfig(ownerKey) {
    return __async(this, null, function* () {
      return copy(this.space(ownerKey).searchConfig);
    });
  }
  /** Writes all explicitly selected search weights. */
  writeMemorySearchConfig(ownerKey, config) {
    return __async(this, null, function* () {
      assertSearchConfig(config);
      this.space(ownerKey).searchConfig = copy(config);
    });
  }
  /** Counts genuine candidate metadata without pretending a scheduler is running. */
  readMemoryAutoSaveStatus(ownerKey) {
    return __async(this, null, function* () {
      const space = this.space(ownerKey), chats = /* @__PURE__ */ new Set();
      let pendingCandidates = 0, processingCandidates = 0, failedCandidates = 0, lastError = "";
      for (const candidate of space.candidates) {
        if (candidate.status === "pending") {
          pendingCandidates += 1;
          chats.add(candidate.chatId);
        } else if (candidate.status === "processing") processingCandidates += 1;
        else if (candidate.status === "failed") {
          failedCandidates += 1;
          lastError = candidate.lastError;
        }
      }
      const nextRunAtMs = space.settings.nextAutoSaveRunAtMs;
      return { ownerKey, pendingCandidates, pendingChats: chats.size, processingCandidates, failedCandidates, nextRunAtMs, minutesUntilNextRun: Math.max(0, Math.ceil((nextRunAtMs - Date.now()) / 6e4)), lastError };
    });
  }
  /** Reads actual persisted progress; the new-install idle state is not a fake running job. */
  readMemoryRebuildProgress(ownerKey) {
    return __async(this, null, function* () {
      return copy(this.space(ownerKey).rebuildProgress);
    });
  }
  /** Cancels only an actual active job record. */
  cancelMemoryRebuild(ownerKey) {
    return __async(this, null, function* () {
      const progress = this.space(ownerKey).rebuildProgress;
      if (progress.status !== "running" && progress.status !== "preparing") throw new Error("No memory rebuild is active");
      progress.status = "cancelled";
    });
  }
  /** Restores explicit complete character/tag backup records through this snapshot. */
  restoreCharacters(cards, tags) {
    return __async(this, null, function* () {
      const result2 = { new: 0, updated: 0, skipped: 0, total: cards.length };
      for (const tag of tags) {
        assertTag(tag);
        this.reserveEntity(tag.id);
        const matches = this.state.tags.filter(
          /** Resolves the imported tag by its full stable identity. */
          (existing) => existing.id === tag.id
        );
        if (matches.length === 0) this.state.tags.push(copy(tag));
        else Object.assign(matches[0], copy(tag));
      }
      for (const card of cards) {
        assertCard(card);
        requireId(card.id, "backup character id");
        this.reserveEntity(card.id);
        const matches = this.state.cards.filter(
          /** Resolves the imported character by exact identity. */
          (existing) => existing.id === card.id
        );
        if (matches.length === 0) {
          this.state.cards.push(copy(card));
          this.addSpace("character:" + card.id);
          result2.new += 1;
        } else {
          Object.assign(matches[0], copy(card));
          result2.updated += 1;
        }
      }
      return result2;
    });
  }
  /** Restores full group backups without creating missing character references. */
  restoreGroups(groups) {
    return __async(this, null, function* () {
      const result2 = { new: 0, updated: 0, skipped: 0, total: groups.length };
      for (const group of groups) {
        assertGroup(group);
        this.reserveEntity(group.id);
        const matches = this.state.groups.filter(
          /** Resolves each imported group by exact stable identity. */
          (existing) => existing.id === group.id
        );
        if (matches.length === 0) {
          this.state.groups.push(copy(group));
          result2.new += 1;
        } else {
          Object.assign(matches[0], copy(group));
          result2.updated += 1;
        }
      }
      return result2;
    });
  }
  /** Imports complete document-aware records, relationships, settings and USER text in this operation. */
  importMemorySpace(ownerKey, imported, strategy) {
    return __async(this, null, function* () {
      assertMemorySpace(imported);
      const space = this.space(ownerKey), result2 = { newMemories: 0, updatedMemories: 0, skippedMemories: 0, newLinks: 0 };
      const idMapping = /* @__PURE__ */ new Map();
      for (const memory of imported.memories) {
        const existing = space.memories.find(
          /** Matches the import's actual UUID, not a guessed title. */
          (item) => item.uuid === memory.uuid
        );
        if (existing !== void 0 && strategy === "SKIP") {
          result2.skippedMemories += 1;
          idMapping.set(memory.id, existing.id);
          continue;
        }
        const value = copy(memory);
        if (existing !== void 0 && strategy === "UPDATE") {
          value.id = existing.id;
          Object.assign(existing, value);
          result2.updatedMemories += 1;
        } else {
          if (strategy === "CREATE_NEW") {
            value.id = this.allocate();
            value.uuid = uuid();
          } else {
            if (space.memories.some(
              /** Rejects an occupied imported identity instead of remapping it silently. */
              (item) => item.id === value.id
            )) throw new Error("Imported memory id is occupied: " + value.id);
            this.reserve(value.id);
          }
          space.memories.push(value);
          result2.newMemories += 1;
        }
        idMapping.set(memory.id, value.id);
        space.chunks = space.chunks.filter(
          /** Replaces only the explicit updated document's prior chunks. */
          (chunk) => chunk.memoryUuid !== value.uuid
        );
        for (const chunk of imported.chunks) if (chunk.memoryUuid === memory.uuid) {
          const incoming = { ...copy(chunk), memoryUuid: value.uuid, id: strategy === "CREATE_NEW" ? this.allocate() : chunk.id };
          if (space.chunks.some(
            /** Rejects duplicate chunk identities in the target space. */
            (record2) => record2.id === incoming.id
          )) throw new Error("Imported chunk id is occupied: " + incoming.id);
          this.reserve(incoming.id);
          space.chunks.push(incoming);
        }
      }
      for (const link2 of imported.links) {
        const source = idMapping.get(link2.sourceMemoryId), target = idMapping.get(link2.targetMemoryId);
        if (source === void 0 || target === void 0) throw new Error("Imported link has no mapped endpoint");
        const existing = space.links.find(
          /** Resolves only a matching relationship identity in non-copy imports. */
          (record2) => record2.id === link2.id
        );
        if (existing !== void 0 && strategy === "SKIP") continue;
        const incoming = { ...copy(link2), sourceMemoryId: source, targetMemoryId: target, id: strategy === "CREATE_NEW" ? this.allocate() : link2.id };
        this.reserve(incoming.id);
        if (existing !== void 0 && strategy === "UPDATE") Object.assign(existing, incoming);
        else {
          space.links.push(incoming);
          result2.newLinks += 1;
        }
      }
      space.settings = copy(imported.settings);
      space.searchConfig = copy(imported.searchConfig);
      space.rebuildProgress = copy(imported.rebuildProgress);
      space.rebuildTask = copy(imported.rebuildTask);
      space.embeddings = copy(imported.embeddings);
      if (space.rebuildTask !== null) this.reserve(space.rebuildTask.id);
      for (const candidate of imported.candidates) {
        const incoming = { ...copy(candidate), id: strategy === "CREATE_NEW" ? this.allocate() : candidate.id };
        this.reserve(incoming.id);
        const existing = space.candidates.find(
          /** Resolves a candidate's actual persistent identity. */
          (record2) => record2.id === incoming.id
        );
        if (existing === void 0) space.candidates.push(incoming);
        else if (strategy === "UPDATE") Object.assign(existing, incoming);
      }
      return result2;
    });
  }
};

// src/storage/files.ts
function completed(result2) {
  if (!result2.successful) throw new Error(result2.details);
}
function dataDirectory() {
  const root = ToolPkg.getConfigDir();
  if (typeof root !== "string" || root.trim() === "") throw new Error("Plugin config directory is unavailable");
  return root.replace(/\\/g, "/").replace(/\/$/, "") + "/character-memory";
}
var FileCharacterRepository = class _FileCharacterRepository {
  /** Retains only an authoritative successfully opened or published snapshot. */
  constructor(directory, state, directories) {
    this.directory = directory;
    this.state = state;
    this.directories = directories;
    this.busy = false;
    this.waiters = [];
    this.publicationFailure = null;
  }
  /** Initializes a new product installation; existing broken state is never reconstructed. */
  static open(directories = null) {
    return __async(this, null, function* () {
      const directory = dataDirectory(), info = yield Tools.Files.exists(directory);
      if (info.exists) {
        if (!info.isDirectory) throw new Error("Plugin data path is not a directory: " + directory);
        const content = yield Tools.Files.read(directory + "/state.json"), state2 = decodeCharacterState(content.content);
        const repository2 = new _FileCharacterRepository(directory, state2, directories);
        const pending = yield Tools.Files.exists(directory + "/state.next.json");
        if (pending.exists) throw new Error("Unfinished plugin snapshot publication: " + directory + "/state.next.json");
        yield repository2.verifyDocuments(state2);
        return repository2;
      }
      completed(yield Tools.Files.mkdir(directory, true));
      const state = createInitialState(Date.now());
      assertCharacterState(state);
      const repository = new _FileCharacterRepository(directory, state, directories);
      const documents = state.owners.map(
        /** Initializes each genuine first-install owner document exactly once. */
        (owner2) => ({ ownerKey: owner2.ownerKey, path: directory + "/" + owner2.userDocumentPath, content: "", newDocument: true })
      );
      yield repository.publish(state, documents, []);
      return repository;
    });
  }
  /** Runs commands, APIs and UI operations in one private snapshot with one publication phase. */
  run(action) {
    return __async(this, null, function* () {
      yield this.acquire();
      try {
        if (this.publicationFailure !== null) throw this.publicationFailure.error;
        yield this.verifyDocuments(this.state);
        const state = copyState(this.state), session = new RepositorySession(state, this.directory, this.directories);
        const result2 = yield action(session);
        assertCharacterState(state);
        const documents = session.documentWrites();
        const deletedDocuments = this.state.owners.filter(
          /** Identifies removed owners without guessing names or touching other plugin paths. */
          (owner2) => !state.owners.some(
            /** Retains exact owner identities that still exist. */
            (retained2) => retained2.ownerKey === owner2.ownerKey
          )
        ).map(
          /** Selects only this plugin's validated owner USER.md file. */
          (owner2) => this.directory + "/" + owner2.userDocumentPath
        );
        if (JSON.stringify(state) !== JSON.stringify(this.state) || documents.length !== 0) {
          yield this.publish(state, documents, deletedDocuments);
          this.state = state;
        }
        return result2;
      } finally {
        this.release();
      }
    });
  }
  /** Requires every existing referenced USER.md file and rejects interrupted document publications. */
  verifyDocuments(state) {
    return __async(this, null, function* () {
      for (const owner2 of state.owners) {
        const path = this.directory + "/" + owner2.userDocumentPath;
        const document = yield Tools.Files.read(path);
        assertString(document.content, "USER.md content");
        const pending = yield Tools.Files.exists(path + ".next");
        if (pending.exists) throw new Error("Unfinished owner USER.md publication: " + path + ".next");
      }
    });
  }
  /** Acquires the sole publisher without evaluating queued domain work beforehand. */
  acquire() {
    if (!this.busy) {
      this.busy = true;
      return Promise.resolve();
    }
    return new Promise(
      /** Stores the next exact lock grant. */
      (resolve) => {
        this.waiters.push(resolve);
      }
    );
  }
  /** Releases the next caller after its predecessor's original success or failure. */
  release() {
    const waiter = this.waiters.shift();
    if (waiter === void 0) this.busy = false;
    else waiter();
  }
  /** Verifies staging bytes before invoking the existing Files move operation. */
  stage(path, content) {
    return __async(this, null, function* () {
      completed(yield Tools.Files.write(path, content, false));
      const result2 = yield Tools.Files.read(path);
      if (result2.content !== content) throw new Error("Plugin staging verification failed: " + path);
    });
  }
  /** Publishes and verifies one real file without promising unsupported host atomicity. */
  move(source, target, content) {
    return __async(this, null, function* () {
      completed(yield Tools.Files.move(source, target));
      const result2 = yield Tools.Files.read(target);
      if (result2.content !== content) throw new Error("Plugin file publication verification failed: " + target);
    });
  }
  /** Stages all domain/document writes before publication and stops permanently on a publication error. */
  publish(state, documents, deletedDocuments) {
    return __async(this, null, function* () {
      const content = JSON.stringify(state, null, 2) + "\n", pending = this.directory + "/state.next.json", target = this.directory + "/state.json";
      try {
        yield this.stage(pending, content);
        for (const document of documents) {
          if (document.newDocument) {
            const existing = yield Tools.Files.exists(document.path);
            if (existing.exists) throw new Error("New owner document path is already occupied: " + document.path);
            completed(yield Tools.Files.mkdir(document.path.slice(0, document.path.lastIndexOf("/")), true));
          } else {
            const existing = yield Tools.Files.read(document.path);
            assertString(existing.content, "existing USER.md");
          }
          yield this.stage(document.path + ".next", document.content);
        }
        for (const document of documents) yield this.move(document.path + ".next", document.path, document.content);
        for (const path of deletedDocuments) completed(yield Tools.Files.deleteFile(path, false));
        yield this.move(pending, target, content);
      } catch (error) {
        this.publicationFailure = { error };
        throw error;
      }
    });
  }
};

// src/service-runtime.ts
function createServiceRuntime(initialize) {
  let initialization = null;
  return {
    /** Captures the initialization Promise before invoking the supplied business initializer. */
    getService() {
      if (initialization === null) initialization = Promise.resolve().then(initialize);
      return initialization;
    }
  };
}
var directorySources = null;
var opening = false;
function connectDirectorySources(sources) {
  if (opening || directorySources !== null) throw new Error("External directory readers must be connected exactly once before service initialization");
  directorySources = sources;
}
var runtime = createServiceRuntime(
  /** Opens only this plugin's real directory on first use, never during registration. */
  () => __async(null, null, function* () {
    opening = true;
    return createCharacterCardsService(yield FileCharacterRepository.open(directorySources));
  })
);
function initializeService() {
  return __async(this, null, function* () {
    yield runtime.getService();
  });
}
function getService() {
  return runtime.getService();
}
function dispatch2(request) {
  return __async(this, null, function* () {
    return (yield getService()).dispatch(request);
  });
}
function dispatchDomain2(operation, input) {
  return __async(this, null, function* () {
    return (yield getService()).dispatchDomain(operation, input);
  });
}
function runMemoryJobs() {
  return __async(this, null, function* () {
    return (yield getService()).runMemoryJobs();
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
function parseDomainMessage(value) {
  const envelope = record(value, "character-memory.domain"), keys = Object.keys(envelope);
  if (keys.length !== 2 || !keys.every(
    /** Rejects protocol spellings other than the established operation/input fields. */
    (key) => key === "operation" || key === "input"
  )) throw new Error("character-memory.domain requires exactly operation and input");
  if (typeof envelope.operation !== "string" || envelope.operation.trim() === "") throw new Error("character-memory.domain.operation must be nonblank");
  const operation = envelope.operation;
  return { operation, input: parseDomainPayload(operation, envelope.input) };
}
function callMainDomain(operation, input) {
  const checked = parseDomainPayload(operation, input);
  return ToolPkg.ipc.call("character-memory.domain", { operation, input: checked }, { targetRuntime: "main" });
}
function chatIdentity(value, label) {
  if (typeof value !== "string" || value.trim() === "") throw new Error(label + " must be a nonblank string");
  return value;
}
function fields2(value, expected, label) {
  const object3 = record(value, label), keys = Object.keys(object3);
  if (keys.length !== expected.length || keys.some(
    /** Requires each key to be one of the exact documented field names. */
    (key) => expected.indexOf(key) < 0
  )) throw new Error(label + " has invalid fields");
  return object3;
}
function nullableString(value, label) {
  if (value === null || typeof value === "string") return value;
  throw new Error(label + " must be a string or null");
}
function parseChatRequest(value) {
  const input = fields2(value, ["chatId"], "chat UI request");
  return { chatId: input.chatId === null ? null : chatIdentity(input.chatId, "chatId") };
}
function parseSectionsRequest(value) {
  const input = fields2(value, ["chats"], "chat.list.sections");
  if (!Array.isArray(input.chats)) throw new Error("chat.list.sections.chats must be an array");
  const chats = [], ids = /* @__PURE__ */ new Set();
  for (const raw of input.chats) {
    const chat = fields2(raw, ["id", "title", "updatedAt", "displayOrder", "workspaceId", "workspaceName", "locked", "pinned"], "chat summary");
    const id2 = chatIdentity(chat.id, "chat.id");
    if (ids.has(id2)) throw new Error("Duplicate chat summary: " + id2);
    ids.add(id2);
    if (typeof chat.title !== "string" || typeof chat.updatedAt !== "string") throw new Error("Chat summary title and updatedAt must be strings");
    for (const key of ["workspaceId", "workspaceName"]) if (chat[key] !== null && typeof chat[key] !== "string") throw new Error("Chat summary " + key + " must be a string or null");
    if (typeof chat.displayOrder !== "number" || !Number.isSafeInteger(chat.displayOrder)) throw new Error("Chat summary displayOrder must be an integer");
    if (typeof chat.locked !== "boolean" || typeof chat.pinned !== "boolean") throw new Error("Chat summary locked and pinned must be booleans");
    const workspaceId = nullableString(chat.workspaceId, "chat.workspaceId");
    const workspaceName = nullableString(chat.workspaceName, "chat.workspaceName");
    chats.push({ id: id2, title: chat.title, updatedAt: chat.updatedAt, displayOrder: chat.displayOrder, workspaceId, workspaceName, locked: chat.locked, pinned: chat.pinned });
  }
  return { chats };
}
function requireUiCaller(caller) {
  if (caller !== "host") throw new Error("Chat UI contributions require an authenticated host caller");
}
function decodeSelection(selection2, directory) {
  const parsed = parseChatSelection(selection2), id2 = parsed.id;
  switch (parsed.kind) {
    case "card": {
      const cards = directory.cards.filter(
        /** Requires one exact persisted identity, not a replacement active actor. */
        (card2) => card2.id === id2
      );
      if (cards.length !== 1) throw new Error("Selection does not identify exactly one character: " + id2);
      const card = cards[0];
      return { entity: "card", id: id2, prompt: { CharacterCard: { id: id2 } }, title: card.name, avatarUri: card.avatarUri };
    }
    case "group": {
      const groups = directory.groups.filter(
        /** Requires one exact persisted group identity. */
        (group) => group.id === id2
      );
      if (groups.length !== 1) throw new Error("Selection does not identify exactly one group: " + id2);
      return { entity: "group", id: id2, prompt: { CharacterGroup: { id: id2 } }, title: groups[0].name, avatarUri: null };
    }
    default:
      throw new Error("Invalid plugin selection entity");
  }
}
function encodeSelection(selection2) {
  return selectionForActive(selection2);
}
function contextActions(routes, target, directory) {
  const selected = target.selection === null ? null : decodeSelection(target.selection, directory);
  const routeId = routes.selection;
  const input = { mode: "select", chatId: target.chatId };
  return {
    selectors: [
      { id: "characters", title: "\u5207\u6362\u89D2\u8272\u5361", icon: "Badge", routeId, input: { ...input, kind: "card", selected: selected !== null && selected.entity === "card" ? selected.prompt : null } },
      { id: "groups", title: "\u5207\u6362\u7FA4\u7EC4", icon: "Groups", routeId, input: { ...input, kind: "group", selected: selected !== null && selected.entity === "group" ? selected.prompt : null } },
      ...target.chatId !== null && selected !== null && selected.entity === "group" ? [{ id: "group-execution", title: "\u7FA4\u7EC4\u6267\u884C", icon: "Groups", routeId: routes.execution, input: { mode: "group-execution", chatId: target.chatId } }] : []
    ],
    identity: selected === null ? null : { title: selected.title, avatarUri: selected.avatarUri, action: previewAction(routes.editor, selected.entity, selected.id) },
    backgroundUri: null
  };
}
function previewAction(routeId, entity2, id2) {
  return { routeId, input: { mode: "preview", entity: entity2, id: id2 } };
}
function listSections(routeId, directory, bindings, chatIds) {
  const byChat = /* @__PURE__ */ new Map();
  for (const binding of bindings) {
    if (byChat.has(binding.chatId)) throw new Error("Duplicate chat binding: " + binding.chatId);
    decodeSelection(binding.selection, directory);
    byChat.set(binding.chatId, binding.selection);
  }
  const sections = [];
  for (const card of directory.cards) {
    const selection2 = "card:" + card.id;
    sections.push({ id: selection2, title: card.name, avatarUri: card.avatarUri, selection: selection2, preview: previewAction(routeId, "card", card.id), chatIds: boundChats(selection2) });
  }
  for (const group of directory.groups) {
    const selection2 = "group:" + group.id;
    sections.push({ id: selection2, title: group.name, avatarUri: null, selection: selection2, preview: previewAction(routeId, "group", group.id), chatIds: boundChats(selection2) });
  }
  function boundChats(selection2) {
    const result2 = [];
    for (const chatId of chatIds) if (byChat.get(chatId) === selection2) result2.push(chatId);
    return result2;
  }
  return { sections };
}
function chatContextActionsApi(event) {
  return __async(this, null, function* () {
    requireUiCaller(event.callerPackage);
    const input = parseChatRequest(event.payload), service = yield getService();
    const directory = yield service.snapshot();
    if (input.chatId === null) {
      const active = yield service.dispatchDomain("activePrompt.get", {});
      return contextActions(chatUiRoutes, { chatId: null, selection: encodeSelection(active) }, directory);
    }
    const extension = yield Tools.Chat.readExtension({ kind: "chat", chatId: input.chatId });
    const selection2 = extension === null ? null : decodeChatMarker(extension).selection;
    return contextActions(chatUiRoutes, { chatId: input.chatId, selection: selection2 }, directory);
  });
}
function chatListSectionsApi(event) {
  return __async(this, null, function* () {
    requireUiCaller(event.callerPackage);
    const input = parseSectionsRequest(event.payload), service = yield getService(), directory = yield service.snapshot();
    const bindings = [];
    for (const chat of input.chats) {
      const extension = yield Tools.Chat.readExtension({ kind: "chat", chatId: chat.id });
      if (extension !== null) bindings.push({ chatId: chat.id, selection: decodeChatMarker(extension).selection });
    }
    return listSections(chatUiRoutes.editor, directory, bindings, input.chats.map(
      /** Preserves the host's actual chat ordering in each section. */
      (chat) => chat.id
    ));
  });
}
function registerUiContributionApis() {
  ToolPkg.registerApi({ name: "chat.context.actions", function: chatContextActionsApi });
  ToolPkg.registerApi({ name: "chat.list.sections", function: chatListSectionsApi });
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
        const result2 = parseScreenResult(value, current.input);
        completing = true;
        try {
          let completion;
          switch (result2.mode) {
            case "select": {
              if (selectorTarget === null) throw new Error("Selector has no explicit global or chat target");
              const selection2 = encodeSelection(result2.selection);
              if (selectorTarget.chatId === null) {
                const identity4 = parseChatSelection(selection2);
                const active = identity4.kind === "card" ? yield callMainDomain("activePrompt.setCard", { id: identity4.id }) : yield callMainDomain("activePrompt.setGroup", { id: identity4.id });
                if (!active.active || active.id !== identity4.id || active.type !== (identity4.kind === "card" ? "character_card" : "character_group")) throw new Error("Main runtime did not confirm the selected global active prompt");
              } else {
                const chatId = selectorTarget.chatId;
                const binding = yield callMainDomain("chat.configuration.binding.write", { chatId, selection: selection2 });
                if (binding.chatId !== chatId || binding.selection !== selection2) throw new Error("Main runtime did not confirm the selected chat binding");
              }
              const complete = session.completeScreen(result2);
              completion = { ...complete, value: { selection: selection2, contextKey: selection2 } };
              break;
            }
            case "memory-attachment": {
              const complete = session.completeScreen(result2);
              completion = { ...complete, value: { type: "text", name: "\u8BB0\u5FC6\u9644\u4EF6", content: result2.content, mediaType: "text/plain" } };
              break;
            }
            case "preview":
            case "edit":
              completion = session.completeScreen(result2);
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

// src/host.ts
function register(definition2, screen2, attachmentScreen2, sidebarScreen2, selectionScreen2, groupExecutionScreen2) {
  const route = `toolpkg:${definition2.id}:ui:main`;
  ToolPkg.registerUiRoute({ id: "main", route, screen: screen2, runtime: "compose_dsl", keepAlive: true, title: { zh: definition2.title, en: "Characters" } });
  const attachmentRoute = `toolpkg:${definition2.id}:ui:memory-attachment`;
  ToolPkg.registerUiRoute({ id: "memory-attachment", route: attachmentRoute, screen: attachmentScreen2, runtime: "compose_dsl", keepAlive: false, title: { zh: "\u8BB0\u5FC6\u9644\u4EF6", en: "Memory attachment" } });
  const sidebarRoute = `toolpkg:${definition2.id}:ui:chat-sidebar`;
  ToolPkg.registerUiRoute({ id: "chat-sidebar", route: sidebarRoute, screen: sidebarScreen2, runtime: "compose_dsl", keepAlive: true, title: { zh: "\u4F1A\u8BDD\u4FA7\u8FB9\u680F", en: "Chat sidebar" } });
  const selectionRoute = `toolpkg:${definition2.id}:ui:selection`;
  ToolPkg.registerUiRoute({ id: "selection", route: selectionRoute, screen: selectionScreen2, runtime: "compose_dsl", keepAlive: false, title: { zh: "\u5207\u6362\u89D2\u8272\u5361", en: "Switch character" } });
  ToolPkg.registerUiRoute({ id: "group-execution", route: `toolpkg:${definition2.id}:ui:group-execution`, screen: groupExecutionScreen2, runtime: "compose_dsl", keepAlive: false, title: { zh: "\u7FA4\u7EC4\u6267\u884C", en: "Group execution" } });
  ToolPkg.registerNavigationEntry({ id: "sidebar-characters", route: sidebarRoute, surface: "chat_sidebar_tabs", title: { zh: "\u89D2\u8272\u5206\u7C7B", en: "Characters" }, icon: "Badge", order: definition2.order, params: { view: "characters" } });
  ToolPkg.registerNavigationEntry({ id: "sidebar-groups", route: sidebarRoute, surface: "chat_sidebar_tabs", title: { zh: "\u4F1A\u8BDD\u7FA4\u7EC4", en: "Conversation groups" }, icon: "Groups", order: definition2.order + 1, params: { view: "groups" } });
  ToolPkg.registerNavigationEntry({ id: "sidebar", route, surface: "main_sidebar_plugins", title: { zh: definition2.title, en: "Characters" }, icon: definition2.icon, order: definition2.order });
  ToolPkg.registerNavigationEntry({ id: "toolbox", route, surface: "toolbox", title: { zh: definition2.title, en: "Characters" }, icon: definition2.icon, order: definition2.order });
  ToolPkg.registerNavigationEntry({ id: "memory-attachment", route: attachmentRoute, surface: "chat_attachments", title: { zh: "\u8BB0\u5FC6\u9644\u4EF6", en: "Memory attachment" }, icon: "Memory", order: definition2.order, params: { mode: "memory-attachment", ownerKey: null, folderPath: null } });
  return true;
}
function receiveUiRequest(value) {
  return __async(this, null, function* () {
    const request = record(value, "character-memory.request");
    if (typeof request.action !== "string" || request.action.trim() === "") throw new Error("character-memory.request.action must be a nonblank string");
    return dispatch2(request);
  });
}
function webArguments(value, expected) {
  if (value.length !== 1 || !Array.isArray(value[0]) || value[0].length !== expected) throw new Error(`CharacterMemoryHost expects ${expected} arguments`);
  return value[0];
}
function receiveDomainRequest(value) {
  return __async(this, null, function* () {
    const message = parseDomainMessage(value);
    return dispatchDomain2(message.operation, message.input);
  });
}
function registerUiRequestChannel() {
  ToolPkg.ipc.on("character-memory.request", receiveUiRequest);
  ToolPkg.ipc.on("character-memory.domain", receiveDomainRequest);
}
function registerServiceLifecycle() {
  ToolPkg.registerAppLifecycleHook({ id: "service-initialize", event: "application_on_create", function: onServiceInitialize });
}
function onServiceInitialize(_event) {
  return __async(this, null, function* () {
    yield initializeService();
  });
}
function renderScreen(ctx, definition2, requiredMode = null) {
  const controller2 = ctx.createWebViewController("character-memory-web");
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
      if (ready.current) yield controller2.evaluateJavascript(`window.applyCharacterMemoryTheme(${JSON.stringify(theme)});`);
    });
  }
  function initialize() {
    return __async(this, null, function* () {
      try {
        controller2.addJavascriptInterface("CharacterMemoryHost", {
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
            return screenSession.currentScreen();
          },
          /** Returns an explicit completion action result for this exact presented screen. */
          completeScreen: (...args) => {
            const [result2] = webArguments(args, 1);
            const current = screenSession.currentScreen();
            if (current.input.mode === "manage") throw new Error("Management has no presentation completion channel");
            return screenSession.completeScreen(parseScreenResult(result2, current.input));
          },
          /** Returns an explicit cancellation action result without changing the selected identity. */
          cancelScreen: (...args) => {
            webArguments(args, 0);
            return screenSession.cancelScreen();
          },
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
      } catch (failure2) {
        setError(String(failure2));
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
  return ctx.UI.Box({ fillMaxSize: true, onLoad: initialize }, path === "" ? ctx.UI.Text({ text: error === "" ? `\u6B63\u5728\u52A0\u8F7D${definition2.title}\u2026` : error }) : ctx.UI.WebView({
    key: "character-memory-web",
    controller: controller2,
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

// src/command-spec.ts
var CHARACTER_USAGE = [
  "operit2 character list",
  "operit2 character show <id>",
  "operit2 character create <name> [character-setting]",
  "operit2 character create --record <character-json>",
  "operit2 character update <id> <field> <value>",
  "operit2 character update <id> --record <character-json>",
  "operit2 character update <id> --patch <changes-json>",
  "operit2 character delete <id>",
  "operit2 character set-active <id>",
  "operit2 character combine <id> [CHAT|VOICE] [tag-id-csv]",
  "operit2 character reset-default",
  "operit2 character export <id> <operit|tavern>",
  "operit2 character import <operit|tavern> <json-content>",
  "operit2 character export-backup",
  "operit2 character import-backup <json-content>"
];
var GROUP_USAGE = [
  "operit2 group list",
  "operit2 group show <id>",
  "operit2 group create <name> [description]",
  "operit2 group create --record <group-json>",
  "operit2 group update <id> <field> <value>",
  "operit2 group update <id> --record <group-json>",
  "operit2 group update <id> --patch <changes-json>",
  "operit2 group delete <id>",
  "operit2 group set-active <id>",
  "operit2 group duplicate <source-id> [new-name]",
  "operit2 group export <id>",
  "operit2 group import <json-content>",
  "operit2 group export-backup",
  "operit2 group import-backup <json-content>"
];
var TAG_USAGE = [
  "operit2 tag list",
  "operit2 tag show <id>",
  "operit2 tag create <name> [prompt-content] [description] [tag-type]",
  "operit2 tag create --record <tag-json>",
  "operit2 tag update <id> <field> <value>",
  "operit2 tag update <id> --record <tag-json>",
  "operit2 tag update <id> --patch <changes-json>",
  "operit2 tag delete <id>"
];
var ACTIVE_PROMPT_USAGE = [
  "operit2 active-prompt show",
  "operit2 active-prompt set-card <id>",
  "operit2 active-prompt set-group <id>",
  "operit2 active-prompt activate-for-chat [character-card-name] [character-group-id]",
  "operit2 active-prompt resolved-card"
];
var MEMORY_USAGE = [
  "operit2 memory <character|shared|mount|unmount>",
  "operit2 memory resolve <character-id>"
];
var CHARACTER_MEMORY_USAGE = [
  "operit2 memory character <character-id> user <show|write|path>",
  "operit2 memory character <character-id> item <list|search|show|create|update|delete|move>",
  "operit2 memory character <character-id> graph",
  "operit2 memory character <character-id> link <create|update|delete>",
  "operit2 memory character <character-id> settings <show|write>",
  "operit2 memory character <character-id> search-config <show|write>",
  "operit2 memory character <character-id> export",
  "operit2 memory character <character-id> import <SKIP|UPDATE|CREATE_NEW> <json-content>"
];
var SHARED_MEMORY_USAGE = [
  "operit2 memory shared <list|create|rename|delete>",
  "operit2 memory shared <shared-id> user <show|write|path>",
  "operit2 memory shared <shared-id> item <list|search|show|create|update|delete|move>",
  "operit2 memory shared <shared-id> graph",
  "operit2 memory shared <shared-id> link <create|update|delete>",
  "operit2 memory shared <shared-id> settings <show|write>",
  "operit2 memory shared <shared-id> search-config <show|write>",
  "operit2 memory shared <shared-id> export",
  "operit2 memory shared <shared-id> import <SKIP|UPDATE|CREATE_NEW> <json-content>"
];
var USER_USAGE = [
  "operit2 memory <owner> user show",
  "operit2 memory <owner> user write <content>",
  "operit2 memory <owner> user path"
];
var ITEM_USAGE = [
  "operit2 memory <owner> item list",
  "operit2 memory <owner> item search <query>",
  "operit2 memory <owner> item show <title>",
  "operit2 memory <owner> item create <title> <content> [folder] [tags-csv]",
  "operit2 memory <owner> item create --record <memory-json>",
  "operit2 memory <owner> item update <title> <field> <value>",
  "operit2 memory <owner> item update <title> --patch <changes-json>",
  "operit2 memory <owner> item update <title> --record <memory-json>",
  "operit2 memory <owner> item delete <id>",
  "operit2 memory <owner> item move <ids-csv> <folder>"
];
var LINK_USAGE = [
  "operit2 memory <owner> link create <source-title> <target-title> <type> <weight> <description>",
  "operit2 memory <owner> link update <link-id> <field> <value>",
  "operit2 memory <owner> link update <link-id> --patch <changes-json>",
  "operit2 memory <owner> link delete <link-id>"
];
var MOUNT_USAGE = ["operit2 memory mount <character-id> <shared-id> --read <true|false> --write <true|false>"];
var UNMOUNT_USAGE = ["operit2 memory unmount <character-id> <shared-id>"];

// src/commands.ts
function result(stdout, json2) {
  return { stdout, json: jsonValue(json2) };
}
function lines(values) {
  return `${values.join("\n")}
`;
}
function invocation(operation, payload, render) {
  const checked = parseDomainPayload(operation, payload);
  return {
    kind: "domain",
    operation,
    payload: checked,
    /** Executes the shared business service once after all arguments have been validated. */
    execute() {
      return __async(this, null, function* () {
        return render(yield dispatchDomain2(operation, checked));
      });
    }
  };
}
function wantsHelp(args) {
  return args.length === 0 || args.length === 1 && (/* @__PURE__ */ new Set(["help", "--help", "-h"])).has(args[0]);
}
function arity(args, min, max, usage) {
  if (args.length < min || args.length > max) throw new Error(`usage: ${usage}`);
}
function csv(raw) {
  const values = /* @__PURE__ */ new Set();
  for (const part of raw.split(",")) {
    const value = part.trim();
    if (value !== "") values.add(value);
  }
  return [...values];
}
function optionalIdentifier(raw) {
  const value = raw.trim();
  return value === "" ? null : value;
}
function bool(raw, field) {
  if (raw === "true") return true;
  if (raw === "false") return false;
  throw new Error(`invalid bool for ${field}: ${raw}; expected true | false`);
}
function integer2(raw, field) {
  if (!/^[+-]?\d+$/.test(raw)) throw new Error(`${field} must be an integer: ${raw}`);
  const value = Number(raw);
  if (!Number.isSafeInteger(value)) throw new Error(`${field} must be a safe integer: ${raw}`);
  return value;
}
function number2(raw, field) {
  if (raw.trim() === "") throw new Error(`${field} must be a finite number`);
  const value = Number(raw);
  if (!Number.isFinite(value)) throw new Error(`${field} must be a finite number: ${raw}`);
  return value;
}
function json(raw, field) {
  try {
    return JSON.parse(raw);
  } catch (error) {
    throw new Error(`${field}: invalid JSON: ${error instanceof Error ? error.message : String(error)}`);
  }
}
function characterField(field, raw) {
  let value;
  switch (field) {
    case "name":
    case "description":
    case "characterSetting":
    case "openingStatement":
    case "otherContentChat":
    case "otherContentVoice":
    case "advancedCustomPrompt":
    case "marks":
    case "chatModelBindingMode":
    case "memoryBindingMode":
      value = raw;
      break;
    case "avatarUri":
    case "chatModelId":
    case "ttsConfigId":
    case "sharedMemoryId":
      value = optionalIdentifier(raw);
      break;
    case "attachedTagIds":
      value = csv(raw);
      break;
    case "sharedMemoryMounts":
    case "toolAccessConfig":
      value = json(raw, field);
      break;
    case "isDefault":
      value = bool(raw, field);
      break;
    case "createdAt":
    case "updatedAt":
      value = integer2(raw, field);
      break;
    default:
      throw new Error(`unsupported character field: ${field}`);
  }
  return parseCharacterChanges({ [field]: value });
}
function groupField(field, raw) {
  switch (field) {
    case "name":
    case "description":
      return parseGroupChanges({ [field]: raw });
    case "createdAt":
    case "updatedAt":
      return parseGroupChanges({ [field]: integer2(raw, field) });
    case "members": {
      const members2 = [];
      const values = raw.split(",");
      for (let index = 0; index < values.length; index += 1) {
        const characterCardId = values[index].trim();
        if (characterCardId !== "") members2.push({ characterCardId, orderIndex: index });
      }
      return parseGroupChanges({ members: members2 });
    }
    default:
      throw new Error(`unsupported group field: ${field}`);
  }
}
function tagField(field, raw) {
  switch (field) {
    case "name":
    case "description":
    case "promptContent":
    case "tagType":
      return parseTagChanges({ [field]: raw });
    case "createdAt":
    case "updatedAt":
      return parseTagChanges({ [field]: integer2(raw, field) });
    default:
      throw new Error(`unsupported tag field: ${field}`);
  }
}
function changes(args, family, decodeField) {
  arity(args, 4, 4, `operit2 ${family} update <id> <field|--record|--patch> <value>`);
  const field = args[2];
  if (field === "--patch") return { field: "record", changes: parseJsonObject(args[3], `${family} changes`) };
  if (field === "--record") {
    const complete = parseJsonObject(args[3], `${family} record`);
    if (Object.prototype.hasOwnProperty.call(complete, "id") && complete.id !== args[1]) throw new Error(`${family} record id does not match ${args[1]}`);
    const { id: identity4, ...mutable } = complete;
    void identity4;
    return { field: "record", changes: mutable };
  }
  return { field, changes: record(decodeField(field, args[3]), `${family} changes`) };
}
function displayOptional(value) {
  return value === null ? "-" : value;
}
function members(group) {
  return group.members.map((member2) => `${member2.characterCardId}:${member2.orderIndex}`).join(", ");
}
function characterText(card) {
  return lines([
    `Character ${card.id}`,
    `Name: ${card.name}`,
    `Description: ${card.description}`,
    `Character setting: ${card.characterSetting}`,
    `Opening statement: ${card.openingStatement}`,
    `Chat content: ${card.otherContentChat}`,
    `Voice content: ${card.otherContentVoice}`,
    `Tags: ${card.attachedTagIds.join(", ")}`,
    `Advanced prompt: ${card.advancedCustomPrompt}`,
    `Marks: ${card.marks}`,
    `Chat model binding: ${card.chatModelBindingMode}`,
    `Chat model id: ${displayOptional(card.chatModelId)}`,
    `Shared memory mounts: ${card.sharedMemoryMounts.length}`,
    `Tool access enabled: ${card.toolAccessConfig.enabled}`,
    `Default: ${card.isDefault}`,
    `Created at: ${card.createdAt}`,
    `Updated at: ${card.updatedAt}`
  ]);
}
function groupText(group) {
  return lines([`Character group ${group.id}`, `Name: ${group.name}`, `Description: ${group.description}`, `Members: ${members(group)}`, `Created at: ${group.createdAt}`, `Updated at: ${group.updatedAt}`]);
}
function tagText(tag) {
  return lines([`Prompt tag: ${tag.name}`, `ID: ${tag.id}`, `Type: ${tag.tagType}`, `Description: ${tag.description}`, `Prompt content: ${tag.promptContent}`, `Created at: ${tag.createdAt}`, `Updated at: ${tag.updatedAt}`]);
}
function parseCharacterCommand(args) {
  if (wantsHelp(args)) return { kind: "usage", usage: CHARACTER_USAGE };
  switch (args[0]) {
    case "list": {
      arity(args, 1, 1, CHARACTER_USAGE[0]);
      return invocation("character.list", {}, (cards) => result(lines([`Characters: ${cards.length}`, ...cards.map(
        /** Formats one stored character's legacy summary. */
        (card) => `- ${card.id} | ${card.name} | default: ${card.isDefault} | tags: ${card.attachedTagIds.join(", ")} | ${card.description}`
      )]), cards));
    }
    case "show": {
      arity(args, 2, 2, CHARACTER_USAGE[1]);
      return invocation("character.get", { id: args[1] }, (card) => result(characterText(card), card));
    }
    case "create": {
      let values;
      if (args[1] === "--record") {
        arity(args, 3, 3, CHARACTER_USAGE[3]);
        values = parseDomainPayload("character.create", { values: parseJsonObject(args[2], "character record") }).values;
      } else {
        arity(args, 2, 3, CHARACTER_USAGE[2]);
        values = { name: args[1] };
        if (args.length === 3) values.characterSetting = args[2];
      }
      return invocation("character.create", { values }, (card) => result(lines([`Created character ${card.id}`]), { id: card.id, name: card.name, characterSetting: card.characterSetting }));
    }
    case "update": {
      const patch = changes(args, "character", characterField);
      const payload = parseDomainPayload("character.update", { id: args[1], changes: patch.changes });
      return invocation("character.update", payload, (card) => result(lines([`Updated character ${card.id}`, `Field: ${patch.field}`]), { id: card.id, field: patch.field, character: card }));
    }
    case "delete": {
      arity(args, 2, 2, CHARACTER_USAGE[7]);
      return invocation("character.delete", { id: args[1] }, (value) => result(lines([`Deleted character ${value.id}`]), value));
    }
    case "set-active": {
      arity(args, 2, 2, CHARACTER_USAGE[8]);
      return invocation("character.setActive", { id: args[1] }, (value) => result(lines([`Active character: ${value.id}`]), value));
    }
    case "combine": {
      arity(args, 2, 4, CHARACTER_USAGE[9]);
      const payload = { id: args[1] };
      if (args.length >= 3) payload.promptFunctionType = args[2];
      if (args.length === 4) payload.additionalTagIds = csv(args[3]);
      return invocation("character.combine", parseDomainPayload("character.combine", payload), (value) => result(value.prompt, value));
    }
    case "reset-default": {
      arity(args, 1, 1, CHARACTER_USAGE[10]);
      return invocation("character.resetDefault", {}, (value) => result(lines(["Default character reset"]), value));
    }
    case "export": {
      arity(args, 3, 3, CHARACTER_USAGE[11]);
      const payload = parseDomainPayload("character.export", { id: args[1], format: args[2] });
      return invocation("character.export", payload, (value) => result(value.content, value));
    }
    case "import": {
      arity(args, 3, 3, CHARACTER_USAGE[12]);
      const payload = parseDomainPayload("character.import", { format: args[1], content: args[2] });
      return invocation("character.import", payload, (value) => result(characterText(value), value));
    }
    case "export-backup": {
      arity(args, 1, 1, CHARACTER_USAGE[13]);
      return invocation("character.exportBackup", {}, (value) => result(value.content, value));
    }
    case "import-backup": {
      arity(args, 2, 2, CHARACTER_USAGE[14]);
      return invocation("character.importBackup", { content: args[1] }, (value) => result(lines([`Imported characters: ${value.new}; updated: ${value.updated}; skipped: ${value.skipped}; total: ${value.total}`]), value));
    }
    default:
      throw new Error(`unknown character action: ${args[0]}; use character help`);
  }
}
function parseGroupCommand(args) {
  if (wantsHelp(args)) return { kind: "usage", usage: GROUP_USAGE };
  switch (args[0]) {
    case "list": {
      arity(args, 1, 1, GROUP_USAGE[0]);
      return invocation("group.list", {}, (groups) => result(lines([`Character groups: ${groups.length}`, ...groups.map(
        /** Formats one stored group's original summary. */
        (group) => `- ${group.id} | ${group.name} | members: ${members(group)} | ${group.description}`
      )]), groups));
    }
    case "show": {
      arity(args, 2, 2, GROUP_USAGE[1]);
      return invocation("group.get", { id: args[1] }, (group) => result(groupText(group), group));
    }
    case "create": {
      let values;
      if (args[1] === "--record") {
        arity(args, 3, 3, GROUP_USAGE[3]);
        values = parseDomainPayload("group.create", { values: parseJsonObject(args[2], "group record") }).values;
      } else {
        arity(args, 2, 3, GROUP_USAGE[2]);
        values = { name: args[1] };
        if (args.length === 3) values.description = args[2];
      }
      return invocation("group.create", { values }, (group) => result(lines([`Created character group ${group.id}`]), { id: group.id, name: group.name, description: group.description }));
    }
    case "update": {
      const patch = changes(args, "group", groupField);
      const payload = parseDomainPayload("group.update", { id: args[1], changes: patch.changes });
      return invocation("group.update", payload, (group) => result(lines([`Updated character group ${group.id}`, `Field: ${patch.field}`]), { id: group.id, field: patch.field, group }));
    }
    case "delete": {
      arity(args, 2, 2, GROUP_USAGE[7]);
      return invocation("group.delete", { id: args[1] }, (value) => result(lines([`Deleted character group ${value.id}`]), value));
    }
    case "set-active": {
      arity(args, 2, 2, GROUP_USAGE[8]);
      return invocation("group.setActive", { id: args[1] }, (value) => result(lines([`Active character group: ${value.id}`]), value));
    }
    case "duplicate": {
      arity(args, 2, 3, GROUP_USAGE[9]);
      const payload = { id: args[1] };
      const newName = args.length === 3 ? args[2] : null;
      if (newName !== null) payload.newName = newName;
      return invocation("group.duplicate", payload, (group) => result(lines([`Duplicated character group ${payload.id} -> ${group.id}`]), { sourceId: payload.id, newId: group.id, newName }));
    }
    case "export": {
      arity(args, 2, 2, GROUP_USAGE[10]);
      return invocation("group.export", { id: args[1] }, (value) => result(value.content, value));
    }
    case "import": {
      arity(args, 2, 2, GROUP_USAGE[11]);
      return invocation("group.import", { content: args[1] }, (value) => result(groupText(value), value));
    }
    case "export-backup": {
      arity(args, 1, 1, GROUP_USAGE[12]);
      return invocation("group.exportBackup", {}, (value) => result(value.content, value));
    }
    case "import-backup": {
      arity(args, 2, 2, GROUP_USAGE[13]);
      return invocation("group.importBackup", { content: args[1] }, (value) => result(lines([`Imported groups: ${value.new}; updated: ${value.updated}; skipped: ${value.skipped}; total: ${value.total}`]), value));
    }
    default:
      throw new Error(`unknown group action: ${args[0]}; use group help`);
  }
}
function parseTagCommand(args) {
  if (wantsHelp(args)) return { kind: "usage", usage: TAG_USAGE };
  switch (args[0]) {
    case "list": {
      arity(args, 1, 1, TAG_USAGE[0]);
      return invocation("tag.list", {}, (tags) => result(lines([`Prompt tags: ${tags.length}`, ...tags.map(
        /** Formats one prompt tag's original summary. */
        (tag) => `- ${tag.id} (${tag.name}) [${tag.tagType}] ${tag.description}`
      )]), tags));
    }
    case "show": {
      arity(args, 2, 2, TAG_USAGE[1]);
      return invocation("tag.get", { id: args[1] }, (tag) => result(tagText(tag), tag));
    }
    case "create": {
      let values;
      if (args[1] === "--record") {
        arity(args, 3, 3, TAG_USAGE[3]);
        values = parseDomainPayload("tag.create", { values: parseJsonObject(args[2], "tag record") }).values;
      } else {
        arity(args, 2, 5, TAG_USAGE[2]);
        const fields5 = { name: args[1] };
        if (args.length >= 3) fields5.promptContent = args[2];
        if (args.length >= 4) fields5.description = args[3];
        if (args.length === 5) fields5.tagType = args[4];
        values = parseDomainPayload("tag.create", { values: fields5 }).values;
      }
      return invocation("tag.create", { values }, (tag) => result(lines([`Prompt tag created: ${tag.id}`]), { id: tag.id, created: true }));
    }
    case "update": {
      const patch = changes(args, "tag", tagField);
      const payload = parseDomainPayload("tag.update", { id: args[1], changes: patch.changes });
      return invocation("tag.update", payload, (tag) => result(lines([`Prompt tag updated: ${tag.id}`]), { id: tag.id, updated: true }));
    }
    case "delete": {
      arity(args, 2, 2, TAG_USAGE[7]);
      return invocation("tag.delete", { id: args[1] }, (value) => result(lines([`Prompt tag deleted: ${value.id}`]), value));
    }
    default:
      throw new Error(`unknown tag action: ${args[0]}; use tag help`);
  }
}
function parseActivePromptCommand(args) {
  if (wantsHelp(args)) return { kind: "usage", usage: ACTIVE_PROMPT_USAGE };
  switch (args[0]) {
    case "show": {
      arity(args, 1, 1, ACTIVE_PROMPT_USAGE[0]);
      return invocation("activePrompt.get", {}, (value) => result(lines([`Active prompt: ${"CharacterCard" in value ? `character card ${value.CharacterCard.id}` : `character group ${value.CharacterGroup.id}`}`]), value));
    }
    case "set-card": {
      arity(args, 2, 2, ACTIVE_PROMPT_USAGE[1]);
      return invocation("activePrompt.setCard", { id: args[1] }, (value) => result(lines([`Active character card: ${value.id}`]), value));
    }
    case "set-group": {
      arity(args, 2, 2, ACTIVE_PROMPT_USAGE[2]);
      return invocation("activePrompt.setGroup", { id: args[1] }, (value) => result(lines([`Active character group: ${value.id}`]), value));
    }
    case "activate-for-chat": {
      arity(args, 1, 3, ACTIVE_PROMPT_USAGE[3]);
      const characterCardName = args.length >= 2 ? optionalIdentifier(args[1]) : null;
      const characterGroupId = args.length === 3 ? optionalIdentifier(args[2]) : null;
      return invocation("activePrompt.activateForChat", { characterCardName, characterGroupId }, (value) => result(lines(["Active prompt updated for chat binding"]), value));
    }
    case "resolved-card": {
      arity(args, 1, 1, ACTIVE_PROMPT_USAGE[4]);
      return invocation("activePrompt.resolvedCard", {}, (value) => result(lines([`Resolved character card: ${value.id}`]), value));
    }
    default:
      throw new Error(`unknown active-prompt action: ${args[0]}; use active-prompt help`);
  }
}
function memoryTags(item) {
  return item.tags.map((tag) => tag.name).join(", ");
}
function memoryRow(item) {
  return `- ${item.id} | ${item.title} | folder: ${displayOptional(item.folderPath)} | tags: ${memoryTags(item)}`;
}
function memoryText(item) {
  return lines([`Memory item ${item.id}`, `UUID: ${item.uuid}`, `Title: ${item.title}`, `Content: ${item.content}`, `Content type: ${item.contentType}`, `Source: ${item.source}`, `Credibility: ${item.credibility}`, `Importance: ${item.importance}`, `Folder: ${displayOptional(item.folderPath)}`, `Created at: ${item.createdAt}`, `Updated at: ${item.updatedAt}`, `Last accessed at: ${item.lastAccessedAt}`, `Tags: ${memoryTags(item)}`]);
}
function memoryField(field, raw) {
  switch (field) {
    case "uuid":
    case "title":
    case "content":
    case "contentType":
    case "source":
    case "folderPath":
      return { [field]: raw };
    case "documentPath":
    case "chunkIndexFilePath":
      return { [field]: optionalIdentifier(raw) };
    case "credibility":
    case "importance":
      return { [field]: number2(raw, field) };
    case "isDocumentNode":
      return { [field]: bool(raw, field) };
    case "createdAt":
    case "updatedAt":
    case "lastAccessedAt":
      return { [field]: integer2(raw, field) };
    case "tags":
      return { tags: csv(raw) };
    case "properties":
      return { properties: json(raw, field) };
    default:
      throw new Error(`unsupported memory field: ${field}`);
  }
}
function memoryRecord(raw) {
  const complete = parseJsonObject(raw, "memory record");
  if (Object.prototype.hasOwnProperty.call(complete, "tags")) {
    if (!Array.isArray(complete.tags)) throw new Error("memory record.tags must be an array of stored tag records");
    complete.tags = complete.tags.map((value, index) => {
      const tag = record(value, `memory record.tags[${index}]`);
      if (Object.keys(tag).length !== 2 || typeof tag.name !== "string") throw new Error(`memory record.tags[${index}] must have id and name`);
      parseMemoryIdentifier(tag.id, `memory record.tags[${index}].id`, false);
      return tag.name;
    });
  }
  return complete;
}
function parseMemoryItems(ownerKey, args) {
  if (wantsHelp(args)) return { kind: "usage", usage: ITEM_USAGE };
  switch (args[0]) {
    case "list": {
      arity(args, 1, 1, ITEM_USAGE[0]);
      return invocation("memory.list", { ownerKey }, (value) => result(lines([`Memory items: ${value.items.length}`, ...value.items.map(memoryRow)]), value));
    }
    case "search": {
      arity(args, 2, 2, ITEM_USAGE[1]);
      return invocation("memory.search", { ownerKey, query: args[1] }, (value) => result(lines([`Memory search results: ${value.items.length}`, `Query: ${value.query}`, ...value.items.map(memoryRow)]), value));
    }
    case "show": {
      arity(args, 2, 2, ITEM_USAGE[2]);
      return invocation("memory.get", { ownerKey, title: args[1] }, (value) => result(memoryText(value.item), value));
    }
    case "create": {
      let values;
      if (args[1] === "--record") {
        arity(args, 3, 3, ITEM_USAGE[4]);
        values = parseDomainPayload("memory.create", { ownerKey, values: memoryRecord(args[2]) }).values;
      } else {
        arity(args, 3, 5, ITEM_USAGE[3]);
        values = { title: args[1], content: args[2], contentType: "text", source: "cli" };
        if (args.length >= 4) values.folderPath = args[3];
        if (args.length === 5) values.tags = csv(args[4]);
      }
      return invocation("memory.create", { ownerKey, values }, (value) => result(lines([`Created memory item ${value.item.id}`, `Title: ${value.item.title}`]), value));
    }
    case "update": {
      arity(args, 4, 4, ITEM_USAGE[5]);
      const fields5 = args[2] === "--patch" ? parseJsonObject(args[3], "memory changes") : args[2] === "--record" ? memoryRecord(args[3]) : memoryField(args[2], args[3]);
      const { id: expectedId, ...mutable } = fields5;
      const input = { ownerKey, originalTitle: args[1], changes: mutable };
      if (Object.prototype.hasOwnProperty.call(fields5, "id")) {
        if (args[2] !== "--record") throw new Error("memory patch cannot change the immutable id");
        input.expectedId = expectedId;
      }
      const payload = parseDomainPayload("memory.update", input);
      return invocation("memory.update", payload, (value) => result(memoryText(value.item), value));
    }
    case "delete": {
      arity(args, 2, 2, ITEM_USAGE[8]);
      return invocation("memory.delete", { ownerKey, id: parseMemoryIdentifier(args[1], "memory id") }, (value) => result(lines([`Deleted memory item ${value.id}: ${value.deleted}`]), value));
    }
    case "move": {
      arity(args, 3, 3, ITEM_USAGE[9]);
      const ids = [];
      for (const part of args[1].split(",")) ids.push(parseMemoryIdentifier(part.trim(), "memory id"));
      return invocation("memory.move", { ownerKey, ids, folderPath: args[2] }, (value) => result(lines([`Moved memory items: ${value.moved}`, `Folder: ${value.folder}`]), value));
    }
    default:
      throw new Error(`unknown memory item action: ${args[0]}; use memory <owner> item help`);
  }
}
function parseUser(ownerKey, args) {
  if (wantsHelp(args)) return { kind: "usage", usage: USER_USAGE };
  switch (args[0]) {
    case "show": {
      arity(args, 1, 1, USER_USAGE[0]);
      return invocation("memory.user.read", { ownerKey }, (value) => result(value.content, value));
    }
    case "write": {
      arity(args, 2, 2, USER_USAGE[1]);
      return invocation("memory.user.write", { ownerKey, content: args[1] }, (value) => result(lines([`Updated ${ownerKey}/USER.md`]), value));
    }
    case "path": {
      arity(args, 1, 1, USER_USAGE[2]);
      return invocation("memory.user.path", { ownerKey }, (value) => result(lines([`USER.md path: ${value.path}`]), value));
    }
    default:
      throw new Error(`unknown memory user action: ${args[0]}; use memory <owner> user help`);
  }
}
function parseLinks(ownerKey, args) {
  if (wantsHelp(args)) return { kind: "usage", usage: LINK_USAGE };
  switch (args[0]) {
    case "create": {
      arity(args, 6, 6, LINK_USAGE[0]);
      return invocation("memory.link.create", { ownerKey, sourceTitle: args[1], targetTitle: args[2], linkType: args[3], weight: number2(args[4], "weight"), description: args[5] }, (value) => result(lines([`Created memory link ${value.link.id}`]), value));
    }
    case "update": {
      arity(args, 4, 4, LINK_USAGE[1]);
      const linkId = parseMemoryIdentifier(args[1], "link id");
      let patch;
      if (args[2] === "--patch") patch = parseJsonObject(args[3], "link changes");
      else if (args[2] === "weight") patch = { weight: number2(args[3], "weight") };
      else if (args[2] === "linkType" || args[2] === "description") patch = { [args[2]]: args[3] };
      else throw new Error(`unsupported link field: ${args[2]}`);
      const payload = parseDomainPayload("memory.link.update", { ownerKey, linkId, changes: patch });
      return invocation("memory.link.update", payload, (value) => result(lines([`Updated memory link ${value.link.id}`]), value));
    }
    case "delete": {
      arity(args, 2, 2, LINK_USAGE[3]);
      return invocation("memory.link.delete", { ownerKey, linkId: parseMemoryIdentifier(args[1], "link id") }, (value) => result(lines([`Deleted memory link ${value.linkId}: ${value.deleted}`]), value));
    }
    default:
      throw new Error(`unknown memory link action: ${args[0]}; use memory <owner> link help`);
  }
}
function parseOwner2(ownerKey, args, usage) {
  parseDomainPayload("memory.graph", { ownerKey });
  if (wantsHelp(args)) return { kind: "usage", usage };
  switch (args[0]) {
    case "user":
      return parseUser(ownerKey, args.slice(1));
    case "item":
      return parseMemoryItems(ownerKey, args.slice(1));
    case "link":
      return parseLinks(ownerKey, args.slice(1));
    case "graph": {
      arity(args, 1, 1, "operit2 memory <owner> graph");
      return invocation("memory.graph", { ownerKey }, (value) => result(lines([`Memory graph for ${ownerKey}`, `Nodes: ${value.graph.nodes.length}`, `Edges: ${value.graph.edges.length}`]), value));
    }
    case "export": {
      arity(args, 1, 1, "operit2 memory <owner> export");
      return invocation("memory.export", { ownerKey }, (value) => result(value.content, value));
    }
    case "import": {
      arity(args, 3, 3, "operit2 memory <owner> import <SKIP|UPDATE|CREATE_NEW> <json-content>");
      const payload = parseDomainPayload("memory.import", { ownerKey, strategy: args[1], content: args[2] });
      return invocation("memory.import", payload, (value) => result(lines([`Imported memory backup for ${ownerKey}`]), value));
    }
    case "settings": {
      const help = ["operit2 memory <owner> settings show", "operit2 memory <owner> settings write <settings-json>"];
      const sub = args.slice(1);
      if (wantsHelp(sub)) return { kind: "usage", usage: help };
      if (sub[0] === "show") {
        arity(sub, 1, 1, help[0]);
        return invocation("memory.settings.read", { ownerKey }, (value) => result(lines([JSON.stringify(value, null, 2)]), value));
      }
      if (sub[0] === "write") {
        arity(sub, 2, 2, help[1]);
        const payload = parseDomainPayload("memory.settings.write", { ownerKey, settings: parseJsonObject(sub[1], "memory settings") });
        return invocation("memory.settings.write", payload, (value) => result(lines([`Updated memory settings for ${ownerKey}`]), value));
      }
      throw new Error(`unknown memory settings action: ${sub[0]}`);
    }
    case "search-config": {
      const help = ["operit2 memory <owner> search-config show", "operit2 memory <owner> search-config write <config-json>"];
      const sub = args.slice(1);
      if (wantsHelp(sub)) return { kind: "usage", usage: help };
      if (sub[0] === "show") {
        arity(sub, 1, 1, help[0]);
        return invocation("memory.searchConfig.read", { ownerKey }, (value) => result(lines([JSON.stringify(value, null, 2)]), value));
      }
      if (sub[0] === "write") {
        arity(sub, 2, 2, help[1]);
        const payload = parseDomainPayload("memory.searchConfig.write", { ownerKey, config: parseJsonObject(sub[1], "memory search config") });
        return invocation("memory.searchConfig.write", payload, (value) => result(lines([`Updated memory search config for ${ownerKey}`]), value));
      }
      throw new Error(`unknown memory search-config action: ${sub[0]}`);
    }
    default:
      throw new Error(`unknown memory owner action: ${args[0]}; use memory <owner> help`);
  }
}
function parseShared(args) {
  if (wantsHelp(args)) return { kind: "usage", usage: SHARED_MEMORY_USAGE };
  switch (args[0]) {
    case "list": {
      arity(args, 1, 1, "operit2 memory shared list");
      return invocation("memory.shared.list", {}, (stores) => result(lines([`Shared memory stores: ${stores.length}`, ...stores.map(
        /** Formats one canonical shared store. */
        (store) => `- ${store.id} | ${store.name} | created: ${store.createdAt} | updated: ${store.updatedAt}`
      )]), stores));
    }
    case "create": {
      arity(args, 2, 2, "operit2 memory shared create <name>");
      return invocation("memory.shared.create", { name: args[1] }, (store) => result(lines([`Created shared memory store ${store.id}`, `Name: ${store.name}`]), store));
    }
    case "rename": {
      arity(args, 3, 3, "operit2 memory shared rename <shared-id> <name>");
      return invocation("memory.shared.rename", { id: args[1], name: args[2] }, (store) => result(lines([`Renamed shared memory store ${store.id}`, `Name: ${store.name}`]), store));
    }
    case "delete": {
      arity(args, 2, 2, "operit2 memory shared delete <shared-id>");
      return invocation("memory.shared.delete", { id: args[1] }, (value) => result(lines([`Deleted shared memory store ${value.sharedId}: ${value.deleted}`, `Characters cleaned: ${value.cleanedCharacters}`]), value));
    }
    default:
      return parseOwner2(`shared:${args[0]}`, args.slice(1), SHARED_MEMORY_USAGE);
  }
}
function parseMount(args) {
  if (wantsHelp(args)) return { kind: "usage", usage: MOUNT_USAGE };
  arity(args, 6, 6, MOUNT_USAGE[0]);
  const flags = /* @__PURE__ */ new Map();
  for (let index = 2; index < args.length; index += 2) {
    const flag = args[index];
    if (flag !== "--read" && flag !== "--write") throw new Error(`unknown memory mount option: ${flag}`);
    if (flags.has(flag)) throw new Error(`duplicate memory mount option: ${flag}`);
    flags.set(flag, bool(args[index + 1], flag));
  }
  if (!flags.has("--read") || !flags.has("--write")) throw new Error("memory mount requires --read and --write");
  const readable = flags.get("--read");
  const writable = flags.get("--write");
  return invocation("memory.mount", { characterId: args[0], sharedId: args[1], readable, writable }, (value) => result(lines([`Mounted shared memory ${value.sharedId} on ${value.characterId}`, `Readable: ${value.mount.readable}`, `Writable: ${value.mount.writable}`]), value));
}
function parseMemoryCommand(args) {
  if (wantsHelp(args)) return { kind: "usage", usage: MEMORY_USAGE };
  switch (args[0]) {
    case "character": {
      const sub = args.slice(1);
      if (wantsHelp(sub)) return { kind: "usage", usage: CHARACTER_MEMORY_USAGE };
      return parseOwner2(`character:${sub[0]}`, sub.slice(1), CHARACTER_MEMORY_USAGE);
    }
    case "shared":
      return parseShared(args.slice(1));
    case "mount":
      return parseMount(args.slice(1));
    case "unmount": {
      const sub = args.slice(1);
      if (wantsHelp(sub)) return { kind: "usage", usage: UNMOUNT_USAGE };
      arity(sub, 2, 2, UNMOUNT_USAGE[0]);
      return invocation("memory.unmount", { characterId: sub[0], sharedId: sub[1] }, (value) => result(lines([`Unmounted shared memory ${value.sharedId} from ${value.characterId}: ${value.unmounted}`]), value));
    }
    case "resolve": {
      arity(args, 2, 2, "operit2 memory resolve <character-id>");
      return invocation("memory.resolveOwner", { characterId: args[1] }, (value) => result(lines([JSON.stringify(value)]), value));
    }
    default:
      throw new Error(`unknown memory namespace: ${args[0]}; use memory help`);
  }
}
function failure(family, code, error) {
  const message = error instanceof Error ? error.message : String(error);
  return { stderr: `${message}
`, json: { ok: false, command: family, error: { code, message } } };
}
function execute(family, parse, event) {
  return __async(this, null, function* () {
    let parsed;
    try {
      if (event.eventPayload.commandName !== family) throw new Error(`command event ${event.eventPayload.commandName} does not match ${family}`);
      if (!Array.isArray(event.eventPayload.args)) throw new Error("command args must be a string array");
      for (const value of event.eventPayload.args) if (typeof value !== "string") throw new Error("command args must contain only strings");
      parsed = parse(event.eventPayload.args);
    } catch (error) {
      return failure(family, "invalid_arguments", error);
    }
    if (parsed.kind === "usage") return result(lines(parsed.usage), { usage: parsed.usage });
    try {
      return yield parsed.execute();
    } catch (error) {
      return failure(family, "domain_operation_failed", error);
    }
  });
}
function onCharacterCommand(event) {
  return __async(this, null, function* () {
    return execute("character", parseCharacterCommand, event);
  });
}
function onGroupCommand(event) {
  return __async(this, null, function* () {
    return execute("group", parseGroupCommand, event);
  });
}
function onTagCommand(event) {
  return __async(this, null, function* () {
    return execute("tag", parseTagCommand, event);
  });
}
function onActivePromptCommand(event) {
  return __async(this, null, function* () {
    return execute("active-prompt", parseActivePromptCommand, event);
  });
}
function onMemoryCommand(event) {
  return __async(this, null, function* () {
    return execute("memory", parseMemoryCommand, event);
  });
}
function registerDomainCommands() {
  ToolPkg.registerCoreCommand({ id: "character", name: "character", title: { zh: "\u89D2\u8272\u5361", en: "Characters" }, description: { zh: "\u7BA1\u7406\u89D2\u8272\u5361\u3001\u5B8C\u6574\u7ED1\u5B9A\u3001\u63D0\u793A\u8BCD\u548C\u5BFC\u5165\u5BFC\u51FA\u3002", en: "Manage characters, bindings, prompts, and interchange." }, usage: "/character help", function: onCharacterCommand });
  ToolPkg.registerCoreCommand({ id: "group", name: "group", title: { zh: "\u89D2\u8272\u7FA4\u7EC4", en: "Character groups" }, description: { zh: "\u7BA1\u7406\u89D2\u8272\u7FA4\u7EC4\u3001\u6210\u5458\u987A\u5E8F\u3001\u6FC0\u6D3B\u548C\u5BFC\u5165\u5BFC\u51FA\u3002", en: "Manage character groups, ordered members, activation, and interchange." }, usage: "/group help", function: onGroupCommand });
  ToolPkg.registerCoreCommand({ id: "tag", name: "tag", title: { zh: "\u63D0\u793A\u8BCD\u6807\u7B7E", en: "Prompt tags" }, description: { zh: "\u8BFB\u53D6\u3001\u521B\u5EFA\u3001\u7F16\u8F91\u548C\u5220\u9664\u63D0\u793A\u8BCD\u6807\u7B7E\u3002", en: "Read, create, update, and delete prompt tags." }, usage: "/tag help", function: onTagCommand });
  ToolPkg.registerCoreCommand({ id: "active-prompt", name: "active-prompt", title: { zh: "\u5F53\u524D\u63D0\u793A\u8BCD", en: "Active prompt" }, description: { zh: "\u67E5\u8BE2\u548C\u8BBE\u7F6E\u5F53\u524D\u89D2\u8272\u3001\u7FA4\u7EC4\u4EE5\u53CA\u804A\u5929\u7ED1\u5B9A\u3002", en: "Read and select the active character, group, and chat binding." }, usage: "/active-prompt help", function: onActivePromptCommand });
  ToolPkg.registerCoreCommand({ id: "memory", name: "memory", title: { zh: "\u89D2\u8272\u8BB0\u5FC6", en: "Character memory" }, description: { zh: "\u7BA1\u7406\u89D2\u8272\u548C\u5171\u4EAB\u8BB0\u5FC6\u3001\u6302\u8F7D\u3001USER.md\u3001\u6761\u76EE\u3001\u5173\u7CFB\u3001\u56FE\u8C31\u53CA\u8BBE\u7F6E\u3002", en: "Manage character and shared memory, mounts, USER.md, items, links, graphs, and settings." }, usage: "/memory help", function: onMemoryCommand });
}

// src/memory-jobs/hooks.ts
function onMemoryMessagePersisted(event) {
  return __async(this, null, function* () {
    const payload = event.eventPayload;
    assertString(payload.sender, "persisted message sender");
    if (payload.sender !== "ai" && payload.sender !== "assistant") return;
    const chatId = requireId(payload.chatId, "persisted chat id");
    assertInteger(payload.timestamp, "persisted message timestamp", 0);
    assertString(payload.content, "persisted message content");
    if (payload.content.trim() === "") return;
    assertInteger(payload.selectedVariantIndex, "persisted message selected variant index", 0);
    if (payload.selectedVariantIndex > 2147483647) throw new Error("Persisted message variant exceeds the host range");
    yield dispatchDomain2("memory.candidate.enqueue", { chatId, timestamp: payload.timestamp, variantIndex: payload.selectedVariantIndex, sourceType: "reply_finalized_auto", ownerKey: null });
  });
}
function onMemoryInterval(_event) {
  return __async(this, null, function* () {
    yield runMemoryJobs();
  });
}
function registerMemoryJobHooks() {
  ToolPkg.registerChatMessageHook({ id: "memory-candidate-enqueue", function: onMemoryMessagePersisted });
  ToolPkg.registerHostEventHook({ id: "memory-jobs-interval", source: "interval", trigger: { kind: "interval", intervalMs: 6e4 }, function: onMemoryInterval });
}

// src/public-api.ts
function conversationGroupListApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("conversation-group.list", parseDomainPayload("conversation-group.list", event.payload));
  });
}
function conversationGroupCreateApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("conversation-group.create", parseDomainPayload("conversation-group.create", event.payload));
  });
}
function conversationGroupUpdateApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("conversation-group.update", parseDomainPayload("conversation-group.update", event.payload));
  });
}
function conversationGroupDeleteApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("conversation-group.delete", parseDomainPayload("conversation-group.delete", event.payload));
  });
}
function conversationGroupMoveChatApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("conversation-group.moveChat", parseDomainPayload("conversation-group.moveChat", event.payload));
  });
}
function conversationGroupReorderApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("conversation-group.reorder", parseDomainPayload("conversation-group.reorder", event.payload));
  });
}
function chatConfigurationResolveApi(event) {
  return __async(this, null, function* () {
    if (event.callerPackage !== "host") throw new Error("chat.configuration.resolve requires an authenticated host caller");
    return dispatchDomain2("chat.configuration.resolve", parseDomainPayload("chat.configuration.resolve", event.payload));
  });
}
function chatConfigurationBindingReadApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("chat.configuration.binding.read", parseDomainPayload("chat.configuration.binding.read", event.payload));
  });
}
function chatConfigurationBindingWriteApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("chat.configuration.binding.write", parseDomainPayload("chat.configuration.binding.write", event.payload));
  });
}
function chatConfigurationBindingDeleteApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("chat.configuration.binding.delete", parseDomainPayload("chat.configuration.binding.delete", event.payload));
  });
}
function registerChatConfigurationApis() {
  ToolPkg.registerApi({ name: "chat.configuration.resolve", function: chatConfigurationResolveApi });
  ToolPkg.registerApi({ name: "chat.configuration.binding.read", function: chatConfigurationBindingReadApi });
  ToolPkg.registerApi({ name: "chat.configuration.binding.write", function: chatConfigurationBindingWriteApi });
  ToolPkg.registerApi({ name: "chat.configuration.binding.delete", function: chatConfigurationBindingDeleteApi });
}
function memoryChatListApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.chat.list", parseDomainPayload("memory.chat.list", event.payload));
  });
}
function memoryChatUpdateApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.chat.update", parseDomainPayload("memory.chat.update", event.payload));
  });
}
function memoryCategorizeApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.categorize", parseDomainPayload("memory.categorize", event.payload));
  });
}
function memoryRebuildStartApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.rebuild.start", parseDomainPayload("memory.rebuild.start", event.payload));
  });
}
function memoryRebuildProgressApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.rebuild.progress", parseDomainPayload("memory.rebuild.progress", event.payload));
  });
}
function memoryRebuildCancelApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.rebuild.cancel", parseDomainPayload("memory.rebuild.cancel", event.payload));
  });
}
function memoryEmbeddingsRebuildApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.embeddings.rebuild", parseDomainPayload("memory.embeddings.rebuild", event.payload));
  });
}
function memoryCandidateEnqueueApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.candidate.enqueue", parseDomainPayload("memory.candidate.enqueue", event.payload));
  });
}
function memorySearchWithOptionsApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.searchWithOptions", parseDomainPayload("memory.searchWithOptions", event.payload));
  });
}
function snapshotApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("snapshot", parseDomainPayload("snapshot", event.payload));
  });
}
function characterListApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("character.list", parseDomainPayload("character.list", event.payload));
  });
}
function characterGetApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("character.get", parseDomainPayload("character.get", event.payload));
  });
}
function characterCreateApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("character.create", parseDomainPayload("character.create", event.payload));
  });
}
function characterUpdateApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("character.update", parseDomainPayload("character.update", event.payload));
  });
}
function characterDeleteApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("character.delete", parseDomainPayload("character.delete", event.payload));
  });
}
function characterSetActiveApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("character.setActive", parseDomainPayload("character.setActive", event.payload));
  });
}
function characterCombineApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("character.combine", parseDomainPayload("character.combine", event.payload));
  });
}
function characterResetDefaultApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("character.resetDefault", parseDomainPayload("character.resetDefault", event.payload));
  });
}
function characterExportApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("character.export", parseDomainPayload("character.export", event.payload));
  });
}
function characterImportApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("character.import", parseDomainPayload("character.import", event.payload));
  });
}
function characterExportBackupApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("character.exportBackup", parseDomainPayload("character.exportBackup", event.payload));
  });
}
function characterImportBackupApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("character.importBackup", parseDomainPayload("character.importBackup", event.payload));
  });
}
function groupListApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("group.list", parseDomainPayload("group.list", event.payload));
  });
}
function groupGetApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("group.get", parseDomainPayload("group.get", event.payload));
  });
}
function groupCreateApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("group.create", parseDomainPayload("group.create", event.payload));
  });
}
function groupUpdateApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("group.update", parseDomainPayload("group.update", event.payload));
  });
}
function groupDeleteApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("group.delete", parseDomainPayload("group.delete", event.payload));
  });
}
function groupSetActiveApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("group.setActive", parseDomainPayload("group.setActive", event.payload));
  });
}
function groupDuplicateApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("group.duplicate", parseDomainPayload("group.duplicate", event.payload));
  });
}
function groupExportApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("group.export", parseDomainPayload("group.export", event.payload));
  });
}
function groupImportApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("group.import", parseDomainPayload("group.import", event.payload));
  });
}
function groupExportBackupApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("group.exportBackup", parseDomainPayload("group.exportBackup", event.payload));
  });
}
function groupImportBackupApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("group.importBackup", parseDomainPayload("group.importBackup", event.payload));
  });
}
function activePromptGetApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("activePrompt.get", parseDomainPayload("activePrompt.get", event.payload));
  });
}
function activePromptSetCardApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("activePrompt.setCard", parseDomainPayload("activePrompt.setCard", event.payload));
  });
}
function activePromptSetGroupApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("activePrompt.setGroup", parseDomainPayload("activePrompt.setGroup", event.payload));
  });
}
function activePromptActivateForChatApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("activePrompt.activateForChat", parseDomainPayload("activePrompt.activateForChat", event.payload));
  });
}
function activePromptResolvedCardApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("activePrompt.resolvedCard", parseDomainPayload("activePrompt.resolvedCard", event.payload));
  });
}
function tagListApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("tag.list", parseDomainPayload("tag.list", event.payload));
  });
}
function tagGetApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("tag.get", parseDomainPayload("tag.get", event.payload));
  });
}
function tagCreateApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("tag.create", parseDomainPayload("tag.create", event.payload));
  });
}
function tagUpdateApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("tag.update", parseDomainPayload("tag.update", event.payload));
  });
}
function tagDeleteApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("tag.delete", parseDomainPayload("tag.delete", event.payload));
  });
}
function memorySharedListApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.shared.list", parseDomainPayload("memory.shared.list", event.payload));
  });
}
function memorySharedCreateApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.shared.create", parseDomainPayload("memory.shared.create", event.payload));
  });
}
function memorySharedRenameApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.shared.rename", parseDomainPayload("memory.shared.rename", event.payload));
  });
}
function memorySharedDeleteApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.shared.delete", parseDomainPayload("memory.shared.delete", event.payload));
  });
}
function memoryMountApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.mount", parseDomainPayload("memory.mount", event.payload));
  });
}
function memoryUnmountApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.unmount", parseDomainPayload("memory.unmount", event.payload));
  });
}
function memoryUserReadApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.user.read", parseDomainPayload("memory.user.read", event.payload));
  });
}
function memoryUserWriteApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.user.write", parseDomainPayload("memory.user.write", event.payload));
  });
}
function memoryUserPathApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.user.path", parseDomainPayload("memory.user.path", event.payload));
  });
}
function memoryResolveOwnerApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.resolveOwner", parseDomainPayload("memory.resolveOwner", event.payload));
  });
}
function memorySettingsReadApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.settings.read", parseDomainPayload("memory.settings.read", event.payload));
  });
}
function memorySettingsWriteApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.settings.write", parseDomainPayload("memory.settings.write", event.payload));
  });
}
function memorySearchConfigReadApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.searchConfig.read", parseDomainPayload("memory.searchConfig.read", event.payload));
  });
}
function memorySearchConfigWriteApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.searchConfig.write", parseDomainPayload("memory.searchConfig.write", event.payload));
  });
}
function memoryGraphApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.graph", parseDomainPayload("memory.graph", event.payload));
  });
}
function memoryListApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.list", parseDomainPayload("memory.list", event.payload));
  });
}
function memorySearchApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.search", parseDomainPayload("memory.search", event.payload));
  });
}
function memoryGetApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.get", parseDomainPayload("memory.get", event.payload));
  });
}
function memoryCreateApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.create", parseDomainPayload("memory.create", event.payload));
  });
}
function memoryUpdateApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.update", parseDomainPayload("memory.update", event.payload));
  });
}
function memoryDeleteApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.delete", parseDomainPayload("memory.delete", event.payload));
  });
}
function memoryMoveApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.move", parseDomainPayload("memory.move", event.payload));
  });
}
function memoryLinkCreateApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.link.create", parseDomainPayload("memory.link.create", event.payload));
  });
}
function memoryLinkUpdateApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.link.update", parseDomainPayload("memory.link.update", event.payload));
  });
}
function memoryLinkDeleteApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.link.delete", parseDomainPayload("memory.link.delete", event.payload));
  });
}
function memoryExportApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.export", parseDomainPayload("memory.export", event.payload));
  });
}
function memoryImportApi(event) {
  return __async(this, null, function* () {
    return dispatchDomain2("memory.import", parseDomainPayload("memory.import", event.payload));
  });
}
function registerDomainApis() {
  ToolPkg.registerApi({ name: "conversation-group.list", function: conversationGroupListApi });
  ToolPkg.registerApi({ name: "conversation-group.create", function: conversationGroupCreateApi });
  ToolPkg.registerApi({ name: "conversation-group.update", function: conversationGroupUpdateApi });
  ToolPkg.registerApi({ name: "conversation-group.delete", function: conversationGroupDeleteApi });
  ToolPkg.registerApi({ name: "conversation-group.moveChat", function: conversationGroupMoveChatApi });
  ToolPkg.registerApi({ name: "conversation-group.reorder", function: conversationGroupReorderApi });
  ToolPkg.registerApi({ name: "memory.searchWithOptions", function: memorySearchWithOptionsApi });
  ToolPkg.registerApi({ name: "memory.candidate.enqueue", function: memoryCandidateEnqueueApi });
  ToolPkg.registerApi({ name: "memory.embeddings.rebuild", function: memoryEmbeddingsRebuildApi });
  ToolPkg.registerApi({ name: "memory.rebuild.cancel", function: memoryRebuildCancelApi });
  ToolPkg.registerApi({ name: "memory.rebuild.progress", function: memoryRebuildProgressApi });
  ToolPkg.registerApi({ name: "memory.rebuild.start", function: memoryRebuildStartApi });
  ToolPkg.registerApi({ name: "memory.categorize", function: memoryCategorizeApi });
  ToolPkg.registerApi({ name: "memory.chat.update", function: memoryChatUpdateApi });
  ToolPkg.registerApi({ name: "memory.chat.list", function: memoryChatListApi });
  registerChatConfigurationApis();
  ToolPkg.registerApi({ name: "snapshot", function: snapshotApi });
  ToolPkg.registerApi({ name: "character.list", function: characterListApi });
  ToolPkg.registerApi({ name: "character.get", function: characterGetApi });
  ToolPkg.registerApi({ name: "character.create", function: characterCreateApi });
  ToolPkg.registerApi({ name: "character.update", function: characterUpdateApi });
  ToolPkg.registerApi({ name: "character.delete", function: characterDeleteApi });
  ToolPkg.registerApi({ name: "character.setActive", function: characterSetActiveApi });
  ToolPkg.registerApi({ name: "character.combine", function: characterCombineApi });
  ToolPkg.registerApi({ name: "character.resetDefault", function: characterResetDefaultApi });
  ToolPkg.registerApi({ name: "character.export", function: characterExportApi });
  ToolPkg.registerApi({ name: "character.import", function: characterImportApi });
  ToolPkg.registerApi({ name: "character.exportBackup", function: characterExportBackupApi });
  ToolPkg.registerApi({ name: "character.importBackup", function: characterImportBackupApi });
  ToolPkg.registerApi({ name: "group.list", function: groupListApi });
  ToolPkg.registerApi({ name: "group.get", function: groupGetApi });
  ToolPkg.registerApi({ name: "group.create", function: groupCreateApi });
  ToolPkg.registerApi({ name: "group.update", function: groupUpdateApi });
  ToolPkg.registerApi({ name: "group.delete", function: groupDeleteApi });
  ToolPkg.registerApi({ name: "group.setActive", function: groupSetActiveApi });
  ToolPkg.registerApi({ name: "group.duplicate", function: groupDuplicateApi });
  ToolPkg.registerApi({ name: "group.export", function: groupExportApi });
  ToolPkg.registerApi({ name: "group.import", function: groupImportApi });
  ToolPkg.registerApi({ name: "group.exportBackup", function: groupExportBackupApi });
  ToolPkg.registerApi({ name: "group.importBackup", function: groupImportBackupApi });
  ToolPkg.registerApi({ name: "activePrompt.get", function: activePromptGetApi });
  ToolPkg.registerApi({ name: "activePrompt.setCard", function: activePromptSetCardApi });
  ToolPkg.registerApi({ name: "activePrompt.setGroup", function: activePromptSetGroupApi });
  ToolPkg.registerApi({ name: "activePrompt.activateForChat", function: activePromptActivateForChatApi });
  ToolPkg.registerApi({ name: "activePrompt.resolvedCard", function: activePromptResolvedCardApi });
  ToolPkg.registerApi({ name: "tag.list", function: tagListApi });
  ToolPkg.registerApi({ name: "tag.get", function: tagGetApi });
  ToolPkg.registerApi({ name: "tag.create", function: tagCreateApi });
  ToolPkg.registerApi({ name: "tag.update", function: tagUpdateApi });
  ToolPkg.registerApi({ name: "tag.delete", function: tagDeleteApi });
  ToolPkg.registerApi({ name: "memory.shared.list", function: memorySharedListApi });
  ToolPkg.registerApi({ name: "memory.shared.create", function: memorySharedCreateApi });
  ToolPkg.registerApi({ name: "memory.shared.rename", function: memorySharedRenameApi });
  ToolPkg.registerApi({ name: "memory.shared.delete", function: memorySharedDeleteApi });
  ToolPkg.registerApi({ name: "memory.mount", function: memoryMountApi });
  ToolPkg.registerApi({ name: "memory.unmount", function: memoryUnmountApi });
  ToolPkg.registerApi({ name: "memory.user.read", function: memoryUserReadApi });
  ToolPkg.registerApi({ name: "memory.user.write", function: memoryUserWriteApi });
  ToolPkg.registerApi({ name: "memory.user.path", function: memoryUserPathApi });
  ToolPkg.registerApi({ name: "memory.resolveOwner", function: memoryResolveOwnerApi });
  ToolPkg.registerApi({ name: "memory.settings.read", function: memorySettingsReadApi });
  ToolPkg.registerApi({ name: "memory.settings.write", function: memorySettingsWriteApi });
  ToolPkg.registerApi({ name: "memory.searchConfig.read", function: memorySearchConfigReadApi });
  ToolPkg.registerApi({ name: "memory.searchConfig.write", function: memorySearchConfigWriteApi });
  ToolPkg.registerApi({ name: "memory.graph", function: memoryGraphApi });
  ToolPkg.registerApi({ name: "memory.list", function: memoryListApi });
  ToolPkg.registerApi({ name: "memory.search", function: memorySearchApi });
  ToolPkg.registerApi({ name: "memory.get", function: memoryGetApi });
  ToolPkg.registerApi({ name: "memory.create", function: memoryCreateApi });
  ToolPkg.registerApi({ name: "memory.update", function: memoryUpdateApi });
  ToolPkg.registerApi({ name: "memory.delete", function: memoryDeleteApi });
  ToolPkg.registerApi({ name: "memory.move", function: memoryMoveApi });
  ToolPkg.registerApi({ name: "memory.link.create", function: memoryLinkCreateApi });
  ToolPkg.registerApi({ name: "memory.link.update", function: memoryLinkUpdateApi });
  ToolPkg.registerApi({ name: "memory.link.delete", function: memoryLinkDeleteApi });
  ToolPkg.registerApi({ name: "memory.export", function: memoryExportApi });
  ToolPkg.registerApi({ name: "memory.import", function: memoryImportApi });
}

// src/chat-lifecycle.ts
var CHARACTER_CARDS_NAMESPACE = "com.operit.character_cards";
function fields3(value, keys, label) {
  const input = record(value, label), actual = Object.keys(input);
  if (actual.length !== keys.length || actual.some(
    /** Rejects undocumented fields rather than guessing their meaning or owner. */
    (key) => keys.indexOf(key) === -1
  )) throw new Error(label + " has invalid fields");
  return input;
}
function identity3(value, label) {
  if (typeof value !== "string" || value.trim() === "") throw new Error(label + " must be a nonblank string");
  return value;
}
function parseChatSelectionMarker(value, label) {
  const marker = fields3(value, ["version", "selection"], label);
  if (marker.version !== 1) throw new Error(label + ".version must be 1");
  const selection2 = identity3(marker.selection, label + ".selection");
  parseChatSelection(selection2);
  return { version: 1, selection: selection2 };
}
function scopedInput(value) {
  if (value === null) return null;
  const input = markerObject(value, "chat lifecycle.input");
  if (!Object.prototype.hasOwnProperty.call(input, CHARACTER_CARDS_NAMESPACE)) return null;
  return parseChatSelectionMarker(input[CHARACTER_CARDS_NAMESPACE], "chat lifecycle.input." + CHARACTER_CARDS_NAMESPACE);
}
function parseChatInitialization(value) {
  const keys = Object.keys(value);
  if (keys.length !== 7 || keys.some((key) => ["eventName", "creationKind", "chat", "sourceChatId", "sourceMessageTimestamp", "input", "sourceExtension"].indexOf(key) < 0)) throw new Error("chat lifecycle has invalid fields");
  const chatKeys = Object.keys(value.chat);
  if (chatKeys.length !== 4 || chatKeys.some((key) => ["id", "title", "workspaceId", "parentChatId"].indexOf(key) < 0)) throw new Error("chat lifecycle.chat has invalid fields");
  if (value.eventName !== "before_create") throw new Error("chat lifecycle.eventName must be before_create");
  if (value.creationKind !== "new" && value.creationKind !== "branch") throw new Error("chat lifecycle.creationKind must be new or branch");
  const workspaceId = value.chat.workspaceId === null ? null : identity3(value.chat.workspaceId, "chat lifecycle.chat.workspaceId");
  const parentChatId = value.chat.parentChatId === null ? null : identity3(value.chat.parentChatId, "chat lifecycle.chat.parentChatId");
  const sourceChatId = value.sourceChatId === null ? null : identity3(value.sourceChatId, "chat lifecycle.sourceChatId");
  const sourceMessageTimestamp = value.sourceMessageTimestamp;
  if (sourceMessageTimestamp !== null && (!Number.isSafeInteger(sourceMessageTimestamp) || sourceMessageTimestamp < 0)) throw new Error("chat lifecycle.sourceMessageTimestamp must be a nonnegative safe integer or null");
  if (sourceChatId === null && (value.sourceExtension !== null || sourceMessageTimestamp !== null)) throw new Error("Source namespace metadata and message time require an explicit sourceChatId");
  return {
    eventName: "before_create",
    creationKind: value.creationKind,
    chat: { id: identity3(value.chat.id, "chat lifecycle.chat.id"), title: value.chat.title, workspaceId, parentChatId },
    sourceChatId,
    sourceMessageTimestamp,
    input: scopedInput(value.input),
    sourceExtension: value.sourceExtension === null ? null : markerObject(value.sourceExtension, "chat lifecycle.sourceExtension")
  };
}
function initializeChatExtension(value, reader) {
  return __async(this, null, function* () {
    const event = parseChatInitialization(value);
    if (event.input !== null) {
      yield reader.requireSelection(event.input.selection);
      return { extension: { version: 1, selection: event.input.selection } };
    }
    if (event.sourceChatId !== null) {
      if (event.sourceExtension === null) throw new Error("Explicit sourceChatId has no character namespace extension");
      const marker = decodeChatMarker(event.sourceExtension);
      yield reader.requireSelection(marker.selection);
      return { extension: encodeChatMarker(marker.selection, event.sourceExtension) };
    }
    const selection2 = yield reader.readActiveSelection();
    yield reader.requireSelection(selection2);
    return { extension: { version: 1, selection: selection2 } };
  });
}
function readActiveSelection() {
  return __async(this, null, function* () {
    return selectionForActive(yield dispatchDomain2("activePrompt.get", {}));
  });
}
function requireSelection(selection2) {
  return __async(this, null, function* () {
    const [cards, groups] = yield Promise.all([dispatchDomain2("character.list", {}), dispatchDomain2("group.list", {})]);
    requireChatSelection(selection2, cards, groups);
  });
}
function beforeChatCreate(event) {
  return __async(this, null, function* () {
    if (event.eventName !== "before_create") throw new Error("Character chat lifecycle requires before_create");
    return initializeChatExtension(event.eventPayload, { readActiveSelection, requireSelection });
  });
}
function registerChatInitialization() {
  ToolPkg.registerChatLifecycleHook({ id: "chat-initialization", function: beforeChatCreate });
}

// src/runtime-tools/policy.ts
function requirePolicyOwner(registeredOwner) {
  if (registeredOwner !== CHARACTER_CARDS_NAMESPACE) throw new Error("Tool policy requires this plugin's authenticated registered owner");
}
function executionContext(value, registeredOwner) {
  requirePolicyOwner(registeredOwner);
  const context = record(value, "executionContext"), fields5 = ["chatId", "participantId", "extensionOwner", "messageExtension"];
  const actual = Object.keys(context);
  if (actual.some(
    /** Rejects complete plugin maps and undocumented alternative snapshot fields. */
    (key) => fields5.indexOf(key) === -1
  )) throw new Error("executionContext has unexpected fields");
  for (const key of fields5) {
    if (!Object.prototype.hasOwnProperty.call(context, key)) throw new Error("executionContext." + key + " is required");
  }
  for (const key of ["chatId", "participantId"]) {
    const value2 = context[key];
    if (value2 !== null && (typeof value2 !== "string" || value2.trim() === "" || value2.trim() !== value2)) throw new Error("executionContext." + key + " must be exact nonblank text or null");
  }
  const chatId = context.chatId, participantId = context.participantId;
  if (chatId !== null && typeof chatId !== "string") throw new Error("Invalid execution chatId");
  if (participantId !== null && typeof participantId !== "string") throw new Error("Invalid execution participantId");
  if (typeof context.extensionOwner !== "string" || context.extensionOwner !== registeredOwner) throw new Error("executionContext.extensionOwner does not match the authenticated registered owner");
  if (chatId === null && participantId !== null) throw new Error("A selected execution participant requires a real chat context");
  const messageExtension = context.messageExtension === null ? null : record(context.messageExtension, "executionContext.messageExtension");
  return { chatId, participantId, extensionOwner: context.extensionOwner, messageExtension };
}
function snapshotToolPolicy(context) {
  if (context.messageExtension === null) return null;
  const marker = decodeMessageMarker(context.messageExtension);
  if (context.participantId === null || marker.profile.id !== context.participantId) throw new Error("Execution participant does not match the frozen message snapshot");
  return marker.profile.toolAccess;
}
function toolAllowed(config, name, activationSource) {
  if (!config.enabled) return true;
  const sources = /* @__PURE__ */ new Set([...config.allowedPackages, ...config.allowedSkills, ...config.allowedMcpServers]);
  if (name === "search") return true;
  if (name === "proxy" || name === "package_proxy") return sources.size !== 0 || config.allowedBuiltinTools.length !== 0;
  if (name === "use_package") {
    if (activationSource === void 0) throw new Error("use_package requires its actual package_name");
    return config.allowedBuiltinTools.some(
      /** Matches the explicit builtin activation permission. */
      (tool) => tool === "use_package"
    ) && sources.has(activationSource);
  }
  const separator = name.indexOf(":");
  if (separator !== -1) {
    if (separator === 0 || separator === name.length - 1 || name.indexOf(":", separator + 1) !== -1) throw new Error("Invalid registered package tool name: " + name);
    return sources.has(name.slice(0, separator));
  }
  return config.allowedBuiltinTools.some(
    /** Matches exact builtin identities without case normalization or substring guesses. */
    (tool) => tool === name
  );
}
function toolCallPolicy(event) {
  return __async(this, null, function* () {
    if (event.eventName !== "tool_call_intercept") return;
    requirePolicyOwner(event.containerPackageName);
    if (event.eventPayload.runtimeContext === null) return;
    const policy = snapshotToolPolicy(executionContext(event.eventPayload.runtimeContext, event.containerPackageName));
    if (policy === null) return;
    let source;
    if (event.eventPayload.toolName === "use_package") {
      const values = record(event.eventPayload.parameters, "tool parameters");
      if (typeof values.package_name !== "string" || values.package_name.trim() === "") throw new Error("use_package requires its actual package_name");
      source = values.package_name;
    }
    if (toolAllowed(policy, event.eventPayload.toolName, source)) return { action: "allow" };
    return { action: "block", reason: "Selected execution participant is not allowed to access tool: " + event.eventPayload.toolName };
  });
}
function toolPromptPolicy(event) {
  return __async(this, null, function* () {
    const payload = event.eventPayload;
    if (payload.stage !== "filter_tool_call_tools" && payload.stage !== "build_tool_prompt") return;
    requirePolicyOwner(event.containerPackageName);
    const metadata = record(payload.metadata, "tool-prompt metadata");
    if (metadata.executionContext === null) return;
    const policy = snapshotToolPolicy(executionContext(metadata.executionContext, event.containerPackageName));
    if (policy === null) return;
    if (!Array.isArray(payload.availableTools)) throw new Error("Tool-prompt availableTools must be the actual host catalog");
    return { availableTools: payload.availableTools.filter(
      /** Preserves host descriptors exactly while selecting the visible registered identities. */
      (tool) => tool.name === "use_package" && tool.activationSource === void 0 ? !policy.enabled || policy.allowedBuiltinTools.some(
        /** Keeps the generic activation entry only when at least one explicit source can be activated. */
        (name) => name === "use_package"
      ) && policy.allowedPackages.length + policy.allowedSkills.length + policy.allowedMcpServers.length > 0 : toolAllowed(policy, tool.name, tool.activationSource)
    ) };
  });
}
function registerToolPolicies() {
  ToolPkg.registerToolLifecycleHook({ id: "participant-tool-execution", function: toolCallPolicy });
  ToolPkg.registerToolPromptComposeHook({ id: "participant-tool-visibility", function: toolPromptPolicy });
}

// src/group-execution/control.ts
function controlInput(value) {
  assertObject(value, "group control presentation");
  const requestId = requireId(value.requestId, "group control requestId");
  assertObject(value.input, "group control input");
  if (value.input.mode !== "group-execution" || Object.keys(value.input).length !== 2) throw new Error("Group control requires mode and chatId");
  return { requestId, chatId: requireId(value.input.chatId, "group control chatId") };
}
function checkedStatus(value, chatId, submissionId) {
  if (value === null) {
    if (submissionId !== null) throw new Error("Exact group submission returned no status");
    return null;
  }
  assertObject(value, "group execution status");
  requireId(value.submissionId, "group status submissionId");
  if (value.chatId !== chatId || submissionId !== null && value.submissionId !== submissionId) throw new Error("Group status changed its requested identity");
  if (value.status !== "running" && value.status !== "settled" && value.status !== "failed") throw new Error("Unknown group execution status");
  if (value.status === "settled" !== (value.outcome !== null)) throw new Error("Group status and settlement disagree");
  if (value.status === "failed" ? typeof value.error !== "string" : value.error !== null) throw new Error("Group failure status is malformed");
  if (value.outcome !== null && (value.outcome.chatId !== chatId || value.outcome.submissionId !== value.submissionId)) throw new Error("Group outcome changed its submission identity");
  return value;
}
function createGroupControl(requestId, chatId, publish, assertOwner) {
  let state = { value: null, busy: false, loaded: false, error: "", finished: false }, loading = null;
  function update2(change) {
    state = { ...state, ...change };
    publish(state);
  }
  function idle() {
    assertOwner();
    if (state.finished || state.busy) throw new Error("Group control is closed or already processing an action");
  }
  function request(action, submissionId) {
    return __async(this, null, function* () {
      idle();
      update2({ busy: true, error: "" });
      try {
        const payload = action === "current" ? { action, chatId } : { action, chatId, submissionId };
        const value = yield ToolPkg.ipc.call("character-memory.group-execution", payload, { targetRuntime: "main" });
        assertOwner();
        update2({ value: checkedStatus(value, chatId, submissionId), loaded: true });
      } catch (failure2) {
        update2({ error: String(failure2) });
        throw failure2;
      } finally {
        update2({ busy: false });
      }
    });
  }
  return {
    /** Retains the initial load Promise including its original rejection. */
    load() {
      if (loading === null) loading = request("current", null);
      return loading;
    },
    /** Reads current state only when the user explicitly asks to refresh. */
    refresh() {
      return request("current", null);
    },
    /** Requires the precise displayed state before issuing cancellation or resumption. */
    act(action) {
      idle();
      const value = state.value;
      if (state.error !== "" || value === null) throw new Error("Group controls require a successful status read");
      if (action === "cancel" && value.status !== "running") throw new Error("Only a running submission can be cancelled");
      if (action === "resume" && (value.status !== "settled" || value.outcome === null || value.outcome.status !== "cancelled")) throw new Error("Only a settled cancelled submission can resume");
      return request(action, value.submissionId);
    },
    /** Emits cancellation of the modal only, not cancellation of the group submission. */
    close() {
      idle();
      update2({ finished: true });
      return { type: "toolpkg.presentation.cancel", requestId };
    }
  };
}
function renderGroupExecutionScreen(ctx) {
  const [presentation] = ctx.useState("presentation", null), input = controlInput(presentation);
  const [state, publish] = ctx.useState("group-control-state", { value: null, busy: false, loaded: false, error: "", finished: false });
  const owner2 = ctx.useRef("group-control-owner", JSON.stringify(presentation));
  const controller2 = ctx.useRef("group-control-controller", null);
  function assertOwner() {
    const [current] = ctx.useState("presentation", null);
    if (JSON.stringify(current) !== owner2.current) throw new Error("Group control presentation owner changed");
  }
  assertOwner();
  if (controller2.current === null) controller2.current = createGroupControl(input.requestId, input.chatId, publish, assertOwner);
  const control = controller2.current, value = state.value;
  const labels = { running: "\u6B63\u5728\u6267\u884C", settled: "\u5DF2\u7ED3\u675F", failed: "\u6267\u884C\u5931\u8D25" };
  const outcomes = { completed: "\u5DF2\u5B8C\u6210", cancelled: "\u5DF2\u53D6\u6D88\uFF0C\u53EF\u7EE7\u7EED", blocked: "\u53D1\u9001\u88AB\u963B\u6B62", consumed: "\u53D1\u9001\u88AB\u5176\u4ED6\u5904\u7406\u5668\u63A5\u7BA1", not_persisted: "\u6D88\u606F\u672A\u4FDD\u5B58" };
  const enabled = state.loaded && !state.busy && !state.finished && state.error === "";
  return ctx.UI.Dialog({
    key: "group-execution-dialog",
    closeOnDismissRequest: false,
    properties: { dismissOnBackPress: !state.busy, dismissOnClickOutside: !state.busy },
    /** Loads the actual retained state once when this modal mounts. */
    onLoad: () => control.load(),
    /** Closes this modal without changing the underlying submission. */
    onDismissRequest: () => control.close()
  }, ctx.UI.Column({ fillMaxWidth: true, paddingHorizontal: 20, paddingVertical: 16 }, [
    ctx.UI.Text({ text: "\u7FA4\u7EC4\u6267\u884C", style: "titleMedium" }),
    ctx.UI.Text({ key: "group-execution-status", text: !state.loaded ? "\u6B63\u5728\u8BFB\u53D6\u6267\u884C\u72B6\u6001\u2026" : value === null ? "\u5F53\u524D\u4F1A\u8BDD\u6CA1\u6709\u5DF2\u63D0\u4EA4\u7684\u7FA4\u7EC4\u4EFB\u52A1" : labels[value.status], paddingVertical: 12 }),
    ...value === null ? [] : [
      ctx.UI.Text({ text: "\u63D0\u4EA4\uFF1A" + value.submissionId }),
      ...value.outcome === null ? [] : [ctx.UI.Text({ text: outcomes[value.outcome.status] + " \xB7 " + value.outcome.cursor + "/" + value.outcome.plannedTurns })],
      ...value.error === null ? [] : [ctx.UI.Text({ text: value.error, color: ctx.MaterialTheme.colorScheme.error })]
    ],
    ...state.error === "" ? [] : [ctx.UI.Text({ key: "group-execution-error", text: state.error, color: ctx.MaterialTheme.colorScheme.error })],
    ...state.busy ? [ctx.UI.CircularProgressIndicator({ width: 20, height: 20 })] : [],
    ctx.UI.Row({ fillMaxWidth: true }, [
      ctx.UI.TextButton({
        key: "group-execution-refresh",
        enabled: !state.busy && !state.finished,
        /** Requests a new status snapshot only for this explicit refresh click. */
        onClick: () => control.refresh()
      }, ctx.UI.Text({ text: "\u5237\u65B0" })),
      ctx.UI.TextButton({
        key: "group-execution-cancel",
        enabled: enabled && value !== null && value.status === "running",
        /** Cancels only the exact running submission currently displayed. */
        onClick: () => control.act("cancel")
      }, ctx.UI.Text({ text: "\u53D6\u6D88\u6267\u884C" })),
      ctx.UI.TextButton({
        key: "group-execution-resume",
        enabled: enabled && value !== null && value.status === "settled" && value.outcome !== null && value.outcome.status === "cancelled",
        /** Resumes only the exact cancelled submission without resending its user input. */
        onClick: () => control.act("resume")
      }, ctx.UI.Text({ text: "\u7EE7\u7EED\u6267\u884C" }))
    ]),
    ctx.UI.IconButton({
      key: "group-execution-close",
      enabled: !state.busy && !state.finished,
      /** Dismisses this presentation and leaves execution unchanged. */
      onClick: () => control.close()
    }, ctx.UI.Icon({ name: "Close", contentDescription: "\u5173\u95ED" }))
  ]));
}

// src/group-execution/planner.ts
var GROUP_ROLE_RESPONSE_PLANNER_PROMPT = '\u4F60\u662F\u7FA4\u804A\u89D2\u8272\u53D1\u8A00\u89C4\u5212\u5668\u3002\u53EA\u8FD4\u56DE\u6709\u6548\u7684 JSON\u3002\n\u4EFB\u52A1\uFF1A\u89C4\u5212\u672C\u8F6E\u7684\u53D1\u8A00\u987A\u5E8F\u3002\u4F60\u53EF\u4EE5\u89C4\u5212\u591A\u8F6E\u5BF9\u8BDD\u3002\n\u8F93\u51FA\u683C\u5F0F\uFF1A\n{"rounds":[[{"id":"<\u6210\u5458ID>","speak":true}],[{"id":"<\u6210\u5458ID2>","speak":true}]]}\n\u89C4\u5219\uFF1A\n- \u6BCF\u4E00\u8F6E\uFF08round\uFF09\u662F\u4E00\u4E2A\u6570\u7EC4\uFF0C\u5305\u542B\u8BE5\u8F6E\u5E94\u8BE5\u53D1\u8A00\u7684\u6210\u5458\u3002\n- \u4F60\u53EF\u4EE5\u89C4\u5212\u591A\u8F6E\u5BF9\u8BDD\uFF0C\u8BA9\u6210\u5458\u4E4B\u95F4\u76F8\u4E92\u8BA8\u8BBA\u3002\n- \u5BF9\u4E8E\u7B80\u5355\u56DE\u5E94\uFF0C\u4F7F\u7528\u5355\u8F6E\uFF0C\u5305\u542B\u4E00\u4E2A\u6216\u591A\u4E2A\u6210\u5458\u3002\n- \u5BF9\u4E8E\u8BA8\u8BBA\u573A\u666F\uFF0C\u4F7F\u7528\u591A\u8F6E\uFF08\u4F8B\u5982\uFF1A\u6210\u5458A\u53D1\u8A00\uFF0C\u7136\u540E\u6210\u5458B\u56DE\u5E94\uFF0C\u7136\u540E\u6210\u5458A\u518D\u56DE\u590D\uFF09\u3002\n- \u4F60\u53EF\u4EE5\u7701\u7565\u6210\u5458\u6765\u8DF3\u8FC7\u4ED6\u4EEC\uFF0C\u6216\u8BBE\u7F6E speak=false\u3002\n- \u5982\u679C\u6CA1\u6709\u4EBA\u5E94\u8BE5\u56DE\u5E94\uFF0C\u8FD4\u56DE {"rounds":[[]]}\u3002\n- \u53EA\u4F7F\u7528\u63D0\u4F9B\u7684\u6210\u5458 ID\u3002\n- \u6700\u591A 5 \u8F6E\uFF0C\u907F\u514D\u8FC7\u5EA6\u6765\u56DE\u3002';
function memberIds(participants) {
  if (participants.length === 0) throw new Error("Group planner requires actual participants");
  const ids = /* @__PURE__ */ new Set();
  for (const participant of participants) {
    requireId(participant.id, "planner participant id");
    requireId(participant.name, "planner participant name");
    if (ids.has(participant.id)) throw new Error("Duplicate planner participant: " + participant.id);
    ids.add(participant.id);
  }
  return ids;
}
function buildGroupPlannerPrompt(participants, userText) {
  memberIds(participants);
  if (typeof userText !== "string") throw new Error("Planner user text must be a string");
  const lines2 = participants.map(
    /** Publishes exactly the real ID and display name supplied by the plugin's ordered domain read. */
    (participant) => "- id: " + participant.id + ", name: " + participant.name
  );
  return GROUP_ROLE_RESPONSE_PLANNER_PROMPT + "\n\u6210\u5458\u5217\u8868\uFF1A\n" + lines2.join("\n") + "\n\n\u7528\u6237\u6D88\u606F\uFF1A\n" + userText;
}
function object(value, keys, path) {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error(path + " must be an object");
  const result2 = value, actual = Object.keys(result2);
  if (actual.length !== keys.length || actual.some(
    /** Rejects alternate containers, names and extra speculative planner fields. */
    (key) => keys.indexOf(key) < 0
  )) throw new Error(path + " has unexpected or missing fields");
  return result2;
}
function parseGroupResponsePlan(raw, participants) {
  const ids = memberIds(participants), value = JSON.parse(raw), root = object(value, ["rounds"], "group plan");
  if (!Array.isArray(root.rounds) || root.rounds.length === 0 || root.rounds.length > 5) throw new Error("Group plan requires one to five explicit rounds");
  const rounds = [];
  for (let roundIndex = 0; roundIndex < root.rounds.length; roundIndex++) {
    const source = root.rounds[roundIndex];
    if (!Array.isArray(source)) throw new Error("Group plan round must be an array");
    const round = [], seen = /* @__PURE__ */ new Set();
    for (const sourceEntry of source) {
      const entry = object(sourceEntry, ["id", "speak"], "group plan entry"), id2 = requireId(entry.id, "planned participant id");
      if (!ids.has(id2)) throw new Error("Unknown planned group participant: " + id2);
      if (seen.has(id2)) throw new Error("Duplicate participant in one group round: " + id2);
      if (typeof entry.speak !== "boolean") throw new Error("Group plan speak must be a boolean");
      seen.add(id2);
      round.push({ id: id2, speak: entry.speak });
    }
    rounds.push(round);
  }
  return { rounds };
}
function planGroupResponse(participants, userText) {
  return __async(this, null, function* () {
    const content = buildGroupPlannerPrompt(participants, userText);
    const result2 = yield Tools.Chat.call({ functionType: "ROLE_RESPONSE_PLANNER", turns: [{ kind: "user", content }], recordTokenUsage: true, enableThinking: false });
    if (typeof result2.text !== "string") throw new Error("Group planner AI result has no real assistant text");
    return parseGroupResponsePlan(result2.text, participants);
  });
}

// src/group-execution/contracts.ts
function object2(value, path) {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error(path + " must be an object");
  return value;
}
function fields4(value, expected) {
  const keys = Object.keys(value).sort();
  if (keys.join(",") !== [...expected].sort().join(",")) throw new Error("Native send outcome has unexpected or missing fields");
}
function assistant(value) {
  const record2 = object2(value, "Native assistant locator");
  fields4(record2, ["messageTimestamp", "variantIndex"]);
  assertInteger(record2.messageTimestamp, "native assistant timestamp", 0);
  assertInteger(record2.variantIndex, "native assistant variant", 0);
  if (record2.variantIndex > 2147483647) throw new Error("Native assistant variant exceeds the host i32 range");
  return { messageTimestamp: record2.messageTimestamp, variantIndex: record2.variantIndex };
}
function status(value) {
  if (value !== "completed" && value !== "cancelled") throw new Error("Native send status must be completed or cancelled");
  return value;
}
function decodeNativeGroupSendResult(value, input, previousTimestamp) {
  const result2 = object2(value, "Native send result"), initial = "kind" in input;
  const chatId = initial ? input.submission.chatId : input.chatId;
  const requestKey = initial && input.kind === "record_only" ? input.requestKey : input.turn.requestKey;
  if (result2.chatId !== chatId) throw new Error("Native send result belongs to a different chat");
  if (typeof result2.message !== "string") throw new Error("Native send result must retain its real submitted text");
  assertInteger(result2.sentAt, "native send sentAt", 0);
  const outcome = object2(result2.outcome, "Native send outcome");
  switch (outcome.type) {
    case "committed": {
      fields4(outcome, ["type", "status", "userMessageTimestamp", "assistant"]);
      const terminal = status(outcome.status);
      let userMessageTimestamp;
      if (outcome.userMessageTimestamp === null) userMessageTimestamp = null;
      else {
        const timestamp = outcome.userMessageTimestamp;
        assertInteger(timestamp, "native user message timestamp", 1);
        userMessageTimestamp = timestamp;
      }
      if (previousTimestamp !== null && userMessageTimestamp !== null && userMessageTimestamp !== previousTimestamp) throw new Error("Group continuation created or referenced a different user message");
      const located = outcome.assistant === null ? null : assistant(outcome.assistant);
      if (terminal === "completed" && userMessageTimestamp === null) throw new Error("Completed group commit requires the actual user message timestamp");
      if (located !== null && userMessageTimestamp === null) throw new Error("Group assistant commit has no actual originating user message");
      if (initial && input.kind === "record_only" && located !== null) throw new Error("User-only group commit cannot declare an assistant reply");
      if (!(initial && input.kind === "record_only") && terminal === "completed" && located === null) throw new Error("Completed group execution requires an actual assistant locator");
      return { type: "committed", receipt: { status: terminal, chatId, requestKey, userMessageTimestamp, assistant: located } };
    }
    case "not_persisted":
      fields4(outcome, ["type", "status"]);
      return { type: "not_persisted", status: status(outcome.status) };
    case "blocked": {
      fields4(outcome, ["type", "message"]);
      if (outcome.message !== null && typeof outcome.message !== "string") throw new Error("Blocked send message must be explicit text or null");
      return { type: "blocked", message: outcome.message };
    }
    case "consumed": {
      fields4(outcome, ["type", "metadata"]);
      const metadata = jsonValue(outcome.metadata, "native consumed metadata");
      if (metadata === null || typeof metadata !== "object" || Array.isArray(metadata)) throw new Error("Consumed send metadata must be a JSON object");
      return { type: "consumed", metadata };
    }
    default:
      throw new Error("Unknown native send outcome type");
  }
}

// src/group-execution/executor.ts
function canonical(value) {
  if (value === null || typeof value !== "object") return JSON.stringify(value);
  const parts = [];
  if (Array.isArray(value)) {
    for (const item of value) parts.push(canonical(item));
    return "[" + parts.join(",") + "]";
  }
  for (const key of Object.keys(value).sort()) parts.push(JSON.stringify(key) + ":" + canonical(value[key]));
  return "{" + parts.join(",") + "}";
}
function attachment(value) {
  if (!Number.isSafeInteger(value.fileSize) || value.fileSize < 0) throw new Error("Group attachment size must be a nonnegative safe integer");
  return { ...value };
}
function validateGroupSubmission(value) {
  requireId(value.selection, "group selection");
  parseChatSelection(value.selection);
  requireId(value.submissionId, "group submission id");
  requireId(value.chatId, "group chat id");
  const attachments = [];
  for (const item of value.attachments) attachments.push(attachment(item));
  if (value.text.trim().length === 0 && attachments.length === 0) throw new Error("Group submission requires actual text or attachments");
  if (value.replyToMessageTimestamp !== null && (!Number.isSafeInteger(value.replyToMessageTimestamp) || value.replyToMessageTimestamp < 1)) {
    throw new Error("Group reply target must be a positive safe integer timestamp");
  }
  return { submissionId: value.submissionId, chatId: value.chatId, selection: value.selection, runtime: value.runtime, notifyReply: value.notifyReply, text: value.text, attachments, replyToMessageTimestamp: value.replyToMessageTimestamp };
}
var GroupExecutionFailures = class extends Error {
  /** Exposes both original error objects rather than rewriting or discarding either failure. */
  constructor(executionFailure, cancellationFailure) {
    super("Group execution and cancellation both failed");
    this.executionFailure = executionFailure;
    this.cancellationFailure = cancellationFailure;
    this.name = "GroupExecutionFailures";
  }
};
var GroupExecutionReleaseFailures = class extends Error {
  /** Preserves both exact failure objects without replacing either with a manufactured completion result. */
  constructor(executionFailure, releaseFailure) {
    super("Group execution and native sequence release both failed");
    this.executionFailure = executionFailure;
    this.releaseFailure = releaseFailure;
    this.name = "GroupExecutionReleaseFailures";
  }
};
var GroupExecutionController = class {
  /** Accepts only explicit domain and transport dependencies; it never creates a default or partially working backend. */
  constructor(dependencies) {
    this.dependencies = dependencies;
    this.sessions = /* @__PURE__ */ new Map();
    this.chats = /* @__PURE__ */ new Map();
  }
  /** Starts one retained submission and rejects a changed payload or competing sequence before doing any work. */
  submit(input) {
    const saved = validateGroupSubmission(input), key = JSON.stringify([saved.chatId, saved.submissionId]), inputKey = canonical(jsonValue(saved));
    const existing = this.sessions.get(key);
    if (existing !== void 0) {
      if (existing.inputKey !== inputKey) throw new Error("Duplicate group submission id has a different input");
      if (existing.retained === null) throw new Error("Group submission retained result was not initialized");
      return existing.retained;
    }
    if (this.chats.has(saved.chatId)) throw new Error("Another group sequence already owns this chat");
    const session = {
      input: saved,
      inputKey,
      selection: null,
      speakers: [],
      planned: false,
      transportStarted: false,
      finalizationAttempted: false,
      cursor: 0,
      attempt: 0,
      userMessageTimestamp: null,
      receipts: [],
      disposition: null,
      phase: "planning",
      cancelled: false,
      activeRequestKey: null,
      cancellation: null,
      retained: null
    };
    this.sessions.set(key, session);
    this.chats.set(saved.chatId, key);
    session.retained = Promise.resolve().then(
      /** Starts after retaining identity so two synchronous submits cannot both begin planning. */
      () => this.run(session, true)
    );
    return session.retained;
  }
  /** Records cancellation and invokes only the active correlated turn's real cancellation capability. */
  cancel(chatId, submissionId) {
    const session = this.session(chatId, submissionId);
    if (session.phase === "completed" || session.phase === "failed" || session.phase === "finishing" || session.phase === "stopped") throw new Error("Group sequence is no longer cancellable");
    session.cancelled = true;
    if (session.cancellation !== null) return session.cancellation;
    if (session.activeRequestKey === null) return Promise.resolve();
    const requestKey = session.activeRequestKey;
    session.cancellation = Promise.resolve().then(
      /** Retains synchronous and asynchronous host cancellation failures identically without retrying the request. */
      () => this.dependencies.transport.cancel({ chatId, requestKey, runtime: session.input.runtime })
    );
    return session.cancellation;
  }
  /** Resumes only a settled cancelled cursor without rerunning the planner or submitting the original user input twice. */
  resume(chatId, submissionId) {
    const session = this.session(chatId, submissionId);
    if (session.phase !== "cancelled" || session.activeRequestKey !== null) throw new Error("Only a settled cancelled group sequence can resume");
    session.cancelled = false;
    session.cancellation = null;
    session.phase = session.planned ? "running" : "planning";
    session.retained = Promise.resolve().then(
      /** Reuses the acknowledged cursor and calls the planner only when cancellation prevented its first invocation. */
      () => this.run(session, !session.planned)
    );
    return session.retained;
  }
  /** Requires an exact retained session and never reconstructs one from current selection or history guesses. */
  session(chatId, submissionId) {
    requireId(chatId, "group chat id");
    requireId(submissionId, "group submission id");
    const session = this.sessions.get(JSON.stringify([chatId, submissionId]));
    if (session === void 0) throw new Error("Group submission does not exist in this runtime");
    return session;
  }
  /** Rechecks actual binding and member references before every turn, including a cancelled turn's resumed execution. */
  current(session, participantId) {
    return __async(this, null, function* () {
      const selection2 = yield this.dependencies.readSelection(session.input.chatId), parsed = parseChatSelection(selection2);
      if (parsed.kind !== "group") throw new Error("Group execution requires a real group selection");
      if (session.input.selection !== selection2) throw new Error("Submitted group selection differs from the actual conversation binding");
      if (session.selection === null) session.selection = selection2;
      else if (session.selection !== selection2) throw new Error("Chat selection changed during group execution");
      const participants = yield this.dependencies.readParticipants(selection2);
      if (participantId !== null) {
        let matches = 0;
        for (const participant of participants) if (participant.id === participantId) matches++;
        if (matches !== 1) throw new Error("Planned group participant was deleted or removed: " + participantId);
      }
    });
  }
  /** Produces a progress copy from real receipts and the retained cursor, including explicit cancellation before persistence. */
  outcome(session, status2) {
    if (session.selection === null) throw new Error("Group outcome has no authenticated selection");
    const receipts = [];
    for (const item of session.receipts) receipts.push({ ...item, assistant: item.assistant === null ? null : { ...item.assistant } });
    return {
      status: status2,
      disposition: session.disposition,
      submissionId: session.input.submissionId,
      chatId: session.input.chatId,
      selection: session.selection,
      cursor: session.cursor,
      plannedTurns: session.speakers.length,
      userMessageTimestamp: session.userMessageTimestamp,
      receipts
    };
  }
  /** Plans once, executes serially, preserves genuine cancellation receipts and propagates every original failure without retry. */
  run(session, plan) {
    return __async(this, null, function* () {
      try {
        yield this.current(session, null);
        if (session.selection === null) throw new Error("Group execution selection is not initialized");
        if (session.cancelled) {
          session.phase = "cancelled";
          return this.outcome(session, "cancelled");
        }
        if (plan) {
          const participants = yield this.dependencies.readParticipants(session.selection);
          if (session.cancelled) {
            session.phase = "cancelled";
            return this.outcome(session, "cancelled");
          }
          const planned = yield this.dependencies.plan(participants, session.input.text);
          const checked = parseGroupResponsePlan(JSON.stringify(planned), participants);
          for (const round of checked.rounds) for (const entry of round) if (entry.speak) session.speakers.push(entry.id);
          session.planned = true;
        }
        session.phase = "running";
        while (true) {
          if (session.cancelled) {
            session.phase = "cancelled";
            return this.outcome(session, "cancelled");
          }
          if (session.cursor >= session.speakers.length && session.userMessageTimestamp !== null) break;
          const participantId = session.cursor < session.speakers.length ? session.speakers[session.cursor] : null;
          yield this.current(session, participantId);
          const requestKey = JSON.stringify([session.input.chatId, session.input.submissionId, session.cursor, session.attempt++]);
          const selected = participantId === null ? null : { requestKey, selection: session.selection, participantId, notifyReply: session.input.notifyReply && session.cursor === session.speakers.length - 1 };
          if (session.cancelled) {
            session.phase = "cancelled";
            return this.outcome(session, "cancelled");
          }
          session.activeRequestKey = requestKey;
          let requested;
          let value;
          session.transportStarted = true;
          if (session.userMessageTimestamp === null) {
            requested = selected === null ? { kind: "record_only", submission: session.input, requestKey } : { kind: "execute", submission: session.input, turn: selected };
            value = yield this.dependencies.transport.submit(requested);
          } else {
            if (selected === null) throw new Error("A group continuation must identify an actual planned speaker");
            requested = {
              submissionId: session.input.submissionId,
              chatId: session.input.chatId,
              runtime: session.input.runtime,
              userMessageTimestamp: session.userMessageTimestamp,
              turn: selected
            };
            value = yield this.dependencies.transport.continue(requested);
          }
          const decision = decodeNativeGroupSendResult(value, requested, session.userMessageTimestamp);
          session.activeRequestKey = null;
          if (session.cancellation !== null) yield session.cancellation;
          if (decision.type !== "committed") {
            session.disposition = decision;
            session.finalizationAttempted = true;
            yield this.dependencies.transport.abandon({ submissionId: session.input.submissionId, chatId: session.input.chatId, runtime: session.input.runtime });
            session.phase = "stopped";
            this.chats.delete(session.input.chatId);
            return this.outcome(session, decision.type);
          }
          const accepted = decision.receipt;
          if (accepted.userMessageTimestamp !== null) session.userMessageTimestamp = accepted.userMessageTimestamp;
          session.receipts.push(accepted);
          if (accepted.status === "completed" && participantId !== null) session.cursor++;
          if (accepted.status === "cancelled" || session.cancelled) {
            session.phase = "cancelled";
            return this.outcome(session, "cancelled");
          }
        }
        if (session.userMessageTimestamp === null) throw new Error("Completed group sequence has no actual persisted user message");
        session.phase = "finishing";
        session.finalizationAttempted = true;
        yield this.dependencies.transport.finish({
          submissionId: session.input.submissionId,
          chatId: session.input.chatId,
          runtime: session.input.runtime,
          userMessageTimestamp: session.userMessageTimestamp,
          receipts: this.outcome(session, "completed").receipts
        });
        session.phase = "completed";
        this.chats.delete(session.input.chatId);
        return this.outcome(session, "completed");
      } catch (error) {
        session.phase = "failed";
        session.activeRequestKey = null;
        this.chats.delete(session.input.chatId);
        let failure2 = error;
        if (session.cancellation !== null) {
          try {
            yield session.cancellation;
          } catch (cancellationError) {
            if (cancellationError !== error) failure2 = new GroupExecutionFailures(error, cancellationError);
          }
        }
        if (session.transportStarted && !session.finalizationAttempted) {
          session.finalizationAttempted = true;
          try {
            yield this.dependencies.transport.abandon({ submissionId: session.input.submissionId, chatId: session.input.chatId, runtime: session.input.runtime });
          } catch (releaseError) {
            throw new GroupExecutionReleaseFailures(failure2, releaseError);
          }
        }
        throw failure2;
      }
    });
  }
};

// src/group-execution/domain-adapter.ts
function createGroupExecutionController(service, transport) {
  return new GroupExecutionController({
    /** Reads only the actual conversation's owner-isolated namespace, never global selection or a file mirror. */
    readSelection(chatId) {
      return __async(this, null, function* () {
        return (yield service.dispatchDomain("chat.configuration.binding.read", { chatId })).selection;
      });
    },
    /** Requires real saved group members in their explicit domain order before planning or executing. */
    readParticipants(selection2) {
      return __async(this, null, function* () {
        const parsed = parseChatSelection(selection2);
        if (parsed.kind !== "group") throw new Error("Group planner requires a group selection");
        return orderedGroupParticipants(yield service.dispatchDomain("group.get", { id: parsed.id }), service);
      });
    },
    plan: planGroupResponse,
    transport
  });
}

// src/group-execution/native-transport.ts
function sequenceKey(chatId, submissionId) {
  return JSON.stringify([chatId, submissionId]);
}
function verifySnapshot(chatId, selected, assistant2) {
  return __async(this, null, function* () {
    const extension = yield Tools.Chat.readExtension({ kind: "message", chatId, messageTimestamp: assistant2.messageTimestamp, variantIndex: assistant2.variantIndex });
    if (extension === null) throw new Error("Committed group assistant has no participant message extension");
    const marker = decodeMessageMarker(extension);
    if (marker.selection !== selected.selection || marker.profile.id !== selected.participantId || marker.promptFunctionType !== "CHAT") {
      throw new Error("Committed group assistant snapshot identifies a different participant or selection");
    }
  });
}
var NativeGroupTurnTransport = class {
  constructor() {
    this.sequences = /* @__PURE__ */ new Map();
    this.attempts = /* @__PURE__ */ new Map();
  }
  /** Sends the complete original input once, or explicitly records it without requesting an arbitrary participant. */
  submit(input) {
    return __async(this, null, function* () {
      const submitted = input.submission, key = sequenceKey(submitted.chatId, submitted.submissionId), prior = this.sequences.get(key);
      if (prior !== void 0 && (prior.last === null || prior.last.type !== "committed" || prior.last.receipt.status !== "cancelled" || prior.userTimestamp !== null)) {
        throw new Error("A second initial submit requires explicitly resumed cancellation before user-message persistence");
      }
      const sequence = {
        chatId: submitted.chatId,
        runtime: submitted.runtime,
        notifyReply: submitted.notifyReply,
        last: null,
        userTimestamp: null,
        assistants: [],
        finished: false
      };
      this.sequences.set(key, sequence);
      const request = {
        kind: "submit",
        chatId: submitted.chatId,
        runtime: submitted.runtime,
        notifyReply: input.kind === "record_only" ? false : input.turn.notifyReply,
        input: { text: submitted.text, attachments: submitted.attachments, replyToMessageTimestamp: submitted.replyToMessageTimestamp },
        turn: input.kind === "record_only" ? { kind: "record_only" } : { kind: "execute", participantId: input.turn.participantId }
      };
      return this.run(sequence, input, request);
    });
  }
  /** Refers to the actual committed user record and never resubmits its text, attachments or reply target. */
  continue(input) {
    return __async(this, null, function* () {
      const sequence = this.requireSequence(input.chatId, input.submissionId, input.runtime);
      if (sequence.finished || sequence.userTimestamp !== input.userMessageTimestamp) throw new Error("Group continuation requires its acknowledged user timestamp");
      return this.run(sequence, input, {
        kind: "continue",
        chatId: input.chatId,
        runtime: input.runtime,
        userMessageTimestamp: input.userMessageTimestamp,
        participantId: input.turn.participantId,
        notifyReply: input.turn.notifyReply
      });
    });
  }
  /** Cancels only a currently correlated local attempt through the host's authenticated owner/chat capture. */
  cancel(input) {
    return __async(this, null, function* () {
      const attempt = this.attempts.get(input.requestKey);
      if (attempt === void 0 || attempt.chatId !== input.chatId || attempt.runtime !== input.runtime || !attempt.active) throw new Error("Cancellation has no matching active send attempt");
      const result2 = yield Tools.Chat.cancel(input.chatId);
      if (result2.chatId !== input.chatId) throw new Error("Native cancellation identifies a different conversation");
      assertBoolean(result2.cancelRequested, "native cancelRequested");
    });
  }
  /** Checks plugin sequence completion against real per-send receipts after host finalization has already finished. */
  finish(input) {
    return __async(this, null, function* () {
      const sequence = this.requireSequence(input.chatId, input.submissionId, input.runtime);
      if (sequence.finished || sequence.userTimestamp !== input.userMessageTimestamp || sequence.last === null || sequence.last.type !== "committed" || sequence.last.receipt.status !== "completed") {
        throw new Error("Group completion does not match its actual completed sends");
      }
      const assistants = input.receipts.flatMap(
        /** Counts only actual assistant locators acknowledged by the controller. */
        (receipt) => receipt.assistant === null ? [] : [receipt.assistant]
      );
      if (assistants.length !== sequence.assistants.length) throw new Error("Group completion changed its acknowledged assistant count");
      for (let index = 0; index < assistants.length; index++) {
        const actual = assistants[index], expected = sequence.assistants[index];
        if (actual.messageTimestamp !== expected.messageTimestamp || actual.variantIndex !== expected.variantIndex) throw new Error("Group completion changed assistant order or revision identity");
      }
      sequence.finished = true;
    });
  }
  /** Ends plugin-local planning state; accepted native sends clean up even when plugin code stops observing. */
  abandon(input) {
    return __async(this, null, function* () {
      this.requireSequence(input.chatId, input.submissionId, input.runtime).finished = true;
    });
  }
  /** Requires the exact retained local submission rather than selecting another conversation's state. */
  requireSequence(chatId, submissionId, runtime2) {
    const sequence = this.sequences.get(sequenceKey(chatId, submissionId));
    if (sequence === void 0 || sequence.chatId !== chatId || sequence.runtime !== runtime2) throw new Error("Group submission has no matching send sequence");
    return sequence;
  }
  /** Awaits one real finalized send, validates its originating receipt and checks the native-authored snapshot. */
  run(sequence, input, request) {
    return __async(this, null, function* () {
      const requestKey = "kind" in input && input.kind === "record_only" ? input.requestKey : input.turn.requestKey;
      if (this.attempts.has(requestKey)) throw new Error("Group request correlation key was already used");
      const attempt = { chatId: sequence.chatId, runtime: sequence.runtime, active: true };
      this.attempts.set(requestKey, attempt);
      try {
        const actual = yield Tools.Chat.sendMessage(request);
        const decision = decodeNativeGroupSendResult(actual, input, "kind" in input ? null : input.userMessageTimestamp);
        sequence.last = decision;
        if (decision.type === "committed") {
          if (decision.receipt.userMessageTimestamp !== null) {
            assertInteger(decision.receipt.userMessageTimestamp, "native user timestamp", 1);
            sequence.userTimestamp = decision.receipt.userMessageTimestamp;
          }
          if (decision.receipt.assistant !== null) {
            sequence.assistants.push({ ...decision.receipt.assistant });
            if ("kind" in input && input.kind === "record_only") throw new Error("Record-only native send committed an assistant");
            yield verifySnapshot(sequence.chatId, input.turn, decision.receipt.assistant);
          }
        }
        return actual;
      } finally {
        attempt.active = false;
      }
    });
  }
};

// src/group-execution/hooks.ts
var controller = null;
var submissionSequence = 0;
var executions = /* @__PURE__ */ new Map();
function submission(payload, chatId, selection2) {
  const { runtime: runtime2, text: text3, attachments } = payload;
  if (runtime2 === null) throw new Error("Group input requires an actual send-capable runtime slot");
  if (text3 === void 0) throw new Error("Group input requires the submitted text field");
  if (attachments === null) throw new Error("Group input requires complete native attachments");
  if (payload.attachmentCount !== attachments.length || payload.hasAttachments !== (attachments.length !== 0)) {
    throw new Error("Group input attachment counts differ from the complete native input");
  }
  return {
    submissionId: "group-input-" + ++submissionSequence,
    chatId,
    selection: selection2,
    runtime: runtime2,
    notifyReply: payload.notifyReply,
    text: text3,
    attachments,
    replyToMessageTimestamp: payload.replyToMessageTimestamp
  };
}
function observe(execution, work) {
  void work.then(
    /** Publishes only the controller's validated result and precise native message locators. */
    (outcome) => {
      execution.status = "settled";
      execution.outcome = outcome;
    },
    /** Retains the original rejected error and exposes the background failure through the plugin log. */
    (failure2) => {
      execution.status = "failed";
      execution.failure = failure2;
      NativeInterface.logError("Group input " + execution.input.submissionId + " failed: " + String(failure2));
    }
  );
}
function onGroupInputSubmit(event) {
  return __async(this, null, function* () {
    if (event.eventName !== "submit_requested") return null;
    const payload = event.eventPayload;
    if (payload.source === "Sequence") return null;
    const chatId = requireId(payload.chatId, "submitted chatId");
    const extension = yield Tools.Chat.readExtension({ kind: "chat", chatId });
    if (extension === null) return null;
    const binding = decodeChatMarker(extension);
    if (parseChatSelection(binding.selection).kind !== "group") return null;
    const service = yield getService();
    const input = submission(payload, chatId, binding.selection);
    const current = executions.get(chatId);
    if (current !== void 0 && (current.status === "running" || current.outcome !== null && current.outcome.status === "cancelled")) {
      return { action: "Block", clearInput: false, message: "An existing group submission must settle or resume before another input", metadata: { submissionId: current.input.submissionId } };
    }
    if (controller === null) controller = createGroupExecutionController(service, new NativeGroupTurnTransport());
    const work = controller.submit(input);
    const execution = { input, status: "running", outcome: null, failure: null };
    executions.set(chatId, execution);
    observe(execution, work);
    return { action: "Consume", clearInput: true, metadata: { submissionId: input.submissionId, chatId, selection: input.selection, runtime: input.runtime } };
  });
}
function decodeGroupExecutionRequest(payload) {
  assertObject(payload, "group execution request");
  const chatId = requireId(payload.chatId, "group execution chatId");
  const action = payload.action;
  const keys = Object.keys(payload);
  const expected = action === "current" ? ["chatId", "action"] : ["chatId", "submissionId", "action"];
  if (keys.length !== expected.length || keys.some((key) => expected.indexOf(key) < 0)) throw new Error("Group execution request has unexpected or missing fields");
  if (action === "current") return { chatId, action };
  const submissionId = requireId(payload.submissionId, "group execution submissionId");
  if (action !== "status" && action !== "cancel" && action !== "resume") throw new Error("Unknown group execution action");
  return { chatId, submissionId, action };
}
function retained(payload) {
  const execution = executions.get(payload.chatId);
  if (execution === void 0 || execution.input.submissionId !== payload.submissionId || controller === null) throw new Error("Group execution has no matching retained submission");
  return { execution, action: payload.action, controller };
}
function groupExecutionRequest(payload) {
  return __async(this, null, function* () {
    if (payload.action === "current") {
      const execution2 = executions.get(payload.chatId);
      return execution2 === void 0 ? null : jsonValue(executionStatus(execution2));
    }
    const target = retained(payload), execution = target.execution;
    if (target.action === "cancel") {
      if (execution.status !== "running") throw new Error("Only a running group submission can be cancelled");
      yield target.controller.cancel(execution.input.chatId, execution.input.submissionId);
    } else if (target.action === "resume") {
      if (execution.status !== "settled" || execution.outcome === null || execution.outcome.status !== "cancelled") throw new Error("Only a settled cancelled group submission can resume");
      const work = target.controller.resume(execution.input.chatId, execution.input.submissionId);
      execution.status = "running";
      execution.outcome = null;
      execution.failure = null;
      observe(execution, work);
    }
    return jsonValue(executionStatus(execution));
  });
}
function executionStatus(execution) {
  return {
    submissionId: execution.input.submissionId,
    chatId: execution.input.chatId,
    status: execution.status,
    outcome: execution.outcome,
    error: execution.status === "failed" ? String(execution.failure) : null
  };
}
function registerGroupExecutionHooks() {
  ToolPkg.registerChatInputHook({ id: "group-input-submit", function: onGroupInputSubmit });
  ToolPkg.ipc.on(
    "character-memory.group-execution",
    /** Decodes only messages entering through the external package IPC channel. */
    (payload) => groupExecutionRequest(decodeGroupExecutionRequest(payload))
  );
}

// src/ui-sidebar.ts
function exactFields(value, expected, label) {
  const object3 = record(value, label), keys = Object.keys(object3);
  if (keys.length !== expected.length || keys.some(
    /** Rejects every field not explicitly declared by this boundary. */
    (key) => expected.indexOf(key) === -1
  )) throw new Error(label + " has invalid fields");
  return object3;
}
function sidebarIdentity(value, label) {
  if (typeof value !== "string" || value.trim() === "") throw new Error(label + " must be a nonblank string");
  return value;
}
function parseSidebarInput(value) {
  const input = exactFields(value, ["view"], "sidebar input");
  if (input.view !== "characters" && input.view !== "groups") throw new Error("sidebar input.view must be characters or groups");
  return { view: input.view };
}
function parseSidebarChat(value) {
  const chat = exactFields(value, ["id", "title", "updatedAt", "displayOrder", "workspaceId", "workspaceName", "locked", "pinned"], "sidebar chat");
  const id2 = sidebarIdentity(chat.id, "sidebar chat.id");
  if (typeof chat.title !== "string" || typeof chat.updatedAt !== "string") throw new Error("sidebar chat title and updatedAt must be strings");
  if (typeof chat.displayOrder !== "number" || !Number.isSafeInteger(chat.displayOrder)) throw new Error("sidebar chat.displayOrder must be a safe integer");
  if (chat.workspaceId !== null && typeof chat.workspaceId !== "string") throw new Error("sidebar chat.workspaceId must be a string or null");
  if (chat.workspaceName !== null && typeof chat.workspaceName !== "string") throw new Error("sidebar chat.workspaceName must be a string or null");
  if (typeof chat.locked !== "boolean" || typeof chat.pinned !== "boolean") throw new Error("sidebar chat locked and pinned must be booleans");
  return { id: id2, title: chat.title, updatedAt: chat.updatedAt, displayOrder: chat.displayOrder, workspaceId: chat.workspaceId, workspaceName: chat.workspaceName, locked: chat.locked, pinned: chat.pinned };
}
function parseChatSidebarContext(value) {
  const context = exactFields(value, ["chats", "currentChatId", "activeStreamingChatIds"], "chatSidebar");
  if (!Array.isArray(context.chats)) throw new Error("chatSidebar.chats must be an array");
  if (!Array.isArray(context.activeStreamingChatIds)) throw new Error("chatSidebar.activeStreamingChatIds must be an array");
  const chats = [], ids = /* @__PURE__ */ new Set(), streaming = /* @__PURE__ */ new Set();
  for (const raw of context.chats) {
    const chat = parseSidebarChat(raw);
    if (ids.has(chat.id)) throw new Error("Duplicate sidebar chat: " + chat.id);
    ids.add(chat.id);
    chats.push(chat);
  }
  for (const raw of context.activeStreamingChatIds) {
    const id2 = sidebarIdentity(raw, "activeStreamingChatIds entry");
    if (streaming.has(id2)) throw new Error("Duplicate active streaming chat: " + id2);
    streaming.add(id2);
  }
  const currentChatId = context.currentChatId === null ? null : sidebarIdentity(context.currentChatId, "chatSidebar.currentChatId");
  return { chats, currentChatId, activeStreamingChatIds: [...streaming] };
}
function parseCurrentSidebar(value) {
  const current = exactFields(value, ["input", "chatSidebar"], "current sidebar");
  return { input: parseSidebarInput(current.input), chatSidebar: parseChatSidebarContext(current.chatSidebar) };
}
function compareSidebarChats(left, right) {
  return Number(right.pinned) - Number(left.pinned) || left.displayOrder - right.displayOrder || left.id.localeCompare(right.id);
}
function characterSidebarSections(directory, selections, chats) {
  const sections = [], bySelection = /* @__PURE__ */ new Map();
  for (const card of directory.cards) {
    const selection2 = encodeSelection({ CharacterCard: { id: card.id } });
    if (bySelection.has(selection2)) throw new Error("Duplicate persisted character category: " + selection2);
    const section = { id: selection2, title: card.name, avatarUri: card.avatarUri, kind: "card", selection: selection2, chats: [] };
    sections.push(section);
    bySelection.set(selection2, section);
  }
  for (const group of directory.groups) {
    const selection2 = encodeSelection({ CharacterGroup: { id: group.id } });
    if (bySelection.has(selection2)) throw new Error("Duplicate persisted role-group category: " + selection2);
    const section = { id: selection2, title: group.name, avatarUri: null, kind: "group", selection: selection2, chats: [] };
    sections.push(section);
    bySelection.set(selection2, section);
  }
  const selectionByChat = /* @__PURE__ */ new Map();
  for (const state of selections) {
    if (selectionByChat.has(state.chatId)) throw new Error("Duplicate sidebar selection state: " + state.chatId);
    if (state.selection !== null) decodeSelection(state.selection, directory);
    selectionByChat.set(state.chatId, state);
  }
  const unbound = { id: "unbound", title: "\u672A\u7ED1\u5B9A\u89D2\u8272", avatarUri: null, kind: "unbound", selection: null, chats: [] };
  for (const chat of chats) {
    const state = selectionByChat.get(chat.id);
    if (state === void 0) throw new Error("Sidebar has no explicit namespace selection state for chat: " + chat.id);
    if (state.selection === null) {
      unbound.chats.push(chat);
      continue;
    }
    const section = bySelection.get(state.selection);
    if (section === void 0) throw new Error("Plugin selection has no real category: " + state.selection);
    section.chats.push(chat);
  }
  if (unbound.chats.length !== 0) sections.push(unbound);
  for (const section of sections) section.chats.sort(compareSidebarChats);
  return sections;
}
function conversationSidebarScope(section, records2) {
  const chats = new Map(section.chats.map(
    /** Associates the full actual host summary with its exact identity. */
    (chat) => [chat.id, chat]
  ));
  const membership = /* @__PURE__ */ new Set(), ids = /* @__PURE__ */ new Set();
  const groups = [];
  for (const record2 of records2) {
    if (record2.ownerSelection !== section.selection) throw new Error("Conversation group belongs to another explicit scope: " + record2.id);
    if (ids.has(record2.id)) throw new Error("Duplicate persisted conversation group: " + record2.id);
    ids.add(record2.id);
    const members2 = [];
    for (const chatId of record2.chatIds) {
      if (membership.has(chatId)) throw new Error("Chat belongs to multiple groups in this scope: " + chatId);
      const chat = chats.get(chatId);
      if (chat === void 0) throw new Error("Persisted group member has no chat in its declared scope: " + chatId);
      membership.add(chatId);
      members2.push(chat);
    }
    members2.sort(compareSidebarChats);
    groups.push({ id: record2.id, name: record2.name, pinned: record2.pinned, displayOrder: record2.displayOrder, chats: members2 });
  }
  groups.sort(
    /** Preserves pin priority and genuine persisted group order without updating metadata while reading. */
    (left, right) => Number(right.pinned) - Number(left.pinned) || left.displayOrder - right.displayOrder || left.id.localeCompare(right.id)
  );
  return { id: section.id, title: section.title, ownerSelection: section.selection, groups, ungrouped: section.chats.filter(
    /** Classifies explicitly absent membership only after inspecting the complete real scope catalog. */
    (chat) => !membership.has(chat.id)
  ) };
}
function readSidebarCatalog(value) {
  return __async(this, null, function* () {
    const current = parseCurrentSidebar(value), service = yield getService(), directory = yield service.snapshot();
    const selections = [];
    for (const chat of current.chatSidebar.chats) {
      const extension = yield Tools.Chat.readExtension({ kind: "chat", chatId: chat.id });
      if (extension === null) {
        selections.push({ chatId: chat.id, selection: null });
        continue;
      }
      const marker = decodeChatMarker(extension);
      requireChatSelection(marker.selection, directory.cards, directory.groups);
      selections.push({ chatId: chat.id, selection: marker.selection });
    }
    const sections = characterSidebarSections(directory, selections, current.chatSidebar.chats);
    if (current.input.view === "characters") return { view: "characters", sections };
    if (!sections.some(
      /** Ensures an explicit unbound grouping scope remains available for real empty group creation. */
      (section) => section.selection === null
    )) {
      const unbound = { id: "unbound", title: "\u672A\u7ED1\u5B9A\u89D2\u8272", avatarUri: null, kind: "unbound", selection: null, chats: [] };
      sections.push(unbound);
    }
    const scopes = [];
    for (const section of sections) scopes.push(conversationSidebarScope(section, yield service.dispatchDomain("conversation-group.list", { ownerSelection: section.selection })));
    return { view: "groups", scopes };
  });
}
function registerSidebarChannel() {
  ToolPkg.ipc.on("character-sidebar.catalog", readSidebarCatalog);
}
function argumentsFor(value, expected) {
  if (value.length !== 1 || !Array.isArray(value[0]) || value[0].length !== expected) throw new Error("CharacterSidebarHost expects " + expected + " arguments");
  return value[0];
}
function renderSidebarScreen(ctx) {
  const controller2 = ctx.createWebViewController("character-sidebar-web");
  const [input] = ctx.useState("input", null), [chatSidebar] = ctx.useState("chatSidebar", null);
  const [path, setPath] = ctx.useState("character-sidebar-html", ""), [error, setError] = ctx.useState("character-sidebar-error", "");
  const current = ctx.useRef("character-sidebar-current", parseCurrentSidebar({ input, chatSidebar }));
  current.current = parseCurrentSidebar({ input, chatSidebar });
  const ready = ctx.useRef("character-sidebar-ready", false), published = ctx.useRef("character-sidebar-published", "");
  const unsubscribe = ctx.useRef("character-sidebar-theme-subscription", null);
  const origin = "https://character-sidebar.operit.local/";
  function applyTheme(theme) {
    return __async(this, null, function* () {
      if (ready.current) yield controller2.evaluateJavascript("window.applyCharacterSidebarTheme(" + JSON.stringify(theme) + ");");
    });
  }
  function publishContext() {
    return __async(this, null, function* () {
      const serialized = JSON.stringify(current.current);
      if (!ready.current || published.current === serialized) return;
      try {
        yield controller2.evaluateJavascript("window.updateCharacterSidebar(" + serialized + ");");
        published.current = serialized;
      } catch (failure2) {
        setError(String(failure2));
        ctx.reportError(failure2);
      }
    });
  }
  if (ready.current && published.current !== JSON.stringify(current.current)) void publishContext();
  function initialize() {
    return __async(this, null, function* () {
      try {
        controller2.addJavascriptInterface("CharacterSidebarHost", {
          /** Returns the actual host palette for the independent embedded document. */
          currentTheme: () => ctx.Theme.getCurrent(),
          /** Reads exactly the validated route input and current native history context. */
          currentSidebar: (...args) => {
            argumentsFor(args, 0);
            ready.current = true;
            published.current = JSON.stringify(current.current);
            return current.current;
          },
          /** Reads the actual plugin-projected catalog through the one main runtime. */
          catalog: (...args) => {
            argumentsFor(args, 0);
            return ToolPkg.ipc.call("character-sidebar.catalog", current.current, { targetRuntime: "main" });
          },
          /** Delegates finite domain mutations to the same authoritative service, without a Compose-runtime repository. */
          domain: (...args) => {
            const [value] = argumentsFor(args, 1), message = parseDomainMessage(value);
            return ToolPkg.ipc.call("character-memory.domain", message, { targetRuntime: "main" });
          },
          /** Returns the exact existing generic activation action after verifying the target in the supplied host context. */
          activateChat: (...args) => {
            const [value] = argumentsFor(args, 1), chatId = sidebarIdentity(value, "activate chatId");
            if (!current.current.chatSidebar.chats.some(
              /** Requires an exact supplied native identity, not another role's guessed conversation. */
              (chat) => chat.id === chatId
            )) throw new Error("Chat activation target is absent from the actual sidebar context: " + chatId);
            return { type: "toolpkg.chat.activate", chatId };
          },
          /** Calls the actual generic native deletion chain; backend callbacks own extension and membership cleanup. */
          deleteChat: (...args) => {
            const [value] = argumentsFor(args, 1);
            return Tools.Chat.deleteChat(sidebarIdentity(value, "delete chatId"));
          }
        });
        unsubscribe.current = ctx.Theme.subscribe(applyTheme);
        setPath(yield ToolPkg.readResource("character_sidebar_html", "character-sidebar.html"));
      } catch (failure2) {
        setError(String(failure2));
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
  return ctx.UI.Box({ fillMaxSize: true, onLoad: initialize }, error !== "" ? ctx.UI.Text({ text: error }) : path === "" ? ctx.UI.Text({ text: "\u6B63\u5728\u52A0\u8F7D\u63D2\u4EF6\u4FA7\u8FB9\u680F\u2026" }) : ctx.UI.WebView({
    key: "character-sidebar-web",
    controller: controller2,
    fillMaxSize: true,
    url: origin,
    javaScriptEnabled: true,
    domStorageEnabled: true,
    supportZoom: false,
    useWideViewPort: true,
    /** Restricts top-level navigation to this exact offline sidebar document. */
    onShouldOverrideUrlLoading: (request) => request.url === origin ? { action: "allow" } : { action: "cancel" },
    /** Serves the real registered plugin resource through the existing cross-platform WebView host. */
    onInterceptRequest: (request) => request.url === origin ? { action: "respond", response: { mimeType: "text/html", encoding: "utf-8", statusCode: 200, reasonPhrase: "OK", filePath: path } } : { action: "block" },
    /** Disposes references when the embedded sidebar instance is actually released. */
    onLifecycleEvent: (event) => {
      if (event.type === "Disposed") dispose();
    }
  }));
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
      const extension = yield Tools.Chat.readExtension({ kind: "chat", chatId: input.chatId });
      selected = extension === null ? null : decodeChatMarker(extension).selection;
    }
    if (selected !== null) requireChatSelection(selected, cards, groups);
    const options = [];
    let defaultAvatar = null;
    if (input.kind === "card" || input.kind === "all") {
      if (cards.some(
        /** Requests the declared static avatar only for genuinely avatar-free cards, not after a failed image read. */
        (card) => card.avatarUri === null
      )) defaultAvatar = yield ToolPkg.readResource("character_default_avatar", "operit-avatar.png");
      for (const card of cards) {
        let avatar;
        if (card.avatarUri !== null) avatar = { type: "uri", uri: card.avatarUri };
        else {
          if (defaultAvatar === null || defaultAvatar.trim() === "") throw new Error("Declared character default-avatar resource did not resolve");
          avatar = { type: "resource", path: defaultAvatar };
        }
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
function createSelectorController(input, session, publish, assertOwner) {
  let state = { data: null, loading: true, switchingKey: null, error: "", finished: false }, loading = null;
  function update2(changes2) {
    state = { ...state, ...changes2 };
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
          if (!state.finished) update2({ data, loading: false, error: "" });
        },
        /** Keeps a real source failure visible and propagates the same original error to the host. */
        (failure2) => {
          update2({ loading: false, error: String(failure2) });
          throw failure2;
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
        update2({ switchingKey: key, error: "" });
        try {
          const complete = yield session.completeScreen({ mode: "select", selection: options[0].selection });
          assertOwner();
          update2({ finished: true });
          return complete;
        } catch (failure2) {
          update2({ error: String(failure2) });
          throw failure2;
        } finally {
          update2({ switchingKey: null });
        }
      });
    },
    /** Closes only this request and never commits a staged choice or modifies another selector. */
    cancel() {
      idle();
      const cancel = session.cancelScreen();
      update2({ finished: true });
      return cancel;
    }
  };
}
function selectorResultJson(result2) {
  switch (result2.type) {
    case "toolpkg.presentation.complete":
      return { type: result2.type, requestId: result2.requestId, value: result2.value };
    case "toolpkg.presentation.cancel":
      return { type: result2.type, requestId: result2.requestId };
  }
}
function selectorAvatar(ctx, option) {
  const props = { width: 28, height: 28, contentScale: "crop", contentDescription: option.title, modifier: ctx.Modifier.clip({ type: "circle" }) };
  switch (option.avatar.type) {
    case "uri":
      return ctx.UI.Image({ ...props, uri: option.avatar.uri });
    case "resource":
      return ctx.UI.Image({ ...props, path: option.avatar.path });
    case "group":
      return ctx.UI.Box({ width: 28, height: 28, contentAlignment: "center" }, ctx.UI.Icon({ name: "Groups", size: 22, tint: "onSurfaceVariant", contentDescription: option.title }));
  }
}
function selectorRow(ctx, option, state, controller2) {
  if (state.data === null) throw new Error("A selector row requires its actual loaded catalog");
  const active = option.token === state.data.selected, switching = state.switchingKey === option.key;
  const enabled = state.switchingKey === null && !state.finished, colors = ctx.MaterialTheme.colorScheme;
  const titleColor = active ? colors.primary : enabled ? colors.onSurface : colors.onSurfaceVariant.copy({ alpha: 0.65 });
  const descriptionColor = enabled ? colors.onSurfaceVariant : colors.onSurfaceVariant.copy({ alpha: 0.5 });
  const props = { key: option.key, fillMaxWidth: true, paddingStart: 10, paddingTop: 7, paddingEnd: 8, paddingBottom: 7, verticalAlignment: "center", spacing: 8 };
  const contents = [
    selectorAvatar(ctx, option),
    ctx.UI.Column({ weight: 1, horizontalAlignment: "start" }, [
      ctx.UI.Text({ text: option.title, style: "bodySmall", maxLines: 1, overflow: "ellipsis", color: titleColor, fontWeight: active ? "700" : "600" }),
      ...option.description === "" ? [] : [ctx.UI.Text({ text: option.description, style: "labelSmall", maxLines: 1, overflow: "ellipsis", color: descriptionColor })]
    ]),
    ctx.UI.Box({ width: 20, height: 20, contentAlignment: "center" }, switching ? ctx.UI.CircularProgressIndicator({ width: 16, height: 16, strokeWidth: 2 }) : ctx.UI.Icon({ name: active ? "Check" : "CircleOutlined", size: active ? 18 : 16, tint: active ? colors.primary : colors.onSurfaceVariant.copy({ alpha: 0.45 }), contentDescription: active ? "\u5F53\u524D\u9009\u4E2D" : "\u672A\u9009\u4E2D" }))
  ];
  if (!enabled) return ctx.UI.Row(props, contents);
  return ctx.UI.Row({
    ...props,
    /** Returns the real commit result to the existing generic Compose action channel without a synthetic receiver. */
    onClick: () => __async(null, null, function* () {
      return selectorResultJson(yield controller2.select(option.key));
    })
  }, contents);
}
function selectorView(ctx, input, state, controller2) {
  const title = input.kind === "card" ? "\u5207\u6362\u89D2\u8272\u5361" : input.kind === "group" ? "\u5207\u6362\u7FA4\u7EC4" : "\u5207\u6362\u89D2\u8272\u5361\u6216\u7FA4\u7EC4";
  const colors = ctx.MaterialTheme.colorScheme;
  const header = ctx.UI.Row({ fillMaxWidth: true, paddingStart: 16, paddingTop: 8, paddingEnd: 8, paddingBottom: 4, verticalAlignment: "center" }, [
    ctx.UI.Text({ text: title, style: "titleSmall", fontWeight: "700", weight: 1 }),
    ...state.data === null ? [] : [ctx.UI.Text({ text: state.data.options.length + " \u4E2A", style: "labelSmall", color: colors.onSurfaceVariant }), ctx.UI.Spacer({ width: 2 })],
    ctx.UI.IconButton({
      key: "selector-close",
      enabled: state.switchingKey === null && !state.finished,
      width: 32,
      height: 32,
      /** Emits the explicit cancellation discriminator, not a route pop that bypasses the plugin session. */
      onClick: () => selectorResultJson(controller2.cancel())
    }, ctx.UI.Icon({ name: "Close", size: 18, contentDescription: "\u5173\u95ED" }))
  ]);
  let body;
  if (state.loading) body = ctx.UI.Box({ weight: 1, fillMaxWidth: true, contentAlignment: "center" }, ctx.UI.CircularProgressIndicator({ width: 20, height: 20, strokeWidth: 2 }));
  else if (state.data === null) body = ctx.UI.Text({ key: "selector-load-error", text: state.error, color: colors.error, paddingHorizontal: 16, paddingVertical: 18, weight: 1 });
  else if (state.data.options.length === 0) body = ctx.UI.Text({ text: input.kind === "group" ? "\u6682\u65E0\u7FA4\u7EC4" : "\u6682\u65E0\u89D2\u8272\u5361", style: "bodySmall", color: colors.onSurfaceVariant, paddingStart: 16, paddingTop: 18, paddingEnd: 16, paddingBottom: 20, weight: 1 });
  else {
    const rows = [];
    for (const option of state.data.options) {
      if (rows.length !== 0) rows.push(ctx.UI.HorizontalDivider({ thickness: 1, color: colors.outlineVariant.copy({ alpha: 0.45 }) }));
      rows.push(selectorRow(ctx, option, state, controller2));
    }
    body = ctx.UI.LazyColumn({ key: "selector-records", weight: 1, fillMaxWidth: true, paddingStart: 8, paddingTop: 4, paddingEnd: 8, paddingBottom: 8 }, rows);
  }
  return ctx.UI.Dialog({
    key: "character-selection",
    containerColor: "surfaceContainerHigh",
    shape: { cornerRadius: 28 },
    closeOnDismissRequest: false,
    properties: { usePlatformDefaultWidth: false, dismissOnBackPress: state.switchingKey === null, dismissOnClickOutside: state.switchingKey === null },
    /** Loads once through the main domain IPC; a rerender cannot initialize another store or retry a failed read. */
    onLoad: () => controller2.load(),
    /** Returns explicit V1 cancellation for host dismissal rather than changing a global active selection. */
    onDismissRequest: () => selectorResultJson(controller2.cancel())
  }, ctx.UI.Column({ fillMaxWidth: true, height: 420, modifier: ctx.Modifier.widthIn({ maxWidth: 360 }).heightIn({ minHeight: 420, maxHeight: 420 }) }, [
    header,
    ctx.UI.HorizontalDivider({ thickness: 1 }),
    body,
    ...state.error === "" || state.data === null ? [] : [ctx.UI.Text({ key: "selector-commit-error", text: state.error, color: colors.error, style: "bodySmall", paddingHorizontal: 16, paddingVertical: 8 })]
  ]));
}
function renderSelectionScreen(ctx) {
  const [presentation] = ctx.useState("presentation", null);
  const [state, setState] = ctx.useState("native-selector-state", { data: null, loading: true, switchingKey: null, error: "", finished: false });
  const stored = ctx.useRef("native-selector-controller", null);
  const identity4 = ctx.useRef("native-selector-request", JSON.stringify(presentation));
  function assertOwner() {
    const [current] = ctx.useState("presentation", null);
    if (identity4.current !== JSON.stringify(current)) throw new Error("A native selector cannot change its owning presentation request");
  }
  assertOwner();
  const parsed = selectorSession(presentation);
  if (stored.current === null) stored.current = createSelectorController(parsed.input, parsed.session, setState, assertOwner);
  return selectorView(ctx, parsed.input, state, stored.current);
}

// src/main.ts
connectDirectorySources({
  /** Reads the actual configured model directory only when the shared service requests it. */
  listModels: () => Tools.SoftwareSettings.listModelSummaries(),
  /** Reads the actual TTS directory without opening storage during registration. */
  listTtsConfigs: () => Tools.SoftwareSettings.listTtsConfigs(),
  /** Reads complete real builtin, package, skill and MCP sources through the generic settings capability. */
  readToolCatalog: () => Tools.SoftwareSettings.readToolSourceCatalog()
});
registerUiRequestChannel();
registerSidebarChannel();
function screen(ctx) {
  return renderScreen(ctx, definition);
}
function attachmentScreen(ctx) {
  return renderScreen(ctx, definition, "memory-attachment");
}
function sidebarScreen(ctx) {
  return renderSidebarScreen(ctx);
}
function selectionScreen(ctx) {
  return renderSelectionScreen(ctx);
}
function groupExecutionScreen(ctx) {
  return renderGroupExecutionScreen(ctx);
}
function registerToolPkg() {
  registerSelectionSettingsAccess();
  registerDomainCommands();
  registerDomainApis();
  registerServiceLifecycle();
  registerMemoryJobHooks();
  registerToolPolicies();
  registerChatInitialization();
  registerGroupExecutionHooks();
  registerUiContributionApis();
  return register(definition, screen, attachmentScreen, sidebarScreen, selectionScreen, groupExecutionScreen);
}

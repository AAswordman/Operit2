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

// src/ui/chat-input-memory/index.ui.ts
var index_ui_exports = {};
__export(index_ui_exports, {
  default: () => Screen
});
module.exports = __toCommonJS(index_ui_exports);

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

// src/storage/state.ts
var MEMORY_RECALL_LIMIT_RANGE = { min: 1, max: 50 };

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

// src/ui-memory-menu.ts
function request(input) {
  return ToolPkg.ipc.call("character-memory.request", input, { targetRuntime: "main" });
}
function memorySummary(preferences) {
  if (preferences.userDescriptionEnabled) return preferences.autoUpdateEnabled ? "用户资料 · 记忆库更新" : "用户资料";
  return preferences.autoUpdateEnabled ? "记忆库更新" : "关";
}
function memoryQueueDescription(queue) {
  if (queue === null) return "暂无队列状态。自动提取会定期检查当前记忆库。";
  return `待处理 ${queue.pendingCandidates} 条 · ${queue.pendingChats} 个聊天
处理中 ${queue.processingCandidates} · 失败 ${queue.failedCandidates}
下次检查：约 ${queue.minutesUntilNextRun} 分钟后

自动检查需要至少 5 个候选；不足时继续等待。手动更新不受此门槛限制。` + (queue.lastError === "" ? "" : "\n\n" + queue.lastError);
}
function renderMemoryMenu(ctx) {
  const [chatId] = ctx.useState("chatId", null);
  if (chatId !== null) chatIdentity(chatId, "memory menu chatId");
  const owner2 = ctx.useRef("memory-menu-chat", chatId);
  const [available, setAvailable] = ctx.useState("memory-menu-available", null);
  const [preferences, setPreferences] = ctx.useState("memory-menu-preferences", null);
  const [recall, setRecall] = ctx.useState("memory-menu-recall", null);
  const [limitInput, setLimitInput] = ctx.useState("memory-recall-limit-input", "3");
  const [queue, setQueue] = ctx.useState("memory-menu-queue", null);
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
        const binding = yield callMainDomain("chat.configuration.binding.find", { chatId });
        assertOwner();
        const hasCharacter = binding !== null && parseChatSelection(binding.selection).kind === "card";
        setAvailable(hasCharacter);
        if (!hasCharacter) return;
      }
      const value = yield request({ action: "readGlobalMemoryPreferences" });
      assertOwner();
      setPreferences(value);
      const recallValue = yield request({ action: "readMemoryRecallSettings" });
      assertOwner();
      setRecall(recallValue);
      setLimitInput(String(recallValue.limit));
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
  function writeRecall(settings2) {
    return __async(this, null, function* () {
      assertOwner();
      const value = yield request({ action: "writeMemoryRecallSettings", settings: settings2 });
      assertOwner();
      setRecall(value);
    });
  }
  function toggleRecall() {
    return __async(this, null, function* () {
      const [current] = ctx.useState("memory-menu-recall", null);
      if (current === null) throw new Error("Memory recall settings are not loaded");
      yield writeRecall({ ...current, enabled: !current.enabled });
    });
  }
  function toggleRecallRepeat() {
    return __async(this, null, function* () {
      const [current] = ctx.useState("memory-menu-recall", null);
      if (current === null) throw new Error("Memory recall settings are not loaded");
      yield writeRecall({ ...current, allowRepeated: !current.allowRepeated });
    });
  }
  function changeRecallLimit(limit) {
    return __async(this, null, function* () {
      const [current] = ctx.useState("memory-menu-recall", null);
      if (current === null) throw new Error("Memory recall settings are not loaded");
      if (limit === current.limit) return;
      yield writeRecall({ ...current, limit });
    });
  }
  function changeRecallLimitInput(value) {
    setLimitInput(value);
    const limit = Number(value);
    if (!Number.isInteger(limit) || limit < MEMORY_RECALL_LIMIT_RANGE.min || limit > MEMORY_RECALL_LIMIT_RANGE.max) return;
    if (recall !== null && limit === recall.limit) return;
    return changeRecallLimit(limit);
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
        return { type: "toolpkg.ui.message", message: "记忆提取完成" };
      } catch (error) {
        assertOwner();
        return { type: "toolpkg.ui.message", message: "记忆提取失败：" + String(error) };
      } finally {
        liveBusy.current = false;
        assertOwner();
        setBusy(false);
      }
    });
  }
  function retryFailed() {
    return __async(this, null, function* () {
      assertOwner();
      if (liveBusy.current || chatId === null) return null;
      liveBusy.current = true;
      setBusy(true);
      try {
        const result = yield request({ action: "retryFailedCandidates", chatId });
        assertOwner();
        yield refreshQueue();
        return { type: "toolpkg.ui.message", message: "已重新排队 " + result.retried + " 条失败候选" };
      } catch (error) {
        assertOwner();
        return { type: "toolpkg.ui.message", message: "重试失败候选失败：" + String(error) };
      } finally {
        liveBusy.current = false;
        assertOwner();
        setBusy(false);
      }
    });
  }
  function clearFailed() {
    return __async(this, null, function* () {
      assertOwner();
      if (liveBusy.current || chatId === null) return null;
      liveBusy.current = true;
      setBusy(true);
      try {
        const result = yield request({ action: "clearFailedCandidates", chatId });
        assertOwner();
        yield refreshQueue();
        return { type: "toolpkg.ui.message", message: "已清除 " + result.cleared + " 条失败记录" };
      } catch (error) {
        assertOwner();
        return { type: "toolpkg.ui.message", message: "清除失败记录失败：" + String(error) };
      } finally {
        liveBusy.current = false;
        assertOwner();
        setBusy(false);
      }
    });
  }
  function icon(name, color = colors.onSurfaceVariant) {
    return ctx.UI.Icon({ name, size: 16, tint: color });
  }
  function switchRow(key, name, title, checked, toggleRow) {
    return ctx.UI.Row({
      key: "memory-" + key,
      fillMaxWidth: true,
      paddingHorizontal: 12,
      verticalAlignment: "center",
      modifier: ctx.Modifier.heightIn({ minHeight: 32 }),
      onClick: () => toggleRow()
    }, [
      icon(name, checked ? colors.primary : colors.onSurfaceVariant),
      ctx.UI.Spacer({ width: 12 }),
      ctx.UI.Text({ text: title, style: "bodySmall", color: colors.onSurface, maxLines: 1, overflow: "ellipsis", weight: 1 }),
      ctx.UI.Text({ text: checked ? "开" : "关", style: "bodySmall", color: colors.primary, fontWeight: "600" }),
      ctx.UI.Spacer({ width: 8 }),
      ctx.UI.Switch({ checked, width: 40, height: 28, onCheckedChange: () => toggleRow() })
    ]);
  }
  function limitInputRow(title) {
    return ctx.UI.Row({
      key: "memory-recallLimit",
      fillMaxWidth: true,
      paddingHorizontal: 12,
      verticalAlignment: "center",
      modifier: ctx.Modifier.heightIn({ minHeight: 32 })
    }, [
      icon("Tune"),
      ctx.UI.Spacer({ width: 12 }),
      ctx.UI.Text({ text: title, style: "bodySmall", color: colors.onSurface, maxLines: 1, overflow: "ellipsis", weight: 1 }),
      ctx.UI.Spacer({ width: 8 }),
      ctx.UI.TextField({
        key: "memory-recall-limit-input",
        value: limitInput,
        singleLine: true,
        placeholder: "1-" + MEMORY_RECALL_LIMIT_RANGE.max,
        onValueChange: changeRecallLimitInput,
        style: { fontSize: 13 },
        decoration: { isDense: true, contentPadding: { vertical: 8 } },
        modifier: ctx.Modifier.width(72)
      })
    ]);
  }
  if (available === false) return ctx.UI.Box({ key: "memory-menu-unavailable" });
  if (preferences === null) return ctx.UI.Box({ key: "memory-menu-loading", onLoad: load });
  const children = [ctx.UI.Row({
    key: "memory-section-header",
    fillMaxWidth: true,
    paddingHorizontal: 12,
    verticalAlignment: "center",
    modifier: ctx.Modifier.heightIn({ minHeight: 36 }),
    onClick: () => {
      assertOwner();
      setExpanded(!expanded);
    }
  }, [
    icon("DataObjectOutlined", colors.onSurfaceVariant.copy({ alpha: 0.7 })),
    ctx.UI.Spacer({ width: 12 }),
    ctx.UI.Text({ text: "记忆", style: "bodySmall" }),
    ctx.UI.Spacer({ width: 8 }),
    ctx.UI.Text({ text: memorySummary(preferences), style: "bodySmall", color: colors.primary, fontWeight: "600", textAlign: "end", maxLines: 1, overflow: "ellipsis", weight: 1 }),
    ctx.UI.IconButton({
      key: "memory-info",
      tooltip: "记忆状态说明",
      width: 24,
      height: 28,
      contentPadding: { horizontal: 0, vertical: 0 },
      onClick: showInfo,
      content: ctx.UI.Icon({ name: "InfoOutline", size: 16, tint: colors.onSurfaceVariant.copy({ alpha: 0.7 }) })
    }),
    ctx.UI.Spacer({ width: 6 }),
    ctx.UI.Icon({ name: expanded ? "KeyboardArrowUp" : "KeyboardArrowDown", size: 20, tint: colors.onSurfaceVariant })
  ])];
  if (expanded) {
    const sectionChildren = [
      switchRow("userDescriptionEnabled", "AssignmentIndOutlined", "提供用户资料", preferences.userDescriptionEnabled, () => toggle("userDescriptionEnabled")),
      switchRow("autoUpdateEnabled", preferences.autoUpdateEnabled ? "Save" : "SaveOutlined", "自动更新记忆库", preferences.autoUpdateEnabled, () => toggle("autoUpdateEnabled")),
      switchRow("memoryRecall", "AutoAwesome", "自动检索注入记忆", recall !== null && recall.enabled, toggleRecall),
      limitInputRow("记忆注入条数"),
      switchRow("memoryRecallRepeat", "Repeat", "允许重复命中", recall !== null && recall.allowRepeated, toggleRecallRepeat),
      ctx.UI.Row({
        key: "memory-manual",
        fillMaxWidth: true,
        paddingHorizontal: 12,
        verticalAlignment: "center",
        modifier: ctx.Modifier.heightIn({ minHeight: 32 }),
        ...!busy && chatId !== null ? { onClick: manual } : {}
      }, [
        icon("SaveOutlined"),
        ctx.UI.Spacer({ width: 12 }),
        ctx.UI.Text({
          text: busy ? "正在更新记忆…" : "手动更新记忆",
          style: "bodySmall",
          maxLines: 2,
          overflow: "ellipsis",
          weight: 1,
          color: !busy && chatId !== null ? colors.onSurface : colors.onSurfaceVariant.copy({ alpha: 0.45 })
        })
      ])
    ];
    if (queue !== null && queue.failedCandidates > 0) {
      const enabled = !busy && chatId !== null;
      sectionChildren.push(
        ctx.UI.Row({
          key: "memory-retry-failed",
          fillMaxWidth: true,
          paddingHorizontal: 12,
          verticalAlignment: "center",
          modifier: ctx.Modifier.heightIn({ minHeight: 32 }),
          ...enabled ? { onClick: retryFailed } : {}
        }, [
          icon("Refresh"),
          ctx.UI.Spacer({ width: 12 }),
          ctx.UI.Text({
            text: "重试失败候选（" + queue.failedCandidates + "）",
            style: "bodySmall",
            maxLines: 1,
            overflow: "ellipsis",
            weight: 1,
            color: enabled ? colors.onSurface : colors.onSurfaceVariant.copy({ alpha: 0.45 })
          })
        ]),
        ctx.UI.Row({
          key: "memory-clear-failed",
          fillMaxWidth: true,
          paddingHorizontal: 12,
          verticalAlignment: "center",
          modifier: ctx.Modifier.heightIn({ minHeight: 32 }),
          ...enabled ? { onClick: clearFailed } : {}
        }, [
          icon("DeleteOutline"),
          ctx.UI.Spacer({ width: 12 }),
          ctx.UI.Text({
            text: "清除失败记录（" + queue.failedCandidates + "）",
            style: "bodySmall",
            maxLines: 1,
            overflow: "ellipsis",
            weight: 1,
            color: enabled ? colors.onSurface : colors.onSurfaceVariant.copy({ alpha: 0.45 })
          })
        ])
      );
    }
    children.push(ctx.UI.Column({ key: "memory-section-children", modifier: ctx.Modifier.padding(12, 0, 8, 4) }, sectionChildren));
  }
  return ctx.UI.Column({ key: "memory-section", fillMaxWidth: true }, children);
}

// src/ui/chat-input-memory/index.ui.ts
function Screen(ctx) {
  return renderMemoryMenu(ctx);
}

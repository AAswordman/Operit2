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

// src/ui/chat-input-menu/index.ui.ts
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
function parseChatSelection(selection) {
  requireId(selection, "chat selection");
  const parsed = /^(card|group):([^\s]+)$/u.exec(selection);
  if (parsed === null) throw new Error("Invalid plugin chat selection: " + selection);
  const kind = parsed[1];
  if (kind !== "card" && kind !== "group") throw new Error("Invalid chat selection kind: " + kind);
  return { kind, id: parsed[2] };
}
function requireChatSelection(selection, cards, groups) {
  const parsed = parseChatSelection(selection);
  const records2 = parsed.kind === "card" ? cards : groups;
  let matches = 0;
  for (const record2 of records2) if (record2.id === parsed.id) matches += 1;
  if (matches !== 1) throw new Error("Chat selection does not identify exactly one stored record: " + selection);
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
function encodeSelection(selection) {
  return selectionForActive(selection);
}

// src/ui-selector.ts
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

// src/ui-input-menu.ts
function selectedOption(data) {
  if (data.selected === null) return null;
  const matching = data.options.filter(
    /** Matches the exact opaque selection confirmed by the existing selector reader. */
    (option) => option.token === data.selected
  );
  if (matching.length !== 1) throw new Error("Current character must identify exactly one selector record");
  return matching[0];
}
function menuAvatar(ctx, option, unboundAvatar) {
  const props = { width: 32, height: 32, contentScale: "crop", modifier: ctx.Modifier.clip({ type: "circle" }) };
  if (option === null) {
    if (unboundAvatar === null) throw new Error("Unbound character avatar resource is not loaded");
    return ctx.UI.Image({ ...props, path: unboundAvatar, contentDescription: "\u672A\u7ED1\u5B9A" });
  }
  switch (option.avatar.type) {
    case "uri":
      return ctx.UI.Image({ ...props, uri: option.avatar.uri, contentDescription: option.title });
    case "resource":
      return ctx.UI.Image({ ...props, path: option.avatar.path, contentDescription: option.title });
    case "group":
      return ctx.UI.Box({ width: 32, height: 32, contentAlignment: "center" }, ctx.UI.Icon({ name: "Groups", size: 24, tint: ctx.MaterialTheme.colorScheme.onSurfaceVariant }));
  }
}
function renderChatInputMenu(ctx) {
  const [chatId] = ctx.useState("chatId", null);
  if (chatId !== null) chatIdentity(chatId, "input menu chatId");
  const owner2 = ctx.useRef("input-menu-chat", chatId);
  const [state, setState] = ctx.useState("input-menu-state", { data: null, unboundAvatar: null, error: "" });
  function assertOwner() {
    const [currentChatId] = ctx.useState("chatId", null);
    if (currentChatId !== owner2.current) throw new Error("Input menu chat owner changed");
  }
  assertOwner();
  function load() {
    return __async(this, null, function* () {
      assertOwner();
      try {
        const data = yield readSelectorData({ mode: "select", chatId, kind: "all", selected: null });
        const option2 = selectedOption(data);
        const unboundAvatar = option2 === null ? yield ToolPkg.readResource("character_default_avatar", "operit-avatar.png") : null;
        if (option2 === null && (typeof unboundAvatar !== "string" || unboundAvatar.trim() === "")) throw new Error("Declared unbound avatar resource did not resolve");
        assertOwner();
        setState({ data, unboundAvatar, error: "" });
      } catch (error) {
        assertOwner();
        setState({ data: null, unboundAvatar: null, error: String(error) });
      }
    });
  }
  const colors = ctx.MaterialTheme.colorScheme;
  if (state.error !== "") return ctx.UI.Text({ text: state.error, color: colors.error, paddingHorizontal: 12, paddingVertical: 6 });
  if (state.data === null) return ctx.UI.Box({ key: "current-character-loading", onLoad: load, modifier: ctx.Modifier.heightIn({ minHeight: 48 }), fillMaxWidth: true, contentAlignment: "center" }, ctx.UI.CircularProgressIndicator({ width: 18, height: 18 }));
  const option = selectedOption(state.data);
  return ctx.UI.Row({
    key: "current-character",
    fillMaxWidth: true,
    paddingHorizontal: 12,
    paddingVertical: 6,
    verticalAlignment: "center",
    modifier: ctx.Modifier.heightIn({ minHeight: 48 }),
    /** Opens the registered TS selector while the host carries its real chat lifecycle guard. */
    onClick: () => {
      assertOwner();
      return {
        type: "toolpkg.ui.present",
        routeId: chatUiRoutes.selection,
        input: { mode: "select", chatId, kind: "card", selected: option !== null && option.kind === "card" ? option.selection : null }
      };
    }
  }, [
    menuAvatar(ctx, option, state.unboundAvatar),
    ctx.UI.Spacer({ width: 10 }),
    ctx.UI.Column({ weight: 1 }, [
      ctx.UI.Text({ text: "\u5F53\u524D\u89D2\u8272\u5361", style: "labelSmall", color: colors.onSurfaceVariant }),
      ctx.UI.Text({ text: option === null ? "\u672A\u7ED1\u5B9A" : option.title, style: "bodySmall", color: colors.onSurface, fontWeight: "600", maxLines: 1, overflow: "ellipsis" })
    ]),
    ctx.UI.Icon({ name: "ChevronRight", size: 20, tint: colors.onSurfaceVariant })
  ]);
}

// src/ui/chat-input-menu/index.ui.ts
function Screen(ctx) {
  return renderChatInputMenu(ctx);
}

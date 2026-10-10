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

// src/ui/chat-sidebar/index.ui.ts
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
function encodeSelection(selection) {
  return selectionForActive(selection);
}

// src/chat-lifecycle.ts
var CHARACTER_CARDS_NAMESPACE = "com.operit.character_cards";
function fields(value, keys, label) {
  const input = record(value, label), actual = Object.keys(input);
  if (actual.length !== keys.length || actual.some(
    /** Rejects undocumented fields rather than guessing their meaning or owner. */
    (key) => keys.indexOf(key) === -1
  )) throw new Error(label + " has invalid fields");
  return input;
}
function identity(value, label) {
  if (typeof value !== "string" || value.trim() === "") throw new Error(label + " must be a nonblank string");
  return value;
}
function parseChatSelectionMarker(value, label) {
  const marker = fields(value, ["version", "selection"], label);
  if (marker.version !== 1) throw new Error(label + ".version must be 1");
  const selection = identity(marker.selection, label + ".selection");
  parseChatSelection(selection);
  return { version: 1, selection };
}
function createChatInput(selection) {
  const marker = parseChatSelectionMarker({ version: 1, selection }, "chat creation selection");
  return { [CHARACTER_CARDS_NAMESPACE]: { version: 1, selection: marker.selection } };
}

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

// src/ui-sidebar-native-layout.ts
var legacySidebarMetrics = Object.freeze({
  categoryStart: 20,
  categoryEnd: 12,
  categoryAvatar: 22,
  categoryRadius: 18,
  groupStart: 46,
  groupEnd: 12,
  groupRadius: 12,
  chatStart: 56,
  chatEnd: 12,
  chatHeight: 34,
  chatRadius: 8,
  railWidth: 20,
  createHeight: 34,
  createRadius: 17,
  previewLimit: 4
});
function nativeSidebarRows(ctx, current, state, actions, defaultAvatar) {
  const colors = ctx.MaterialTheme.colorScheme, m = legacySidebarMetrics;
  const query = state.query.trim().toLocaleLowerCase();
  const muted = colors.onSurfaceVariant.copy({ alpha: 0.8 });
  const rows = [];
  const gap = (width) => ctx.UI.Spacer({ width });
  function more(kind, id2, selected = false) {
    return ctx.UI.HoverOnly(
      { key: "sidebar-" + kind + "-menu-" + id2 + "-visibility", alwaysVisible: selected },
      actions.menu(kind, id2)
    );
  }
  function chatRow(chat, section, groupId) {
    const selected = current.chatSidebar.currentChatId === chat.id;
    const titleColor = selected ? colors.onSecondaryContainer : muted;
    const running = current.chatSidebar.activeStreamingChatIds.includes(chat.id);
    const status = ctx.UI.ActivityDots({
      key: "sidebar-status-" + chat.id,
      width: 10,
      height: 13.5,
      running,
      selected,
      restingColor: selected ? colors.onSecondaryContainer.copy({ alpha: 0.7 }) : colors.onSurfaceVariant.copy({ alpha: 0.3 }),
      activeColor: colors.primary
    });
    const content = [ctx.UI.Draggable({
      key: "sidebar-chat-drag-" + chat.id,
      data: chat.id,
      dragType: "character-sidebar.chat",
      enabled: true,
      feedback: ctx.UI.Row(
        { width: 220, height: 34, paddingHorizontal: 10, verticalAlignment: "center", background: colors.surfaceContainerHighest, backgroundShape: { cornerRadius: 8 } },
        ctx.UI.Text({ text: chat.title, weight: 1, maxLines: 1, overflow: "ellipsis", fontSize: 13, color: colors.onSurface })
      )
    }, status), gap(5), ctx.UI.Text({
      key: "sidebar-chat-title-" + chat.id,
      text: chat.title,
      weight: 1,
      fontSize: 13,
      fontWeight: selected ? "600" : "400",
      maxLines: 1,
      overflow: "ellipsis",
      color: titleColor,
      ...{ letterSpacing: -0.1 }
    })];
    if (chat.pinned) content.push(gap(5), ctx.UI.Icon({ name: "PushPin", size: 12, tint: titleColor.copy ? titleColor.copy({ alpha: 0.6 }) : titleColor }));
    if (chat.locked) content.push(gap(5), ctx.UI.Icon({ name: "Lock", size: 12, tint: titleColor.copy ? titleColor.copy({ alpha: 0.6 }) : titleColor }));
    content.push(gap(4), more("chat", chat.id, selected));
    const surface = ctx.UI.Row({
      key: "sidebar-chat-hit-" + chat.id,
      weight: 1,
      height: 31,
      verticalAlignment: "center",
      paddingHorizontal: 6,
      modifier: (selected ? ctx.Modifier.background(colors.secondaryContainer.copy({ alpha: 0.78 }), { cornerRadius: m.chatRadius }).border(1, colors.onSecondaryContainer.copy({ alpha: 0.08 }), { cornerRadius: m.chatRadius }) : ctx.Modifier).combinedClickable({ onClick: () => actions.activate(chat.id), onLongClick: () => actions.longPress(chat.id) })
    }, content);
    const rail = ctx.UI.Box({ key: "sidebar-chat-rail-" + chat.id, width: m.railWidth, height: m.chatHeight, contentAlignment: "center" }, [
      ctx.UI.Box({ width: 1, height: m.chatHeight, background: colors.outlineVariant.copy({ alpha: 0.45 }) }),
      ...selected ? [ctx.UI.Box({ width: 2, height: 16, background: colors.primary, backgroundShape: { cornerRadius: 1.5 } })] : []
    ]);
    return ctx.UI.DragTarget(
      {
        key: "sidebar-chat-drop-" + chat.id,
        acceptedTypes: ["character-sidebar.chat"],
        acceptedData: section.chats.filter((value) => value.id !== chat.id).map((value) => value.id),
        hoverBorderColor: colors.primary.copy({ alpha: 0.55 }),
        shape: { cornerRadius: 8 },
        fillMaxWidth: true,
        onDrop: (id2) => actions.drop(String(id2), section.id, groupId, chat.id)
      },
      ctx.UI.Row({
        key: "sidebar-chat-" + chat.id,
        fillMaxWidth: true,
        height: m.chatHeight,
        paddingStart: m.chatStart,
        paddingEnd: m.chatEnd,
        verticalAlignment: "center"
      }, [
        rail,
        gap(2),
        ctx.UI.HoverRegion(
          { weight: 1, hoverBackground: colors.onSurfaceVariant.copy({ alpha: 0.07 }), shape: { cornerRadius: m.chatRadius } },
          ctx.UI.SwipeActions({
            key: "sidebar-chat-swipe-" + chat.id,
            actionThreshold: 0.4,
            onStartAction: () => actions.rename(chat.id),
            onEndAction: () => actions.deleteChat(chat.id),
            startBackground: ctx.UI.Box(
              { fillMaxSize: true, paddingStart: 12, background: colors.primary, contentAlignment: "start" },
              ctx.UI.Icon({ name: "Edit", size: 18, tint: colors.onPrimary })
            ),
            endBackground: ctx.UI.Box(
              { fillMaxSize: true, paddingEnd: 12, background: colors.error, contentAlignment: "end" },
              ctx.UI.Icon({ name: "Delete", size: 18, tint: colors.onError })
            )
          }, surface)
        )
      ])
    );
  }
  function chats(values, id2, section, groupId, matchAll = false) {
    const matches = query === "" || matchAll ? values : values.filter((chat) => chat.title.toLocaleLowerCase().includes(query));
    const expanded = state.expandedGroups.includes("preview:" + id2);
    const preview = query !== "" || expanded ? matches : matches.filter((chat, index) => index < m.previewLimit || chat.pinned || current.chatSidebar.currentChatId === chat.id || current.chatSidebar.activeStreamingChatIds.includes(chat.id));
    const result = preview.map((chat) => chatRow(chat, section, groupId));
    const hidden = matches.filter((chat) => !preview.includes(chat)).length;
    if (query === "" && (hidden > 0 || expanded && matches.length > m.previewLimit)) result.push(ctx.UI.Row({
      key: "sidebar-preview-" + id2,
      fillMaxWidth: true,
      paddingStart: m.chatStart,
      paddingEnd: m.chatEnd,
      paddingTop: 2,
      height: 34,
      verticalAlignment: "center",
      spacing: 8,
      onClick: () => actions.toggleGroup("preview:" + id2)
    }, [
      ctx.UI.Icon({ name: expanded ? "ExpandLess" : "ExpandMore", size: 18, tint: colors.onSurfaceVariant.copy({ alpha: 0.72 }) }),
      ctx.UI.Text({ text: expanded ? "\u6536\u8D77" : "\u5C55\u5F00\u66F4\u591A " + hidden, style: "labelMedium", fontWeight: "600", color: colors.onSurfaceVariant.copy({ alpha: 0.72 }) })
    ]));
    return result;
  }
  function category(section) {
    const collapsed = state.collapsed.includes(section.id);
    const avatar = section.kind === "card" ? ctx.UI.Image({
      key: "sidebar-avatar-" + section.id,
      uri: section.avatarUri ?? defaultAvatar ?? "",
      width: m.categoryAvatar,
      height: m.categoryAvatar,
      contentScale: "crop",
      contentDescription: section.title,
      modifier: ctx.Modifier.clip({ type: "circle" })
    }) : ctx.UI.Box(
      {
        width: m.categoryAvatar,
        height: m.categoryAvatar,
        contentAlignment: "center",
        background: colors.surfaceContainerLow.copy({ alpha: 0.72 }),
        backgroundShape: { type: "circle" }
      },
      ctx.UI.Icon({ name: section.kind === "group" ? "Groups" : "AccountTree", size: 14, tint: colors.onSurfaceVariant })
    );
    return ctx.UI.Column(
      {
        key: "sidebar-section-" + section.id,
        fillMaxWidth: true,
        paddingStart: m.categoryStart,
        paddingEnd: m.categoryEnd,
        paddingTop: 10,
        paddingBottom: 5
      },
      ctx.UI.DragTarget(
        {
          key: "sidebar-category-drop-" + section.id,
          acceptedTypes: ["character-sidebar.chat"],
          acceptedData: section.chats.map((chat) => chat.id),
          onDrop: (id2) => actions.drop(String(id2), section.id, null)
        },
        ctx.UI.Row({
          fillMaxWidth: true,
          verticalAlignment: "center",
          height: 24,
          onClick: () => actions.toggleSection(section.id),
          modifier: ctx.Modifier.clip({ cornerRadius: m.categoryRadius })
        }, [
          avatar,
          gap(8),
          ctx.UI.Text({
            text: section.title,
            style: "titleSmall",
            fontWeight: "700",
            maxLines: 1,
            overflow: "ellipsis",
            modifier: ctx.Modifier.widthIn({ max: 170 }),
            color: colors.onSurface
          }),
          gap(8),
          ctx.UI.Text({ text: String(section.chats.length), style: "labelSmall", fontWeight: "700", color: colors.onSurfaceVariant.copy({ alpha: 0.64 }) }),
          ctx.UI.GradientRule({ weight: 1, paddingHorizontal: 10, height: 2, startColor: colors.outlineVariant.copy({ alpha: 0.62 }), endColor: colors.outlineVariant.copy({ alpha: 0 }) }),
          ctx.UI.Icon({ name: collapsed ? "KeyboardArrowDown" : "KeyboardArrowUp", size: 23, tint: colors.onSurfaceVariant.copy({ alpha: 0.78 }) })
        ])
      )
    );
  }
  function groupRow(group, section, ungrouped = false) {
    const expanded = !state.collapsed.includes("group:" + group.id);
    const groupBody = ctx.UI.Row({
      fillMaxWidth: true,
      verticalAlignment: "center",
      paddingStart: 12,
      paddingEnd: 8,
      paddingTop: 6,
      paddingBottom: 6,
      ...{ background: colors.surfaceContainerLow.copy({ alpha: 0.72 }), backgroundShape: { cornerRadius: m.groupRadius }, modifier: ctx.Modifier.border(1, colors.outlineVariant.copy({ alpha: 0.24 }), { cornerRadius: m.groupRadius }) },
      onClick: () => actions.toggleSection("group:" + group.id)
    }, [
      ctx.UI.Icon({ name: "FolderOutlined", size: 16, tint: colors.onSurfaceVariant.copy({ alpha: 0.76 }) }),
      gap(7),
      ctx.UI.Text({ text: group.name, weight: 1, style: "labelLarge", fontWeight: "700", maxLines: 1, overflow: "ellipsis", color: colors.onSurface.copy({ alpha: 1 }) }),
      ...group.pinned ? [gap(4), ctx.UI.Icon({ name: "PushPinRounded", size: 12, tint: colors.onSurfaceVariant.copy({ alpha: 0.65 }) })] : [],
      gap(2),
      ungrouped ? ctx.UI.HoverOnly({ width: 24, height: 24 }, actions.menu("ungrouped", section.id)) : more("group", group.id),
      gap(2),
      ctx.UI.Icon({ name: expanded ? "KeyboardArrowUp" : "KeyboardArrowDown", size: 20, tint: colors.onSurfaceVariant.copy({ alpha: 0.62 }) })
    ]);
    return ctx.UI.Column(
      {
        key: "sidebar-group-" + group.id,
        fillMaxWidth: true,
        paddingStart: m.groupStart,
        paddingEnd: m.groupEnd,
        paddingTop: 4,
        paddingBottom: expanded ? 2 : 0
      },
      ctx.UI.DragTarget(
        {
          key: "sidebar-group-drop-" + group.id,
          acceptedTypes: ["character-sidebar.chat"],
          acceptedData: section.chats.map((chat) => chat.id),
          hoverBorderColor: colors.primary.copy({ alpha: 0.55 }),
          shape: { cornerRadius: m.groupRadius },
          onDrop: (id2) => actions.drop(String(id2), section.id, ungrouped ? null : group.id)
        },
        ctx.UI.HoverRegion({ shape: { cornerRadius: m.groupRadius } }, groupBody)
      )
    );
  }
  const sections = state.data.view === "characters" ? state.data.sections : state.data.scopes.map((scope) => ({
    id: scope.id,
    title: scope.title,
    kind: scope.kind ?? "unbound",
    selection: scope.ownerSelection,
    avatarUri: scope.avatarUri ?? null,
    chats: [...scope.ungrouped, ...scope.groups.flatMap((group) => group.chats)],
    conversationGroups: scope.groups,
    ungroupedChats: scope.ungrouped
  }));
  for (const section of sections) {
    const groups = section.conversationGroups ?? [], ungrouped = section.ungroupedChats ?? section.chats;
    if (section.chats.length === 0 && groups.length === 0) continue;
    const categoryMatches = query === "" || section.title.toLocaleLowerCase().includes(query);
    const visibleGroups = groups.filter((group) => categoryMatches || group.name.toLocaleLowerCase().includes(query) || group.chats.some((chat) => chat.title.toLocaleLowerCase().includes(query)));
    if (!categoryMatches && visibleGroups.length === 0 && !ungrouped.some((chat) => chat.title.toLocaleLowerCase().includes(query))) continue;
    rows.push(category(section));
    if (state.collapsed.includes(section.id)) continue;
    for (const group of visibleGroups) {
      rows.push(groupRow(group, section));
      if (!state.collapsed.includes("group:" + group.id)) rows.push(...chats(group.chats, group.id, section, group.id, categoryMatches || group.name.toLocaleLowerCase().includes(query)));
    }
    if (ungrouped.length > 0 && (categoryMatches || "\u672A\u5206\u7EC4".includes(query) || ungrouped.some((chat) => chat.title.toLocaleLowerCase().includes(query)))) {
      const id2 = "ungrouped:" + section.id;
      rows.push(groupRow({ id: id2, name: "\u672A\u5206\u7EC4", pinned: ungrouped.every((chat) => chat.pinned), displayOrder: 0, chats: ungrouped }, section, true));
      if (!state.collapsed.includes("group:" + id2)) rows.push(...chats(ungrouped, id2, section, null, categoryMatches || "\u672A\u5206\u7EC4".includes(query)));
    }
  }
  if (rows.length === 0) rows.push(ctx.UI.Text({ text: query === "" ? "\u6682\u65E0\u4F1A\u8BDD" : "\u6CA1\u6709\u5339\u914D\u7684\u4F1A\u8BDD", color: muted, fontSize: 12, paddingHorizontal: 20, paddingVertical: 12 }));
  return rows;
}

// src/sidebar-group-delete.ts
function deleteConversationGroupMembers(chatIds, deleteChat) {
  return __async(this, null, function* () {
    if (new Set(chatIds).size !== chatIds.length) throw new Error("Duplicate confirmed folder members");
    const progress = { deletedChatIds: [], failedChatIds: [] };
    for (const chatId of chatIds) {
      try {
        const result = yield deleteChat(chatId);
        if (result.chatId !== chatId || !Number.isFinite(result.deletedAt)) throw new Error("Native deletion did not confirm requested chat: " + chatId);
        progress.deletedChatIds.push(chatId);
      } catch (error) {
        progress.failedChatIds.push({ chatId, error: String(error) });
      }
    }
    return progress;
  });
}

// src/ui-sidebar-native.ts
function renderSidebarScreen(ctx) {
  const [input] = ctx.useState("input", null);
  const [chatSidebar] = ctx.useState("chatSidebar", null);
  const supplied = { input, chatSidebar };
  const current = ctx.useRef("native-sidebar-current", parseCurrentSidebar(supplied));
  current.current = parseCurrentSidebar(supplied);
  const catalogKey = (value) => JSON.stringify({ input: value.input, chats: value.chatSidebar.chats });
  const identity2 = catalogKey(current.current), colors = ctx.MaterialTheme.colorScheme;
  const [state, setState] = ctx.useState("native-sidebar-state", {
    data: null,
    defaultAvatar: null,
    context: "",
    loading: true,
    busy: false,
    error: "",
    query: "",
    search: false,
    collapsed: [],
    expandedGroups: [],
    dialog: null
  });
  const latest = ctx.useRef("native-sidebar-state-ref", state);
  latest.current = state;
  const pending = ctx.useRef("native-sidebar-pending", null);
  function update(change) {
    latest.current = { ...latest.current, ...change };
    setState(latest.current);
  }
  function assertCurrent() {
    if (catalogKey(current.current) !== identity2) throw new Error("The sidebar context changed; refresh before using this action");
  }
  function refresh() {
    return __async(this, null, function* () {
      const request = current.current, key = catalogKey(request);
      const active = pending.current;
      if (active?.key === key && typeof active.promise?.then === "function") return active.promise;
      const promise = load();
      pending.current = { key, promise };
      try {
        yield promise;
      } finally {
        if (pending.current?.promise === promise) pending.current = null;
      }
      function load() {
        return __async(this, null, function* () {
          update({ loading: latest.current.data === null || latest.current.data.view !== request.input.view, error: "" });
          try {
            const data2 = yield ToolPkg.ipc.call("character-sidebar.catalog", request, { targetRuntime: "main" });
            if (catalogKey(current.current) === key) {
              if (data2.view !== request.input.view) throw new Error("The sidebar catalog belongs to another view");
              let defaultAvatar = latest.current.defaultAvatar;
              if ((data2.view === "characters" ? data2.sections : data2.scopes).some((section) => section.kind === "card" && section.avatarUri === null) && defaultAvatar === null) {
                defaultAvatar = yield imageSource(yield ToolPkg.readResource("character_default_avatar", "operit-avatar.png"));
                if (defaultAvatar.trim() === "") throw new Error("Declared default avatar resource is missing");
              }
              const sections = data2.view === "characters" ? data2.sections : data2.scopes;
              for (let offset = 0; offset < sections.length; offset += 8) {
                yield Promise.all(sections.slice(offset, offset + 8).map((section) => __async(null, null, function* () {
                  if (section.avatarUri !== null && section.avatarUri !== void 0)
                    section.avatarUri = yield imageSource(section.avatarUri);
                })));
              }
              if (catalogKey(current.current) === key) update({ data: data2, defaultAvatar, context: key, loading: false });
            }
          } catch (failure) {
            if (catalogKey(current.current) === key) update({ loading: false, context: key, error: String(failure) });
          }
        });
      }
    });
  }
  function refreshInputs() {
    return __async(this, null, function* () {
      if (latest.current.data !== null && latest.current.error === "" && latest.current.context === catalogKey(current.current)) return;
      yield refresh();
    });
  }
  function run(action) {
    return __async(this, null, function* () {
      assertCurrent();
      if (latest.current.busy) throw new Error("A sidebar operation is already running");
      update({ busy: true, error: "" });
      try {
        return yield action();
      } catch (failure) {
        update({ error: String(failure) });
        return null;
      } finally {
        update({ busy: false });
      }
    });
  }
  function activate2(chatId) {
    assertCurrent();
    if (!current.current.chatSidebar.chats.some((chat) => chat.id === chatId)) throw new Error("The conversation is absent from the supplied sidebar history");
    return { type: "toolpkg.chat.activate", chatId };
  }
  function scopeFor(id2) {
    const data2 = visibleCatalog();
    if (data2 === null) throw new Error("Conversation groups are not loaded");
    const scopes = data2.view !== "characters" ? data2.scopes : data2.sections.map((section) => ({
      id: section.id,
      title: section.title,
      ownerSelection: section.selection,
      groups: section.conversationGroups ?? [],
      ungrouped: section.ungroupedChats ?? section.chats
    }));
    const scope = scopes.find((owner2) => owner2.id === id2 || owner2.groups.some((group) => group.id === id2));
    if (scope === void 0) throw new Error("The conversation-group scope no longer exists");
    return scope;
  }
  function readGroup(id2) {
    const group = scopeFor(id2).groups.find((group2) => group2.id === id2);
    if (group === void 0) throw new Error("The native folder no longer exists");
    return group;
  }
  function createChat(selection, groupId) {
    return __async(this, null, function* () {
      assertCurrent();
      if (latest.current.busy) return null;
      update({ busy: true, error: "" });
      try {
        const created = yield Tools.Chat.createNew({ setAsCurrentChat: false, input: selection === "" ? null : createChatInput(selection) });
        yield Tools.Chat.updateGroup([created.chatId], groupId === null ? null : readGroup(groupId).name);
        return { type: "toolpkg.chat.activate", chatId: created.chatId };
      } catch (failure) {
        update({ error: String(failure) });
        return null;
      } finally {
        update({ busy: false });
      }
    });
  }
  function dialog(kind, id2, name = "") {
    assertCurrent();
    update({ dialog: { kind, id: id2, name, progress: null } });
  }
  function submit() {
    return __async(this, null, function* () {
      const staged = latest.current.dialog;
      if (staged === null) throw new Error("No sidebar dialog is open");
      switch (staged.kind) {
        case "chatActions":
          throw new Error("Choose a conversation action");
        case "renameChat":
          yield Tools.Chat.updateTitle(staged.id, staged.name);
          break;
        case "deleteChat":
          yield Tools.Chat.deleteChat(staged.id);
          break;
        case "create": {
          const name = staged.name.trim();
          if (name === "") throw new Error("\u5206\u7EC4\u540D\u79F0\u4E0D\u80FD\u4E3A\u7A7A");
          const selection = scopeFor(staged.id).ownerSelection;
          const created = yield Tools.Chat.createNew({ setAsCurrentChat: false, input: selection === null ? null : createChatInput(selection) });
          yield Tools.Chat.updateGroup([created.chatId], name);
          update({ dialog: null });
          yield refresh();
          return { type: "toolpkg.chat.activate", chatId: created.chatId };
        }
        case "renameUngrouped": {
          yield Tools.Chat.updateGroup(scopeFor(staged.id).ungrouped.map((chat) => chat.id), staged.name.trim());
          break;
        }
        case "deleteUngrouped": {
          const progress = { deletedChatIds: [], failedChatIds: [] };
          for (const chat of scopeFor(staged.id).ungrouped) {
            try {
              yield Tools.Chat.deleteChat(chat.id);
              progress.deletedChatIds.push(chat.id);
            } catch (error) {
              progress.failedChatIds.push({ chatId: chat.id, error: String(error) });
            }
          }
          update({ dialog: { ...staged, progress } });
          yield refresh();
          return null;
        }
        case "rename":
          yield Tools.Chat.updateGroup(readGroup(staged.id).chats.map((chat) => chat.id), staged.name.trim());
          break;
        case "delete": {
          if (staged.progress !== null) throw new Error("This confirmed deletion has already been attempted");
          const group = readGroup(staged.id);
          const progress = yield deleteConversationGroupMembers(group.chats.map((chat) => chat.id), (id2) => Tools.Chat.deleteChat(id2));
          update({ dialog: { ...staged, progress } });
          yield refresh();
          return null;
        }
      }
      update({ dialog: null });
      yield refresh();
      return null;
    });
  }
  function chatFor(id2) {
    const chat = current.current.chatSidebar.chats.find((value) => value.id === id2);
    if (chat === void 0) throw new Error("The conversation no longer exists");
    return chat;
  }
  function renameChat(id2) {
    dialog("renameChat", id2, chatFor(id2).title);
  }
  function deleteChat(id2) {
    dialog("deleteChat", id2, chatFor(id2).title);
  }
  function longPress(id2) {
    dialog("chatActions", id2, chatFor(id2).title);
  }
  function changeFlag(id2, pinned) {
    return __async(this, null, function* () {
      yield run(() => __async(null, null, function* () {
        const chat = chatFor(id2);
        if (pinned) yield Tools.Chat.updatePinned(id2, !chat.pinned);
        else yield Tools.Chat.updateLocked(id2, !chat.locked);
        update({ dialog: null });
        yield refresh();
      }));
    });
  }
  function reorder(chatId, targetId) {
    return __async(this, null, function* () {
      const ids = current.current.chatSidebar.chats.map((chat) => chat.id);
      const from = ids.indexOf(chatId), to = ids.indexOf(targetId);
      if (from < 0 || to < 0) throw new Error("The reordered conversation no longer exists");
      if (from === to) return;
      ids.splice(to, 0, ids.splice(from, 1)[0]);
      yield Tools.Chat.reorder(ids);
    });
  }
  function orderNeighbours(id2) {
    const data2 = visibleCatalog();
    if (data2 === null) return [];
    const scopes = data2.view !== "characters" ? data2.scopes : data2.sections.map((section) => ({ groups: section.conversationGroups ?? [], ungrouped: section.ungroupedChats ?? section.chats }));
    for (const scope of scopes) {
      const values = scope.groups.find((group) => group.chats.some((chat) => chat.id === id2))?.chats ?? scope.ungrouped;
      if (values.some((chat) => chat.id === id2)) return values.map((chat) => chat.id);
    }
    return [];
  }
  function moveRelative(id2, delta) {
    return __async(this, null, function* () {
      const ids = orderNeighbours(id2), index = ids.indexOf(id2), target = ids[index + delta];
      if (index < 0 || target === void 0) return;
      yield run(() => __async(null, null, function* () {
        yield reorder(id2, target);
        update({ dialog: null });
        yield refresh();
      }));
    });
  }
  function menu(kind, id2) {
    if (kind === "chat") {
      const chat = chatFor(id2);
      return ctx.UI.PopupMenu({
        key: "sidebar-chat-menu-" + id2,
        width: 22,
        height: 22,
        tooltip: "\u5BF9\u8BDD\u64CD\u4F5C",
        enabled: !state.busy,
        items: [
          { label: "\u7F16\u8F91\u540D\u79F0", icon: "Edit" },
          { label: chat.pinned ? "\u53D6\u6D88\u7F6E\u9876" : "\u7F6E\u9876", icon: "PushPin" },
          { label: chat.locked ? "\u89E3\u9501" : "\u9501\u5B9A", icon: chat.locked ? "LockOpen" : "Lock" },
          { label: "\u5220\u9664", icon: "DeleteOutline", danger: true }
        ],
        onSelected: (index) => __async(null, null, function* () {
          if (index === 0) renameChat(id2);
          else if (index === 1) yield changeFlag(id2, true);
          else if (index === 2) yield changeFlag(id2, false);
          else if (index === 3) deleteChat(id2);
        })
      }, ctx.UI.Icon({ name: "MoreHoriz", size: 16, tint: colors.onSurfaceVariant.copy({ alpha: 0.78 }) }));
    }
    if (kind === "ungrouped") {
      const scope = scopeFor(id2);
      const pinned = scope.ungrouped.length > 0 && scope.ungrouped.every((chat) => chat.pinned);
      return ctx.UI.PopupMenu({
        key: "sidebar-ungrouped-menu-" + id2,
        width: 22,
        height: 22,
        menuMaxWidth: 138,
        tooltip: "\u5206\u7EC4\u64CD\u4F5C",
        enabled: !state.busy && state.data !== null,
        items: [
          { label: "\u65B0\u5EFA\u5BF9\u8BDD", icon: "AddCommentOutlined" },
          { label: "\u7F16\u8F91\u540D\u79F0", icon: "EditOutlined" },
          { label: pinned ? "\u53D6\u6D88\u7F6E\u9876" : "\u7F6E\u9876", icon: pinned ? "PushPinOutlined" : "PushPinRounded" },
          { label: "\u5220\u9664", icon: "DeleteOutlineRounded", danger: true, dividerBefore: true }
        ],
        onSelected: (index) => __async(null, null, function* () {
          if (index === 0) return createChat(scope.ownerSelection ?? "", null);
          if (index === 1) dialog("renameUngrouped", id2, "\u672A\u5206\u7EC4");
          if (index === 2) yield run(() => __async(null, null, function* () {
            for (const chat of scope.ungrouped) yield Tools.Chat.updatePinned(chat.id, !pinned);
            yield refresh();
          }));
          if (index === 3) dialog("deleteUngrouped", id2, "\u672A\u5206\u7EC4");
          return null;
        })
      }, ctx.UI.Icon({ name: "MoreHorizRounded", size: 16, tint: colors.onSurfaceVariant.copy({ alpha: 0.78 }) }));
    }
    const group = scopeFor(id2).groups.find((value) => value.id === id2);
    const items = [
      { label: "\u65B0\u5EFA\u5BF9\u8BDD", icon: "AddCommentOutlined" },
      { label: "\u7F16\u8F91\u540D\u79F0", icon: "EditOutlined" },
      { label: group.pinned ? "\u53D6\u6D88\u7F6E\u9876" : "\u7F6E\u9876", icon: group.pinned ? "PushPinOutlined" : "PushPinRounded" },
      { label: "\u5220\u9664", icon: "DeleteOutlineRounded", danger: true, dividerBefore: true }
    ];
    return ctx.UI.PopupMenu({
      key: "sidebar-group-menu-" + id2,
      width: 22,
      height: 22,
      menuMaxWidth: 138,
      tooltip: "\u5206\u7EC4\u64CD\u4F5C",
      items,
      enabled: !state.busy,
      onSelected: (index) => __async(null, null, function* () {
        if (index === 0) {
          const owner2 = scopeFor(id2).ownerSelection;
          return createChat(owner2 ?? "", id2);
        } else if (index === 1) dialog("rename", id2, group.name);
        else if (index === 2) yield run(() => __async(null, null, function* () {
          for (const chat of group.chats) yield Tools.Chat.updatePinned(chat.id, !group.pinned);
          yield refresh();
        }));
        else if (index === 3) dialog("delete", id2, group.name);
        return null;
      })
    }, ctx.UI.Icon({ name: "MoreHorizRounded", size: 16, tint: colors.onSurfaceVariant.copy({ alpha: 0.78 }) }));
  }
  function drop(chatId, scopeId, groupId, targetId) {
    return __async(this, null, function* () {
      assertCurrent();
      if (latest.current.busy) throw new Error("A sidebar operation is already running");
      const scope = scopeFor(scopeId);
      if (!scope.ungrouped.some((chat) => chat.id === chatId) && !scope.groups.some((group) => group.chats.some((chat) => chat.id === chatId)))
        throw new Error("The conversation no longer belongs to the target scope");
      if (groupId !== null && !scope.groups.some((group) => group.id === groupId))
        throw new Error("The target conversation group no longer exists");
      const existing = scope.groups.find((group) => group.chats.some((chat) => chat.id === chatId))?.id ?? null;
      if (existing === groupId && targetId === void 0) return;
      update({ busy: true, error: "" });
      try {
        if (existing !== groupId) yield Tools.Chat.updateGroup([chatId], groupId === null ? null : readGroup(groupId).name);
        if (targetId !== void 0) yield reorder(chatId, targetId);
        yield refresh();
      } catch (failure) {
        update({ error: String(failure) });
        throw failure;
      } finally {
        update({ busy: false });
      }
    });
  }
  function visibleCatalog() {
    const data2 = latest.current.data;
    return data2 === null || data2.view !== current.current.input.view ? null : latest.current.context === catalogKey(current.current) ? data2 : reconcileSidebarCatalog(data2, current.current);
  }
  const data = visibleCatalog();
  const toggleSection = (id2) => update({ collapsed: state.collapsed.includes(id2) ? state.collapsed.filter((key) => key !== id2) : [...state.collapsed, id2] });
  const toggleGroup = (id2) => update({ expandedGroups: state.expandedGroups.includes(id2) ? state.expandedGroups.filter((key) => key !== id2) : [...state.expandedGroups, id2] });
  const rows = [ctx.UI.Row({
    key: "sidebar-toolbar",
    fillMaxWidth: true,
    verticalAlignment: "center",
    spacing: 8,
    paddingStart: 14,
    paddingEnd: 12,
    paddingBottom: 8
  }, [
    ctx.UI.Surface(
      {
        key: "sidebar-create-bar",
        weight: 1,
        height: 34,
        containerColor: colors.primaryContainer,
        contentColor: colors.onPrimaryContainer,
        shape: { type: "pill" }
      },
      ctx.UI.Row({ fillMaxSize: true, verticalAlignment: "center" }, [
        ctx.UI.Row({
          key: "sidebar-new-chat",
          weight: 1,
          height: 34,
          verticalAlignment: "center",
          horizontalArrangement: "center",
          onClick: () => __async(null, null, function* () {
            const active = yield callMainDomain("activePrompt.get", {});
            return createChat(encodeSelection(active), null);
          }),
          spacing: 6
        }, [
          ctx.UI.Icon({ name: "AddRounded", size: 17, tint: colors.onPrimaryContainer }),
          ctx.UI.Text({ text: "\u65B0\u5EFA\u5BF9\u8BDD", fontSize: 13, fontWeight: "600", color: colors.onPrimaryContainer, ...{ letterSpacing: -0.1 } })
        ]),
        ctx.UI.Box({ width: 1, height: 16, background: colors.onPrimaryContainer.copy({ alpha: 0.16 }) }),
        ctx.UI.Row({
          key: "sidebar-new-group",
          width: 38,
          height: 34,
          verticalAlignment: "center",
          horizontalArrangement: "center",
          onClick: () => __async(null, null, function* () {
            const active = yield callMainDomain("activePrompt.get", {});
            const selection = encodeSelection(active);
            const catalog = visibleCatalog();
            if (catalog === null) throw new Error("Sidebar has not loaded");
            const scope = catalog.view === "characters" ? catalog.sections.find((section) => section.selection === selection) : catalog.scopes.find((scope2) => scope2.ownerSelection === selection);
            if (scope === void 0) throw new Error("The active owner is absent from the actual sidebar catalog");
            dialog("create", scope.id);
          })
        }, ctx.UI.Icon({ name: "CreateNewFolderOutlined", size: 16, tint: colors.onPrimaryContainer, contentDescription: "\u65B0\u5EFA\u5206\u7EC4" }))
      ])
    ),
    ctx.UI.Row(
      {
        key: "sidebar-search-toggle",
        width: 34,
        height: 34,
        verticalAlignment: "center",
        horizontalArrangement: "center",
        onClick: () => update({ search: !state.search }),
        modifier: ctx.Modifier.clip({ cornerRadius: 6 }),
        background: state.search ? colors.secondaryContainer.copy({ alpha: 0.35 }) : colors.surface.copy({ alpha: 0 })
      },
      ctx.UI.Icon({ name: state.search ? "SearchOff" : "Search", size: 17, tint: colors.onSurfaceVariant.copy({ alpha: 0.78 }), contentDescription: state.search ? "\u6536\u8D77\u641C\u7D22" : "\u641C\u7D22\u5BF9\u8BDD" })
    )
  ])];
  if (state.search) rows.push(ctx.UI.TextField({
    key: "sidebar-search",
    value: state.query,
    onValueChange: (query) => update({ query }),
    placeholder: "\u641C\u7D22\u5BF9\u8BDD",
    singleLine: true,
    fillMaxWidth: true,
    paddingHorizontal: 12,
    paddingBottom: 12,
    ...{ shape: { cornerRadius: 14 } }
  }));
  if (state.error !== "") rows.push(ctx.UI.Text({ key: "sidebar-error", text: state.error, fontSize: 11, color: colors.error, paddingHorizontal: 20, paddingVertical: 6 }));
  if (data !== null) rows.push(...nativeSidebarRows(ctx, current.current, {
    data,
    query: state.query,
    collapsed: state.collapsed,
    expandedGroups: state.expandedGroups
  }, {
    activate: activate2,
    toggleSection,
    toggleGroup,
    menu,
    longPress,
    rename: renameChat,
    deleteChat,
    drop
  }, state.defaultAvatar));
  else if (state.loading) rows.push(ctx.UI.Box(
    { fillMaxWidth: true, height: 80, contentAlignment: "center" },
    ctx.UI.LoadingIndicator({ size: 24 })
  ));
  const content = [ctx.UI.Column({ key: "native-sidebar-history", fillMaxWidth: true, paddingBottom: 16, spacing: 0 }, rows)];
  if (state.dialog !== null) {
    const staged = state.dialog, body = [ctx.UI.Text({ text: { create: "\u65B0\u5EFA\u5206\u7EC4", rename: "\u91CD\u547D\u540D\u5206\u7EC4", delete: "\u5220\u9664\u5206\u7EC4\u53CA\u5176\u5BF9\u8BDD", renameUngrouped: "\u91CD\u547D\u540D\u5206\u7EC4", deleteUngrouped: "\u5220\u9664\u5206\u7EC4\u53CA\u5176\u5BF9\u8BDD", renameChat: "\u7F16\u8F91\u5BF9\u8BDD\u540D\u79F0", deleteChat: "\u5220\u9664\u5BF9\u8BDD", chatActions: "\u804A\u5929\u8BB0\u5F55" }[staged.kind], fontSize: 16, fontWeight: "600" })];
    if (staged.kind === "create" || staged.kind === "rename" || staged.kind === "renameUngrouped" || staged.kind === "renameChat") body.push(ctx.UI.TextField({ key: "sidebar-group-name", value: staged.name, label: staged.kind === "renameChat" ? "\u5BF9\u8BDD\u540D\u79F0" : "\u5206\u7EC4\u540D\u79F0", onValueChange: (name) => update({ dialog: { ...staged, name } }), enabled: !state.busy }));
    if (staged.kind === "delete" || staged.kind === "deleteUngrouped" || staged.kind === "deleteChat") body.push(ctx.UI.Text({ text: staged.kind === "deleteChat" ? `\u786E\u8BA4\u5220\u9664\u5BF9\u8BDD\u201C${staged.name}\u201D\uFF1F\u6B64\u64CD\u4F5C\u4E0D\u53EF\u64A4\u9500\u3002` : `\u786E\u8BA4\u5220\u9664\u201C${staged.name}\u201D\u53CA\u5176\u4E2D\u7684\u5168\u90E8\u5BF9\u8BDD\uFF1F\u6B64\u64CD\u4F5C\u4E0D\u53EF\u64A4\u9500\u3002`, fontSize: 12 }));
    if (staged.kind === "chatActions") {
      const chat = chatFor(staged.id), ids = orderNeighbours(staged.id), index = ids.indexOf(staged.id);
      body.push(ctx.UI.Text({ text: chat.title, fillMaxWidth: true, maxLines: 2, style: "titleMedium", color: colors.onSurfaceVariant }));
      const tile = (label, icon, callback, enabled = true, danger = false) => ctx.UI.Surface(
        { fillMaxWidth: true, onClick: enabled ? callback : void 0, shape: { cornerRadius: 12 }, color: colors.surface },
        ctx.UI.Row({ fillMaxWidth: true, paddingHorizontal: 16, paddingVertical: 10, verticalAlignment: "center", spacing: 12 }, [
          ctx.UI.Icon({ name: icon, size: 20, tint: danger ? colors.error : colors.onSurfaceVariant }),
          ctx.UI.Text({ text: label, weight: 1, fontSize: 14, color: danger ? colors.error : colors.onSurface })
        ])
      );
      body.push(
        tile("\u7F16\u8F91\u540D\u79F0", "Edit", () => renameChat(chat.id)),
        tile("\u4E0A\u79FB", "KeyboardArrowUp", () => moveRelative(chat.id, -1), index > 0),
        tile("\u4E0B\u79FB", "KeyboardArrowDown", () => moveRelative(chat.id, 1), index >= 0 && index < ids.length - 1),
        tile(chat.pinned ? "\u53D6\u6D88\u7F6E\u9876" : "\u7F6E\u9876", "PushPin", () => changeFlag(chat.id, true)),
        tile(chat.locked ? "\u89E3\u9501" : "\u9501\u5B9A", chat.locked ? "LockOpen" : "Lock", () => changeFlag(chat.id, false)),
        tile("\u5220\u9664", "DeleteOutline", () => deleteChat(chat.id), true, true)
      );
    }
    if (staged.progress !== null) body.push(ctx.UI.Text({ key: "sidebar-delete-result", text: `\u5DF2\u5220\u9664 ${staged.progress.deletedChatIds.length} \u6761\u5BF9\u8BDD
` + staged.progress.failedChatIds.map((value) => value.chatId + ": " + value.error).join("\n"), fontSize: 11, color: staged.progress.failedChatIds.length === 0 ? colors.onSurface : colors.error }));
    if (state.error !== "") body.push(ctx.UI.Text({ text: state.error, color: colors.error, fontSize: 11 }));
    body.push(ctx.UI.Row({ spacing: 8 }, [
      ctx.UI.TextButton({ text: "\u53D6\u6D88", enabled: !state.busy, onClick: () => update({ dialog: null }) }),
      ...staged.progress === null && staged.kind !== "chatActions" ? [ctx.UI.Button({ text: "\u786E\u8BA4", enabled: !state.busy, onClick: () => run(submit) })] : []
    ]));
    content.push(ctx.UI.Dialog({ key: "sidebar-edit-dialog", closeOnDismissRequest: !state.busy, onDismissRequest: () => {
      if (!state.busy) update({ dialog: null });
    } }, ctx.UI.Column({ padding: 20, spacing: 12, width: 320 }, body)));
  }
  return ctx.UI.Column({ key: "native-character-sidebar", fillMaxWidth: true, onLoad: refresh, onInputsChanged: refreshInputs }, content);
}

// src/ui-sidebar.ts
function exactFields(value, expected, label) {
  const object = record(value, label), keys = Object.keys(object);
  if (keys.length !== expected.length || keys.some(
    /** Rejects every field not explicitly declared by this boundary. */
    (key) => expected.indexOf(key) === -1
  )) throw new Error(label + " has invalid fields");
  return object;
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
  const chat = exactFields(value, ["id", "title", "updatedAt", "displayOrder", "workspaceId", "workspaceName", "locked", "pinned", "group"], "sidebar chat");
  if (chat.group !== null && typeof chat.group !== "string") throw new Error("sidebar chat.group must be a string or null");
  const id2 = sidebarIdentity(chat.id, "sidebar chat.id");
  if (typeof chat.title !== "string" || typeof chat.updatedAt !== "string") throw new Error("sidebar chat title and updatedAt must be strings");
  if (typeof chat.displayOrder !== "number" || !Number.isSafeInteger(chat.displayOrder)) throw new Error("sidebar chat.displayOrder must be a safe integer");
  if (chat.workspaceId !== null && typeof chat.workspaceId !== "string") throw new Error("sidebar chat.workspaceId must be a string or null");
  if (chat.workspaceName !== null && typeof chat.workspaceName !== "string") throw new Error("sidebar chat.workspaceName must be a string or null");
  if (typeof chat.locked !== "boolean" || typeof chat.pinned !== "boolean") throw new Error("sidebar chat locked and pinned must be booleans");
  return { id: id2, title: chat.title, updatedAt: chat.updatedAt, displayOrder: chat.displayOrder, workspaceId: chat.workspaceId, workspaceName: chat.workspaceName, locked: chat.locked, pinned: chat.pinned, group: chat.group };
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
function conversationSidebarScope(section) {
  const byName = /* @__PURE__ */ new Map(), ungrouped = [];
  for (const chat of section.chats) {
    const name = chat.group?.trim();
    if (!name) {
      ungrouped.push(chat);
      continue;
    }
    const members = byName.get(name) ?? [];
    members.push(chat);
    byName.set(name, members);
  }
  const groups = [...byName].map(([name, chats]) => ({
    id: section.id + ":folder:" + JSON.stringify(name),
    name,
    pinned: chats.length > 0 && chats.every((chat) => chat.pinned),
    displayOrder: Math.min(...chats.map((chat) => chat.displayOrder)),
    chats: chats.sort(compareSidebarChats)
  }));
  groups.sort((a, b) => Number(b.pinned) - Number(a.pinned) || a.displayOrder - b.displayOrder || a.name.localeCompare(b.name));
  return {
    id: section.id,
    title: section.title,
    ownerSelection: section.selection,
    avatarUri: section.avatarUri,
    kind: section.kind,
    groups,
    ungrouped: ungrouped.sort(compareSidebarChats)
  };
}
function reconcileSidebarCatalog(catalog, current) {
  const live = new Map(current.chatSidebar.chats.map((chat) => [chat.id, chat]));
  const sections = catalog.view === "characters" ? catalog.sections : catalog.scopes.map((scope) => ({
    id: scope.id,
    title: scope.title,
    kind: scope.kind ?? "unbound",
    selection: scope.ownerSelection,
    avatarUri: scope.avatarUri ?? null,
    chats: [...scope.ungrouped, ...scope.groups.flatMap((group) => group.chats)],
    conversationGroups: scope.groups
  }));
  const projected = sections.map((section) => {
    const chats = section.chats.flatMap((chat) => {
      const updated = live.get(chat.id);
      return updated === void 0 ? [] : [updated];
    }).sort(compareSidebarChats);
    const scope = conversationSidebarScope({ ...section, chats });
    for (const group of scope.groups) group.id = section.conversationGroups?.find((previous) => previous.name === group.name)?.id ?? group.id;
    return { section: { ...section, chats, conversationGroups: scope.groups, ungroupedChats: scope.ungrouped }, scope };
  });
  return catalog.view === "characters" ? { view: "characters", sections: projected.map((value) => value.section) } : { view: "groups", scopes: projected.map((value) => value.scope) };
}

// src/ui/chat-sidebar/index.ui.ts
function Screen(ctx) {
  return renderSidebarScreen(ctx);
}

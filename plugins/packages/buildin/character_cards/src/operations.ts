import { readCharacter, createCharacterDraft, createGroupDraft, createTagDraft } from "./drafts";
import { characterModelBindingMode, sameCharacterModelBinding } from "./model-binding";
import type { DomainInput, DomainOperation, DomainOutput } from "./api";
import type { ChatConfigurationService } from "./domain";
import { parseDomainPayload, resolveChatConfiguration } from "./domain";
import { combineCharacterPrompt } from "./character-prompt";
import { listBindableChats } from "./chat-binding-catalog";
import { parseChatSelection } from "./chat-bindings";
import { clearFailedCandidates, enqueueCandidate, retryFailedCandidates, startMemoryRebuild } from "./memory-jobs/scheduler";
import { listMemoryChats, pruneMissingMemoryChats, requireChat } from "./memory-jobs/chat";
import type { CharacterRepository } from "./canonical";
import type { Card, Group, GroupValues, Memory, MemoryGraphResult, MemoryOwner, MemorySearchOptions, MemorySettings, MemorySearchConfig, Request, RequestOutput, Snapshot, Store, StoreValues, Tag, TagChanges, TagEdit, TagValues, MemoryLink } from "./model";
import { assertBoolean, assertGroup, assertGroupValues, assertMemory, assertMemorySettings, assertObject, assertSearchConfig, assertSearchOptions, assertString, assertStrings, assertTag, assertTagValues, normalizeNames, requireId, requireName, assertNumber } from "./validation";
import { decodeCharacterImport, decodeGroupImport, encodeCharacterExport } from "./serialization";
import { readThemeChoices } from "./selection-application";
import { independentConfigurations } from "./selection-settings";
import { uuid } from "./identity";
import { autoCategorizeMemory, updateChatMemory } from "./memory-jobs/analysis";
import { rebuildMemoryEmbeddings } from "./memory-jobs/embeddings";
import { decodeCharacterBackup, decodeGroupBackup, decodeMemoryBackup, exportCharacterBackup, exportGroupBackup, exportMemoryBackup } from "./backup";

interface TagPlan {
  created: { draftId: string; values: TagValues }[];
  updated: Tag[];
  deleted: string[];
  attachedIds: string[];
}
/** Builds the editor aggregate exclusively from independent real canonical reads. */
export async function snapshot(host: CharacterRepository): Promise<Snapshot> {
  const [
    cards,
    groups,
    stores,
    tags,
    active,
    models,
    ttsConfigs,
    toolCatalog,
    memoryPreferences,
  ] = await Promise.all([
    host.listCharacters(),
    host.listGroups(),
    host.listStores(),
    host.listTags(),
    host.readActive(),
    Promise.resolve().then(() => host.listModels()),
    Promise.resolve().then(() => host.listTtsConfigs()),
    Promise.resolve().then(() => host.readToolCatalog()),
    host.readGlobalMemoryPreferences(),
  ]);
  return {
    cards,
    groups,
    stores,
    tags,
    active,
    models,
    ttsConfigs,
    toolCatalog,
    memoryPreferences,
  };
}

/** Requires exactly one canonical record instead of silently choosing an ambiguous identity. */
function findRecord<T extends { id: string }>(
  values: T[],
  id: string,
  label: string,
): T {
  let found: T | undefined;
  for (const value of values) {
    if (value.id !== id) continue;
    if (found !== undefined) throw new Error(`${label} 标识重复：${id}`);
    found = value;
  }
  if (found === undefined) throw new Error(`${label} 不存在：${id}`);
  return found;
}

/** Prepares every staged tag mutation and attached-id mapping before any write is issued. */
function prepareTagChanges(
  attachedIds: string[],
  changes: TagChanges,
  tags: Tag[],
): TagPlan {
  assertStrings(attachedIds, "角色标签");
  assertObject(changes, "标签修改");
  if (!Array.isArray(changes.created) || !Array.isArray(changes.updated))
    throw new Error("标签修改格式无效");
  assertStrings(changes.deleted, "删除标签标识");
  const created: TagPlan["created"] = [];
  const updated: Tag[] = [];
  const deleted = new Set<string>();
  const changedIds = new Set<string>();
  const draftIds = new Set<string>();
  for (const id of changes.deleted) {
    requireId(id, "删除标签标识");
    findRecord(tags, id, "标签");
    if (deleted.has(id)) throw new Error("标签删除项重复");
    deleted.add(id);
  }
  for (const draft of changes.created) {
    assertObject(draft, "新标签");
    const draftId = requireId(draft.draftId, "新标签草稿标识");
    if (draftIds.has(draftId)) throw new Error("标签草稿标识重复");
    for (const tag of tags)
      if (tag.id === draftId) throw new Error("标签草稿标识与已有标签冲突");
    assertTagValues(draft.values);
    const values = normalizedTagValues(draft.values);
    draftIds.add(draftId);
    created.push({ draftId, values });
  }
  for (const edit of changes.updated) {
    assertTagValues(edit);
    const id = requireId(edit.id, "修改标签标识");
    if (changedIds.has(id) || deleted.has(id))
      throw new Error("同一标签存在冲突修改");
    const original = findRecord(tags, id, "标签");
    changedIds.add(id);
    updated.push({ ...original, ...normalizedTagValues(edit) });
  }
  const projected: { id: string; name: string }[] = [];
  for (const tag of tags) {
    if (deleted.has(tag.id)) continue;
    let current = tag;
    for (const update of updated) if (update.id === tag.id) current = update;
    projected.push({ id: tag.id, name: current.name.trim() });
  }
  for (const draft of created)
    projected.push({ id: draft.draftId, name: draft.values.name });
  for (const draft of created)
    for (const tag of projected)
      if (tag.id !== draft.draftId && tag.name === draft.values.name)
        throw new Error(`标签名称已存在：${draft.values.name}`);
  for (const update of updated) {
    const original = findRecord(tags, update.id, "标签");
    if (original.name.trim() === update.name) continue;
    for (const tag of projected)
      if (tag.id !== update.id && tag.name === update.name)
        throw new Error(`标签名称已存在：${update.name}`);
  }
  const attached: string[] = [];
  const attachedSet = new Set<string>();
  for (const id of attachedIds) {
    requireId(id, "角色标签标识");
    if (deleted.has(id)) continue;
    if (!draftIds.has(id)) findRecord(tags, id, "角色标签");
    if (attachedSet.has(id)) continue;
    attachedSet.add(id);
    attached.push(id);
  }
  return { created, updated, deleted: [...deleted], attachedIds: attached };
}

/** Normalizes the editable tag fields without discarding prompt content or tag type. */
function normalizedTagValues(values: TagValues): TagValues {
  return {
    name: requireName(values.name, "标签名称"),
    description: values.description.trim(),
    promptContent: values.promptContent,
    tagType: values.tagType,
  };
}

/** Commits a validated staged plan only as part of the enclosing explicit save. */
async function commitTagChanges(
  plan: TagPlan,
  host: CharacterRepository,
): Promise<string[]> {
  const resolved = new Map<string, string>();
  for (const draft of plan.created) {
    const created = await host.createTag(draft.values);
    assertTag(created);
    resolved.set(draft.draftId, created.id);
  }
  for (const tag of plan.updated) {
    const saved = await host.updateTag(tag);
    }
  for (const id of plan.deleted) await host.deleteTag(id);
  const ids: string[] = [];
  for (const id of plan.attachedIds) {
    if (resolved.has(id)) {
      const canonicalId = resolved.get(id);
      if (canonicalId === undefined)
        throw new Error("新标签没有返回 canonical 标识");
      ids.push(canonicalId);
    } else ids.push(id);
  }
  return normalizeNames(ids);
}

/** Validates an explicitly created or changed model binding before publishing it. */
function assertCharacterModelBinding(card: Card): void {
  const mode = characterModelBindingMode(card.chatModelBindingMode);
  if (mode !== "FOLLOW_GLOBAL" && mode !== "FIXED_MODEL")
    throw new Error("聊天模型绑定模式无效，请重新绑定模型");
  if (mode === "FIXED_MODEL") {
    requireId(card.chatProviderId, "固定聊天模型提供商");
    requireId(card.chatModelId, "固定聊天模型");
  }
}

/** Normalizes editable content while preserving stored model and memory selections for unrelated edits. */
function normalizedCharacter(card: Card): Card {
  if (
    card.memoryBindingMode !== "CHARACTER" &&
    card.memoryBindingMode !== "SHARED"
  )
    throw new Error("记忆绑定模式无效");
  if (card.ttsConfigId !== null) requireId(card.ttsConfigId, "TTS 配置");
  if (card.themeConfigId !== null) requireId(card.themeConfigId, "主题配置");
  const policy = card.toolAccessConfig;
  if (
    policy.enabled &&
    (policy.allowedPackages.length !== 0 ||
      policy.allowedSkills.length !== 0 ||
      policy.allowedMcpServers.length !== 0) &&
    !new Set(normalizeNames(policy.allowedBuiltinTools)).has("use_package")
  )
    throw new Error("选择外部工具来源时必须允许 use_package");
  const mounts = [];
  const mountIds = new Set<string>();
  for (const mount of card.sharedMemoryMounts) {
    if (mountIds.has(mount.sharedMemoryId)) throw new Error("共享记忆挂载重复");
    mountIds.add(mount.sharedMemoryId);
    mounts.push({
      sharedMemoryId: mount.sharedMemoryId,
      readable: mount.readable,
      writable: mount.writable,
    });
  }
  return {
    ...card,
    name: requireName(card.name, "角色卡名称"),
    description: card.description.trim(),
    attachedTagIds: [...card.attachedTagIds],
    sharedMemoryMounts: mounts,
    toolAccessConfig: {
      enabled: card.toolAccessConfig.enabled,
      allowedBuiltinTools: normalizeNames(
        card.toolAccessConfig.allowedBuiltinTools,
      ),
      allowedPackages: normalizeNames(card.toolAccessConfig.allowedPackages),
      allowedSkills: normalizeNames(card.toolAccessConfig.allowedSkills),
      allowedMcpServers: normalizeNames(
        card.toolAccessConfig.allowedMcpServers,
      ),
    },
  };
}

/** Saves one complete character and its staged tag edits in the enclosing plugin snapshot operation. */
export async function saveCharacter(
  card: Card,
  create: boolean,
  tagChanges: TagChanges,
  host: CharacterRepository,
): Promise<Card> {

  assertBoolean(create, "角色新建标识");
  let value = normalizedCharacter(readCharacter(card));
  let validateMemoryBinding = create;
  if (create) {
    assertCharacterModelBinding(value);
    value = { ...value, id: "", createdAt: 0, updatedAt: 0 };
  } else {
    const id = requireId(value.id, "角色卡标识");
    const original = await host.getCharacter(id);

    if (!sameCharacterModelBinding(original, value)) assertCharacterModelBinding(value);
    validateMemoryBinding = original.memoryBindingMode !== value.memoryBindingMode || original.sharedMemoryId !== value.sharedMemoryId;
    value = {
      ...value,
      id: original.id,
      createdAt: original.createdAt,
    };
  }
  const [tags, stores, cards] = await Promise.all([
    host.listTags(),
    host.listStores(),
    host.listCharacters(),
  ]);
  let retainedName = false;
  if (!create)
    for (const existing of cards)
      if (existing.id === value.id && existing.name.trim() === value.name)
        retainedName = true;
  if (!retainedName)
    for (const existing of cards)
      if (existing.id !== value.id && existing.name.trim() === value.name)
        throw new Error(`角色卡名称已存在：${value.name}`);
  if (value.ttsConfigId !== null) {
    const configs = await host.listTtsConfigs();
    findRecord(configs, value.ttsConfigId, "TTS 配置");
  }
  const plan = prepareTagChanges(value.attachedTagIds, tagChanges, tags);
  if (validateMemoryBinding && value.memoryBindingMode === "SHARED")
    findRecord(
      stores,
      requireId(value.sharedMemoryId, "共享记忆库标识"),
      "共享记忆库",
    );
  for (const mount of value.sharedMemoryMounts)
    findRecord(stores, mount.sharedMemoryId, "共享记忆库");
  value = { ...value, attachedTagIds: await commitTagChanges(plan, host) };
  const saved = create
    ? await host.createCharacter(value)
    : await host.updateCharacter(value);

  requireId(saved.id, "保存的角色卡标识");
  return saved;
}

/** Validates and saves every editable group field while retaining canonical creation metadata. */
export async function saveGroup(
  group: GroupValues,
  create: boolean,
  host: CharacterRepository,
): Promise<Group> {
  assertGroupValues(group);
  assertBoolean(create, "群组新建标识");
  const name = requireName(group.name, "角色组名称");
  const cards = await host.listCharacters();
  const members = [];
  for (const member of group.members) {
    findRecord(cards, member.characterCardId, "群组角色");
    members.push({
      characterCardId: member.characterCardId,
      orderIndex: member.orderIndex,
    });
  }
  let value: Group;
  if (create)
    value = {
      id: "",
      name,
      description: group.description.trim(),
      members,
      themeConfigId: group.themeConfigId,
      createdAt: 0,
      updatedAt: 0,
    };
  else {
    const original = await host.getGroup(requireId(group.id, "群组标识"));
    assertGroup(original);
    value = {
      ...original,
      name,
      description: group.description.trim(),
      members,
      themeConfigId: group.themeConfigId,
    };
  }
  const saved = create
    ? await host.createGroup(value)
    : await host.updateGroup(value);
  return saved;
}

/** Writes every editable tag field in one typed host operation. */
export async function saveTag(
  tag: TagEdit,
  create: boolean,
  host: CharacterRepository,
): Promise<Tag> {
  assertTagValues(tag);
  assertBoolean(create, "标签新建标识");
  const values = normalizedTagValues(tag);
  let saved: Tag;
  if (create) saved = await host.createTag(values);
  else {
    const tags = await host.listTags();
    const original = findRecord(tags, requireId(tag.id, "标签标识"), "标签");
    saved = await host.updateTag({ ...original, ...values });
  }
  assertTag(saved);
  return saved;
}

/** Creates or renames an authoritative shared store without copying its metadata into plugin config. */
export async function saveStore(
  store: StoreValues,
  create: boolean,
  host: CharacterRepository,
): Promise<Store> {
  assertObject(store, "共享记忆库");
  assertBoolean(create, "记忆库新建标识");
  const name = requireName(store.name, "共享记忆库名称");
  let saved: Store;
  if (create) saved = await host.createStore(name);
  else {
    const id = requireId(store.id, "共享记忆库标识");
    findRecord(
      await host.listStores(),
      id,
      "共享记忆库",
    );
    saved = await host.renameStore(id, name);
  }
  return saved;
}

/** Requires a real card or group before changing the canonical active prompt. */
export async function activate(
  type: "card" | "group",
  id: string,
  host: CharacterRepository,
): Promise<void> {
  requireId(id, "当前提示词标识");
  if (type === "card") {
    await host.getCharacter(id);
    await host.writeActive({ CharacterCard: { id } });
  } else if (type === "group") {
    assertGroup(await host.getGroup(id));
    await host.writeActive({ CharacterGroup: { id } });
  } else throw new Error("当前提示词类型无效");
}

/** Parses the exact Rust character/shared memory namespace without guessing strings. */
export function parseOwner(ownerKey: string): MemoryOwner {
  assertString(ownerKey, "记忆库标识");
  const parts = ownerKey.split(":");
  if (parts.length !== 2 || !/^[^:\s]+$/.test(parts[1]))
    throw new Error("记忆库标识格式无效");
  if (parts[0] === "character") return { kind: "CHARACTER", id: parts[1] };
  if (parts[0] === "shared") return { kind: "SHARED", id: parts[1] };
  throw new Error("记忆库命名空间无效");
}

/** Resolves memory only from an explicitly bound character without assuming an installation card. */
async function menuMemoryOwner(
  chatId: string,
  host: CharacterRepository,
): Promise<string> {
  requireId(chatId, "memory menu chat id");
  const binding = await host.chatBindings.findChatBinding(chatId);
  const selection =
    binding === null ? null : parseChatSelection(binding.selection);
  if (selection?.kind === "model") throw new Error("Model-only conversations have no role memory owner");
  if (selection?.kind === "card") return resolveMemoryOwner(selection.id, host);
  throw new Error("Select a character memory owner for this conversation");
}

/** Requires the selected owner to exist in the canonical character or shared store registry. */
export async function requireOwner(
  ownerKey: string,
  host: Pick<CharacterRepository, "getCharacter" | "listStores">,
): Promise<MemoryOwner> {
  const owner = parseOwner(ownerKey);
  if (owner.kind === "CHARACTER") await host.getCharacter(owner.id);
  else
    findRecord(
      await host.listStores(),
      owner.id,
      "共享记忆库",
    );
  return owner;
}

/** Resolves the primary memory namespace from the character's complete persisted binding. */
export async function resolveMemoryOwner(
  characterId: string,
  host: Pick<CharacterRepository, "getCharacter" | "listStores">,
): Promise<string> {
  const card = await host.getCharacter(requireId(characterId, "角色卡标识"));

  let ownerKey: string;
  if (card.memoryBindingMode === "CHARACTER") ownerKey = `character:${card.id}`;
  else if (card.memoryBindingMode === "SHARED")
    ownerKey = `shared:${requireId(card.sharedMemoryId, "共享记忆库绑定")}`;
  else throw new Error("记忆绑定模式无效");
  await requireOwner(ownerKey, host);
  return ownerKey;
}

/** Reads full canonical memories for one validated owner. */
export async function listMemories(
  ownerKey: string,
  host: CharacterRepository,
): Promise<Memory[]> {
  await requireOwner(ownerKey, host);
  return await host.listMemories(ownerKey);
}

/** Reads the actual UUID graph and complete records without projecting lossy tool query results. */
export async function graph(
  ownerKey: string,
  host: CharacterRepository,
): Promise<MemoryGraphResult> {
  await requireOwner(ownerKey, host);
  const [value, items] = await Promise.all([
    host.readMemoryGraph(ownerKey),
    host.listMemories(ownerKey),
  ]);
  return { graph: value, items };
}

/** Normalizes a user-selected folder using the canonical slash-delimited repository contract. */
function normalizeFolderPath(value: string | null): string | null {
  if (value === null) return null;
  const parts: string[] = [];
  for (const part of value.replace(/\\/g, "/").split("/")) {
    const name = part.trim();
    if (name !== "") parts.push(name);
  }
  return parts.length === 0 ? null : parts.join("/");
}

/** Performs full-record canonical search with explicit owner scope and explicit filters. */
export async function searchMemories(
  options: MemorySearchOptions,
  host: CharacterRepository,
): Promise<Memory[]> {
  assertSearchOptions(options);
  await requireOwner(options.ownerKey, host);
  return await host.searchMemories({
      ...options,
      folderPath: normalizeFolderPath(options.folderPath),
    });
}

/** Combines setting, mode-specific content, staged canonical tags, and advanced prompt in source order. */
export async function combinePrompts(
  characterId: string,
  additionalTagIds: string[],
  promptFunctionType: "CHAT" | "VOICE",
  host: Pick<CharacterRepository, "getCharacter" | "getTag">,
): Promise<string> {
  const card = await host.getCharacter(requireId(characterId, "角色卡标识"));
  return combineCharacterPrompt(card, additionalTagIds, promptFunctionType, host);
}

/** Saves full settings through the owner service rather than writing preference files. */
export async function writeMemorySettings(
  ownerKey: string,
  settings: MemorySettings,
  host: CharacterRepository,
): Promise<MemorySettings> {
  assertMemorySettings(settings);
  await requireOwner(ownerKey, host);
  await host.writeMemorySettings(ownerKey, { ...settings });
  return host.readMemorySettings(ownerKey);
}

/** Saves all search configuration fields through the canonical owner service. */
export async function writeMemorySearchConfig(
  ownerKey: string,
  config: MemorySearchConfig,
  host: CharacterRepository,
): Promise<MemorySearchConfig> {
  assertSearchConfig(config);
  await requireOwner(ownerKey, host);
  await host.writeMemorySearchConfig(ownerKey, { ...config });
  return host.readMemorySearchConfig(ownerKey);
}

/** Dispatches plugin-owned domain operations against the explicitly supplied plugin-local repository. */
export function dispatch<R extends Request>(
  request: R,
  host: CharacterRepository,
): Promise<RequestOutput<R>>;

/** Executes the complete discriminated editor protocol against one canonical repository. */
export async function dispatch(
  request: Request,
  host: CharacterRepository,
): Promise<RequestOutput> {
  assertObject(request, "角色卡请求");
  switch (request.action) {
    case "snapshot":
      return snapshot(host);
    case "readGlobalMemoryPreferences":
      return host.readGlobalMemoryPreferences();
    case "readChatMemoryAutoSaveStatus":
      return host.readMemoryAutoSaveStatus(
        await menuMemoryOwner(request.chatId, host),
      );
    case "updateCurrentChatMemory":
      return executeDomain(
        "memory.chat.update",
        {
          ownerKey: await menuMemoryOwner(request.chatId, host),
          chatId: request.chatId,
        },
        host,
      );
    case "retryFailedCandidates":
      return executeDomain(
        "memory.candidates.retryFailed",
        { ownerKey: "ownerKey" in request ? request.ownerKey : await menuMemoryOwner(request.chatId, host) },
        host,
      );
    case "clearFailedCandidates":
      return executeDomain(
        "memory.candidates.clearFailed",
        { ownerKey: "ownerKey" in request ? request.ownerKey : await menuMemoryOwner(request.chatId, host) },
        host,
      );
    case "writeGlobalMemoryPreference":
      return host.writeGlobalMemoryPreference(request.key, request.value);
    case "readMemoryRecallSettings":
      return host.readMemoryRecallSettings();
    case "writeMemoryRecallSettings":
      return host.writeMemoryRecallSettings(request.settings);
    case "searchMemoryDebug": {
      await requireOwner(request.ownerKey, host);
      assertSearchConfig(request.config);
      return host.searchMemoryDebug(
        {
          ownerKey: request.ownerKey,
          query: request.query,
          folderPath: null,
          relevanceThreshold: request.relevanceThreshold,
          createdAtStartMs: null,
          createdAtEndMs: null,
        },
        request.config,
      );
    }
    case "readMemoryLink":
      await requireOwner(request.ownerKey, host);
      return host.readMemoryLink(request.ownerKey, request.linkId);
    case "listThemeChoices":
      return readThemeChoices(independentConfigurations);
    case "listCharacters":
      return host.listCharacters();
    case "getCharacter":
      return host.getCharacter(requireId(request.id, "角色卡标识"));
    case "saveCharacter":
      await saveCharacter(
        request.card,
        request.create,
        request.tagChanges,
        host,
      );
      return snapshot(host);
    case "deleteCharacter":
      await deleteCharacter(request.id, host);
      return snapshot(host);
    case "activate":
      await activate(request.type, request.id, host);
      return snapshot(host);
    case "readActive":
      return host.readActive();
    case "listBindableChats":
      return listBindableChats(host.chatBindings);
    case "readChatBinding":
      return executeDomain(
        "chat.configuration.binding.read",
        { chatId: request.chatId },
        host,
      );
    case "writeChatBinding":
      return executeDomain(
        "chat.configuration.binding.write",
        { chatId: request.chatId, selection: request.selection },
        host,
      );
    case "deleteChatBinding":
      return executeDomain(
        "chat.configuration.binding.delete",
        { chatId: request.chatId },
        host,
      );
    case "listGroups":
      return host.listGroups();
    case "getGroup": {
      const group = await host.getGroup(requireId(request.id, "群组标识"));
      assertGroup(group);
      return group;
    }
    case "saveGroup":
      await saveGroup(request.group, request.create, host);
      return snapshot(host);
    case "deleteGroup":
      await host.deleteGroup(requireId(request.id, "群组标识"));
      return snapshot(host);
    case "listTags":
      return host.listTags();
    case "saveTag":
      return saveTag(request.tag, request.create, host);
    case "deleteTag":
      await host.deleteTag(requireId(request.id, "标签标识"));
      return snapshot(host);
    case "listStores":
      return host.listStores();
    case "saveStore":
      await saveStore(request.store, request.create, host);
      return snapshot(host);
    case "deleteStore":
      await host.deleteStore(requireId(request.id, "共享记忆库标识"));
      return snapshot(host);
    case "listModels":
      return host.listModels();
    case "listTtsConfigs":
      return host.listTtsConfigs();
    case "readToolCatalog":
      return host.readToolCatalog();
    case "resolveMemoryOwner":
      return resolveMemoryOwner(request.characterId, host);
    case "readUser":
      return executeDomain("memory.user.read", { ownerKey: request.ownerKey }, host);
    case "writeUser":
      await executeDomain("memory.user.write", { ownerKey: request.ownerKey, content: request.content }, host);
      return { saved: true };
    case "graph":
      return graph(request.ownerKey, host);
    case "listMemories":
      return listMemories(request.ownerKey, host);
    case "searchMemory":
      return searchMemories(
        {
          ownerKey: request.ownerKey,
          query: request.query,
          folderPath: null,
          relevanceThreshold: 0,
          createdAtStartMs: null,
          createdAtEndMs: null,
        },
        host,
      );
    case "searchMemories":
      return searchMemories(request.options, host);
    case "saveMemory": {
      const values = {
        title: request.title,
        content: request.content,
        contentType: request.contentType.trim(),
        source: request.source.trim(),
        credibility: request.credibility,
        importance: request.importance,
        folderPath: request.folderPath,
        tags: normalizeNames(request.tags.split(",")),
      };
      if (request.originalTitle === null) await executeDomain("memory.create", { ownerKey: request.ownerKey, values }, host);
      else await executeDomain("memory.update", { ownerKey: request.ownerKey, originalTitle: request.originalTitle, changes: values }, host);
      return graph(request.ownerKey, host);
    }
    case "deleteMemory": {
      const { item } = await executeDomain("memory.get", { ownerKey: request.ownerKey, title: request.title }, host);
      await executeDomain("memory.delete", { ownerKey: request.ownerKey, id: item.id }, host);
      return graph(request.ownerKey, host);
    }
    case "createLink":
      await executeDomain("memory.link.create", { ownerKey: request.ownerKey, sourceTitle: request.sourceTitle, targetTitle: request.targetTitle, linkType: request.linkType, weight: request.weight, description: request.description }, host);
      return graph(request.ownerKey, host);
    case "deleteLink":
      await executeDomain("memory.link.delete", { ownerKey: request.ownerKey, linkId: request.linkId }, host);
      return graph(request.ownerKey, host);
    case "updateLink":
      await executeDomain("memory.link.update", { ownerKey: request.ownerKey, linkId: request.linkId, changes: { linkType: request.linkType, weight: request.weight, description: request.description } }, host);
      return graph(request.ownerKey, host);
    case "readMemorySettings": {
      await requireOwner(request.ownerKey, host);
      const settings = await host.readMemorySettings(request.ownerKey);
      assertMemorySettings(settings);
      return settings;
    }
    case "writeMemorySettings":
      return writeMemorySettings(request.ownerKey, request.settings, host);
    case "readMemorySearchConfig": {
      await requireOwner(request.ownerKey, host);
      const config = await host.readMemorySearchConfig(request.ownerKey);
      assertSearchConfig(config);
      return config;
    }
    case "writeMemorySearchConfig":
      return writeMemorySearchConfig(request.ownerKey, request.config, host);
    case "writeMemoryOwnerSettings": {
      // Validate both drafts before mutation; the repository owner publishes this entire request atomically.
      assertMemorySettings(request.settings);
      assertSearchConfig(request.config);
      await requireOwner(request.ownerKey, host);
      return host.writeMemoryOwnerSettings(request.ownerKey, request.settings, request.config);
    }
    case "combinePrompts":
      return combinePrompts(
        request.characterId,
        request.additionalTagIds,
        request.promptFunctionType,
        host,
      );
    case "importCharacter": {
      const tags = await host.listTags();
      const plan = decodeCharacterImport(request.content, request.format, tags);
      await saveCharacter(plan.card, true, plan.tagChanges, host);
      return snapshot(host);
    }
    case "exportCharacter": {
      const card = await host.getCharacter(requireId(request.id, "角色卡标识"));
      const tags = await host.listTags();
      return encodeCharacterExport(card, request.format, tags);
    }
    case "importGroup":
      await saveGroup(decodeGroupImport(request.content), true, host);
      return snapshot(host);
    case "exportGroup": {
      const group = await host.getGroup(requireId(request.id, "群组标识"));
      assertGroup(group);
      return JSON.stringify(group, null, 2);
    }
    case "listMemoryFolders":
      await requireOwner(request.ownerKey, host);
      return host.listMemoryFolders(request.ownerKey);
    case "readMemoryAutoSaveStatus":
      await requireOwner(request.ownerKey, host);
      return host.readMemoryAutoSaveStatus(request.ownerKey);
    case "listMemoryChats":
      return executeDomain(
        "memory.chat.list",
        { ownerKey: request.ownerKey },
        host,
      );
    case "pruneMissingChatBindings":
      return executeDomain(
        "memory.chat.pruneMissing",
        { ownerKey: request.ownerKey },
        host,
      );
    case "updateChatMemory":
      return executeDomain(
        "memory.chat.update",
        { ownerKey: request.ownerKey, chatId: request.chatId },
        host,
      );
    case "autoCategorizeMemory":
      return executeDomain(
        "memory.categorize",
        { ownerKey: request.ownerKey },
        host,
      );
    case "startMemoryRebuild":
      return executeDomain(
        "memory.rebuild.start",
        { ownerKey: request.ownerKey, rebuild: request.rebuild },
        host,
      );
    case "readMemoryRebuildProgress":
      return executeDomain(
        "memory.rebuild.progress",
        { ownerKey: request.ownerKey },
        host,
      );
    case "cancelMemoryRebuild":
      return executeDomain(
        "memory.rebuild.cancel",
        { ownerKey: request.ownerKey },
        host,
      );
    case "rebuildMemoryEmbeddings":
      return executeDomain(
        "memory.embeddings.rebuild",
        { ownerKey: request.ownerKey },
        host,
      );
    case "exportMemory":
      return executeDomain("memory.export", { ownerKey: request.ownerKey }, host);
    case "importMemory":
      return executeDomain("memory.import", { ownerKey: request.ownerKey, content: request.content, strategy: request.strategy }, host);
    default: {
      const action: never = request;
      throw new Error(`未知角色卡请求：${JSON.stringify(action)}`);
    }
  }
}



type DomainHandlers = { [K in DomainOperation]: (input: DomainInput<K>, host: CharacterRepository) => Promise<DomainOutput<K>> };

/** Converts canonical deletion lifecycle completion to the public activation-independent result. */
async function deleteCharacter(id: string, host: CharacterRepository): Promise<{ id: string; deleted: boolean }> {
  await host.deleteCharacter(requireId(id, "角色卡标识")); return { id, deleted: true };
}

/** Applies a complete relationship edit to its existing canonical scope and identity. */
async function updateLink(input: DomainInput<"memory.link.update">, host: CharacterRepository): Promise<{ ownerKey: string; link: MemoryLink }> {
  await requireOwner(input.ownerKey, host);
  const original = await host.readMemoryLink(input.ownerKey, input.linkId);
  const changes: { type_?: string; weight?: number; description?: string } = {};
  if (Object.prototype.hasOwnProperty.call(input.changes, "linkType")) changes.type_ = requireName(input.changes.linkType, "关系类型");
  if (Object.prototype.hasOwnProperty.call(input.changes, "weight")) { assertNumber(input.changes.weight, "关系强度", 0, 1); changes.weight = input.changes.weight; }
  if (Object.prototype.hasOwnProperty.call(input.changes, "description")) changes.description = input.changes.description;
  const value = { ...original, ...changes };
  const link = await host.updateLink(input.ownerKey, original.id, { type_: value.type_, weight: value.weight, description: value.description });
  return { ownerKey: input.ownerKey, link };
}

/** Activates only an explicitly supplied actor; persisted chat bindings use their separate typed APIs. */
async function activateForChat(input: DomainInput<"activePrompt.activateForChat">, host: CharacterRepository): Promise<DomainOutput<"activePrompt.activateForChat">> {
  if (input.characterGroupId !== null && input.characterCardName !== null) throw new Error("Chat activation cannot select both a card and a group");
  if (input.characterGroupId !== null && input.characterGroupId.trim() !== "") await activate("group", input.characterGroupId.trim(), host);
  else if (input.characterCardName !== null && input.characterCardName.trim() !== "") {
    const name = input.characterCardName.trim(), cards = await host.listCharacters();
    let target: Card | undefined;
    for (const card of cards) if (card.name === name) { if (target !== undefined) throw new Error("聊天角色名称不唯一"); target = card; }
    if (target === undefined) throw new Error(`聊天角色不存在：${name}`);
    await activate("card", target.id, host);
  } else throw new Error("Chat activation requires an explicit character or group selection");
  return { characterCardName: input.characterCardName, characterGroupId: input.characterGroupId, updated: true };
}

/** Applies live privacy preferences to current and historical configuration reads. */
export async function resolveConfiguration(
  input: DomainInput<"chat.configuration.resolve">,
  service: ChatConfigurationService,
): Promise<DomainOutput<"chat.configuration.resolve">> {
  const result = await resolveChatConfiguration(input, service);
  if (!("profile" in result) || (await service.readGlobalMemoryPreferences()).userDescriptionEnabled) return result;
  return {
    ...result,
    profile: { ...result.profile, userPreferencesText: "" },
    participants: result.participants.map(profile => ({ ...profile, userPreferencesText: "" })),
  };
}

/** Implements every published domain operation with typed handlers sharing the editor's actual business service. */
const handlers: DomainHandlers = {
  /** Ranks persisted document source vectors without writing during retrieval. */
  "memory.chunks": (input, repository) => repository.searchMemoryChunks(input.ownerKey, input.memoryId, input.query, input.count),
  /** Searches bounded records and document chunks with one operation-scoped query vector. */
  "memory.searchWithOptions": async (input, repository) => {
    await requireOwner(input.ownerKey, repository);
    return repository.searchMemoryPage({ ...input, folderPath: normalizeFolderPath(input.folderPath) });
  },
  /** Lists actual owner-bound generic chat summaries. */
  "memory.chat.list": (input, repository) => listMemoryChats(input.ownerKey, repository),
  "memory.chat.pruneMissing": (input, repository) => pruneMissingMemoryChats(input.ownerKey, repository),
  /** Extracts real chat messages through the configured functional MEMORY model. */
  "memory.chat.update": (input, repository) => updateChatMemory(input.ownerKey, input.chatId, repository),
  /** Performs genuine ten-record functional categorization in the same operation. */
  "memory.categorize": (input, repository) => autoCategorizeMemory(input.ownerKey, repository),
  /** Stores a complete requested rebuild plan for real interval execution. */
  "memory.rebuild.start": (input, repository) => startMemoryRebuild(input.ownerKey, input.rebuild, repository),
  /** Reads the real durable job counters. */
  "memory.rebuild.progress": (input, repository) => repository.readMemoryRebuildProgress(input.ownerKey),
  /** Records an explicit cancellation before another source window may be processed. */
  "memory.rebuild.cancel": async (input, repository) => { await repository.cancelMemoryRebuild(input.ownerKey); return repository.readMemoryRebuildProgress(input.ownerKey); },
  /** Rebuilds actual configured provider embeddings using generic HTTP. */
  "memory.embeddings.rebuild": (input, repository) => rebuildMemoryEmbeddings(input.ownerKey, repository),
  /** Enqueues only actual persisted message sources and genuine bound owners. */
  "memory.candidate.enqueue": (input, repository) => enqueueCandidate(input, repository),
  /** Returns genuine terminal failures to the pending queue for one later interval retry. */
  "memory.candidates.retryFailed": (input, repository) => retryFailedCandidates(input.ownerKey, repository),
  /** Removes genuine terminal failures from the owner queue. */
  "memory.candidates.clearFailed": (input, repository) => clearFailedCandidates(input.ownerKey, repository),
  /** Resolves chat configuration using this operation's existing repository snapshot. */
  "chat.configuration.resolve": (input, repository) => resolveConfiguration(input, {
    /** Reads only the requested character. */
    getCharacter: (id) => repository.getCharacter(id),
    /** Reads only the requested group. */
    getGroup: (id) => repository.getGroup(id),
    /** Reads only a referenced tag. */
    getTag: (id) => repository.getTag(id),
    /** Reads only a referenced shared library. */
    getStore: (id) => repository.getStore(id),
    /** Reads only the model directory. */
    listModels: () => repository.listModels(),
    /** Reads only the speech directory. */
    listTtsConfigs: () => repository.listTtsConfigs(),
    readGlobalMemoryPreferences: () => repository.readGlobalMemoryPreferences(),
    dispatchDomain: <K extends DomainOperation>(operation: K, payload: DomainInput<K>) => executeDomain(operation, payload, repository),
  }),
  /** Requires the chat's persisted selection rather than global active state. */
  "chat.configuration.binding.find": (input, repository) => repository.chatBindings.findChatBinding(input.chatId),
  "chat.configuration.binding.read": (input, repository) => repository.chatBindings.readChatBinding(input.chatId),
  /** Stages a genuine selector submission in the shared publication. */
  "chat.configuration.binding.write": async (input, repository) => { await requireChat(input.chatId); return repository.chatBindings.writeChatBinding(input); },
  /** Deletes the requested existing binding through the same file operation. */
  "chat.configuration.binding.delete": (input, repository) => repository.chatBindings.deleteChatBinding(input.chatId),
  /** Reads the complete real editor directories. */
  snapshot: (_input, host) => snapshot(host),
  /** Reads complete canonical character cards. */
  "character.list": (_input, host) => host.listCharacters(),
  /** Requires the selected canonical character. */
  "character.get": (input, host) => host.getCharacter(input.id),
  /** Initializes an explicit new form and persists all provided character fields. */
  "character.create": (input, host) => saveCharacter({ ...createCharacterDraft(), ...input.values }, true, { created: [], updated: [], deleted: [] }, host),
  /** Merges an explicit edit into the complete canonical record before one manager write. */
  "character.update": async (input, host) => { const original = await host.getCharacter(input.id); return saveCharacter({ ...original, ...input.changes, id: original.id }, false, { created: [], updated: [], deleted: [] }, host); },
  /** Deletes any existing character using the canonical lifecycle. */
  "character.delete": (input, host) => deleteCharacter(input.id, host),
  /** Activates the selected canonical character. */
  "character.setActive": async (input, host) => { await activate("card", input.id, host); return { type: "character_card", id: input.id, active: true }; },
  /** Composes the actual canonical prompt using the explicitly selected function and tags. */
  "character.combine": async (input, host) => {
    const options = { promptFunctionType: "CHAT" as const, additionalTagIds: [] as string[], ...input };
    const prompt = await combinePrompts(input.id, options.additionalTagIds, options.promptFunctionType, host);
    return { id: input.id, promptFunctionType: options.promptFunctionType, additionalTagIds: options.additionalTagIds, prompt };
  },
  /** Serializes the plugin-owned character interchange representation. */
  "character.export": async (input, host) => { const card = await host.getCharacter(input.id), tags = await host.listTags(); return { id: input.id, format: input.format, content: encodeCharacterExport(card, input.format, tags) }; },
  /** Parses and stages interchange imports in the plugin before full canonical persistence. */
  "character.import": async (input, host) => { const plan = decodeCharacterImport(input.content, input.format, await host.listTags()); return saveCharacter(plan.card, true, plan.tagChanges, host); },
  /** Exports the actual canonical records using the original manager backup format. */
  "character.exportBackup": async (_input, host) => ({ content: await exportCharacterBackup(host) }),
  /** Validates the backup in the plugin before invoking canonical identity-preserving restore. */
  "character.importBackup": async (input, host) => { const document = decodeCharacterBackup(input.content); const result = await host.restoreCharacters(document.characterCards, document.promptTags); await host.restoreConversationGroups(document.conversationGroups); return result; },
  /** Reads complete canonical groups. */
  "group.list": async (_input, host) => { const groups = await host.listGroups(); for (const group of groups) assertGroup(group); return groups; },
  /** Requires the selected canonical group. */
  "group.get": async (input, host) => { const group = await host.getGroup(input.id); assertGroup(group); return group; },
  /** Persists every field of an explicit new group. */
  "group.create": (input, host) => saveGroup({ ...createGroupDraft(), ...input.values }, true, host),
  /** Merges an explicit edit into the full canonical group before saving. */
  "group.update": async (input, host) => { const original = await host.getGroup(input.id); return saveGroup({ ...original, ...input.changes, id: original.id }, false, host); },
  /** Deletes the group through the plugin repository lifecycle. */
  "group.delete": async (input, host) => { await host.getGroup(input.id); await host.deleteGroup(input.id); return { id: input.id, deleted: true }; },
  /** Activates the selected real group. */
  "group.setActive": async (input, host) => { await activate("group", input.id, host); return { type: "character_group", id: input.id, active: true }; },
  /** Copies the full canonical group's editable state into an intentional new group. */
  "group.duplicate": async (input, host) => { const group = await host.getGroup(input.id); const options = { newName: group.name, ...input }; return saveGroup({ ...group, id: "", name: requireName(options.newName, "新群组名称") }, true, host); },
  /** Exports the complete canonical group serde record. */
  "group.export": async (input, host) => { const group = await host.getGroup(input.id); assertGroup(group); return { id: input.id, content: JSON.stringify(group, null, 2) }; },
  /** Parses and validates a full imported group before canonical creation. */
  "group.import": (input, host) => saveGroup(decodeGroupImport(input.content), true, host),
  /** Exports the actual canonical group backup shape. */
  "group.exportBackup": async (_input, host) => ({ content: await exportGroupBackup(host) }),
  /** Validates the group backup before canonical identity-preserving restoration. */
  "group.importBackup": (input, host) => host.restoreGroups(decodeGroupBackup(input.content).characterGroups),
  /** Reads the canonical externally tagged active prompt. */
  "activePrompt.get": (_input, host) => host.readActive(),
  /** Selects model execution explicitly without applying any role-owned configuration. */
  "activePrompt.setModel": async (_input, host) => { await host.writeActive({ Model: { id: "chat" } }); return { type: "model", id: "chat", active: true }; },
  /** Activates the selected canonical character. */
  "activePrompt.setCard": async (input, host) => { await activate("card", input.id, host); return { type: "character_card", id: input.id, active: true }; },
  /** Activates the selected canonical group. */
  "activePrompt.setGroup": async (input, host) => { await activate("group", input.id, host); return { type: "character_group", id: input.id, active: true }; },
  /** Applies the plugin's explicit chat-binding selection logic. */
  "activePrompt.activateForChat": (input, host) => activateForChat(input, host),
  /** Resolves the real next-send actor under the canonical active card or group selection. */
  "activePrompt.resolvedCard": async (_input, host) => { const active = await host.readActive(); if (!("CharacterCard" in active)) throw new Error("An active group requires an explicit execution participant"); const card = await host.getCharacter(active.CharacterCard.id); return { id: card.id }; },
  /** Reads complete canonical tags. */
  "tag.list": async (_input, host) => { const tags = await host.listTags(); for (const tag of tags) assertTag(tag); return tags; },
  /** Requires an existing complete canonical tag. */
  "tag.get": async (input, host) => { const tag = await host.getTag(input.id); assertTag(tag); return tag; },
  /** Initializes a new tag form and writes every provided editable field. */
  "tag.create": (input, host) => saveTag({ id: "", ...createTagDraft(), ...input.values }, true, host),
  /** Writes every editable field by merging the patch into the complete canonical tag. */
  "tag.update": async (input, host) => { const original = await host.getTag(input.id); return saveTag({ ...original, ...input.changes, id: original.id }, false, host); },
  /** Deletes an existing canonical prompt tag. */
  "tag.delete": async (input, host) => { await host.getTag(input.id); await host.deleteTag(input.id); return { id: input.id, deleted: true }; },
  /** Reads complete canonical shared stores. */
  "memory.shared.list": (_input, host) => host.listStores(),
  /** Creates the canonical shared store. */
  "memory.shared.create": (input, host) => saveStore({ id: "", name: input.name }, true, host),
  /** Renames the selected canonical shared store. */
  "memory.shared.rename": (input, host) => saveStore({ id: input.id, name: input.name }, false, host),
  /** Removes the shared store and counts the actual canonical character bindings affected. */
  "memory.shared.delete": async (input, host) => {
    const cards = await host.listCharacters(); let cleanedCharacters = 0;
    for (const card of cards) {
      let affected = card.sharedMemoryId === input.id;
      for (const mount of card.sharedMemoryMounts) if (mount.sharedMemoryId === input.id) affected = true;
      if (affected) cleanedCharacters += 1;
    }
    await host.deleteStore(input.id); return { sharedId: input.id, deleted: true, cleanedCharacters };
  },
  /** Saves an explicit shared-store mount through the full canonical character writer. */
  "memory.mount": async (input, host) => {
    await host.getStore(input.sharedId);
    const original = await host.getCharacter(input.characterId);
    const mount = { sharedMemoryId: input.sharedId, readable: input.readable, writable: input.writable };
    const mounts = [];
    for (const value of original.sharedMemoryMounts) if (value.sharedMemoryId !== input.sharedId) mounts.push(value);
    mounts.push(mount);
    await saveCharacter({ ...original, sharedMemoryMounts: mounts }, false, { created: [], updated: [], deleted: [] }, host);
    return { characterId: input.characterId, sharedId: input.sharedId, mount, mounted: true };
  },
  /** Removes only the explicitly selected mount through a full canonical character write. */
  "memory.unmount": async (input, host) => {
    const original = await host.getCharacter(input.characterId), mounts = [];
    for (const mount of original.sharedMemoryMounts) if (mount.sharedMemoryId !== input.sharedId) mounts.push(mount);
    await saveCharacter({ ...original, sharedMemoryMounts: mounts }, false, { created: [], updated: [], deleted: [] }, host);
    return { characterId: input.characterId, sharedId: input.sharedId, unmounted: true };
  },
  /** Reads the selected real USER.md contents. */
  "memory.user.read": async (input, host) => { await requireOwner(input.ownerKey, host); return host.readUser(input.ownerKey); },
  /** Writes owner-scoped USER.md without touching preference files. */
  "memory.user.write": async (input, host) => { await requireOwner(input.ownerKey, host); await host.writeUser(input.ownerKey, input.content); return { ownerKey: input.ownerKey, contentLength: input.content.length, updated: true }; },
  /** Reads the real plugin-owned USER.md path accepted by Files. */
  "memory.user.path": async (input, host) => { await requireOwner(input.ownerKey, host); return { ownerKey: input.ownerKey, path: await host.readUserPath(input.ownerKey) }; },
  /** Resolves the primary canonical memory binding to its real owner enum. */
  "memory.resolveOwner": async (input, host) => parseOwner(await resolveMemoryOwner(input.characterId, host)),
  /** Reads complete canonical owner settings. */
  "memory.settings.read": async (input, host) => { await requireOwner(input.ownerKey, host); return host.readMemorySettings(input.ownerKey); },
  /** Writes complete canonical owner settings. */
  "memory.settings.write": (input, host) => writeMemorySettings(input.ownerKey, input.settings, host),
  /** Reads the complete persisted search configuration. */
  "memory.searchConfig.read": async (input, host) => { await requireOwner(input.ownerKey, host); return host.readMemorySearchConfig(input.ownerKey); },
  /** Writes every persisted search configuration field. */
  "memory.searchConfig.write": (input, host) => writeMemorySearchConfig(input.ownerKey, input.config, host),
  /** Reads the actual repository graph with canonical UUID nodes. */
  "memory.graph": async (input, host) => ({ ownerKey: input.ownerKey, graph: (await graph(input.ownerKey, host)).graph }),
  /** Reads complete repository memories in the selected owner scope. */
  "memory.list": async (input, host) => ({ ownerKey: input.ownerKey, items: await listMemories(input.ownerKey, host) }),
  /** Searches complete repository records with the original explicit editor search filters. */
  "memory.search": async (input, host) => ({ ownerKey: input.ownerKey, query: input.query, items: await searchMemories({ ...input, folderPath: null, relevanceThreshold: 0, createdAtStartMs: null, createdAtEndMs: null }, host) }),
  /** Resolves an exact title to one complete canonical memory. */
  "memory.get": async (input, host) => ({ ownerKey: input.ownerKey, item: await host.readMemoryByTitle(input.ownerKey, input.title) }),
  /** Initializes an intentional full new record and writes all memory metadata in the plugin snapshot. */
  "memory.create": async (input, host) => {
    await requireOwner(input.ownerKey, host);
    const values = { id: "", uuid: "", contentType: "text", source: "", credibility: 0.5, importance: 0.5,
      documentPath: null, isDocumentNode: false, chunkIndexFilePath: null, folderPath: null, createdAt: 0, updatedAt: 0,
      lastAccessedAt: 0, tags: [] as string[], properties: [], ...input.values };
    assertStrings(values.tags, "记忆标签");
    const tags = []; for (const name of normalizeNames(values.tags)) tags.push({ id: uuid(), name });
    const memory = { ...values, id: "", uuid: "", title: requireName(values.title, "记忆标题"), tags };
    const item = await host.saveMemory(input.ownerKey, memory); assertMemory(item);
    return { ownerKey: input.ownerKey, item, created: true };
  },
  /** Merges every supplied memory field into its full canonical record while preserving untouched metadata. */
  "memory.update": async (input, host) => {
    const original = await host.readMemoryByTitle(input.ownerKey, input.originalTitle);
    if (input.expectedId !== undefined && input.expectedId !== original.id) throw new Error("Memory identity changed since editing began");
    const changes = { ...input.changes };
    const tags = [];
    if (Object.prototype.hasOwnProperty.call(changes, "tags")) { assertStrings(changes.tags, "记忆标签"); for (const name of normalizeNames(changes.tags)) tags.push({ id: uuid(), name }); }
    else for (const tag of original.tags) tags.push(tag);
    const memory = { ...original, ...changes, id: original.id, tags };
    assertMemory(memory);
    const item = await host.saveMemory(input.ownerKey, memory); assertMemory(item);
    return { ownerKey: input.ownerKey, item, updated: true };
  },
  /** Deletes the exact owner-scoped canonical memory id. */
  "memory.delete": async (input, host) => { await requireOwner(input.ownerKey, host); await host.deleteMemory(input.ownerKey, input.id); return { ownerKey: input.ownerKey, id: input.id, deleted: true }; },
  /** Moves the explicitly selected ids using the canonical repository's bulk operation. */
  "memory.move": async (input, host) => {
    await requireOwner(input.ownerKey, host);
    const selected = new Set(input.ids);
    await host.moveMemories(input.ownerKey, [...selected], input.folderPath);
    return { ownerKey: input.ownerKey, ids: [...selected], folder: input.folderPath, moved: selected.size };
  },
  /** Resolves exact source and target records before creating a canonical relationship. */
  "memory.link.create": async (input, host) => {
    await requireOwner(input.ownerKey, host);
    const source = await host.readMemoryByTitle(input.ownerKey, input.sourceTitle), target = await host.readMemoryByTitle(input.ownerKey, input.targetTitle);
    if (source.id === target.id) throw new Error("不能创建记忆自身关系");
    const link = await host.createLink(input.ownerKey, { sourceMemoryId: source.id, targetMemoryId: target.id, type_: requireName(input.linkType, "关系类型"), weight: input.weight, description: input.description });
    return { ownerKey: input.ownerKey, link };
  },
  /** Applies a full canonical relationship edit to the selected id. */
  "memory.link.update": (input, host) => updateLink(input, host),
  /** Deletes the exact owner-scoped canonical relationship id. */
  "memory.link.delete": async (input, host) => { await requireOwner(input.ownerKey, host); await host.deleteLink(input.ownerKey, input.linkId); return { ownerKey: input.ownerKey, linkId: input.linkId, deleted: true }; },
  /** Serializes the repository's portable backup format in the plugin. */
  "memory.export": async (input, host) => { await requireOwner(input.ownerKey, host); return { ownerKey: input.ownerKey, content: await exportMemoryBackup(input.ownerKey, host) }; },
  /** Validates the complete portable backup before notification-preserving canonical restoration. */
  "memory.import": async (input, host) => { await requireOwner(input.ownerKey, host); const document = decodeMemoryBackup(input.content); const result = await host.importMemorySpace(input.ownerKey, document.space, input.strategy); await host.writeUser(input.ownerKey, document.userMarkdown); return { ownerKey: input.ownerKey, result }; },
};

/** Validates and invokes one exact typed handler without a CLI, tool alias, or alternate dispatch transport. */
export function executeDomain<K extends DomainOperation>(operation: K, input: DomainInput<K>, host: CharacterRepository): Promise<DomainOutput<K>> {
  return handlers[operation](parseDomainPayload(operation, input), host);
}

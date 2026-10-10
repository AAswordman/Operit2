import type { DomainInput, DomainOperation, DomainOutput } from "./api";
import { executeDomain } from "./operations";
import { MemoryJobRunner } from "./memory-jobs/scheduler";
import { decodeMemoryBackup, exportMemoryBackup } from "./backup";
import type { CharacterRepository, CharacterRepositoryOwner } from "./canonical";
import type {
  ActivePrompt, Card, Group, GroupValues, Memory, MemoryGraphResult, MemoryOwner,
  MemorySearchOptions, MemorySettings, MemorySearchConfig, MemoryValues, Request, RequestOutput,
  Snapshot, Store, StoreValues, Tag, TagChanges, TagEdit, TagValues,
} from "./model";
import {
  assertBoolean, assertCard, assertGraph, assertGroup, assertGroupValues, assertInteger,
  assertMemory, assertMemorySettings, assertMemoryValues, assertNumber, assertObject,
  assertSearchConfig, assertSearchOptions, assertString, assertStrings, assertTag,
  assertTagValues, normalizeNames, requireDecimal, requireId, requireName,
} from "./validation";
import { decodeCharacterImport, decodeGroupImport, encodeCharacterExport } from "./serialization";
import { createSelectionApplication, readSelectionReferences, readThemeChoices } from "./selection-application";
import type { IndependentConfigurationAccess } from "./selection-application";
import { independentConfigurations } from "./selection-settings";
import { domainSelection, editorSelection, isSelectionOperation, isSelectionRequest } from "./selection-application";

export type { CharacterRepository } from "./canonical";
export { createCharacterDraft } from "./drafts";

interface TagPlan {
  created: { draftId: string; values: TagValues }[];
  updated: Tag[];
  deleted: string[];
  attachedIds: string[];
}
export interface CharacterCardsService {
  /** Advances genuine persisted jobs using this service's same repository owner. */
  runMemoryJobs(): Promise<{ owners: number; rebuildWindows: number; candidateBatches: number }>;
  /** Dispatches an editor or registered domain command through the supplied canonical port. */
  dispatch<R extends Request>(request: R): Promise<RequestOutput<R>>;
  /** Reads independent canonical catalogs without inserting empty substitute collections. */
  snapshot(): Promise<Snapshot>;
  /** Reads only role records, without unrelated model, speech or tool catalogs. */
  sidebarDirectory(): Promise<Pick<Snapshot, "cards" | "groups">>;
  /** Executes an exact typed domain method on the same service dependency. */
  dispatchDomain<K extends DomainOperation>(operation: K, input: DomainInput<K>): Promise<DomainOutput<K>>;
}

/** Binds every entry point to one real file owner and the same required independent configuration dependencies. */
export function createCharacterCardsService(owner: CharacterRepositoryOwner, configurations: IndependentConfigurationAccess = independentConfigurations): CharacterCardsService {
  const jobs = new MemoryJobRunner(owner), application = createSelectionApplication(configurations);
  /** Validates refs before any selection mutation and applies configs only after actual file/native acknowledgement. */
  function dispatchRequest<R extends Request>(request: R): Promise<RequestOutput<R>>;
  /** Executes the discriminated editor request while preserving its complete result union. */
  function dispatchRequest(request: Request): Promise<RequestOutput> {
    if (request.action === "listThemeChoices") return readThemeChoices(configurations);
      if (!isSelectionRequest(request)) return owner.run(
        /** Executes ordinary editor operations against this same serialized snapshot. */
        repository => dispatch(request, repository),
      );
      return application.commit(
        /** Publishes the actual selection once; independent preference writes are not included in this file publication. */
        validate => owner.run(
          /** Prevalidates the chosen complete record inside the same immutable snapshot used by its actual mutation. */
          async repository => {
            const references = await validate(await readSelectionReferences(repository, editorSelection(request)));
            return { references, value: await dispatch(request, repository) };
          },
        ),
      );
  }
  return {
    /** Runs the private job scheduler without constructing another store or business service. */
    runMemoryJobs: () => jobs.tick(),
    /** Preserves action-specific results through the shared service dispatcher. */
    dispatch: dispatchRequest,
    /** Reads one consistent domain snapshot through the same operation queue. */
    snapshot: () => owner.run(snapshot),
    /** Preserves the same repository queue and integrity checks as full snapshots. */
    sidebarDirectory: () => owner.run(async repository => {
      const [cards, groups] = await Promise.all([repository.listCharacters(), repository.listGroups()]);
      records(cards, assertCard, "角色卡列表"); records(groups, assertGroup, "角色组列表");
      return { cards, groups };
    }),
    /** Keeps every real selection alias and sidebar binding on the same awaited application sequence. */
    dispatchDomain<K extends DomainOperation>(operation: K, input: DomainInput<K>): Promise<DomainOutput<K>> {
      if (!isSelectionOperation(operation)) return owner.run(
        /** Runs non-selection APIs without touching independent current configurations. */
        repository => dispatchDomain(operation, input, repository),
      );
      return application.commit(
        /** Returns a confirmed owner write before applying any independent preferences. */
        validate => owner.run(
          /** Validates the actual domain request and all record references before writing the selected actor. */
          async repository => {
            const selection = await domainSelection(operation, input, repository);
            const references = await validate(await readSelectionReferences(repository, selection));
            return { references, value: await dispatchDomain(operation, input, repository) };
          },
        ),
      );
    },
  };
}

/** Requires real arrays and validates every complete canonical record. */
function records<T>(value: unknown, check: (record: unknown) => asserts record is T, label: string): T[] {
  if (!Array.isArray(value)) throw new Error(`${label} 返回格式不正确`);
  for (const record of value) check(record);
  return value as T[];
}

/** Validates the complete canonical shared memory store record. */
function assertStore(value: unknown): asserts value is Store {
  assertObject(value, "共享记忆库");
  requireId(value.id, "共享记忆库标识");
  assertString(value.name, "共享记忆库名称");
  assertInteger(value.createdAt, "记忆库创建时间", 0);
  assertInteger(value.updatedAt, "记忆库修改时间", 0);
}

/** Validates the exact externally tagged ActivePrompt enum without accepting an empty active value. */
function assertActive(value: unknown): asserts value is ActivePrompt {
  assertObject(value, "当前提示词");
  const keys = Object.keys(value);
  if (keys.length !== 1 || (keys[0] !== "CharacterCard" && keys[0] !== "CharacterGroup")) throw new Error("当前提示词类型无效");
  const selection = value[keys[0]];
  assertObject(selection, "当前提示词选择");
  requireId(selection.id, "当前提示词标识");
}

/** Builds the editor aggregate exclusively from independent real canonical reads. */
export async function snapshot(host: CharacterRepository): Promise<Snapshot> {
  const [cards, groups, stores, tags, active, models, ttsConfigs, toolCatalog] = await Promise.all([
    host.listCharacters(), host.listGroups(), host.listStores(), host.listTags(),
    host.readActive(), host.listModels(), host.listTtsConfigs(), host.readToolCatalog(),
  ]);
  records(cards, assertCard, "角色卡列表");
  records(groups, assertGroup, "角色组列表");
  records(stores, assertStore, "共享记忆库列表");
  records(tags, assertTag, "标签列表");
  assertActive(active);
  if (!Array.isArray(models) || !Array.isArray(ttsConfigs)) throw new Error("模型或 TTS 目录返回格式不正确");
  assertObject(toolCatalog, "工具来源目录");
  for (const field of ["builtinTools", "packages", "skills", "mcpServers"] as const) if (!Array.isArray(toolCatalog[field])) throw new Error(`工具来源目录缺少 ${field}`);
  return { cards, groups, stores, tags, active, models, ttsConfigs, toolCatalog };
}

/** Requires exactly one canonical record instead of silently choosing an ambiguous identity. */
function findRecord<T extends { id: string }>(values: T[], id: string, label: string): T {
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
function prepareTagChanges(attachedIds: string[], changes: TagChanges, tags: Tag[]): TagPlan {
  assertStrings(attachedIds, "角色标签");
  assertObject(changes, "标签修改");
  if (!Array.isArray(changes.created) || !Array.isArray(changes.updated)) throw new Error("标签修改格式无效");
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
    for (const tag of tags) if (tag.id === draftId) throw new Error("标签草稿标识与已有标签冲突");
    assertTagValues(draft.values);
    const values = normalizedTagValues(draft.values);
    draftIds.add(draftId);
    created.push({ draftId, values });
  }
  for (const edit of changes.updated) {
    assertTagValues(edit);
    const id = requireId(edit.id, "修改标签标识");
    if (changedIds.has(id) || deleted.has(id)) throw new Error("同一标签存在冲突修改");
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
  for (const draft of created) projected.push({ id: draft.draftId, name: draft.values.name });
  for (const draft of created) for (const tag of projected) if (tag.id !== draft.draftId && tag.name === draft.values.name) throw new Error(`标签名称已存在：${draft.values.name}`);
  for (const update of updated) {
    const original = findRecord(tags, update.id, "标签");
    if (original.name.trim() === update.name) continue;
    for (const tag of projected) if (tag.id !== update.id && tag.name === update.name) throw new Error(`标签名称已存在：${update.name}`);
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
  return { name: requireName(values.name, "标签名称"), description: values.description.trim(), promptContent: values.promptContent, tagType: values.tagType };
}

/** Commits a validated staged plan only as part of the enclosing explicit save. */
async function commitTagChanges(plan: TagPlan, host: CharacterRepository): Promise<string[]> {
  const resolved = new Map<string, string>();
  for (const draft of plan.created) {
    const created = await host.createTag(draft.values);
    assertTag(created);
    resolved.set(draft.draftId, created.id);
  }
  for (const tag of plan.updated) {
    const saved = await host.updateTag(tag);
    assertTag(saved);
  }
  for (const id of plan.deleted) await host.deleteTag(id);
  const ids: string[] = [];
  for (const id of plan.attachedIds) {
    if (resolved.has(id)) {
      const canonicalId = resolved.get(id);
      if (canonicalId === undefined) throw new Error("新标签没有返回 canonical 标识");
      ids.push(canonicalId);
    } else ids.push(id);
  }
  return normalizeNames(ids);
}

/** Normalizes every editable character field while retaining complete binding and policy records. */
function normalizedCharacter(card: Card): Card {
  if (card.chatModelBindingMode !== "FOLLOW_GLOBAL" && card.chatModelBindingMode !== "FIXED_MODEL") throw new Error("聊天模型绑定模式无效");
  if (card.memoryBindingMode !== "CHARACTER" && card.memoryBindingMode !== "SHARED") throw new Error("记忆绑定模式无效");
  if (card.chatModelBindingMode === "FIXED_MODEL") requireId(card.chatModelId, "固定聊天模型");
  if (card.memoryBindingMode === "SHARED") requireId(card.sharedMemoryId, "共享记忆库");
  if (card.ttsConfigId !== null) requireId(card.ttsConfigId, "TTS 配置");
  if (card.themeConfigId !== null) requireId(card.themeConfigId, "主题配置");
  const policy = card.toolAccessConfig;
  if (policy.enabled && (policy.allowedPackages.length !== 0 || policy.allowedSkills.length !== 0 || policy.allowedMcpServers.length !== 0) && !new Set(normalizeNames(policy.allowedBuiltinTools)).has("use_package")) throw new Error("选择外部工具来源时必须允许 use_package");
  const mounts = [];
  const mountIds = new Set<string>();
  for (const mount of card.sharedMemoryMounts) {
    if (mountIds.has(mount.sharedMemoryId)) throw new Error("共享记忆挂载重复");
    mountIds.add(mount.sharedMemoryId);
    mounts.push({ sharedMemoryId: mount.sharedMemoryId, readable: mount.readable, writable: mount.writable });
  }
  return {
    ...card, name: requireName(card.name, "角色卡名称"), description: card.description.trim(),
    attachedTagIds: [...card.attachedTagIds], sharedMemoryMounts: mounts,
    toolAccessConfig: {
      enabled: card.toolAccessConfig.enabled,
      allowedBuiltinTools: normalizeNames(card.toolAccessConfig.allowedBuiltinTools),
      allowedPackages: normalizeNames(card.toolAccessConfig.allowedPackages),
      allowedSkills: normalizeNames(card.toolAccessConfig.allowedSkills),
      allowedMcpServers: normalizeNames(card.toolAccessConfig.allowedMcpServers),
    },
  };
}

/** Saves one complete character and its staged tag edits in the enclosing plugin snapshot operation. */
export async function saveCharacter(card: Card, create: boolean, tagChanges: TagChanges, host: CharacterRepository): Promise<Card> {
  assertCard(card);
  assertBoolean(create, "角色新建标识");
  let value = normalizedCharacter(card);
  if (create) {
    if (value.isDefault) throw new Error("不能新建默认角色卡");
    value = { ...value, id: "", createdAt: 0, updatedAt: 0 };
  } else {
    const id = requireId(value.id, "角色卡标识");
    const original = await host.getCharacter(id);
    assertCard(original);
    value = { ...value, id: original.id, isDefault: original.isDefault, createdAt: original.createdAt };
  }
  const [tags, stores, cards] = await Promise.all([host.listTags(), host.listStores(), host.listCharacters()]);
  records(cards, assertCard, "角色卡列表");
  let retainedName = false;
  if (!create) for (const existing of cards) if (existing.id === value.id && existing.name.trim() === value.name) retainedName = true;
  if (!retainedName) for (const existing of cards) if (existing.id !== value.id && existing.name.trim() === value.name) throw new Error(`角色卡名称已存在：${value.name}`);
  if (value.ttsConfigId !== null) {
    const configs = await host.listTtsConfigs();
    findRecord(configs, value.ttsConfigId, "TTS 配置");
  }
  records(tags, assertTag, "标签列表");
  records(stores, assertStore, "共享记忆库列表");
  const plan = prepareTagChanges(value.attachedTagIds, tagChanges, tags);
  if (value.memoryBindingMode === "SHARED") findRecord(stores, requireId(value.sharedMemoryId, "共享记忆库标识"), "共享记忆库");
  for (const mount of value.sharedMemoryMounts) findRecord(stores, mount.sharedMemoryId, "共享记忆库");
  value = { ...value, attachedTagIds: await commitTagChanges(plan, host) };
  const saved = create ? await host.createCharacter(value) : await host.updateCharacter(value);
  assertCard(saved);
  requireId(saved.id, "保存的角色卡标识");
  return saved;
}

/** Validates and saves every editable group field while retaining canonical creation metadata. */
export async function saveGroup(group: GroupValues, create: boolean, host: CharacterRepository): Promise<Group> {
  assertGroupValues(group);
  assertBoolean(create, "群组新建标识");
  const name = requireName(group.name, "角色组名称");
  const cards = records(await host.listCharacters(), assertCard, "角色卡列表");
  const members = [];
  for (const member of group.members) {
    findRecord(cards, member.characterCardId, "群组角色");
    members.push({ characterCardId: member.characterCardId, orderIndex: member.orderIndex });
  }
  let value: Group;
  if (create) value = { id: "", name, description: group.description.trim(), members, themeConfigId: group.themeConfigId, createdAt: 0, updatedAt: 0 };
  else {
    const original = await host.getGroup(requireId(group.id, "群组标识"));
    assertGroup(original);
    value = { ...original, name, description: group.description.trim(), members, themeConfigId: group.themeConfigId };
  }
  const saved = create ? await host.createGroup(value) : await host.updateGroup(value);
  assertGroup(saved);
  return saved;
}

/** Writes every editable tag field in one typed host operation. */
export async function saveTag(tag: TagEdit, create: boolean, host: CharacterRepository): Promise<Tag> {
  assertTagValues(tag);
  assertBoolean(create, "标签新建标识");
  const values = normalizedTagValues(tag);
  let saved: Tag;
  if (create) saved = await host.createTag(values);
  else {
    const tags = records(await host.listTags(), assertTag, "标签列表");
    const original = findRecord(tags, requireId(tag.id, "标签标识"), "标签");
    saved = await host.updateTag({ ...original, ...values });
  }
  assertTag(saved);
  return saved;
}

/** Creates or renames an authoritative shared store without copying its metadata into plugin config. */
export async function saveStore(store: StoreValues, create: boolean, host: CharacterRepository): Promise<Store> {
  assertObject(store, "共享记忆库");
  assertBoolean(create, "记忆库新建标识");
  const name = requireName(store.name, "共享记忆库名称");
  let saved: Store;
  if (create) saved = await host.createStore(name);
  else {
    const id = requireId(store.id, "共享记忆库标识");
    findRecord(records(await host.listStores(), assertStore, "共享记忆库列表"), id, "共享记忆库");
    saved = await host.renameStore(id, name);
  }
  assertStore(saved);
  return saved;
}

/** Requires a real card or group before changing the canonical active prompt. */
export async function activate(type: "card" | "group", id: string, host: CharacterRepository): Promise<void> {
  requireId(id, "当前提示词标识");
  if (type === "card") {
    assertCard(await host.getCharacter(id));
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
  if (parts.length !== 2 || !/^[^:\s]+$/.test(parts[1])) throw new Error("记忆库标识格式无效");
  if (parts[0] === "character") return { kind: "CHARACTER", id: parts[1] };
  if (parts[0] === "shared") return { kind: "SHARED", id: parts[1] };
  throw new Error("记忆库命名空间无效");
}

/** Requires the selected owner to exist in the canonical character or shared store registry. */
export async function requireOwner(ownerKey: string, host: CharacterRepository): Promise<MemoryOwner> {
  const owner = parseOwner(ownerKey);
  if (owner.kind === "CHARACTER") assertCard(await host.getCharacter(owner.id));
  else findRecord(records(await host.listStores(), assertStore, "共享记忆库列表"), owner.id, "共享记忆库");
  return owner;
}

/** Resolves the primary memory namespace from the character's complete persisted binding. */
export async function resolveMemoryOwner(characterId: string, host: CharacterRepository): Promise<string> {
  const card = await host.getCharacter(requireId(characterId, "角色卡标识"));
  assertCard(card);
  let ownerKey: string;
  if (card.memoryBindingMode === "CHARACTER") ownerKey = `character:${card.id}`;
  else if (card.memoryBindingMode === "SHARED") ownerKey = `shared:${requireId(card.sharedMemoryId, "共享记忆库绑定")}`;
  else throw new Error("记忆绑定模式无效");
  await requireOwner(ownerKey, host);
  return ownerKey;
}

/** Reads full canonical memories for one validated owner. */
export async function listMemories(ownerKey: string, host: CharacterRepository): Promise<Memory[]> {
  await requireOwner(ownerKey, host);
  return records(await host.listMemories(ownerKey), assertMemory, "记忆列表");
}

/** Reads the actual UUID graph and complete records without projecting lossy tool query results. */
export async function graph(ownerKey: string, host: CharacterRepository): Promise<MemoryGraphResult> {
  await requireOwner(ownerKey, host);
  const [value, items] = await Promise.all([host.readMemoryGraph(ownerKey), host.listMemories(ownerKey)]);
  assertGraph(value);
  records(items, assertMemory, "记忆列表");
  return { graph: value, items };
}

/** Resolves a legacy editor title exactly and rejects ambiguous records instead of picking one. */
function memoryByTitle(items: Memory[], title: string): Memory {
  const name = requireName(title, "记忆标题");
  let found: Memory | undefined;
  for (const item of items) {
    if (item.title !== name) continue;
    if (found !== undefined) throw new Error(`记忆标题不唯一，请使用数据库标识编辑：${name}`);
    found = item;
  }
  if (found === undefined) throw new Error(`记忆不存在：${name}`);
  return found;
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

/** Persists complete editable memory values while retaining repository-owned identity and document data. */
export async function saveMemory(ownerKey: string, values: MemoryValues, id: string | null, host: CharacterRepository): Promise<Memory> {
  assertMemoryValues(values);
  await requireOwner(ownerKey, host);
  const normalized = { ...values, title: requireName(values.title, "记忆标题"), contentType: values.contentType.trim(), source: values.source.trim(), folderPath: normalizeFolderPath(values.folderPath), tags: normalizeNames(values.tags) };
  let saved: Memory;
  if (id === null) saved = await host.createMemory(ownerKey, normalized);
  else {
    requireDecimal(id, "记忆记录标识", true);
    saved = await host.updateMemory(ownerKey, id, normalized);
  }
  assertMemory(saved);
  return saved;
}

/** Performs full-record canonical search with explicit owner scope and explicit filters. */
export async function searchMemories(options: MemorySearchOptions, host: CharacterRepository): Promise<Memory[]> {
  assertSearchOptions(options);
  await requireOwner(options.ownerKey, host);
  return records(await host.searchMemories({ ...options, folderPath: normalizeFolderPath(options.folderPath) }), assertMemory, "记忆搜索");
}

/** Combines setting, mode-specific content, staged canonical tags, and advanced prompt in source order. */
export async function combinePrompts(characterId: string, additionalTagIds: string[], promptFunctionType: "CHAT" | "VOICE", host: CharacterRepository): Promise<string> {
  const card = await host.getCharacter(requireId(characterId, "角色卡标识"));
  assertCard(card);
  assertStrings(additionalTagIds, "附加标签");
  if (promptFunctionType !== "CHAT" && promptFunctionType !== "VOICE") throw new Error("提示词功能类型无效");
  const tags = records(await host.listTags(), assertTag, "标签列表");
  const parts: string[] = [];
  const setting = card.characterSetting.trim();
  if (setting !== "") parts.push(setting);
  const otherContent = (promptFunctionType === "CHAT" ? card.otherContentChat : card.otherContentVoice).trim();
  if (otherContent !== "") parts.push(otherContent);
  const ids = normalizeNames([...card.attachedTagIds, ...additionalTagIds]);
  for (const id of ids) {
    const content = findRecord(tags, id, "提示词标签").promptContent.trim();
    if (content !== "") parts.push(content);
  }
  const advanced = card.advancedCustomPrompt.trim();
  if (advanced !== "") parts.push(advanced);
  return parts.join("\n\n").trim();
}

/** Saves full settings through the owner service rather than writing preference files. */
export async function writeMemorySettings(ownerKey: string, settings: MemorySettings, host: CharacterRepository): Promise<MemorySettings> {
  assertMemorySettings(settings);
  await requireOwner(ownerKey, host);
  await host.writeMemorySettings(ownerKey, { ...settings });
  const saved = await host.readMemorySettings(ownerKey);
  assertMemorySettings(saved);
  return saved;
}

/** Saves all search configuration fields through the canonical owner service. */
export async function writeMemorySearchConfig(ownerKey: string, config: MemorySearchConfig, host: CharacterRepository): Promise<MemorySearchConfig> {
  assertSearchConfig(config);
  await requireOwner(ownerKey, host);
  await host.writeMemorySearchConfig(ownerKey, { ...config });
  const saved = await host.readMemorySearchConfig(ownerKey);
  assertSearchConfig(saved);
  return saved;
}

/** Dispatches plugin-owned domain operations against the explicitly supplied plugin-local repository. */
export function dispatch<R extends Request>(request: R, host: CharacterRepository): Promise<RequestOutput<R>>;

/** Executes the complete discriminated editor protocol against one canonical repository. */
export async function dispatch(request: Request, host: CharacterRepository): Promise<RequestOutput> {
  assertObject(request, "角色卡请求");
  switch (request.action) {
    case "snapshot": return snapshot(host);
    case "listThemeChoices": return readThemeChoices(independentConfigurations);
    case "listCharacters": return records(await host.listCharacters(), assertCard, "角色卡列表");
    case "getCharacter": { const card = await host.getCharacter(requireId(request.id, "角色卡标识")); assertCard(card); return card; }
    case "saveCharacter": await saveCharacter(request.card, request.create, request.tagChanges, host); return snapshot(host);
    case "deleteCharacter": {
      const id = requireId(request.id, "角色卡标识");
      const card = await host.getCharacter(id); assertCard(card);
      if (card.isDefault) throw new Error("默认角色卡不能删除");
      await host.deleteCharacter(id); return snapshot(host);
    }
    case "activate": await activate(request.type, request.id, host); return snapshot(host);
    case "readActive": { const active = await host.readActive(); assertActive(active); return active; }
    case "readChatBinding": return executeDomain("chat.configuration.binding.read", { chatId: request.chatId }, host);
    case "writeChatBinding": return executeDomain("chat.configuration.binding.write", { chatId: request.chatId, selection: request.selection }, host);
    case "deleteChatBinding": return executeDomain("chat.configuration.binding.delete", { chatId: request.chatId }, host);
    case "listGroups": return records(await host.listGroups(), assertGroup, "角色组列表");
    case "getGroup": { const group = await host.getGroup(requireId(request.id, "群组标识")); assertGroup(group); return group; }
    case "saveGroup": await saveGroup(request.group, request.create, host); return snapshot(host);
    case "deleteGroup": await host.deleteGroup(requireId(request.id, "群组标识")); return snapshot(host);
    case "listTags": return records(await host.listTags(), assertTag, "标签列表");
    case "saveTag": return saveTag(request.tag, request.create, host);
    case "deleteTag": await host.deleteTag(requireId(request.id, "标签标识")); return snapshot(host);
    case "listStores": return records(await host.listStores(), assertStore, "共享记忆库列表");
    case "saveStore": await saveStore(request.store, request.create, host); return snapshot(host);
    case "deleteStore": await host.deleteStore(requireId(request.id, "共享记忆库标识")); return snapshot(host);
    case "listModels": return host.listModels();
    case "listTtsConfigs": return host.listTtsConfigs();
    case "readToolCatalog": return host.readToolCatalog();
    case "resolveMemoryOwner": return resolveMemoryOwner(request.characterId, host);
    case "readUser": {
      await requireOwner(request.ownerKey, host);
      const user = await host.readUser(request.ownerKey);
      assertObject(user, "用户资料"); assertString(user.content, "用户资料内容");
      if (user.ownerKey !== request.ownerKey) throw new Error("用户资料 owner 与请求不一致");
      return user;
    }
    case "writeUser": await requireOwner(request.ownerKey, host); assertString(request.content, "用户资料内容"); await host.writeUser(request.ownerKey, request.content); return { saved: true };
    case "graph": return graph(request.ownerKey, host);
    case "listMemories": return listMemories(request.ownerKey, host);
    case "searchMemory": return searchMemories({ ownerKey: request.ownerKey, query: request.query, folderPath: null, relevanceThreshold: 0, createdAtStartMs: null, createdAtEndMs: null }, host);
    case "searchMemories": return searchMemories(request.options, host);
    case "saveMemory": {
      assertString(request.tags, "记忆标签"); assertString(request.folderPath, "记忆文件夹");
      const values: MemoryValues = { title: request.title, content: request.content, contentType: request.contentType, source: request.source, credibility: request.credibility, importance: request.importance, folderPath: request.folderPath, tags: normalizeNames(request.tags.split(",")) };
      const id = request.originalTitle === null ? null : memoryByTitle(await listMemories(request.ownerKey, host), request.originalTitle).id;
      await saveMemory(request.ownerKey, values, id, host); return graph(request.ownerKey, host);
    }
    case "deleteMemory": {
      const memory = memoryByTitle(await listMemories(request.ownerKey, host), request.title);
      await host.deleteMemory(request.ownerKey, memory.id); return graph(request.ownerKey, host);
    }
    case "createLink": {
      assertNumber(request.weight, "关系强度", 0, 1); assertString(request.description, "关系描述");
      const type_ = requireName(request.linkType, "关系类型");
      const items = await listMemories(request.ownerKey, host);
      const source = memoryByTitle(items, request.sourceTitle), target = memoryByTitle(items, request.targetTitle);
      if (source.id === target.id) throw new Error("不能创建记忆自身关系");
      await host.createLink(request.ownerKey, { sourceMemoryId: source.id, targetMemoryId: target.id, type_, weight: request.weight, description: request.description });
      return graph(request.ownerKey, host);
    }
    case "deleteLink": await requireOwner(request.ownerKey, host); requireDecimal(request.linkId, "记忆关系标识", true); await host.deleteLink(request.ownerKey, request.linkId); return graph(request.ownerKey, host);
    case "updateLink": {
      await requireOwner(request.ownerKey, host); requireDecimal(request.linkId, "记忆关系标识", true);
      assertNumber(request.weight, "关系强度", 0, 1); assertString(request.description, "关系描述");
      await host.updateLink(request.ownerKey, request.linkId, { type_: requireName(request.linkType, "关系类型"), weight: request.weight, description: request.description });
      return graph(request.ownerKey, host);
    }
    case "readMemorySettings": { await requireOwner(request.ownerKey, host); const settings = await host.readMemorySettings(request.ownerKey); assertMemorySettings(settings); return settings; }
    case "writeMemorySettings": return writeMemorySettings(request.ownerKey, request.settings, host);
    case "readMemorySearchConfig": { await requireOwner(request.ownerKey, host); const config = await host.readMemorySearchConfig(request.ownerKey); assertSearchConfig(config); return config; }
    case "writeMemorySearchConfig": return writeMemorySearchConfig(request.ownerKey, request.config, host);
    case "combinePrompts": return combinePrompts(request.characterId, request.additionalTagIds, request.promptFunctionType, host);
    case "importCharacter": {
      const tags = records(await host.listTags(), assertTag, "标签列表");
      const plan = decodeCharacterImport(request.content, request.format, tags);
      await saveCharacter(plan.card, true, plan.tagChanges, host); return snapshot(host);
    }
    case "exportCharacter": {
      const card = await host.getCharacter(requireId(request.id, "角色卡标识")); assertCard(card);
      const tags = records(await host.listTags(), assertTag, "标签列表");
      return encodeCharacterExport(card, request.format, tags);
    }
    case "importGroup": await saveGroup(decodeGroupImport(request.content), true, host); return snapshot(host);
    case "exportGroup": { const group = await host.getGroup(requireId(request.id, "群组标识")); assertGroup(group); return JSON.stringify(group, null, 2); }
    case "listMemoryFolders": await requireOwner(request.ownerKey, host); return host.listMemoryFolders(request.ownerKey);
    case "readMemoryAutoSaveStatus": await requireOwner(request.ownerKey, host); return host.readMemoryAutoSaveStatus(request.ownerKey);
    case "listMemoryChats": return executeDomain("memory.chat.list", { ownerKey: request.ownerKey }, host);
    case "updateChatMemory": return executeDomain("memory.chat.update", { ownerKey: request.ownerKey, chatId: request.chatId }, host);
    case "autoCategorizeMemory": return executeDomain("memory.categorize", { ownerKey: request.ownerKey }, host);
    case "startMemoryRebuild": return executeDomain("memory.rebuild.start", { ownerKey: request.ownerKey, rebuild: request.rebuild }, host);
    case "readMemoryRebuildProgress": return executeDomain("memory.rebuild.progress", { ownerKey: request.ownerKey }, host);
    case "cancelMemoryRebuild": return executeDomain("memory.rebuild.cancel", { ownerKey: request.ownerKey }, host);
    case "rebuildMemoryEmbeddings": return executeDomain("memory.embeddings.rebuild", { ownerKey: request.ownerKey }, host);
    case "exportMemory": await requireOwner(request.ownerKey, host); return { ownerKey: request.ownerKey, content: await exportMemoryBackup(request.ownerKey, host) };
    case "importMemory": {
      await requireOwner(request.ownerKey, host);
      if (request.strategy !== "SKIP" && request.strategy !== "UPDATE" && request.strategy !== "CREATE_NEW") throw new Error("记忆导入策略无效");
      const document = decodeMemoryBackup(request.content);
      const result = await host.importMemorySpace(request.ownerKey, document.space, request.strategy);
      await host.writeUser(request.ownerKey, document.userMarkdown);
      return { ownerKey: request.ownerKey, result };
    }
    default: { const action: never = request; throw new Error(`未知角色卡请求：${JSON.stringify(action)}`); }
  }
}



/** Executes typed plugin domain commands through the same direct canonical service as the editor. */
export function dispatchDomain<K extends DomainOperation>(operation: K, input: DomainInput<K>, host: CharacterRepository): Promise<DomainOutput<K>> {
  return executeDomain(operation, input, host);
}

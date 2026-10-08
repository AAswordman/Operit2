import type { DomainInput, DomainOperation, DomainOutput } from "./api";
import type { CharacterRepository } from "./canonical";
import type { Card, Memory, MemoryLink } from "./model";
import { createCharacterDraft, createGroupDraft, createTagDraft } from "./drafts";
import { parseDomainPayload, resolveChatConfiguration } from "./domain";
import { autoCategorizeMemory, updateChatMemory } from "./memory-jobs/analysis";
import { listMemoryChats, requireChat } from "./memory-jobs/chat";
import { rebuildMemoryEmbeddings } from "./memory-jobs/embeddings";
import { enqueueCandidate, startMemoryRebuild } from "./memory-jobs/scheduler";
import { decodeCharacterBackup, decodeGroupBackup, decodeMemoryBackup, exportCharacterBackup, exportGroupBackup, exportMemoryBackup } from "./backup";
import { decodeCharacterImport, decodeGroupImport, encodeCharacterExport } from "./serialization";
import { assertCard, assertGroup, assertMemory, assertNumber, assertStrings, assertTag, normalizeNames, requireId, requireName } from "./validation";
import { activate, combinePrompts, graph, listMemories, parseOwner, requireOwner, resolveMemoryOwner, saveCharacter, saveGroup, saveStore, saveTag, searchMemories, snapshot, writeMemorySearchConfig, writeMemorySettings } from "./service";

type DomainHandlers = { [K in DomainOperation]: (input: DomainInput<K>, host: CharacterRepository) => Promise<DomainOutput<K>> };

/** Requires one existing canonical memory for title-addressed public editor operations. */
function findMemory(items: Memory[], title: string): Memory {
  const name = requireName(title, "记忆标题");
  let found: Memory | undefined;
  for (const memory of items) {
    if (memory.title !== name) continue;
    if (found !== undefined) throw new Error(`记忆标题不唯一：${name}`);
    found = memory;
  }
  if (found === undefined) throw new Error(`记忆不存在：${name}`);
  return found;
}

/** Converts canonical deletion lifecycle completion to the public activation-independent result. */
async function deleteCharacter(id: string, host: CharacterRepository): Promise<{ id: string; deleted: boolean }> {
  requireId(id, "角色卡标识"); const card = await host.getCharacter(id); assertCard(card);
  if (card.isDefault) throw new Error("默认角色卡不能删除");
  await host.deleteCharacter(id); return { id, deleted: true };
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

/** Implements every published domain operation with typed handlers sharing the editor's actual business service. */
const handlers: DomainHandlers = {
  /** Reads exact opaque-scope manual metadata from the same repository as the editor. */
  "conversation-group.list": (input, repository) => repository.listConversationGroups(input.ownerSelection),
  /** Creates a real empty manual group independently from role composition. */
  "conversation-group.create": (input, repository) => repository.createConversationGroup(input),
  /** Writes only explicit name or pin metadata without moving a chat. */
  "conversation-group.update": (input, repository) => repository.updateConversationGroup(input.id, input.changes),
  /** Releases members without invoking any host chat deletion. */
  "conversation-group.delete": (input, repository) => repository.deleteConversationGroup(input.id),
  /** Verifies genuine host chat existence before committing one explicit membership transfer. */
  "conversation-group.moveChat": async (input, repository) => { await requireChat(input.chatId); return repository.moveConversationGroupChat(input.chatId, input.groupId, input.ownerSelection); },
  /** Commits a complete scoped order without mutating another section. */
  "conversation-group.reorder": (input, repository) => repository.reorderConversationGroups(input.ownerSelection, input.ids),
  /** Keeps full-filter searches and provider embedding persistence on the same serialized service. */
  "memory.searchWithOptions": async (input, repository) => ({ ownerKey: input.ownerKey, items: await searchMemories(input, repository) }),
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
  "memory.rebuild.cancel": async (input, repository) => { await repository.cancelMemoryRebuild(input.ownerKey); return repository.readMemoryRebuildProgress(input.ownerKey); },
  /** Rebuilds actual configured provider embeddings using generic HTTP. */
  "memory.embeddings.rebuild": (input, repository) => rebuildMemoryEmbeddings(input.ownerKey, repository),
  /** Enqueues only actual persisted message sources and genuine bound owners. */
  "memory.candidate.enqueue": (input, repository) => enqueueCandidate(input, repository),
  /** Resolves binding, catalogs, prompt and USER.md from one serialized repository operation. */
  "chat.configuration.resolve": (input, repository) => resolveChatConfiguration(input, {
    /** Reads the same in-flight directory without reacquiring the publisher. */
    snapshot: () => snapshot(repository),
    /** Executes nested resolution reads on this exact snapshot without another service. */
    dispatchDomain: <K extends DomainOperation>(operation: K, payload: DomainInput<K>) => executeDomain(operation, payload, repository),
  }),
  /** Requires the chat's persisted selection rather than global active state. */
  "chat.configuration.binding.read": (input, repository) => repository.readChatBinding(input.chatId),
  /** Stages a genuine selector submission in the shared publication. */
  "chat.configuration.binding.write": async (input, repository) => { await requireChat(input.chatId); return repository.writeChatBinding(input); },
  /** Deletes the requested existing binding through the same file operation. */
  "chat.configuration.binding.delete": (input, repository) => repository.deleteChatBinding(input.chatId),
  /** Reads the complete real editor directories. */
  snapshot: (_input, host) => snapshot(host),
  /** Reads complete canonical character cards. */
  "character.list": async (_input, host) => { const cards = await host.listCharacters(); for (const card of cards) assertCard(card); return cards; },
  /** Requires the selected canonical character. */
  "character.get": async (input, host) => { const card = await host.getCharacter(input.id); assertCard(card); return card; },
  /** Initializes an explicit new form and persists all provided character fields. */
  "character.create": (input, host) => saveCharacter({ ...createCharacterDraft(), ...input.values }, true, { created: [], updated: [], deleted: [] }, host),
  /** Merges an explicit edit into the complete canonical record before one manager write. */
  "character.update": async (input, host) => { const original = await host.getCharacter(input.id); return saveCharacter({ ...original, ...input.changes, id: original.id }, false, { created: [], updated: [], deleted: [] }, host); },
  /** Deletes an existing non-default character using the canonical lifecycle. */
  "character.delete": (input, host) => deleteCharacter(input.id, host),
  /** Activates the selected canonical character. */
  "character.setActive": async (input, host) => { await activate("card", input.id, host); return { type: "character_card", id: input.id, active: true }; },
  /** Composes the actual canonical prompt using the explicitly selected function and tags. */
  "character.combine": async (input, host) => {
    const options = { promptFunctionType: "CHAT" as const, additionalTagIds: [] as string[], ...input };
    const prompt = await combinePrompts(input.id, options.additionalTagIds, options.promptFunctionType, host);
    return { id: input.id, promptFunctionType: options.promptFunctionType, additionalTagIds: options.additionalTagIds, prompt };
  },
  /** Resets the canonical default character through its established manager operation. */
  "character.resetDefault": async (_input, host) => { const card = await host.resetDefaultCharacter(); assertCard(card); return { defaultCharacterReset: true }; },
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
  /** Activates the selected canonical character. */
  "activePrompt.setCard": async (input, host) => { await activate("card", input.id, host); return { type: "character_card", id: input.id, active: true }; },
  /** Activates the selected canonical group. */
  "activePrompt.setGroup": async (input, host) => { await activate("group", input.id, host); return { type: "character_group", id: input.id, active: true }; },
  /** Applies the plugin's explicit chat-binding selection logic. */
  "activePrompt.activateForChat": (input, host) => activateForChat(input, host),
  /** Resolves the real next-send actor under the canonical active card or group selection. */
  "activePrompt.resolvedCard": async (_input, host) => { const active = await host.readActive(); if (!("CharacterCard" in active)) throw new Error("An active group requires an explicit execution participant"); return { id: active.CharacterCard.id }; },
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
  "memory.get": async (input, host) => ({ ownerKey: input.ownerKey, item: findMemory(await listMemories(input.ownerKey, host), input.title) }),
  /** Initializes an intentional full new record and writes all memory metadata in the plugin snapshot. */
  "memory.create": async (input, host) => {
    await requireOwner(input.ownerKey, host);
    const values = { id: "", uuid: "", contentType: "text", source: "", credibility: 0.5, importance: 0.5,
      documentPath: null, isDocumentNode: false, chunkIndexFilePath: null, folderPath: null, createdAt: 0, updatedAt: 0,
      lastAccessedAt: 0, tags: [] as string[], properties: [], ...input.values };
    assertStrings(values.tags, "记忆标签");
    const tags = []; for (const name of normalizeNames(values.tags)) tags.push({ id: String(tags.length + 1), name });
    const memory = { ...values, id: "", uuid: "", title: requireName(values.title, "记忆标题"), tags };
    const item = await host.saveMemory(input.ownerKey, memory); assertMemory(item);
    return { ownerKey: input.ownerKey, item, created: true };
  },
  /** Merges every supplied memory field into its full canonical record while preserving untouched metadata. */
  "memory.update": async (input, host) => {
    const original = findMemory(await listMemories(input.ownerKey, host), input.originalTitle);
    if (input.expectedId !== undefined && input.expectedId !== original.id) throw new Error("Memory identity changed since editing began");
    const changes = { ...input.changes };
    const tags = [];
    if (Object.prototype.hasOwnProperty.call(changes, "tags")) { assertStrings(changes.tags, "记忆标签"); for (const name of normalizeNames(changes.tags)) tags.push({ id: String(tags.length + 1), name }); }
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
    const items = await listMemories(input.ownerKey, host), existing = new Set<string>(); for (const item of items) existing.add(item.id);
    const selected = new Set<string>(); for (const id of input.ids) { if (!existing.has(id)) throw new Error(`记忆不存在：${id}`); selected.add(id); }
    await host.moveMemories(input.ownerKey, [...selected], input.folderPath);
    return { ownerKey: input.ownerKey, ids: [...selected], folder: input.folderPath, moved: selected.size };
  },
  /** Resolves exact source and target records before creating a canonical relationship. */
  "memory.link.create": async (input, host) => {
    const items = await listMemories(input.ownerKey, host), source = findMemory(items, input.sourceTitle), target = findMemory(items, input.targetTitle);
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

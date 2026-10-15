import type { CharacterRepository } from "../canonical";
import type { Memory, MemoryChatMessage, MemoryExtractionResult, MemorySettings } from "../model";
import { assertNumber, assertObject, assertString, assertStrings, normalizeNames, requireName } from "../validation";
import { extractionTemplate } from "./prompt-template";
import { readMessages, requireChat, requireMessageOwner } from "./chat";

interface Entity { title: string; content: string; tags: string[]; folder_path: string; alias_for?: string | null }
interface Update { title: string; content: string; reason: string; credibility?: number | null; importance?: number | null }
interface Merge { source_titles: string[]; title: string; content: string; tags: string[]; folder_path: string; reason: string }
interface Link { source: string; target: string; type: string; description: string; weight: number }
interface Analysis { main: Entity | null; new: Entity[]; update: Update[]; merge: Merge[]; links: Link[]; profile_markdown?: string | null }

/** Parses only strict JSON or its explicitly documented complete Markdown fence. */
export function parseModelJson(text: string): unknown {
  assertString(text, "model output"); const trimmed = text.trim();
  const fence = /^```(?:json)?\s*\r?\n([\s\S]*?)\r?\n```\s*$/u.exec(trimmed);
  return JSON.parse(fence === null ? trimmed : fence[1]);
}

/** Checks each required named field before any domain record is modified. */
function entity(value: unknown, alias: boolean): asserts value is Entity {
  assertObject(value, "analysis entity"); requireName(value.title, "entity title"); assertString(value.content, "entity content"); assertStrings(value.tags, "entity tags"); assertString(value.folder_path, "entity folder");
  if (alias && value.alias_for !== undefined && value.alias_for !== null) requireName(value.alias_for, "alias target");
}

/** Validates a new entity including its explicitly optional exact alias target. */
function newEntity(value: unknown): asserts value is Entity { entity(value, true); }

/** Validates all complete fields for one named memory update. */
function update(value: unknown): asserts value is Update {
  assertObject(value, "memory update"); requireName(value.title, "update title"); assertString(value.content, "update content"); assertString(value.reason, "update reason");
  for (const key of ["credibility", "importance"] as const) if (value[key] !== undefined && value[key] !== null) assertNumber(value[key], key, 0, 1);
}

/** Validates an explicit merge including every required source title. */
function merge(value: unknown): asserts value is Merge {
  assertObject(value, "memory merge"); assertStrings(value.source_titles, "merge sources"); if (value.source_titles.length === 0) throw new Error("Memory merge requires source records");
  entity(value, false); assertString(value.reason, "merge reason");
}

/** Validates a real named relationship before resolving either endpoint. */
function link(value: unknown): asserts value is Link {
  assertObject(value, "memory relationship"); for (const key of ["source", "target", "type"] as const) requireName(value[key], key);
  assertString(value.description, "relationship description"); assertNumber(value.weight, "relationship weight", 0, 1);
}

/** Produces a typed object array only after validating every supplied full record. */
function objectArray<T>(value: unknown, label: string, check: (item: unknown) => asserts item is T): T[] {
  if (!Array.isArray(value)) throw new Error("Memory analysis requires a named object array: " + label);
  const result: T[] = []; for (const item of value) { check(item); result.push(item); } return result;
}

/** Validates the transferred protocol; explicit {} is the product's no-reusable-facts result. */
function analysis(value: unknown): Analysis | null {
  assertObject(value, "memory analysis"); if (Object.keys(value).length === 0) return null;
  let main: Entity | null = null;
  if (value.main !== null) { entity(value.main, false); main = value.main; }
  const result: Analysis = { main, new: objectArray(value.new, "new", newEntity), update: objectArray(value.update, "update", update),
    merge: objectArray(value.merge, "merge", merge), links: objectArray(value.links, "links", link) };
  if (result.new.length > 5) throw new Error("Memory extraction exceeds the five-entity gate");
  if (Object.prototype.hasOwnProperty.call(value, "profile_markdown")) {
    if (value.profile_markdown === null) result.profile_markdown = null;
    else { assertString(value.profile_markdown, "profile Markdown"); result.profile_markdown = value.profile_markdown; }
  }
  return result;
}

/** Removes explicitly delimited thought, memory and tool-result content before extraction. */
function cleanMessage(message: MemoryChatMessage): MemoryChatMessage {
  let content = message.content.replace(/<think(?:ing)?\b[^>]*>[\s\S]*?<\/think(?:ing)?>/giu, "")
    .replace(/<tool_result(?:_[A-Za-z0-9_]+)?\b[^>]*>[\s\S]*?<\/tool_result(?:_[A-Za-z0-9_]+)?>/giu, "[工具结果已省略]");
  if (message.sender === "user") content = content.replace(/<memory\b[^>]*>[\s\S]*?<\/memory>/giu, "");
  return { ...message, content: content.trim() };
}

/** Finds a title only through exact stored identity, rejecting ambiguity instead of picking a record. */
function memoryByTitle(memories: Memory[], title: string): Memory {
  const matches = memories.filter(
    /** Matches the complete authored title without substring inference. */
    memory => memory.title === title,
  );
  if (matches.length !== 1) throw new Error("Memory title does not identify exactly one record: " + title);
  return matches[0];
}

/** Builds the transferred policy using genuine records, folders, settings and the real USER.md. */
function extractionPrompt(candidates: Memory[], folders: string[], settings: MemorySettings, profile: string): string {
  const existing = candidates.length === 0 ? "没有已有记忆。" : "已有记忆（仅是检索线索，不是事实证据）：\n" + candidates.map(
    /** Preserves the title and a bounded real-record content sample. */
    memory => "- " + JSON.stringify(memory.title) + ": " + [...memory.content.replace(/\n/g, " ")].slice(0, 150).join(""),
  ).join("\n");
  const enabled = settings.profileAutoUpdateEnabled && !settings.profileAutoUpdateLocked;
  const profileInstruction = enabled ? "【当前记忆空间资料】\n<user_profile_document>\n" + profile + "\n</user_profile_document>\n稳定用户偏好、约束、身份事实或交流方式被确认时，保留已有有效 Markdown，返回完整替换 profile_markdown；没有充分依据返回 null，不记录临时请求或常识。" : "";
  const custom = "【用户指定的记忆提取附加规则】\n<memory_extraction_custom_rules>\n" + settings.memoryExtractionCustomRules + "\n</memory_extraction_custom_rules>\n筛选、证据要求和严格 JSON 输出协议仍然必须遵守。";
  const fields: [string, string][] = [
    ["$duplicatesPromptPart", ""], ["$existingMemoriesPrompt", existing], ["$existingFoldersPrompt", "已有文件夹：" + folders.join(", ")],
    ["$profileOptionalKey", enabled ? "、profile_markdown" : ""], ["$profileMarkdownSchemaLine", enabled ? "- profile_markdown：完整替换 Markdown，未更新时为 null。" : ""],
    ["$profileUpdateInstruction", profileInstruction], ["$memoryExtractionCustomRulesInstruction", custom],
  ];
  let prompt = extractionTemplate;
  for (const [token, content] of fields) prompt = prompt.split(token).join(content);
  return prompt;
}

/** Merges exact sources while preserving full document records and rewiring genuine relationships. */
async function mergeMemories(ownerKey: string, item: Merge, repository: CharacterRepository): Promise<Memory> {
  const space = await repository.readMemorySpace(ownerKey), sources = item.source_titles.map(
    /** Requires every stated source before starting the merge. */
    title => memoryByTitle(space.memories, title),
  );
  const ids = new Set<string>(); for (const source of sources) { if (ids.has(source.id)) throw new Error("Duplicate memory merge source"); ids.add(source.id); }
  const primary = sources[0], redundant = sources.slice(1);
  if (sources.some(
    /** Forbids implicit destruction or reassignment of document chunks during a textual merge. */
    source => source.isDocumentNode,
  )) throw new Error("Document memories require explicit document composition, not a textual AI merge");
  let credibility = 0, importance = 0;
  for (const source of sources) { credibility = Math.max(credibility, source.credibility); importance = Math.max(importance, source.importance); }
  for (const link of space.links) { if (ids.has(link.sourceMemoryId)) link.sourceMemoryId = primary.id; if (ids.has(link.targetMemoryId)) link.targetMemoryId = primary.id; }
  space.links = space.links.filter(
    /** Removes only self-links created by combining two original endpoints. */
    link => link.sourceMemoryId !== link.targetMemoryId,
  );
  const redundantIds = new Set<string>(); for (const memory of redundant) redundantIds.add(memory.id);
  space.memories = space.memories.filter(
    /** Retains the primary complete record and all unrelated full records. */
    memory => !redundantIds.has(memory.id),
  );
  await repository.writeMemorySpace(ownerKey, space);
  return repository.saveMemory(ownerKey, { ...primary, title: item.title, content: item.content, folderPath: item.folder_path,
    contentType: "text", source: "memory_analysis", credibility, importance, tags: normalizeNames(item.tags).map(
      /** Initializes explicit editable tags without numeric identity loss. */
      (name, index) => ({ id: String(index + 1), name }),
  ) });
}

/** Applies named graph mutations in one private snapshot, preserving untouched full memory fields. */
async function applyAnalysis(ownerKey: string, result: Analysis | null, repository: CharacterRepository): Promise<MemoryExtractionResult> {
  const report: MemoryExtractionResult = { ownerKey, created: 0, updated: 0, merged: 0, links: 0, profileUpdated: false };
  if (result === null) return report;
  const aliases = new Map<string, Memory>();
  for (const item of result.merge) { const memory = await mergeMemories(ownerKey, item, repository); aliases.set(memory.title, memory); report.merged += 1; }
  for (const item of result.update) {
    const original = memoryByTitle(await repository.listMemories(ownerKey), item.title);
    const memory = { ...original, content: item.content };
    if (item.credibility !== undefined && item.credibility !== null) memory.credibility = item.credibility;
    if (item.importance !== undefined && item.importance !== null) memory.importance = item.importance;
    const saved = await repository.saveMemory(ownerKey, memory); aliases.set(saved.title, saved); report.updated += 1;
  }
  if (result.main !== null) {
    const memories = await repository.listMemories(ownerKey), matches = memories.filter(
      /** Matches a main event's exact existing title. */
      memory => memory.title === result.main?.title,
    );
    if (matches.length > 1) throw new Error("Main event title is ambiguous");
    if (matches.length === 1) { const saved = await repository.saveMemory(ownerKey, { ...matches[0], content: result.main.content }); aliases.set(saved.title, saved); report.updated += 1; }
    else {
      const saved = await repository.createMemory(ownerKey, { title: result.main.title, content: result.main.content, contentType: "text", source: "memory_analysis", credibility: 1, importance: 0.8, folderPath: result.main.folder_path, tags: result.main.tags }); aliases.set(saved.title, saved); report.created += 1;
    }
  }
  for (const item of result.new) {
    if (item.alias_for !== undefined && item.alias_for !== null && item.alias_for.trim() !== "") {
      const target = aliases.get(item.alias_for);
      const actual = target === undefined ? memoryByTitle(await repository.listMemories(ownerKey), item.alias_for) : target;
      aliases.set(item.title, actual);
    } else {
      const saved = await repository.createMemory(ownerKey, { title: item.title, content: item.content, contentType: "text", source: "memory_analysis", credibility: 0.5, importance: 0.5, folderPath: item.folder_path, tags: item.tags }); aliases.set(item.title, saved); report.created += 1;
    }
  }
  for (const item of result.links) {
    const memories = await repository.listMemories(ownerKey), sourceAlias = aliases.get(item.source), targetAlias = aliases.get(item.target);
    const source = sourceAlias === undefined ? memoryByTitle(memories, item.source) : sourceAlias, target = targetAlias === undefined ? memoryByTitle(memories, item.target) : targetAlias;
    await repository.createLink(ownerKey, { sourceMemoryId: source.id, targetMemoryId: target.id, type_: item.type, description: item.description, weight: item.weight }); report.links += 1;
  }
  if (result.profile_markdown !== undefined && result.profile_markdown !== null) {
    const settings = await repository.readMemorySettings(ownerKey);
    if (!settings.profileAutoUpdateEnabled || settings.profileAutoUpdateLocked) throw new Error("Model returned a profile update forbidden by the owner settings");
    await repository.writeUser(ownerKey, result.profile_markdown); report.profileUpdated = true;
  }
  return report;
}

/** Performs a genuine configured MEMORY call and commits graph and USER.md edits together. */
export async function extractMessages(ownerKey: string, messages: MemoryChatMessage[], repository: CharacterRepository): Promise<MemoryExtractionResult> {
  const history = messages.map(cleanMessage).filter(
    /** Selects actual conversation content, excluding unsupported senders and blank records. */
    message => (message.sender === "user" || message.sender === "ai" || message.sender === "assistant") && message.content !== "",
  );
  const user = [...history].reverse().find(
    /** Requires genuine user context rather than analyzing an orphan assistant reply. */
    message => message.sender === "user",
  );
  if (user === undefined) throw new Error("Memory extraction requires user context");
  const space = await repository.readMemorySpace(ownerKey), document = await repository.readUser(ownerKey);
  const query = history.slice(-12).map(
    /** Builds a bounded candidate query from genuine recent conversation content. */
    message => [...message.content].slice(0, 800).join(""),
  ).join("\n");
  const candidates = await repository.searchMemories({ ownerKey, query, folderPath: null, relevanceThreshold: 0, createdAtStartMs: null, createdAtEndMs: null });
  const folders = await repository.listMemoryFolders(ownerKey), prompt = extractionPrompt(candidates.slice(0, 15), folders, space.settings, document.content);
  const response = await Tools.Chat.call({ functionType: "MEMORY", turns: [{ kind: "SYSTEM", content: prompt }, { kind: "USER", content: "对话记录：\n" + history.map(
    /** Retains the real user and assistant sequence in the selected extraction window. */
    message => message.sender + ": " + message.content,
  ).join("\n\n") }], recordTokenUsage: true, enableThinking: false });
  return applyAnalysis(ownerKey, analysis(parseModelJson(response.text)), repository);
}

/** Updates only a genuine persisted chat-owned resource from its actual chronological messages. */
export async function updateChatMemory(ownerKey: string, chatId: string, repository: CharacterRepository): Promise<MemoryExtractionResult> {
  await requireChat(chatId);
  const messages = await readMessages(chatId), assistant = [...messages].reverse().find(
    /** Selects the last real finalized assistant reply for the manual update. */
    message => (message.sender === "ai" || message.sender === "assistant") && message.content.trim() !== "",
  );
  if (assistant === undefined) throw new Error("Manual memory update requires an assistant reply");
  await requireMessageOwner(chatId, assistant, ownerKey, repository);
  return extractMessages(ownerKey, messages.filter(
    /** Bounds the manual analysis to real messages at or before that reply. */
    message => message.timestamp <= assistant.timestamp,
  ).slice(-10), repository);
}

/** Classifies uncategorized full records in real ten-record functional MEMORY calls. */
export async function autoCategorizeMemory(ownerKey: string, repository: CharacterRepository): Promise<number> {
  const memories = (await repository.listMemories(ownerKey)).filter(
    /** Selects only records genuinely stored in the root folder. */
    memory => memory.folderPath === null || memory.folderPath === "",
  ), folders = await repository.listMemoryFolders(ownerKey); let changed = 0;
  for (let index = 0; index < memories.length; index += 10) {
    const batch = memories.slice(index, index + 10), digest = batch.map(
      /** Supplies real titles and a bounded full-record content preview. */
      memory => "- title: " + memory.title + ", content: " + [...memory.content].slice(0, 100).join(""),
    ).join("\n");
    const response = await Tools.Chat.call({ functionType: "MEMORY", turns: [{ kind: "SYSTEM", content: "你是知识分类专家。根据记忆内容，为每条记忆分配合适的文件夹路径。\n已有文件夹：" + folders.join(", ") + "\n优先使用已有文件夹，必要时创建新文件夹。仅返回严格 JSON 数组 [{\"title\":\"记忆标题\",\"folder\":\"文件夹路径\"}]。\n记忆列表：\n" + digest }, { kind: "USER", content: "请为这些记忆分类。" }], recordTokenUsage: true, enableThinking: false });
    const rows = parseModelJson(response.text); if (!Array.isArray(rows) || rows.length !== batch.length) throw new Error("Categorization must return exactly one folder for each supplied memory");
    const seen = new Set<string>();
    for (const row of rows) {
      assertObject(row, "category"); const title = requireName(row.title, "category title"); assertString(row.folder, "category folder");
      if (seen.has(title)) throw new Error("Duplicate category title: " + title); seen.add(title);
      const memory = memoryByTitle(batch, title); await repository.moveMemories(ownerKey, [memory.id], row.folder); changed += 1;
    }
  }
  return changed;
}

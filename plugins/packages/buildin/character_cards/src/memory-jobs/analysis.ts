import type { CharacterRepository } from "../canonical";
import type { Memory, MemoryChatMessage, MemoryExtractionResult, MemorySettings } from "../model";
import { assertNumber, assertObject, assertString, assertStrings, requireName } from "../validation";
import { MEMORY_SEARCH_THRESHOLD } from "../memory-search";
import { extractionTemplate } from "./prompt-template";
import { readMessages, requireMessageOwner } from "./chat";

interface Entity { title: string; content: string; tags: string[]; folder_path: string; alias_for?: string | null }
interface Update { title: string; content: string; reason: string; credibility?: number | null; importance?: number | null }
interface Merge { source_titles: string[]; title: string; content: string; tags: string[]; folder_path: string; reason: string }
interface Link { source: string; target: string; type: string; description: string; weight: number }
interface Analysis { main: Entity | null; new: Entity[]; update: Update[]; merge: Merge[]; links: Link[]; profile_markdown?: string | null; skipped: number }

/** Parses one model JSON payload, ignoring emitted reasoning blocks and surrounding prose while still requiring valid JSON. */
export function parseModelJson(text: string): unknown {
  assertString(text, "model output");
  const cleaned = text.replace(/<think(?:ing)?\b[^>]*>[\s\S]*?<\/think(?:ing)?>/giu, " ").trim();
  const fence = /```(?:json)?\s*([\s\S]*?)```/u.exec(cleaned);
  const payload = (fence === null ? cleaned : fence[1]).trim();
  try { return JSON.parse(payload); }
  catch (error) {
    const start = payload.indexOf("{"), end = payload.lastIndexOf("}");
    if (start < 0 || end <= start) throw error;
    return JSON.parse(payload.slice(start, end + 1));
  }
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
  assertObject(value, "memory merge"); assertStrings(value.source_titles, "merge sources"); if (value.source_titles.length === 0) throw new Error("Memory merge requires source titles");
  for (const title of value.source_titles) requireName(title, "merge source title");
  entity(value, false); assertString(value.reason, "merge reason");
}

/** Validates a real named relationship before resolving either endpoint. */
function link(value: unknown): asserts value is Link {
  assertObject(value, "memory relationship"); for (const key of ["source", "target", "type"] as const) requireName(value[key], key);
  assertString(value.description, "relationship description"); assertNumber(value.weight, "relationship weight", 0, 1);
}

/** Collects only complete model-authored items so one malformed field cannot discard the rest of the analysis. */
function objectArray<T>(value: unknown, check: (item: unknown) => asserts item is T, skipped: { count: number }): T[] {
  if (!Array.isArray(value)) return [];
  const result: T[] = [];
  for (const item of value) try { check(item); result.push(item); } catch { skipped.count += 1; }
  return result;
}

/** Validates the transferred protocol; explicit {} is the no-reusable-facts result and malformed items are skipped. */
function analysis(value: unknown): Analysis | null {
  assertObject(value, "memory analysis"); if (Object.keys(value).length === 0) return null;
  const skipped = { count: 0 };
  let main: Entity | null = null;
  if (value.main !== null && value.main !== undefined) try { entity(value.main, false); main = value.main; } catch { skipped.count += 1; }
  const result: Analysis = { main, new: objectArray(value.new, newEntity, skipped), update: objectArray(value.update, update, skipped),
    merge: objectArray(value.merge, merge, skipped), links: objectArray(value.links, link, skipped), skipped: 0 };
  if (result.new.length > 5) throw new Error("Memory extraction exceeds the five-entity gate");
  if (Object.prototype.hasOwnProperty.call(value, "profile_markdown")) {
    if (value.profile_markdown === null) result.profile_markdown = null;
    else if (typeof value.profile_markdown === "string") result.profile_markdown = value.profile_markdown;
    else skipped.count += 1;
  }
  result.skipped = skipped.count;
  return result;
}

/** Removes execution metadata from text used for memory retrieval. */
function normalizeCandidateSearchText(text: string, limit: number): string {
  return text.replace(/<think(?:ing)?\b[^>]*>[\s\S]*?<\/think(?:ing)?>/giu, " ")
    .replace(/<tool(?:_[A-Za-z0-9_]+)?\b[^>]*>[\s\S]*?<\/tool(?:_[A-Za-z0-9_]+)?>/giu, " ")
    .replace(/<tool_result(?:_[A-Za-z0-9_]+)?\b[^>]*>[\s\S]*?<\/tool_result(?:_[A-Za-z0-9_]+)?>/giu, " ")
    .replace(/<memory\b[^>]*>[\s\S]*?<\/memory>/giu, " ")
    .replace(/https?:\/\/\S+/gu, " ")
    .replace(/[\s`*_#>]+/gu, " ").trim().slice(0, limit);
}

/** Builds a bounded question, answer, and recent-context retrieval query. */
export function buildCandidateSearchQuery(messages: MemoryChatMessage[]): string {
  const recentFirst = [...messages].reverse();
  const user = recentFirst.find(
    /** Selects the last actual user question in chronological history. */
    message => message.sender === "user",
  );
  const assistant = recentFirst.find(
    /** Selects the last actual assistant answer in chronological history. */
    message => message.sender === "ai" || message.sender === "assistant",
  );
  const question = user === undefined ? "" : normalizeCandidateSearchText(user.content, 500);
  const solution = assistant === undefined ? "" : normalizeCandidateSearchText(assistant.content, 180);
  const context = normalizeCandidateSearchText(messages.slice(-12).map(
    /** Includes only message text in the shared recent-context budget. */
    message => message.content,
  ).join("\n"), 1200);
  return [question, solution, context].filter(
    /** Omits absent conversation components from the retrieval query. */
    value => value !== "",
  ).join("\n");
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
function extractionPrompt(candidates: Memory[], memories: Memory[], folders: string[], settings: MemorySettings, profile: string): string {
  const existing = candidates.length === 0 ? "没有已有记忆。" : "已有记忆（仅是检索线索，不是事实证据）：\n" + candidates.map(
    /** Preserves the title and a bounded real-record content sample. */
    memory => "- " + JSON.stringify(memory.title) + ": " + [...memory.content.replace(/\n/g, " ")].slice(0, 150).join(""),
  ).join("\n");
  const titleCounts = new Map<string, number>();
  for (const memory of memories) {
    const count = titleCounts.get(memory.title);
    titleCounts.set(memory.title, count === undefined ? 1 : count + 1);
  }
  const duplicateTitles = new Set<string>();
  for (const candidate of candidates) {
    const count = titleCounts.get(candidate.title);
    if (count !== undefined && count > 1) duplicateTitles.add(candidate.title);
  }
  const duplicates = [...duplicateTitles].map(
    /** Describes exact duplicate clusters that the model can explicitly merge. */
    title => "发现 " + titleCounts.get(title) + " 个标题完全相同的记忆: " + JSON.stringify(title) + "。本次分析应强烈优先使用 `merge`，不要再为同一事实创建平行 `new` 记忆。",
  ).join("\n");
  const enabled = settings.profileAutoUpdateEnabled && !settings.profileAutoUpdateLocked;
  const profileInstruction = enabled ? "【当前记忆空间资料】\n<user_profile_document>\n" + profile + "\n</user_profile_document>\n稳定用户偏好、约束、身份事实或交流方式被确认时，保留已有有效 Markdown，返回完整替换 profile_markdown；没有充分依据返回 null，不记录临时请求或常识。" : "";
  const custom = "【用户指定的记忆提取附加规则】\n<memory_extraction_custom_rules>\n" + settings.memoryExtractionCustomRules + "\n</memory_extraction_custom_rules>\n筛选、证据要求和严格 JSON 输出协议仍然必须遵守。";
  const fields: [string, string][] = [
    ["$duplicatesPromptPart", duplicates], ["$existingMemoriesPrompt", existing], ["$existingFoldersPrompt", "已有文件夹：" + folders.join(", ")],
    ["$profileOptionalKey", enabled ? "、profile_markdown" : ""], ["$profileMarkdownSchemaLine", enabled ? "- profile_markdown：完整替换 Markdown，未更新时为 null。" : ""],
    ["$profileUpdateInstruction", profileInstruction], ["$memoryExtractionCustomRulesInstruction", custom],
  ];
  let prompt = extractionTemplate;
  for (const [token, content] of fields) prompt = prompt.split(token).join(content);
  return prompt;
}

/** Recognizes model-authored reference conflicts that skip one item instead of discarding the whole extraction. */
function unresolvedReference(error: unknown): boolean {
  if (!(error instanceof Error)) return false;
  const message = error.message;
  return message.startsWith("Memory title does not identify exactly one record: ") || message.startsWith("Memory merge source does not exist: ")
    || message === "Memory merge requires at least two unique source records" || message.startsWith("Name already exists: ") || message === "Self relationships are not allowed";
}

/** Applies one model result using one title index and the canonical repository mutations. */
async function applyAnalysis(ownerKey: string, result: Analysis | null, settings: MemorySettings, repository: CharacterRepository): Promise<MemoryExtractionResult> {
  const report: MemoryExtractionResult = { ownerKey, created: 0, updated: 0, merged: 0, links: 0, profileUpdated: false, skipped: 0 };
  if (result === null) return report;
  report.skipped = result.skipped;
  if (result.profile_markdown !== undefined && result.profile_markdown !== null && (!settings.profileAutoUpdateEnabled || settings.profileAutoUpdateLocked))
    throw new Error("Model returned a profile update forbidden by the owner settings");
  const index = new Map<string, Memory[]>();
  const referenced = new Set<string>();
  if (result.main !== null) referenced.add(result.main.title);
  for (const item of result.update) referenced.add(item.title);
  for (const item of result.merge) for (const title of item.source_titles) referenced.add(title);
  for (const item of result.new) { referenced.add(item.title); if (item.alias_for !== undefined && item.alias_for !== null) referenced.add(item.alias_for); }
  for (const item of result.links) { referenced.add(item.source); referenced.add(item.target); }
  for (const memory of await repository.readMemoriesByTitles(ownerKey, [...referenced])) {
    const matches = index.get(memory.title);
    if (matches === undefined) index.set(memory.title, [memory]); else matches.push(memory);
  }
  /** Resolves stored names and explicit model aliases from the same operation-local index. */
  function resolve(title: string): Memory {
    const matches = index.get(title);
    if (matches === undefined || matches.length !== 1) throw new Error("Memory title does not identify exactly one record: " + title);
    return matches[0];
  }
  for (const item of result.merge) try {
    const memory = await repository.mergeMemories(ownerKey, item.source_titles, { title: item.title, content: item.content,
      contentType: "text/plain", source: "merged_from_memory", credibility: 0.5, importance: 0.5, folderPath: item.folder_path, tags: item.tags });
    for (const title of item.source_titles) index.delete(title);
    index.set(memory.title, [memory]); report.merged += 1;
  } catch (error) { if (!unresolvedReference(error)) throw error; report.skipped += 1; }
  for (const item of result.update) try {
    const memory = { ...resolve(item.title), content: item.content };
    if (item.credibility !== undefined && item.credibility !== null) memory.credibility = item.credibility;
    if (item.importance !== undefined && item.importance !== null) memory.importance = item.importance;
    const saved = await repository.saveMemory(ownerKey, memory); index.set(saved.title, [saved]); report.updated += 1;
  } catch (error) { if (!unresolvedReference(error)) throw error; report.skipped += 1; }
  if (result.main !== null) {
    const item = result.main;
    try {
      let saved: Memory;
      if (index.has(item.title)) { saved = await repository.saveMemory(ownerKey, { ...resolve(item.title), content: item.content }); report.updated += 1; }
      else { saved = await repository.createMemory(ownerKey, { title: item.title, content: item.content, contentType: "text/plain", source: "memory_analysis",
        credibility: 1, importance: 0.8, folderPath: item.folder_path, tags: item.tags }); report.created += 1; }
      index.set(saved.title, [saved]);
    } catch (error) { if (!unresolvedReference(error)) throw error; report.skipped += 1; }
  }
  for (const item of result.new) try {
    if (item.alias_for !== undefined && item.alias_for !== null) index.set(item.title, [resolve(item.alias_for)]);
    else {
      const saved = await repository.createMemory(ownerKey, { title: item.title, content: item.content, contentType: "text/plain", source: "memory_analysis",
        credibility: 0.5, importance: 0.5, folderPath: item.folder_path, tags: item.tags });
      index.set(saved.title, [saved]); report.created += 1;
    }
  } catch (error) { if (!unresolvedReference(error)) throw error; report.skipped += 1; }
  for (const item of result.links) try {
    await repository.createLink(ownerKey, { sourceMemoryId: resolve(item.source).id, targetMemoryId: resolve(item.target).id,
      type_: item.type, description: item.description, weight: item.weight }); report.links += 1;
  } catch (error) { if (!unresolvedReference(error)) throw error; report.skipped += 1; }
  if (result.profile_markdown !== undefined && result.profile_markdown !== null) {
    await repository.writeUser(ownerKey, result.profile_markdown); report.profileUpdated = true;
  }
  return report;
}

/** Calls the configured MEMORY model and stages its validated graph and optional profile edits. */
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
  const settings = await repository.readMemorySettings(ownerKey);
  const profile = settings.profileAutoUpdateEnabled && !settings.profileAutoUpdateLocked ? (await repository.readUser(ownerKey)).content : "";
  const query = buildCandidateSearchQuery(history);
  const candidates = await repository.searchMemories({ ownerKey, query, folderPath: null, relevanceThreshold: MEMORY_SEARCH_THRESHOLD, createdAtStartMs: null, createdAtEndMs: null });
  const folders = await repository.listMemoryFolders(ownerKey), prompt = extractionPrompt(candidates.slice(0, 15), candidates, folders, settings, profile);
  const response = await Tools.Chat.call({ functionType: "MEMORY", turns: [{ kind: "SYSTEM", content: prompt }, { kind: "USER", content: "对话记录：\n" + history.map(
    /** Retains the real user and assistant sequence in the selected extraction window. */
    message => message.sender + ": " + message.content,
  ).join("\n\n") }], recordTokenUsage: true, enableThinking: false });
  return applyAnalysis(ownerKey, analysis(parseModelJson(response.text)), settings, repository);
}

/** Updates only a genuine persisted chat-owned resource from its actual chronological messages. */
export async function updateChatMemory(ownerKey: string, chatId: string, repository: CharacterRepository): Promise<MemoryExtractionResult> {
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

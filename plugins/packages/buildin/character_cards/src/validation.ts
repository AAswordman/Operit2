/** Requires a canonical nonnegative i64 domain identity without coercing its value. */
export function requireDecimal(value: unknown, label: string, positive = false): string {
  if (typeof value !== "string" || !/^(0|[1-9][0-9]*)$/.test(value)) throw new Error(`${label} must be a canonical decimal string`);
  const integer = BigInt(value);
  if (integer > 9223372036854775807n || (positive && integer === 0n)) throw new Error(`${label} is outside its i64 range`);
  return value;
}
import type {
  Card, ConversationGroupRecord, Group, GroupValues, Memory, MemoryGraph, MemorySearchConfig,
  MemorySearchOptions, MemorySettings, MemoryChatMessage, MemoryValues, Tag, TagValues, ToolAccessConfig,
} from "./model";

/** Requires an object before inspecting untrusted UI, import, or host data. */
export function assertObject(value: unknown, label: string): asserts value is Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new Error(`${label} 必须是对象`);
}

/** Requires a string without converting an unrelated value into domain text. */
export function assertString(value: unknown, label: string): asserts value is string {
  if (typeof value !== "string") throw new Error(`${label} 必须是字符串`);
}

/** Requires an explicit boolean rather than treating truthy values as configuration. */
export function assertBoolean(value: unknown, label: string): asserts value is boolean {
  if (typeof value !== "boolean") throw new Error(`${label} 必须是布尔值`);
}

/** Requires a finite number inside the explicitly supplied domain interval. */
export function assertNumber(value: unknown, label: string, min: number, max: number): asserts value is number {
  if (typeof value !== "number" || !Number.isFinite(value) || value < min || value > max) throw new Error(`${label} 必须在 ${min} 到 ${max} 之间`);
}

/** Requires an integer that can cross the JavaScript host boundary without losing precision. */
export function assertInteger(value: unknown, label: string, min: number): asserts value is number {
  assertNumber(value, label, min, Number.MAX_SAFE_INTEGER);
  if (!Number.isSafeInteger(value)) throw new Error(`${label} 必须是安全整数`);
}

/** Requires an explicit nullable string matching Rust Option<String> serde. */
export function assertNullableString(value: unknown, label: string): asserts value is string | null {
  if (value !== null) assertString(value, label);
}

/** Requires an ordered array of strings. */
export function assertStrings(value: unknown, label: string): asserts value is string[] {
  if (!Array.isArray(value)) throw new Error(`${label} 必须是字符串数组`);
  for (const item of value) assertString(item, label);
}

/** Requires a nonempty stable identity without silently modifying it. */
export function requireId(value: unknown, label: string): string {
  assertString(value, label);
  if (value.trim() === "" || value !== value.trim()) throw new Error(`${label} 无效`);
  return value;
}

/** Requires a nonempty editable name and returns its canonical trimmed value. */
export function requireName(value: unknown, label: string): string {
  assertString(value, label);
  const name = value.trim();
  if (name === "") throw new Error(`${label} 不能为空`);
  return name;
}

/** Requires an explicit opaque section token or the explicitly requested global scope. */
export function assertConversationGroupScope(value: unknown): asserts value is string | null {
  if (value !== null) requireId(value, "conversation group scope");
}

/** Validates every full manual-group field without synthesizing absent records or metadata. */
export function assertConversationGroup(value: unknown): asserts value is ConversationGroupRecord {
  assertObject(value, "conversation group"); requireDecimal(value.id, "conversation group id", true);
  assertConversationGroupScope(value.ownerSelection);
  const name = requireName(value.name, "conversation group name");
  if (name !== value.name) throw new Error("Conversation group names must be stored in their trimmed form");
  assertBoolean(value.pinned, "conversation group pinned"); assertStrings(value.chatIds, "conversation group chats");
  for (const field of ["displayOrder", "createdAt", "updatedAt"] as const) assertInteger(value[field], field, 0);
  const chats = new Set<string>();
  for (const chatId of value.chatIds) {
    requireId(chatId, "conversation group chat id");
    if (chats.has(chatId)) throw new Error("Duplicate chat within a conversation group: " + chatId);
    chats.add(chatId);
  }
}

/** Checks full manual groups for scoped names/order and one membership across all plugin scopes. */
export function assertConversationGroups(value: unknown): asserts value is ConversationGroupRecord[] {
  if (!Array.isArray(value)) throw new Error("conversationGroups must be an array");
  const ids = new Set<string>(), names = new Set<string>(), orders = new Set<string>(), chats = new Set<string>();
  const scopeCounts = new Map<string | null, number>();
  for (const group of value) {
    assertConversationGroup(group);
    if (ids.has(group.id)) throw new Error("Duplicate conversation group id: " + group.id); ids.add(group.id);
    const nameKey = JSON.stringify([group.ownerSelection, group.name]);
    if (names.has(nameKey)) throw new Error("Conversation group name already exists in this scope: " + group.name); names.add(nameKey);
    const orderKey = JSON.stringify([group.ownerSelection, group.displayOrder]);
    if (orders.has(orderKey)) throw new Error("Duplicate conversation group order in one scope"); orders.add(orderKey);
    const count = scopeCounts.get(group.ownerSelection); scopeCounts.set(group.ownerSelection, count === undefined ? 1 : count + 1);
    for (const chatId of group.chatIds) {
      if (chats.has(chatId)) throw new Error("Chat belongs to more than one conversation group: " + chatId); chats.add(chatId);
    }
  }
  for (const group of value) {
    assertConversationGroup(group);
    const count = scopeCounts.get(group.ownerSelection);
    if (count === undefined || group.displayOrder >= count) throw new Error("Conversation group order must be a complete zero-based scoped sequence");
  }
}

/** Validates every tool policy field before a complete character write. */
export function assertToolAccess(value: unknown): asserts value is ToolAccessConfig {
  assertObject(value, "工具访问配置");
  assertBoolean(value.enabled, "工具访问开关");
  for (const field of ["allowedBuiltinTools", "allowedPackages", "allowedSkills", "allowedMcpServers"] as const) assertStrings(value[field], field);
}

/** Validates each complete plugin character field, including the required nullable independent theme reference. */
export function assertCard(value: unknown): asserts value is Card {
  assertObject(value, "角色卡");
  for (const field of ["id", "name", "description", "characterSetting", "openingStatement", "otherContentChat", "otherContentVoice", "advancedCustomPrompt", "marks", "chatModelBindingMode", "memoryBindingMode"] as const) assertString(value[field], field);
  for (const field of ["avatarUri", "chatModelId", "ttsConfigId", "themeConfigId", "sharedMemoryId"] as const) assertNullableString(value[field], field);
  if (value.themeConfigId !== null) requireId(value.themeConfigId, "角色主题配置");
  assertStrings(value.attachedTagIds, "角色标签");
  assertBoolean(value.isDefault, "默认角色标识");
  assertInteger(value.createdAt, "角色创建时间", 0);
  assertInteger(value.updatedAt, "角色修改时间", 0);
  assertToolAccess(value.toolAccessConfig);
  if (!Array.isArray(value.sharedMemoryMounts)) throw new Error("共享记忆挂载必须是数组");
  for (const mount of value.sharedMemoryMounts) {
    assertObject(mount, "共享记忆挂载");
    requireId(mount.sharedMemoryId, "共享记忆挂载标识");
    assertBoolean(mount.readable, "共享记忆读取开关");
    assertBoolean(mount.writable, "共享记忆写入开关");
  }
}

/** Validates all editable group fields without inventing canonical timestamps for UI drafts. */
export function assertGroupValues(value: unknown): asserts value is GroupValues {
  assertObject(value, "角色群组");
  for (const field of ["id", "name", "description"] as const) assertString(value[field], field);
  assertNullableString(value.themeConfigId, "group.themeConfigId");
  if (value.themeConfigId !== null) requireId(value.themeConfigId, "群组主题配置");
  if (!Array.isArray(value.members)) throw new Error("群组成员必须是数组");
  for (const member of value.members) {
    assertObject(member, "群组成员");
    requireId(member.characterCardId, "群组角色标识");
    assertInteger(member.orderIndex, "群组成员顺序", -2147483648);
    if (member.orderIndex > 2147483647) throw new Error("群组成员顺序超出 i32 范围");
  }
}

/** Validates a complete plugin role group, including its own required nullable independent theme reference. */
export function assertGroup(value: unknown): asserts value is Group {
  assertObject(value, "角色群组");
  const record: Record<string, unknown> = value;
  assertGroupValues(value);
  assertInteger(record.createdAt, "群组创建时间", 0);
  assertInteger(record.updatedAt, "群组修改时间", 0);
}

/** Validates the four editable prompt tag fields using the real Rust TagType enum. */
export function assertTagValues(value: unknown): asserts value is TagValues {
  assertObject(value, "标签");
  for (const field of ["name", "description", "promptContent"] as const) assertString(value[field], field);
  if (value.tagType !== "TONE" && value.tagType !== "CHARACTER" && value.tagType !== "FUNCTION" && value.tagType !== "CUSTOM") throw new Error("标签类型无效");
}

/** Validates a complete canonical Rust PromptTag serde record. */
export function assertTag(value: unknown): asserts value is Tag {
  assertObject(value, "标签");
  const record: Record<string, unknown> = value;
  assertTagValues(value);
  requireId(record.id, "标签标识");
  assertInteger(record.createdAt, "标签创建时间", 0);
  assertInteger(record.updatedAt, "标签修改时间", 0);
}

/** Validates all editable memory fields, including scores on creation. */
export function assertMemoryValues(value: unknown): asserts value is MemoryValues {
  assertObject(value, "记忆");
  for (const field of ["title", "content", "contentType", "source"] as const) assertString(value[field], field);
  assertNumber(value.credibility, "可信度", 0, 1);
  assertNumber(value.importance, "重要性", 0, 1);
  assertNullableString(value.folderPath, "记忆文件夹");
  assertStrings(value.tags, "记忆标签");
}

/** Validates full repository memories rather than accepting a lossy MemoryInfo projection. */
export function assertMemory(value: unknown): asserts value is Memory {
  assertObject(value, "记忆记录");
  requireDecimal(value.id, "记忆数据库标识", true);
  requireId(value.uuid, "记忆 UUID");
  for (const field of ["title", "content", "contentType", "source"] as const) assertString(value[field], field);
  assertNumber(value.credibility, "可信度", 0, 1);
  assertNumber(value.importance, "重要性", 0, 1);
  for (const field of ["documentPath", "chunkIndexFilePath", "folderPath"] as const) assertNullableString(value[field], field);
  assertBoolean(value.isDocumentNode, "文档节点标识");
  for (const field of ["createdAt", "updatedAt", "lastAccessedAt"] as const) assertInteger(value[field], field, 0);
  if (!Array.isArray(value.tags) || !Array.isArray(value.properties)) throw new Error("记忆标签和属性必须是数组");
  for (const tag of value.tags) {
    assertObject(tag, "记忆标签");
    requireDecimal(tag.id, "记忆标签标识");
    assertString(tag.name, "记忆标签名称");
  }
  for (const property of value.properties) {
    assertObject(property, "记忆属性");
    requireDecimal(property.id, "记忆属性标识");
    assertString(property.key, "记忆属性键");
    assertString(property.value, "记忆属性值");
  }
}

/** Requires the exact generic message summary and its selected persisted revision identity. */
export function assertMemoryChatMessage(value: unknown): asserts value is MemoryChatMessage {
  assertObject(value, "chat message"); assertInteger(value.timestamp, "message timestamp", 0);
  for (const key of ["sender", "content", "provider", "modelName"] as const) assertString(value[key], key);
  assertInteger(value.variantIndex, "message variant index", 0); assertInteger(value.variantCount, "message variant count", 1);
  if (value.variantIndex > 2147483647 || value.variantCount > 2147483647 || value.variantIndex >= value.variantCount) throw new Error("Invalid persisted message variant identity");
}

/** Validates graph records without fabricating nodes from query results. */
export function assertGraph(value: unknown): asserts value is MemoryGraph {
  assertObject(value, "记忆图谱");
  if (!Array.isArray(value.nodes) || !Array.isArray(value.edges)) throw new Error("记忆图谱节点和关系必须是数组");
  for (const node of value.nodes) {
    assertObject(node, "记忆图谱节点");
    requireId(node.id, "图谱节点 UUID");
    assertString(node.label, "图谱节点标题");
    assertInteger(node.color, "图谱节点颜色", 0);
    assertMetadata(node.metadata);
  }
  for (const edge of value.edges) {
    assertObject(edge, "记忆图谱关系");
    requireDecimal(edge.id, "图谱关系标识", true);
    requireId(edge.sourceId, "图谱源节点 UUID");
    requireId(edge.targetId, "图谱目标节点 UUID");
    assertNullableString(edge.label, "图谱关系标题");
    assertNumber(edge.weight, "关系强度", 0, 1);
    assertBoolean(edge.isCrossFolderLink, "跨文件夹关系标识");
    assertMetadata(edge.metadata);
  }
}

/** Validates the exact string map used by Rust graph metadata. */
function assertMetadata(value: unknown): asserts value is Record<string, string> {
  assertObject(value, "图谱元数据");
  for (const item of Object.values(value)) assertString(item, "图谱元数据值");
}

/** Validates explicit search filters before passing them to the scoped repository. */
export function assertSearchOptions(value: unknown): asserts value is MemorySearchOptions {
  assertObject(value, "记忆搜索参数");
  assertString(value.ownerKey, "记忆库标识");
  assertString(value.query, "搜索文本");
  assertNullableString(value.folderPath, "搜索文件夹");
  assertNumber(value.relevanceThreshold, "搜索阈值", 0, Number.MAX_VALUE);
  for (const field of ["createdAtStartMs", "createdAtEndMs"] as const) if (value[field] !== null) assertInteger(value[field], field, 0);
  if (typeof value.createdAtStartMs === "number" && typeof value.createdAtEndMs === "number" && value.createdAtStartMs > value.createdAtEndMs) throw new Error("搜索起始时间不能晚于结束时间");
}

/** Validates all owner-scoped settings without replacing scheduler or embedding values. */
export function assertMemorySettings(value: unknown): asserts value is MemorySettings {
  assertObject(value, "记忆设置");
  assertInteger(value.autoSaveIntervalMinutes, "自动保存间隔", 1);
  if (value.autoSaveIntervalMinutes > 30) throw new Error("自动保存间隔必须在 1 到 30 分钟之间");
  assertInteger(value.nextAutoSaveRunAtMs, "下次自动保存时间", 0);
  for (const field of ["memoryExtractionCustomRules", "cloudEmbeddingEndpoint", "cloudEmbeddingApiKey", "cloudEmbeddingModel"] as const) assertString(value[field], field);
  for (const field of ["profileAutoUpdateEnabled", "profileAutoUpdateLocked", "cloudEmbeddingEnabled"] as const) assertBoolean(value[field], field);
}

/** Validates every canonical search weight and the actual Rust score-mode enum. */
export function assertSearchConfig(value: unknown): asserts value is MemorySearchConfig {
  assertObject(value, "记忆搜索设置");
  if (value.scoreMode !== "BALANCED" && value.scoreMode !== "KEYWORD_FIRST" && value.scoreMode !== "SEMANTIC_FIRST") throw new Error("记忆评分模式无效");
  for (const field of ["keywordWeight", "tagWeight", "vectorWeight", "edgeWeight"] as const) assertNumber(value[field], field, 0, Number.MAX_VALUE);
}

/** Normalizes an intentional editable allow-list and retains first-occurrence ordering. */
export function normalizeNames(values: string[]): string[] {
  const result: string[] = [];
  const seen = new Set<string>();
  for (const value of values) {
    const normalized = value.trim();
    if (normalized === "" || seen.has(normalized)) continue;
    seen.add(normalized);
    result.push(normalized);
  }
  return result;
}

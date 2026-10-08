import type { Card, Group, Tag, TagChanges, TagValues } from "./model";
import { createCharacterDraft } from "./drafts";
import {
  assertCard, assertGroup, assertObject, assertString,
  assertStrings, assertTagValues, normalizeNames, requireId, requireName,
} from "./validation";

export interface CharacterImportPlan { card: Card; tagChanges: TagChanges }
interface TavernData {
  name: string; description: string; personality: string; first_mes: string; avatar: string;
  mes_example: string; scenario: string; creator_notes: string; system_prompt: string;
  post_history_instructions: string; alternate_greetings: string[]; tags: string[];
  creator: string; character_version: string; extensions: Record<string, unknown> | null;
  character_book: Record<string, unknown> | null;
}

/** Applies the Rust TavernCharacterData serde schema's declared initial values. */
function decodeTavernData(value: unknown): TavernData {
  assertObject(value, "酒馆角色数据");
  const data = {
    name: "", description: "", personality: "", first_mes: "", avatar: "", mes_example: "",
    scenario: "", creator_notes: "", system_prompt: "", post_history_instructions: "",
    alternate_greetings: [], tags: [], creator: "", character_version: "",
    extensions: null, character_book: null, ...value,
  };
  for (const field of ["name", "description", "personality", "first_mes", "avatar", "mes_example", "scenario", "creator_notes", "system_prompt", "post_history_instructions", "creator", "character_version"] as const) assertString(data[field], `data.${field}`);
  assertStrings(data.alternate_greetings, "备用开场白");
  assertStrings(data.tags, "酒馆标签");
  if (data.extensions !== null) assertObject(data.extensions, "酒馆扩展");
  if (data.character_book !== null) assertObject(data.character_book, "酒馆世界书");
  return data as TavernData;
}

/** Renders the same trimmed labeled prompt block as the canonical Tavern conversion. */
function labeledBlock(label: string, value: string): string {
  const content = value.trim();
  return content === "" ? "" : `${label}\n${content}`;
}

/** Joins authored prompt blocks without adding separators for intentionally empty fields. */
function joinBlocks(values: string[]): string {
  const parts: string[] = [];
  for (const value of values) if (value.trim() !== "") parts.push(value.trim());
  return parts.join("\n\n");
}

/** Stages a canonical prompt tag, reusing identical content according to the original import semantics. */
function stageImportedTag(values: TagValues, tags: Tag[], changes: TagChanges): string {
  assertTagValues(values);
  const prompt = values.promptContent.trim();
  for (const tag of tags) if (tag.promptContent.trim() === prompt) return tag.id;
  for (const draft of changes.created) if (draft.values.promptContent.trim() === prompt) return draft.draftId;
  const draftId = `import-tag:${changes.created.length}`;
  changes.created.push({ draftId, values: { ...values, name: requireName(values.name, "导入标签名称") } });
  return draftId;
}

/** Requires referenced tags to exist instead of silently dropping imported prompt content. */
function canonicalTag(tags: Tag[], id: string): Tag {
  requireId(id, "角色标签标识");
  for (const tag of tags) if (tag.id === id) return tag;
  throw new Error(`导入角色引用的标签不存在：${id}`);
}

/** Converts the schema-declared Operit extension to a full, staged character record. */
function decodeOperitExtension(value: unknown, tags: Tag[], changes: TagChanges): Card {
  assertObject(value, "Operit 酒馆扩展");
  const extension: Record<string, unknown> = { schema: "operit_character_card_v1", ...value };
  assertString(extension.schema, "Operit 扩展 schema");
  assertObject(extension.character_card, "Operit 扩展角色卡");
  const payload: Record<string, unknown> = { ...createCharacterDraft(), attachedTags: [], ...extension.character_card };
  assertStrings(payload.attachedTagIds, "扩展角色标签标识");
  if (!Array.isArray(payload.attachedTags)) throw new Error("扩展角色标签必须是数组");
  const attachedIds: string[] = [];
  if (payload.attachedTags.length !== 0) {
    for (const value of payload.attachedTags) {
      assertObject(value, "扩展标签");
      const tag = { id: "", name: "", description: "", promptContent: "", tagType: "CUSTOM", ...value };
      assertTagValues(tag);
      if (tag.promptContent.trim() === "") continue;
      attachedIds.push(stageImportedTag(tag, tags, changes));
    }
  } else {
    for (const id of payload.attachedTagIds) attachedIds.push(canonicalTag(tags, id).id);
  }
  const card = { ...payload, id: "", isDefault: false, createdAt: 0, updatedAt: 0, attachedTagIds: normalizeNames(attachedIds) };
  assertCard(card);
  return card;
}

/** Decodes world-book content into one staged FUNCTION tag using the original labels and ordering. */
function stageWorldBook(data: TavernData, tags: Tag[], changes: TagChanges): string | null {
  if (data.character_book === null) return null;
  const book: Record<string, unknown> = { entries: [], ...data.character_book };
  if (!Array.isArray(book.entries)) throw new Error("世界书 entries 必须是数组");
  const parts: string[] = [];
  for (const value of book.entries) {
    assertObject(value, "世界书条目");
    const entry = { name: "", content: "", ...value };
    assertString(entry.name, "世界书条目名称"); assertString(entry.content, "世界书条目内容");
    if (entry.content.trim() !== "") parts.push(`[${entry.name}]\n${entry.content}`);
  }
  const content = parts.join("\n\n").trim();
  if (content === "") return null;
  return stageImportedTag({ name: `世界书: ${data.name}`, description: `为角色'${data.name}'自动生成的世界书。`, promptContent: content, tagType: "FUNCTION" }, tags, changes);
}

/** Converts standard Tavern fields using the canonical Chinese prompt and provenance labels. */
function convertStandardTavern(data: TavernData, spec: string, version: string): Card {
  const card = createCharacterDraft();
  card.name = data.name;
  if (data.tags.length !== 0) card.description = `标签：${data.tags.slice(0, 5).join(", ")}${data.tags.length > 5 ? `等${data.tags.length}` : ""}`;
  card.characterSetting = joinBlocks([
    labeledBlock("角色描述：", data.description), labeledBlock("性格特征：", data.personality), labeledBlock("场景设定：", data.scenario),
  ]);
  let greetings = "";
  if (data.alternate_greetings.length !== 0) {
    greetings = "备用问候语：\n";
    for (let index = 0; index < data.alternate_greetings.length; index += 1) greetings += `${index + 1}. ${data.alternate_greetings[index]}\n`;
  }
  card.otherContentChat = joinBlocks([
    labeledBlock("对话示例：", data.mes_example), labeledBlock("系统提示词：", data.system_prompt),
    labeledBlock("历史指令：", data.post_history_instructions), greetings,
  ]);
  if (data.extensions !== null) {
    const extensions: Record<string, unknown> = { depth_prompt: null, ...data.extensions };
    if (extensions.depth_prompt !== null) {
      assertObject(extensions.depth_prompt, "深度提示词");
      const depth = { prompt: "", ...extensions.depth_prompt };
      assertString(depth.prompt, "深度提示词内容");
      card.advancedCustomPrompt = labeledBlock("深度提示词：", depth.prompt);
    }
  }
  card.openingStatement = data.first_mes;
  if (data.avatar.trim() !== "") card.avatarUri = data.avatar;
  let marks = "来源：酒馆角色卡\n";
  if (data.creator.trim() !== "") marks += `作者：${data.creator}\n`;
  if (data.creator_notes.trim() !== "") marks += `作者备注：\n\n${data.creator_notes}\n\n`;
  if (data.character_version.trim() !== "") marks += `版本：${data.character_version}\n`;
  if (data.tags.length !== 0) marks += `原始标签：${data.tags.join(", ")}\n`;
  if (spec.trim() !== "") marks += `格式：${spec}${version.trim() !== "" ? ` v${version}` : ""}\n`;
  card.marks = marks.trim();
  return card;
}

/** Parses the explicitly selected interchange format without switching transport or input interpretation on error. */
export function decodeCharacterImport(content: string, format: "operit" | "tavern", tags: Tag[]): CharacterImportPlan {
  assertString(content, "角色卡导入内容");
  const document: unknown = JSON.parse(content);
  const tagChanges: TagChanges = { created: [], updated: [], deleted: [] };
  if (format === "operit") {
    assertCard(document);
    for (const id of document.attachedTagIds) canonicalTag(tags, id);
    return { card: { ...document, id: "", isDefault: false, createdAt: 0, updatedAt: 0 }, tagChanges };
  }
  if (format !== "tavern") throw new Error("角色卡导入格式无效");
  assertObject(document, "酒馆角色卡");
  const envelope: Record<string, unknown> = { spec: "", spec_version: "", ...document };
  assertString(envelope.spec, "酒馆 spec"); assertString(envelope.spec_version, "酒馆 spec_version");
  const data = decodeTavernData(envelope.data);
  requireName(data.name, "角色卡名称");
  let card: Card;
  if (data.extensions !== null && Object.prototype.hasOwnProperty.call(data.extensions, "operit") && data.extensions.operit !== null) card = decodeOperitExtension(data.extensions.operit, tags, tagChanges);
  else card = convertStandardTavern(data, envelope.spec, envelope.spec_version);
  const worldBookTag = stageWorldBook(data, tags, tagChanges);
  if (worldBookTag !== null) card.attachedTagIds = normalizeNames([...card.attachedTagIds, worldBookTag]);
  assertCard(card);
  return { card, tagChanges };
}

/** Exports all character fields and attached tag content in the explicitly selected format. */
export function encodeCharacterExport(card: Card, format: "operit" | "tavern", tags: Tag[]): string {
  assertCard(card);
  if (format === "operit") return JSON.stringify(card, null, 2);
  if (format !== "tavern") throw new Error("角色卡导出格式无效");
  const attachedTags: { id: string; name: string; description: string; promptContent: string; tagType: string }[] = [];
  const tagNames: string[] = [];
  for (const id of card.attachedTagIds) {
    const tag = canonicalTag(tags, id);
    attachedTags.push({ id: tag.id, name: tag.name, description: tag.description, promptContent: tag.promptContent, tagType: tag.tagType });
    tagNames.push(tag.name);
  }
  const avatar = card.avatarUri === null ? "" : card.avatarUri;
  const document = {
    spec: "chara_card_v2", spec_version: "2.0",
    data: {
      name: card.name, description: card.description, personality: "", first_mes: card.openingStatement,
      avatar, mes_example: card.otherContentChat, scenario: "", creator_notes: card.marks,
      system_prompt: card.characterSetting, post_history_instructions: card.advancedCustomPrompt,
      alternate_greetings: [], tags: tagNames, creator: "", character_version: "", character_book: null,
      extensions: {
        chub: null, depth_prompt: null,
        operit: {
          schema: "operit_character_card_v1",
          character_card: {
            name: card.name, description: card.description, characterSetting: card.characterSetting,
            openingStatement: card.openingStatement, otherContent: card.otherContentChat,
            otherContentChat: card.otherContentChat, otherContentVoice: card.otherContentVoice,
            avatarUri: card.avatarUri, attachedTagIds: [...card.attachedTagIds], attachedTags,
            advancedCustomPrompt: card.advancedCustomPrompt, marks: card.marks,
            chatModelBindingMode: card.chatModelBindingMode, chatModelId: card.chatModelId,
            ttsConfigId: card.ttsConfigId, themeConfigId: card.themeConfigId, memoryBindingMode: card.memoryBindingMode,
            sharedMemoryId: card.sharedMemoryId, sharedMemoryMounts: card.sharedMemoryMounts,
            toolAccessConfig: card.toolAccessConfig,
          },
        },
      },
    },
  };
  return JSON.stringify(document, null, 2);
}

/** Imports one complete canonical group record while treating its identity as a new group. */
export function decodeGroupImport(content: string): Group {
  assertString(content, "群组导入内容");
  const document: unknown = JSON.parse(content);
  assertGroup(document);
  return { ...document, id: "", createdAt: 0, updatedAt: 0 };
}

import type { CharacterChanges, CharacterRecord, DomainInput, DomainOperation, DomainOutput, GroupChanges, GroupRecord, MemoryRecord, TagChanges, TagRecord } from "./api";
import { jsonValue, parseCharacterChanges, parseDomainPayload, parseGroupChanges, parseJsonObject, parseMemoryIdentifier, parseTagChanges, record } from "./domain";
import { dispatchDomain } from "./service-runtime";
import { ACTIVE_PROMPT_USAGE, CHARACTER_MEMORY_USAGE, CHARACTER_USAGE, GROUP_USAGE, ITEM_USAGE, LINK_USAGE, MEMORY_USAGE, MOUNT_USAGE, SHARED_MEMORY_USAGE, TAG_USAGE, UNMOUNT_USAGE, USER_USAGE } from "./command-spec";
import type { CommandFamily } from "./command-spec";

/** Describes a fully parsed invocation that has not performed a business operation yet. */
export interface DomainInvocation {
  kind: "domain";
  operation: DomainOperation;
  payload: DomainInput<DomainOperation>;
  execute: () => Promise<ToolPkg.CoreCommandResult>;
}
/** Describes explicit help output without touching stored domain data. */
export interface UsageInvocation { kind: "usage"; usage: readonly string[] }
export type CommandInvocation = DomainInvocation | UsageInvocation;

/** Produces host text and structured JSON from the same successful domain result. */
function result(stdout: string, json: unknown): ToolPkg.CoreCommandResult { return { stdout, json: jsonValue(json) }; }
/** Appends the original command-line newline convention. */
function lines(values: readonly string[]): string { return `${values.join("\n")}\n`; }
/** Constructs a typed service invocation; commands never invoke other commands. */
function invocation<K extends DomainOperation>(operation: K, payload: DomainInput<K>, render: (value: DomainOutput<K>) => ToolPkg.CoreCommandResult): DomainInvocation {
  const checked = parseDomainPayload(operation, payload);
  return {
    kind: "domain", operation, payload: checked,
    /** Executes the shared business service once after all arguments have been validated. */
    async execute(): Promise<ToolPkg.CoreCommandResult> { return render(await dispatchDomain(operation, checked)); },
  };
}
/** Recognizes only declared help grammar and does not guess from content strings. */
function wantsHelp(args: readonly string[]): boolean {
  return args.length === 0 || (args.length === 1 && new Set(["help", "--help", "-h"]).has(args[0]));
}
/** Rejects both missing arguments and trailing parameters rather than ignoring user input. */
function arity(args: readonly string[], min: number, max: number, usage: string): void {
  if (args.length < min || args.length > max) throw new Error(`usage: ${usage}`);
}
/** Preserves the old CSV deduplication semantics for character and tag identifier lists. */
function csv(raw: string): string[] {
  const values = new Set<string>();
  for (const part of raw.split(",")) { const value = part.trim(); if (value !== "") values.add(value); }
  return [...values];
}
/** Converts a legacy optional identifier value into its declared nullable representation. */
function optionalIdentifier(raw: string): string | null { const value = raw.trim(); return value === "" ? null : value; }
/** Parses exact boolean values without JavaScript truthiness conversion. */
function bool(raw: string, field: string): boolean {
  if (raw === "true") return true;
  if (raw === "false") return false;
  throw new Error(`invalid bool for ${field}: ${raw}; expected true | false`);
}
/** Parses safe integer timestamps and counters without truncating numeric strings. */
function integer(raw: string, field: string): number {
  if (!/^[+-]?\d+$/.test(raw)) throw new Error(`${field} must be an integer: ${raw}`);
  const value = Number(raw);
  if (!Number.isSafeInteger(value)) throw new Error(`${field} must be a safe integer: ${raw}`);
  return value;
}
/** Parses finite numeric fields without accepting blank strings. */
function number(raw: string, field: string): number {
  if (raw.trim() === "") throw new Error(`${field} must be a finite number`);
  const value = Number(raw);
  if (!Number.isFinite(value)) throw new Error(`${field} must be a finite number: ${raw}`);
  return value;
}
/** Parses a structured field's explicit JSON spelling with the original error included. */
function json(raw: string, field: string): unknown {
  try { return JSON.parse(raw); }
  catch (error) { throw new Error(`${field}: invalid JSON: ${error instanceof Error ? error.message : String(error)}`); }
}
/** Decodes every character field by its declared record type. */
function characterField(field: string, raw: string): CharacterChanges {
  let value: unknown;
  switch (field) {
    case "name": case "description": case "characterSetting": case "openingStatement": case "otherContentChat":
    case "otherContentVoice": case "advancedCustomPrompt": case "marks": case "chatModelBindingMode": case "memoryBindingMode": value = raw; break;
    case "avatarUri": case "chatModelId": case "ttsConfigId": case "sharedMemoryId": value = optionalIdentifier(raw); break;
    case "attachedTagIds": value = csv(raw); break;
    case "sharedMemoryMounts": case "toolAccessConfig": value = json(raw, field); break;
    case "isDefault": value = bool(raw, field); break;
    case "createdAt": case "updatedAt": value = integer(raw, field); break;
    default: throw new Error(`unsupported character field: ${field}`);
  }
  return parseCharacterChanges({ [field]: value });
}
/** Decodes ordered group members using the original CSV index semantics. */
function groupField(field: string, raw: string): GroupChanges {
  switch (field) {
    case "name": case "description": return parseGroupChanges({ [field]: raw });
    case "createdAt": case "updatedAt": return parseGroupChanges({ [field]: integer(raw, field) });
    case "members": {
      const members: GroupRecord["members"] = [];
      const values = raw.split(",");
      for (let index = 0; index < values.length; index += 1) {
        const characterCardId = values[index].trim();
        if (characterCardId !== "") members.push({ characterCardId, orderIndex: index });
      }
      return parseGroupChanges({ members });
    }
    default: throw new Error(`unsupported group field: ${field}`);
  }
}
/** Decodes prompt tag fields without rewriting tag-type spelling. */
function tagField(field: string, raw: string): TagChanges {
  switch (field) {
    case "name": case "description": case "promptContent": case "tagType": return parseTagChanges({ [field]: raw });
    case "createdAt": case "updatedAt": return parseTagChanges({ [field]: integer(raw, field) });
    default: throw new Error(`unsupported tag field: ${field}`);
  }
}
/** Parses record or patch updates while checking the complete record's identity. */
function changes(args: readonly string[], family: string, decodeField: (field: string, raw: string) => object): { field: string; changes: Record<string, unknown> } {
  arity(args, 4, 4, `operit2 ${family} update <id> <field|--record|--patch> <value>`);
  const field = args[2];
  if (field === "--patch") return { field: "record", changes: parseJsonObject(args[3], `${family} changes`) };
  if (field === "--record") {
    const complete = parseJsonObject(args[3], `${family} record`);
    if (Object.prototype.hasOwnProperty.call(complete, "id") && complete.id !== args[1]) throw new Error(`${family} record id does not match ${args[1]}`);
    const { id: identity, ...mutable } = complete;
    void identity;
    return { field: "record", changes: mutable };
  }
  return { field, changes: record(decodeField(field, args[3]), `${family} changes`) };
}
/** Formats null-valued stored fields as the original human-readable dash. */
function displayOptional(value: string | null): string { return value === null ? "-" : value; }
/** Formats group members without changing their stored order indices. */
function members(group: GroupRecord): string {
  /** Formats the explicit identity and index of one group member. */
  return group.members.map(member => `${member.characterCardId}:${member.orderIndex}`).join(", ");
}
/** Reproduces the original detailed character text from the full stored record. */
function characterText(card: CharacterRecord): string {
  return lines([
    `Character ${card.id}`, `Name: ${card.name}`, `Description: ${card.description}`, `Character setting: ${card.characterSetting}`,
    `Opening statement: ${card.openingStatement}`, `Chat content: ${card.otherContentChat}`, `Voice content: ${card.otherContentVoice}`,
    `Tags: ${card.attachedTagIds.join(", ")}`, `Advanced prompt: ${card.advancedCustomPrompt}`, `Marks: ${card.marks}`,
    `Chat model binding: ${card.chatModelBindingMode}`, `Chat model id: ${displayOptional(card.chatModelId)}`,
    `Shared memory mounts: ${card.sharedMemoryMounts.length}`, `Tool access enabled: ${card.toolAccessConfig.enabled}`,
    `Default: ${card.isDefault}`, `Created at: ${card.createdAt}`, `Updated at: ${card.updatedAt}`,
  ]);
}
/** Reproduces the original detailed group text. */
function groupText(group: GroupRecord): string {
  return lines([`Character group ${group.id}`, `Name: ${group.name}`, `Description: ${group.description}`, `Members: ${members(group)}`, `Created at: ${group.createdAt}`, `Updated at: ${group.updatedAt}`]);
}
/** Reproduces the original detailed prompt tag text. */
function tagText(tag: TagRecord): string {
  return lines([`Prompt tag: ${tag.name}`, `ID: ${tag.id}`, `Type: ${tag.tagType}`, `Description: ${tag.description}`, `Prompt content: ${tag.promptContent}`, `Created at: ${tag.createdAt}`, `Updated at: ${tag.updatedAt}`]);
}
/** Parses the original character root, preserving its structured result shapes. */
export function parseCharacterCommand(args: readonly string[]): CommandInvocation {
  if (wantsHelp(args)) return { kind: "usage", usage: CHARACTER_USAGE };
  switch (args[0]) {
    case "list": {
      arity(args, 1, 1, CHARACTER_USAGE[0]);
      /** Formats the original complete character list and summary lines. */
      return invocation("character.list", {}, cards => result(lines([`Characters: ${cards.length}`, ...cards.map(
        /** Formats one stored character's legacy summary. */
        card => `- ${card.id} | ${card.name} | default: ${card.isDefault} | tags: ${card.attachedTagIds.join(", ")} | ${card.description}`,
      )]), cards));
    }
    case "show": {
      arity(args, 2, 2, CHARACTER_USAGE[1]);
      /** Returns the actual full character record. */
      return invocation("character.get", { id: args[1] }, card => result(characterText(card), card));
    }
    case "create": {
      let values: DomainInput<"character.create">["values"];
      if (args[1] === "--record") {
        arity(args, 3, 3, CHARACTER_USAGE[3]);
        values = parseDomainPayload("character.create", { values: parseJsonObject(args[2], "character record") }).values;
      } else {
        arity(args, 2, 3, CHARACTER_USAGE[2]);
        values = { name: args[1] };
        if (args.length === 3) values.characterSetting = args[2];
      }
      /** Preserves the legacy create JSON instead of returning an editor snapshot. */
      return invocation("character.create", { values }, card => result(lines([`Created character ${card.id}`]), { id: card.id, name: card.name, characterSetting: card.characterSetting }));
    }
    case "update": {
      const patch = changes(args, "character", characterField);
      const payload = parseDomainPayload("character.update", { id: args[1], changes: patch.changes });
      /** Preserves the field-update envelope and complete updated card. */
      return invocation("character.update", payload, card => result(lines([`Updated character ${card.id}`, `Field: ${patch.field}`]), { id: card.id, field: patch.field, character: card }));
    }
    case "delete": {
      arity(args, 2, 2, CHARACTER_USAGE[7]);
      /** Returns the manager's real deletion status. */
      return invocation("character.delete", { id: args[1] }, value => result(lines([`Deleted character ${value.id}`]), value));
    }
    case "set-active": {
      arity(args, 2, 2, CHARACTER_USAGE[8]);
      /** Returns the actual activation result. */
      return invocation("character.setActive", { id: args[1] }, value => result(lines([`Active character: ${value.id}`]), value));
    }
    case "combine": {
      arity(args, 2, 4, CHARACTER_USAGE[9]);
      const payload: Record<string, unknown> = { id: args[1] };
      if (args.length >= 3) payload.promptFunctionType = args[2];
      if (args.length === 4) payload.additionalTagIds = csv(args[3]);
      /** Prints the exact composed prompt without appending a newline. */
      return invocation("character.combine", parseDomainPayload("character.combine", payload), value => result(value.prompt, value));
    }
    case "reset-default": {
      arity(args, 1, 1, CHARACTER_USAGE[10]);
      /** Returns the actual reset operation status. */
      return invocation("character.resetDefault", {}, value => result(lines(["Default character reset"]), value));
    }
    case "export": {
      arity(args, 3, 3, CHARACTER_USAGE[11]);
      const payload = parseDomainPayload("character.export", { id: args[1], format: args[2] });
      /** Prints the domain's original interchange content. */
      return invocation("character.export", payload, value => result(value.content, value));
    }
    case "import": {
      arity(args, 3, 3, CHARACTER_USAGE[12]);
      const payload = parseDomainPayload("character.import", { format: args[1], content: args[2] });
      /** Returns the full character created by the importer. */
      return invocation("character.import", payload, value => result(characterText(value), value));
    }
    case "export-backup": {
      arity(args, 1, 1, CHARACTER_USAGE[13]);
      /** Prints the manager's complete character and prompt-tag backup. */
      return invocation("character.exportBackup", {}, value => result(value.content, value));
    }
    case "import-backup": {
      arity(args, 2, 2, CHARACTER_USAGE[14]);
      /** Returns the manager's actual import counts. */
      return invocation("character.importBackup", { content: args[1] }, value => result(lines([`Imported characters: ${value.new}; updated: ${value.updated}; skipped: ${value.skipped}; total: ${value.total}`]), value));
    }
    default: throw new Error(`unknown character action: ${args[0]}; use character help`);
  }
}

/** Parses the original group root, preserving ordered membership and legacy result envelopes. */
export function parseGroupCommand(args: readonly string[]): CommandInvocation {
  if (wantsHelp(args)) return { kind: "usage", usage: GROUP_USAGE };
  switch (args[0]) {
    case "list": {
      arity(args, 1, 1, GROUP_USAGE[0]);
      /** Formats the original group listing. */
      return invocation("group.list", {}, groups => result(lines([`Character groups: ${groups.length}`, ...groups.map(
        /** Formats one stored group's original summary. */
        group => `- ${group.id} | ${group.name} | members: ${members(group)} | ${group.description}`,
      )]), groups));
    }
    case "show": {
      arity(args, 2, 2, GROUP_USAGE[1]);
      /** Returns the full stored group. */
      return invocation("group.get", { id: args[1] }, group => result(groupText(group), group));
    }
    case "create": {
      let values: DomainInput<"group.create">["values"];
      if (args[1] === "--record") {
        arity(args, 3, 3, GROUP_USAGE[3]);
        values = parseDomainPayload("group.create", { values: parseJsonObject(args[2], "group record") }).values;
      } else {
        arity(args, 2, 3, GROUP_USAGE[2]);
        values = { name: args[1] };
        if (args.length === 3) values.description = args[2];
      }
      /** Preserves the original group-create JSON. */
      return invocation("group.create", { values }, group => result(lines([`Created character group ${group.id}`]), { id: group.id, name: group.name, description: group.description }));
    }
    case "update": {
      const patch = changes(args, "group", groupField);
      const payload = parseDomainPayload("group.update", { id: args[1], changes: patch.changes });
      /** Preserves the original group field-update JSON. */
      return invocation("group.update", payload, group => result(lines([`Updated character group ${group.id}`, `Field: ${patch.field}`]), { id: group.id, field: patch.field, group }));
    }
    case "delete": {
      arity(args, 2, 2, GROUP_USAGE[7]);
      /** Returns the real group deletion status. */
      return invocation("group.delete", { id: args[1] }, value => result(lines([`Deleted character group ${value.id}`]), value));
    }
    case "set-active": {
      arity(args, 2, 2, GROUP_USAGE[8]);
      /** Returns the actual activated group. */
      return invocation("group.setActive", { id: args[1] }, value => result(lines([`Active character group: ${value.id}`]), value));
    }
    case "duplicate": {
      arity(args, 2, 3, GROUP_USAGE[9]);
      const payload: DomainInput<"group.duplicate"> = { id: args[1] };
      const newName = args.length === 3 ? args[2] : null;
      if (newName !== null) payload.newName = newName;
      /** Preserves the original duplication envelope and optional name representation. */
      return invocation("group.duplicate", payload, group => result(lines([`Duplicated character group ${payload.id} -> ${group.id}`]), { sourceId: payload.id, newId: group.id, newName }));
    }
    case "export": {
      arity(args, 2, 2, GROUP_USAGE[10]);
      /** Prints native group interchange content. */
      return invocation("group.export", { id: args[1] }, value => result(value.content, value));
    }
    case "import": {
      arity(args, 2, 2, GROUP_USAGE[11]);
      /** Returns the full imported group. */
      return invocation("group.import", { content: args[1] }, value => result(groupText(value), value));
    }
    case "export-backup": {
      arity(args, 1, 1, GROUP_USAGE[12]);
      /** Prints the manager's group backup content. */
      return invocation("group.exportBackup", {}, value => result(value.content, value));
    }
    case "import-backup": {
      arity(args, 2, 2, GROUP_USAGE[13]);
      /** Returns the manager's actual group import counts. */
      return invocation("group.importBackup", { content: args[1] }, value => result(lines([`Imported groups: ${value.new}; updated: ${value.updated}; skipped: ${value.skipped}; total: ${value.total}`]), value));
    }
    default: throw new Error(`unknown group action: ${args[0]}; use group help`);
  }
}

/** Parses prompt tag commands without converting them into a character snapshot call. */
export function parseTagCommand(args: readonly string[]): CommandInvocation {
  if (wantsHelp(args)) return { kind: "usage", usage: TAG_USAGE };
  switch (args[0]) {
    case "list": {
      arity(args, 1, 1, TAG_USAGE[0]);
      /** Formats the original tag directory. */
      return invocation("tag.list", {}, tags => result(lines([`Prompt tags: ${tags.length}`, ...tags.map(
        /** Formats one prompt tag's original summary. */
        tag => `- ${tag.id} (${tag.name}) [${tag.tagType}] ${tag.description}`,
      )]), tags));
    }
    case "show": {
      arity(args, 2, 2, TAG_USAGE[1]);
      /** Returns the complete prompt tag. */
      return invocation("tag.get", { id: args[1] }, tag => result(tagText(tag), tag));
    }
    case "create": {
      let values: DomainInput<"tag.create">["values"];
      if (args[1] === "--record") {
        arity(args, 3, 3, TAG_USAGE[3]);
        values = parseDomainPayload("tag.create", { values: parseJsonObject(args[2], "tag record") }).values;
      } else {
        arity(args, 2, 5, TAG_USAGE[2]);
        const fields: Record<string, unknown> = { name: args[1] };
        if (args.length >= 3) fields.promptContent = args[2];
        if (args.length >= 4) fields.description = args[3];
        if (args.length === 5) fields.tagType = args[4];
        values = parseDomainPayload("tag.create", { values: fields }).values;
      }
      /** Preserves the original tag-create status envelope. */
      return invocation("tag.create", { values }, tag => result(lines([`Prompt tag created: ${tag.id}`]), { id: tag.id, created: true }));
    }
    case "update": {
      const patch = changes(args, "tag", tagField);
      const payload = parseDomainPayload("tag.update", { id: args[1], changes: patch.changes });
      /** Preserves the original tag-update status envelope. */
      return invocation("tag.update", payload, tag => result(lines([`Prompt tag updated: ${tag.id}`]), { id: tag.id, updated: true }));
    }
    case "delete": {
      arity(args, 2, 2, TAG_USAGE[7]);
      /** Returns the actual prompt tag deletion status. */
      return invocation("tag.delete", { id: args[1] }, value => result(lines([`Prompt tag deleted: ${value.id}`]), value));
    }
    default: throw new Error(`unknown tag action: ${args[0]}; use tag help`);
  }
}

/** Parses the active prompt tree while keeping activation inside the shared service. */
export function parseActivePromptCommand(args: readonly string[]): CommandInvocation {
  if (wantsHelp(args)) return { kind: "usage", usage: ACTIVE_PROMPT_USAGE };
  switch (args[0]) {
    case "show": {
      arity(args, 1, 1, ACTIVE_PROMPT_USAGE[0]);
      /** Formats the externally tagged active prompt without changing its JSON shape. */
      return invocation("activePrompt.get", {}, value => result(lines([`Active prompt: ${"CharacterCard" in value ? `character card ${value.CharacterCard.id}` : `character group ${value.CharacterGroup.id}`}`]), value));
    }
    case "set-card": {
      arity(args, 2, 2, ACTIVE_PROMPT_USAGE[1]);
      /** Returns the active character card selected by the service. */
      return invocation("activePrompt.setCard", { id: args[1] }, value => result(lines([`Active character card: ${value.id}`]), value));
    }
    case "set-group": {
      arity(args, 2, 2, ACTIVE_PROMPT_USAGE[2]);
      /** Returns the active character group selected by the service. */
      return invocation("activePrompt.setGroup", { id: args[1] }, value => result(lines([`Active character group: ${value.id}`]), value));
    }
    case "activate-for-chat": {
      arity(args, 1, 3, ACTIVE_PROMPT_USAGE[3]);
      const characterCardName = args.length >= 2 ? optionalIdentifier(args[1]) : null;
      const characterGroupId = args.length === 3 ? optionalIdentifier(args[2]) : null;
      /** Returns the actual chat-binding update result. */
      return invocation("activePrompt.activateForChat", { characterCardName, characterGroupId }, value => result(lines(["Active prompt updated for chat binding"]), value));
    }
    case "resolved-card": {
      arity(args, 1, 1, ACTIVE_PROMPT_USAGE[4]);
      /** Returns the manager's sending-card resolution. */
      return invocation("activePrompt.resolvedCard", {}, value => result(lines([`Resolved character card: ${value.id}`]), value));
    }
    default: throw new Error(`unknown active-prompt action: ${args[0]}; use active-prompt help`);
  }
}

/** Formats the original memory tag-name summary. */
function memoryTags(item: MemoryRecord): string {
  /** Selects each stored tag name without changing the record. */
  return item.tags.map(tag => tag.name).join(", ");
}
/** Formats one legacy memory listing row. */
function memoryRow(item: MemoryRecord): string { return `- ${item.id} | ${item.title} | folder: ${displayOptional(item.folderPath)} | tags: ${memoryTags(item)}`; }
/** Formats the original detailed memory record output. */
function memoryText(item: MemoryRecord): string {
  return lines([`Memory item ${item.id}`, `UUID: ${item.uuid}`, `Title: ${item.title}`, `Content: ${item.content}`, `Content type: ${item.contentType}`, `Source: ${item.source}`, `Credibility: ${item.credibility}`, `Importance: ${item.importance}`, `Folder: ${displayOptional(item.folderPath)}`, `Created at: ${item.createdAt}`, `Updated at: ${item.updatedAt}`, `Last accessed at: ${item.lastAccessedAt}`, `Tags: ${memoryTags(item)}`]);
}
/** Decodes a memory field using its declared record type. */
function memoryField(field: string, raw: string): Record<string, unknown> {
  switch (field) {
    case "uuid": case "title": case "content": case "contentType": case "source": case "folderPath": return { [field]: raw };
    case "documentPath": case "chunkIndexFilePath": return { [field]: optionalIdentifier(raw) };
    case "credibility": case "importance": return { [field]: number(raw, field) };
    case "isDocumentNode": return { [field]: bool(raw, field) };
    case "createdAt": case "updatedAt": case "lastAccessedAt": return { [field]: integer(raw, field) };
    case "tags": return { tags: csv(raw) };
    case "properties": return { properties: json(raw, field) };
    default: throw new Error(`unsupported memory field: ${field}`);
  }
}
/** Converts the complete serde memory format into the named-tag mutation input format. */
function memoryRecord(raw: string): Record<string, unknown> {
  const complete = parseJsonObject(raw, "memory record");
  if (Object.prototype.hasOwnProperty.call(complete, "tags")) {
    if (!Array.isArray(complete.tags)) throw new Error("memory record.tags must be an array of stored tag records");
    /** Validates each complete stored tag record before converting it to the mutation tag name. */
    complete.tags = complete.tags.map((value, index) => {
      const tag = record(value, `memory record.tags[${index}]`);
      if (Object.keys(tag).length !== 2 || typeof tag.name !== "string") throw new Error(`memory record.tags[${index}] must have id and name`);
      parseMemoryIdentifier(tag.id, `memory record.tags[${index}].id`, false);
      return tag.name;
    });
  }
  return complete;
}
/** Parses the complete owner-scoped memory item subtree. */
function parseMemoryItems(ownerKey: string, args: readonly string[]): CommandInvocation {
  if (wantsHelp(args)) return { kind: "usage", usage: ITEM_USAGE };
  switch (args[0]) {
    case "list": {
      arity(args, 1, 1, ITEM_USAGE[0]);
      /** Preserves the original owner-plus-items list envelope. */
      return invocation("memory.list", { ownerKey }, value => result(lines([`Memory items: ${value.items.length}`, ...value.items.map(memoryRow)]), value));
    }
    case "search": {
      arity(args, 2, 2, ITEM_USAGE[1]);
      /** Preserves the original search query and owner envelope. */
      return invocation("memory.search", { ownerKey, query: args[1] }, value => result(lines([`Memory search results: ${value.items.length}`, `Query: ${value.query}`, ...value.items.map(memoryRow)]), value));
    }
    case "show": {
      arity(args, 2, 2, ITEM_USAGE[2]);
      /** Preserves title lookup and the original owner-plus-item envelope. */
      return invocation("memory.get", { ownerKey, title: args[1] }, value => result(memoryText(value.item), value));
    }
    case "create": {
      let values: DomainInput<"memory.create">["values"];
      if (args[1] === "--record") {
        arity(args, 3, 3, ITEM_USAGE[4]);
        values = parseDomainPayload("memory.create", { ownerKey, values: memoryRecord(args[2]) }).values;
      } else {
        arity(args, 3, 5, ITEM_USAGE[3]);
        values = { title: args[1], content: args[2], contentType: "text", source: "cli" };
        if (args.length >= 4) values.folderPath = args[3];
        if (args.length === 5) values.tags = csv(args[4]);
      }
      /** Preserves the original owner, created item, and status envelope. */
      return invocation("memory.create", { ownerKey, values }, value => result(lines([`Created memory item ${value.item.id}`, `Title: ${value.item.title}`]), value));
    }
    case "update": {
      arity(args, 4, 4, ITEM_USAGE[5]);
      const fields = args[2] === "--patch" ? parseJsonObject(args[3], "memory changes") : args[2] === "--record" ? memoryRecord(args[3]) : memoryField(args[2], args[3]);
      const { id: expectedId, ...mutable } = fields;
      const input: Record<string, unknown> = { ownerKey, originalTitle: args[1], changes: mutable };
      if (Object.prototype.hasOwnProperty.call(fields, "id")) {
        if (args[2] !== "--record") throw new Error("memory patch cannot change the immutable id");
        input.expectedId = expectedId;
      }
      const payload = parseDomainPayload("memory.update", input);
      /** Returns the updated canonical record instead of a refreshed UI graph. */
      return invocation("memory.update", payload, value => result(memoryText(value.item), value));
    }
    case "delete": {
      arity(args, 2, 2, ITEM_USAGE[8]);
      /** Deletes by the exact plugin-owned string identity without converting it to a number. */
      return invocation("memory.delete", { ownerKey, id: parseMemoryIdentifier(args[1], "memory id") }, value => result(lines([`Deleted memory item ${value.id}: ${value.deleted}`]), value));
    }
    case "move": {
      arity(args, 3, 3, ITEM_USAGE[9]);
      const ids: string[] = [];
      for (const part of args[1].split(",")) ids.push(parseMemoryIdentifier(part.trim(), "memory id"));
      /** Preserves the original moved count, CSV ID list, and folder key. */
      return invocation("memory.move", { ownerKey, ids, folderPath: args[2] }, value => result(lines([`Moved memory items: ${value.moved}`, `Folder: ${value.folder}`]), value));
    }
    default: throw new Error(`unknown memory item action: ${args[0]}; use memory <owner> item help`);
  }
}
/** Parses the owner's USER.md subtree without interpreting content as commands. */
function parseUser(ownerKey: string, args: readonly string[]): CommandInvocation {
  if (wantsHelp(args)) return { kind: "usage", usage: USER_USAGE };
  switch (args[0]) {
    case "show": {
      arity(args, 1, 1, USER_USAGE[0]);
      /** Prints the original markdown without appending a newline. */
      return invocation("memory.user.read", { ownerKey }, value => result(value.content, value));
    }
    case "write": {
      arity(args, 2, 2, USER_USAGE[1]);
      /** Returns the repository's write status and UTF-8 content length. */
      return invocation("memory.user.write", { ownerKey, content: args[1] }, value => result(lines([`Updated ${ownerKey}/USER.md`]), value));
    }
    case "path": {
      arity(args, 1, 1, USER_USAGE[2]);
      /** Returns the path supplied by the cross-platform host repository. */
      return invocation("memory.user.path", { ownerKey }, value => result(lines([`USER.md path: ${value.path}`]), value));
    }
    default: throw new Error(`unknown memory user action: ${args[0]}; use memory <owner> user help`);
  }
}
/** Parses owner-scoped relationships as domain mutations, never memory tool calls. */
function parseLinks(ownerKey: string, args: readonly string[]): CommandInvocation {
  if (wantsHelp(args)) return { kind: "usage", usage: LINK_USAGE };
  switch (args[0]) {
    case "create": {
      arity(args, 6, 6, LINK_USAGE[0]);
      /** Returns the link created by the owner's repository. */
      return invocation("memory.link.create", { ownerKey, sourceTitle: args[1], targetTitle: args[2], linkType: args[3], weight: number(args[4], "weight"), description: args[5] }, value => result(lines([`Created memory link ${value.link.id}`]), value));
    }
    case "update": {
      arity(args, 4, 4, LINK_USAGE[1]);
      const linkId = parseMemoryIdentifier(args[1], "link id");
      let patch: Record<string, unknown>;
      if (args[2] === "--patch") patch = parseJsonObject(args[3], "link changes");
      else if (args[2] === "weight") patch = { weight: number(args[3], "weight") };
      else if (args[2] === "linkType" || args[2] === "description") patch = { [args[2]]: args[3] };
      else throw new Error(`unsupported link field: ${args[2]}`);
      const payload = parseDomainPayload("memory.link.update", { ownerKey, linkId, changes: patch });
      /** Returns the canonical updated link. */
      return invocation("memory.link.update", payload, value => result(lines([`Updated memory link ${value.link.id}`]), value));
    }
    case "delete": {
      arity(args, 2, 2, LINK_USAGE[3]);
      /** Returns the repository's relationship deletion status. */
      return invocation("memory.link.delete", { ownerKey, linkId: parseMemoryIdentifier(args[1], "link id") }, value => result(lines([`Deleted memory link ${value.linkId}: ${value.deleted}`]), value));
    }
    default: throw new Error(`unknown memory link action: ${args[0]}; use memory <owner> link help`);
  }
}
/** Parses an owner subtree after namespace and identifier validation. */
function parseOwner(ownerKey: string, args: readonly string[], usage: readonly string[]): CommandInvocation {
  parseDomainPayload("memory.graph", { ownerKey });
  if (wantsHelp(args)) return { kind: "usage", usage };
  switch (args[0]) {
    case "user": return parseUser(ownerKey, args.slice(1));
    case "item": return parseMemoryItems(ownerKey, args.slice(1));
    case "link": return parseLinks(ownerKey, args.slice(1));
    case "graph": {
      arity(args, 1, 1, "operit2 memory <owner> graph");
      /** Preserves the owner-plus-graph envelope used by the original CLI. */
      return invocation("memory.graph", { ownerKey }, value => result(lines([`Memory graph for ${ownerKey}`, `Nodes: ${value.graph.nodes.length}`, `Edges: ${value.graph.edges.length}`]), value));
    }
    case "export": {
      arity(args, 1, 1, "operit2 memory <owner> export");
      /** Prints the repository's real backup JSON. */
      return invocation("memory.export", { ownerKey }, value => result(value.content, value));
    }
    case "import": {
      arity(args, 3, 3, "operit2 memory <owner> import <SKIP|UPDATE|CREATE_NEW> <json-content>");
      const payload = parseDomainPayload("memory.import", { ownerKey, strategy: args[1], content: args[2] });
      /** Returns the repository's real memory and link import counts. */
      return invocation("memory.import", payload, value => result(lines([`Imported memory backup for ${ownerKey}`]), value));
    }
    case "settings": {
      const help = ["operit2 memory <owner> settings show", "operit2 memory <owner> settings write <settings-json>"];
      const sub = args.slice(1);
      if (wantsHelp(sub)) return { kind: "usage", usage: help };
      if (sub[0] === "show") {
        arity(sub, 1, 1, help[0]);
        /** Returns the owner's actual memory settings. */
        return invocation("memory.settings.read", { ownerKey }, value => result(lines([JSON.stringify(value, null, 2)]), value));
      }
      if (sub[0] === "write") {
        arity(sub, 2, 2, help[1]);
        const payload = parseDomainPayload("memory.settings.write", { ownerKey, settings: parseJsonObject(sub[1], "memory settings") });
        /** Returns the settings saved by the owner-scoped manager. */
        return invocation("memory.settings.write", payload, value => result(lines([`Updated memory settings for ${ownerKey}`]), value));
      }
      throw new Error(`unknown memory settings action: ${sub[0]}`);
    }
    case "search-config": {
      const help = ["operit2 memory <owner> search-config show", "operit2 memory <owner> search-config write <config-json>"];
      const sub = args.slice(1);
      if (wantsHelp(sub)) return { kind: "usage", usage: help };
      if (sub[0] === "show") {
        arity(sub, 1, 1, help[0]);
        /** Returns the owner's actual search configuration. */
        return invocation("memory.searchConfig.read", { ownerKey }, value => result(lines([JSON.stringify(value, null, 2)]), value));
      }
      if (sub[0] === "write") {
        arity(sub, 2, 2, help[1]);
        const payload = parseDomainPayload("memory.searchConfig.write", { ownerKey, config: parseJsonObject(sub[1], "memory search config") });
        /** Returns the search configuration saved by the repository. */
        return invocation("memory.searchConfig.write", payload, value => result(lines([`Updated memory search config for ${ownerKey}`]), value));
      }
      throw new Error(`unknown memory search-config action: ${sub[0]}`);
    }
    default: throw new Error(`unknown memory owner action: ${args[0]}; use memory <owner> help`);
  }
}
/** Parses the shared namespace's library management commands before owner-scoped operations. */
function parseShared(args: readonly string[]): CommandInvocation {
  if (wantsHelp(args)) return { kind: "usage", usage: SHARED_MEMORY_USAGE };
  switch (args[0]) {
    case "list": {
      arity(args, 1, 1, "operit2 memory shared list");
      /** Preserves full shared store records and their original summary format. */
      return invocation("memory.shared.list", {}, stores => result(lines([`Shared memory stores: ${stores.length}`, ...stores.map(
        /** Formats one canonical shared store. */
        store => `- ${store.id} | ${store.name} | created: ${store.createdAt} | updated: ${store.updatedAt}`,
      )]), stores));
    }
    case "create": {
      arity(args, 2, 2, "operit2 memory shared create <name>");
      /** Returns the shared store created by the manager. */
      return invocation("memory.shared.create", { name: args[1] }, store => result(lines([`Created shared memory store ${store.id}`, `Name: ${store.name}`]), store));
    }
    case "rename": {
      arity(args, 3, 3, "operit2 memory shared rename <shared-id> <name>");
      /** Returns the actual renamed shared store. */
      return invocation("memory.shared.rename", { id: args[1], name: args[2] }, store => result(lines([`Renamed shared memory store ${store.id}`, `Name: ${store.name}`]), store));
    }
    case "delete": {
      arity(args, 2, 2, "operit2 memory shared delete <shared-id>");
      /** Preserves both deletion status and character-mount cleanup counts. */
      return invocation("memory.shared.delete", { id: args[1] }, value => result(lines([`Deleted shared memory store ${value.sharedId}: ${value.deleted}`, `Characters cleaned: ${value.cleanedCharacters}`]), value));
    }
    default: return parseOwner(`shared:${args[0]}`, args.slice(1), SHARED_MEMORY_USAGE);
  }
}
/** Parses explicit named mount permissions and rejects duplicate or unknown options. */
function parseMount(args: readonly string[]): CommandInvocation {
  if (wantsHelp(args)) return { kind: "usage", usage: MOUNT_USAGE };
  arity(args, 6, 6, MOUNT_USAGE[0]);
  const flags = new Map<string, boolean>();
  for (let index = 2; index < args.length; index += 2) {
    const flag = args[index];
    if (flag !== "--read" && flag !== "--write") throw new Error(`unknown memory mount option: ${flag}`);
    if (flags.has(flag)) throw new Error(`duplicate memory mount option: ${flag}`);
    flags.set(flag, bool(args[index + 1], flag));
  }
  if (!flags.has("--read") || !flags.has("--write")) throw new Error("memory mount requires --read and --write");
  const readable = flags.get("--read") as boolean;
  const writable = flags.get("--write") as boolean;
  /** Returns the explicit persisted shared-memory mount. */
  return invocation("memory.mount", { characterId: args[0], sharedId: args[1], readable, writable }, value => result(lines([`Mounted shared memory ${value.sharedId} on ${value.characterId}`, `Readable: ${value.mount.readable}`, `Writable: ${value.mount.writable}`]), value));
}
/** Parses the complete original memory tree and typed owner-scoped extensions. */
export function parseMemoryCommand(args: readonly string[]): CommandInvocation {
  if (wantsHelp(args)) return { kind: "usage", usage: MEMORY_USAGE };
  switch (args[0]) {
    case "character": {
      const sub = args.slice(1);
      if (wantsHelp(sub)) return { kind: "usage", usage: CHARACTER_MEMORY_USAGE };
      return parseOwner(`character:${sub[0]}`, sub.slice(1), CHARACTER_MEMORY_USAGE);
    }
    case "shared": return parseShared(args.slice(1));
    case "mount": return parseMount(args.slice(1));
    case "unmount": {
      const sub = args.slice(1);
      if (wantsHelp(sub)) return { kind: "usage", usage: UNMOUNT_USAGE };
      arity(sub, 2, 2, UNMOUNT_USAGE[0]);
      /** Preserves the original unmount status semantics. */
      return invocation("memory.unmount", { characterId: sub[0], sharedId: sub[1] }, value => result(lines([`Unmounted shared memory ${value.sharedId} from ${value.characterId}: ${value.unmounted}`]), value));
    }
    case "resolve": {
      arity(args, 2, 2, "operit2 memory resolve <character-id>");
      /** Returns the actual bound owner rather than assuming character memory. */
      return invocation("memory.resolveOwner", { characterId: args[1] }, value => result(lines([JSON.stringify(value)]), value));
    }
    default: throw new Error(`unknown memory namespace: ${args[0]}; use memory help`);
  }
}
/** Reports a failed operation explicitly in both human-readable and JSON command modes. */
function failure(family: CommandFamily, code: string, error: unknown): ToolPkg.CoreCommandResult {
  const message = error instanceof Error ? error.message : String(error);
  return { stderr: `${message}\n`, json: { ok: false, command: family, error: { code, message } } };
}
/** Validates the host event, parses all arguments, and invokes one shared domain operation. */
async function execute(family: CommandFamily, parse: (args: readonly string[]) => CommandInvocation, event: ToolPkg.CoreCommandHookEvent): Promise<ToolPkg.CoreCommandResult> {
  let parsed: CommandInvocation;
  try {
    if (event.eventPayload.commandName !== family) throw new Error(`command event ${event.eventPayload.commandName} does not match ${family}`);
    if (!Array.isArray(event.eventPayload.args)) throw new Error("command args must be a string array");
    for (const value of event.eventPayload.args) if (typeof value !== "string") throw new Error("command args must contain only strings");
    parsed = parse(event.eventPayload.args);
  } catch (error) { return failure(family, "invalid_arguments", error); }
  if (parsed.kind === "usage") return result(lines(parsed.usage), { usage: parsed.usage });
  try { return await parsed.execute(); }
  catch (error) { return failure(family, "domain_operation_failed", error); }
}
/** Executes the plugin-owned character root. */
export async function onCharacterCommand(event: ToolPkg.CoreCommandHookEvent): Promise<ToolPkg.CoreCommandResult> { return execute("character", parseCharacterCommand, event); }
/** Executes the plugin-owned group root. */
export async function onGroupCommand(event: ToolPkg.CoreCommandHookEvent): Promise<ToolPkg.CoreCommandResult> { return execute("group", parseGroupCommand, event); }
/** Executes the plugin-owned prompt tag root. */
export async function onTagCommand(event: ToolPkg.CoreCommandHookEvent): Promise<ToolPkg.CoreCommandResult> { return execute("tag", parseTagCommand, event); }
/** Executes the plugin-owned active prompt root. */
export async function onActivePromptCommand(event: ToolPkg.CoreCommandHookEvent): Promise<ToolPkg.CoreCommandResult> { return execute("active-prompt", parseActivePromptCommand, event); }
/** Executes the plugin-owned memory root. */
export async function onMemoryCommand(event: ToolPkg.CoreCommandHookEvent): Promise<ToolPkg.CoreCommandResult> { return execute("memory", parseMemoryCommand, event); }
/** Registers the original command roots, transferred out of the host's built-in dispatcher. */
export function registerDomainCommands(): void {
  ToolPkg.registerCoreCommand({ id: "character", name: "character", title: { zh: "角色卡", en: "Characters" }, description: { zh: "管理角色卡、完整绑定、提示词和导入导出。", en: "Manage characters, bindings, prompts, and interchange." }, usage: "/character help", function: onCharacterCommand });
  ToolPkg.registerCoreCommand({ id: "group", name: "group", title: { zh: "角色群组", en: "Character groups" }, description: { zh: "管理角色群组、成员顺序、激活和导入导出。", en: "Manage character groups, ordered members, activation, and interchange." }, usage: "/group help", function: onGroupCommand });
  ToolPkg.registerCoreCommand({ id: "tag", name: "tag", title: { zh: "提示词标签", en: "Prompt tags" }, description: { zh: "读取、创建、编辑和删除提示词标签。", en: "Read, create, update, and delete prompt tags." }, usage: "/tag help", function: onTagCommand });
  ToolPkg.registerCoreCommand({ id: "active-prompt", name: "active-prompt", title: { zh: "当前提示词", en: "Active prompt" }, description: { zh: "查询和设置当前角色、群组以及聊天绑定。", en: "Read and select the active character, group, and chat binding." }, usage: "/active-prompt help", function: onActivePromptCommand });
  ToolPkg.registerCoreCommand({ id: "memory", name: "memory", title: { zh: "角色记忆", en: "Character memory" }, description: { zh: "管理角色和共享记忆、挂载、USER.md、条目、关系、图谱及设置。", en: "Manage character and shared memory, mounts, USER.md, items, links, graphs, and settings." }, usage: "/memory help", function: onMemoryCommand });
}

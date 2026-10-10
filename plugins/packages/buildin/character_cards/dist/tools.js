/* METADATA
{
  "name": "character_memory",
  "display_name": {
    "zh": "角色记忆",
    "en": "Character Memory"
  },
  "description": {
    "zh": "在角色插件自己的文件目录中读写记忆、文档和关系。",
    "en": "Read and edit memories, documents and relationships in the character plugin files."
  },
  "tools": [
    {
      "name": "get_memory_owner_key",
      "description": "Resolve the actual memory owner bound to the execution participant.",
      "parameters": [
        {
          "name": "caller_card_id",
          "type": "string",
          "required": false,
          "description": "Explicit calling participant identity; must match an authenticated execution participant when one exists."
        }
      ]
    },
    {
      "name": "query_memory",
      "description": "Search complete memories and document chunks with reusable owner-scoped pagination.",
      "parameters": [
        {
          "name": "query",
          "type": "string",
          "required": true,
          "description": "Search expression or * to list all."
        },
        {
          "name": "target_owner_key",
          "type": "string",
          "required": false,
          "description": "Existing memory owner key: character:<id> or shared:<id>; omission uses the actual execution participant binding."
        },
        {
          "name": "caller_card_id",
          "type": "string",
          "required": false,
          "description": "Explicit calling participant identity; must match an authenticated execution participant when one exists."
        },
        {
          "name": "folder_path",
          "type": "string",
          "required": false,
          "description": "Folder search scope."
        },
        {
          "name": "start_time",
          "type": "string",
          "required": false,
          "description": "Local YYYY-MM-DD or YYYY-MM-DD HH:mm inclusive boundary."
        },
        {
          "name": "end_time",
          "type": "string",
          "required": false,
          "description": "Local YYYY-MM-DD or YYYY-MM-DD HH:mm inclusive boundary."
        },
        {
          "name": "snapshot_id",
          "type": "string",
          "required": false,
          "description": "Owner-scoped pagination ID returned by a previous query."
        },
        {
          "name": "threshold",
          "type": "number",
          "required": false,
          "description": "Nonnegative relevance threshold, default 0."
        },
        {
          "name": "limit",
          "type": "integer",
          "required": false,
          "description": "Maximum records; default 20 for search and all for *."
        }
      ]
    },
    {
      "name": "get_memory_by_title",
      "description": "Read an exact memory title or real document chunks by query or one-based index/range.",
      "parameters": [
        {
          "name": "target_owner_key",
          "type": "string",
          "required": false,
          "description": "Existing memory owner key: character:<id> or shared:<id>; omission uses the actual execution participant binding."
        },
        {
          "name": "caller_card_id",
          "type": "string",
          "required": false,
          "description": "Explicit calling participant identity; must match an authenticated execution participant when one exists."
        },
        {
          "name": "title",
          "type": "string",
          "required": true,
          "description": "Exact memory title."
        },
        {
          "name": "query",
          "type": "string",
          "required": false,
          "description": "Document search expression."
        },
        {
          "name": "chunk_index",
          "type": "integer",
          "required": false,
          "description": "One-based document chunk index."
        },
        {
          "name": "chunk_range",
          "type": "string",
          "required": false,
          "description": "One-based inclusive start-end chunk range."
        },
        {
          "name": "limit",
          "type": "integer",
          "required": false,
          "description": "Document search result limit, default 20."
        }
      ]
    },
    {
      "name": "create_memory",
      "description": "Create a full memory record with plugin-owned identity and persistence.",
      "parameters": [
        {
          "name": "target_owner_key",
          "type": "string",
          "required": false,
          "description": "Existing memory owner key: character:<id> or shared:<id>; omission uses the actual execution participant binding."
        },
        {
          "name": "caller_card_id",
          "type": "string",
          "required": false,
          "description": "Explicit calling participant identity; must match an authenticated execution participant when one exists."
        },
        {
          "name": "title",
          "type": "string",
          "required": true,
          "description": "Exact memory title."
        },
        {
          "name": "content",
          "type": "string",
          "required": true,
          "description": "Complete memory content."
        },
        {
          "name": "content_type",
          "type": "string",
          "required": false,
          "description": "MIME type, default text/plain."
        },
        {
          "name": "source",
          "type": "string",
          "required": false,
          "description": "Source, default ai_created."
        },
        {
          "name": "folder_path",
          "type": "string",
          "required": false,
          "description": "Destination folder."
        },
        {
          "name": "tags",
          "type": "string",
          "required": false,
          "description": "Comma-delimited tag names."
        }
      ]
    },
    {
      "name": "update_memory",
      "description": "Patch an exact memory title without replacing unspecified fields.",
      "parameters": [
        {
          "name": "target_owner_key",
          "type": "string",
          "required": false,
          "description": "Existing memory owner key: character:<id> or shared:<id>; omission uses the actual execution participant binding."
        },
        {
          "name": "caller_card_id",
          "type": "string",
          "required": false,
          "description": "Explicit calling participant identity; must match an authenticated execution participant when one exists."
        },
        {
          "name": "old_title",
          "type": "string",
          "required": true,
          "description": "Exact existing title."
        },
        {
          "name": "new_title",
          "type": "string",
          "required": false,
          "description": "New title."
        },
        {
          "name": "content",
          "type": "string",
          "required": false,
          "description": "New content."
        },
        {
          "name": "content_type",
          "type": "string",
          "required": false,
          "description": "New MIME type."
        },
        {
          "name": "source",
          "type": "string",
          "required": false,
          "description": "New source."
        },
        {
          "name": "credibility",
          "type": "number",
          "required": false,
          "description": "Credibility between 0 and 1."
        },
        {
          "name": "importance",
          "type": "number",
          "required": false,
          "description": "Importance between 0 and 1."
        },
        {
          "name": "folder_path",
          "type": "string",
          "required": false,
          "description": "New folder."
        },
        {
          "name": "tags",
          "type": "string",
          "required": false,
          "description": "Comma-delimited tag names."
        }
      ]
    },
    {
      "name": "delete_memory",
      "description": "Delete the exact memory selected by title.",
      "parameters": [
        {
          "name": "target_owner_key",
          "type": "string",
          "required": false,
          "description": "Existing memory owner key: character:<id> or shared:<id>; omission uses the actual execution participant binding."
        },
        {
          "name": "caller_card_id",
          "type": "string",
          "required": false,
          "description": "Explicit calling participant identity; must match an authenticated execution participant when one exists."
        },
        {
          "name": "title",
          "type": "string",
          "required": true,
          "description": "Exact memory title."
        }
      ]
    },
    {
      "name": "move_memory",
      "description": "Move memories selected by titles and/or a source-folder intersection.",
      "parameters": [
        {
          "name": "target_owner_key",
          "type": "string",
          "required": false,
          "description": "Existing memory owner key: character:<id> or shared:<id>; omission uses the actual execution participant binding."
        },
        {
          "name": "caller_card_id",
          "type": "string",
          "required": false,
          "description": "Explicit calling participant identity; must match an authenticated execution participant when one exists."
        },
        {
          "name": "titles",
          "type": "string",
          "required": false,
          "description": "Titles delimited by comma, newline or pipe."
        },
        {
          "name": "source_folder_path",
          "type": "string",
          "required": false,
          "description": "Exact source folder, including the empty root spelling."
        },
        {
          "name": "target_folder_path",
          "type": "string",
          "required": true,
          "description": "Target folder, including the empty root spelling."
        }
      ]
    },
    {
      "name": "update_user_preferences",
      "description": "Write the full owner USER.md using the same main-runtime file service.",
      "parameters": [
        {
          "name": "target_owner_key",
          "type": "string",
          "required": false,
          "description": "Existing memory owner key: character:<id> or shared:<id>; omission uses the actual execution participant binding."
        },
        {
          "name": "caller_card_id",
          "type": "string",
          "required": false,
          "description": "Explicit calling participant identity; must match an authenticated execution participant when one exists."
        },
        {
          "name": "content",
          "type": "string",
          "required": true,
          "description": "Complete USER.md content."
        }
      ]
    },
    {
      "name": "link_memories",
      "description": "Create a directed relationship between two exact memory titles.",
      "parameters": [
        {
          "name": "target_owner_key",
          "type": "string",
          "required": false,
          "description": "Existing memory owner key: character:<id> or shared:<id>; omission uses the actual execution participant binding."
        },
        {
          "name": "caller_card_id",
          "type": "string",
          "required": false,
          "description": "Explicit calling participant identity; must match an authenticated execution participant when one exists."
        },
        {
          "name": "source_title",
          "type": "string",
          "required": true,
          "description": "Exact source title."
        },
        {
          "name": "target_title",
          "type": "string",
          "required": true,
          "description": "Exact target title."
        },
        {
          "name": "link_type",
          "type": "string",
          "required": false,
          "description": "Relationship type, default related."
        },
        {
          "name": "weight",
          "type": "number",
          "required": false,
          "description": "Relationship weight between 0 and 1; default 0.5."
        },
        {
          "name": "description",
          "type": "string",
          "required": false,
          "description": "Complete relationship description."
        }
      ]
    },
    {
      "name": "query_memory_links",
      "description": "Query complete directed relationships, including lossless IDs and descriptions.",
      "parameters": [
        {
          "name": "target_owner_key",
          "type": "string",
          "required": false,
          "description": "Existing memory owner key: character:<id> or shared:<id>; omission uses the actual execution participant binding."
        },
        {
          "name": "caller_card_id",
          "type": "string",
          "required": false,
          "description": "Explicit calling participant identity; must match an authenticated execution participant when one exists."
        },
        {
          "name": "link_id",
          "type": "string",
          "required": false,
          "description": "Lossless decimal-string relationship ID."
        },
        {
          "name": "source_title",
          "type": "string",
          "required": false,
          "description": "Exact source title."
        },
        {
          "name": "target_title",
          "type": "string",
          "required": false,
          "description": "Exact target title."
        },
        {
          "name": "link_type",
          "type": "string",
          "required": false,
          "description": "Exact directed relationship type."
        },
        {
          "name": "limit",
          "type": "integer",
          "required": false,
          "description": "Maximum results, default 20, capped at 200."
        }
      ]
    },
    {
      "name": "update_memory_link",
      "description": "Patch one unambiguous relationship without replacing unspecified fields.",
      "parameters": [
        {
          "name": "target_owner_key",
          "type": "string",
          "required": false,
          "description": "Existing memory owner key: character:<id> or shared:<id>; omission uses the actual execution participant binding."
        },
        {
          "name": "caller_card_id",
          "type": "string",
          "required": false,
          "description": "Explicit calling participant identity; must match an authenticated execution participant when one exists."
        },
        {
          "name": "link_id",
          "type": "string",
          "required": false,
          "description": "Lossless decimal-string relationship ID."
        },
        {
          "name": "source_title",
          "type": "string",
          "required": false,
          "description": "Exact source title."
        },
        {
          "name": "target_title",
          "type": "string",
          "required": false,
          "description": "Exact target title."
        },
        {
          "name": "link_type",
          "type": "string",
          "required": false,
          "description": "Exact directed relationship type."
        },
        {
          "name": "new_link_type",
          "type": "string",
          "required": false,
          "description": "New relationship type."
        },
        {
          "name": "weight",
          "type": "number",
          "required": false,
          "description": "New weight between 0 and 1."
        },
        {
          "name": "description",
          "type": "string",
          "required": false,
          "description": "New relationship description."
        }
      ]
    },
    {
      "name": "delete_memory_link",
      "description": "Delete one exact-ID or unambiguous directed relationship.",
      "parameters": [
        {
          "name": "target_owner_key",
          "type": "string",
          "required": false,
          "description": "Existing memory owner key: character:<id> or shared:<id>; omission uses the actual execution participant binding."
        },
        {
          "name": "caller_card_id",
          "type": "string",
          "required": false,
          "description": "Explicit calling participant identity; must match an authenticated execution participant when one exists."
        },
        {
          "name": "link_id",
          "type": "string",
          "required": false,
          "description": "Lossless decimal-string relationship ID."
        },
        {
          "name": "source_title",
          "type": "string",
          "required": false,
          "description": "Exact source title."
        },
        {
          "name": "target_title",
          "type": "string",
          "required": false,
          "description": "Exact target title."
        },
        {
          "name": "link_type",
          "type": "string",
          "required": false,
          "description": "Exact directed relationship type."
        }
      ]
    },
    {
      "name": "list_character_cards",
      "description": "List actual character cards from this plugin configuration files.",
      "parameters": []
    }
  ]
}
*/
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

// src/runtime-tools/tools.ts
var tools_exports = {};
__export(tools_exports, {
  create_memory: () => create_memory,
  delete_memory: () => delete_memory,
  delete_memory_link: () => delete_memory_link,
  get_memory_by_title: () => get_memory_by_title,
  get_memory_owner_key: () => get_memory_owner_key,
  link_memories: () => link_memories,
  list_character_cards: () => list_character_cards,
  move_memory: () => move_memory,
  query_memory: () => query_memory,
  query_memory_links: () => query_memory_links,
  update_memory: () => update_memory,
  update_memory_link: () => update_memory_link,
  update_user_preferences: () => update_user_preferences
});
module.exports = __toCommonJS(tools_exports);

// src/runtime-tools/contract.ts
var hostRuntimeParameterNames = [
  "__operit_package_lang",
  "__operit_package_state",
  "__operit_package_caller_name",
  "__operit_package_chat_id",
  "__operit_package_caller_participant_id",
  "__operit_package_name",
  "__operit_toolpkg_runtime_kind",
  "__operit_toolpkg_api_version",
  "__operit_execution_context_key",
  "__operit_toolpkg_subpackage_id",
  "containerPackageName",
  "toolPkgId",
  "__operit_ui_package_name",
  "__operit_script_screen"
];
var callDomain = (operation, input) => ToolPkg.ipc.call("character-memory.domain", { operation, input }, { targetRuntime: "main" });
function parameters(value) {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new Error("Tool parameters must be an object");
  return value;
}
function optionalText(input, name) {
  if (!Object.prototype.hasOwnProperty.call(input, name)) return void 0;
  const value = input[name];
  if (typeof value !== "string") throw new Error(name + " must be a string");
  return value;
}
function requiredText(input, name) {
  const value = optionalText(input, name);
  if (value === void 0 || value.trim() === "") throw new Error("Missing or empty required parameter: " + name);
  return value;
}
function optionalNumber(input, name) {
  if (!Object.prototype.hasOwnProperty.call(input, name)) return void 0;
  const value = input[name];
  if (typeof value !== "number" || !Number.isFinite(value)) throw new Error(name + " must be a finite number");
  return value;
}
function limit(input, defaultLimit) {
  const value = optionalNumber(input, "limit");
  if (value === void 0) return defaultLimit;
  if (!Number.isSafeInteger(value) || value < 0) throw new Error("Invalid limit. Expected a nonnegative integer.");
  return Math.max(1, value);
}
function score(input, name) {
  const value = optionalNumber(input, name);
  if (value !== void 0 && (value < 0 || value > 1)) throw new Error(name + " must be between 0 and 1");
  return value;
}
function callerContext(input) {
  const field = (name) => {
    const value = optionalText(input, name);
    if (value === void 0) return null;
    if (value.trim() === "") throw new Error(name + " must not be blank");
    return value;
  };
  return { chatId: field("__operit_package_chat_id"), participantId: field("__operit_package_caller_participant_id"), callerName: field("__operit_package_caller_name") };
}
function formatTime(value) {
  const date = new Date(value);
  if (!Number.isSafeInteger(value) || Number.isNaN(date.getTime())) throw new Error("Invalid memory timestamp");
  const pad = (part) => String(part).padStart(2, "0");
  return date.getFullYear() + "-" + pad(date.getMonth() + 1) + "-" + pad(date.getDate()) + " " + pad(date.getHours()) + ":" + pad(date.getMinutes());
}
function timeBoundary(input, name) {
  const value = optionalText(input, name);
  if (value === void 0) return null;
  const match = /^(\d{4})-(\d{2})-(\d{2})(?: (\d{2}):(\d{2}))?$/.exec(value.trim());
  if (match === null) throw new Error("Invalid " + name + ". Expected format YYYY-MM-DD or YYYY-MM-DD HH:mm.");
  const end = name === "end_time", year = Number(match[1]), month = Number(match[2]), day = Number(match[3]);
  const hour = match[4] === void 0 ? end ? 23 : 0 : Number(match[4]);
  const minute = match[5] === void 0 ? end ? 59 : 0 : Number(match[5]);
  const date = /* @__PURE__ */ new Date(0);
  date.setFullYear(year, month - 1, day);
  date.setHours(hour, minute, end ? 59 : 0, end ? 999 : 0);
  if (month < 1 || month > 12 || date.getFullYear() !== year || date.getMonth() !== month - 1 || date.getDate() !== day || date.getHours() !== hour || date.getMinutes() !== minute) throw new Error("Local time is invalid: " + value);
  for (const offset of [-120, -90, -60, -30, 30, 60, 90, 120]) {
    const other = new Date(date.getTime() + offset * 6e4);
    if (other.getFullYear() === year && other.getMonth() === month - 1 && other.getDate() === day && other.getHours() === hour && other.getMinutes() === minute) throw new Error("Local time is ambiguous: " + value);
  }
  return date.getTime();
}

// src/validation.ts
function requireDecimal(value, label, positive = false) {
  if (typeof value !== "string" || !/^(0|[1-9][0-9]*)$/.test(value)) throw new Error(`${label} must be a canonical decimal string`);
  const integer2 = BigInt(value);
  if (integer2 > 9223372036854775807n || positive && integer2 === 0n) throw new Error(`${label} is outside its i64 range`);
  return value;
}
function assertObject(value, label) {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new Error(`${label} \u5FC5\u987B\u662F\u5BF9\u8C61`);
}
function assertString(value, label) {
  if (typeof value !== "string") throw new Error(`${label} \u5FC5\u987B\u662F\u5B57\u7B26\u4E32`);
}
function assertBoolean(value, label) {
  if (typeof value !== "boolean") throw new Error(`${label} \u5FC5\u987B\u662F\u5E03\u5C14\u503C`);
}
function assertNumber(value, label, min, max) {
  if (typeof value !== "number" || !Number.isFinite(value) || value < min || value > max) throw new Error(`${label} \u5FC5\u987B\u5728 ${min} \u5230 ${max} \u4E4B\u95F4`);
}
function assertInteger(value, label, min) {
  assertNumber(value, label, min, Number.MAX_SAFE_INTEGER);
  if (!Number.isSafeInteger(value)) throw new Error(`${label} \u5FC5\u987B\u662F\u5B89\u5168\u6574\u6570`);
}
function assertNullableString(value, label) {
  if (value !== null) assertString(value, label);
}
function requireId(value, label) {
  assertString(value, label);
  if (value.trim() === "" || value !== value.trim()) throw new Error(`${label} \u65E0\u6548`);
  return value;
}
function requireName(value, label) {
  assertString(value, label);
  const name = value.trim();
  if (name === "") throw new Error(`${label} \u4E0D\u80FD\u4E3A\u7A7A`);
  return name;
}
function assertMemory(value) {
  assertObject(value, "\u8BB0\u5FC6\u8BB0\u5F55");
  requireDecimal(value.id, "\u8BB0\u5FC6\u6570\u636E\u5E93\u6807\u8BC6", true);
  requireId(value.uuid, "\u8BB0\u5FC6 UUID");
  for (const field of ["title", "content", "contentType", "source"]) assertString(value[field], field);
  assertNumber(value.credibility, "\u53EF\u4FE1\u5EA6", 0, 1);
  assertNumber(value.importance, "\u91CD\u8981\u6027", 0, 1);
  for (const field of ["documentPath", "chunkIndexFilePath", "folderPath"]) assertNullableString(value[field], field);
  assertBoolean(value.isDocumentNode, "\u6587\u6863\u8282\u70B9\u6807\u8BC6");
  for (const field of ["createdAt", "updatedAt", "lastAccessedAt"]) assertInteger(value[field], field, 0);
  if (!Array.isArray(value.tags) || !Array.isArray(value.properties)) throw new Error("\u8BB0\u5FC6\u6807\u7B7E\u548C\u5C5E\u6027\u5FC5\u987B\u662F\u6570\u7EC4");
  for (const tag of value.tags) {
    assertObject(tag, "\u8BB0\u5FC6\u6807\u7B7E");
    requireDecimal(tag.id, "\u8BB0\u5FC6\u6807\u7B7E\u6807\u8BC6");
    assertString(tag.name, "\u8BB0\u5FC6\u6807\u7B7E\u540D\u79F0");
  }
  for (const property2 of value.properties) {
    assertObject(property2, "\u8BB0\u5FC6\u5C5E\u6027");
    requireDecimal(property2.id, "\u8BB0\u5FC6\u5C5E\u6027\u6807\u8BC6");
    assertString(property2.key, "\u8BB0\u5FC6\u5C5E\u6027\u952E");
    assertString(property2.value, "\u8BB0\u5FC6\u5C5E\u6027\u503C");
  }
}
function assertMemoryChatMessage(value) {
  assertObject(value, "chat message");
  assertInteger(value.timestamp, "message timestamp", 0);
  for (const key of ["sender", "content", "provider", "modelName"]) assertString(value[key], key);
  assertInteger(value.variantIndex, "message variant index", 0);
  assertInteger(value.variantCount, "message variant count", 1);
  if (value.variantIndex > 2147483647 || value.variantCount > 2147483647 || value.variantIndex >= value.variantCount) throw new Error("Invalid persisted message variant identity");
}
function assertMemorySettings(value) {
  assertObject(value, "\u8BB0\u5FC6\u8BBE\u7F6E");
  assertInteger(value.autoSaveIntervalMinutes, "\u81EA\u52A8\u4FDD\u5B58\u95F4\u9694", 1);
  if (value.autoSaveIntervalMinutes > 30) throw new Error("\u81EA\u52A8\u4FDD\u5B58\u95F4\u9694\u5FC5\u987B\u5728 1 \u5230 30 \u5206\u949F\u4E4B\u95F4");
  assertInteger(value.nextAutoSaveRunAtMs, "\u4E0B\u6B21\u81EA\u52A8\u4FDD\u5B58\u65F6\u95F4", 0);
  for (const field of ["memoryExtractionCustomRules", "cloudEmbeddingEndpoint", "cloudEmbeddingApiKey", "cloudEmbeddingModel"]) assertString(value[field], field);
  for (const field of ["profileAutoUpdateEnabled", "profileAutoUpdateLocked", "cloudEmbeddingEnabled"]) assertBoolean(value[field], field);
}
function assertSearchConfig(value) {
  assertObject(value, "\u8BB0\u5FC6\u641C\u7D22\u8BBE\u7F6E");
  if (value.scoreMode !== "BALANCED" && value.scoreMode !== "KEYWORD_FIRST" && value.scoreMode !== "SEMANTIC_FIRST") throw new Error("\u8BB0\u5FC6\u8BC4\u5206\u6A21\u5F0F\u65E0\u6548");
  for (const field of ["keywordWeight", "tagWeight", "vectorWeight", "edgeWeight"]) assertNumber(value[field], field, 0, Number.MAX_VALUE);
}

// src/memory-jobs/embeddings.ts
function assertEmbedding(value) {
  assertObject(value, "embedding");
  requireName(value.endpoint, "embedding endpoint");
  requireName(value.model, "embedding model");
  assertString(value.text, "embedding text");
  assertInteger(value.updatedAt, "embedding timestamp", 0);
  if (!Array.isArray(value.vector) || value.vector.length === 0) throw new Error("Embedding must contain a nonempty vector");
  let norm = 0;
  for (const component of value.vector) {
    assertNumber(component, "embedding component", -Number.MAX_VALUE, Number.MAX_VALUE);
    norm += component * component;
  }
  if (!Number.isFinite(norm) || norm === 0) throw new Error("Embedding vector has an invalid norm");
}
function cosineSimilarity(left, right) {
  if (left.length === 0 || left.length !== right.length) throw new Error("Embedding dimensions do not match");
  let dot = 0, leftNorm = 0, rightNorm = 0;
  for (let index = 0; index < left.length; index += 1) {
    dot += left[index] * right[index];
    leftNorm += left[index] ** 2;
    rightNorm += right[index] ** 2;
  }
  if (leftNorm === 0 || rightNorm === 0) throw new Error("Embedding norm must be positive");
  return Math.max(-1, Math.min(1, dot / Math.sqrt(leftNorm * rightNorm)));
}

// src/storage/state.ts
function userDocumentPath(ownerKey) {
  return "owners/" + encodeURIComponent(ownerKey) + "/USER.md";
}
function array(value, label) {
  if (!Array.isArray(value)) throw new Error(label + " must be an array");
}
function identities(records, label) {
  const ids = /* @__PURE__ */ new Set();
  for (const record2 of records) {
    assertObject(record2, label);
    const id2 = requireId(record2.id, label + " id");
    if (ids.has(id2)) throw new Error(label + " has duplicate id: " + id2);
    ids.add(id2);
  }
  return ids;
}
function assertMemorySpace(value) {
  assertObject(value, "memory space");
  requireId(value.ownerKey, "memory owner");
  array(value.memories, "memories");
  array(value.links, "links");
  array(value.chunks, "chunks");
  array(value.candidates, "candidates");
  const memoryIds = identities(value.memories, "memories");
  identities(value.links, "links");
  identities(value.chunks, "chunks");
  identities(value.candidates, "candidates");
  const memoriesByUuid = /* @__PURE__ */ new Map();
  for (const memory of value.memories) {
    assertMemory(memory);
    if (memoriesByUuid.has(memory.uuid)) throw new Error("Duplicate memory UUID: " + memory.uuid);
    memoriesByUuid.set(memory.uuid, memory);
  }
  for (const link of value.links) {
    assertObject(link, "link");
    requireDecimal(link.id, "link id", true);
    const source = requireDecimal(link.sourceMemoryId, "source memory id", true), target = requireDecimal(link.targetMemoryId, "target memory id", true);
    if (!memoryIds.has(source) || !memoryIds.has(target)) throw new Error("Relationship references a missing memory: " + link.id);
    if (source === target) throw new Error("Relationship cannot point to itself");
    assertString(link.type_, "link type");
    assertNumber(link.weight, "link weight", 0, 1);
    assertString(link.description, "link description");
  }
  const indices = /* @__PURE__ */ new Set();
  for (const chunk of value.chunks) {
    assertObject(chunk, "chunk");
    requireDecimal(chunk.id, "chunk id", true);
    const memoryUuid = requireId(chunk.memoryUuid, "chunk memory UUID");
    assertInteger(chunk.chunkIndex, "chunk index", 0);
    assertString(chunk.content, "chunk content");
    const memory = memoriesByUuid.get(memoryUuid);
    if (memory === void 0 || !memory.isDocumentNode) throw new Error("Chunk references a missing document memory");
    const identity = JSON.stringify([chunk.memoryUuid, chunk.chunkIndex]);
    if (indices.has(identity)) throw new Error("Duplicate document chunk index");
    indices.add(identity);
  }
  const candidateSources = /* @__PURE__ */ new Set();
  for (const candidate of value.candidates) {
    assertObject(candidate, "candidate");
    requireDecimal(candidate.id, "candidate id", true);
    requireId(candidate.chatId, "candidate chat id");
    for (const field of ["triggerMessageTimestamp", "triggerVariantIndex", "createdAt", "updatedAt", "attemptCount"]) assertInteger(candidate[field], field, 0);
    for (const field of ["status", "lastError", "sourceType"]) assertString(candidate[field], field);
    if (candidate.status !== "pending" && candidate.status !== "processing" && candidate.status !== "failed") throw new Error("Invalid candidate status");
    if (candidate.sourceType !== "reply_finalized_auto" && candidate.sourceType !== "selected_user_message") throw new Error("Invalid candidate source type");
    if (typeof candidate.triggerVariantIndex !== "number" || candidate.triggerVariantIndex > 2147483647) throw new Error("Candidate message variant exceeds the host range");
    const sourceKey = JSON.stringify([candidate.chatId, candidate.triggerMessageTimestamp, candidate.triggerVariantIndex, candidate.sourceType]);
    if (candidateSources.has(sourceKey)) throw new Error("Duplicate persisted candidate source");
    candidateSources.add(sourceKey);
  }
  assertMemorySettings(value.settings);
  assertSearchConfig(value.searchConfig);
  assertString(value.userDocumentPath, "USER.md path");
  if (value.userDocumentPath !== userDocumentPath(requireId(value.ownerKey, "owner key"))) throw new Error("Invalid owner USER.md path");
  assertObject(value.rebuildProgress, "rebuild progress");
  for (const field of ["status", "currentChatTitle", "lastError"]) assertString(value.rebuildProgress[field], field);
  for (const field of ["totalChats", "completedChats", "totalWindows", "completedWindows", "totalSourceMessages", "processedSourceMessages", "failedWindows"]) assertInteger(value.rebuildProgress[field], field, 0);
  if (!["idle", "preparing", "running", "completed", "failed", "cancelled"].some(
    /** Accepts only the declared durable task lifecycle states. */
    (status) => value.rebuildProgress !== null && typeof value.rebuildProgress === "object" && "status" in value.rebuildProgress && value.rebuildProgress.status === status
  )) throw new Error("Invalid memory rebuild status");
  if (value.rebuildTask !== null) {
    assertObject(value.rebuildTask, "rebuild task");
    requireDecimal(value.rebuildTask.id, "rebuild task id", true);
    array(value.rebuildTask.windows, "rebuild windows");
    assertInteger(value.rebuildTask.nextWindow, "rebuild cursor", 0);
    if (value.rebuildTask.windows.length === 0 || value.rebuildTask.nextWindow > value.rebuildTask.windows.length) throw new Error("Invalid rebuild cursor or source plan");
    let sourceTotal = 0, processedTotal = 0, completedChats = 0, windowIndex = 0;
    const chats = /* @__PURE__ */ new Set(), finishedChats = /* @__PURE__ */ new Set();
    let previousChat = null;
    for (const window of value.rebuildTask.windows) {
      assertObject(window, "rebuild window");
      requireId(window.chatId, "rebuild chat id");
      assertString(window.chatTitle, "rebuild chat title");
      assertInteger(window.sourceMessageCount, "source message count", 1);
      array(window.messages, "rebuild messages");
      if (window.messages.length < window.sourceMessageCount) throw new Error("Rebuild source count exceeds the stored messages");
      sourceTotal += window.sourceMessageCount;
      if (windowIndex < value.rebuildTask.nextWindow) processedTotal += window.sourceMessageCount;
      const chatId = requireId(window.chatId, "window chat id");
      if (previousChat !== null && previousChat !== chatId) {
        finishedChats.add(previousChat);
        if (finishedChats.has(chatId)) throw new Error("Rebuild plan repeats a completed chat block");
        if (windowIndex <= value.rebuildTask.nextWindow) completedChats += 1;
      }
      previousChat = chatId;
      chats.add(chatId);
      windowIndex += 1;
      for (const message of window.messages) {
        assertMemoryChatMessage(message);
        if (message.sender !== "user" && message.sender !== "ai" && message.sender !== "assistant") throw new Error("Rebuild window has an unsupported source sender");
        assertString(message.content, "source message content");
        if (message.content.trim() === "") throw new Error("Rebuild window contains a blank source message");
      }
    }
    if (value.rebuildTask.nextWindow === value.rebuildTask.windows.length) completedChats += 1;
    if (value.rebuildProgress.totalWindows !== value.rebuildTask.windows.length || value.rebuildProgress.completedWindows !== value.rebuildTask.nextWindow) throw new Error("Rebuild plan and progress disagree");
    if (value.rebuildProgress.totalSourceMessages !== sourceTotal || value.rebuildProgress.processedSourceMessages !== processedTotal || value.rebuildProgress.totalChats !== chats.size || value.rebuildProgress.completedChats !== completedChats) throw new Error("Rebuild counters disagree with the actual source plan");
    if (value.rebuildProgress.status === "completed" && value.rebuildTask.nextWindow !== value.rebuildTask.windows.length) throw new Error("Completed rebuild has unprocessed source windows");
    if (value.rebuildProgress.status === "preparing" && value.rebuildTask.nextWindow !== 0) throw new Error("Preparing rebuild already has processed source windows");
  } else if (value.rebuildProgress.status !== "idle") throw new Error("Rebuild progress is missing its source plan");
  array(value.embeddings, "embeddings");
  const embeddingKeys = /* @__PURE__ */ new Set();
  for (const item of value.embeddings) {
    assertEmbedding(item);
    const key = JSON.stringify([item.endpoint, item.model, item.text]);
    if (embeddingKeys.has(key)) throw new Error("Duplicate stored embedding input");
    embeddingKeys.add(key);
  }
}

// src/backup.ts
function decodeMemoryBackup(content) {
  assertString(content, "memory backup");
  const document = JSON.parse(content);
  assertObject(document, "memory backup");
  if (document.version !== "2.0") throw new Error("Unsupported memory backup version; this plugin uses full-record version 2.0");
  assertInteger(document.exportDate, "exportDate", 0);
  assertMemorySpace(document.space);
  assertString(document.userMarkdown, "USER.md backup content");
  return { version: "2.0", exportDate: document.exportDate, space: document.space, userMarkdown: document.userMarkdown };
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
function score2(value, path) {
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
function array2(validate) {
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
var strings = array2(text);
var identifiers = array2(nonblank);
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
  sharedMemoryMounts: optional(array2(mount)),
  toolAccessConfig: optional(toolAccess),
  isDefault: optional(boolean),
  createdAt: optional(integer),
  updatedAt: optional(integer)
};
var groupFields = { name: optional(nonblank), description: optional(text), themeConfigId: optional(nullable(text)), members: optional(array2(member)), createdAt: optional(integer), updatedAt: optional(integer) };
var tagFields = { name: optional(nonblank), description: optional(text), promptContent: optional(text), tagType: optional(enumeration("TONE", "CHARACTER", "FUNCTION", "CUSTOM")), createdAt: optional(integer), updatedAt: optional(integer) };
var memoryFields = {
  uuid: optional(text),
  title: optional(nonblank),
  content: optional(text),
  contentType: optional(text),
  source: optional(text),
  credibility: optional(score2),
  importance: optional(score2),
  documentPath: optional(nullable(text)),
  isDocumentNode: optional(boolean),
  chunkIndexFilePath: optional(nullable(text)),
  folderPath: optional(nullable(text)),
  createdAt: optional(integer),
  updatedAt: optional(integer),
  lastAccessedAt: optional(integer),
  tags: optional(strings),
  properties: optional(array2(property))
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
  "memory.move": shape({ ownerKey: required(owner), ids: required(array2(memoryIdentifier)), folderPath: required(text) }),
  "memory.link.create": shape({ ownerKey: required(owner), sourceTitle: required(nonblank), targetTitle: required(nonblank), linkType: required(nonblank), weight: required(score2), description: required(text) }),
  "memory.link.update": shape({ ownerKey: required(owner), linkId: required(memoryIdentifier), changes: required(shape({ linkType: optional(nonblank), weight: optional(score2), description: optional(text) }, true)) }),
  "memory.link.delete": shape({ ownerKey: required(owner), linkId: required(memoryIdentifier) }),
  "memory.export": ownerOnly,
  "memory.import": shape({ ownerKey: required(owner), content: required(nonblank), strategy: required(enumeration("SKIP", "UPDATE", "CREATE_NEW")) })
};
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

// src/runtime-tools/memory.ts
var MemoryTools = class {
  /** Binds executable tools to the actual typed main-runtime dispatcher. */
  constructor(domain) {
    this.domain = domain;
    this.snapshots = /* @__PURE__ */ new Map();
    this.nextSnapshot = 0;
    this.access = 0;
  }
  /** Resolves one exact caller identity rather than interpreting an opaque chat context key. */
  caller(input, context) {
    const explicit = optionalText(input, "caller_card_id");
    if (explicit !== void 0) {
      if (explicit.trim() === "") throw new Error("caller_card_id must not be blank");
      if (context.participantId !== null && explicit !== context.participantId) throw new Error("caller_card_id does not match the authenticated execution participant");
      return explicit;
    }
    if (context.participantId === null) throw new Error("caller_card_id or target_owner_key parameter is required");
    return context.participantId;
  }
  /** Resolves the actual persisted binding through the same domain service. */
  async boundOwner(participantId) {
    const owner2 = await this.domain("memory.resolveOwner", { characterId: participantId });
    switch (owner2.kind) {
      case "CHARACTER":
        return "character:" + owner2.id;
      case "SHARED":
        return "shared:" + owner2.id;
      default:
        throw new Error("Invalid memory owner kind");
    }
  }
  /** Validates an explicit owner, and enforces the selected participant's actual mount permissions. */
  async owner(input, context, write) {
    const explicit = optionalText(input, "target_owner_key");
    if (explicit !== void 0 && !/^(character|shared):[^:\s]+$/.test(explicit)) throw new Error("Invalid target_owner_key");
    const selected = explicit === void 0 ? await this.boundOwner(this.caller(input, context)) : explicit;
    if (context.participantId !== null) {
      const primary = await this.boundOwner(context.participantId);
      if (selected !== primary) {
        const card = await this.domain("character.get", { id: context.participantId });
        const permitted = card.sharedMemoryMounts.some(
          /** Tests the real persisted mount key and requested effect. */
          (mount2) => "shared:" + mount2.sharedMemoryId === selected && (write ? mount2.writable : mount2.readable)
        );
        if (!permitted) throw new Error("Memory owner access denied: " + selected);
      }
    }
    return selected;
  }
  /** Reads a validated complete repository export; no private copy of durable data is retained. */
  async space(ownerKey) {
    const exported = await this.domain("memory.export", { ownerKey });
    const space = decodeMemoryBackup(exported.content).space;
    if (space.ownerKey !== ownerKey) throw new Error("Memory export owner does not match the requested owner");
    return space;
  }
  /** Projects the historical model-facing result, keeping record identities internal to the plugin. */
  info(ownerKey, memory) {
    return { ownerKey, title: memory.title, content: memory.content, source: memory.source, tags: memory.tags.map(
      /** Retains actual tag names in the historical query projection. */
      (tag) => tag.name
    ), createdAt: formatTime(memory.createdAt), chunkInfo: null, chunkIndices: null };
  }
  /** Formats the original five-chunk document result with one-based display indices. */
  formatChunks(title, total, chunks) {
    return "Document: " + title + "\n" + chunks.slice(0, 5).map(
      /** Uses real persisted chunk indices instead of reindexing the selected subset. */
      (chunk) => "Chunk " + (chunk.chunkIndex + 1) + "/" + total + ":\n" + chunk.content
    ).join("\n---\n");
  }
  /** Reads only a committed main-service embedding; tool projections never mutate a detached export cache.
   * A missing or duplicate entry is a consistency error rather than permission to compute outside the main repository.
   */
  committedEmbedding(space, text2) {
    const endpoint = space.settings.cloudEmbeddingEndpoint.replace(/\/+$/, "");
    const model = space.settings.cloudEmbeddingModel;
    if (!space.settings.cloudEmbeddingEnabled) throw new Error("Cloud embedding is disabled");
    const matches = space.embeddings.filter(
      /** Selects the exact persisted provider and input provenance. */
      (item) => item.endpoint === endpoint && item.model === model && item.text === text2
    );
    if (matches.length !== 1) throw new Error("Embedding is missing or duplicated in the committed main-service cache: " + text2);
    assertEmbedding(matches[0]);
    return matches[0].vector;
  }
  /** Combines lexical coverage and genuine committed embedding channels with the original chunk RRF weights. */
  async searchChunks(space, memory, query, count) {
    const chunks = space.chunks.filter(
      /** Selects the complete real document by UUID. */
      (chunk) => chunk.memoryUuid === memory.uuid
    ).sort(
      /** Preserves the persisted document order. */
      (left, right) => left.chunkIndex - right.chunkIndex
    );
    if (query.trim() === "*" || query.trim() === "") return chunks.slice(0, count);
    const keywords = query.split(query.indexOf("|") === -1 ? /\s+/u : /\|/u).map(
      /** Preserves the explicitly authored semantic channels. */
      (word) => word.trim()
    ).filter(
      /** Rejects empty separators before requesting real embeddings. */
      (word) => word.length !== 0
    );
    const patterns = [...new Set(keywords)].map(
      /** Implements authored wildcard spelling using escaped literal segments. */
      (word) => new RegExp(word.split("*").map(
        /** Escapes one exact authored keyword segment. */
        (segment) => segment.replace(/[.*+?^$\{\}()|[\]\\]/g, "\\$&")
      ).join(".*"), "isu")
    );
    let keywordMultiplier, vectorMultiplier;
    switch (space.searchConfig.scoreMode) {
      case "BALANCED":
        keywordMultiplier = 1;
        vectorMultiplier = 1;
        break;
      case "KEYWORD_FIRST":
        keywordMultiplier = 1.3;
        vectorMultiplier = 0.8;
        break;
      case "SEMANTIC_FIRST":
        keywordMultiplier = 0.8;
        vectorMultiplier = 1.3;
        break;
      default:
        throw new Error("Invalid memory scoring mode");
    }
    const scores = /* @__PURE__ */ new Map();
    if (space.searchConfig.keywordWeight > 0) {
      const lexical = chunks.map(
        /** Measures exact lexical coverage in each real persisted chunk. */
        (chunk) => ({ chunk, hits: patterns.filter(
          /** Matches document content against the declared lexical expression. */
          (pattern) => pattern.test(chunk.content)
        ).length })
      ).filter(
        /** Includes only genuine lexical matches in the coverage channel. */
        (item) => item.hits > 0
      ).sort(
        /** Orders coverage ties by original document position. */
        (left, right) => right.hits - left.hits || left.chunk.chunkIndex - right.chunk.chunkIndex
      );
      for (let rank = 0; rank < lexical.length; rank += 1) {
        const item = lexical[rank];
        scores.set(item.chunk.id, { chunk: item.chunk, score: space.searchConfig.keywordWeight * keywordMultiplier * (1 + 0.6 * item.hits / Math.max(1, patterns.length)) / (61 + rank) });
      }
    }
    if (space.searchConfig.vectorWeight > 0) {
      const normalization = 1 / Math.sqrt(Math.max(1, keywords.length));
      for (const keyword of keywords) {
        await this.domain("memory.searchWithOptions", { ownerKey: space.ownerKey, query: keyword, folderPath: null, relevanceThreshold: 0, createdAtStartMs: null, createdAtEndMs: null });
        const committed = await this.space(space.ownerKey), queryVector = this.committedEmbedding(committed, keyword);
        const semantic = [];
        for (const chunk of chunks) semantic.push({ chunk, similarity: cosineSimilarity(queryVector, this.committedEmbedding(committed, chunk.content)) });
        semantic.sort(
          /** Ranks actual provider similarities with a stable persisted chunk tie breaker. */
          (left, right) => right.similarity - left.similarity || left.chunk.chunkIndex - right.chunk.chunkIndex
        );
        for (let rank = 0; rank < semantic.length; rank += 1) {
          const item = semantic[rank], contribution = (1 / (61 + rank) + item.similarity * space.searchConfig.vectorWeight * vectorMultiplier) * normalization;
          const entry = scores.get(item.chunk.id);
          if (entry === void 0) scores.set(item.chunk.id, { chunk: item.chunk, score: contribution });
          else entry.score += contribution;
        }
      }
    }
    return [...scores.values()].sort(
      /** Preserves original RRF ordering and lossless numeric identity ties. */
      (left, right) => right.score - left.score || (BigInt(left.chunk.id) < BigInt(right.chunk.id) ? -1 : BigInt(left.chunk.id) > BigInt(right.chunk.id) ? 1 : 0)
    ).slice(0, count).map(
      /** Returns complete original chunk records rather than a synthetic projection. */
      (item) => item.chunk
    );
  }
  /** Allocates or reuses owner-scoped pagination under the original snapshot contract. */
  snapshot(ownerKey, requested) {
    let snapshots = this.snapshots.get(ownerKey);
    if (snapshots === void 0) {
      snapshots = /* @__PURE__ */ new Map();
      this.snapshots.set(ownerKey, snapshots);
    }
    const id2 = requested === void 0 ? "memory-query-" + ++this.nextSnapshot : requested;
    if (id2.trim() === "") throw new Error("snapshot_id must not be blank");
    let state = snapshots.get(id2);
    const created = state === void 0;
    if (state === void 0) {
      state = { seen: /* @__PURE__ */ new Set(), access: ++this.access };
      snapshots.set(id2, state);
    }
    state.access = ++this.access;
    while (snapshots.size > 32) {
      let oldestKey, oldest = Infinity;
      for (const [key, item] of snapshots) if (key !== id2 && item.access < oldest) {
        oldest = item.access;
        oldestKey = key;
      }
      if (oldestKey === void 0) throw new Error("Query snapshot index is inconsistent");
      snapshots.delete(oldestKey);
    }
    return { id: id2, state, created };
  }
  /** Resolves a relationship by its exact lossless ID or an unambiguous title pair. */
  link(space, input) {
    const id2 = optionalText(input, "link_id");
    let matches;
    if (id2 !== void 0) matches = space.links.filter(
      /** Matches the validated string identity without a floating-point conversion. */
      (link) => link.id === parseMemoryIdentifier(id2, "link_id")
    );
    else {
      const source = this.memory(space, requiredText(input, "source_title")), target = this.memory(space, requiredText(input, "target_title"));
      const type = optionalText(input, "link_type");
      matches = space.links.filter(
        /** Uses the exact directed title pair and optional relation type. */
        (link) => link.sourceMemoryId === source.id && link.targetMemoryId === target.id && (type === void 0 || link.type_ === type)
      );
    }
    if (matches.length === 0) throw new Error("No matching link found");
    if (matches.length !== 1) throw new Error("Multiple links matched. Provide link_id or a more specific link_type.");
    return matches[0];
  }
  /** Requires an exact title and rejects ambiguous records instead of choosing one silently. */
  memory(space, title) {
    const matches = space.memories.filter(
      /** Preserves exact title lookup semantics. */
      (memory) => memory.title === title
    );
    if (matches.length === 0) throw new Error("Memory not found with title: " + title);
    if (matches.length !== 1) throw new Error("Multiple memories matched title: " + title);
    return matches[0];
  }
  /** Resolves complete relationship display fields from the authoritative same-owner records. */
  linkInfo(space, link) {
    const source = space.memories.find(
      /** Resolves the real source identity. */
      (memory) => memory.id === link.sourceMemoryId
    ), target = space.memories.find(
      /** Resolves the real target identity. */
      (memory) => memory.id === link.targetMemoryId
    );
    if (source === void 0 || target === void 0) throw new Error("Memory link refers to a missing record: " + link.id);
    return { linkId: link.id, sourceTitle: source.title, targetTitle: target.title, linkType: link.type_, weight: link.weight, description: link.description };
  }
  /** Executes the selected tool using the actual shared memory domain service. */
  async execute(operation, input, context) {
    if (operation === "get_memory_owner_key") return this.boundOwner(this.caller(input, context));
    const write = !["query_memory", "get_memory_by_title", "query_memory_links"].some(
      /** Classifies only the declared read tools. */
      (name) => name === operation
    );
    const ownerKey = await this.owner(input, context, write);
    switch (operation) {
      case "query_memory":
        return this.query(ownerKey, input);
      case "get_memory_by_title":
        return this.byTitle(ownerKey, input);
      case "create_memory": {
        const contentType = optionalText(input, "content_type"), source = optionalText(input, "source"), folderPath = optionalText(input, "folder_path"), tags = optionalText(input, "tags");
        const result = await this.domain("memory.create", { ownerKey, values: { title: requiredText(input, "title"), content: requiredText(input, "content"), contentType: contentType === void 0 ? "text/plain" : contentType, source: source === void 0 ? "ai_created" : source, folderPath: folderPath === void 0 ? null : folderPath, tags: tags === void 0 ? [] : this.tags(tags) } });
        return "Successfully created memory: '" + result.item.title + "' (UUID: " + result.item.uuid + ")";
      }
      case "update_memory": {
        const oldTitle = requiredText(input, "old_title"), current = await this.domain("memory.get", { ownerKey, title: oldTitle }), changes = {};
        for (const [parameter, field] of [["new_title", "title"], ["content", "content"], ["content_type", "contentType"], ["source", "source"], ["folder_path", "folderPath"]]) {
          const value = optionalText(input, parameter);
          if (value !== void 0) changes[field] = value;
        }
        for (const field of ["credibility", "importance"]) {
          const value = score(input, field);
          if (value !== void 0) changes[field] = value;
        }
        const tags = optionalText(input, "tags");
        if (tags !== void 0) changes.tags = this.tags(tags);
        if (Object.keys(changes).length === 0) throw new Error("At least one memory change must be provided");
        const updated = await this.domain("memory.update", { ownerKey, originalTitle: oldTitle, expectedId: current.item.id, changes });
        return "Successfully updated memory from '" + oldTitle + "' to '" + updated.item.title + "'";
      }
      case "delete_memory": {
        const title = requiredText(input, "title"), memory = await this.domain("memory.get", { ownerKey, title });
        await this.domain("memory.delete", { ownerKey, id: memory.item.id });
        return "Successfully deleted memory: '" + title + "'";
      }
      case "move_memory":
        return this.move(ownerKey, input);
      case "update_user_preferences":
        await this.domain("memory.user.write", { ownerKey, content: requiredText(input, "content") });
        return "Successfully updated USER.md";
      case "link_memories": {
        const sourceTitle = requiredText(input, "source_title"), targetTitle = requiredText(input, "target_title"), linkType = optionalText(input, "link_type"), weight = score(input, "weight"), description = optionalText(input, "description");
        const result = await this.domain("memory.link.create", { ownerKey, sourceTitle, targetTitle, linkType: linkType === void 0 ? "related" : linkType, weight: weight === void 0 ? 0.5 : weight, description: description === void 0 ? "" : description });
        return { sourceTitle, targetTitle, linkType: result.link.type_, weight: result.link.weight, description: result.link.description };
      }
      case "query_memory_links":
        return this.links(ownerKey, input);
      case "update_memory_link": {
        const space = await this.space(ownerKey), link = this.link(space, input), changes = {};
        const type = optionalText(input, "new_link_type"), weight = score(input, "weight"), description = optionalText(input, "description");
        if (type !== void 0) {
          if (type.trim() === "") throw new Error("new_link_type must not be blank");
          changes.linkType = type;
        }
        if (weight !== void 0) changes.weight = weight;
        if (description !== void 0) changes.description = description;
        if (Object.keys(changes).length === 0) throw new Error("At least one of new_link_type, weight, description must be provided");
        const result = await this.domain("memory.link.update", { ownerKey, linkId: link.id, changes });
        return { totalCount: 1, links: [this.linkInfo(space, result.link)] };
      }
      case "delete_memory_link": {
        const link = this.link(await this.space(ownerKey), input);
        await this.domain("memory.link.delete", { ownerKey, linkId: link.id });
        return "Successfully deleted memory link: " + link.id;
      }
      default:
        throw new Error("Unknown memory tool: " + operation);
    }
  }
  /** Splits the original comma-delimited tag contract without touching stored tag identities. */
  tags(value) {
    return value.split(",").map(
      /** Normalizes only delimiters and surrounding authored tag whitespace. */
      (tag) => tag.trim()
    ).filter(
      /** Excludes empty tag entries as declared by the tool contract. */
      (tag) => tag !== ""
    );
  }
  /** Performs filtered full-record search and commits pagination only after every document read succeeds. */
  async query(ownerKey, input) {
    const query = requiredText(input, "query"), count = limit(input, query.trim() === "*" ? Number.MAX_SAFE_INTEGER : 20), threshold = optionalNumber(input, "threshold"), folder = optionalText(input, "folder_path");
    if (threshold !== void 0 && threshold < 0) throw new Error("Invalid threshold. Expected number >= 0.");
    const start = timeBoundary(input, "start_time"), end = timeBoundary(input, "end_time");
    if (start !== null && end !== null && start > end) throw new Error("start_time must not be after end_time");
    const result = await this.domain("memory.searchWithOptions", { ownerKey, query, folderPath: folder === void 0 ? null : folder, relevanceThreshold: threshold === void 0 ? 0 : threshold, createdAtStartMs: start, createdAtEndMs: end });
    if (result.ownerKey !== ownerKey) throw new Error("Memory search owner does not match the requested owner");
    const found = result.items, space = await this.space(ownerKey);
    const snapshot = this.snapshot(ownerKey, optionalText(input, "snapshot_id"));
    const unseen = found.filter(
      /** Excludes only records already delivered from this same owner and snapshot. */
      (memory) => !snapshot.state.seen.has(ownerKey + ":" + memory.id)
    ), returned = unseen.slice(0, count);
    const memories = [];
    for (const memory of returned) {
      const info = this.info(ownerKey, memory);
      if (memory.isDocumentNode) {
        const total = space.chunks.filter(
          /** Counts every real chunk belonging to this document. */
          (chunk) => chunk.memoryUuid === memory.uuid
        ).length, chunks = await this.searchChunks(space, memory, query, Math.min(count, 20));
        info.content = query.trim() === "*" || count > 20 ? "Document: " + memory.title + " (" + total + " chunks)" : this.formatChunks(memory.title, total, chunks);
        if (chunks.length !== 0) {
          info.chunkInfo = "Chunks " + chunks.slice(0, 5).map(
            /** Displays selected indices using the historical one-based spelling. */
            (chunk) => chunk.chunkIndex + 1
          ).join(", ") + "/" + total;
          info.chunkIndices = chunks.map(
            /** Returns actual zero-based persisted indices for subsequent tool calls. */
            (chunk) => chunk.chunkIndex
          );
        }
      }
      memories.push(info);
    }
    for (const memory of returned) snapshot.state.seen.add(ownerKey + ":" + memory.id);
    return { memories, snapshotId: snapshot.id, snapshotCreated: snapshot.created, excludedBySnapshotCount: found.length - unseen.length };
  }
  /** Reads an exact title or validates the original one-based document query/index/range parameters. */
  async byTitle(ownerKey, input) {
    const title = requiredText(input, "title"), result = await this.domain("memory.get", { ownerKey, title }), memory = result.item;
    if (!memory.isDocumentNode) return { memories: [this.info(ownerKey, memory)], snapshotId: null, snapshotCreated: false, excludedBySnapshotCount: 0 };
    const space = await this.space(ownerKey), chunks = space.chunks.filter(
      /** Selects only this exact real document. */
      (chunk) => chunk.memoryUuid === memory.uuid
    ).sort(
      /** Retains stable document order. */
      (left, right) => left.chunkIndex - right.chunkIndex
    ), total = chunks.length;
    const query = optionalText(input, "query"), index = optionalNumber(input, "chunk_index"), range = optionalText(input, "chunk_range");
    const supplied = [query !== void 0, index !== void 0, range !== void 0].filter(
      /** Counts explicitly supplied selectors instead of guessing precedence. */
      (value) => value
    ).length;
    if (supplied === 0) return "Document: " + title + " (" + total + " chunks). Specify query, chunk_range (1-based start-end) or chunk_index (1-based).";
    if (supplied !== 1) throw new Error("Specify exactly one of query, chunk_range or chunk_index");
    let selected;
    if (query !== void 0) {
      if (query.trim() === "") throw new Error("query must not be blank");
      selected = await this.searchChunks(space, memory, query, limit(input, 20));
    } else {
      let start, end;
      if (range !== void 0) {
        const parsed = /^(\d+)-(\d+)$/.exec(range);
        if (parsed === null) throw new Error("Invalid chunk_range. Expected start-end");
        start = Number(parsed[1]);
        end = Number(parsed[2]);
      } else {
        if (index === void 0) throw new Error("Document selector is missing");
        start = index;
        end = index;
      }
      if (!Number.isSafeInteger(start) || !Number.isSafeInteger(end) || start < 1 || end > total || start > end) throw new Error("Chunk range out of bounds. Valid range: 1-" + total);
      selected = chunks.filter(
        /** Selects the validated one-based range using persisted zero-based indices. */
        (chunk) => chunk.chunkIndex >= start - 1 && chunk.chunkIndex <= end - 1
      );
    }
    if (selected.length === 0) throw new Error("No matching chunks found");
    return this.formatChunks(title, total, selected);
  }
  /** Moves the exact title/folder intersection through a single domain mutation. */
  async move(ownerKey, input) {
    const target = optionalText(input, "target_folder_path"), source = optionalText(input, "source_folder_path"), titles = optionalText(input, "titles");
    if (target === void 0) throw new Error("target_folder_path parameter is required");
    if (source === void 0 && titles === void 0) throw new Error("Provide titles and/or source_folder_path to select memories to move");
    const names = titles === void 0 ? null : new Set(titles.split(/[,\n|]/).map(
      /** Retains the original three title delimiters. */
      (title) => title.trim()
    ).filter(
      /** Rejects delimiter-only selection instead of selecting all memories. */
      (title) => title !== ""
    ));
    if (names !== null && names.size === 0) throw new Error("titles must select at least one memory");
    const records = await this.domain("memory.list", { ownerKey }), selected = records.items.filter(
      /** Intersects both explicit selectors without silently changing selection strategy. */
      (memory) => (names === null || names.has(memory.title)) && (source === void 0 || (memory.folderPath === null ? "" : memory.folderPath) === source)
    );
    if (selected.length === 0) throw new Error("No matching memories found to move");
    const ids = [...new Set(selected.map(
      /** Keeps each lossless memory identity once. */
      (memory) => memory.id
    ))];
    const result = await this.domain("memory.move", { ownerKey, ids, folderPath: target });
    return "Successfully moved " + result.moved + " memories to '" + target + "'";
  }
  /** Queries actual directed relationships with precise filters and the original 200-result cap. */
  async links(ownerKey, input) {
    const space = await this.space(ownerKey), id2 = optionalText(input, "link_id"), source = optionalText(input, "source_title"), target = optionalText(input, "target_title"), type = optionalText(input, "link_type");
    if (id2 !== void 0) parseMemoryIdentifier(id2, "link_id");
    const sourceId = source === void 0 ? null : this.memory(space, source).id, targetId = target === void 0 ? null : this.memory(space, target).id;
    const links = space.links.filter(
      /** Applies every supplied filter to complete canonical relation records. */
      (link) => (id2 === void 0 || link.id === id2) && (sourceId === null || link.sourceMemoryId === sourceId) && (targetId === null || link.targetMemoryId === targetId) && (type === void 0 || link.type_ === type)
    ).slice(0, Math.min(limit(input, 20), 200)).map(
      /** Includes real relationship descriptions and unchanged string identities. */
      (link) => this.linkInfo(space, link)
    );
    return { totalCount: links.length, links };
  }
};

// src/runtime-tools/tools.ts
var executor = new MemoryTools(callDomain);
function execute(name, value) {
  const input = parameters(value);
  return executor.execute(name, input, callerContext(input));
}
async function get_memory_owner_key(input) {
  return execute("get_memory_owner_key", input);
}
async function query_memory(input) {
  return execute("query_memory", input);
}
async function get_memory_by_title(input) {
  return execute("get_memory_by_title", input);
}
async function create_memory(input) {
  return execute("create_memory", input);
}
async function update_memory(input) {
  return execute("update_memory", input);
}
async function delete_memory(input) {
  return execute("delete_memory", input);
}
async function move_memory(input) {
  return execute("move_memory", input);
}
async function update_user_preferences(input) {
  return execute("update_user_preferences", input);
}
async function link_memories(input) {
  return execute("link_memories", input);
}
async function query_memory_links(input) {
  return execute("query_memory_links", input);
}
async function update_memory_link(input) {
  return execute("update_memory_link", input);
}
async function delete_memory_link(input) {
  return execute("delete_memory_link", input);
}
async function list_character_cards(input) {
  const values = parameters(input), runtimeNames = new Set(hostRuntimeParameterNames);
  for (const key of Object.keys(values)) {
    if (!runtimeNames.has(key)) throw new Error("Unknown list_character_cards parameter: " + key);
  }
  callerContext(values);
  const records = await callDomain("character.list", {});
  return { totalCount: records.length, cards: records.map(
    /** Preserves the historical tool projection from actual complete stored card records. */
    (card) => ({ id: card.id, name: card.name, description: card.description, isDefault: card.isDefault, createdAt: card.createdAt, updatedAt: card.updatedAt })
  ) };
}

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
import { callDomain, callerContext, hostRuntimeParameterNames, parameters } from "./contract";
import type { MemoryToolName, MemoryToolResult, MemoryToolResults } from "./contract";
import { MemoryTools } from "./memory";

const executor = new MemoryTools(callDomain);
/** Executes a declared tool through typed IPC, preserving the host-injected execution identity. */
function execute<N extends MemoryToolName>(name: N, value: unknown): Promise<MemoryToolResult<N>> {
  const input = parameters(value);
  return executor.execute(name, input, callerContext(input));
}

/** Resolve the actual memory owner bound to the execution participant. */
export async function get_memory_owner_key(input: unknown): Promise<MemoryToolResults["get_memory_owner_key"]> { return execute("get_memory_owner_key", input); }

/** Search complete memories and document chunks with reusable owner-scoped pagination. */
export async function query_memory(input: unknown): Promise<MemoryToolResults["query_memory"]> { return execute("query_memory", input); }

/** Read an exact memory title or real document chunks by query or one-based index/range. */
export async function get_memory_by_title(input: unknown): Promise<MemoryToolResults["get_memory_by_title"]> { return execute("get_memory_by_title", input); }

/** Create a full memory record with plugin-owned identity and persistence. */
export async function create_memory(input: unknown): Promise<MemoryToolResults["create_memory"]> { return execute("create_memory", input); }

/** Patch an exact memory title without replacing unspecified fields. */
export async function update_memory(input: unknown): Promise<MemoryToolResults["update_memory"]> { return execute("update_memory", input); }

/** Delete the exact memory selected by title. */
export async function delete_memory(input: unknown): Promise<MemoryToolResults["delete_memory"]> { return execute("delete_memory", input); }

/** Move memories selected by titles and/or a source-folder intersection. */
export async function move_memory(input: unknown): Promise<MemoryToolResults["move_memory"]> { return execute("move_memory", input); }

/** Write the full owner USER.md using the same main-runtime file service. */
export async function update_user_preferences(input: unknown): Promise<MemoryToolResults["update_user_preferences"]> { return execute("update_user_preferences", input); }

/** Create a directed relationship between two exact memory titles. */
export async function link_memories(input: unknown): Promise<MemoryToolResults["link_memories"]> { return execute("link_memories", input); }

/** Query complete directed relationships, including lossless IDs and descriptions. */
export async function query_memory_links(input: unknown): Promise<MemoryToolResults["query_memory_links"]> { return execute("query_memory_links", input); }

/** Patch one unambiguous relationship without replacing unspecified fields. */
export async function update_memory_link(input: unknown): Promise<MemoryToolResults["update_memory_link"]> { return execute("update_memory_link", input); }

/** Delete one exact-ID or unambiguous directed relationship. */
export async function delete_memory_link(input: unknown): Promise<MemoryToolResults["delete_memory_link"]> { return execute("delete_memory_link", input); }

/** Lists the actual plugin records through the single authenticated main-runtime domain dispatcher. */
export async function list_character_cards(input: unknown): Promise<{ totalCount: number; cards: { id: string; name: string; description: string; isDefault: boolean; createdAt: number; updatedAt: number }[] }> {
  const values = parameters(input), runtimeNames = new Set<string>(hostRuntimeParameterNames);
  for (const key of Object.keys(values)) {
    if (!runtimeNames.has(key)) throw new Error("Unknown list_character_cards parameter: " + key);
  }
  callerContext(values);
  const records = await callDomain("character.list", {});
  return { totalCount: records.length, cards: records.map(
    /** Preserves the historical tool projection from actual complete stored card records. */
    card => ({ id: card.id, name: card.name, description: card.description, isDefault: card.isDefault, createdAt: card.createdAt, updatedAt: card.updatedAt }),
  ) };
}

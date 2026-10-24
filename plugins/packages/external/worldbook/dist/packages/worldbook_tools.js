"use strict";
/* METADATA
{
  "name": "worldbook_tools",
  "display_name": {
    "zh": "世界书工具",
    "en": "World Book Tools"
  },
  "description": {
    "zh": "世界书条目的增删改查工具，支持关键词匹配、正则表达式和常驻激活。",
    "en": "CRUD tools for world book entries with keyword matching, regex support, and always-active mode."
  },
  "category": "Utility",
  "tools": [
    {
      "name": "list_entries",
      "description": {
        "zh": "列出所有世界书条目摘要。",
        "en": "List summaries for all world book entries."
      },
      "parameters": []
    },
    {
      "name": "get_entry",
      "description": {
        "zh": "获取指定世界书条目的完整详情。",
        "en": "Get the full details of a world book entry."
      },
      "parameters": [
        {
          "name": "id",
          "description": {
            "zh": "条目 ID",
            "en": "Entry ID"
          },
          "type": "string",
          "required": true
        }
      ]
    },
    {
      "name": "create_entry",
      "description": {
        "zh": "创建新的世界书条目。",
        "en": "Create a new world book entry."
      },
      "parameters": [
        {
          "name": "name",
          "description": {
            "zh": "条目名称",
            "en": "Entry name"
          },
          "type": "string",
          "required": true
        },
        {
          "name": "content",
          "description": {
            "zh": "注入内容",
            "en": "Injected content"
          },
          "type": "string",
          "required": true
        },
        {
          "name": "keywords",
          "description": {
            "zh": "关键词列表，逗号分隔",
            "en": "Comma-separated keywords"
          },
          "type": "string",
          "required": false
        },
        {
          "name": "is_regex",
          "description": {
            "zh": "关键词是否为正则表达式",
            "en": "Whether keywords are regular expressions"
          },
          "type": "boolean",
          "required": false
        },
        {
          "name": "case_sensitive",
          "description": {
            "zh": "关键词匹配是否大小写敏感",
            "en": "Whether keyword matching is case sensitive"
          },
          "type": "boolean",
          "required": false
        },
        {
          "name": "always_active",
          "description": {
            "zh": "是否常驻激活",
            "en": "Whether the entry is always active"
          },
          "type": "boolean",
          "required": false
        },
        {
          "name": "enabled",
          "description": {
            "zh": "是否启用",
            "en": "Whether the entry is enabled"
          },
          "type": "boolean",
          "required": false
        },
        {
          "name": "priority",
          "description": {
            "zh": "优先级",
            "en": "Priority"
          },
          "type": "number",
          "required": false
        },
        {
          "name": "scan_depth",
          "description": {
            "zh": "扫描深度",
            "en": "Scan depth"
          },
          "type": "number",
          "required": false
        },
        {
          "name": "inject_target",
          "description": {
            "zh": "注入目标，可选 system 或 user，默认 system",
            "en": "Injection target: system or user (default system)"
          },
          "type": "string",
          "required": false
        },
        {
          "name": "character_card_id",
          "description": {
            "zh": "绑定角色卡 ID；填写后仅在对应角色卡会话中生效",
            "en": "Bound character card ID; when set, the entry only works for that character card"
          },
          "type": "string",
          "required": false
        }
      ]
    },
    {
      "name": "update_entry",
      "description": {
        "zh": "更新已有世界书条目。",
        "en": "Update an existing world book entry."
      },
      "parameters": [
        {
          "name": "id",
          "description": {
            "zh": "条目 ID",
            "en": "Entry ID"
          },
          "type": "string",
          "required": true
        },
        {
          "name": "name",
          "description": {
            "zh": "新名称",
            "en": "New name"
          },
          "type": "string",
          "required": false
        },
        {
          "name": "content",
          "description": {
            "zh": "新注入内容",
            "en": "New injected content"
          },
          "type": "string",
          "required": false
        },
        {
          "name": "keywords",
          "description": {
            "zh": "新关键词列表，逗号分隔",
            "en": "New comma-separated keywords"
          },
          "type": "string",
          "required": false
        },
        {
          "name": "is_regex",
          "description": {
            "zh": "关键词是否为正则表达式",
            "en": "Whether keywords are regular expressions"
          },
          "type": "boolean",
          "required": false
        },
        {
          "name": "case_sensitive",
          "description": {
            "zh": "关键词匹配是否大小写敏感",
            "en": "Whether keyword matching is case sensitive"
          },
          "type": "boolean",
          "required": false
        },
        {
          "name": "always_active",
          "description": {
            "zh": "是否常驻激活",
            "en": "Whether the entry is always active"
          },
          "type": "boolean",
          "required": false
        },
        {
          "name": "enabled",
          "description": {
            "zh": "是否启用",
            "en": "Whether the entry is enabled"
          },
          "type": "boolean",
          "required": false
        },
        {
          "name": "priority",
          "description": {
            "zh": "优先级",
            "en": "Priority"
          },
          "type": "number",
          "required": false
        },
        {
          "name": "scan_depth",
          "description": {
            "zh": "扫描深度",
            "en": "Scan depth"
          },
          "type": "number",
          "required": false
        },
        {
          "name": "inject_target",
          "description": {
            "zh": "注入目标，可选 system 或 user",
            "en": "Injection target: system or user"
          },
          "type": "string",
          "required": false
        },
        {
          "name": "character_card_id",
          "description": {
            "zh": "绑定角色卡 ID；填写后仅在对应角色卡会话中生效",
            "en": "Bound character card ID; when set, the entry only works for that character card"
          },
          "type": "string",
          "required": false
        }
      ]
    },
    {
      "name": "delete_entry",
      "description": {
        "zh": "删除世界书条目。",
        "en": "Delete a world book entry."
      },
      "parameters": [
        {
          "name": "id",
          "description": {
            "zh": "条目 ID",
            "en": "Entry ID"
          },
          "type": "string",
          "required": true
        }
      ]
    },
    {
      "name": "toggle_entry",
      "description": {
        "zh": "切换世界书条目的启用状态。",
        "en": "Toggle a world book entry's enabled state."
      },
      "parameters": [
        {
          "name": "id",
          "description": {
            "zh": "条目 ID",
            "en": "Entry ID"
          },
          "type": "string",
          "required": true
        }
      ]
    },
    {
      "name": "import_entries",
      "description": {
        "zh": "从世界书 JSON 文件或 JSON 内容导入条目。兼容 Operit、SillyTavern lorebook，以及角色卡内嵌 character_book。",
        "en": "Import entries from a world book JSON file or JSON content. Supports Operit, SillyTavern lorebooks, and embedded character_book formats."
      },
      "parameters": [
        {
          "name": "path",
          "description": {
            "zh": "导入文件路径，支持普通文件路径或 content:// URI；与 content 二选一。",
            "en": "Import file path, supports normal file paths or content:// URIs; mutually exclusive with content."
          },
          "type": "string",
          "required": false
        },
        {
          "name": "content",
          "description": {
            "zh": "原始 JSON 文本；与 path 二选一。",
            "en": "Raw JSON text; mutually exclusive with path."
          },
          "type": "string",
          "required": false
        },
        {
          "name": "character_card_id",
          "description": {
            "zh": "可选，导入后统一绑定到指定角色卡。",
            "en": "Optional; bind all imported entries to the specified character card."
          },
          "type": "string",
          "required": false
        }
      ]
    },
    {
      "name": "list_character_cards_proxy",
      "description": {
        "zh": "通过代理列出所有角色卡，用于世界书 UI 选择角色卡。",
        "en": "List all character cards through a proxy for world book UI selection."
      },
      "parameters": []
    }
  ]
}
*/
Object.defineProperty(exports, "__esModule", { value: true });
const worldbook_service_js_1 = require("../shared/worldbook_service.js");
async function wrap(handler, params) {
    try {
        const result = await handler(params);
        complete(result);
    }
    catch (error) {
        const handledError = error;
        complete({ success: false, code: handledError.code, message: handledError.message });
    }
}
async function listEntries() {
    const entries = await (0, worldbook_service_js_1.listWorldBookEntries)();
    return { success: true, count: entries.length, entries };
}
async function getEntry(params) {
    const entry = await (0, worldbook_service_js_1.getWorldBookEntry)(String(params.id || ""));
    return { success: true, entry };
}
async function createEntry(params) {
    const entry = await (0, worldbook_service_js_1.createWorldBookEntry)(params);
    return { success: true, message: "条目已创建", entry };
}
async function updateEntry(params) {
    const entry = await (0, worldbook_service_js_1.updateWorldBookEntry)(params);
    return { success: true, message: "条目已更新", entry };
}
async function deleteEntry(params) {
    const removed = await (0, worldbook_service_js_1.deleteWorldBookEntry)(String(params.id || ""));
    return { success: true, message: `条目已删除: ${removed.name}` };
}
async function toggleEntry(params) {
    const entry = await (0, worldbook_service_js_1.toggleWorldBookEntry)(String(params.id || ""));
    return {
        success: true,
        message: `${entry.name} 已${entry.enabled ? "启用" : "禁用"}`,
        entry
    };
}
async function importEntries(params) {
    const result = await (0, worldbook_service_js_1.importWorldBookEntries)(params);
    return {
        success: true,
        message: result.warning_count > 0
            ? `已导入 ${result.imported_count} 个条目，并产生 ${result.warning_count} 条兼容性提示`
            : `已导入 ${result.imported_count} 个条目`,
        result
    };
}
async function listCharacterCardsProxy() {
    const cards = await (0, worldbook_service_js_1.listWorldBookCharacterCards)();
    return { success: true, totalCount: cards.length, cards };
}
exports.list_entries = (params) => wrap(listEntries, params);
exports.get_entry = (params) => wrap(getEntry, params);
exports.create_entry = (params) => wrap(createEntry, params);
exports.update_entry = (params) => wrap(updateEntry, params);
exports.delete_entry = (params) => wrap(deleteEntry, params);
exports.toggle_entry = (params) => wrap(toggleEntry, params);
exports.import_entries = (params) => wrap(importEntries, params);
exports.list_character_cards_proxy = (params) => wrap(listCharacterCardsProxy, params);

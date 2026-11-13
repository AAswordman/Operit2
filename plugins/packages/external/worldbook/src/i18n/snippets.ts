/** Describes one localized variable template without changing user-authored content. */
export interface WorldBookContentSnippet {
  id: string;
  label: string;
  description: string;
  content: string;
}

export const WORLDBOOK_SNIPPETS_ZH_CN: WorldBookContentSnippet[] = [
  {
    "id": "format_message_variable",
    "label": "format_message_variable",
    "description": "格式化输出当前消息/会话局部变量对象",
    "content": "{{format_message_variable::stat_data}}"
  },
  {
    "id": "format_chat_variable",
    "label": "format_chat_variable",
    "description": "兼容 chat 命名，效果与 message 作用域一致",
    "content": "{{format_chat_variable::stat_data}}"
  },
  {
    "id": "format_character_variable",
    "label": "format_character_variable",
    "description": "格式化输出角色变量对象",
    "content": "{{format_character_variable::stat_data}}"
  },
  {
    "id": "format_global_variable",
    "label": "format_global_variable",
    "description": "格式化输出全局变量对象",
    "content": "{{format_global_variable::world_state}}"
  },
  {
    "id": "get_message_variable",
    "label": "get_message_variable",
    "description": "读取当前消息/会话局部变量中的单个路径",
    "content": "{{get_message_variable::stat_data.系统.积分}}"
  },
  {
    "id": "get_chat_variable",
    "label": "get_chat_variable",
    "description": "兼容 chat 命名，效果与 message 作用域一致",
    "content": "{{get_chat_variable::stat_data.系统.积分}}"
  },
  {
    "id": "get_character_variable",
    "label": "get_character_variable",
    "description": "读取角色变量中的单个路径",
    "content": "{{get_character_variable::stat_data.关系.好感}}"
  },
  {
    "id": "get_global_variable",
    "label": "get_global_variable",
    "description": "读取全局变量中的单个路径",
    "content": "{{get_global_variable::world_state.rules.version}}"
  },
  {
    "id": "getvar_macro",
    "label": "getvar",
    "description": "本地变量路径读取简写",
    "content": "<%= getvar('stat_data.心理.羁绊值') %>"
  },
  {
    "id": "getglobalvar_macro",
    "label": "getglobalvar",
    "description": "全局变量路径读取简写",
    "content": "<%= getglobalvar('world_state.rules.version') %>"
  },
  {
    "id": "getvar_braces",
    "label": "{{getvar::...}}",
    "description": "本地变量路径读取的花括号版本",
    "content": "{{getvar::stat_data.心理.羁绊值}}"
  },
  {
    "id": "getglobalvar_braces",
    "label": "{{getglobalvar::...}}",
    "description": "全局变量路径读取的花括号版本",
    "content": "{{getglobalvar::world_state.rules.version}}"
  },
  {
    "id": "dot_shorthand",
    "label": "{{.变量}}",
    "description": "本地变量顶层键简写，仅支持单层变量名",
    "content": "{{.stat_data}}"
  },
  {
    "id": "dollar_shorthand",
    "label": "{{$变量}}",
    "description": "全局变量顶层键简写，仅支持单层变量名",
    "content": "{{$world_state}}"
  },
  {
    "id": "init_var_block",
    "label": "[InitVar]初始化",
    "description": "在条目中声明变量初始化区块",
    "content": "[InitVar]初始化\n```json\n{\n  \"stat_data\": {\n    \"系统\": {\n      \"积分\": 0\n    }\n  }\n}\n```"
  },
  {
    "id": "update_variable_patch",
    "label": "UpdateVariable / JSONPatch",
    "description": "在助手回复后写回变量的 JSONPatch 模板",
    "content": "<UpdateVariable>\n<Analysis>描述本次变量变化原因</Analysis>\n<JSONPatch>\n```json\n[\n  {\n    \"op\": \"replace\",\n    \"path\": \"/stat_data/系统/积分\",\n    \"value\": 1\n  }\n]\n```\n</JSONPatch>\n</UpdateVariable>"
  },
  {
    "id": "jsonpatch_only",
    "label": "JSONPatch",
    "description": "仅输出 JSONPatch 块，运行时同样会识别",
    "content": "<JSONPatch>\n```json\n[\n  {\n    \"op\": \"replace\",\n    \"path\": \"/stat_data/系统/积分\",\n    \"value\": 1\n  }\n]\n```\n</JSONPatch>"
  }
];

export const WORLDBOOK_SNIPPETS_EN_US: WorldBookContentSnippet[] = [
  {
    "id": "format_message_variable",
    "label": "format_message_variable",
    "description": "Format the current message/conversation local variable object",
    "content": "{{format_message_variable::stat_data}}"
  },
  {
    "id": "format_chat_variable",
    "label": "format_chat_variable",
    "description": "Chat alias with the same scope as message variables",
    "content": "{{format_chat_variable::stat_data}}"
  },
  {
    "id": "format_character_variable",
    "label": "format_character_variable",
    "description": "Format the character variable object",
    "content": "{{format_character_variable::stat_data}}"
  },
  {
    "id": "format_global_variable",
    "label": "format_global_variable",
    "description": "Format the global variable object",
    "content": "{{format_global_variable::world_state}}"
  },
  {
    "id": "get_message_variable",
    "label": "get_message_variable",
    "description": "Read a path from current message/conversation local variables",
    "content": "{{get_message_variable::stat_data.system.points}}"
  },
  {
    "id": "get_chat_variable",
    "label": "get_chat_variable",
    "description": "Chat alias with the same scope as message variables",
    "content": "{{get_chat_variable::stat_data.system.points}}"
  },
  {
    "id": "get_character_variable",
    "label": "get_character_variable",
    "description": "Read a path from character variables",
    "content": "{{get_character_variable::stat_data.relationships.affinity}}"
  },
  {
    "id": "get_global_variable",
    "label": "get_global_variable",
    "description": "Read a path from global variables",
    "content": "{{get_global_variable::world_state.rules.version}}"
  },
  {
    "id": "getvar_macro",
    "label": "getvar",
    "description": "Shorthand for reading a local variable path",
    "content": "<%= getvar('stat_data.feelings.bond') %>"
  },
  {
    "id": "getglobalvar_macro",
    "label": "getglobalvar",
    "description": "Shorthand for reading a global variable path",
    "content": "<%= getglobalvar('world_state.rules.version') %>"
  },
  {
    "id": "getvar_braces",
    "label": "{{getvar::...}}",
    "description": "Brace syntax for reading a local variable path",
    "content": "{{getvar::stat_data.feelings.bond}}"
  },
  {
    "id": "getglobalvar_braces",
    "label": "{{getglobalvar::...}}",
    "description": "Brace syntax for reading a global variable path",
    "content": "{{getglobalvar::world_state.rules.version}}"
  },
  {
    "id": "dot_shorthand",
    "label": "{{.variable}}",
    "description": "Shorthand for a top-level local variable key; single-level names only",
    "content": "{{.stat_data}}"
  },
  {
    "id": "dollar_shorthand",
    "label": "{{$variable}}",
    "description": "Shorthand for a top-level global variable key; single-level names only",
    "content": "{{$world_state}}"
  },
  {
    "id": "init_var_block",
    "label": "[InitVar] Initialization",
    "description": "Declare a variable initialization block in an entry",
    "content": "[InitVar] Initialization\n```json\n{\n  \"stat_data\": {\n    \"system\": {\n      \"points\": 0\n    }\n  }\n}\n```"
  },
  {
    "id": "update_variable_patch",
    "label": "UpdateVariable / JSONPatch",
    "description": "JSONPatch template for updating variables after an assistant reply",
    "content": "<UpdateVariable>\n<Analysis>Describe why the variables changed</Analysis>\n<JSONPatch>\n```json\n[\n  {\n    \"op\": \"replace\",\n    \"path\": \"/stat_data/system/points\",\n    \"value\": 1\n  }\n]\n```\n</JSONPatch>\n</UpdateVariable>"
  },
  {
    "id": "jsonpatch_only",
    "label": "JSONPatch",
    "description": "Output a JSONPatch block directly; the runtime recognizes it too",
    "content": "<JSONPatch>\n```json\n[\n  {\n    \"op\": \"replace\",\n    \"path\": \"/stat_data/system/points\",\n    \"value\": 1\n  }\n]\n```\n</JSONPatch>"
  }
];

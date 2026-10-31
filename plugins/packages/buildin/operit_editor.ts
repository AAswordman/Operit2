/* METADATA
{
    "name": "operit_editor",
    "display_name": {
        "zh": "Operit 平台编辑器",
        "en": "Operit Platform Editor"
    },
    "description": {
        "zh": "通过真实的 core command 执行入口管理 Operit2 的包、Skill、MCP、模型、偏好设置、聊天与工作区，返回实际执行结果。",
        "en": "Manage Operit2 packages, skills, MCP, models, preferences, chats, and workspaces through the real core command executor."
    },
    "enabledByDefault": false,
    "category": "System",
    "tools": [
        {
            "name": "operit_editor",
            "display": {"label": {"zh": "管理 Operit 配置", "en": "Manage Operit settings"}, "icon": "edit"},
            "description": {
                "zh": "执行 Operit2 平台编辑命令，直接调用 Tools.SoftwareSettings.exec(args)，与系统 execute_cli_command 使用同一执行入口。args=[] 获取实际命令帮助；例如 [\"package\",\"list\"]、[\"package\",\"enable\",\"包名\"]、[\"skill\",\"show\",\"PackageBuilder\"]、[\"prefs\",\"show\"]。先读取真实状态，变更用户配置或删除资源前取得用户确认，修改后重新读取状态核验。不执行 shell 命令或任意脚本片段，不接受自然语言 query。",
                "en": "Execute Operit2 editing commands via Tools.SoftwareSettings.exec(args), using the same executor as execute_cli_command. Use [] for live help, or e.g. [\"package\",\"list\"], [\"package\",\"enable\",\"name\"], [\"prefs\",\"show\"]. Read state first, obtain user confirmation before mutations or deletion, and verify state afterward. Not a shell or arbitrary script evaluator; natural-language query is not supported."
            },
            "parameters": [
                {
                    "name": "args",
                    "description": {
                        "zh": "必填，CLI 字符串参数数组（也接受数组的 JSON 字符串），不含 operit2 可执行文件名。空数组获取帮助，保留每个参数的原始内容，例如 [\"package\",\"exec\",\"包名:工具名\",\"{\\\"name\\\":\\\"世界\\\"}\"]。",
                        "en": "Required CLI string argument array (a JSON-encoded array is also accepted), without the operit2 executable name. An empty array requests help. Each argument is forwarded unchanged, including JSON tool payloads."
                    },
                    "type": "array",
                    "required": true
                }
            ]
        }
    ]
}*/

/// <reference path="../../types/index.d.ts" />

type OperitEditorParams = {
    args: string[] | string;
};

/** Validates structured CLI arguments without splitting or rewriting their contents. */
function commandArgs(value: unknown): string[] {
    let args = value;
    if (typeof args === "string") {
        try {
            args = JSON.parse(args);
        } catch {
            throw new Error('operit_editor: args 必须是字符串数组或其 JSON 字符串，例如 ["package","list"]。');
        }
    }
    if (!Array.isArray(args)) {
        throw new Error('operit_editor: 必须提供 args 字符串数组；用 [] 获取命令帮助，不接受自然语言 query。');
    }
    for (let index = 0; index < args.length; index++) {
        if (typeof args[index] !== "string") {
            throw new Error(`operit_editor: args[${index}] 必须是字符串。`);
        }
    }
    return args;
}

/** Executes the requested command through the host tool and preserves its result or failure. */
async function operit_editor(params: OperitEditorParams) {
    if (!params || typeof params !== "object" || Array.isArray(params)) {
        throw new Error('operit_editor: 必须提供参数对象，例如 {"args":["package","list"]}。');
    }
    if (Object.prototype.hasOwnProperty.call(params, "query")) {
        throw new Error('operit_editor: query 已移除；请使用 args 字符串数组执行实际命令，用 [] 获取命令帮助。');
    }
    return await Tools.SoftwareSettings.exec(commandArgs(params.args));
}

exports.operit_editor = operit_editor;
exports.main = operit_editor;

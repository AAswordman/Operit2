/* METADATA
{
    "name": "operit_editor",
    "display_name": {
        "zh": "Operit 平台编辑器",
        "en": "Operit Platform Editor"
    },
    "description": {
        "zh": "Operit2 平台编辑与排查手册。面向当前版本 core command，不复刻旧版 soft setting 工具面。",
        "en": "Operit2 platform editing and troubleshooting guide for the current core command surface."
    },
    "enabledByDefault": false,
    "category": "System",
    "tools": [
        {
            "name": "operit_editor",
            "description": {
                "zh": "读取 Operit2 平台编辑手册。真正的配置、包、Skill、MCP、模型、聊天、工作区操作请直接调用系统工具 execute_cli_command。",
                "en": "Read the Operit2 platform editing guide. Use the system execute_cli_command tool for package, skill, MCP, model, chat, and workspace operations."
            },
            "parameters": [
                {
                    "name": "query",
                    "description": {
                        "zh": "可选，说明本次要编辑或排查的目标。",
                        "en": "Optional editing or troubleshooting target."
                    },
                    "type": "string",
                    "required": false
                }
            ]
        }
    ]
}*/

type OperitEditorParams = {
    query?: string;
};

const OPERIT_EDITOR_GUIDE = `
# Operit2 平台编辑器

这个包只提供当前 Operit2 平台编辑手册。执行动作使用系统工具 execute_cli_command，参数是 CLI 字符串数组。

常用入口：

- 查看总帮助：["package", "help"] 之外的总入口可用空数组或具体一级命令查看。
- 包管理：["package", "list"]、["package", "more"]、["package", "load", "<name>"]、["package", "show", "<name>"]、["package", "enable", "<name>"]、["package", "disable", "<name>"]、["package", "use", "<name>"]、["package", "exec", "<package:tool>", "<params-json>"]。
- Skill：["skill", "list"]、["skill", "show", "<name>"]、["skill", "visible", "<name>", "true"]、["skill", "visible", "<name>", "false"]、["skill", "errors"]。
- MCP：["mcp", "dir"]、["mcp", "list"]、["mcp", "show", "<name>"]、["mcp", "enable", "<name>"]、["mcp", "disable", "<name>"]、["mcp", "start", "<name>"]、["mcp", "tools", "<name>"]。
- 模型：["model", "list"]、["model", "show", "<id>"]、["model", "function-list"]、["model", "function-show", "<type>"]、["model", "function-set", "<type>", "<provider-id>", "<model-id>"]。
- 偏好设置：["prefs", "show"]、["prefs", "thinking"]、["prefs", "stream"]、["prefs", "media-history"]、["prefs", "mcp-timeout"]。
- 日志：["log", "show"]、["log", "package"]、["log", "path"]、["log", "clear"]。
- 工具：["tool", "list"]、["tool", "show", "<name>"]、["tool", "exec", "<name>", "<params-json>"]。
- 工作区：["workspace", "commands"]、["workspace", "run", "<command-id>"]、["workspace", "list"]、["workspace", "bind-default"]。

插件创作约定：

- 使用 PackageBuilder skill 中随包携带的当前版本类型定义。
- 开发目录固定在手机下载/Operit/dev_package/<package-id>。
- 包 id 在首次确定后保持不变。
- 使用终端完成 TypeScript/JavaScript 开发、编译、安装和测试。
- 安装与调试走当前版本 package core command 和系统工具，不使用旧版 SoftwareSettings 的包开关、脚本直跑、模型设置枚举接口。

包系统说明：

- 内置包来自应用内置资源。
- 准内置包来自应用内资源中的 external 候选，查看用 ["package", "more"]，加入加载列表用 ["package", "load", "<name>"]。
- 当前会话调用某个包前，用 ["package", "use", "<name>"] 让 runtime 激活它。
- ToolPkg 子包由包系统解析和展示，不手写另一套识别逻辑。

执行原则：

- 先用对应 core command 查看真实状态，再执行修改命令。
- 需要变更用户配置、启停包、启停 MCP、删除资源时，先向用户确认。
- 不从云端拉取 PackageBuilder 类型；使用当前软件随包携带的类型。
`.trim();

async function operit_editor(params: OperitEditorParams = {}) {
    const query = params.query?.trim();
    if (!query) {
        return OPERIT_EDITOR_GUIDE;
    }
    return `目标：${query}\n\n${OPERIT_EDITOR_GUIDE}`;
}

exports.operit_editor = operit_editor;
exports.main = operit_editor;

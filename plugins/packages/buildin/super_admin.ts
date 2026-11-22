type SuperAdminParams = Record<string, unknown>;

type TerminalParams = SuperAdminParams & {
    command?: string;
    profileId?: string;
};

type TerminalSessionParams = SuperAdminParams & {
    sessionId: string;
};

type TerminalInputParams = TerminalSessionParams & {
    input?: string;
    control?: string;
};

type PersistedTerminalOutput = {
    command: string;
    output: string;
    exitCode: unknown;
    sessionId: unknown;
    timedOut: boolean;
    context_preserved: boolean;
    output_saved_to: string;
    output_chars: number;
    operit_clean_on_exit_dir: string;
    hint: string;
    platform: string;
    terminal: string;
    terminalType: string;
    terminalEnvironment?: unknown;
    timeoutMsUsed?: number;
};

/* METADATA
{
    "api_version": "2.0.0",
    "name": "super_admin",
    "display_name": {
        "zh": "超级管理员",
        "en": "Super Admin"
    },
    "description": {
        "zh": "提供终端命令和会话控制。每次获取会话必须进入当前文件目录 `.`：已绑定的主工作区，或未绑定时这组父子对话固定的产出目录；进入失败则终止，不在主目录继续执行。附件和产出直接使用提供的相对路径，无需翻根目录、主目录或全盘查找。跨挂载访问时，终端使用 Host 解析的路径，不直接套用 VFS 绝对路径。请按实际权限操作。",
        "en": "Runs terminal commands and manages sessions. Each session acquisition must enter the current file directory `.`: the bound primary workspace, or the conversation family's persistent output area when unbound. Failure stops execution instead of continuing in the home directory. Use supplied relative attachment and output paths directly without root/home scans or global searches. For other mounts, use the Host-resolved terminal path instead of a VFS absolute path. Respect actual permissions."
    },
    "enabledByDefault": true,
    "category": "System",
    "tools": [
        {
            "name": "list_terminals",
            "display": {
                "label": {
                    "zh": "列出终端配置",
                    "en": "List terminal profiles"
                },
                "icon": "terminal"
            },
            "description": {
                "zh": "查询用户配置的全部终端、可用状态和默认配置 ID。执行前可用此工具选择具体配置；不同版本或不同发行版有不同的 profileId。",
                "en": "List saved terminal profiles, availability and the default profile ID. Different versions or distributions have independent profile IDs."
            },
            "parameters": []
        },
        {
            "name": "terminal",
            "display": {
                "label": {
                    "zh": "运行终端命令",
                    "en": "Run terminal command"
                },
                "icon": "terminal",
                "targetParameter": "command"
            },
            "description": {
                "zh": "在用户配置的终端中执行命令并持续返回输出。省略 profileId 使用默认配置；先用 list_terminals 查询具体配置，不要按平台猜测 shell。包执行默认等待20秒，随后返回 execution_id，命令继续运行；使用 package_wait 读取后续输出和完成结果。终端开始事件提供 sessionId，可用于 get_screen、input、close。",
                "en": "Execute in a user-configured terminal and stream output until completion. Omit profileId for the default or select a profile from list_terminals. Package execution waits up to 20 seconds, then returns execution_id while the command continues. Use package_wait for further output and completion. The terminal start event supplies sessionId for get_screen, input and close."
            },
            "parameters": [
                {
                    "name": "command",
                    "description": {
                        "zh": "与所选终端语法匹配的命令",
                        "en": "Command matching the selected terminal syntax."
                    },
                    "type": "string",
                    "required": true
                },
                {
                    "name": "profileId",
                    "description": {
                        "zh": "list_terminals 返回的配置 ID；省略使用默认配置。",
                        "en": "Profile ID from list_terminals; omission uses the default."
                    },
                    "type": "string",
                    "required": false
                }
            ]
        },
        {
            "name": "close",
            "display": {
                "label": {
                    "zh": "关闭终端会话",
                    "en": "Close terminal session"
                },
                "icon": "terminal",
                "targetParameter": "sessionId"
            },
            "description": {
                "zh": "关闭指定终端会话并释放资源，终止该会话中正在执行的命令。关闭后该会话 ID 不再可用；再次运行终端工具会创建新会话，原会话上下文不会保留。",
                "en": "Close the specified terminal session, release its resources, and terminate its running command. The closed session ID is no longer usable. Running a terminal tool again creates a new session without the previous context."
            },
            "parameters": [
                {
                    "name": "sessionId",
                    "description": {
                        "zh": "要关闭的终端会话ID。",
                        "en": "ID of the terminal session to close."
                    },
                    "type": "string",
                    "required": true
                }
            ]
        },
        {
            "name": "get_screen",
            "display": {
                "label": {
                    "zh": "读取终端屏幕",
                    "en": "Read terminal screen"
                },
                "icon": "terminal",
                "targetParameter": "sessionId"
            },
            "description": {
                "zh": "获取当前终端会话可见屏幕内容（仅一屏，不包含历史滚动缓冲）。",
                "en": "Get the current visible screen content for the active terminal session (single screen only, no scrollback history)."
            },
            "parameters": [
                {
                    "name": "sessionId",
                    "description": {
                        "zh": "目标终端会话ID。",
                        "en": "Target terminal session ID."
                    },
                    "type": "string",
                    "required": true
                }
            ]
        },
        {
            "name": "input",
            "display": {
                "label": {
                    "zh": "发送终端输入",
                    "en": "Send terminal input"
                },
                "icon": "terminal",
                "targetParameter": "sessionId"
            },
            "description": {
                "zh": "向当前终端会话写入输入。input 与 control 至少传一个。常见用法：先写 input，再写 control=enter 提交；control=ctrl 且 input=c 可发送 Ctrl+C。",
                "en": "Write input to the active terminal session. Provide at least one of input or control. Typical usage: send input first, then control=enter to submit; use control=ctrl with input=c for Ctrl+C."
            },
            "parameters": [
                {
                    "name": "sessionId",
                    "description": {
                        "zh": "目标终端会话ID。",
                        "en": "Target terminal session ID."
                    },
                    "type": "string",
                    "required": true
                },
                {
                    "name": "input",
                    "description": {
                        "zh": "写入终端的文本",
                        "en": "Text to write to terminal."
                    },
                    "type": "string",
                    "required": false
                },
                {
                    "name": "control",
                    "description": {
                        "zh": "控制键，例如 enter / tab / esc / ctrl",
                        "en": "Control key, e.g. enter / tab / esc / ctrl."
                    },
                    "type": "string",
                    "required": false
                }
            ]
        }
    ]
}*/

/**
 * Creates the super admin terminal tool exports.
 */
const superAdmin = (function () {
    const MAX_INLINE_TERMINAL_OUTPUT_CHARS = 12000;
    const DEFAULT_TERMINAL_SESSION_PREFIX = "super_admin_default_session";

    /**
     * Returns a stable foreground session name scoped to the chat and interpreter type.
     */
    function getDefaultTerminalSessionName(profileId: string): string {
        return `${DEFAULT_TERMINAL_SESSION_PREFIX}_${profileId}_${getChatId()}`;
    }

    /**
     * Saves oversized terminal output to a temporary file and returns its summary.
     */
    async function persistTerminalOutputIfTooLong(command: string, result: any): Promise<PersistedTerminalOutput | null> {
        const outputStr = typeof result?.output === "string"
            ? result.output
            : String(result?.output ?? "");
        if (outputStr.length <= MAX_INLINE_TERMINAL_OUTPUT_CHARS) {
            return null;
        }
        await Tools.Files.mkdir(OPERIT_CLEAN_ON_EXIT_DIR, true);
        const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
        const rand = Math.floor(Math.random() * 1000000);
        const filePath = `${OPERIT_CLEAN_ON_EXIT_DIR}/terminal_output_${timestamp}_${rand}.log`;
        await Tools.Files.write(filePath, outputStr, false);
        return {
            command,
            output: "(saved_to_file)",
            exitCode: result?.exitCode,
            sessionId: result?.sessionId,
            platform: result.platform,
            terminal: result.terminal,
            terminalType: result.terminalType,
            timedOut: result?.timedOut === true,
            context_preserved: true,
            output_saved_to: filePath,
            output_chars: outputStr.length,
            operit_clean_on_exit_dir: OPERIT_CLEAN_ON_EXIT_DIR,
            hint: "Output is large and saved to file. Use read_file_part or grep_code to inspect it.",
        };
    }
    /**
     * Executes a terminal command and returns output plus terminal environment details.
     * @param command - Command to execute.
     */
    async function terminal(params: TerminalParams) {
        try {
            if (!params.command) {
                throw new Error("命令不能为空");
            }
            const command = params.command;
            const terminalInfo = await Tools.System.terminal.info();
            const profileId = params.profileId ?? terminalInfo.defaultProfileId;
            const profile = terminalInfo.profiles.find(entry => entry.id === profileId);
            if (!profile) throw new Error(`Terminal profile is not registered: ${profileId}`);
            if (!profile.available) throw new Error(`Terminal profile is unavailable: ${profileId}`);
            console.log(`执行终端命令: ${command}`);
            const stream = Tools.System.terminal.runStream({
                sessionName: getDefaultTerminalSessionName(profileId),
                profileId,
                command
            });
            for await (const event of stream) {
                if (event.type !== "completed") {
                    sendIntermediateResult(event);
                    continue;
                }
                const result = event.result;
                if (result.timedOut) throw new Error("Terminal stream ended before command completion");
                const persistedResult = await persistTerminalOutputIfTooLong(command, result);
                return {
                    ...(persistedResult || result), command,
                    background: false, commandRunning: false, started: true, status: "completed",
                    terminalEnvironment: { ...terminalInfo, platform: result.platform, terminal: result.terminal, terminalType: result.terminalType },
                    context_preserved: true, message: "命令已完成。"
                };
            }
            throw new Error("Terminal stream ended without a completion result");
        }
        catch (error) {
            console.error(`[terminal] 错误: ${error.message}`);
            console.error(error.stack);
            throw error;
        }
    }

    /** Lists user-managed profiles and their default without platform assumptions. */
    async function list_terminals() {
        return Tools.System.terminal.info();
    }

    /** Closes the specified terminal session through the host terminal API. */
    async function close(params: TerminalSessionParams) {
        return Tools.System.terminal.close(params.sessionId);
    }
    /**
     * Gets the visible screen content for a terminal session.
     * @param sessionId - Target session ID.
     */
    async function get_screen(params: TerminalSessionParams) {
        try {
            const sessionId = params.sessionId;
            const result = await Tools.System.terminal.screen(sessionId);
            return {
                sessionId: result.sessionId,
                terminalType: result.terminalType,
                rows: result.rows,
                cols: result.cols,
                content: result.content,
                commandRunning: result.commandRunning
            };
        }
        catch (error) {
            console.error(`[get_screen] 错误: ${error.message}`);
            console.error(error.stack);
            throw error;
        }
    }
    /**
     * Writes input or a control key to a terminal session.
     * @param sessionId - Target session ID.
     * @param input - Text input.
     * @param control - Control key.
     */
    async function input(params: TerminalInputParams) {
        try {
            if (params.input === undefined && params.control === undefined) {
                throw new Error("input和control至少需要提供一个");
            }
            const sessionId = params.sessionId;
            const result = await Tools.System.terminal.input(sessionId, {
                input: params.input,
                control: params.control
            });
            return {
                sessionId: sessionId,
                input: params.input,
                control: params.control,
                result
            };
        }
        catch (error) {
            console.error(`[input] 错误: ${error.message}`);
            console.error(error.stack);
            throw error;
        }
    }
    return {
        terminal,
        list_terminals,
        close,
        get_screen,
        input
    };
})();
// 逐个导出
exports.terminal = superAdmin.terminal;
exports.list_terminals = superAdmin.list_terminals;
exports.close = superAdmin.close;
exports.get_screen = superAdmin.get_screen;
exports.input = superAdmin.input;

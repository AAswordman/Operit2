type SuperAdminParams = Record<string, unknown>;

type TerminalParams = SuperAdminParams & {
    command?: string;
    background?: string;
    timeoutMs?: string | number;
};

type TerminalSessionParams = SuperAdminParams & {
    sessionId: string;
};

type TerminalWaitParams = TerminalSessionParams & {
    timeoutMs?: string | number;
};

type TerminalInputParams = TerminalSessionParams & {
    input?: string;
    control?: string;
};

type TerminalCommandType = "powershell" | "bash" | "shell";

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
    "name": "super_admin",

    "display_name": {
        "zh": "超级管理员",
        "en": "Super Admin"
    },
    "description": { "zh": "超级管理员工具集，提供终端命令和会话控制的高级功能。", "en": "Super admin toolkit providing advanced terminal command and session control capabilities." },
    "enabledByDefault": true,
    "category": "System",
    "tools": [
        {
            "name": "terminal_wait",
            "description": { "zh": "等待同一终端会话中的上一条命令结束。它读取会话自身的 commandRunning 状态，所以命令一结束就返回，而不是固定睡眠。典型用法：先用 background=\"true\" 启动耗时命令并拿到 sessionId，做完其他工具调用后再调用本工具。超时只表示这次没等到：它不会取消正在跑的命令，也不动终端会话，命令会继续跑；再调一次即可继续等。", "en": "Wait until the previous command in the same terminal session finishes. It reads the session's own commandRunning state, so it returns as soon as the command actually ends instead of sleeping for a fixed time. Typical usage: start the slow command with background=\"true\", do other tool calls, then call this. A timeout only means the command had not finished yet: it does not cancel the running command and does not touch the terminal session, so call it again to keep waiting." },
            "parameters": [
                {
                    "name": "sessionId",
                    "description": { "zh": "目标终端会话ID。", "en": "Target terminal session ID." },
                    "type": "string",
                    "required": true
                },
                {
                    "name": "timeoutMs",
                    "description": { "zh": "可选超时（毫秒，最低3000ms）。未传时默认50000ms（50秒，低于 JS 运行时单次 60 秒预算，所以会正常返回而不是被打断）。命令没跑完就再调一次。", "en": "Optional timeout (ms, minimum 3000ms). Defaults to 50000ms (50s, below the 60s step budget of the JavaScript runtime, so it returns normally instead of being interrupted). Call it again if the command is still running." },
                    "type": "string",
                    "required": false
                }
            ]
        },
        {
            "name": "get_screen",
            "description": { "zh": "获取当前终端会话可见屏幕内容（仅一屏，不包含历史滚动缓冲）。", "en": "Get the current visible screen content for the active terminal session (single screen only, no scrollback history)." },
            "parameters": [
                {
                    "name": "sessionId",
                    "description": { "zh": "目标终端会话ID。", "en": "Target terminal session ID." },
                    "type": "string",
                    "required": true
                }
            ]
        },
        {
            "name": "input",
            "description": { "zh": "向当前终端会话写入输入。input 与 control 至少传一个。常见用法：先写 input，再写 control=enter 提交；control=ctrl 且 input=c 可发送 Ctrl+C。", "en": "Write input to the active terminal session. Provide at least one of input or control. Typical usage: send input first, then control=enter to submit; use control=ctrl with input=c for Ctrl+C." },
            "parameters": [
                {
                    "name": "sessionId",
                    "description": { "zh": "目标终端会话ID。", "en": "Target terminal session ID." },
                    "type": "string",
                    "required": true
                },
                {
                    "name": "input",
                    "description": { "zh": "写入终端的文本", "en": "Text to write to terminal." },
                    "type": "string",
                    "required": false
                },
                {
                    "name": "control",
                    "description": { "zh": "控制键，例如 enter / tab / esc / ctrl", "en": "Control key, e.g. enter / tab / esc / ctrl." },
                    "type": "string",
                    "required": false
                }
            ]
        }
    ],
    "states": [
        {
            "id": "windows",
            "condition": "platform.windows",
            "inheritTools": true,
            "tools": [
                {
                    "name": "powershell",
                    "description": { "zh": "在 Windows PowerShell 终端会话中执行命令并收集输出结果。会话按当前对话维护，上下文连贯。强烈建议每次都显式传 timeoutMs，避免命令卡住。前台未传 timeoutMs 时默认15秒；background=true 时不使用该默认超时。命令超时时会取消当前命令并保留终端会话。", "en": "Execute commands in a Windows PowerShell terminal session and collect output. The session is maintained per chat and preserves context. Strongly recommend explicitly passing timeoutMs every time to avoid hangs. Foreground mode defaults to 15s timeout when timeoutMs is omitted; background=true does not use this default timeout. When a command times out, the current command is cancelled and the terminal session is kept." },
                    "parameters": [
                        {
                            "name": "command",
                            "description": { "zh": "要执行的 PowerShell 命令", "en": "PowerShell command to execute." },
                            "type": "string",
                            "required": true
                        },
                        {
                            "name": "background",
                            "description": { "zh": "是否在后台运行命令,\"true\" 表示后台执行并立即返回,适合启动服务器等长时间运行的任务（AI 不会收到该命令的输出结果），\"false\" 或未提供则前台执行并等待并返回命令结果", "en": "Run command in background. 'true' runs in background and returns immediately (good for long-running tasks like servers; AI will not receive output). 'false' or omitted runs in foreground and returns the command result." },
                            "type": "string",
                            "required": false
                        },
                        {
                            "name": "timeoutMs",
                            "description": { "zh": "可选超时（毫秒，最低3000ms）。强烈建议显式传入；未传时前台默认15000ms，background=true时不使用默认超时。", "en": "Optional timeout (ms, minimum 3000ms). Strongly recommended to pass explicitly; if omitted, foreground defaults to 15000ms, and background=true does not use the default timeout." },
                            "type": "string",
                            "required": false
                        }
                    ]
                },
                {
                    "name": "bash",
                    "description": { "zh": "在 Windows Git Bash 终端会话中执行命令并收集输出结果。会话按当前对话维护，上下文连贯。强烈建议每次都显式传 timeoutMs，避免命令卡住。前台未传 timeoutMs 时默认15秒；background=true 时不使用该默认超时。命令超时时会取消当前命令并保留终端会话。", "en": "Execute commands in a Windows Git Bash terminal session and collect output. The session is maintained per chat and preserves context. Strongly recommend explicitly passing timeoutMs every time to avoid hangs. Foreground mode defaults to 15s timeout when timeoutMs is omitted; background=true does not use this default timeout. When a command times out, the current command is cancelled and the terminal session is kept." },
                    "parameters": [
                        {
                            "name": "command",
                            "description": { "zh": "要执行的 Bash 命令", "en": "Bash command to execute." },
                            "type": "string",
                            "required": true
                        },
                        {
                            "name": "background",
                            "description": { "zh": "是否在后台运行命令,\"true\" 表示后台执行并立即返回,适合启动服务器等长时间运行的任务（AI 不会收到该命令的输出结果），\"false\" 或未提供则前台执行并等待并返回命令结果", "en": "Run command in background. 'true' runs in background and returns immediately (good for long-running tasks like servers; AI will not receive output). 'false' or omitted runs in foreground and returns the command result." },
                            "type": "string",
                            "required": false
                        },
                        {
                            "name": "timeoutMs",
                            "description": { "zh": "可选超时（毫秒，最低3000ms）。强烈建议显式传入；未传时前台默认15000ms，background=true时不使用默认超时。", "en": "Optional timeout (ms, minimum 3000ms). Strongly recommended to pass explicitly; if omitted, foreground defaults to 15000ms, and background=true does not use the default timeout." },
                            "type": "string",
                            "required": false
                        }
                    ]
                }
            ]
        },
        {
            "id": "linux",
            "condition": "platform.linux",
            "inheritTools": true,
            "tools": [
                {
                    "name": "bash",
                    "description": { "zh": "在 Bash 终端会话中执行命令并收集输出结果。会话按当前对话维护，上下文连贯。强烈建议每次都显式传 timeoutMs，避免命令卡住。前台未传 timeoutMs 时默认15秒；background=true 时不使用该默认超时。命令超时时会取消当前命令并保留终端会话。", "en": "Execute commands in a Bash terminal session and collect output. The session is maintained per chat and preserves context. Strongly recommend explicitly passing timeoutMs every time to avoid hangs. Foreground mode defaults to 15s timeout when timeoutMs is omitted; background=true does not use this default timeout. When a command times out, the current command is cancelled and the terminal session is kept." },
                    "parameters": [
                        {
                            "name": "command",
                            "description": { "zh": "要执行的 Bash 命令", "en": "Bash command to execute." },
                            "type": "string",
                            "required": true
                        },
                        {
                            "name": "background",
                            "description": { "zh": "是否在后台运行命令,\"true\" 表示后台执行并立即返回,适合启动服务器等长时间运行的任务（AI 不会收到该命令的输出结果），\"false\" 或未提供则前台执行并等待并返回命令结果", "en": "Run command in background. 'true' runs in background and returns immediately (good for long-running tasks like servers; AI will not receive output). 'false' or omitted runs in foreground and returns the command result." },
                            "type": "string",
                            "required": false
                        },
                        {
                            "name": "timeoutMs",
                            "description": { "zh": "可选超时（毫秒，最低3000ms）。强烈建议显式传入；未传时前台默认15000ms，background=true时不使用默认超时。", "en": "Optional timeout (ms, minimum 3000ms). Strongly recommended to pass explicitly; if omitted, foreground defaults to 15000ms, and background=true does not use the default timeout." },
                            "type": "string",
                            "required": false
                        }
                    ]
                }
            ]
        },
        {
            "id": "macos",
            "condition": "platform.macos",
            "inheritTools": true,
            "tools": [
                {
                    "name": "bash",
                    "description": { "zh": "在 macOS Bash 终端会话中执行命令并收集输出结果。会话按当前对话维护，上下文连贯。强烈建议每次都显式传 timeoutMs，避免命令卡住。前台未传 timeoutMs 时默认15秒；background=true 时不使用该默认超时。命令超时时会取消当前命令并保留终端会话。", "en": "Execute commands in a macOS Bash terminal session and collect output. The session is maintained per chat and preserves context. Strongly recommend explicitly passing timeoutMs every time to avoid hangs. Foreground mode defaults to 15s timeout when timeoutMs is omitted; background=true does not use this default timeout. When a command times out, the current command is cancelled and the terminal session is kept." },
                    "parameters": [
                        {
                            "name": "command",
                            "description": { "zh": "要执行的 Bash 命令", "en": "Bash command to execute." },
                            "type": "string",
                            "required": true
                        },
                        {
                            "name": "background",
                            "description": { "zh": "是否在后台运行命令,\"true\" 表示后台执行并立即返回,适合启动服务器等长时间运行的任务（AI 不会收到该命令的输出结果），\"false\" 或未提供则前台执行并等待并返回命令结果", "en": "Run command in background. 'true' runs in background and returns immediately (good for long-running tasks like servers; AI will not receive output). 'false' or omitted runs in foreground and returns the command result." },
                            "type": "string",
                            "required": false
                        },
                        {
                            "name": "timeoutMs",
                            "description": { "zh": "可选超时（毫秒，最低3000ms）。强烈建议显式传入；未传时前台默认15000ms，background=true时不使用默认超时。", "en": "Optional timeout (ms, minimum 3000ms). Strongly recommended to pass explicitly; if omitted, foreground defaults to 15000ms, and background=true does not use the default timeout." },
                            "type": "string",
                            "required": false
                        }
                    ]
                }
            ]
        },
        {
            "id": "ios",
            "condition": "platform.ios",
            "inheritTools": true,
            "tools": [
                {
                    "name": "shell",
                    "description": { "zh": "在 iOS Shell 终端会话中执行命令并收集输出结果。默认使用 iSH Alpine Linux Shell；在具备相应权限时也可使用系统 Shell。会话按当前对话维护，上下文连贯。强烈建议每次都显式传 timeoutMs，避免命令卡住。前台未传 timeoutMs 时默认15秒；background=true 时不使用该默认超时。命令超时时会取消当前命令并保留终端会话。", "en": "Execute commands in an iOS shell terminal session and collect output. The default backend is the iSH Alpine Linux shell; a system shell may also be available on privileged hosts. The session is maintained per chat and preserves context. Strongly recommend explicitly passing timeoutMs every time to avoid hangs. Foreground mode defaults to 15s timeout when timeoutMs is omitted; background=true does not use this default timeout. When a command times out, the current command is cancelled and the terminal session is kept." },
                    "parameters": [
                        {
                            "name": "command",
                            "description": { "zh": "要执行的 Shell 命令", "en": "Shell command to execute." },
                            "type": "string",
                            "required": true
                        },
                        {
                            "name": "background",
                            "description": { "zh": "是否在后台运行命令,\"true\" 表示后台执行并立即返回,适合启动服务器等长时间运行的任务（AI 不会收到该命令的输出结果），\"false\" 或未提供则前台执行并等待并返回命令结果", "en": "Run command in background. 'true' runs in background and returns immediately (good for long-running tasks like servers; AI will not receive output). 'false' or omitted runs in foreground and returns the command result." },
                            "type": "string",
                            "required": false
                        },
                        {
                            "name": "timeoutMs",
                            "description": { "zh": "可选超时（毫秒，最低3000ms）。强烈建议显式传入；未传时前台默认15000ms，background=true时不使用默认超时。", "en": "Optional timeout (ms, minimum 3000ms). Strongly recommended to pass explicitly; if omitted, foreground defaults to 15000ms, and background=true does not use the default timeout." },
                            "type": "string",
                            "required": false
                        }
                    ]
                }
            ]
        },
        {
            "id": "android",
            "condition": "platform.android",
            "inheritTools": true,
            "tools": [
                {
                    "name": "bash",
                    "description": { "zh": "在 Android proot Linux Bash 终端会话中执行命令并收集输出结果。会话按当前对话维护，上下文连贯。强烈建议每次都显式传 timeoutMs，避免命令卡住。前台未传 timeoutMs 时默认15秒；background=true 时不使用该默认超时。命令超时时会取消当前命令并保留终端会话。", "en": "Execute commands in an Android proot Linux Bash terminal session and collect output. The session is maintained per chat and preserves context. Strongly recommend explicitly passing timeoutMs every time to avoid hangs. Foreground mode defaults to 15s timeout when timeoutMs is omitted; background=true does not use this default timeout. When a command times out, the current command is cancelled and the terminal session is kept." },
                    "parameters": [
                        {
                            "name": "command",
                            "description": { "zh": "要执行的 Bash 命令", "en": "Bash command to execute." },
                            "type": "string",
                            "required": true
                        },
                        {
                            "name": "background",
                            "description": { "zh": "是否在后台运行命令,\"true\" 表示后台执行并立即返回,适合启动服务器等长时间运行的任务（AI 不会收到该命令的输出结果），\"false\" 或未提供则前台执行并等待并返回命令结果", "en": "Run command in background. 'true' runs in background and returns immediately (good for long-running tasks like servers; AI will not receive output). 'false' or omitted runs in foreground and returns the command result." },
                            "type": "string",
                            "required": false
                        },
                        {
                            "name": "timeoutMs",
                            "description": { "zh": "可选超时（毫秒，最低3000ms）。强烈建议显式传入；未传时前台默认15000ms，background=true时不使用默认超时。", "en": "Optional timeout (ms, minimum 3000ms). Strongly recommended to pass explicitly; if omitted, foreground defaults to 15000ms, and background=true does not use the default timeout." },
                            "type": "string",
                            "required": false
                        }
                    ]
                },
                {
                    "name": "shell",
                    "description": { "zh": "在 Android adb shell 终端会话中执行命令并收集输出结果。会话按当前对话维护，上下文连贯。强烈建议每次都显式传 timeoutMs，避免命令卡住。前台未传 timeoutMs 时默认15秒；background=true 时不使用该默认超时。命令超时时会取消当前命令并保留终端会话。", "en": "Execute commands in an Android adb shell terminal session and collect output. The session is maintained per chat and preserves context. Strongly recommend explicitly passing timeoutMs every time to avoid hangs. Foreground mode defaults to 15s timeout when timeoutMs is omitted; background=true does not use this default timeout. When a command times out, the current command is cancelled and the terminal session is kept." },
                    "parameters": [
                        {
                            "name": "command",
                            "description": { "zh": "要执行的 Shell 命令", "en": "Shell command to execute." },
                            "type": "string",
                            "required": true
                        },
                        {
                            "name": "background",
                            "description": { "zh": "是否在后台运行命令,\"true\" 表示后台执行并立即返回,适合启动服务器等长时间运行的任务（AI 不会收到该命令的输出结果），\"false\" 或未提供则前台执行并等待并返回命令结果", "en": "Run command in background. 'true' runs in background and returns immediately (good for long-running tasks like servers; AI will not receive output). 'false' or omitted runs in foreground and returns the command result." },
                            "type": "string",
                            "required": false
                        },
                        {
                            "name": "timeoutMs",
                            "description": { "zh": "可选超时（毫秒，最低3000ms）。强烈建议显式传入；未传时前台默认15000ms，background=true时不使用默认超时。", "en": "Optional timeout (ms, minimum 3000ms). Strongly recommended to pass explicitly; if omitted, foreground defaults to 15000ms, and background=true does not use the default timeout." },
                            "type": "string",
                            "required": false
                        }
                    ]
                }
            ]
        },
        {
            "id": "web",
            "condition": "platform.web",
            "inheritTools": true,
            "tools": [
                {
                    "name": "shell",
                    "description": { "zh": "在浏览器本地 Linux 虚拟机 Shell 终端会话中执行命令并收集输出结果。会话按当前对话维护，上下文连贯。强烈建议每次都显式传 timeoutMs，避免命令卡住。前台未传 timeoutMs 时默认15秒；background=true 时不使用默认超时。命令超时时会取消当前执行中的命令并保留终端会话。", "en": "Execute commands in a browser-local Linux VM shell session and collect output. The session is maintained per chat and preserves context. Strongly recommend explicitly passing timeoutMs every time to avoid hangs. Foreground mode defaults to 15s when timeoutMs is omitted; background=true does not use the default timeout. When a command times out, the current command is cancelled and the terminal session is kept." },
                    "parameters": [
                        {
                            "name": "command",
                            "description": { "zh": "要执行的 Shell 命令", "en": "Shell command to execute." },
                            "type": "string",
                            "required": true
                        },
                        {
                            "name": "background",
                            "description": { "zh": "是否在后台运行命令,\"true\" 表示后台执行并立即返回,适合启动服务器等长时间运行的任务（AI 不会收到该命令的输出结果）,\"false\" 或未提供则前台执行并等待并返回命令结果", "en": "Run command in background. 'true' runs the command in the background and returns immediately, suitable for long-running servers. 'false' or omitted waits for and returns the command result." },
                            "type": "string",
                            "required": false
                        },
                        {
                            "name": "timeoutMs",
                            "description": { "zh": "可选超时（毫秒，最低3000ms）。强烈建议显式传入；未传时前台默认15000ms，background=true时不使用默认超时。", "en": "Optional timeout in milliseconds (minimum 3000ms). Strongly recommended; foreground defaults to 15000ms when omitted, and background=true does not use a default." },
                            "type": "string",
                            "required": false
                        }
                    ]
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
    const DEFAULT_FOREGROUND_TIMEOUT_MS = 15000;
    // A package call is resumed with a 60s step budget by the JavaScript runtime, so a wait that
    // returns later than that is interrupted instead of answered. Stay below it.
    const DEFAULT_WAIT_TIMEOUT_MS = 50000;
    // Reading the session state is a host round trip; 200ms keeps the wait responsive without
    // turning it into a busy loop over the tool bridge.
    const terminalWaitPollIntervalMs = 200;
    const MIN_TIMEOUT_MS = 3000;
    const DEFAULT_TERMINAL_SESSION_PREFIX = "super_admin_default_session";
    const BACKGROUND_TERMINAL_SESSION_PREFIX = "super_admin_background";

    /**
     * Returns a stable foreground session name scoped to the chat and interpreter type.
     */
    function getDefaultTerminalSessionName(type: TerminalCommandType): string {
        return `${DEFAULT_TERMINAL_SESSION_PREFIX}_${type}_${getChatId()}`;
    }

    /**
     * Returns a distinct background session name scoped to the chat and interpreter type.
     */
    function getBackgroundTerminalSessionName(type: TerminalCommandType): string {
        return `${BACKGROUND_TERMINAL_SESSION_PREFIX}_${type}_${getChatId()}_${Date.now()}`;
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
            context_preserved: result?.timedOut !== true,
            output_saved_to: filePath,
            output_chars: outputStr.length,
            operit_clean_on_exit_dir: OPERIT_CLEAN_ON_EXIT_DIR,
            hint: "Output is large and saved to file. Use read_file_part or grep_code to inspect it.",
        };
    }
    /**
     * Executes a terminal command and returns output plus terminal environment details.
     * @param command - Command to execute.
     * @param background - "true" starts a background terminal command and returns immediately.
     * @param timeoutMs - Optional timeout in milliseconds, with a minimum of 3000ms.
     */
    async function runTerminalCommand(params: TerminalParams, type: TerminalCommandType) {
        try {
            if (!params.command) {
                throw new Error("命令不能为空");
            }
            const command = params.command;
            const background = params.background;
            const timeoutMs = params.timeoutMs;
            const terminalInfo = await Tools.System.terminal.info();
            console.log(`执行终端命令: ${command}`);
            const isBackground = background === "true";
            let timeout;
            if (!isBackground) {
                if (timeoutMs !== undefined) {
                    const parsedTimeout = parseInt(String(timeoutMs), 10);
                    if (!Number.isFinite(parsedTimeout) || parsedTimeout < MIN_TIMEOUT_MS) {
                        throw new Error(`timeoutMs必须是整数且不少于${MIN_TIMEOUT_MS}毫秒`);
                    }
                    timeout = parsedTimeout;
                }
                else {
                    timeout = DEFAULT_FOREGROUND_TIMEOUT_MS;
                }
            }
            if (isBackground) {
                const session = await Tools.System.terminal.create(getBackgroundTerminalSessionName(type), type);
                const terminalEnvironment = {
                    ...terminalInfo,
                    platform: session.platform,
                    terminal: session.terminal,
                    terminalType: session.terminalType
                };
                const sessionId = session.sessionId;
                /**
                 * Runs the background terminal command inside the created session.
                 */
                (async () => {
                    try {
                        await Tools.System.terminal.exec(sessionId, command);
                    }
                    catch (error) {
                        console.error(`[terminal/background] 错误: ${error.message}`);
                        console.error(error.stack);
                    }
                })();
                return {
                    command: command,
                    background: true,
                    sessionId: sessionId,
                    started: true,
                    platform: session.platform,
                    terminal: session.terminal,
                    terminalType: session.terminalType,
                    terminalEnvironment
                };
            }
            const session = await Tools.System.terminal.create(getDefaultTerminalSessionName(type), type);
            const sessionId = session.sessionId;
            const result = await Tools.System.terminal.exec(sessionId, command, timeout);
            const terminalEnvironment = {
                ...terminalInfo,
                platform: result.platform,
                terminal: result.terminal,
                terminalType: result.terminalType
            };
            const timedOut = result.timedOut === true;
            const persistedResult = await persistTerminalOutputIfTooLong(command, result);
            if (persistedResult) {
                persistedResult.timeoutMsUsed = timeout;
                persistedResult.terminalEnvironment = terminalEnvironment;
                return persistedResult;
            }
            return {
                command: command,
                output: result.output,
                exitCode: result.exitCode,
                sessionId: result.sessionId,
                platform: result.platform,
                terminal: result.terminal,
                terminalType: result.terminalType,
                timedOut: timedOut,
                timeoutMsUsed: timeout,
                terminalEnvironment,
                context_preserved: !timedOut
            };
        }
        catch (error) {
            console.error(`[${type}] 错误: ${error.message}`);
            console.error(error.stack);
            throw error;
        }
    }

    /**
     * Executes a command through the shared terminal implementation for PowerShell tools.
     */
    async function powershell(params: TerminalParams) {
        return runTerminalCommand(params, "powershell");
    }

    /**
     * Executes a command through the shared terminal implementation for Bash tools.
     */
    async function bash(params: TerminalParams) {
        return runTerminalCommand(params, "bash");
    }

    /**
     * Executes a command in an Android shell terminal session.
     */
    async function shell(params: TerminalParams) {
        return runTerminalCommand(params, "shell");
    }

    /**
     * Waits for the previous command in the session by reading the session's own commandRunning
     * flag until it clears or the deadline passes.
     *
     * The previous implementation pushed a `printf '<marker>'` command into the same session and
     * waited for the marker to come back. Every host refuses that: a session that still has a
     * command running rejects any second command with
     * "Terminal session already has a command in progress" (hosts/android/src/terminal.rs:394 and
     * the same guard in the linux, windows and common native hosts). So the marker could never be
     * delivered to a busy session — the call failed immediately instead of waiting.
     * @param sessionId - Target session ID.
     * @param timeoutMs - Optional timeout in milliseconds, with a minimum of 3000ms.
     */
    async function terminal_wait(params: TerminalWaitParams) {
        try {
            const timeoutMs = params.timeoutMs;
            let timeout = DEFAULT_WAIT_TIMEOUT_MS;
            if (timeoutMs !== undefined) {
                const parsedTimeout = parseInt(String(timeoutMs), 10);
                if (!Number.isFinite(parsedTimeout) || parsedTimeout < MIN_TIMEOUT_MS) {
                    throw new Error(`timeoutMs必须是整数且不少于${MIN_TIMEOUT_MS}毫秒`);
                }
                timeout = parsedTimeout;
            }
            const sessionId = params.sessionId;
            const startedAt = Date.now();
            const deadline = startedAt + timeout;
            let polls = 0;
            let commandRunning = false;
            let screen: any = null;
            for (;;) {
                // `screen` reports the session's live state, which is the observation primitive
                // that already exists; waiting needs no new host capability.
                screen = await Tools.System.terminal.screen(sessionId);
                polls += 1;
                commandRunning = screen?.commandRunning === true;
                if (!commandRunning) {
                    break;
                }
                const remaining = deadline - Date.now();
                if (remaining <= 0) {
                    break;
                }
                await new Promise<void>((resolve) => {
                    setTimeout(resolve, Math.min(terminalWaitPollIntervalMs, remaining));
                });
            }
            const elapsedMs = Date.now() - startedAt;
            return {
                sessionId,
                commandRunning,
                waitCompleted: !commandRunning,
                timedOut: commandRunning,
                timeoutMsUsed: timeout,
                elapsedMs,
                polls,
                platform: screen?.platform,
                terminal: screen?.terminal,
                terminalType: screen?.terminalType,
                context_preserved: true
            };
        }
        catch (error) {
            console.error(`[terminal_wait] 错误: ${error.message}`);
            console.error(error.stack);
            throw error;
        }
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
        powershell,
        bash,
        shell,
        terminal_wait,
        get_screen,
        input
    };
})();
// 逐个导出
exports.powershell = superAdmin.powershell;
exports.bash = superAdmin.bash;
exports.shell = superAdmin.shell;
exports.terminal_wait = superAdmin.terminal_wait;
exports.get_screen = superAdmin.get_screen;
exports.input = superAdmin.input;

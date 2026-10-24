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
    "description": { "zh": "超级管理员工具集，提供终端命令和会话控制的高级功能。耗时的命令可以用 background=\"true\" 启动、立即拿到 sessionId，之后用 terminal_wait / get_screen 跟进，期间继续做别的事。", "en": "Super admin toolkit providing advanced terminal command and session control capabilities. Long commands can be started with background=\"true\" (returns a sessionId immediately) and followed up with terminal_wait / get_screen while you continue with other work." },
    "enabledByDefault": true,
    "category": "System",
    "tools": [
        {
            "name": "terminal_wait",
            "description": { "zh": "等待同一终端会话中的上一条命令结束。它直接观察会话的 commandRunning 状态，所以命令一结束就返回，而不是固定睡眠。典型用法：先用 background=\"true\" 启动耗时命令并拿到 sessionId，做完其他工具调用后再调用本工具等待它结束。超时只表示这次没等到：它不会取消正在跑的命令，也不会动终端会话，命令会继续跑；再调一次即可继续等。", "en": "Wait until the previous command in the same terminal session finishes. It observes the session's own commandRunning state, so it returns as soon as the command actually ends instead of sleeping for a fixed time. Typical usage: start the slow command with background=\"true\", do other tool calls, then call this to wait for it. A timeout only means the command had not finished yet: it does not cancel the running command and does not touch the terminal session, so call it again to keep waiting." },
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
            "description": { "zh": "获取当前终端会话可见屏幕内容（仅一屏，不包含历史滚动缓冲）。适合查看 background=\"true\" 启动的命令当前跑到哪一步；要看完整长日志，请让命令把输出重定向到文件，再用 read_file_part / grep_code 读取。", "en": "Get the current visible screen content for the active terminal session (single screen only, no scrollback history). Useful for checking where a command started with background=\"true\" currently is; for complete long logs, redirect the output to a file inside the command and read it with read_file_part / grep_code." },
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
        },
        {
            "name": "terminal_jobs",
            "description": { "zh": "列出本对话的终端会话及其实时状态（是否还有命令在跑、最后几行输出），不用再靠 pgrep/tail 猜。不传 sessionId 时查询本对话的三个默认会话；background 启动的会话名带时间戳，请传它返回的 sessionId。", "en": "List this chat's terminal sessions with their live state (whether a command is still running, last few output lines) so you do not have to guess with pgrep/tail. Without sessionId it inspects the three default sessions of this chat; a background session carries a timestamp in its name, so pass the sessionId it returned." },
            "parameters": [
                {
                    "name": "sessionId",
                    "description": { "zh": "可选，精确的会话 ID（例如 background 启动时返回的那个）。", "en": "Optional exact session ID (for example the one returned by a background run)." },
                    "type": "string",
                    "required": false
                }
            ]
        },
        {
            "name": "terminal_env",
            "description": { "zh": "打印当前终端真实的发行版、内核和工具链（os-release / uname / busybox / grep 版本），并给出已知不兼容参数提示。换环境或遇到怪参数时先调它，别猜。", "en": "Report the distribution, kernel and toolchain the terminal really runs (os-release / uname / busybox / grep version) plus hints for known-incompatible flags. Call it before guessing when the environment or a flag behaves oddly." },
            "parameters": [
                {
                    "name": "type",
                    "description": { "zh": "可选，探测哪个解释器：bash / shell / powershell，默认 bash。", "en": "Optional interpreter to probe: bash / shell / powershell. Defaults to bash." },
                    "type": "string",
                    "required": false
                }
            ]
        },
        {
            "name": "list_tools",
            "description": { "zh": "实时列出此刻可调用的工具（名字、所属包、必填参数、一句话说明），可以按包名或关键词过滤。对话里那份 use_package 列表是快照、可能过期；不确定某个工具存不存在、叫什么名字时，先用它查，不要猜也不要放弃。", "en": "List the tools that are callable right now (name, owning package, required parameters, one-line description), optionally filtered by package or keyword. The use_package list in the conversation is a snapshot and may be stale; when you are unsure whether a tool exists or what it is called, look it up here instead of guessing or giving up." },
            "parameters": [
                {
                    "name": "package",
                    "description": { "zh": "可选，只看某个包（例如 super_admin）。", "en": "Optional package name to filter (for example super_admin)." },
                    "type": "string",
                    "required": false
                },
                {
                    "name": "query",
                    "description": { "zh": "可选，按工具名/描述里的关键词过滤（不区分大小写）。", "en": "Optional case-insensitive substring over tool name and description." },
                    "type": "string",
                    "required": false
                },
                {
                    "name": "limit",
                    "description": { "zh": "可选，最多返回多少条，默认 80，上限 200。", "en": "Optional maximum number of tools to return; default 80, capped at 200." },
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
                            "description": { "zh": "是否在后台运行。\"true\"：立即返回 sessionId，命令在该会话里继续运行，这期间你可以继续调用其他工具；之后用 terminal_wait 等它结束（完成即返回），用 get_screen 看当前屏幕。长日志请让命令自己重定向到文件（例如 > /tmp/build.log 2>&1），再用 read_file_part / grep_code 读取完整内容。适合编译、安装依赖、跑测试这类可能超过十几秒的任务。\"false\" 或未提供：前台执行，等命令结束并返回输出；超过 timeoutMs 会被取消。", "en": "Run in the background. 'true': returns immediately with a sessionId and the command keeps running in that session while you make other tool calls; read its progress later with terminal_wait (returns as soon as it finishes) and get_screen (current screen only). For long logs, redirect inside the command (e.g. > /tmp/build.log 2>&1) and read the file with read_file_part / grep_code. Use this for work that may exceed ~15s: builds, dependency installs, test suites. 'false' or omitted: run in the foreground, wait for the command and return its output; the command is cancelled if it exceeds timeoutMs." },
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
                            "description": { "zh": "是否在后台运行。\"true\"：立即返回 sessionId，命令在该会话里继续运行，这期间你可以继续调用其他工具；之后用 terminal_wait 等它结束（完成即返回），用 get_screen 看当前屏幕。长日志请让命令自己重定向到文件（例如 > /tmp/build.log 2>&1），再用 read_file_part / grep_code 读取完整内容。适合编译、安装依赖、跑测试这类可能超过十几秒的任务。\"false\" 或未提供：前台执行，等命令结束并返回输出；超过 timeoutMs 会被取消。", "en": "Run in the background. 'true': returns immediately with a sessionId and the command keeps running in that session while you make other tool calls; read its progress later with terminal_wait (returns as soon as it finishes) and get_screen (current screen only). For long logs, redirect inside the command (e.g. > /tmp/build.log 2>&1) and read the file with read_file_part / grep_code. Use this for work that may exceed ~15s: builds, dependency installs, test suites. 'false' or omitted: run in the foreground, wait for the command and return its output; the command is cancelled if it exceeds timeoutMs." },
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
                            "description": { "zh": "是否在后台运行。\"true\"：立即返回 sessionId，命令在该会话里继续运行，这期间你可以继续调用其他工具；之后用 terminal_wait 等它结束（完成即返回），用 get_screen 看当前屏幕。长日志请让命令自己重定向到文件（例如 > /tmp/build.log 2>&1），再用 read_file_part / grep_code 读取完整内容。适合编译、安装依赖、跑测试这类可能超过十几秒的任务。\"false\" 或未提供：前台执行，等命令结束并返回输出；超过 timeoutMs 会被取消。", "en": "Run in the background. 'true': returns immediately with a sessionId and the command keeps running in that session while you make other tool calls; read its progress later with terminal_wait (returns as soon as it finishes) and get_screen (current screen only). For long logs, redirect inside the command (e.g. > /tmp/build.log 2>&1) and read the file with read_file_part / grep_code. Use this for work that may exceed ~15s: builds, dependency installs, test suites. 'false' or omitted: run in the foreground, wait for the command and return its output; the command is cancelled if it exceeds timeoutMs." },
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
                            "description": { "zh": "是否在后台运行。\"true\"：立即返回 sessionId，命令在该会话里继续运行，这期间你可以继续调用其他工具；之后用 terminal_wait 等它结束（完成即返回），用 get_screen 看当前屏幕。长日志请让命令自己重定向到文件（例如 > /tmp/build.log 2>&1），再用 read_file_part / grep_code 读取完整内容。适合编译、安装依赖、跑测试这类可能超过十几秒的任务。\"false\" 或未提供：前台执行，等命令结束并返回输出；超过 timeoutMs 会被取消。", "en": "Run in the background. 'true': returns immediately with a sessionId and the command keeps running in that session while you make other tool calls; read its progress later with terminal_wait (returns as soon as it finishes) and get_screen (current screen only). For long logs, redirect inside the command (e.g. > /tmp/build.log 2>&1) and read the file with read_file_part / grep_code. Use this for work that may exceed ~15s: builds, dependency installs, test suites. 'false' or omitted: run in the foreground, wait for the command and return its output; the command is cancelled if it exceeds timeoutMs." },
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
                            "description": { "zh": "是否在后台运行。\"true\"：立即返回 sessionId，命令在该会话里继续运行，这期间你可以继续调用其他工具；之后用 terminal_wait 等它结束（完成即返回），用 get_screen 看当前屏幕。长日志请让命令自己重定向到文件（例如 > /tmp/build.log 2>&1），再用 read_file_part / grep_code 读取完整内容。适合编译、安装依赖、跑测试这类可能超过十几秒的任务。\"false\" 或未提供：前台执行，等命令结束并返回输出；超过 timeoutMs 会被取消。", "en": "Run in the background. 'true': returns immediately with a sessionId and the command keeps running in that session while you make other tool calls; read its progress later with terminal_wait (returns as soon as it finishes) and get_screen (current screen only). For long logs, redirect inside the command (e.g. > /tmp/build.log 2>&1) and read the file with read_file_part / grep_code. Use this for work that may exceed ~15s: builds, dependency installs, test suites. 'false' or omitted: run in the foreground, wait for the command and return its output; the command is cancelled if it exceeds timeoutMs." },
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
                            "description": { "zh": "是否在后台运行。\"true\"：立即返回 sessionId，命令在该会话里继续运行，这期间你可以继续调用其他工具；之后用 terminal_wait 等它结束（完成即返回），用 get_screen 看当前屏幕。长日志请让命令自己重定向到文件（例如 > /tmp/build.log 2>&1），再用 read_file_part / grep_code 读取完整内容。适合编译、安装依赖、跑测试这类可能超过十几秒的任务。\"false\" 或未提供：前台执行，等命令结束并返回输出；超过 timeoutMs 会被取消。", "en": "Run in the background. 'true': returns immediately with a sessionId and the command keeps running in that session while you make other tool calls; read its progress later with terminal_wait (returns as soon as it finishes) and get_screen (current screen only). For long logs, redirect inside the command (e.g. > /tmp/build.log 2>&1) and read the file with read_file_part / grep_code. Use this for work that may exceed ~15s: builds, dependency installs, test suites. 'false' or omitted: run in the foreground, wait for the command and return its output; the command is cancelled if it exceeds timeoutMs." },
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
                            "description": { "zh": "是否在后台运行。\"true\"：立即返回 sessionId，命令在该会话里继续运行，这期间你可以继续调用其他工具；之后用 terminal_wait 等它结束（完成即返回），用 get_screen 看当前屏幕。长日志请让命令自己重定向到文件（例如 > /tmp/build.log 2>&1），再用 read_file_part / grep_code 读取完整内容。适合编译、安装依赖、跑测试这类可能超过十几秒的任务。\"false\" 或未提供：前台执行，等命令结束并返回输出；超过 timeoutMs 会被取消。", "en": "Run in the background. 'true': returns immediately with a sessionId and the command keeps running in that session while you make other tool calls; read its progress later with terminal_wait (returns as soon as it finishes) and get_screen (current screen only). For long logs, redirect inside the command (e.g. > /tmp/build.log 2>&1) and read the file with read_file_part / grep_code. Use this for work that may exceed ~15s: builds, dependency installs, test suites. 'false' or omitted: run in the foreground, wait for the command and return its output; the command is cancelled if it exceeds timeoutMs." },
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
                            "description": { "zh": "是否在后台运行。\"true\"：立即返回 sessionId，命令在该会话里继续运行，这期间你可以继续调用其他工具；之后用 terminal_wait 等它结束（完成即返回），用 get_screen 看当前屏幕。长日志请让命令自己重定向到文件（例如 > /tmp/build.log 2>&1），再用 read_file_part / grep_code 读取完整内容。适合编译、安装依赖、跑测试这类可能超过十几秒的任务。\"false\" 或未提供：前台执行，等命令结束并返回输出；超过 timeoutMs 会被取消。", "en": "Run in the background. 'true': returns immediately with a sessionId and the command keeps running in that session while you make other tool calls; read its progress later with terminal_wait (returns as soon as it finishes) and get_screen (current screen only). For long logs, redirect inside the command (e.g. > /tmp/build.log 2>&1) and read the file with read_file_part / grep_code. Use this for work that may exceed ~15s: builds, dependency installs, test suites. 'false' or omitted: run in the foreground, wait for the command and return its output; the command is cancelled if it exceeds timeoutMs." },
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
    // Polling the session's own state is a host round trip; 200ms keeps a wait responsive without
    // turning it into a busy loop over the tool bridge.
    const terminalWaitPollIntervalMs = 200;
    // A foreground command can leave bytes in the shared PTY after it returns (a killed child,
    // a late writer, the echoed input line). Every run prints a marker first and only what
    // follows it is reported.
    const OUTPUT_MARKER_PREFIX = "__OPERIT_OUTPUT_BEGIN_";
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
    async function persistTerminalOutputIfTooLong(command: string, result: any, output: string): Promise<PersistedTerminalOutput | null> {
        const outputStr = output;
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
            // The marker is assembled from two fragments so that the echoed input line cannot
            // contain it; whatever precedes its output belongs to an earlier command.
            const nonce = `${Date.now()}_${Math.floor(Math.random() * 1000000)}`;
            const marker = `${OUTPUT_MARKER_PREFIX}${nonce}__`;
            const markerPrefix = type === "powershell"
                ? ""
                : `A='${OUTPUT_MARKER_PREFIX}'; B='${nonce}__'; printf '%s%s\\n' "$A" "$B"; `;
            const result = await Tools.System.terminal.exec(sessionId, markerPrefix + command, timeout);
            const rawOutput = typeof result?.output === "string" ? result.output : String(result?.output ?? "");
            const markerIndex = markerPrefix ? rawOutput.indexOf(marker) : -1;
            const output = markerIndex >= 0
                ? rawOutput.slice(markerIndex + marker.length).replace(/^\r?\n/, "")
                : rawOutput;
            const terminalEnvironment = {
                ...terminalInfo,
                platform: result.platform,
                terminal: result.terminal,
                terminalType: result.terminalType
            };
            const timedOut = result.timedOut === true;
            const persistedResult = await persistTerminalOutputIfTooLong(command, result, output);
            if (persistedResult) {
                persistedResult.timeoutMsUsed = timeout;
                persistedResult.terminalEnvironment = terminalEnvironment;
                return persistedResult;
            }
            return {
                command: command,
                output: output,
                outputIsolated: markerIndex >= 0,
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
     * Polls the session's own `commandRunning` flag until it clears or the deadline passes.
     * Reading the flag is the only correct observation: every host rejects a second command on a
     * session that still has one running ("Terminal session already has a command in progress"),
     * so a marker command can never be delivered to a busy session — it is refused, not queued.
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
                // `screen` reports the session's live state; it is the observation primitive that
                // already existed, so waiting needs no new host capability.
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
    /**
     * Reports this chat's terminal sessions without guessing through `pgrep`.
     *
     * The default sessions are only public by *name*; the host's `screen` resolves sessions by
     * *id* and rejects a name with "Terminal session does not exist: <name>". `create` is
     * create-or-get, so passing the name returns the existing session (and its id) instead of a
     * new one; `isNewSession` tells us whether the session actually existed before this query.
     *
     * @param sessionId - Optional exact session ID (for example the one returned by background execution).
     */
    async function terminal_jobs(params: SuperAdminParams) {
        const chatId = getChatId();
        const explicit = typeof params.sessionId === "string" ? params.sessionId.trim() : "";
        const defaultTypes: TerminalCommandType[] = ["bash", "shell", "powershell"];
        const candidates: Array<{ name: string; type: TerminalCommandType; sessionId?: string }> = explicit
            ? [{ name: "", type: "bash", sessionId: explicit }]
            : defaultTypes.map((type) => ({ name: getDefaultTerminalSessionName(type), type }));
        const jobs: Array<Record<string, unknown>> = [];
        const notes: string[] = [];
        for (const candidate of candidates) {
            let sessionId = candidate.sessionId ?? "";
            try {
                let createdByQuery = false;
                if (!sessionId) {
                    const info: any = await Tools.System.terminal.create(candidate.name, candidate.type);
                    sessionId = String(info?.sessionId ?? "");
                    if (!sessionId) {
                        throw new Error(`create 没有返回 sessionId: ${candidate.name}`);
                    }
                    // 名字→id 的兑换会顺手建出会话；如实说明，不要让人以为它本来就在。
                    createdByQuery = info?.isNewSession === true;
                    if (createdByQuery) {
                        notes.push(`${candidate.name} 原本不存在（本次查询新建了它）`);
                    }
                }
                const screen = await Tools.System.terminal.screen(sessionId);
                const content = typeof screen?.content === "string" ? screen.content : "";
                const lines = content.split(/\r?\n/).filter((line: string) => line.trim().length > 0);
                jobs.push({
                    sessionId,
                    sessionName: candidate.name || undefined,
                    exists: !createdByQuery,
                    createdByQuery,
                    commandRunning: screen?.commandRunning === true,
                    terminalType: screen?.terminalType,
                    rows: screen?.rows,
                    cols: screen?.cols,
                    tail: lines.slice(-3)
                });
            }
            catch (error) {
                jobs.push({
                    sessionId: sessionId || candidate.name,
                    sessionName: candidate.name || undefined,
                    exists: false,
                    error: error?.message ?? String(error)
                });
            }
        }
        return {
            chatId,
            jobs,
            note: explicit
                ? "只查询了传入的 sessionId。"
                : "只查询本对话的默认会话；background 启动的会话名带时间戳，请传它的 sessionId。" +
                    (notes.length ? ` ${notes.join("；")}。` : "")
        };
    }

    /**
     * Reports the interpreter and distribution the terminal really runs, plus tool-compatibility hints.
     * @param type - Optional interpreter to probe: bash, shell or powershell.
     */
    async function terminal_env(params: SuperAdminParams) {
        const requested = typeof params.type === "string" ? params.type.trim() : "";
        const type: TerminalCommandType = requested === "powershell" || requested === "shell" ? requested : "bash";
        const probe = type === "powershell"
            ? "$PSVersionTable.PSVersion.ToString(); [System.Environment]::OSVersion.VersionString"
            : "head -4 /etc/os-release 2>/dev/null; uname -sr; (busybox 2>/dev/null | head -1) || echo no-busybox; (grep --version 2>/dev/null | head -1) || true";
        const result: any = await runTerminalCommand({ command: probe, timeoutMs: "20000" }, type);
        const text = String(result?.output ?? "");
        const busybox = !/no-busybox/.test(text) && /BusyBox/i.test(text);
        const hints = busybox
            ? ["当前是 BusyBox：`grep --include`、`tar --numeric-owner`、`du -sh` 这类 GNU 用法可能不生效，换 `find | xargs grep` 或先装 coreutils。"]
            : [];
        return {
            type,
            interpreterReport: text,
            busybox,
            hints,
            context_preserved: result?.context_preserved
        };
    }
    /**
     * Lists the tools that are callable right now, so a stale package snapshot is never a dead end.
     * @param package - Optional package name to filter (for example super_admin).
     * @param query - Optional case-insensitive substring over tool name and description.
     * @param limit - Optional maximum number of tools to return (default 80, capped at 200).
     */
    async function list_tools(params: SuperAdminParams) {
        if (typeof getToolCatalog !== "function") {
            throw new Error("当前运行时不提供 getToolCatalog，无法列出工具目录");
        }
        const packageFilter = typeof params.package === "string" ? params.package.trim() : "";
        const query = typeof params.query === "string" ? params.query.trim().toLowerCase() : "";
        const parsedLimit = parseInt(String(params.limit ?? ""), 10);
        const limit = Number.isFinite(parsedLimit) && parsedLimit > 0 ? Math.min(parsedLimit, 200) : 80;
        const catalog: any = await getToolCatalog();
        const all: any[] = Array.isArray(catalog?.tools) ? catalog.tools : [];
        const matched = all.filter((tool: any) => {
            const name = String(tool?.name ?? "");
            const owner = String(tool?.packageName ?? "");
            if (packageFilter && owner !== packageFilter && !name.startsWith(`${packageFilter}:`)) {
                return false;
            }
            if (query) {
                const haystack = `${name} ${String(tool?.description ?? "")}`.toLowerCase();
                if (!haystack.includes(query)) {
                    return false;
                }
            }
            return true;
        });
        const shown = matched.slice(0, limit).map((tool: any) => {
            const required = (Array.isArray(tool?.parameters) ? tool.parameters : [])
                .filter((parameter: any) => parameter?.required === true)
                .map((parameter: any) => String(parameter?.name ?? ""));
            const description = String(tool?.description ?? "").replace(/\s+/g, " ").trim();
            return {
                name: String(tool?.name ?? ""),
                source: String(tool?.source ?? ""),
                packageName: tool?.packageName ?? null,
                requiredParameters: required,
                description: description.length > 120 ? `${description.slice(0, 120)}…` : description
            };
        });
        return {
            total: matched.length,
            shown: shown.length,
            omitted: Math.max(0, matched.length - shown.length),
            tools: shown,
            note: "这是运行时实时读到的目录，比对话里那份 use_package 快照新；要调包内工具，用它的完整名字（packageName:toolName）。"
        };
    }
    return {
        powershell,
        bash,
        shell,
        terminal_wait,
        get_screen,
        input,
        terminal_jobs,
        terminal_env,
        list_tools
    };
})();
// 逐个导出
exports.powershell = superAdmin.powershell;
exports.bash = superAdmin.bash;
exports.shell = superAdmin.shell;
exports.terminal_wait = superAdmin.terminal_wait;
exports.get_screen = superAdmin.get_screen;
exports.input = superAdmin.input;
exports.terminal_jobs = superAdmin.terminal_jobs;
exports.terminal_env = superAdmin.terminal_env;
exports.list_tools = superAdmin.list_tools;

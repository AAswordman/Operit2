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

type TerminalCommandType = "powershell" | "bash" | "zsh" | "shell";

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
    "description": { "zh": "提供终端命令和会话控制。每次获取会话必须进入当前文件目录 `.`：已绑定的主工作区，或未绑定时这组父子对话固定的产出目录；进入失败则终止，不在主目录继续执行。附件和产出直接使用提供的相对路径，无需翻根目录、主目录或全盘查找。跨挂载访问时，终端使用 Host 解析的路径，不直接套用 VFS 绝对路径。请按实际权限操作。", "en": "Runs terminal commands and manages sessions. Each session acquisition must enter the current file directory `.`: the bound primary workspace, or the conversation family's persistent output area when unbound. Failure stops execution instead of continuing in the home directory. Use supplied relative attachment and output paths directly without root/home scans or global searches. For other mounts, use the Host-resolved terminal path instead of a VFS absolute path. Respect actual permissions." },
    "enabledByDefault": true,
    "category": "System",
    "tools": [
        {
            "name": "close",
            "display": {"label": {"zh": "关闭终端会话", "en": "Close terminal session"}, "icon": "terminal", "targetParameter": "sessionId"},
            "description": { "zh": "关闭指定终端会话并释放资源，终止该会话中正在执行的命令。关闭后该会话 ID 不再可用；再次运行终端工具会创建新会话，原会话上下文不会保留。", "en": "Close the specified terminal session, release its resources, and terminate its running command. The closed session ID is no longer usable. Running a terminal tool again creates a new session without the previous context." },
            "parameters": [
                {
                    "name": "sessionId",
                    "description": { "zh": "要关闭的终端会话ID。", "en": "ID of the terminal session to close." },
                    "type": "string",
                    "required": true
                }
            ]
        },
        {
            "name": "terminal_wait",
            "display": {"label": {"zh": "等待终端命令", "en": "Wait for command"}, "icon": "terminal", "targetParameter": "sessionId"},
            "description": { "zh": "等待同一终端会话中的上一条命令执行完成。与 sleep 不同，本工具会在命令实际完成时提前返回，而不是固定睡眠。等待默认 3 秒，到时仅返回后台运行状态，不取消命令；可继续等待、读屏或关闭会话。", "en": "Wait until the previous command in the same terminal session finishes. Unlike sleep, this tool can return early as soon as the command actually completes. Waiting defaults to 20 seconds. When the wait expires, the command keeps running in its original session; wait again, read its screen, or close the session." },
            "parameters": [
                {
                    "name": "sessionId",
                    "description": { "zh": "目标终端会话ID。", "en": "Target terminal session ID." },
                    "type": "string",
                    "required": true
                },
                {
                    "name": "timeoutMs",
                    "description": { "zh": "可选前台等待时间（毫秒，最低1000ms）。未传时默认20000ms（20秒）。", "en": "Optional foreground wait (ms, minimum 1000ms). Defaults to 20000ms (20 seconds) if omitted." },
                    "type": "string",
                    "required": false
                }
            ]
        },
        {
            "name": "get_screen",
            "display": {"label": {"zh": "读取终端屏幕", "en": "Read terminal screen"}, "icon": "terminal", "targetParameter": "sessionId"},
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
            "display": {"label": {"zh": "发送终端输入", "en": "Send terminal input"}, "icon": "terminal", "targetParameter": "sessionId"},
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
                    "display": {"label": {"zh": "运行 PowerShell", "en": "Run PowerShell"}, "icon": "terminal", "targetParameter": "command"},
                    "description": { "zh": "在 Windows PowerShell 终端会话中执行命令并收集输出结果。会话按当前对话维护，上下文连贯。前台未传 timeoutMs 时默认20秒；background=true 时不使用该默认超时。前台等待到时后，命令转入后台继续运行并返回 sessionId，下一条命令使用新会话；用 get_screen、terminal_wait、close 管理原会话。", "en": "Execute commands in a Windows PowerShell terminal session and collect output. The session is maintained per chat and preserves context. Foreground mode defaults to 20s timeout when timeoutMs is omitted; background=true does not use this default timeout. When the foreground wait expires, the command continues in the background and returns its sessionId. The next command uses a new session; use get_screen, terminal_wait or close to manage the original session." },
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
                            "description": { "zh": "可选前台等待时间（毫秒，最低1000ms）。未传时前台默认20000ms，background=true时不使用默认超时。", "en": "Optional foreground wait (ms, minimum 1000ms). if omitted, foreground defaults to 20000ms, and background=true does not use the default timeout." },
                            "type": "string",
                            "required": false
                        }
                    ]
                },
                {
                    "name": "bash",
                    "display": {"label": {"zh": "运行 Bash", "en": "Run Bash"}, "icon": "terminal", "targetParameter": "command"},
                    "description": { "zh": "在 Windows Git Bash 终端会话中执行命令并收集输出结果。会话按当前对话维护，上下文连贯。前台未传 timeoutMs 时默认20秒；background=true 时不使用该默认超时。前台等待到时后，命令转入后台继续运行并返回 sessionId，下一条命令使用新会话；用 get_screen、terminal_wait、close 管理原会话。", "en": "Execute commands in a Windows Git Bash terminal session and collect output. The session is maintained per chat and preserves context. Foreground mode defaults to 20s timeout when timeoutMs is omitted; background=true does not use this default timeout. When the foreground wait expires, the command continues in the background and returns its sessionId. The next command uses a new session; use get_screen, terminal_wait or close to manage the original session." },
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
                            "description": { "zh": "可选前台等待时间（毫秒，最低1000ms）。未传时前台默认20000ms，background=true时不使用默认超时。", "en": "Optional foreground wait (ms, minimum 1000ms). if omitted, foreground defaults to 20000ms, and background=true does not use the default timeout." },
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
                    "display": {"label": {"zh": "运行 Bash", "en": "Run Bash"}, "icon": "terminal", "targetParameter": "command"},
                    "description": { "zh": "在 Bash 终端会话中执行命令并收集输出结果。会话按当前对话维护，上下文连贯。前台未传 timeoutMs 时默认20秒；background=true 时不使用该默认超时。前台等待到时后，命令转入后台继续运行并返回 sessionId，下一条命令使用新会话；用 get_screen、terminal_wait、close 管理原会话。", "en": "Execute commands in a Bash terminal session and collect output. The session is maintained per chat and preserves context. Foreground mode defaults to 20s timeout when timeoutMs is omitted; background=true does not use this default timeout. When the foreground wait expires, the command continues in the background and returns its sessionId. The next command uses a new session; use get_screen, terminal_wait or close to manage the original session." },
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
                            "description": { "zh": "可选前台等待时间（毫秒，最低1000ms）。未传时前台默认20000ms，background=true时不使用默认超时。", "en": "Optional foreground wait (ms, minimum 1000ms). if omitted, foreground defaults to 20000ms, and background=true does not use the default timeout." },
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
                    "display": {"label": {"zh": "运行 Bash", "en": "Run Bash"}, "icon": "terminal", "targetParameter": "command"},
                    "description": { "zh": "在 macOS Bash 终端会话中执行命令并收集输出结果。会话按当前对话维护，上下文连贯。前台未传 timeoutMs 时默认20秒；background=true 时不使用该默认超时。前台等待到时后，命令转入后台继续运行并返回 sessionId，下一条命令使用新会话；用 get_screen、terminal_wait、close 管理原会话。", "en": "Execute commands in a macOS Bash terminal session and collect output. The session is maintained per chat and preserves context. Foreground mode defaults to 20s timeout when timeoutMs is omitted; background=true does not use this default timeout. When the foreground wait expires, the command continues in the background and returns its sessionId. The next command uses a new session; use get_screen, terminal_wait or close to manage the original session." },
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
                            "description": { "zh": "可选前台等待时间（毫秒，最低1000ms）。未传时前台默认20000ms，background=true时不使用默认超时。", "en": "Optional foreground wait (ms, minimum 1000ms). if omitted, foreground defaults to 20000ms, and background=true does not use the default timeout." },
                            "type": "string",
                            "required": false
                        }
                    ]
                },
                {
                    "name": "zsh",
                    "display": {"label": {"zh": "运行 Zsh", "en": "Run Zsh"}, "icon": "terminal", "targetParameter": "command"},
                    "description": { "zh": "在 macOS Zsh 终端会话中执行命令并收集输出结果。会话按当前对话维护，上下文连贯。前台未传 timeoutMs 时默认20秒；background=true 时不使用该默认超时。前台等待到时后，命令转入后台继续运行并返回 sessionId，下一条命令使用新会话；用 get_screen、terminal_wait、close 管理原会话。", "en": "Execute commands in a macOS Zsh terminal session and collect output. The session is maintained per chat and preserves context. Foreground mode defaults to 20s timeout when timeoutMs is omitted; background=true does not use this default timeout. When the foreground wait expires, the command continues in the background and returns its sessionId. The next command uses a new session; use get_screen, terminal_wait or close to manage the original session." },
                    "parameters": [
                        {
                            "name": "command",
                            "description": { "zh": "要执行的 Zsh 命令", "en": "Zsh command to execute." },
                            "type": "string",
                            "required": true
                        },
                        {
                            "name": "background",
                            "description": { "zh": "是否在后台运行命令，\"true\" 表示后台执行并立即返回，适合启动服务器；\"false\" 或未提供则前台执行并返回命令结果。", "en": "Run command in the background. 'true' returns immediately, suitable for long-running servers; 'false' or omitted waits for and returns the command result." },
                            "type": "string",
                            "required": false
                        },
                        {
                            "name": "timeoutMs",
                            "description": { "zh": "可选前台等待时间（毫秒，最低1000ms）。未传时前台默认20000ms，background=true 时不使用默认超时。", "en": "Optional foreground wait in milliseconds (minimum 1000ms). Foreground defaults to 20000ms when omitted; background=true does not use a default." },
                            "type": "string",
                            "required": false
                        }
                    ]
                },
                {
                    "name": "shell",
                    "display": {
                        "label": {
                            "zh": "运行 Shell",
                            "en": "Run Shell"
                        },
                        "icon": "terminal",
                        "targetParameter": "command"
                    },
                    "description": {
                        "zh": "在 macOS 的 ARM64 Linux 引擎中运行 Alpine Shell 命令并收集输出结果。会话按当前对话维护，上下文连贯。前台未传 timeoutMs 时默认20秒；background=true 时不使用该默认超时。前台等待到时后，命令转入后台继续运行并返回 sessionId，下一条命令使用新会话；用 get_screen、terminal_wait、close 管理原会话。",
                        "en": "Execute Alpine shell commands in the ARM64 Linux runtime on macOS and collect output. The session is maintained per chat and preserves context. Foreground mode defaults to 20s timeout when timeoutMs is omitted; background=true does not use this default timeout. When the foreground wait expires, the command continues in the background and returns its sessionId. The next command uses a new session; use get_screen, terminal_wait or close to manage the original session."
                    },
                    "parameters": [
                        {
                            "name": "command",
                            "description": {
                                "zh": "要执行的 Shell 命令",
                                "en": "Shell command to execute."
                            },
                            "type": "string",
                            "required": true
                        },
                        {
                            "name": "background",
                            "description": {
                                "zh": "是否在后台运行命令,\"true\" 表示后台执行并立即返回,适合启动服务器等长时间运行的任务（AI 不会收到该命令的输出结果），\"false\" 或未提供则前台执行并等待并返回命令结果",
                                "en": "Run command in background. 'true' runs in background and returns immediately (good for long-running tasks like servers; AI will not receive output). 'false' or omitted runs in foreground and returns the command result."
                            },
                            "type": "string",
                            "required": false
                        },
                        {
                            "name": "timeoutMs",
                            "description": {
                                "zh": "可选前台等待时间（毫秒，最低1000ms）。未传时前台默认20000ms，background=true时不使用默认超时。",
                                "en": "Optional foreground wait (ms, minimum 1000ms). if omitted, foreground defaults to 20000ms, and background=true does not use the default timeout."
                            },
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
                    "display": {"label": {"zh": "运行 Shell", "en": "Run Shell"}, "icon": "terminal", "targetParameter": "command"},
                    "description": { "zh": "在 iOS Shell 终端会话中执行命令并收集输出结果。使用 Operit ARM64 Linux 环境；系统 Shell 需要特殊权限，且本工具不能选择该后端。会话按当前对话维护，上下文连贯。前台未传 timeoutMs 时默认20秒；background=true 时不使用该默认超时。前台等待到时后，命令转入后台继续运行并返回 sessionId，下一条命令使用新会话；用 get_screen、terminal_wait、close 管理原会话。", "en": "Execute commands in an iOS shell terminal session and collect output. It uses the Operit ARM64 Linux environment; the system shell requires special privileges and cannot be selected through this tool. The session is maintained per chat and preserves context. Foreground mode defaults to 20s timeout when timeoutMs is omitted; background=true does not use this default timeout. When the foreground wait expires, the command continues in the background and returns its sessionId. The next command uses a new session; use get_screen, terminal_wait or close to manage the original session." },
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
                            "description": { "zh": "可选前台等待时间（毫秒，最低1000ms）。未传时前台默认20000ms，background=true时不使用默认超时。", "en": "Optional foreground wait (ms, minimum 1000ms). if omitted, foreground defaults to 20000ms, and background=true does not use the default timeout." },
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
                    "display": {"label": {"zh": "运行 Bash", "en": "Run Bash"}, "icon": "terminal", "targetParameter": "command"},
                    "description": { "zh": "在 Android proot Linux Bash 终端会话中执行命令并收集输出结果；它是用户态 Linux 环境，与 Android 系统 shell 的文件视图和权限可能不同。会话按当前对话维护，上下文连贯。前台未传 timeoutMs 时默认20秒；background=true 时不使用该默认超时。前台等待到时后，命令转入后台继续运行并返回 sessionId，下一条命令使用新会话；用 get_screen、terminal_wait、close 管理原会话。", "en": "Execute commands in an Android proot Linux Bash session. This userspace Linux environment can have a different file view and permissions from the Android system shell. The session is maintained per chat and preserves context. Foreground mode defaults to 20s timeout when timeoutMs is omitted; background=true does not use this default timeout. When the foreground wait expires, the command continues in the background and returns its sessionId. The next command uses a new session; use get_screen, terminal_wait or close to manage the original session." },
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
                            "description": { "zh": "可选前台等待时间（毫秒，最低1000ms）。未传时前台默认20000ms，background=true时不使用默认超时。", "en": "Optional foreground wait (ms, minimum 1000ms). if omitted, foreground defaults to 20000ms, and background=true does not use the default timeout." },
                            "type": "string",
                            "required": false
                        }
                    ]
                },
                {
                    "name": "shell",
                    "display": {"label": {"zh": "运行 Shell", "en": "Run Shell"}, "icon": "terminal", "targetParameter": "command"},
                    "description": { "zh": "在 Android 系统 /system/bin/sh 终端会话中执行命令并收集输出结果。该进程使用应用权限，不是 adb shell；它与 proot Bash 的环境和可见路径可能不同。会话按当前对话维护，上下文连贯。前台未传 timeoutMs 时默认20秒；background=true 时不使用该默认超时。前台等待到时后，命令转入后台继续运行并返回 sessionId，下一条命令使用新会话；用 get_screen、terminal_wait、close 管理原会话。", "en": "Execute commands in the Android system /system/bin/sh terminal session. It uses app permissions, not adb shell privileges; its environment and visible paths may differ from proot Bash. The session is maintained per chat and preserves context. Foreground mode defaults to 20s timeout when timeoutMs is omitted; background=true does not use this default timeout. When the foreground wait expires, the command continues in the background and returns its sessionId. The next command uses a new session; use get_screen, terminal_wait or close to manage the original session." },
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
                            "description": { "zh": "可选前台等待时间（毫秒，最低1000ms）。未传时前台默认20000ms，background=true时不使用默认超时。", "en": "Optional foreground wait (ms, minimum 1000ms). if omitted, foreground defaults to 20000ms, and background=true does not use the default timeout." },
                            "type": "string",
                            "required": false
                        }
                    ]
                }
            ]
        },
        {
            "id": "ohos",
            "condition": "platform.ohos",
            "inheritTools": true,
            "tools": [
                {
                    "name": "shell",
                    "display": {"label": {"zh": "运行 Shell", "en": "Run Shell"}, "icon": "terminal", "targetParameter": "command"},
                    "description": {"zh": "在 OpenHarmony QEMU-vroot Alpine Linux 环境中执行命令；系统 /bin/sh 是另一种终端，本工具不能选择它。环境可用性以实际启动结果为准。会话按对话维护；前台默认超时 3 秒，后台无默认超时。", "en": "Execute commands in the OpenHarmony QEMU-vroot Alpine Linux environment. The system /bin/sh is a separate backend and cannot be selected through this tool. Availability depends on the actual runtime. Sessions are maintained per chat; foreground commands default to 20 seconds and background commands have no default timeout."},
                    "parameters": [
                        {"name": "command", "description": {"zh": "要执行的 Shell 命令", "en": "Shell command to execute."}, "type": "string", "required": true},
                        {"name": "background", "description": {"zh": "true 为后台执行；省略或 false 为前台执行。", "en": "Set true for background execution; omit or use false for foreground."}, "type": "string", "required": false},
                        {"name": "timeoutMs", "description": {"zh": "可选超时毫秒数，最低 1000。", "en": "Optional timeout in milliseconds, minimum 1000."}, "type": "string", "required": false}
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
                    "display": {"label": {"zh": "运行 Shell", "en": "Run Shell"}, "icon": "terminal", "targetParameter": "command"},
                    "description": { "zh": "在浏览器本地 Linux 虚拟机 Shell 中执行命令；它不是宿主系统终端，VFS 路径不一定是虚拟机内的路径。会话按当前对话维护，上下文连贯。前台未传 timeoutMs 时默认20秒；background=true 时不使用默认超时。前台等待到时后，命令转入后台继续运行并返回 sessionId，下一条命令使用新会话；用 get_screen、terminal_wait、close 管理原会话。", "en": "Execute commands in a browser-local Linux VM shell, not a host terminal. VFS paths are not necessarily VM paths. The session is maintained per chat and preserves context. Foreground mode defaults to 20s when timeoutMs is omitted; background=true does not use the default timeout. When the foreground wait expires, the command continues in the background and returns its sessionId. The next command uses a new session; use get_screen, terminal_wait or close to manage the original session." },
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
                            "description": { "zh": "可选前台等待时间（毫秒，最低1000ms）。未传时前台默认20000ms，background=true时不使用默认超时。", "en": "Optional foreground wait in milliseconds (minimum 1000ms). foreground defaults to 20000ms when omitted, and background=true does not use a default." },
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
    const DEFAULT_FOREGROUND_TIMEOUT_MS = 20000;
    const DEFAULT_WAIT_TIMEOUT_MS = DEFAULT_FOREGROUND_TIMEOUT_MS;
    const MIN_TIMEOUT_MS = 1000;
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
     * @param background - "true" starts a background terminal command and returns immediately.
     * @param timeoutMs - Optional timeout in milliseconds, with a minimum of 1000ms.
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
                    const parsedTimeout = Number(timeoutMs);
                    if (!Number.isSafeInteger(parsedTimeout) || parsedTimeout < MIN_TIMEOUT_MS) {
                        throw new Error(`timeoutMs必须是整数且不少于${MIN_TIMEOUT_MS}毫秒`);
                    }
                    timeout = parsedTimeout;
                }
                else {
                    timeout = DEFAULT_FOREGROUND_TIMEOUT_MS;
                }
            }
            const result = await Tools.System.terminal.run({
                sessionName: isBackground ? getBackgroundTerminalSessionName(type) : getDefaultTerminalSessionName(type),
                terminalType: type,
                command,
                waitMs: isBackground ? 0 : timeout
            });
            const terminalEnvironment = {
                ...terminalInfo,
                platform: result.platform,
                terminal: result.terminal,
                terminalType: result.terminalType
            };
            const running = result.timedOut === true;
            const persistedResult = await persistTerminalOutputIfTooLong(command, result);
            return {
                ...(persistedResult || result),
                command,
                exitCode: running ? null : result.exitCode,
                background: running,
                commandRunning: running,
                started: true,
                status: running ? "running" : "completed",
                timeoutMsUsed: isBackground ? 0 : timeout,
                terminalEnvironment,
                context_preserved: true,
                next_command_uses_new_session: running && !isBackground,
                message: running
                    ? (isBackground
                        ? "命令已在独立后台会话中启动。请用 sessionId 调用 get_screen 阅读输出、terminal_wait 等待完成，或 close 终止并关闭会话。"
                        : "命令已转入后台继续运行，未被取消。下一条命令将使用新的默认会话，不继承本会话的临时环境变量。请用 sessionId 调用 get_screen 阅读输出、terminal_wait 等待完成，或 close 终止并关闭会话。")
                    : "命令已完成。"
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
     * Executes a command through the shared terminal implementation for macOS Zsh tools.
     */
    async function zsh(params: TerminalParams) {
        return runTerminalCommand(params, "zsh");
    }

    /**
     * Executes a command in the host's shell terminal session.
     */
    async function shell(params: TerminalParams) {
        return runTerminalCommand(params, "shell");
    }

    /**
     * Waits until prior work in the same terminal session has completed.
     * @param sessionId - Target session ID.
     * @param timeoutMs - Optional timeout in milliseconds, with a minimum of 1000ms.
     */
    async function terminal_wait(params: TerminalWaitParams) {
        try {
            const timeoutMs = params.timeoutMs;
            let timeout = DEFAULT_WAIT_TIMEOUT_MS;
            if (timeoutMs !== undefined) {
                const parsedTimeout = Number(timeoutMs);
                if (!Number.isSafeInteger(parsedTimeout) || parsedTimeout < MIN_TIMEOUT_MS) {
                    throw new Error(`timeoutMs必须是整数且不少于${MIN_TIMEOUT_MS}毫秒`);
                }
                timeout = parsedTimeout;
            }
            const sessionId = params.sessionId;
            const startedAt = Date.now();
            const result = await Tools.System.terminal.wait(sessionId, timeout);
            const running = result.timedOut === true;
            const persistedResult = await persistTerminalOutputIfTooLong(result.command, result);
            return {
                ...(persistedResult || result),
                sessionId,
                exitCode: running ? null : result.exitCode,
                background: running,
                commandRunning: running,
                timedOut: running,
                timeoutMsUsed: timeout,
                elapsedMs: Date.now() - startedAt,
                waitCompleted: !running,
                context_preserved: true,
                message: running ? "等待时间已到，命令仍在原会话后台运行，可继续等待、读屏或关闭会话。" : "命令已完成。"
            };
        }
        catch (error) {
            console.error(`[terminal_wait] 错误: ${error.message}`);
            console.error(error.stack);
            throw error;
        }
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
        powershell,
        bash,
        zsh,
        shell,
        close,
        terminal_wait,
        get_screen,
        input
    };
})();
// 逐个导出
exports.powershell = superAdmin.powershell;
exports.bash = superAdmin.bash;
exports.zsh = superAdmin.zsh;
exports.shell = superAdmin.shell;
exports.close = superAdmin.close;
exports.terminal_wait = superAdmin.terminal_wait;
exports.get_screen = superAdmin.get_screen;
exports.input = superAdmin.input;

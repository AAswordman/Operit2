import { manageSessions } from "./sessions";

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
    output_saved_to: string;
    output_chars: number;
    operit_clean_on_exit_dir: string;
    hint: string;
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
        "zh": "提供终端命令和会话控制。",
        "en": "Runs terminal commands and manages sessions."
    },
    "enabledByDefault": true,
    "category": "System",
    "states": [
        {
            "id": "windows", "condition": "platform.windows", "inheritTools": true,
            "tools": [{
                "name": "platform_paths", "advice": true, "parameters": [],
                "description": {
                    "zh": "当前为 Windows。文件工具中 /virtual/windows/c/... 对应本机 C:\\...，其他盘符同理。原生终端使用 Windows 路径和所选 Shell 的语法；WSL 配置使用所选发行版的 Linux 路径，由 Host 转换工作目录，不要手拼 /mnt/<盘符> 或把 Windows VFS 前缀交给 Bash。",
                    "en": "Current platform: Windows. File-tool path /virtual/windows/c/... maps to native C:\\...; other drives follow the same pattern. Native terminals use Windows paths and the selected shell syntax. WSL profiles use Linux paths in the selected distribution; the Host converts the working directory. Do not guess /mnt/<drive> mappings or pass Windows VFS prefixes to Bash."
                }
            }]
        },
        {
            "id": "linux", "condition": "platform.linux", "inheritTools": true,
            "tools": [{
                "name": "platform_paths", "advice": true, "parameters": [],
                "description": {
                    "zh": "当前为 Linux。文件工具中 /virtual/linux/... 对应本机 /...，例如 /virtual/linux/app/run.py 对应终端 /app/run.py。终端的 /app 是本机目录；文件工具的 /app 是应用 VFS 别名，两者不能按名字视为同一目录。终端使用所选 Shell 的 POSIX 路径和语法。",
                    "en": "Current platform: Linux. File-tool path /virtual/linux/... maps to native /..., e.g. /virtual/linux/app/run.py maps to terminal path /app/run.py. Terminal /app is a native directory; file-tool /app is an application VFS alias, so matching names do not imply the same directory. Use POSIX paths and the selected shell syntax."
                }
            }]
        },
        {
            "id": "macos", "condition": "platform.macos", "inheritTools": true,
            "tools": [{
                "name": "platform_paths", "advice": true, "parameters": [],
                "description": {
                    "zh": "当前为 macOS。文件工具中 /virtual/macos/... 对应本机 /...，例如 /virtual/macos/Users/alice/project 对应终端 /Users/alice/project。终端使用所选 Shell 的语法；文件访问受应用已有授权限制。",
                    "en": "Current platform: macOS. File-tool path /virtual/macos/... maps to native /..., e.g. /virtual/macos/Users/alice/project maps to terminal /Users/alice/project. Use the selected shell syntax. File access remains subject to existing application grants."
                }
            }]
        },
        {
            "id": "android", "condition": "platform.android", "inheritTools": true,
            "tools": [{
                "name": "platform_paths", "advice": true, "parameters": [],
                "description": {
                    "zh": "当前为 Android。文件工具中 /virtual/android/root/... 对应系统 /...，/virtual/android/sdcard/... 对应共享存储 /sdcard/...。/virtual/android/documents/<id>/... 是已授权的 SAF 文档树，可用文件工具访问，不能作为终端物理路径。系统 Shell、PRoot/chroot 发行版和 Termux 各有自己的根目录与权限；所选配置由环境信息给出，Host 负责工作目录映射，不要混用各自的 /root、/home、/data 路径。",
                    "en": "Current platform: Android. File-tool /virtual/android/root/... maps to system /...; /virtual/android/sdcard/... maps to shared storage /sdcard/.... /virtual/android/documents/<id>/... is an authorized SAF document tree for file tools, not a physical terminal path. System shell, PRoot/chroot distributions and Termux have distinct roots and permissions. Environment context identifies the selected profile; the Host maps its working directory. Do not mix their /root, /home or /data paths."
                }
            }]
        },
        {
            "id": "ios", "condition": "platform.ios", "inheritTools": true,
            "tools": [{
                "name": "platform_paths", "advice": true, "parameters": [],
                "description": {
                    "zh": "当前为 iOS。文件工具中 /virtual/ios/... 表示宿主 /...，访问受应用沙箱及已有授权限制。终端配置可能有独立文件系统；使用 Host 已映射的工作目录和相对路径，不要把宿主 VFS 地址当作终端内部路径。",
                    "en": "Current platform: iOS. File-tool /virtual/ios/... denotes host /..., subject to the application sandbox and existing grants. Terminal profiles can have a separate filesystem. Use the Host-mapped working directory and relative paths; host VFS addresses are not terminal-internal paths."
                }
            }]
        },
        {
            "id": "ohos", "condition": "platform.ohos", "inheritTools": true,
            "tools": [{
                "name": "platform_paths", "advice": true, "parameters": [],
                "description": {
                    "zh": "当前为 OpenHarmony。文件工具中 /virtual/ohos/... 对应宿主 /...，访问受应用沙箱和授权限制。终端中的宿主目录由 Host 映射到 /mnt/host-root 下；直接使用当前工作目录的相对路径，不要套用 Android 的共享存储路径。",
                    "en": "Current platform: OpenHarmony. File-tool /virtual/ohos/... maps to host /..., subject to sandbox permissions and grants. The Host maps host directories beneath /mnt/host-root in the terminal. Use paths relative to the current working directory; Android shared-storage paths do not apply."
                }
            }]
        },
        {
            "id": "web", "condition": "platform.web", "inheritTools": true,
            "tools": [{
                "name": "platform_paths", "advice": true, "parameters": [],
                "description": {
                    "zh": "当前为 Web。文件工具访问浏览器私有存储和已授权资源挂载，没有本机盘符或本机 POSIX 根目录。终端运行于虚拟环境，Host 通过 /mnt/operit-host 暴露浏览器文件系统并映射工作目录；使用当前目录的相对路径，不要推测用户电脑上的物理路径。",
                    "en": "Current platform: Web. File tools access browser-private storage and authorized resource mounts, with no native drive letters or native POSIX root. The terminal runs in a virtual environment; the Host exposes browser storage through /mnt/operit-host and maps the working directory. Use relative paths from that directory rather than guessing physical paths on the user's computer."
                }
            }]
        }
    ],
    "tools": [
        {
            "name": "usage_advice", "advice": true, "parameters": [],
            "description": {
                "zh": "路径约定：文件工具使用 VFS 地址。/operitapp/workspaces/<id>/ 是应用管理的项目目录，/operitapp/outputs/<id>/ 是这组父子对话的固定产出目录；绑定的项目也可能在 /virtual/... 下。终端启动时 Host 已将当前文件目录映射到所选配置中，`.` 就是已绑定的主工作区，未绑定时为固定产出目录；进入失败会终止执行。操作项目文件、附件和产出时直接使用已提供的相对路径，无需先搜索根目录或主目录。/operitapp 和 /virtual 是应用文件地址，不是 Shell 的同名挂载点；跨挂载路径需要 Host 解析，不要把 VFS 绝对路径直接放进终端命令。",
                "en": "Paths: file tools use VFS addresses. /operitapp/workspaces/<id>/ holds app-managed projects; /operitapp/outputs/<id>/ is this conversation family's persistent output directory. Bound projects may also reside under /virtual/.... At terminal startup the Host maps the current file directory into the selected profile: `.` is the bound primary workspace, or the persistent output directory when unbound. A failed directory change stops execution. Use supplied relative paths for project files, attachments and outputs without scanning root or home. /operitapp and /virtual are application file addresses, not same-named shell mounts. Cross-mount paths require Host resolution; do not place VFS absolute paths directly into terminal commands."
            }
        },
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
                "zh": "每次启动一个独立命令进程，完整脚本在启动时传入，工作目录为当前文件目录。持续执行到真实进程退出；包调用返回 running 时使用 package_wait 领取后续输出和最终结果，不要重复执行。启动事件的 sessionId 可用于读屏、输入或关闭。每次调用环境独立，变量和 cd 不跨调用保留；需要持久交互请用 terminal_open。命令结束后闲置10分钟自动关闭会话。",
                "en": "Start a fresh command process with the full script supplied at launch in the current file directory. Execute through real process exit; if the package call returns running, use package_wait for further output and the final result without resubmitting. Use sessionId from the start event for screen, input or close. Variables and directory changes do not carry across calls; use terminal_open for persistent interaction. Completed sessions close after ten idle minutes."
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
            "name": "terminal_open",
            "display": {
                "label": {
                    "zh": "打开交互终端",
                    "en": "Open interactive terminal"
                },
                "icon": "terminal"
            },
            "description": {
                "zh": "打开持久交互 Shell，工作目录为当前文件目录。使用 input 发送原始文本、Enter 或 Ctrl+C，get_screen 读取屏幕。变量、cd 和前台程序在该会话中持续保留。交互输入不判定单条命令退出码。交互终端存活期间不自动清理；使用完须 close。",
                "en": "Open a persistent interactive shell in the current file directory. Use input for literal text, Enter or Ctrl+C and get_screen for the screen. Variables, directory changes and foreground programs persist. Typed commands have no managed exit receipt. Live interactive sessions require explicit close."
            },
            "parameters": [
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
                "zh": "向当前终端会话写入输入。input 与 control 至少传一个。input 与 control=enter 可一起发送，原子提交一行；control=ctrl 且 input=c 可发送 Ctrl+C。",
                "en": "Write input to the active terminal session. Provide at least one of input or control. Send input together with control=enter to submit a line atomically; use control=ctrl with input=c for Ctrl+C."
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
}
*/

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
    async function persistTerminalOutputIfTooLong(outputStr: string): Promise<PersistedTerminalOutput | null> {
        if (outputStr.length <= MAX_INLINE_TERMINAL_OUTPUT_CHARS) {
            return null;
        }
        await Tools.Files.mkdir(OPERIT_CLEAN_ON_EXIT_DIR, true);
        const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
        const rand = Math.floor(Math.random() * 1000000);
        const filePath = `${OPERIT_CLEAN_ON_EXIT_DIR}/terminal_output_${timestamp}_${rand}.log`;
        await Tools.Files.write(filePath, outputStr, false);
        return {
            output_saved_to: filePath,
            output_chars: outputStr.length,
            operit_clean_on_exit_dir: OPERIT_CLEAN_ON_EXIT_DIR,
            hint: "Output is large and saved to file. Use read_file_part or grep_code to inspect it.",
        };
    }
    /** Returns completion metadata without repeating output already published as stream events. */
    async function commandResult(result: Awaited<ReturnType<typeof Tools.System.terminal.run>>) {
        const persisted = await persistTerminalOutputIfTooLong(result.output);
        return {
            sessionId: result.sessionId,
            platform: result.platform,
            terminal: result.terminal,
            terminalType: result.terminalType,
            ...persisted,
            exitCode: result.exitCode,
            timedOut: false,
            background: false, commandRunning: false, started: true,
            status: "completed", context_preserved: false,
            message: "命令已完成。"
        };
    }

    /** Streams one command through completion while the package runtime owns foreground waiting. */
    async function terminal(params: TerminalParams) {
        let acquiredFamily: string | undefined;
        let activeSession: string | undefined;
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
            const sessionName = getDefaultTerminalSessionName(profileId);
            const sessionFamily = `${sessionName}_profile_${profileId}`;
            await manageSessions({ action: "acquire_family", sessionName: sessionFamily });
            acquiredFamily = sessionFamily;
            console.log(`执行终端命令: ${command}`);
            for await (const event of Tools.System.terminal.runStream({
                sessionName,
                profileId,
                command,
            })) {
                if (event.type === "completed") {
                    const result = event.result;
                    return {
                        ...await commandResult(result),
                        terminalEnvironment: { ...terminalInfo, platform: result.platform, terminal: result.terminal, terminalType: result.terminalType },
                    };
                }
                if (event.type === "start") {
                    await manageSessions({ action: "track", sessionId: event.sessionId, sessionName: sessionFamily });
                    activeSession = event.sessionId;
                    await manageSessions({ action: "release_family", sessionName: sessionFamily });
                    acquiredFamily = undefined;
                }
                sendIntermediateResult({
                    type: event.type, sessionId: event.sessionId,
                    platform: event.platform, terminal: event.terminal, terminalType: event.terminalType,
                    ...(event.type === "chunk" ? {
                        chunk: event.chunk, chunkIndex: event.chunkIndex, receivedChars: event.receivedChars,
                    } : {}),
                });
            }
            throw new Error("Terminal stream ended without a completion result");
        }
        catch (error) {
            console.error(`[terminal] 错误: ${error.message}`);
            console.error(error.stack);
            throw error;
        }
        finally {
            if (activeSession !== undefined) await manageSessions({ action: "release_session", sessionId: activeSession });
            if (acquiredFamily !== undefined) await manageSessions({ action: "release_family", sessionName: acquiredFamily });
        }
    }

    /** Lists user-managed profiles and their default without platform assumptions. */
    async function list_terminals() {
        return Tools.System.terminal.info();
    }

    /** Closes the specified terminal session through the host terminal API. */
    async function close(params: TerminalSessionParams) {
        await manageSessions({ action: "acquire_session", sessionId: params.sessionId });
        try {
            const result = await Tools.System.terminal.close(params.sessionId);
            if (result.success) await manageSessions({ action: "forget", sessionId: params.sessionId });
            return result;
        } finally {
            await manageSessions({ action: "release_session", sessionId: params.sessionId });
        }
    }
    /**
     * Gets the visible screen content for a terminal session.
     * @param sessionId - Target session ID.
     */
    async function get_screen(params: TerminalSessionParams) {
        await manageSessions({ action: "acquire_session", sessionId: params.sessionId });
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
        finally {
            await manageSessions({ action: "release_session", sessionId: params.sessionId });
        }
    }
    /**
     * Writes input or a control key to a terminal session.
     * @param sessionId - Target session ID.
     * @param input - Text input.
     * @param control - Control key.
     */
    async function input(params: TerminalInputParams) {
        await manageSessions({ action: "acquire_session", sessionId: params.sessionId });
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
        finally {
            await manageSessions({ action: "release_session", sessionId: params.sessionId });
        }
    }
    /** Opens a persistent interactive shell; its process remains available until explicit close. */
    async function terminal_open(params: SuperAdminParams & { profileId?: string }) {
        const info = await Tools.System.terminal.info();
        const profileId = params.profileId ?? info.defaultProfileId;
        const profile = info.profiles.find(entry => entry.id === profileId);
        if (!profile?.available) throw new Error(`Terminal profile is unavailable: ${profileId}`);
        const sessionName = getDefaultTerminalSessionName(profileId);
        const session = await Tools.System.terminal.open({ sessionName, profileId });
        await manageSessions({ action: "track", sessionId: session.sessionId, sessionName: `${sessionName}_profile_${profileId}` });
        await manageSessions({ action: "release_session", sessionId: session.sessionId });
        return { ...session, context_preserved: true, mode: "interactive",
            message: "交互终端已打开。input 发送文本或按键，get_screen 读屏；用 close 显式关闭。交互输入不提供单条命令退出码。" };
    }

    return {
        terminal_open,
        terminal,
        list_terminals,
        close,
        get_screen,
        input
    };
})();
// 逐个导出
exports.terminal_open = superAdmin.terminal_open;
exports.terminal = superAdmin.terminal;
exports.list_terminals = superAdmin.list_terminals;
exports.close = superAdmin.close;
exports.get_screen = superAdmin.get_screen;
exports.input = superAdmin.input;

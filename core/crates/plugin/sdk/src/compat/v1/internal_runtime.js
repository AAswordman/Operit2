const v1RuntimeStates = {
    android: { type: "bash", programType: "bash", packages: "apk", python: "python3", paths: "posix" },
    ios: { type: "shell", programType: "shell", packages: "apk", python: "python3", paths: "posix" },
    web: { type: "shell", programType: "shell", packages: "native", python: "python3", paths: "posix" },
    windows: { type: "bash", programType: "powershell", packages: "native", python: "python", paths: "msys" },
    linux: { type: "bash", programType: "bash", packages: "native", python: "python3", paths: "posix" },
    macos: { type: "bash", programType: "bash", packages: "native", python: "python3", paths: "posix" },
    ohos: { type: "shell", programType: "shell", packages: "apk", python: "python3", paths: "posix" },
};
/** Resolves exactly the conditional state selected by the Rust package manager. */
function v1RuntimeState() {
    const id = getState();
    if (id === undefined || !Object.prototype.hasOwnProperty.call(v1RuntimeStates, id))
        throw new Error("V1 runtime requires a supported package state");
    return v1RuntimeStates[id];
}
/** Quotes one POSIX-shell argument without interpreting its contents. */
function v1ShellQuote(value) {
    return "'" + value.replace(/'/g, "'\\''") + "'";
}
/** Validates a concrete runtime field before starting a host operation. */
function v1RequiredText(value, field) {
    if (typeof value !== "string" || value.length === 0 || value.indexOf("\0") !== -1)
        throw new Error(field + " must be a nonempty string without NUL bytes");
    return value;
}
/** Validates an explicitly supplied execution limit and preserves zero as a host-defined limit. */
function v1Timeout(value, defaultValue) {
    if (value === undefined)
        return defaultValue;
    const timeout = Number(value);
    if (!Number.isSafeInteger(timeout) || timeout < 0)
        throw new Error("timeoutMs must be a nonnegative integer");
    return timeout;
}
/** Implements the admitted apt command grammar using real APK operations and exact package mappings. */
const v1ApkShim = `#!/bin/sh
# Reports unsupported apt arguments without executing a different operation.
fail() { printf '%s\\n' "$*" >&2; exit 64; }
operation=
packages=
for argument in "$@"; do
    case "$argument" in
        -y|--yes|--assume-yes|--no-install-recommends|-q|-qq) ;;
        update|install|remove|purge)
            [ -z "$operation" ] || fail "Only one apt operation is supported"
            operation="$argument" ;;
        --*) fail "Unsupported apt option: $argument" ;;
        -*) fail "Unsupported apt option: $argument" ;;
        *)
            [ "$operation" = install ] || [ "$operation" = remove ] || [ "$operation" = purge ] || fail "Unsupported apt operation: $argument"
            case "$argument" in
                python3-pip) package=py3-pip ;;
                python3-venv) package=py3-virtualenv ;;
                build-essential) package=build-base ;;
                pkg-config) package=pkgconf ;;
                libssl-dev) package=openssl-dev ;;
                python3|python3-dev|nodejs|npm|bash|curl|wget|git|unzip|zip|ffmpeg|openssl|ca-certificates|coreutils|findutils|grep|sed|gawk|make|gcc|g++|libffi-dev) package="$argument" ;;
                *) fail "No v1 APK package mapping: $argument" ;;
            esac
            packages="$packages $package" ;;
    esac
done
case "$operation" in
    update) [ -z "$packages" ] || fail "apt update does not accept packages"; exec apk update ;;
    install) [ -n "$packages" ] || fail "apt install requires packages"; exec apk add --no-cache $packages ;;
    remove|purge) [ -n "$packages" ] || fail "apt remove requires packages"; exec apk del $packages ;;
    *) fail "Supported apt operations: update, install, remove, purge" ;;
esac
`;
/** Installs the selected environment's command shims inside its existing writable home directory. */
async function v1InitializeSession(session, state) {
    if (state.packages !== "apk")
        return;
    const script = 'mkdir -p "$HOME/.operit-v1/bin" && ' +
        "printf '%s' " + v1ShellQuote(v1ApkShim) + ' > "$HOME/.operit-v1/bin/apt" && ' +
        'cp "$HOME/.operit-v1/bin/apt" "$HOME/.operit-v1/bin/apt-get" && ' +
        'chmod 700 "$HOME/.operit-v1/bin/apt" "$HOME/.operit-v1/bin/apt-get" && ' +
        "printf '%s\\n' " + v1ShellQuote('case ":$PATH:" in *":$HOME/.operit-v1/bin:"*) ;; *) export PATH="$HOME/.operit-v1/bin:$PATH" ;; esac') + ' > "$HOME/.operit-v1/environment.sh" && ' +
        'export BASH_ENV="$HOME/.operit-v1/environment.sh" && . "$BASH_ENV"';
    const result = await __operitV1CanonicalTools.System.terminal.exec(session.sessionId, script, 15000);
    v1RequireCompleted(result);
}
/** Rejects failed or interrupted subprocesses with their actual host diagnostics. */
function v1RequireCompleted(result) {
    if (result.timedOut === true)
        throw new Error("V1 runtime command timed out: " + result.output);
    if (result.exitCode !== 0)
        throw new Error("V1 runtime command exited " + result.exitCode + ": " + result.output);
}
/** Creates the exact interpreter selected by package state and initializes its compatibility commands. */
async function v1CreateSession(params) {
    const state = v1RuntimeState();
    const name = v1RequiredText(params.sessionName, "sessionName");
    const session = await __operitV1CanonicalTools.System.terminal.create("v1_" + getCallerName() + "_" + name, state.type);
    await v1InitializeSession(session, state);
    return session;
}
/** Runs a command in its explicit session, or the legacy caller-scoped default session. */
async function v1Execute(params) {
    v1RuntimeState();
    const command = v1RequiredText(params.command, "command");
    const sessionId = params.sessionId === ""
        ? (await v1CreateSession({ sessionName: "default_" + getChatId() })).sessionId
        : v1RequiredText(params.sessionId, "sessionId");
    return __operitV1CanonicalTools.System.terminal.exec(sessionId, command, v1Timeout(params.timeoutMs, 15000));
}
/** Preserves the legacy terminal result and keeps foreground sessions isolated by caller and chat. */
async function v1Terminal(params) {
    const command = v1RequiredText(params.command, "command");
    if (params.background !== undefined && params.background !== "true" && params.background !== "false")
        throw new Error("background must be 'true' or 'false'");
    const background = params.background === "true";
    const name = "terminal_" + getChatId() + (background ? "_background_" + Date.now() : "");
    const session = await v1CreateSession({ sessionName: name });
    if (background) {
        const pending = __operitV1CanonicalTools.System.terminal.exec(session.sessionId, command, v1Timeout(params.timeoutMs, 0));
        pending.catch(v1ReportBackgroundError);
        return { command, background: true, started: true, sessionId: session.sessionId, platform: session.platform, terminal: session.terminal, terminalType: session.terminalType };
    }
    const result = await v1Execute({ ...params, sessionId: session.sessionId });
    return { ...result, context_preserved: result.timedOut !== true, timeoutMsUsed: v1Timeout(params.timeoutMs, 15000) };
}
/** Reports detached command failures through the existing execution console. */
function v1ReportBackgroundError(error) {
    console.error(error);
}
/** Executes one installed runtime program and preserves its real output and exit status. */
async function v1Program(command, timeoutMs) {
    const session = await v1CreateSession({ sessionName: "program_" + getChatId() });
    const result = await v1Execute({ command, sessionId: session.sessionId, timeoutMs: v1Timeout(timeoutMs, 60000) });
    v1RequireCompleted(result);
    return result;
}
/** Encodes UTF-8 source as hex so Windows native argument parsing cannot remove JavaScript quotes. */
function v1Utf8Hex(value) {
    return encodeURIComponent(value).replace(/%([0-9A-F]{2})|([^%])/g,
    /** Converts either a percent-encoded byte or an unescaped ASCII character into one hex byte. */
    function (_match, byte, ascii) {
        if (byte !== undefined)
            return byte.toLowerCase();
        if (ascii === undefined)
            throw new Error("Invalid UTF-8 encoder token");
        return ascii.charCodeAt(0).toString(16).padStart(2, "0");
    });
}
/** Runs Node source as one quoted argument without changing embedded shell or JavaScript strings. */
async function v1RunNode(params) {
    const state = v1RuntimeState();
    const session = await __operitV1CanonicalTools.System.terminal.create("v1_node_" + getCallerName() + "_" + getChatId(), state.programType);
    const script = v1RequiredText(params.script, "script");
    const launcher = "eval(Buffer.from('" + v1Utf8Hex(script) + "','hex').toString('utf8'))";
    const command = state.programType === "powershell"
        ? "& node -e '" + launcher.replace(/'/g, "''") + "'"
        : "node -e " + v1ShellQuote(launcher);
    const result = await __operitV1CanonicalTools.System.terminal.exec(session.sessionId, command, v1Timeout(params.timeoutMs, 60000));
    v1RequireCompleted(result);
    return result;
}
/** Executes the supplied FFmpeg arguments and reports the actual subprocess receipt. */
async function v1FfmpegExecute(params) {
    const startedAt = Date.now();
    const result = await v1Program("ffmpeg " + v1RequiredText(params.command, "command"), params.timeoutMs);
    return { ...result, returnCode: result.exitCode, duration: Date.now() - startedAt };
}
/** Reads version information from the installed FFmpeg executable. */
async function v1FfmpegInfo() {
    return v1FfmpegExecute({ command: "-version" });
}
/** Performs Linux file operations in the same managed filesystem as compatibility terminal commands. */
const v1FileProgram = String.raw `import base64,json,pathlib,shutil,subprocess,sys,zipfile
request=json.loads(sys.argv[1])
args=request['args']
operation=request['operation']
# Resolves the explicit runtime path vocabulary selected by the package state.
def path(value):
    if not isinstance(value,str) or not value.startswith('/') or chr(92) in value or chr(0) in value:
        raise ValueError('V1 Linux file paths must be absolute slash-separated paths')
    if request['paths']=='msys':
        return pathlib.Path(subprocess.check_output(['cygpath','-w',value],text=True).strip())
    return pathlib.Path(value)
p=path(args['path'])
result={'path':args['path']}
if operation=='read':
    data=p.read_bytes()
    result.update(content=data.decode('utf-8'),size=len(data))
elif operation=='readBinary':
    data=p.read_bytes()
    result.update(contentBase64=base64.b64encode(data).decode('ascii'),size=len(data))
elif operation=='exists':
    exists=p.exists()
    result.update(exists=exists,isDirectory=p.is_dir(),size=p.stat().st_size if exists else 0)
elif operation=='write':
    with p.open('a' if args.get('append',False) else 'w',encoding='utf-8',newline='') as output:
        output.write(args['content'])
elif operation=='writeBinary':
    with p.open('ab' if args.get('append',False) else 'wb') as output:
        output.write(base64.b64decode(args['contentBase64'],validate=True))
elif operation=='mkdir':
    p.mkdir(parents=args.get('createParents',False),exist_ok=True)
elif operation=='deleteFile':
    if p.is_dir():
        shutil.rmtree(p) if args.get('recursive',False) else p.rmdir()
    else:
        p.unlink()
elif operation=='move':
    p.rename(path(args['destination']))
elif operation=='copy':
    destination=path(args['destination'])
    if p.is_dir():
        if not args.get('recursive',False):
            raise ValueError('Copying a directory requires recursive=true')
        shutil.copytree(p,destination)
    else:
        shutil.copyfile(p,destination)
elif operation=='unzip':
    destination=path(args['destination']).resolve()
    with zipfile.ZipFile(p) as archive:
        for entry in archive.infolist():
            target=(destination/entry.filename).resolve()
            if target!=destination and destination not in target.parents:
                raise ValueError('ZIP entry escapes the extraction directory')
            if entry.external_attr>>16&0o170000==0o120000:
                raise ValueError('ZIP symbolic links are unsupported')
        archive.extractall(destination)
else:
    raise ValueError('Unsupported V1 runtime file operation: '+operation)
if operation not in ('read','readBinary','exists'):
    result.update(operation=operation,successful=True,details=operation+' completed')
print(json.dumps(result,ensure_ascii=True))
`;
/** Invokes the file helper with JSON arguments and rejects malformed or failed native output. */
async function v1File(params) {
    const operation = v1RequiredText(params.operation, "operation");
    if (operation !== "write" && operation !== "writeBinary")
        return v1FileRequest(operation, params.args);
    const field = operation === "write" ? "content" : "contentBase64";
    const content = params.args[field];
    if (typeof content !== "string")
        throw new Error(field + " must be a string");
    const chunkSize = operation === "writeBinary" ? 8192 : 1024;
    let offset = 0;
    let receipt;
    do {
        let end = Math.min(offset + chunkSize, content.length);
        const last = content.charCodeAt(end - 1);
        if (end < content.length && last >= 0xD800 && last <= 0xDBFF)
            end--;
        const args = { ...params.args, [field]: content.slice(offset, end), append: params.args.append === true || offset > 0 };
        receipt = await v1FileRequest(operation, args);
        offset = end;
    } while (offset < content.length);
    return receipt;
}
/** Sends one bounded file request to Python without exceeding native command argument limits. */
async function v1FileRequest(operation, args) {
    const state = v1RuntimeState();
    const request = JSON.stringify({ operation, args, paths: state.paths });
    const result = await v1Program(state.python + " -c " + v1ShellQuote(v1FileProgram) + " " + v1ShellQuote(request), 60000);
    return JSON.parse(result.output);
}

// The installer supplies the canonical terminal namespace for this internal module.
globalThis.__operitV1InternalRuntime = Object.freeze({
    create_session: v1CreateSession,
    execute: v1Execute,
    terminal: v1Terminal,
    file: v1File,
    run_javascript_node: v1RunNode,
    ffmpeg_execute: v1FfmpegExecute,
    ffmpeg_info: v1FfmpegInfo,
});

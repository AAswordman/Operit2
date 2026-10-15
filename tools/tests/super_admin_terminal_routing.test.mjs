import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
import test from 'node:test';
import vm from 'node:vm';

const root = new URL('../../', import.meta.url);
const androidImplementations = { bash: 'proot', shell: 'android-system' };

/** Reads current production source rather than generated package artifacts. */
function source(path) {
  return readFileSync(new URL(path, root), 'utf8');
}

/** Extracts one Rust method body with balanced braces for bounded contract assertions. */
function rustMethod(text, name) {
  const start = text.indexOf(`fn ${name}(`);
  assert.notEqual(start, -1, `${name} must be implemented`);
  const opening = text.indexOf('{', start);
  let depth = 1;
  let end = opening + 1;
  for (; depth > 0; end++) {
    assert.ok(end < text.length, `${name} must have a balanced body`);
    if (text[end] === '{') depth++;
    if (text[end] === '}') depth--;
  }
  return text.slice(opening + 1, end - 1);
}

/** Loads Super Admin with strict, explicitly typed terminal tools and complete host results. */
function fixture({ platform = 'android', defaultType = 'bash', implementations = androidImplementations, output = 'ok\n', timedOut = false } = {}) {
  const sessions = new Map();
  const calls = [];
  const files = new Map();
  const errors = [];
  let chatId = 'chat-a';
  const terminalInfo = {
    platform,
    terminal: implementations[defaultType],
    terminalType: defaultType,
    types: Object.entries(implementations).map(([terminalType, terminal]) => ({ terminal, terminalType, available: true })),
  };
  const context = vm.createContext({
    exports: {},
    OPERIT_CLEAN_ON_EXIT_DIR: '/app/data/temp/clean_on_exit',
    /** Supplies the active chat identity without a platform-dependent branch. */
    getChatId() { return chatId; },
    console: {
      /** Keeps command logging out of test output. */
      log() {},
      /** Records errors without changing their propagation. */
      error(...args) { errors.push(args); },
    },
    Tools: {
      Files: {
        /** Records creation of the virtual temporary output directory. */
        async mkdir(path, parents) { calls.push({ operation: 'mkdir', path, parents }); },
        /** Saves oversized command output inside the mocked virtual filesystem. */
        async write(path, content, append) {
          assert.equal(append, false);
          files.set(path, content);
        },
      },
      System: {
        terminal: {
          /** Returns capabilities whose default may differ from the selected session. */
          async info() { return terminalInfo; },
          /** Requires exact interpreter selection and keys reusable sessions by name and type. */
          async create(name, terminalType) {
            calls.push({ operation: 'create', name, terminalType });
            if (!Object.hasOwn(implementations, terminalType)) {
              throw new Error(`Unsupported terminal type: ${terminalType}`);
            }
            const key = `${terminalType}:${name}`;
            const isNewSession = !sessions.has(key);
            if (isNewSession) {
              sessions.set(key, { sessionId: `session-${sessions.size + 1}`, sessionName: name, platform, terminal: implementations[terminalType], terminalType });
            }
            return { ...sessions.get(key), isNewSession };
          },
          /** Returns the actual selected session's identity with the command output. */
          async exec(...args) {
            const [sessionId, command] = args;
            calls.push({ operation: 'exec', args });
            const session = [...sessions.values()].find(value => value.sessionId === sessionId);
            assert.ok(session, 'Execution must target an explicitly created session');
            return { ...session, command, output, exitCode: 0, timedOut };
          },
        },
      },
    },
  });
  vm.runInContext(stripTypeScriptTypes(source('plugins/packages/buildin/super_admin.ts')), context);
  return {
    tools: context.exports, sessions, calls, files, errors, terminalInfo,
    /** Changes the active chat to exercise session identity isolation. */
    setChatId(value) { chatId = value; },
  };
}

/** Verifies both top-level metadata and AI environment metadata reflect the actual session. */
function assertEnvironment(result, platform, terminal, terminalType) {
  assert.equal(result.platform, platform);
  assert.equal(result.terminal, terminal);
  assert.equal(result.terminalType, terminalType);
  assert.equal(result.terminalEnvironment.platform, platform);
  assert.equal(result.terminalEnvironment.terminal, terminal);
  assert.equal(result.terminalEnvironment.terminalType, terminalType);
}

/** Ensures Android shell execution selects the native system shell despite the proot default. */
test('Android shell explicitly selects android-system and reports the actual environment', async () => {
  const f = fixture();
  const result = await f.tools.shell({ command: 'id', timeoutMs: 3000 });
  assertEnvironment(result, 'android', 'android-system', 'shell');
  assert.equal(f.terminalInfo.terminal, 'proot', 'Capability discovery must not be mutated');
  const create = f.calls.find(call => call.operation === 'create');
  assert.equal(create.terminalType, 'shell');
  assert.match(create.name, /^super_admin_default_session_shell_chat-a$/);
  const exec = f.calls.find(call => call.operation === 'exec');
  assert.deepEqual(exec.args, [result.sessionId, 'id', 3000]);
});

/** Ensures switching interpreter tools never shares or changes another interpreter's context. */
test('shell and bash use separate sessions while each interpreter reuses its own chat context', async () => {
  const f = fixture();
  const shell = await f.tools.shell({ command: 'pwd' });
  const bash = await f.tools.bash({ command: 'pwd' });
  const shellAgain = await f.tools.shell({ command: 'pwd' });
  assertEnvironment(bash, 'android', 'proot', 'bash');
  assert.notEqual(shell.sessionId, bash.sessionId);
  assert.equal(shell.sessionId, shellAgain.sessionId);
  assert.equal(f.sessions.size, 2);
  f.setChatId('chat-b');
  const otherChat = await f.tools.shell({ command: 'pwd' });
  assert.notEqual(otherChat.sessionId, shell.sessionId);
  assert.equal(f.sessions.size, 3);
});

/** Ensures background commands use the requested interpreter and report creation metadata. */
test('background shell and bash retain distinct types and actual environments', async () => {
  const f = fixture();
  const shell = await f.tools.shell({ command: 'id', background: 'true' });
  const bash = await f.tools.bash({ command: 'pwd', background: 'true' });
  assertEnvironment(shell, 'android', 'android-system', 'shell');
  assertEnvironment(bash, 'android', 'proot', 'bash');
  assert.equal(shell.started, true);
  assert.equal(shell.background, true);
  assert.notEqual(shell.sessionId, bash.sessionId);
  const creates = f.calls.filter(call => call.operation === 'create');
  assert.match(creates[0].name, /^super_admin_background_shell_chat-a_\d+$/);
  assert.match(creates[1].name, /^super_admin_background_bash_chat-a_\d+$/);
  for (const call of f.calls.filter(call => call.operation === 'exec')) {
    assert.equal(call.args.length, 2, 'Background execution must not introduce a foreground timeout');
  }
});

/** Ensures file-backed output cannot replace native shell identity with the host default. */
test('oversized shell output keeps actual terminal metadata alongside its saved output', async () => {
  const output = 'x'.repeat(12001);
  const f = fixture({ output });
  const result = await f.tools.shell({ command: 'printf large' });
  assert.equal(result.output, '(saved_to_file)');
  assert.equal(f.files.get(result.output_saved_to), output);
  assertEnvironment(result, 'android', 'android-system', 'shell');
  assert.equal(result.timeoutMsUsed, 15000);
});

/** Ensures an unsupported exact type produces an error without creating a different terminal. */
test('unsupported explicit interpreters fail without executing any command', async () => {
  const f = fixture();
  await assert.rejects(f.tools.powershell({ command: 'Get-Location' }), /Unsupported terminal type: powershell/);
  assert.equal(f.sessions.size, 0);
  assert.deepEqual(f.calls.map(call => call.operation), ['create']);
  await assert.rejects(f.tools.powershell({ command: 'Get-Location', background: 'true' }), /Unsupported terminal type: powershell/);
  assert.equal(f.sessions.size, 0);
  assert.deepEqual(f.calls.map(call => call.operation), ['create', 'create']);
});

/** Ensures Windows PowerShell and Git Bash share the same type-aware public API. */
test('Windows tools explicitly select PowerShell and Git Bash with separate sessions', async () => {
  const f = fixture({ platform: 'windows', defaultType: 'powershell', implementations: { powershell: 'native', bash: 'native' } });
  const powershell = await f.tools.powershell({ command: 'Get-Location' });
  const bash = await f.tools.bash({ command: 'pwd' });
  assertEnvironment(powershell, 'windows', 'native', 'powershell');
  assertEnvironment(bash, 'windows', 'native', 'bash');
  assert.notEqual(powershell.sessionId, bash.sessionId);
});

/** Ensures shell-only hosts use their advertised interpreter without plugin platform branches. */
test('shell-only hosts keep their configured backend and report the selected environment', async () => {
  for (const [platform, terminal] of [['ios', 'ish'], ['ohos', 'qemu-vroot'], ['web', 'v86']]) {
    const f = fixture({ platform, defaultType: 'shell', implementations: { shell: terminal } });
    const result = await f.tools.shell({ command: 'pwd' });
    assertEnvironment(result, platform, terminal, 'shell');
  }
  for (const platform of ['linux', 'macos']) {
    const f = fixture({ platform, implementations: { bash: 'native' } });
    assertEnvironment(await f.tools.bash({ command: 'pwd' }), platform, 'native', 'bash');
  }
});

/** Ensures timeout handling preserves interpreter identity and the reusable chat session. */
test('timed-out shell commands keep actual environment metadata and their typed session', async () => {
  const f = fixture({ timedOut: true });
  const first = await f.tools.shell({ command: 'sleep 30', timeoutMs: 3000 });
  const second = await f.tools.shell({ command: 'pwd', timeoutMs: 3000 });
  assert.equal(first.timedOut, true);
  assert.equal(first.context_preserved, false);
  assertEnvironment(first, 'android', 'android-system', 'shell');
  assert.equal(first.sessionId, second.sessionId);
});

/** Ensures the public SDK and standard tool facade actually transmit the explicit interpreter. */
test('canonical SDK, declarations, and standard tools expose exact typed session creation', () => {
  const sdk = source('core/crates/plugin/sdk/src/js_sdk/system.rs');
  const terminalTrait = sdk.slice(sdk.indexOf('pub trait SystemTerminalHost'));
  assert.match(terminalTrait, /fn create\([\s\S]*?sessionName: String,[\s\S]*?r#type: Option<TerminalType>/);
  assert.match(source('plugins/types/system.d.ts'), /function create\(sessionName: string, type\?: TerminalType\)/);
  const facade = rustMethod(source('core/crates/tool/services/src/tools/defaultTool/standard/StandardTerminalTools.rs'), 'createOrGetSession');
  assert.match(facade, /optionalParameterValue\(tool, "type"\)/);
  assert.match(facade, /Some\(terminalType\) => host\.createOrGetTypedSession\(&sessionName, terminalType\)/);
  assert.match(facade, /None => host\.createOrGetSession\(&sessionName\)/);
  const bridge = rustMethod(source('apps/flutter/native/operit-flutter-bridge/src/FlutterOwnerCapabilities.rs'), 'createOrGetTypedSession');
  assert.match(bridge, /self\.inner\.createOrGetTypedSession\(sessionName, terminalType\)\?/);
  assert.match(bridge, /self\.publish_sessions\(\)\?/);
});

/** Ensures Android selects and reports the requested implementation rather than a hardcoded default. */
test('Android typed creation validates the interpreter, keys its context, and reports exact identity', () => {
  const android = source('hosts/android/src/terminal.rs');
  const typed = rustMethod(android, 'createOrGetTypedSession');
  assert.match(typed, /nonBlank\(terminalType, "type"\)\?/);
  assert.match(typed, /androidTerminalName\(&normalizedTerminalType\)\?/);
  assert.match(typed, /sessionKey\(&normalizedTerminalType, &normalizedSessionName\)/);
  assert.match(typed, /initialAndroidWorkingDir\(&normalizedTerminalType\)\?/);
  assert.equal([...typed.matchAll(/terminal: terminal\.to_string\(\)/g)].length, 2);
  assert.doesNotMatch(typed, /PRIMARY_TERMINAL_TYPE|terminal: PROOT_TERMINAL/);
  const identities = rustMethod(android, 'androidTerminalName');
  assert.match(identities, /"bash" => Ok\(PROOT_TERMINAL\)/);
  assert.match(identities, /"shell" => Ok\(ANDROID_SYSTEM_TERMINAL\)/);
  const dispatch = rustMethod(android, 'buildAndroidPtyCommand');
  assert.match(dispatch, /"shell" => buildAndroidShellPtyCommand\(workingDir\)/);
  assert.match(dispatch, /"bash" => buildAndroidBashPtyCommand\(workingDir\)/);
});

/** Ensures every host implements the required typed contract without adding a shared platform branch. */
test('all terminal hosts implement exact typed creation using their own host compatibility layer', () => {
  const paths = [
    'hosts/android/src/terminal.rs',
    'hosts/windows/src/tools/terminal/mod.rs',
    'hosts/linux/src/tools/terminal/mod.rs',
    'hosts/apple/src/tools/terminal/terminal_macos.rs',
    'hosts/common/operit-host-native-terminal/src/lib.rs',
    'hosts/ios/src/terminal.rs',
    'hosts/ohos/src/terminal.rs',
    'hosts/web/src/tools/terminal/mod.rs',
  ];
  for (const path of paths) {
    const typed = rustMethod(source(path), 'createOrGetTypedSession');
    assert.match(typed, /terminalType/, path);
    assert.doesNotMatch(typed, /unwrap_or|or_else/, path);
  }
  const windows = rustMethod(source(paths[1]), 'createOrGetTypedSession');
  assert.match(windows, /let \(normalizedTerminalType, kind\) = normalizeTerminalType\(terminalType\)\?/);
  assert.doesNotMatch(windows, /TerminalKind::PowerShell|PRIMARY_TERMINAL_TYPE/);
  const hostApi = source('core/crates/foundation/host-api/src/lib.rs');
  assert.match(hostApi, /fn createOrGetTypedSession\([\s\S]*?terminalType: &str,[\s\S]*?\) -> HostResult<TerminalSessionInfo>;/);
});


/** Exercises production wire parameter serialization with argument names taken from the canonical SDK. */
test('production Tools transport sends exact interpreter types and preserves explicit invalid values', async () => {
  const sdk = source('core/crates/plugin/sdk/src/js_sdk/system.rs');
  const trait = sdk.slice(sdk.indexOf('pub trait SystemTerminalHost'));
  const signature = trait.match(/fn create\(([\s\S]*?)\) -> JsFuture<TerminalSessionCreationResultData>/)[1];
  const names = [...signature.matchAll(/(?:r#)?(\w+):\s*(?:String|Option<TerminalType>)/g)].map(match => match[1]);
  assert.deepEqual(names, ['sessionName', 'type']);
  const generator = source('core/crates/plugin/codegen/src/runtime_bindings.rs');
  const start = generator.indexOf('r#"// Generated from canonical Rust Tools traits and bindings. Do not edit.') + 3;
  assert.ok(start > 3);
  const end = generator.indexOf('"#', start);
  const calls = [];
  const context = vm.createContext({
    __operitToolParameterRules: { flattenArguments: [], scalarUnionSuffixes: [], preserveObjectNamespaces: [], argumentNames: {} },
    /** Records the exact production transport payload rather than mocking positional argument mapping. */
    async toolCall(name, params) {
      calls.push({ name, params: JSON.parse(JSON.stringify(params)) });
      return { sessionId: 'transport-session' };
    },
  });
  vm.runInContext(generator.slice(start, end), context);
  const create = vm.runInContext(`(function(${names.join(', ')}) {
    return __operitInvokeToolsBinding('System.terminal', 'create', 'create_terminal_session',
      [${JSON.stringify(names)}], Array.prototype.slice.call(arguments), []);
  })`, context);
  for (const type of ['shell', 'bash', 'powershell', '']) {
    await create('chat', type);
    assert.deepEqual(calls.at(-1), { name: 'create_terminal_session', params: { session_name: 'chat', type } });
  }
  await create('default-chat');
  assert.deepEqual(calls.at(-1), { name: 'create_terminal_session', params: { session_name: 'default-chat' } });
});

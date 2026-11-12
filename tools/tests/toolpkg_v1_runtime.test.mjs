import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync, rmSync, mkdirSync, readdirSync, statSync, existsSync, renameSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createServer } from 'node:net';
import { stripTypeScriptTypes, createRequire } from 'node:module';
import test from 'node:test';
import vm from 'node:vm';
import { readSdkScript } from '../runtime/compose_scripts.mjs';
import { productionBootstrap } from './fixtures/production_bootstrap.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const sdk = 'core/crates/plugin/sdk/src/';

/** Reads the production implementation rather than generated artifacts. */
function source(path) { return readFileSync(join(root, path), 'utf8'); }

/** Creates a provider fixture with the exact package state and observable host interpreter selection. */
function provider(state, execute) {
  const calls = [];
  const sessions = new Map();
  const context = vm.createContext({
    exports: {}, console,
    /** Supplies the state selected by Rust instead of inferring an operating system. */
    getState() { return state; },
    /** Supplies the actual invoking package identity. */
    getCallerName() { return 'legacy.package'; },
    /** Supplies a stable conversation identity for session isolation. */
    getChatId() { return 'chat-a'; },
    Tools: { System: { terminal: {
      /** Creates and reuses one host session for each explicit name and interpreter. */
      async create(name, type) {
        calls.push({ operation: 'create', name, type });
        const key = type + ':' + name;
        const isNewSession = !sessions.has(key);
        if (isNewSession) sessions.set(key, { sessionId: key, sessionName: name, platform: state, terminal: 'native', terminalType: type });
        return { ...sessions.get(key), isNewSession };
      },
      /** Returns the native execution result without converting failure into success. */
      async exec(sessionId, command, timeoutMs) {
        calls.push({ operation: 'exec', sessionId, command, timeoutMs });
        const session = [...sessions.values()].find(item => item.sessionId === sessionId);
        assert.ok(session);
        const receipt = execute === undefined ? { output: 'actual output', exitCode: 0, timedOut: false } : await execute(command, session);
        return { ...session, command, ...receipt };
      },
    } } },
  });
  vm.runInContext('var __operitV1CanonicalTools = Tools;', context);
  vm.runInContext(source(sdk + 'compat/v1/internal_runtime.js'), context);
  return { context, calls, tools: context.__operitV1InternalRuntime };
}

/** Loads the real dispatcher, public host bridge, Files adapter, and final runtime installer together. */
function adapter(version = '1.0.0', respond) {
  const calls = [];
  const canonicalCalls = [];
  const context = vm.createContext({ version, state: 'linux', __operitCurrentCallId: 'test-call', __calls: calls, __respond: respond,
    /** Provides the active ToolPkg contract on every invocation. */
    __operitGetCallState() { return { params: { __operit_toolpkg_api_version: context.version, containerPackageName: context.owner } }; },
    /** Publishes bootstrap globals just as the execution runtime does. */
    __operitExpose(name, value) { context[name] = value; },
    /** Tracks host requests through the production structured transport. */
    async __operitNativeCallToolStructured(owner, type, name, params) {
      calls.push({ owner, type, name, params: JSON.parse(JSON.stringify(params)) });
      const data = respond === undefined ? JSON.stringify({ sessionId: 'managed-session', output: 'managed-output', path: params.args?.path, operation: params.operation, successful: true, contentBase64: 'AP8=' }) : await respond(name, params);
      return { success: true, data };
    },
    /** Retains the invoking execution while host work is pending. */
    __operitRetainCallReference() {},
    /** Releases the retained invoking execution after its continuation. */
    __operitReleaseCallReference() {},
    /** Restores the owning execution before the host continuation. */
    __operitActivateCall() {},
    /** Snapshots the plain public request through the host-owned transport contract. */
    __operitHostSnapshotJsonObject(value) { return context.__operitSnapshotJsonObject(value); },
    getState() { return context.state; },
    getCallerName() { return 'legacy.package'; },
    getChatId() { return 'chat-a'; },
    canonicalCalls,
  });
  vm.runInContext(readSdkScript('ToolPkgApiRuntimeScript.rs'), context);
  vm.runInContext(`
    var Tools = { System: { terminal: {
      /** Preserves the canonical session API for current plugins. */
      async create(...args) { canonicalCalls.push(['create', ...args]); globalThis.__calls.push({ name: 'create', params: { sessionName: args[0], type: args[1] } }); if (globalThis.version !== '2.0.0' && typeof globalThis.__respond === 'function') return await globalThis.__respond('create', { sessionName: args[0], type: args[1] }); return { sessionId: 'managed-session', platform: globalThis.state, terminal: 'native', terminalType: args[1] }; },
      /** Preserves the canonical execution API for current plugins. */
      async exec(...args) { canonicalCalls.push(['exec', ...args]); const command = String(args[1]); const operation = command.match(/"operation":"([^"]+)"/)?.[1]; const path = command.match(/"path":"([^"]+)"/)?.[1]; const fileArgs = { path }; if (operation === 'write') { fileArgs.content = command.match(/"content":"([^"]*)"/)?.[1]; fileArgs.append = command.includes('"append":true'); } if (operation === 'writeBinary') fileArgs.contentBase64 = command.match(/"contentBase64":"([^"]*)"/)?.[1]; if (operation === 'mkdir') fileArgs.createParents = command.includes('"createParents":true'); const recordParams = operation ? { operation, args: fileArgs } : { sessionId: args[0], command: args[1], timeoutMs: args[2] }; globalThis.__calls.push({ name: operation ? 'file' : 'exec', params: recordParams }); if (globalThis.version !== '2.0.0' && typeof globalThis.__respond === 'function') return await globalThis.__respond('exec', { sessionId: args[0], command: args[1], timeoutMs: args[2] }); const output = operation ? JSON.stringify({ path, operation, successful: true, content: '{}', contentBase64: 'AP8=', size: 2 }) : 'managed-output'; return { sessionId: args[0], command: args[1], output, exitCode: 0, timedOut: false }; },
      /** Uses the shared canonical callback facade, including for v2 dispatch. */
      execStreaming(...args) { canonicalCalls.push(['execStreaming', ...args]); return __operitExecTerminalStreaming(...args); }
    } }, Files: {} };
    for (var name of ['list','read','readPart','readBinary','write','writeBinary','exists','mkdir','deleteFile','move','copy','find','grep','grepContext','info','apply','create','edit','zip','unzip','open','share','download']) {
      /** Installs observable canonical Files methods. */
      (function(method) {
        /** Returns a real-shaped host result for the unchanged canonical request. */
        Tools.Files[method] = async function(...args) {
          canonicalCalls.push([method, ...args]);
          var receipt = { path: typeof args[0] === 'object' ? args[0].path : args[0], contentBase64: 'AP8=', successful: true };
          return method === 'write' ? { operation: receipt, aiDiffInstructions: '' } : receipt;
        };
      })(name);
    }
    var __operitCanonicalTools = { System: { terminal: {...Tools.System.terminal} }, Files: {...Tools.Files} };
    var __operitV1CanonicalTools = __operitCanonicalTools;
  `, context);
  vm.runInContext(source(sdk + 'compat/v1/files.js'), context);
  vm.runInContext(source(sdk + 'JsExecutionRuntimeBridge.script.js'), context);
  vm.runInContext(source(sdk + 'chat_runtime.js'), context);
  vm.runInContext(source(sdk + 'terminal_runtime.js'), context);
  vm.runInContext(source(sdk + 'compat/v1/internal_runtime.js'), context);
  vm.runInContext(source(sdk + 'compat/v1/runtime.js'), context);
  vm.runInContext('__operitInstallV1Runtime(globalThis)', context);
  return { context, calls, canonicalCalls };
}

/** Verifies internal state selection has explicit native and managed interpreter policies. */
test('internal v1 compatibility selects Windows, Linux and OHOS policies and APK shims', async () => {
  assert.equal(existsSync(join(root, 'plugins/packages/buildin/v1_runtime.ts')), false);
  assert.equal(existsSync(join(root, 'core/crates/runtime/application/assets/plugins/buildin/v1_runtime.js')), false);
  for (const [state, type, alpine] of [['windows','bash',false],['linux','bash',false],['macos','bash',false],['android','bash',true],['ios','shell',true],['web','shell',false],['ohos','shell',true]]) {
    const runtime = provider(state);
    await runtime.tools.create_session({ sessionName: 'phone' });
    assert.equal(runtime.calls[0].type, type);
    assert.equal(runtime.calls.some(item => item.operation === 'exec'), alpine);
  }
  const unsupported = provider('other');
  await assert.rejects(unsupported.tools.create_session({ sessionName: 'phone' }), /supported package state/);
});

/** Verifies the runtime source is embedded and the installer runs after toolCall is created. */
test('Rust bootstrap includes the adapter and host capabilities expose OHOS', () => {
  assert.match(source('core/crates/plugin/codegen/src/runtime_bindings.rs'), /"compat\/v1\/runtime\.js"/);
  assert.match(source('core/crates/plugin/codegen/src/runtime_bindings.rs'), /"compat\/v1\/internal_runtime\.js"/);
  assert.doesNotMatch(source('core/crates/plugin/javascript-bridge/src/javascript/JsLibraries.rs'), /compat\/v1\/internal_runtime\.js/);
  assert.doesNotMatch(source('core/crates/plugin/sdk/src/compat/v1/runtime.js'), /v1_runtime:/);
  assert.equal(existsSync(join(root, 'plugins/packages/buildin/v1_runtime.ts')), false);
  assert.match(source('core/crates/tool/services/src/tools/packTool/RuntimePackageManager.rs'), /"platform\.ohos"\.to_string\(\),\s*ConditionValue::Bool\(platformName == "ohos"\)/);
});

/** Guards the complete generated-plus-host bootstrap against duplicate lexical declarations. */
test('production bootstrap compiles with one internal v1 runtime declaration', () => {
  const bootstrap = productionBootstrap();
  assert.equal((bootstrap.match(/const v1RuntimeStates\b/g) ?? []).length, 1);
  assert.doesNotThrow(() => new vm.Script(bootstrap, { filename: 'production-bootstrap.js' }));
});

/** Exercises all public call shapes against the production transport and dynamic version dispatcher. */
test('1.0.0 and 1.0.1 route exact historical package identities and 2.0.0 preserves canonical requests', async () => {
  const runtime = adapter();
  for (const version of ['1.0.0','1.0.1']) {
    runtime.context.version = version;
    for (const expression of [
      "toolCall('super_admin:terminal', {command:'echo ok'})",
      "toolCall('default', 'code_runner:run_javascript_node', {script:'1'})",
      "toolCall({name:'ffmpeg:ffmpeg_info', params:{}})",
      "toolCall('ffmpeg:ffmpeg_execute', {command:'-version'}, {})",
    ]) {
      const result = await vm.runInContext(expression, runtime.context);
      assert.equal(result.output, 'managed-output');
      assert.equal(runtime.calls.some(call => call.name === 'exec'), true);
    }
  }
  await vm.runInContext("toolCall('custom', 'super_admin:terminal', {})", runtime.context);
  assert.equal(runtime.calls.at(-1).name, 'super_admin:terminal');
  runtime.context.version = '2.0.0';
  const result = await vm.runInContext("toolCall('super_admin:terminal', {command:'original'})", runtime.context);
  assert.equal(typeof result, 'string');
  assert.equal(runtime.calls.at(-1).name, 'super_admin:terminal');
});

/** Verifies both API families share the native stream while retaining v1 callback shapes. */
test('terminal streaming delivers live callbacks and final receipts without serializing functions', async () => {
  for (const version of ['1.0.0', '1.0.1', '2.0.0']) {
    const events = [
      { type: 'start', sessionId: 'managed', command: 'printf sample' },
      { type: 'chunk', sessionId: 'managed', command: 'printf sample', chunk: '雪', chunkIndex: 0, receivedChars: 1 },
      { type: 'chunk', sessionId: 'managed', command: 'printf sample', chunk: '🌍', chunkIndex: 1, receivedChars: 3 },
      { type: 'completed', result: { sessionId: 'managed', command: 'printf sample', output: '雪🌍', exitCode: 7, timedOut: false } },
    ];
    const runtime = adapter(version, async (name, params) => {
      assert.equal(name, 'terminal_observe');
      const op = params.observation;
      if (op.op === 'open') { assert.deepEqual(JSON.parse(JSON.stringify(op.request)), { sessionId: 'managed', command: 'printf sample', timeoutMs: 5000 }); return { handle: 'stream' }; }
      if (op.op === 'next') return { event: events.shift() ?? null };
      assert.equal(op.op, 'close'); return {};
    });
    vm.runInContext(`var streamEvents = []; var controls = {timeoutMs:'5000', onIntermediateResult(event) { streamEvents.push(event); }};`, runtime.context);
    const result = await vm.runInContext("Tools.System.terminal.execStreaming('managed', 'printf sample', controls)", runtime.context);
    assert.equal(result.output, '雪🌍'); assert.equal(result.exitCode, 7);
    assert.deepEqual(JSON.parse(JSON.stringify(runtime.context.streamEvents)).map(event => event.type), ['start', 'chunk', 'chunk']);
    assert.equal(typeof runtime.context.controls.onIntermediateResult, 'function');
    assert.equal(runtime.context.controls.timeoutMs, '5000');
    assert.equal(runtime.calls.filter(call => call.params.observation?.op === 'close').length, 1);
    assert.equal(runtime.calls.some(call => JSON.stringify(call.params).includes('onIntermediateResult')), false);
  }
});

test('v1 terminal streams retain the legacy default when timeout is absent or undefined', async () => {
  for (const version of ['1.0.0', '1.0.1']) {
    for (const options of ['undefined', '{}', '{timeoutMs:undefined}']) {
      const runtime = adapter(version, async (_name, params) => {
        const op = params.observation;
        if (op.op === 'open') { assert.equal(op.request.timeoutMs, 15000); return {handle:'default-stream'}; }
        if (op.op === 'next') return {event:{type:'completed',result:{exitCode:0,output:''}}};
        return {};
      });
      await vm.runInContext("Tools.System.terminal.execStreaming('managed','cmd'," + options + ")", runtime.context);
    }
  }
});

/** A callback exception must cancel observation, and validation must precede native admission. */
test('terminal callback failures close the actual stream and malformed options never reach the host', async () => {
  const runtime = adapter('1.0.0', async (_name, params) => {
    switch (params.observation.op) {
      case 'open': return { handle: 'callback-error' };
      case 'next': return { event: { type: 'chunk', chunk: 'progress' } };
      case 'close': return {};
    }
  });
  await assert.rejects(vm.runInContext("Tools.System.terminal.execStreaming('s','cmd',{onIntermediateResult(){throw Error('callback failed');}})", runtime.context), /callback failed/);
  assert.equal(runtime.calls.at(-1).params.observation.op, 'close');
  runtime.calls.length = 0;
  for (const options of ['null', '[]', '{onIntermediateResult:42}', '{timeoutMs:-1}', '{timeoutMs:1.5}']) {
    await assert.rejects(vm.runInContext("Tools.System.terminal.execStreaming('s','cmd'," + options + ')', runtime.context));
  }
  assert.deepEqual(runtime.calls, []);
});

/** Ensures invalid public overloads retain the production argument validation. */
test('legacy aliases cannot make invalid toolCall overloads valid', async () => {
  const runtime = adapter();
  await assert.rejects(vm.runInContext("toolCall('super_admin:terminal', {}, {}, {})", runtime.context), /command must be a nonempty string/);
  await vm.runInContext("toolCall({name:'ffmpeg:ffmpeg_info',params:{},onIntermediateResult:function(){}})", runtime.context);
  assert.equal(runtime.calls.some(call => call.name.startsWith('v1_runtime')), false);
});

/** Checks that failures from the canonical host are not replaced by another implementation. */
test('internal runtime errors propagate without a package transport fallback', async () => {
  const runtime = adapter('1.0.0');
  runtime.context.__operitV1CanonicalTools.System.terminal.create = async () => { throw new Error('actual host failure'); };
  await assert.rejects(vm.runInContext("Tools.System.terminal.create('phone')", runtime.context), /actual host failure/);
  assert.equal(runtime.calls.length, 0);
});

/** Confirms old terminal methods use the selected provider and current methods keep their host identities. */
test('terminal dispatch changes with the active invocation without recursion', async () => {
  const runtime = adapter();
  await vm.runInContext("Tools.System.terminal.create('phone')", runtime.context);
  await vm.runInContext("Tools.System.terminal.exec('managed-session','echo ok','6000')", runtime.context);
  assert.deepEqual(JSON.parse(JSON.stringify(runtime.calls.map(item => [item.name, item.params]))), [
    ['create', {sessionName:'v1_legacy.package_phone',type:'bash'}],
    ['exec', {sessionId:'managed-session',command:'echo ok',timeoutMs:6000}],
  ]);
  runtime.context.version = '2.0.0';
  await vm.runInContext("Tools.System.terminal.create('native','powershell')", runtime.context);
  assert.deepEqual(Array.from(runtime.canonicalCalls.at(-1)), ['create','native','powershell']);
});

/** Keeps legacy Linux file operations and terminal commands in the same managed filesystem. */
test('legacy Files routes exact Linux addresses and transfers resource bytes across environments', async () => {
  const runtime = adapter();
  const written = await vm.runInContext("Tools.Files.write('/root/phone.json','{}',false,'linux')", runtime.context);
  assert.equal(written.path, '/root/phone.json');
  assert.equal(written.env, 'linux');
  assert.equal(written.successful, true);
  assert.deepEqual(JSON.parse(JSON.stringify(runtime.calls.at(-1).params)), { operation:'write',args:{path:'/root/phone.json',content:'{}',append:false} });
  await vm.runInContext("Tools.Files.copy('/operitapp/data/resource.zip','/root/resource.zip',false,'android','linux')", runtime.context);
  assert.deepEqual(JSON.parse(JSON.stringify(runtime.calls.at(-1).params)), { operation:'writeBinary',args:{path:'/root/resource.zip',contentBase64:'AP8='} });
  await vm.runInContext("Tools.Files.copy('/root/resource.zip','/operitapp/data/export.zip',false,'linux','android')", runtime.context);
  assert.deepEqual(Array.from(runtime.canonicalCalls.at(-1)), ['writeBinary','/operitapp/data/export.zip','AP8=']);
  const androidWrite = await vm.runInContext("Tools.Files.write('/sdcard/a.txt','text',false)", runtime.context);
  assert.equal(androidWrite.successful, true);
  assert.equal(androidWrite.path, '/sdcard/a.txt');
  assert.equal(runtime.canonicalCalls.at(-1)[1], '/sdcard/a.txt');
  runtime.context.version = '2.0.0';
  await vm.runInContext("Tools.Files.read('/virtual/linux/current.txt')", runtime.context);
  assert.deepEqual(Array.from(runtime.canonicalCalls.at(-1)), ['read','/virtual/linux/current.txt']);
});

/** Preserves native command exit status and timeout receipts while program helpers reject failed execution. */
test('execution retains host diagnostics and program tools reject nonzero exits and timeouts', async () => {
  const runtime = provider('linux', async () => ({ output:'actual diagnostic',exitCode:17,timedOut:false }));
  const session = await runtime.tools.create_session({sessionName:'phone'});
  const result = await runtime.tools.execute({ sessionId:session.sessionId,command:'fail',timeoutMs:'9000' });
  assert.equal(result.exitCode,17);
  assert.equal(runtime.calls.at(-1).timeoutMs,9000);
  await assert.rejects(runtime.tools.run_javascript_node({script:'fail()'}), /17: actual diagnostic/);
  const timeout = provider('windows', async () => ({ output:'partial',exitCode:0,timedOut:true }));
  await assert.rejects(timeout.tools.run_javascript_node({script:'1'}), /timed out: partial/);
});

/** Discovers exactly the Git Bash executable exposed by the existing Windows host contract. */
function bashExecutable() {
  if (process.platform !== 'win32') return '/bin/bash';
  const gitExec = execFileSync('git', ['--exec-path'], {encoding:'utf8'}).trim();
  return resolve(gitExec, '../../..', 'bin/bash.exe');
}

/** Runs real shell and Python subprocesses with diagnostics available on assertion failures. */
function run(executable, args, options = {}) {
  const result = spawnSync(executable, args, { encoding:'utf8', timeout:20000, ...options });
  if (result.error) throw result.error;
  return result;
}

/** Exercises the real APK argument shim using an argv-recording executable instead of installing packages. */
test('apt grammar maps exact APK package names and preserves actual installer failures', () => {
  const runtime = provider('android');
  const temporary = mkdtempSync(join(tmpdir(), 'operit-v1-apk-'));
  try {
    const bash = bashExecutable();
    const shim = join(temporary, 'apt');
    const record = join(temporary, 'args');
    writeFileSync(shim, vm.runInContext('v1ApkShim', runtime.context));
    writeFileSync(join(temporary,'apk'), '#!/bin/sh\nprintf "%s\\n" "$@" > "$V1_ARGS"\nexit "$V1_EXIT"\n', {mode:0o700});
    const temporaryPosix = process.platform === 'win32' ? execFileSync(bash,['-c','cygpath -u "$1"','path',temporary],{encoding:'utf8'}).trim() : temporary;
    const env = {...process.env, PATH:temporary + requirePathDelimiter() + process.env.PATH, V1_ARGS:record, V1_EXIT:'0'};
    const command = 'export PATH="$1:$PATH"; sh "$1/apt" -y install nodejs python3-pip build-essential libssl-dev';
    assert.equal(run(bash,['-c',command,'apt-test',temporaryPosix],{env}).status,0);
    assert.deepEqual(readFileSync(record,'utf8').trim().split(/\r?\n/), ['add','--no-cache','nodejs','py3-pip','build-base','openssl-dev']);
    env.V1_EXIT = '23';
    assert.equal(run(bash,['-c',command,'apt-test',temporaryPosix],{env}).status,23);
    const rejected = run(bash,['-c','sh "$1/apt" install "nodejs; echo injected"','apt-test',temporaryPosix],{env});
    assert.equal(rejected.status,64);
    assert.match(rejected.stderr,/No v1 APK package mapping/);
  } finally { rmSync(temporary,{recursive:true,force:true}); }
});

/** Supplies the executable search-path delimiter used by the actual test host. */
function requirePathDelimiter() { return process.platform === 'win32' ? ';' : ':'; }

/** Executes quoted Node source through the real PowerShell host on Windows and Bash host on Linux. */
test('Node helpers preserve quotes, dollar expressions and backticks in native interpreters', async () => {
  const bash = bashExecutable();
  const state = process.platform === 'win32' ? 'windows' : 'linux';
  const runtime = provider(state, async (command, session) => {
    const child = session.terminalType === 'powershell'
      ? run('powershell.exe',['-NoProfile','-NonInteractive','-Command','[Console]::OutputEncoding = [Text.Encoding]::UTF8; '+command])
      : run(bash,['-c',command]);
    return {output:child.stdout+child.stderr,exitCode:child.status,timedOut:false};
  });
  const expected = "one' two \" $(printf injected) `not-a-command` 雪";
  const result = await runtime.tools.run_javascript_node({script:'process.stdout.write('+JSON.stringify(expected)+')'});
  assert.equal(result.output,expected);
  assert.equal(runtime.calls[0].type,state === 'windows' ? 'powershell' : 'bash');
});

/** Exercises actual binary, text, copy and ZIP operations in the terminal runtime's Python filesystem. */
test('runtime file helper reads terminal-written data and rejects malformed paths and ZIP traversal', () => {
  const runtime = provider(process.platform === 'win32' ? 'windows' : 'linux');
  const program = vm.runInContext('v1FileProgram',runtime.context);
  const temporary = mkdtempSync(join(tmpdir(),'operit-v1-files-'));
  try {
    const bash = bashExecutable();
    const python = process.platform === 'win32' ? join(root,'.venv/Scripts/python.exe') : 'python3';
    const prefix = process.platform === 'win32' ? execFileSync(bash,['-c','cygpath -u "$1"','path',temporary],{encoding:'utf8'}).trim() : temporary;
    const env = {...process.env};
    if (process.platform === 'win32') env.PATH = resolve(dirname(bash),'../usr/bin') + ';' + env.PATH;
    /** Executes the exact embedded file program against a temporary runtime directory. */
    function file(operation,args) {
      return run(python,['-c',program,JSON.stringify({operation,args,paths:process.platform === 'win32' ? 'msys' : 'posix'})],{env});
    }
    writeFileSync(join(temporary,'terminal.txt'),'terminal output 雪');
    assert.equal(JSON.parse(file('read',{path:prefix+'/terminal.txt'}).stdout).content,'terminal output 雪');
    assert.equal(file('writeBinary',{path:prefix+'/binary',contentBase64:'AP8='}).status,0);
    assert.equal(JSON.parse(file('readBinary',{path:prefix+'/binary'}).stdout).contentBase64,'AP8=');
    assert.notEqual(file('read',{path:prefix+'/missing'}).status,0);
    assert.notEqual(file('read',{path:prefix+'/bad\\path'}).status,0);
    assert.notEqual(file('read',{path:prefix+'/bad\0path'}).status,0);
    const zip = join(temporary,'bad.zip');
    assert.equal(run(python,['-c',"import zipfile,sys; z=zipfile.ZipFile(sys.argv[1],'w'); z.writestr('../escaped','bad'); z.close()",zip]).status,0);
    const extracted = file('unzip',{path:prefix+'/bad.zip',destination:prefix+'/out'});
    assert.notEqual(extracted.status,0);
    assert.match(extracted.stderr,/escapes the extraction directory/);
  } finally { rmSync(temporary,{recursive:true,force:true}); }
});

/** Verifies actual package file writes stay within native argument limits and preserve byte boundaries. */
test('large resource and Unicode text writes use bounded commands and retain exact file contents', async () => {
  const temporary = mkdtempSync(join(tmpdir(),'operit-v1-resource-'));
  try {
    const bash = bashExecutable();
    const prefix = process.platform === 'win32' ? execFileSync(bash,['-c','cygpath -u "$1"','path',temporary],{encoding:'utf8'}).trim() : temporary;
    const env = {...process.env};
    if (process.platform === 'win32') env.PATH = join(root,'.venv/Scripts') + ';' + env.PATH;
    const runtime = provider(process.platform === 'win32' ? 'windows' : 'linux', async command => {
      assert.ok(command.length < 30000, 'Every request must remain below the Windows command-line limit');
      const child = run(bash,['--noprofile','--norc','-s'],{env,input:command});
      return {output:child.stdout+child.stderr,exitCode:child.status,timedOut:false};
    });
    const binary = Buffer.alloc(50000);
    for (let index = 0; index < binary.length; index++) binary[index] = index % 256;
    await runtime.tools.file({operation:'writeBinary',args:{path:prefix+'/resource.zip',contentBase64:binary.toString('base64')}});
    assert.deepEqual(readFileSync(join(temporary,'resource.zip')),binary);
    const text = 'a'.repeat(1023) + '🌍' + "雪\n'\" $() `".repeat(180);
    await runtime.tools.file({operation:'write',args:{path:prefix+'/text',content:text}});
    await runtime.tools.file({operation:'write',args:{path:prefix+'/text',content:'尾',append:true}});
    assert.equal(readFileSync(join(temporary,'text'),'utf8'),text+'尾');
    await runtime.tools.file({operation:'write',args:{path:prefix+'/text',content:''}});
    assert.equal(readFileSync(join(temporary,'text'),'utf8'),'');
  } finally { rmSync(temporary,{recursive:true,force:true}); }
});

/** Keeps the compatibility surface within the published v1 declarations. */
test('v1 exposes declared mkdir and does not invent System.file or Files.makeDirectory', async () => {
  const runtime = adapter();
  for (const version of ['1.0.0', '1.0.1', '2.0.0']) {
    runtime.context.version = version;
    assert.equal(runtime.context.Tools.System.file, undefined);
    assert.equal(runtime.context.Tools.Files.makeDirectory, undefined);
  }
  runtime.context.version = '1.0.0';
  const result = await vm.runInContext("Tools.Files.mkdir('/root/questionnaire', true, 'linux')", runtime.context);
  assert.equal(result.env, 'linux');
  assert.deepEqual(JSON.parse(JSON.stringify(runtime.calls.at(-1).params)), { operation: 'mkdir', args: { path: '/root/questionnaire', createParents: true } });
  await vm.runInContext("toolCall('make_directory', { path: '/sdcard/questionnaire', create_parents: true })", runtime.context);
  assert.equal(runtime.calls.at(-1).name, 'make_directory');
  assert.deepEqual(runtime.calls.at(-1).params, { path: '/sdcard/questionnaire', create_parents: true });
});

/** Checks repeated initialization and nested login Bash commands keep exactly one shim PATH component. */
test('Alpine shims remain available in nested Bash without accumulating PATH entries', async () => {
  const temporary = mkdtempSync(join(tmpdir(),'operit-v1-session-'));
  try {
    const bash = bashExecutable();
    const fixture = provider('android');
    await fixture.tools.create_session({sessionName:'phone'});
    const initialize = fixture.calls.find(item => item.operation === 'exec').command.replaceAll('$HOME','$V1_TEST_ROOT');
    const bin = join(temporary,'.operit-v1/bin');
    mkdirSync(bin,{recursive:true});
    writeFileSync(join(bin,'apk'),'#!/bin/sh\nprintf "%s\\n" "$@"\n',{mode:0o700});
    const prefix = process.platform === 'win32' ? execFileSync(bash,['-c','cygpath -u "$1"','path',temporary],{encoding:'utf8'}).trim() : temporary;
    const command = initialize+'\n'+initialize+'\nbash -lc '+"'apt install nodejs'"+'\nprintf "%s" "$PATH"';
    const result = run(bash,['--noprofile','--norc','-s'],{input:command,env:{...process.env,V1_TEST_ROOT:prefix}});
    assert.equal(result.status,0,result.stderr);
    assert.match(result.stdout,/add\r?\n--no-cache\r?\nnodejs/);
    assert.equal(result.stdout.split(prefix+'/.operit-v1/bin').length-1,1);
  } finally { rmSync(temporary,{recursive:true,force:true}); }
});

/** Connects the production SDK and runtime provider through their real structured transport. */
function cacheFixture(directory, owner = 'v1.cache.test.' + process.pid) {
  const native = provider('linux', command => {
    const result = spawnSync('/bin/bash', ['-c', command], { encoding: 'utf8', timeout: 15000 });
    assert.ifError(result.error);
    return { output: result.stdout, exitCode: result.status, timedOut: false };
  });
  const runtime = adapter('1.0.0', async (name, params) => native.tools[name === 'create' ? 'create_session' : 'execute'](name === 'create' ? { sessionName: params.sessionName } : params));
  runtime.context.owner = owner;
  let hash = 0;
  for (const unit of owner) hash = (Math.imul(hash, 31) + unit.charCodeAt(0)) | 0;
  const cacheName = owner + '-' + (hash >>> 0).toString(16);
  const vfs = '/operitapp/data/cache/toolpkg/' + cacheName;
  const guest = '/tmp/operit-v1/toolpkg/' + cacheName;
  let lists = 0;
  const canonical = runtime.context.__operitCanonicalTools.Files;
  canonical.list = async path => {
    lists++;
    assert.ok(path === vfs || path.startsWith(vfs + '/'));
    const local = directory + path.slice(vfs.length);
    return { path, entries: readdirSync(local).map(name => ({ name, isDirectory: statSync(join(local, name)).isDirectory() })) };
  };
  canonical.readBinary = async path => ({ path, contentBase64: path === vfs + '/.toolpkg-cache-signature'
    ? Buffer.from('fixture-revision-1').toString('base64')
    : readFileSync(directory + path.slice(vfs.length)).toString('base64') });
  return { ...runtime, native, owner, guest, lists: () => lists,
    old: '/data/user/0/com.ai.assistance.operit/files/toolpkg_cache/' + owner + '-*' };
}

/** Runs the exact nested bash/file-test shape emitted by the unmodified marketplace plugin. */
test('v1 cache translation imports through existing file APIs and preserves nested shell quoting', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'operit-cache-source-'));
  mkdirSync(join(directory, 'resources/quizapp/server'), { recursive: true });
  writeFileSync(join(directory, 'resources/quizapp/server/index.js'), '// original resource 雪');
  const runtime = cacheFixture(directory);
  try {
    const quote = value => "'" + value.replaceAll("'", "'\\''") + "'";
    const target = runtime.old + '/resources/quizapp/server/index.js';
    const command = 'bash -lc ' + quote('if [ -f ' + quote(target) + ' ]; then echo ok; else echo missing; fi');
    runtime.context.command = command;
    const [checked, repeated] = await Promise.all([
      vm.runInContext("Tools.System.terminal.exec('',command,5000)", runtime.context),
      vm.runInContext("Tools.System.terminal.exec('',command,5000)", runtime.context),
    ]);
    assert.equal(checked.output.trim(), 'ok');
    assert.equal(repeated.output.trim(), 'ok');
    assert.equal(runtime.lists(), 4); // One import despite concurrent first accesses.
    const expected = command.replaceAll(runtime.old, runtime.guest);
    assert.equal(runtime.calls.find(call => call.name === 'exec' && call.params.command === expected).params.command, expected);
    assert.equal(readFileSync(join(runtime.guest, 'resources/quizapp/server/index.js'), 'utf8'), '// original resource 雪');
    runtime.context.oldFile = runtime.old + '/bridge_config.json';
    await vm.runInContext("Tools.Files.write(oldFile,'{\"port\":4000}')", runtime.context);
    const file = await vm.runInContext('Tools.Files.read(oldFile)', runtime.context);
    assert.equal(file.path, runtime.context.oldFile);
    assert.equal(file.content, '{"port":4000}');
    assert.equal(readFileSync(join(runtime.guest, 'bridge_config.json'), 'utf8'), file.content);
    // A fresh SDK execution reuses the same revision and preserves terminal-written data.
    writeFileSync(join(runtime.guest, 'resources/quizapp/server/index.js'), '// retained runtime data');
    const restarted = cacheFixture(directory);
    restarted.context.command = command;
    await vm.runInContext("Tools.System.terminal.exec('',command,5000)", restarted.context);
    assert.equal(restarted.lists(), 0);
    assert.equal(readFileSync(join(runtime.guest, 'resources/quizapp/server/index.js'), 'utf8'), '// retained runtime data');
    // Refresh a changed package revision in a new SDK execution.
    const updated = cacheFixture(directory);
    const readBinary = updated.context.__operitCanonicalTools.Files.readBinary;
    updated.context.__operitCanonicalTools.Files.readBinary = async path => path.endsWith('/.toolpkg-cache-signature')
      ? { path, contentBase64: Buffer.from('fixture-revision-2').toString('base64') } : readBinary(path);
    updated.context.command = command;
    await vm.runInContext("Tools.System.terminal.exec('',command,5000)", updated.context);
    assert.equal(updated.lists(), 4);
    assert.equal(readFileSync(join(runtime.guest, 'resources/quizapp/server/index.js'), 'utf8'), '// original resource 雪');

    runtime.context.command = "printf '%s\\n' " + quote('space 雪 $HOME `echo unsafe`') + ' ' + quote(runtime.old + '/resource with space.txt');
    const output = await vm.runInContext("Tools.System.terminal.exec('',command,5000)", runtime.context);
    assert.equal(output.output, 'space 雪 $HOME `echo unsafe`\n' + runtime.guest + '/resource with space.txt\n');
  } finally {
    rmSync(runtime.guest, { recursive: true, force: true });
    rmSync(directory, { recursive: true, force: true });
  }
});

/** A version switch or another container must not select the previous package's cache. */
test('cache compatibility is version scoped, owner scoped, and retries failed imports', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'operit-cache-source-'));
  const runtime = cacheFixture(directory);
  try {
    runtime.context.command = 'echo ' + runtime.old;
    runtime.context.version = '2.0.0';
    await vm.runInContext("Tools.System.terminal.exec('native',command)", runtime.context);
    assert.equal(runtime.canonicalCalls.at(-1)[2], runtime.context.command);
    assert.equal(runtime.lists(), 0);
    runtime.context.version = '1.0.1';
    runtime.context.owner = 'another.container';
    await vm.runInContext("Tools.System.terminal.exec('',command)", runtime.context);
    assert.equal(runtime.calls.at(-1).params.command, runtime.context.command);
    assert.equal(runtime.lists(), 0);
    runtime.context.owner = runtime.owner;
    const list = runtime.context.__operitCanonicalTools.Files.list;
    runtime.context.__operitCanonicalTools.Files.list = async () => { throw new Error('source unavailable'); };
    await assert.rejects(vm.runInContext("Tools.System.terminal.exec('',command)", runtime.context), /source unavailable/);
    runtime.context.__operitCanonicalTools.Files.list = list;
    await vm.runInContext("toolCall({name:'super_admin:terminal',params:{command}})", runtime.context);
    assert.equal(runtime.calls.at(-1).params.command, 'echo ' + runtime.guest);
    assert.equal(runtime.lists(), 1);
    assert.ok(existsSync(runtime.guest));
  } finally {
    rmSync(runtime.guest, { recursive: true, force: true });
    rmSync(directory, { recursive: true, force: true });
  }
});

/** Optional local integration against a downloaded, unmodified marketplace archive extraction. */
test('unmodified marketplace package validates its own cached server through v1 adapters', {
  skip: !process.env.OPERIT_V1_MARKET_PACKAGE,
}, async () => {
  const directory = resolve(process.env.OPERIT_V1_MARKET_PACKAGE);
  const manifest = JSON.parse(readFileSync(join(directory, 'manifest.json'), 'utf8'));
  const runtime = cacheFixture(directory, manifest.toolpkg_id);
  const previousTools = globalThis.Tools;
  try {
    globalThis.Tools = runtime.context.Tools;
    const launcher = createRequire(import.meta.url)(join(directory, 'dist/shared/server_launcher_runtime.js'));
    const checked = await launcher.validateServerJsPath(launcher.DEFAULT_SERVER_ENTRY);
    assert.equal(checked.ok, true, checked.message);
    const extracted = await launcher.ensureQuizappExtracted();
    assert.equal(extracted.status, 'already_present', extracted.message);
    assert.equal(extracted.success, true);
  } finally {
    globalThis.Tools = previousTools;
    rmSync(runtime.guest, { recursive: true, force: true });
  }
});

/** Runs the original Node/SQLite service and its public APIs through the v1 terminal adapter. */
test('unmodified Ferry server starts with SQLite, serves chat APIs, and runs its import tool', {
  skip: !process.env.OPERIT_V1_MARKET_PACKAGE,
}, async () => {
  const directory = resolve(process.env.OPERIT_V1_MARKET_PACKAGE);
  const manifest = JSON.parse(readFileSync(join(directory, 'manifest.json'), 'utf8'));
  const runtime = cacheFixture(directory, manifest.toolpkg_id);
  const previousTools = globalThis.Tools;
  const previousPath = process.env.PATH;
  const testBin = mkdtempSync(join(tmpdir(), 'operit-v1-test-bin-'));
  writeFileSync(join(testBin, 'timeout'), '#!/bin/sh\n[ "$1" = 2 ] && shift\nexec "$@"\n', { mode: 0o700 });
  const bash = bashExecutable();
  const portProbe = createServer();
  await new Promise(resolve => portProbe.listen(0, '127.0.0.1', resolve));
  const port = portProbe.address().port;
  await new Promise(resolve => portProbe.close(resolve));
  try {
    process.env.PATH = testBin + requirePathDelimiter() + previousPath;
    globalThis.Tools = runtime.context.Tools;
    const require = createRequire(join(directory, 'package.json'));
    const launcher = require('./dist/shared/server_launcher_runtime.js');
    const started = await launcher.ensureFerryServer({ port, serverJsPath: launcher.DEFAULT_SERVER_ENTRY });
    const serverLog = join(runtime.guest, 'logs/quizapp.log');
    let directHealth = 'unreachable';
    try { directHealth = `${(await fetch(`http://127.0.0.1:${port}/api/chat/health`)).status}`; } catch (error) { directHealth = String(error); }
    assert.equal(started.success, true, `${started.message}\ndirect HTTP: ${directHealth}\n${existsSync(serverLog) ? readFileSync(serverLog, 'utf8') : 'server log was not created'}`);
    assert.equal(started.status, 'started');

    const request = async (path, method = 'GET', body) => {
      const response = await fetch(`http://127.0.0.1:${port}${path}`, {
        method,
        headers: body === undefined ? {} : { 'content-type': 'application/json' },
        body: body === undefined ? undefined : JSON.stringify(body),
      });
      return { status: response.status, body: await response.json() };
    };
    const health = await request('/api/chat/health');
    assert.equal(health.status, 200);
    assert.equal(health.body.code, 0);

    const question = await request('/api/admin/questions', 'POST', {
      type: 'single', code: 'v1-test', question: 'Adapter DB check',
      options: ['yes', 'no'], answer: ['yes'], explanation: '', subject: 'adapter-test',
    });
    assert.equal(question.status, 200);
    assert.equal(question.body.code, 0);
    const listed = await request('/api/admin/questions');
    assert.ok(listed.body.data.some(item => item.code === 'v1-test'));

    const userMessage = await request('/api/chat/send', 'POST', { content: 'integration prompt', page_context: { page: 'test' } });
    assert.equal(userMessage.body.code, 0);
    const pending = await request('/api/chat/pending');
    assert.equal(pending.body.data.length, 1);
    assert.equal(pending.body.data[0].content, 'integration prompt');
    await request('/api/chat/reply', 'POST', { content: 'integration reply' });
    const messages = await request('/api/chat/messages');
    assert.deepEqual(messages.body.data.map(item => item.content), ['integration prompt', 'integration reply']);

    const admin = require('./dist/packages/ferry_admin.js');
    const imported = await admin.list_questions({ limit: 20 });
    assert.equal(imported.success, true, imported.message);
    assert.ok(imported.rows.some(item => item.code === 'v1-test'));

    const chatContext = vm.createContext({});
    vm.runInContext(source(sdk + 'compat/v1/chat.js'), chatContext);
    chatContext.currentChat = {
      async listAll() { return { currentChatId: 'adapter-test-chat', chats: [] }; },
      async *sendMessageStreaming() { yield { type: 'completed', result: { aiResponse: 'unused' } }; },
    };
    const sendRequests = [];
    chatContext.sendLegacy = async params => {
      sendRequests.push(JSON.parse(JSON.stringify(params)));
      return { chatId: params.chat_id, message: params.message, aiResponse: '<think>private</think>Ferry adapter reply', sentAt: Date.now() };
    };
    vm.runInContext(`globalThis.legacyChat = __operitCreateV1Chat(currentChat, async () => ({}), sendLegacy, async () => ({ cards: [] }));`, chatContext);
    globalThis.Tools.Chat = {
      async listAll() { return { currentChatId: 'adapter-test-chat', chats: [] }; },
      sendMessageStreaming(...args) { return chatContext.legacyChat.sendMessageStreaming(...args); },
    };
    globalThis.Tools.Net = {
      async httpGet(url) {
        const response = await fetch(url);
        return { statusCode: response.status, content: await response.text() };
      },
      async httpPost(url, body) {
        const response = await fetch(url, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) });
        return { statusCode: response.status, content: await response.text() };
      },
    };
    const poller = require('./dist/shared/chat_poller.js');
    poller.applyBridgeBase(`http://127.0.0.1:${port}`);
    await request('/api/chat/send', 'POST', { content: 'poller compatibility check' });
    await poller.pollOnce('compat-test');
    const replied = await request('/api/chat/messages');
    assert.equal(replied.body.data.at(-1).content, 'Ferry adapter reply');
    assert.equal(sendRequests.length, 1);
    assert.equal(sendRequests[0].sender_name, '浏览器用户');
    assert.equal(sendRequests[0].disable_warning, true);
    assert.ok(sendRequests[0].timeout_ms > 0);
    assert.equal(Object.hasOwn(sendRequests[0], 'onIntermediateResult'), false);
    assert.equal((await request('/api/chat/pending')).body.data.length, 0);
    await poller.pollOnce('no-duplicate');
    assert.equal(sendRequests.length, 1);
    chatContext.sendFailure = new Error('host AI timeout');
    chatContext.sendLegacy = async () => { throw chatContext.sendFailure; };
    // The closure above captures its native send function; install a fresh adapter for failure propagation.
    vm.runInContext(`globalThis.legacyChat = __operitCreateV1Chat(currentChat, async () => ({}), sendLegacy, async () => ({ cards: [] }));`, chatContext);
    await request('/api/chat/send', 'POST', { content: 'timeout check' });
    await assert.rejects(poller.pollOnce('timeout-test'), /host AI timeout/);
    assert.match((await request('/api/chat/messages')).body.data.at(-1).content, /host AI timeout/);
  } finally {
    const entry = join(runtime.guest, 'resources/quizapp/server/index.js');
    const processes = execFileSync('ps', ['-eo', 'pid=,args='], { encoding: 'utf8' })
      .split(/\r?\n/)
      .map(line => /^\s*(\d+)\s+(.*)$/.exec(line))
      .filter(match => match && match[2].includes(entry) && !match[2].includes('ps -eo'));
    for (const match of processes) {
      try { process.kill(Number(match[1]), 'SIGTERM'); } catch { /* already exited */ }
    }
    await new Promise(resolve => setTimeout(resolve, 300));
    process.env.PATH = previousPath;
    globalThis.Tools = previousTools;
    rmSync(runtime.guest, { recursive: true, force: true });
    rmSync(testBin, { recursive: true, force: true });
  }
});

/** Keeps the documented legacy namespace and package call route on the same implementation. */
test('v1 FFmpeg namespace exposes execute, info and convert with version-scoped dispatch', async () => {
  const runtime = adapter();
  for (const version of ['1.0.0', '1.0.1']) {
    runtime.context.version = version;
    for (const expression of ['Tools.FFmpeg.info()', 'Tools.FFmpeg.execute("-version")']) {
      const result = await vm.runInContext(expression, runtime.context);
      assert.equal(result.returnCode, 0);
      assert.equal(result.output, 'managed-output');
    }
    assert.equal(typeof runtime.context.Tools.FFmpeg.convert, 'function');
  }
  runtime.context.version = '2.0.0';
  assert.equal(runtime.context.Tools.FFmpeg, undefined);
});

/** Existing phone runtime images must acquire the codec instead of assuming it was bundled. */
test('v1 FFmpeg prepares Alpine codecs once and retries failed preparation', async () => {
  let preparations = 0;
  const runtime = provider('android', command => {
    if (command.includes('apk add --no-cache ffmpeg')) {
      preparations++;
      if (preparations === 1) return { output: 'temporary package mirror failure', exitCode: 1, timedOut: false };
    }
    return { output: 'ffmpeg fixture version', exitCode: 0, timedOut: false };
  });
  await assert.rejects(runtime.tools.ffmpeg_info(), /temporary package mirror failure/);
  await runtime.tools.ffmpeg_info();
  await runtime.tools.ffmpeg_info();
  assert.equal(preparations, 2);
});

/** Runs the real compatibility bridge and runtime subprocesses over an isolated host VFS. */
function voiceFixture(directory) {
  const ffmpeg = process.env.OPERIT_TEST_FFMPEG;
  const native = provider('linux', command => {
    const result = spawnSync('/bin/sh', ['-c', command], {
      encoding: 'utf8', maxBuffer: 8 * 1024 * 1024,
      env: { ...process.env, PATH: dirname(ffmpeg) + ':' + process.env.PATH },
    });
    if (result.error) throw result.error;
    return { output: result.stdout + result.stderr, exitCode: result.status, timedOut: false };
  });
  const runtime = adapter('1.0.0', async (name, params) => native.tools[name === 'create' ? 'create_session' : 'execute'](name === 'create' ? { sessionName: params.sessionName } : params));
  const canonical = runtime.context.__operitCanonicalTools.Files;
  function local(path) {
    assert.match(path, /^\/(?:sdcard|operitapp|app)\//);
    const target = resolve(directory, path.slice(1));
    assert.ok(target.startsWith(directory + '/'));
    return target;
  }
  canonical.exists = async path => ({ path, exists: existsSync(local(path)) });
  canonical.readBinary = async path => ({ path, contentBase64: readFileSync(local(path)).toString('base64') });
  canonical.writeBinary = async (path, base64) => {
    writeFileSync(local(path), Buffer.from(base64, 'base64'));
    return { path, successful: true };
  };
  canonical.mkdir = async path => { mkdirSync(local(path), { recursive: true }); return { path, successful: true }; };
  canonical.write = async (path, content) => {
    mkdirSync(dirname(local(path)), { recursive: true });
    writeFileSync(local(path), content);
    return { operation: { path, successful: true } };
  };
  canonical.read = async options => ({ path: options.path, content: readFileSync(local(options.path), 'utf8') });
  canonical.copy = async (from, to) => { writeFileSync(local(to), readFileSync(local(from))); return { path: from, successful: true }; };
  canonical.move = async (from, to) => { renameSync(local(from), local(to)); return { path: from, successful: true }; };
  return { ...runtime, native, canonical, local };
}

/** Verifies actual decoding, VFS publication, duration probing, failure propagation and cleanup. */
test('v1 FFmpeg converts and probes virtual voice files through real subprocesses', { skip: !process.env.OPERIT_TEST_FFMPEG }, async () => {
  const directory = mkdtempSync(join(tmpdir(), 'operit-voice-vfs-'));
  const before = readdirSync(tmpdir()).filter(name => name.startsWith('operit-v1-ffmpeg-')).sort();
  try {
    const runtime = voiceFixture(directory);
    const input = '/storage/emulated/0/Download/voice_bar_voices/原声.mp3';
    const output = "/sdcard/Download/voice_bar_voices/测试 & 100%20 ' $(touch injected).wav";
    await runtime.context.Tools.Files.mkdir('/sdcard/Download/voice_bar_voices', true);
    execFileSync(process.env.OPERIT_TEST_FFMPEG, ['-v', 'error', '-f', 'lavfi', '-i', 'sine=frequency=440:duration=0.3', runtime.local(input.replace('/storage/emulated/0', '/sdcard'))]);
    const converted = await runtime.context.Tools.FFmpeg.convert(input, output, { audio_codec: 'pcm_s16le' });
    assert.equal(converted.returnCode, 0);
    assert.equal(readFileSync(runtime.local(output)).subarray(0, 4).toString(), 'RIFF');
    assert.equal(existsSync(join(directory, 'injected')), false);
    const probed = await runtime.context.Tools.FFmpeg.execute(`-hide_banner -i "${output}" -f null - 2>&1 || true`);
    assert.match(probed.output, /Duration: 00:00:00\.30/);
    assert.ok(probed.output.includes(output));
    assert.doesNotMatch(probed.output, /operit-v1-ffmpeg-/);
    await assert.rejects(runtime.context.Tools.FFmpeg.execute(`-i "${output}" -c:a nonexistent-codec "${output}.failed.wav"`), /V1 runtime command exited/);
    assert.equal(existsSync(runtime.local(output + '.failed.wav')), false);
    assert.deepEqual(readdirSync(tmpdir()).filter(name => name.startsWith('operit-v1-ffmpeg-')).sort(), before);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});

/** Exercises the untouched marketplace sender with synthetic provider bytes and no network credentials. */
test('unmodified AI Voice Mouth synthesizes, converts and reuses cached voice audio', { skip: !process.env.OPERIT_TEST_FFMPEG || !process.env.OPERIT_VOICE_BAR_FIXTURE }, async () => {
  const directory = mkdtempSync(join(tmpdir(), 'operit-voice-market-'));
  try {
    const runtime = voiceFixture(directory);
    const mp3 = join(directory, 'provider.mp3');
    execFileSync(process.env.OPERIT_TEST_FFMPEG, ['-v', 'error', '-f', 'lavfi', '-i', 'sine=frequency=440:duration=0.3', mp3]);
    const payload = readFileSync(mp3).toString('base64');
    const environment = { VOICE_BAR_PROVIDER: 'elevenlabs', VOICE_BAR_ELEVENLABS_API_KEY: 'synthetic-test-key', VOICE_BAR_ELEVENLABS_VOICE_ID: 'synthetic-voice' };
    runtime.context.getEnv = key => environment[key] || '';
    let requests = 0, result;
    runtime.context.Tools.Net = { http: async () => { requests++; return { statusCode: 200, contentBase64: payload }; } };
    runtime.context.Buffer = Buffer;
    runtime.context.ToolPkg = { _m() {} };
    runtime.context.complete = value => { result = value; };
    const cache = new Map(), fixture = resolve(process.env.OPERIT_VOICE_BAR_FIXTURE);
    function load(path) {
      path = resolve(path);
      assert.ok(path.startsWith(fixture + '/'));
      if (cache.has(path)) return cache.get(path).exports;
      const module = { exports: {} }; cache.set(path, module);
      vm.runInContext('(function(exports,module,require){' + readFileSync(path, 'utf8') + '\n})', runtime.context, { filename: path })(module.exports, module, name => load(resolve(dirname(path), name)));
      return module.exports;
    }
    await runtime.context.Tools.Files.write('/sdcard/Download/Operit/voice_bar/config.json', '{}');
    const plugin = load(join(fixture, 'dist/packages/voice_bar.js'));
    await plugin.say({ text: '兼容性测试' });
    assert.equal(result.success, true, JSON.stringify(result));
    assert.match(result.voice_tag, /^<voice src=/);
    assert.equal(readFileSync(runtime.local(result.path.replace('/storage/emulated/0', '/sdcard'))).subarray(0, 4).toString(), 'RIFF');
    assert.equal(result.duration, 0.3);
    await plugin.say({ text: '兼容性测试' });
    assert.equal(result.success, true);
    assert.equal(result.cached, true);
    assert.equal(requests, 1);
    await plugin.say_fav({ text: '语音收藏兼容测试', title: '收藏测试' });
    assert.equal(result.success, true, JSON.stringify(result));
    assert.equal(result.favorite_saved, true, JSON.stringify(result));
    const favorites = JSON.parse(readFileSync(runtime.local(result.favorite_data_file), 'utf8'));
    assert.equal(favorites.items.at(-1).audio_path, result.path);
    assert.equal(favorites.items.at(-1).text, '语音收藏兼容测试');
  } finally { rmSync(directory, { recursive: true, force: true }); }
});

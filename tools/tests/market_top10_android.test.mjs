import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { readFileSync, existsSync, readdirSync } from 'node:fs';
import { join, resolve, dirname, relative } from 'node:path';
import { installScopedHostRuntime, installGeneratedToolsRuntime } from './fixtures/scoped_host_runtime.mjs';
import { readSdkScript, composeContextScript } from '../runtime/compose_scripts.mjs';

// External market artifacts are deliberately not vendored. The audit report records versions/hashes.
const directory = process.env.OPERIT_MARKET_TOP10_DIR;
const root = resolve(new URL('../../', import.meta.url).pathname);
const availableRanks = ['01', '02', '03', '04', '05', '06', '07', '08', '10'];
const read = path => readFileSync(join(root, path), 'utf8');

/** Loads actual SDK/registration/Java/HTTP/Compose sources over controlled native boundaries. */
function engine(rank, respond = async name => { throw Error('Unexpected host call: ' + name); }) {
  const folder = join(directory, rank, 'unpacked');
  const manifest = JSON.parse(readFileSync(join(folder, 'manifest.json'), 'utf8'));
  const calls = [], handlers = new Map(), env = new Map();
  const context = vm.createContext({console, getState: () => 'android', getEnv: key => env.get(key),
    getCallerName: () => manifest.toolpkg_id, getChatId: () => 'chat-one',
    setTimeout, clearTimeout, sendIntermediateResult: () => {}, complete: value => { context.completed = value; },
    __operitNativeGetEnvForCall: (_, key) => env.get(key) ?? null,
    __operitNativeSetEnv: (_, key, value) => { env.set(key, value); },
    __operitNativeJavaClassExists: () => false,
    __operitNativeJavaGetApplicationContext: () => ({__javaClass: 'android.app.Application', __javaHandle: 'context'}),
    __operitNativeJavaGetCurrentActivity: () => null,
    __operitNativeJavaCallInstance: () => { throw Error('Unsupported Java instance'); },
    __operitNativeJavaNewInstance: name => ({__javaClass: name, __javaHandle: name + '-handle'}),
    __operitNativeJavaCallStatic: name => { throw Error('Class not found: ' + name); },
    __operitNativeJavaCallStaticSuspend: name => { throw Error('Class not found: ' + name); },
    __operitNativeJavaGetStaticField: name => { throw Error('Class not found: ' + name); },
    __operitNativeJavaSetStaticField: name => { throw Error('Class not found: ' + name); },
    __operitToolPkgIpc: {on(channel, handler) { handlers.set(channel, handler); },
      async call(channel, payload) { return await handlers.get(channel)(payload); }},
  });
  const registry = installScopedHostRuntime(context);
  vm.runInContext('globalThis.window = globalThis', context);
  context.__operitNativeCallToolStructured = registry.binding((requestId, type, name, params) => {
    calls.push({type, name, params: structuredClone(params)});
    queueMicrotask(async () => {
      try { registry.settle(requestId, {success: true, data: await respond(name, params)}, false); }
      catch (error) { registry.settle(requestId, {success: false, message: error.message}, false); }
    });
  });
  context.__operitNativeReadToolPkgResource = registry.binding((requestId) => {
    queueMicrotask(() => registry.settle(requestId, '/operitapp/data/cache/toolpkg/resource.dex', false));
  });
  const java = read('core/crates/plugin/javascript-bridge/src/javascript/JsJavaBridge.rs');
  vm.runInContext(java.slice(java.indexOf('r#"') + 3, java.indexOf('"#')), context);
  installGeneratedToolsRuntime(context, manifest.api_version ?? '1.0.0');
  context.__operitGetCallState = () => ({params: {__operit_toolpkg_api_version: context.version,
    containerPackageName: manifest.toolpkg_id, toolPkgId: manifest.toolpkg_id, __operit_package_state: 'android', __operit_package_caller_name: manifest.toolpkg_id, __operit_package_chat_id: 'chat-one'}});
  vm.runInContext(readSdkScript('ToolPkgRegistrationBridge.rs').replace('__OPERIT_TOOLPKG_REGISTRATION_ONLY__', 'false'), context);
  vm.runInContext(read('core/crates/plugin/javascript-bridge/src/javascript/OkHttp3.script.js'), context);
  vm.runInContext(composeContextScript(), context);
  vm.runInContext('__operitInstallV1Runtime(globalThis)', context);
  const modules = new Map();
  let active;
  context.__operitGetActiveModuleExports = () => active;
  function load(path) {
    const absolute = resolve(folder, path);
    assert.ok(absolute.startsWith(folder + '/'));
    if (modules.has(absolute)) return modules.get(absolute).exports;
    const module = {exports: {}};
    modules.set(absolute, module);
    const before = active;
    active = module.exports;
    try {
      const factory = vm.runInContext('(function(exports,require,module){\n' + readFileSync(absolute, 'utf8') + '\n})', context, {filename: absolute});
      factory(module.exports, request => {
        if (!request.startsWith('.')) throw Error('Cannot resolve module "' + request + '"');
        return load(relative(folder, resolve(dirname(absolute), request)));
      }, module);
      context.OperitToolPkgExports.tag(relative(folder, absolute), module.exports);
      return module.exports;
    } finally { active = before; }
  }
  const main = load(manifest.main);
  active = main;
  return {context, manifest, main, load, calls, handlers, env, run: code => vm.runInContext(code, context),
    register() { return main.registerToolPkg?.(); }};
}

for (const rank of availableRanks) {
  test(`market #${Number(rank)}: unmodified main and tool modules load and register`, {skip: !directory}, async () => {
    const h = engine(rank);
    await h.register();
    for (const pkg of h.manifest.subpackages ?? []) h.load(pkg.entry);
    const capture = h.context.__operitToolPkgRegistrationCapture;
    assert.ok(Object.values(capture).some(value => Array.isArray(value) && value.length > 0));
    console.log('market registration', rank, h.manifest.toolpkg_id,
      Object.entries(capture).filter(([,v]) => Array.isArray(v) && v.length).map(([k,v]) => `${k}=${v.length}`).join(' '));
  });
}

test('market #2: original lock IPC and hooks isolate chats and unlock without model calls', {skip: !directory}, async () => {
  const h = engine('02', async name => { assert.equal(name, 'list_chats'); return {currentChatId: 'chat-one', chats: []}; });
  await h.register();
  const tools = h.load('packages/conversation_lock.js');
  await tools.lock_conversation({duration: '30m', reason: 'test'});
  assert.equal(h.context.completed.success, true);
  assert.equal(h.main.onChatInput({eventName: 'submit_requested', eventPayload: {chatId: 'chat-one'}}).action, 'block');
  assert.equal(h.main.onChatInput({eventName: 'submit_requested', eventPayload: {chatId: 'chat-two'}}), null);
  await tools.unlock_conversation();
  assert.equal(h.main.onChatInput({eventName: 'submit_requested', eventPayload: {chatId: 'chat-one'}}), null);
  await tools.lock_conversation({duration: '30m', reason: 'restart persistence check'});
  assert.equal(h.main.onChatInput({eventName: 'submit_requested', eventPayload: {chatId: 'chat-one'}}).action, 'block');
  // fs is deliberately unavailable, matching the ToolPkg loader rather than injecting Node's filesystem.
  const restarted = engine('02'); await restarted.register();
  assert.equal(restarted.main.onChatInput({eventName: 'submit_requested', eventPayload: {chatId: 'chat-one'}}), null);
});

test('market #6: original XHS reader extracts a text note through the actual OkHttp adapter', {skip: !directory}, async () => {
  const note = {title: 'audit title', desc: 'audit description', user: {nickName: 'author'}, imageList: []};
  const html = '<script>window.__INITIAL_STATE__=' + JSON.stringify({note: {noteDetailMap: {one: {note}}}}) + '</script>';
  const h = engine('06', async (name) => { assert.equal(name, 'http_request'); return {statusCode: 200, content: html, contentBase64: null, headers: {}, url: 'https://x', statusMessage: 'OK', contentType: 'text/html', size: html.length, cookies: {}}; });
  const tools = h.load('packages/xhsreader.js');
  await tools.read_xiaohongshu({url: 'https://www.xiaohongshu.com/explore/test'});
  assert.match(JSON.stringify(h.context.completed), /audit title/);
  assert.match(JSON.stringify(h.context.completed), /audit description/);
  assert.equal(h.calls.filter(c => c.name === 'http_request').length, 1);
});

test('market #8: original Moodlet force directive and XML renderer run', {skip: !directory}, async () => {
  const h = engine('08');
  h.env.set('MOODLET_ENABLED', 'true');
  h.env.set('MOODLET_FORCE_MODE', 'true');
  const system = h.main.onSystemPromptCompose({eventName: 'after_compose_system_prompt', eventPayload: {systemPrompt: 'original'}});
  assert.ok(typeof system === 'string' && system.startsWith('original'));
  assert.match(system, /silent/);
  const rendered = h.main.onXmlRender({eventPayload: {tagName: 'silent', xmlContent: '<silent mood="happy">test</silent>'}});
  assert.equal(rendered.handled, true);
  assert.equal(typeof rendered.composeDsl.screen, 'function');
});

function contextSafeWebView(context) {
  context.__operitNativeComposeWebViewControllerCommand = () => ({success: true, data: {}});
}

/** Renders with the real Compose context; Android widgets/WebViews still need a device check. */
for (const rank of ['01', '04', '05', '06', '08', '10']) {
  test(`market #${Number(rank)}: original UI screens build a Compose frame`, {skip: !directory}, async () => {
    const h = engine(rank);
    const folder = join(directory, rank, 'unpacked');
    const screens = readdirSync(folder, {recursive: true}).filter(path => path.endsWith('.ui.js'));
    for (const path of screens) {
      const loaded = h.load(path);
      const screen = loaded.default || loaded.Screen || loaded;
      if (typeof screen !== 'function') continue;
      const runtime = h.context.OperitComposeDslRuntime.createContext({
        packageName: h.manifest.toolpkg_id, toolPkgId: h.manifest.toolpkg_id,
        __operit_call_runtime: {getEnv: key => h.env.get(key)},
      });
      contextSafeWebView(h.context);
      const tree = await screen(runtime.ctx);
      const frame = runtime.composition.finish(tree);
      assert.ok(frame, path);
    }
    console.log('market UI', rank, screens.join(' '));
  });
}

/** Models only native receipts; the unmodified plugin still constructs and handles every request. */
function terminalReceipt(name, params, output) {
  if (name === 'create_terminal_session') return {sessionId: 'session', sessionName: params.session_name,
    terminalType: params.type, platform: 'android', terminal: params.type === 'shell' ? 'android-system' : 'proot', isNewSession: true};
  if (name === 'execute_in_terminal_session') return {sessionId: params.session_id, command: params.command,
    output, exitCode: 0, timedOut: false};
  if (name === 'close_terminal_session') return {sessionId: params.session_id, success: true, message: 'closed'};
  return null;
}

test('market #1: original precise usage tool decodes the v1 shell receipt and sums events', {skip: !directory}, async () => {
  const now = Date.now();
  function localTime(ms) { const d = new Date(ms); const pad = n => String(n).padStart(2, '0');
    return `${d.getFullYear()}-${pad(d.getMonth()+1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`; }
  const output = `time="${localTime(now-20*60000)}" type=ACTIVITY_RESUMED package=com.example.demo
` +
    `time="${localTime(now-10*60000)}" type=ACTIVITY_PAUSED package=com.example.demo
`;
  const h = engine('01', async (name, params) => terminalReceipt(name, params, output));
  await h.load('packages/gentle_guardian_tools.js').get_precise_usage({since_hours: 1});
  assert.equal(h.context.completed.success, true);
  assert.equal(h.context.completed.data.apps[0].package, 'com.example.demo');
  assert.equal(h.context.completed.data.apps[0].minutes, 10);
});

test('market #7: original freeze/query/unfreeze handles structured shell receipts and timer confirmation', {skip: !directory}, async () => {
  const h = engine('07', async (name, params) => {
    if (name === 'make_directory' || name === 'copy_file') return {successful: true, path: params.path ?? params.source ?? params.destination ?? '/operitapp/data/mock'};
    let output = '';
    if (params.command?.startsWith('pm list packages')) output = 'package:com.example.demo\n';
    else if (params.command?.includes('nohup')) output = '__APP_SUSPENDER_TIMER_STARTED__\n';
    else if (params.command?.includes('app_process')) output = '__APP_SUSPENDER_RESULT__|OK|' + (params.command.includes("'\\''unsuspend'\\''") ? 'ACTIVE' : 'SUSPENDED') + '\n';
    const receipt = terminalReceipt(name, params, output);
    assert.ok(receipt, name);
    return receipt;
  });
  const tools = h.load('dist/packages/app_suspender.js');
  assert.equal((await tools.get_app_suspend_status({package_name: 'com.example.demo'})).success, true);
  const suspended = await tools.suspend_app({package_name: 'com.example.demo', auto_unsuspend_after_seconds: '60'});
  assert.equal(suspended.success, true);
  assert.equal(suspended.suspended, true);
  assert.equal(suspended.status, 'suspended');
  assert.equal(suspended.auto_unsuspend_after_seconds, 60);
  const active = await tools.unsuspend_app({package_name: 'com.example.demo'});
  assert.equal(active.success, true);
  assert.equal(active.suspended, false);
  assert.equal(active.status, 'active');
  assert.ok(h.calls.filter(c => c.name === 'create_terminal_session').every(c => c.params.type === 'shell'));
});

test('market #5: pure ChatGPT conversion works; one-click import still needs removed v1 Java classes', {skip: !directory}, async () => {
  const h = engine('05', async (name, params) => terminalReceipt(name, params, 'OK'));
  const converter = h.load('dist/packages/converters/chat_gpt.js');
  const result = await converter.convertChatGPTToOperit([{title: 'audit chat', mapping: {
    one: {message: {author: {role: 'user'}, content: {parts: ['hello']}, create_time: 1780000000}},
    two: {message: {author: {role: 'assistant'}, content: {parts: ['world']}, create_time: 1780000001}},
  }}]);
  assert.equal(result.chatCount, 1);
  assert.equal(result.messageCount, 2);
  h.context.__operitNativeJavaCallStatic = (name) => {
    if (name === 'android.net.Uri') return {__javaClass: name, __javaHandle: 'uri'};
    throw Error('Class not found: ' + name);
  };
  const factory = h.load('dist/packages/operit_archive_importer.js').createOperitArchiveImporter;
  const importer = factory({Tools: h.context.Tools, SESSION_ID: 'audit', JavaBridge: h.context.Java,
    shapeError: message => ({success: false, message}), shellQuote: value => "'" + value + "'",
    describePathStatus: () => '', describeReadError: (_, error) => String(error), errorText: error => String(error), getFileContent: () => ''});
  const imported = await importer.import_operit_archive({json_path: '/sdcard/Download/test.json'});
  assert.equal(imported.success, false);
  assert.equal(imported.unsupported, true);
  assert.match(imported.message, /com.ai.assistance.operit.data.repository.ChatHistoryManager/);
});

for (const provider of ['qwen', 'minimax', 'moss', 'elevenlabs']) {
  test(`market #10: ${provider} response reaches a voice tag through the current adapters`, {skip: !directory}, async () => {
    const h = engine('10', async (name, params) => {
      if (name === 'http_request') return {statusCode: 200, content: JSON.stringify(provider === 'qwen'
        ? {output: {audio: {url: 'https://audio.test/file.wav'}}}
        : provider === 'minimax' ? {base_resp: {status_code: 0}, data: {audio: 'https://audio.test/file.mp3'}}
        : {url: 'https://audio.test/file.mp3'}), contentBase64: 'YQ=='.repeat(50)};
      if (name === 'file_exists') return {exists: false};
      if (name === 'read_file_full') return {path: params.path ?? '/operitapp/data/mock', content: '{}'};
      if (name === 'make_directory' || name === 'download_file' || name === 'copy_file' || name === 'write_file_binary') return {successful: true, path: params.path ?? params.source ?? params.destination ?? '/operitapp/data/mock'};
      if (name === 'write_file') return {operation: {successful: true, path: params.path ?? params.source ?? params.destination ?? '/operitapp/data/mock'}, aiDiffInstructions: ''};
      const output = params.command?.includes('shlex.split') ? JSON.stringify({files: [], args: [], directory: '/tmp/audit-ffmpeg'})
        : params.command?.includes("subprocess.run([") ? JSON.stringify({output: 'Duration: 00:00:02.50', exitCode: 0})
        : params.command?.includes('request=json.loads') ? JSON.stringify({path: '/tmp/audit-ffmpeg', successful: true}) : '';
      const receipt = terminalReceipt(name, params, output);
      assert.ok(receipt, name);
      return receipt;
    });
    h.env.set('VOICE_BAR_PROVIDER', provider);
    h.env.set('VOICE_BAR_' + provider.toUpperCase() + '_API_KEY', 'audit-dummy-key');
    h.env.set('VOICE_BAR_' + provider.toUpperCase() + '_VOICE_ID', 'audit-voice');
    await h.load('dist/packages/voice_bar.js').say({text: 'audit text'});
    assert.equal(h.context.completed.success, true, JSON.stringify(h.context.completed));
    assert.match(h.context.completed.voice_tag, /^<voice /);
    assert.ok(h.calls.some(c => c.name === 'http_request'));
  });
}

test('market #10: MiniMax hex-audio branch exposes the unavailable Node Buffer dependency', {skip: !directory}, async () => {
  const h = engine('10', async (name, params) => {
    if (name === 'http_request') return {statusCode: 200, content: JSON.stringify({base_resp: {status_code: 0}, data: {audio: '01020304'}})};
    if (name === 'file_exists') return {exists: false};
    if (name === 'read_file_full') return {path: params.path ?? '/operitapp/data/mock', content: '{}'};
    if (name === 'make_directory') return {successful: true};
    if (name === 'write_file') return {operation: {successful: true, path: params.path}};
    throw Error('Unexpected native call: ' + name);
  });
  h.env.set('VOICE_BAR_PROVIDER', 'minimax');
  h.env.set('VOICE_BAR_MINIMAX_API_KEY', 'audit-dummy-key');
  h.env.set('VOICE_BAR_MINIMAX_VOICE_ID', 'audit-voice');
  assert.equal(h.run('typeof Buffer'), 'undefined');
  await h.load('dist/packages/voice_bar.js').say({text: 'audit text'});
  assert.equal(h.context.completed.success, false);
  assert.match(h.context.completed.detail, /Buffer is not defined/);
});

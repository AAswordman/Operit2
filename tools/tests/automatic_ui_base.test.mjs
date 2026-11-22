import assert from 'node:assert/strict';
import { readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import test from 'node:test';
import vm from 'node:vm';
import { readSdkScript, wrapComposeScript } from '../runtime/compose_scripts.mjs';
import { composeStreamFixture } from './support/compose_stream_fixture.mjs';

const require = createRequire(new URL('../../plugins/packages/buildin/workflow/package.json', import.meta.url));
const ts = require('typescript');
const root = new URL('../../plugins/packages/buildin/automatic_ui_base/', import.meta.url);
const manifest = JSON.parse(readFileSync(new URL('manifest.json', root), 'utf8'));
const source = readFileSync(new URL('src/tools.ts', root), 'utf8');
const metadata = JSON.parse(source.split('/* METADATA')[1].split('*/')[0]);
const compiled = ts.transpileModule(source, {
  compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.CommonJS },
}).outputText;

/** Loads the migrated package with recorded system calls and real completion behavior. */
function runtime(tools = {}, call = async () => { throw new Error('Unexpected tool call'); }, { chatId, ipc = async () => {} } = {}) {
  const results = [];
  const context = vm.createContext({ exports: {}, Tools: tools, toolCall: call, getChatId: () => chatId, ToolPkg: { ipc: { call: ipc } }, complete: result => results.push(result) });
  vm.runInContext(compiled, context);
  return { context, tools: context.exports, results };
}

/** Runs the production IPC dispatcher with an explicit native-owned call context. */
function installIpc(context, kind = 'main', invoke) {
  context.__operitCurrentCallId = 'automation-test';
  context.__operitGetCallState = () => ({ toolPkgContext: {
    packageTarget: manifest.toolpkg_id,
    contextKey: `toolpkg_main:${manifest.toolpkg_id}`,
    runtime: kind,
  } });
  context.__operitNativeInvokeToolPkgIpc = invoke;
  context.__operitInvokeHostAsync = async (binding, args, decode) => decode(await binding(...args));
  vm.runInContext(readFileSync(new URL('../../core/crates/plugin/javascript-bridge/src/javascript/JsToolPkgIpc.script.js', import.meta.url), 'utf8'), context);
  context.ToolPkg.ipc = context.__operitToolPkgIpc;
}

/** Converts values crossing the VM boundary into ordinary test objects. */
function plain(value) {
  return JSON.parse(JSON.stringify(value));
}

test('the ToolPkg preserves the existing namespace, default disabled state and tool display metadata', () => {
  assert.equal(manifest.toolpkg_id, 'com.operit.ui_automation');
  assert.equal(manifest.api_version, '2.0.0');
  assert.equal(manifest.enabled_by_default, false);
  assert.deepEqual(manifest.subpackages, [{ id: 'Automatic_ui_base', entry: 'dist/tools.js' }]);
  assert.throws(() => readFileSync(new URL('../automatic_ui_base.ts', root)), { code: 'ENOENT' });
  assert.equal(metadata.name, 'Automatic_ui_base');
  assert.equal(metadata.enabledByDefault, true);
  assert.equal(metadata.tools.length, 14);
  const { tools } = runtime();
  for (const tool of metadata.tools) {
    assert.ok(tool.display.label.zh.trim(), tool.name);
    assert.ok(tool.display.label.en.trim(), tool.name);
    assert.ok(tool.display.icon.trim(), tool.name);
    if (!tool.advice) assert.equal(typeof tools[tool.name], 'function', tool.name);
  }
  assert.equal(metadata.tools.find(tool => tool.name === 'get_page_info').parameters[0].name, 'window_id');
});

test('window discovery and focus preserve identities while model results contain only the compact summary', async () => {
  const calls = [];
  const page = {windowId: 'native:42', scope: 'window', truncated: false, summary: 'Button "Save" id="save" [0,0][10,10]', uiElements: {children: Array.from({length: 4501}, (_, index) => ({text: `node-${index}`}))}};
  const {tools, results} = runtime({UI: {
    async listWindows() { calls.push(['list']); return {windows: [{windowId: 'native:42'}]}; },
    async focusWindow(id) { calls.push(['focus', id]); return {}; },
    async getPageInfo(id) { calls.push(['tree', id]); return page; },
  }});
  await tools.list_windows({});
  await tools.focus_window({window_id: 'native:42'});
  await tools.get_page_info({window_id: 'native:42'});
  assert.deepEqual(calls, [['list'], ['focus', 'native:42'], ['tree', 'native:42']]);
  assert.equal(page.uiElements.children.at(-1).text, 'node-4500');
  assert.equal(results[2].data.uiElements, undefined);
  assert.equal(results[2].data.summary, page.summary);
  assert.equal(results[2].data.windowId, 'native:42');
  assert.match(results[2].data.coordinateNote, /window-local coordinates/);
  assert.match(results[2].data.coordinateNote, /scroll clipped or offscreen/i);
});

test('swipe duration, input clearing and element criteria reach the v2 UI provider', async () => {
  const calls = [];
  const { tools, results } = runtime({ UI: {
    async swipe(...args) { calls.push(['swipe', ...args]); return {}; },
    async setText(...args) { calls.push(['setText', ...args]); return {}; },
    async clickElement(params) { calls.push(['clickElement', params]); return {}; },
  } });
  await tools.swipe({ start_x: -20, start_y: 10, end_x: 100, end_y: 200, duration: 765 });
  await tools.set_input_text({ text: '', resource_id: 'editor' });
  await tools.click_element({ text: 'Save', contentDesc: 'Save file', partialMatch: true, index: 1 });
  assert.deepEqual(plain(calls), [
    ['swipe', -20, 10, 100, 200, 765], ['setText', '', 'editor'],
    ['clickElement', { text: 'Save', contentDesc: 'Save file', partialMatch: true, index: 1 }],
  ]);
  assert.ok(results.every(result => result.success));
});

test('screenshots use the system capture tool and register its VFS image', async () => {
  const calls = [];
  const path = '/operitapp/data/temp/clean_on_exit/screenshots/test.png';
  const { tools, results } = runtime({ Media: {
    async registerFromPath(params) { calls.push(plain(params)); return { link: 'image:test' }; },
  } }, async (name, params) => {
    calls.push([name, plain(params)]);
    return path;
  });
  await tools.get_page_screenshot_image({});
  assert.deepEqual(calls, [['capture_screenshot', {}], { type: 'image', path }]);
  assert.deepEqual(plain(results[0].data), { file_path: path, image_link: 'image:test' });
});

test('clearing selection returns the packaged tools and indicator to the main surface', async () => {
  const plugin = indicatorRuntime({ nativeWindows: [nativeWindow('selected')] });
  const focused = [];
  const instance = runtime({ UI: {
    async focusWindow(id) { focused.push(id); return {}; },
    async getPageInfo() { return {windowId: 'active', scope: 'desktop', summary: 'Main surface'}; },
  } }, undefined, { chatId: 'chat', async ipc(channel, payload) {
    if (channel === 'ui_automation.begin') return plugin.begin(payload.chatId, payload.windowId);
    return plugin.complete(payload.chatId, payload.closeWindows);
  } });
  await instance.tools.focus_window({window_id: 'selected'});
  await instance.tools.focus_window({});
  await instance.tools.get_page_info({});
  assert.deepEqual(focused, ['selected', undefined]);
  assert.equal(plugin.windows.get('automation-activity').fill, true);
  assert.ok(![...plugin.windows.keys()].some(id => id.startsWith('automation-activity-window-')));
  assert.match(instance.results.at(-1).data.coordinateNote, /No window is selected/);
  assert.equal(plugin.timers.size, 0);
  await instance.tools.complete_ui_operation({});
});

test('app launch reports failure without listing or launching other apps', async () => {
  const calls = [];
  const operation = { success: false, details: 'Application is unavailable' };
  const { tools, results } = runtime({ System: {
    async startApp(name) { calls.push(name); return operation; },
    async listApps() { assert.fail('Unexpected installed-app discovery'); },
  } });
  await tools.app_launch({ package_name: 'missing.app' });
  assert.deepEqual(calls, ['missing.app']);
  assert.deepEqual(plain(results[0]), { success: false, message: operation.details, data: { operation } });
});

test('provider failures remain errors and package main performs no input', async () => {
  const { tools, results } = runtime({ UI: {
    async getPageInfo() { throw new Error('Whole-device automation is unsupported'); },
  } });
  await tools.get_page_info({});
  assert.equal(results[0].success, false);
  assert.match(results[0].message, /Whole-device automation is unsupported/);
  await tools.main();
  assert.equal(results[1].success, true);
});

/** Loads the actual plugin modules with a recorded floating-window host. */
function indicatorRuntime({ scope = 'system', permissionGranted = true, closeFailures = 0, screenCloseFailures = 0, register = true, nativeWindows = [], onQuery } = {}) {
  const routes = new Map();
  const hooks = new Map();
  const windows = new Map([['other-surface', { id: 'other-surface' }]]);
  const commands = [];
  const screenCalls = [];
  const timers = new Map();
  let timerSerial = 0;
  const context = vm.createContext({ console,
    setTimeout(callback) { const id = ++timerSerial; timers.set(id, callback); return id; },
    clearTimeout(id) { timers.delete(id); },
    Tools: { UI: {
      async listWindows() { await onQuery?.(); return { windows: nativeWindows }; },
      async focusWindow(id, closeWindows) {
        screenCalls.push([id, closeWindows]);
        if (screenCloseFailures > 0) { screenCloseFailures--; throw new Error('Native screen close failed'); }
      },
    } },
    ToolPkg: {
    registerUiRoute(definition) { routes.set(definition.id, definition); },
    registerNavigationEntry() { assert.fail('Automation must not create a standalone page'); },
    registerAppLifecycleHook(definition) { hooks.set(definition.event, definition.function); },
    registerChatRuntimeHook(definition) { hooks.set('state_changed', definition.function); },
    registerToolLifecycleHook(definition) { hooks.set('tool_lifecycle', definition.function); },
    ui: { floatingWindows: {
      async list() { return [...windows.values()]; },
      async capabilities() { return { scope, permissionGranted, supported: scope !== 'unsupported', clickThrough: true }; },
      async requestPermission() { assert.fail('Lifecycle cannot prompt for permission'); },
      async open(request) {
        assert.ok(!windows.has(request.id));
        windows.set(request.id, request);
        commands.push(['open', plain(request)]);
      },
      async close(id) {
        if (closeFailures > 0) { closeFailures--; throw new Error('Host close failed'); }
        assert.ok(windows.delete(id));
        commands.push(['close', id]);
      },
      async update(id, options) {
        assert.ok(windows.has(id));
        windows.get(id).options = { ...windows.get(id).options, ...plain(options) };
        commands.push(['update', id, plain(options)]);
      },
      async hide(id) { assert.ok(windows.has(id)); commands.push(['hide', id]); },
      async show(id) { assert.ok(windows.has(id)); commands.push(['show', id]); },
    } },
  } });
  installIpc(context);
  const modules = new Map();

  /** Resolves the same relative imports used by the packaged CommonJS modules. */
  function load(moduleName) {
    if (modules.has(moduleName)) return modules.get(moduleName);
    const source = readFileSync(new URL(`src/${moduleName}.ts`, root), 'utf8');
    const script = ts.transpileModule(source, {
      compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.CommonJS },
    }).outputText;
    const output = {};
    modules.set(moduleName, output);
    vm.runInContext(`(function(exports, require) { ${script}\n})`, context)(output, modulePath => load(modulePath.slice(2)));
    return output;
  }

  const main = load('main');
  if (register) assert.equal(main.registerToolPkg(), true);
  assert.deepEqual(commands, []);
  return { context, commands, screenCalls, routes, windows, timers,
    async poll() {
      assert.equal(timers.size, 1);
      const [id, callback] = timers.entries().next().value;
      timers.delete(id); callback();
      await new Promise(resolve => setImmediate(resolve));
    },
    begin(chatId, windowId) { return context.ToolPkg.ipc.call('ui_automation.begin', { chatId, ...(windowId !== undefined ? { windowId } : {}) }); },
    complete(chatId, closeWindows) { return context.ToolPkg.ipc.call('ui_automation.complete', { chatId, closeWindows }); },
    /** Delivers a normalized application lifecycle event. */
    app(state) {
      const eventName = `application_on_${state}`;
      return hooks.get(eventName)({ eventName });
    },
    /** Delivers the concrete tool identity and its authenticated turn context. */
    tool(toolName, chatId, eventName = 'tool_execution_started', parameters) {
      return hooks.get('tool_lifecycle')({ eventName, eventPayload: { toolName, parameters, runtimeContext: chatId ? { chatId } : null } });
    },
    /** Delivers the terminal state for one conversation. */
    finish(chatId) {
      return hooks.get('state_changed')({ eventPayload: { chatId, isActive: false } });
    },
  };
}

test('a cold main module registers IPC without replaying the registration entry point', async () => {
  // Production activation captures registerToolPkg in a temporary engine; the
  // runtime IPC entry loads main.js but never calls registerToolPkg again.
  const plugin = indicatorRuntime({ register: false });
  assert.equal(plugin.routes.size, 0);
  await plugin.complete('chat');
  await plugin.begin('chat');
  await plugin.complete('chat');
  assert.deepEqual(plugin.commands.map(([command]) => command), ['open', 'close']);
});

test('sandbox tools reach cold main IPC before provider calls and complete closes the overlay', async () => {
  const plugin = indicatorRuntime({ register: false });
  const calls = [];
  const instance = runtime({ UI: {
    async listWindows() { calls.push('list'); return { windows: [] }; },
    async getPageInfo() { calls.push('tree'); return { summary: 'test' }; },
  }, Media: { async registerFromPath() { return { link: 'image:test' }; } } },
  async () => { calls.push('capture'); return '/test.png'; }, { chatId: 'chat' });
  installIpc(instance.context, 'sandbox', async (owner, caller, target, kind, channel, payload) => {
    assert.equal(owner, manifest.toolpkg_id);
    assert.equal(kind, 'main');
    try {
      const value = await plugin.context.__operitInvokeToolPkgIpcLocal(channel, payload, caller);
      return { success: true, value };
    } catch (error) { return { success: false, message: error.message }; }
  });
  await instance.tools.list_windows({});
  await instance.tools.get_page_info({});
  await instance.tools.get_page_screenshot_image({});
  await instance.tools.complete_ui_operation({});
  assert.deepEqual(calls, ['list', 'tree', 'capture']);
  assert.ok(instance.results.every(result => result.success), JSON.stringify(instance.results));
  assert.deepEqual(plugin.commands.map(([command]) => command), ['open', 'close']);
});

test('automation appears immediately without an independent page and stays across foreground changes', async () => {
  const plugin = indicatorRuntime();
  assert.deepEqual([...plugin.routes.keys()], ['indicator']);
  await plugin.tool('elsewhere:tap', 'unrelated');
  await plugin.tool('tap', 'blocked', 'tool_call_requested');
  await plugin.tool('tap', undefined);
  assert.deepEqual(plugin.commands, []);
  await plugin.begin('first');
  assert.deepEqual(plugin.commands, [['open', {
    id: 'automation-activity', route: plugin.routes.get('indicator').route, fill: true,
    options: { clickThrough: true, focusable: false },
  }]]);
  await plugin.app('foreground');
  await plugin.app('background');
  await plugin.app('foreground');
  await plugin.tool('get_page_info', 'second');
  await plugin.complete('first');
  assert.equal(plugin.commands.length, 1);
  await plugin.complete('second');
  assert.deepEqual(plugin.commands.at(-1), ['close', 'automation-activity']);
  await plugin.complete('second');
  assert.equal(plugin.commands.length, 2);
  assert.deepEqual([...plugin.windows.keys()], ['other-surface']);
  await plugin.begin('first');
  await plugin.finish('first');
  assert.deepEqual(plugin.commands.map(([command]) => command), ['open', 'close', 'open', 'close']);
});

test('application scope follows visibility and termination cleans up even without explicit completion', async () => {
  const local = indicatorRuntime({ scope: 'application' });
  await local.app('background');
  await local.begin('chat');
  assert.deepEqual(local.commands, []);
  await local.app('foreground');
  await local.app('background');
  await local.app('foreground');
  await local.app('terminate');
  assert.deepEqual(local.commands.map(([command]) => command), ['open', 'close', 'open', 'close']);
  await local.app('foreground');
  assert.equal(local.commands.length, 4);

  for (const options of [{ permissionGranted: false }, { scope: 'unsupported' }]) {
    const denied = indicatorRuntime(options);
    await denied.begin('chat');
    await denied.complete('chat');
    assert.deepEqual(denied.commands, []);
  }
});

test('concurrent starts and completions serialize host operations and permit later automation', async () => {
  const plugin = indicatorRuntime();
  await Promise.all([plugin.begin('first'), plugin.begin('second'), plugin.begin('first')]);
  assert.equal(plugin.commands.length, 1);
  await Promise.all([plugin.complete('first'), plugin.complete('second')]);
  assert.equal(plugin.commands.length, 2);
  await plugin.begin('first');
  await plugin.app('terminate');
  assert.deepEqual(plugin.commands.map(([command]) => command), ['open', 'close', 'open', 'close']);
});

test('packaged tools await the overlay before input and completion reaches its owning main runtime', async () => {
  const plugin = indicatorRuntime();
  const calls = [];
  const instance = runtime({ UI: {
    async tap() { assert.ok(plugin.windows.has('automation-activity')); calls.push('tap'); return {}; },
  } }, undefined, { chatId: 'chat', async ipc(channel, payload, options) {
    calls.push([channel, plain(payload), plain(options)]);
    if (channel === 'ui_automation.begin') await plugin.begin(payload.chatId);
    else if (channel === 'ui_automation.complete') await plugin.complete(payload.chatId, payload.closeWindows);
    else assert.fail(channel);
  } });
  await instance.tools.tap({ x: 1, y: 2 });
  await instance.tools.complete_ui_operation({});
  assert.deepEqual(calls, [
    ['ui_automation.begin', { chatId: 'chat' }, { targetRuntime: 'main' }],
    'tap',
    ['ui_automation.complete', { chatId: 'chat', closeWindows: true }, { targetRuntime: 'main' }],
  ]);
  assert.ok(instance.results.every(result => result.success));
  assert.ok(!plugin.windows.has('automation-activity'));
  await instance.tools.complete_ui_operation({});
  assert.equal(plugin.commands.length, 2);
  await instance.tools.tap({ x: 3, y: 4 });
  assert.equal(plugin.commands.length, 3);
});

test('completion reports missing context and host cleanup failures without starting an overlay', async () => {
  const noChat = runtime();
  await noChat.tools.complete_ui_operation({});
  assert.equal(noChat.results[0].success, false);
  const failed = runtime({}, undefined, { chatId: 'chat', async ipc(channel) {
    assert.equal(channel, 'ui_automation.complete');
    throw new Error('Host close failed');
  } });
  await failed.tools.complete_ui_operation({});
  assert.equal(failed.results[0].success, false);
  assert.match(failed.results[0].message, /Host close failed/);
});

test('failed host cleanup can be retried without losing ownership or poisoning the queue', async () => {
  const plugin = indicatorRuntime({ closeFailures: 1 });
  await plugin.begin('chat');
  await assert.rejects(plugin.complete('chat'), /Host close failed/);
  assert.ok(plugin.windows.has('automation-activity'));
  await plugin.complete('chat');
  assert.ok(!plugin.windows.has('automation-activity'));
  await plugin.begin('chat');
  await plugin.finish('chat');
  assert.deepEqual(plugin.commands.map(([command]) => command), ['open', 'close', 'open', 'close']);
});

test('completion propagates screen closure and defers disposal until the last active chat', async () => {
  const plugin = indicatorRuntime({ nativeWindows: [nativeWindow('phone'), nativeWindow('pc')] });
  await plugin.begin('first', 'phone');
  await plugin.begin('second', 'pc');
  await plugin.complete('first', true);
  assert.deepEqual(plugin.screenCalls, []);
  assert.ok([...plugin.windows.keys()].some(id => id.startsWith('automation-activity-window-')));
  await plugin.complete('second', true);
  assert.deepEqual(plugin.screenCalls, [[undefined, true]]);
  assert.deepEqual([...plugin.windows.keys()], ['other-surface']);
  await plugin.finish('unrelated');
  assert.equal(plugin.screenCalls.length, 1);
  await plugin.complete('second', true);
  assert.equal(plugin.screenCalls.length, 2, 'Repeated completion must still await idempotent host cleanup');
  await plugin.tool('focus_window', 'cleanup', 'tool_execution_started', {close_windows: 'true'});
  assert.deepEqual([...plugin.windows.keys()], ['other-surface'], 'Host cleanup must not start another automation operation');
});

test('native screen cleanup errors stay visible and a repeated completion retries before dismissing the overlay', async () => {
  const plugin = indicatorRuntime({ screenCloseFailures: 1 });
  await plugin.begin('chat');
  await assert.rejects(plugin.complete('chat', true), /Native screen close failed/);
  assert.ok(plugin.windows.has('automation-activity'));
  await plugin.complete('chat', true);
  assert.deepEqual(plugin.screenCalls, [[undefined, true], [undefined, true]]);
  assert.ok(!plugin.windows.has('automation-activity'));
  await plugin.begin('chat');
  await plugin.finish('chat');
  assert.equal(plugin.screenCalls.length, 3, 'Turn completion must release screens too');
  await plugin.begin('next');
  await plugin.app('terminate');
  assert.equal(plugin.screenCalls.length, 4, 'Application termination must await the same host cleanup');
});

test('the indicator route renders host-animated glow with a separate activity orb', async () => {
  const { context, routes } = indicatorRuntime();
  context.module = { exports: { default: routes.get('indicator').screen } };
  vm.runInContext(readSdkScript('ToolPkgComposeDslBridge.rs'), context);
  vm.runInContext(wrapComposeScript(), context);
  const fixture = composeStreamFixture(context);
  fixture.adapt();
  await context.__operit_render_compose_dsl({ executionContextKey: 'automation-preview' });
  const tree = fixture.latest.tree;
  assert.equal(tree.type, 'Box');
  assert.equal(tree.props.fillMaxSize, true);
  assert.equal(tree.children[0].props.fillMaxSize, true);
  assert.equal(tree.children[0].type, 'GlowBorder');
  assert.ok(tree.children[0].props.glowWidth > tree.children[0].props.strokeWidth * 5);
  assert.ok(tree.children[0].props.durationMillis > 0);
  assert.equal(tree.children[1].children[0].type, 'CircularProgressIndicator');
  if (process.env.OPERIT_AUTOMATION_PREVIEW_TREE) {
    writeFileSync(process.env.OPERIT_AUTOMATION_PREVIEW_TREE, JSON.stringify(tree));
  }
});

/** Makes a real provider-shaped native window with mutable state for tracking tests. */
function nativeWindow(windowId, bounds = '[120,80][920,680]') {
  return { windowId, bounds, title: windowId, packageName: 'editor', isActive: true, isFocused: true, isMinimized: false };
}

test('selected windows use native screen bounds, follow movement and hide when minimized or closed', async () => {
  const target = nativeWindow('editor:1');
  const nativeWindows = [target];
  const plugin = indicatorRuntime({ nativeWindows });
  await plugin.begin('chat');
  await plugin.begin('chat', target.windowId);
  const opened = plugin.commands.filter(([command]) => command === 'open').at(-1)[1];
  assert.equal(opened.fill, undefined);
  assert.deepEqual(opened.options.screenBounds, { left: 120, top: 80, right: 920, bottom: 680 });
  assert.ok(!plugin.windows.has('automation-activity'));
  assert.equal(plugin.timers.size, 1);
  target.bounds = '[-150,24][850,724]';
  await plugin.poll();
  assert.deepEqual(plugin.commands.at(-1), ['update', opened.id, { screenBounds: { left: -150, top: 24, right: 850, bottom: 724 } }]);
  const count = plugin.commands.length;
  await plugin.begin('chat');
  assert.equal(plugin.commands.length, count, 'ordinary input retains the target without reopening it');
  target.isMinimized = true;
  await plugin.poll();
  assert.deepEqual(plugin.commands.at(-1), ['hide', opened.id]);
  target.isMinimized = false;
  target.bounds = '[0,0][7680,4320]';
  await plugin.poll();
  assert.deepEqual(plugin.commands.at(-2), ['update', opened.id, { screenBounds: { left: 0, top: 0, right: 7680, bottom: 4320 } }]);
  assert.deepEqual(plugin.commands.at(-1), ['show', opened.id]);
  nativeWindows.length = 0;
  await plugin.poll();
  assert.deepEqual(plugin.commands.at(-1), ['hide', opened.id]);
  assert.ok(!plugin.windows.has('automation-activity'), 'a missing selected window never marks the whole display');
  await plugin.complete('chat');
  assert.deepEqual(plugin.commands.at(-1), ['close', opened.id]);
  assert.equal(plugin.timers.size, 0);
});

test('different chats share equal targets and retain independent indicators for different windows', async () => {
  const plugin = indicatorRuntime({ nativeWindows: [nativeWindow('a'), nativeWindow('b')] });
  await plugin.begin('first', 'a');
  await plugin.begin('second', 'a');
  assert.equal(plugin.commands.length, 1);
  await plugin.begin('second', 'b');
  assert.equal(plugin.commands.filter(([command]) => command === 'open').length, 2);
  const a = plugin.commands[0][1].id;
  const b = plugin.commands[1][1].id;
  await plugin.complete('first');
  assert.ok(!plugin.windows.has(a));
  assert.ok(plugin.windows.has(b));
  await plugin.app('terminate');
  assert.ok(!plugin.windows.has(b));
  assert.equal(plugin.timers.size, 0);
});

test('stale IDs, empty bounds and provider errors never fall back to another window', async () => {
  let fail = false;
  const windows = [nativeWindow('other'), nativeWindow('empty', '[0,0][0,0]')];
  const plugin = indicatorRuntime({ nativeWindows: windows, onQuery() { if (fail) throw new Error('Provider unavailable'); } });
  await plugin.begin('chat', 'missing');
  await plugin.begin('chat', 'empty');
  assert.deepEqual(plugin.commands, []);
  await plugin.begin('chat', 'other');
  const id = plugin.commands[0][1].id;
  fail = true;
  await plugin.poll();
  assert.deepEqual(plugin.commands.at(-1), ['hide', id]);
  await plugin.complete('chat');
  assert.equal(plugin.timers.size, 0);
});

test('native lifecycle parameters select the target and the provider refresh does not recurse', async () => {
  let plugin;
  plugin = indicatorRuntime({ nativeWindows: [nativeWindow('native:42')],
    onQuery: () => plugin.tool('list_windows', 'chat'),
  });
  await plugin.tool('focus_window', 'chat', 'tool_execution_started', { window_id: 'native:42' });
  assert.equal(plugin.commands[0][1].options.screenBounds.left, 120);
  await plugin.finish('chat');
  assert.equal(plugin.timers.size, 0);
});

test('packaged focus and snapshots propagate the selected and actual active windows through IPC', async () => {
  const plugin = indicatorRuntime({ nativeWindows: [nativeWindow('a'), nativeWindow('b')] });
  const instance = runtime({ UI: {
    async focusWindow(id) {
      assert.equal(id, 'a');
      assert.ok([...plugin.windows.values()].some(window => window.options?.screenBounds));
      return {};
    },
    async getPageInfo() { return { windowId: 'b', scope: 'window', summary: 'Selected window B' }; },
  } }, undefined, { chatId: 'chat', async ipc(channel, payload) {
    if (channel === 'ui_automation.begin') return plugin.begin(payload.chatId, payload.windowId);
    return plugin.complete(payload.chatId, payload.closeWindows);
  } });
  await instance.tools.focus_window({ window_id: 'a' });
  const first = plugin.commands[0][1].id;
  await instance.tools.get_page_info({});
  assert.ok(!plugin.windows.has(first));
  assert.ok(instance.results.every(result => result.success));
  await instance.tools.complete_ui_operation({});
  assert.equal(plugin.timers.size, 0);
});

test('application-only hosts keep their own full view and never request external window bounds', async () => {
  const plugin = indicatorRuntime({ scope: 'application', onQuery() { assert.fail('External provider must not be queried'); } });
  await plugin.begin('chat', 'external-window');
  assert.equal(plugin.commands[0][1].fill, true);
  assert.equal(plugin.timers.size, 0);
  await plugin.complete('chat');
});

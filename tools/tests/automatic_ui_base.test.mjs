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
  const page = {windowId: 'native:42', scope: 'desktop', truncated: false, summary: 'Button "Save" id="save" [0,0][10,10]', uiElements: {children: Array.from({length: 4501}, (_, index) => ({text: `node-${index}`}))}};
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
function indicatorRuntime({ scope = 'system', permissionGranted = true, closeFailures = 0, register = true } = {}) {
  const routes = new Map();
  const hooks = new Map();
  const windows = new Map([['other-surface', { id: 'other-surface' }]]);
  const commands = [];
  const context = vm.createContext({ console, ToolPkg: {
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
  return { context, commands, routes, windows,
    begin(chatId) { return context.ToolPkg.ipc.call('ui_automation.begin', { chatId }); },
    complete(chatId) { return context.ToolPkg.ipc.call('ui_automation.complete', { chatId }); },
    /** Delivers a normalized application lifecycle event. */
    app(state) {
      const eventName = `application_on_${state}`;
      return hooks.get(eventName)({ eventName });
    },
    /** Delivers the concrete tool identity and its authenticated turn context. */
    tool(toolName, chatId, eventName = 'tool_execution_started') {
      return hooks.get('tool_lifecycle')({ eventName, eventPayload: { toolName, runtimeContext: chatId ? { chatId } : null } });
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
    else if (channel === 'ui_automation.complete') await plugin.complete(payload.chatId);
    else assert.fail(channel);
  } });
  await instance.tools.tap({ x: 1, y: 2 });
  await instance.tools.complete_ui_operation({});
  assert.deepEqual(calls, [
    ['ui_automation.begin', { chatId: 'chat' }, { targetRuntime: 'main' }],
    'tap',
    ['ui_automation.complete', { chatId: 'chat' }, { targetRuntime: 'main' }],
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

test('the indicator route renders a full-size outline with a separate activity orb', async () => {
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
  assert.equal(tree.children[1].children[0].type, 'CircularProgressIndicator');
  if (process.env.OPERIT_AUTOMATION_PREVIEW_TREE) {
    writeFileSync(process.env.OPERIT_AUTOMATION_PREVIEW_TREE, JSON.stringify(tree));
  }
});

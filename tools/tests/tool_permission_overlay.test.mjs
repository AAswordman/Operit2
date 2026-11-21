import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import vm from 'node:vm';
import test from 'node:test';
import { readSdkScript, wrapComposeScript } from '../runtime/compose_scripts.mjs';
import { composeStreamFixture } from './support/compose_stream_fixture.mjs';

const plugin = new URL('../../plugins/packages/external/permission_overlay/', import.meta.url);
const require = createRequire(new URL('../../plugins/packages/buildin/character_cards/package.json', import.meta.url));
const ts = require('typescript');

/** Copies cross-realm structured values for exact assertions. */
function plain(value) { return JSON.parse(JSON.stringify(value)); }

/** Supplies controllable host promises without introducing timing-dependent sleeps. */
function deferred() {
  let resolve, reject;
  const promise = new Promise((done, fail) => { resolve = done; reject = fail; });
  return { promise, resolve, reject };
}

/** Creates requests in distinct chats so accidental scope reuse is observable. */
function request(id = 'first', chatId = 'chat-a') {
  return { requestId: id, chatId, tool: { name: 'write_file', parameters: [
    { name: 'path', value: '/virtual/windows/workspace/example.txt' },
    { name: 'content', value: 'hello\nworld' },
  ] }, description: 'Write a workspace file', requestedAtMillis: id === 'first' ? 1 : 2 };
}

/** Loads current TypeScript in memory and preserves module-level state across callbacks. */
function modules(context) {
  const loaded = new Map();
  /** Evaluates only this plugin's relative source imports. */
  function load(name) {
    if (loaded.has(name)) return loaded.get(name).exports;
    const module = { exports: {} };
    loaded.set(name, module);
    const source = readFileSync(new URL(`src/${name}.ts`, plugin), 'utf8');
    const compiled = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 } }).outputText;
    const evaluate = vm.runInContext(`(function(exports,require,module){${compiled}\n})`, context);
    evaluate(module.exports, path => {
      assert.ok(path.startsWith('./'));
      return load(path.slice(2));
    }, module);
    return module.exports;
  }
  return load;
}

/** Captures creator-facing commands; native lifecycle behavior is tested at the Flutter host. */
function host() {
  const calls = [], errors = [], hooks = [], routes = [];
  const windows = {
    async set(id, content) { calls.push(['set', id, plain(content)]); return content === null ? 'closed' : 'shown'; },
  };
  const toolPermissions = {
    async respond(chatId, requestId, result) { calls.push(['respond', chatId, requestId, result]); },
  };
  const context = vm.createContext({
    console: { ...console, error: (...values) => errors.push(values.map(String).join(' ')) },
    ToolPkg: {
      toolPermissions,
      ui: { floatingWindows: windows },
      registerUiRoute(definition) { routes.push(definition); },
      registerNavigationEntry() {},
      registerToolPermissionHook(definition) { hooks.push(definition); },
    },
  });
  return { context, calls, errors, hooks, routes, windows, toolPermissions, load: modules(context) };
}

/** Exercises real registration capture and the native approval command ABI. */
test('permission API captures durable hooks and prevents registration-time host access', async () => {
  const calls = [];
  const context = vm.createContext({
    __operitExpose(name, value) { context[name] = value; },
    __operitCurrentCallId: 'call',
    __operitGetCallState() { return { params: { __operit_toolpkg_api_version: '2.0.0' } }; },
    __operitNativeToolPermissionCommand(payload) { calls.push(plain(payload)); return null; },
    __operitInvokeHostAsync(binding, args) { return Promise.resolve(binding(...args)); },
  });
  vm.runInContext(readSdkScript('ToolPkgApiRuntimeScript.rs'), context);
  const bridge = readSdkScript('ToolPkgRegistrationBridge.rs');
  vm.runInContext(bridge.replace('__OPERIT_TOOLPKG_REGISTRATION_ONLY__', 'true'), context);
  vm.runInContext(`
    function onApproval(event) {}
    __operitGetActiveModuleExports = () => ({onApproval});
    ToolPkg.registerToolPermissionHook({id:'overlay', function:onApproval});
  `, context);
  assert.deepEqual(JSON.parse(context.__operitToolPkgRegistrationCapture.toolPermissionHooks[0]), { id: 'overlay', function: 'onApproval' });
  assert.equal(context.ToolPkg.ui.toolPermissions, undefined);
  assert.throws(() => vm.runInContext('ToolPkg.toolPermissions.list()', context), /during registration/);
  assert.throws(() => vm.runInContext('ToolPkg.toolPermissions.respond("chat", "id", "allow")', context), /during registration/);
  assert.equal(calls.length, 0);
  vm.runInContext(bridge.replace('__OPERIT_TOOLPKG_REGISTRATION_ONLY__', 'false'), context);
  await vm.runInContext('ToolPkg.toolPermissions.list()', context);
  await vm.runInContext('ToolPkg.toolPermissions.respond("chat-b", "request-b", "allow_session")', context);
  assert.deepEqual(calls, [{ operation: 'list' }, { operation: 'respond', chatId: 'chat-b', requestId: 'request-b', result: 'allow_session' }]);
});

/** Ensures a creator can present all snapshot transitions using only the managed window API. */
test('approval hook declares one window directly from snapshots without manual lifecycle APIs', async () => {
  const fixture = host();
  const main = fixture.load('main');
  main.registerToolPkg();
  assert.equal(fixture.hooks.length, 1);
  const requests = [request(), request('second', 'chat-b')];
  await fixture.hooks[0].function({ eventPayload: { requests } });
  assert.deepEqual(fixture.calls[0], ['set', 'tool-permission', {
    route: 'toolpkg:com.operit.permission_overlay:ui:approval', fill: true,
    state: { request: requests[0], pendingCount: 2 },
  }]);
  await main.onPermissionsChanged({ eventPayload: { requests: [requests[1]] } });
  assert.equal(fixture.calls[1][2].state.request.chatId, 'chat-b');
  await main.onPermissionsChanged({ eventPayload: { requests: [] } });
  assert.deepEqual(fixture.calls[2], ['set', 'tool-permission', null]);
  assert.equal(fixture.errors.length, 0);
});

/** Keeps unexpected host failures visible instead of swallowing creator-facing errors. */
test('approval presentation propagates host errors', async () => {
  const fixture = host();
  fixture.windows.set = async () => { throw new Error('engine startup failed'); };
  await assert.rejects(fixture.load('controller').onPermissionsChanged({ eventPayload: { requests: [request()] } }), /engine startup failed/);
});

/** Builds the real Compose stream for the plugin's current approval screen. */
async function screen(fixture) {
  vm.runInContext(readSdkScript('ToolPkgComposeDslBridge.rs'), fixture.context);
  const ui = fixture.load('ui');
  fixture.context.module = { exports: ui.approvalScreen };
  vm.runInContext(wrapComposeScript(), fixture.context);
  const stream = composeStreamFixture(fixture.context);
  stream.adapt();
  await fixture.context.__operit_render_compose_dsl({ executionContextKey: 'approval-test', theme: { brightness: 'light', colors: {} },
    state: { request: request(), pendingCount: 2 } });
  /** Walks the host-facing tree, including composed content slots. */
  function nodes(node = stream.latest.tree) { return [node, ...node.children.flatMap(nodes), ...Object.values(node.slots).flatMap(children => children.flatMap(nodes))]; }
  const measure = nodes()[0].props.modifier.__modifierOps.find(operation => operation.name === 'onSizeChanged').args[0];
  await fixture.context.__operit_dispatch_compose_dsl_action({ actionId: measure.__actionId, payload: { width: 390, height: 844 } });
  return { nodes,
    async click(text) {
      const node = nodes().find(node => node.props.text === text && node.props.onClick);
      assert.ok(node, `Missing button: ${text}`);
      return fixture.context.__operit_dispatch_compose_dsl_action({ actionId: node.props.onClick.__actionId });
    },
    async update(value) {
      await fixture.context.__operit_render_compose_dsl({ executionContextKey: 'approval-test', __operit_update_inputs: true,
        __operit_input_state: { request: value, pendingCount: 1 } });
    },
  };
}

/** Verifies the real UI stream has complete details, fixed actions, and exact request-scoped decisions. */
test('classic card submits exact displayed requests once and resets for the next chat', async () => {
  const fixture = host(), page = await screen(fixture);
  const text = page.nodes().filter(node => node.type === 'Text').map(node => node.props.text);
  for (const label of ['权限请求', '请求的操作', '使用的工具', '参数详情', 'hello\nworld']) assert.ok(text.includes(label), label);
  const scroll = page.nodes().find(node => node.type === 'ScrollArea');
  assert.ok(scroll);
  assert.equal(page.nodes(scroll).some(node => node.props.text === '允许'), false, 'Actions stay outside scrollable details');
  await Promise.all([page.click('允许'), page.click('允许')]);
  assert.deepEqual(fixture.calls, [['respond', 'chat-a', 'first', 'allow']]);
  await page.update(request('second', 'chat-b'));
  await page.click('本次运行始终允许');
  assert.deepEqual(fixture.calls[1], ['respond', 'chat-b', 'second', 'allow_session']);
  await page.update(request('third', 'chat-c'));
  await page.click('拒绝');
  assert.deepEqual(fixture.calls[2], ['respond', 'chat-c', 'third', 'deny']);
});

/** Keeps a late failure from an old chat from unlocking or replacing the next request's pending action. */
test('late response failure cannot reset submission state for a different request', async () => {
  const fixture = host(), first = deferred(), second = deferred();
  fixture.toolPermissions.respond = (chat, id, result) => {
    fixture.calls.push(['respond', chat, id, result]);
    return id === 'first' ? first.promise : second.promise;
  };
  const page = await screen(fixture);
  const firstClick = page.click('允许');
  await page.update(request('second', 'chat-b'));
  const secondClick = page.click('允许');
  first.reject(new Error('Request already resolved inline'));
  await firstClick;
  const duplicateClick = page.click('处理中…');
  second.resolve();
  await Promise.all([secondClick, duplicateClick]);
  assert.deepEqual(fixture.calls, [['respond', 'chat-a', 'first', 'allow'], ['respond', 'chat-b', 'second', 'allow']]);
  assert.equal(fixture.errors.length, 1);
});

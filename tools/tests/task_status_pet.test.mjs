import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync, readdirSync } from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';
import { compileComposeModule } from '../../plugins/tools/compile_compose_module.mjs';
import { composeContextScript, wrapComposeScript } from '../runtime/compose_scripts.mjs';
import { composeStreamFixture } from './support/compose_stream_fixture.mjs';

const require = createRequire(import.meta.url);
const { PetService } = require('../../plugins/packages/external/task_status_pet/dist/service.js');
const { DEFAULTS, selectView } = require('../../plugins/packages/external/task_status_pet/dist/model.js');

/** Creates an isolated main owner with observable host calls and deterministic completion time. */
function fixture(settings = {}) {
  const calls = [], decisions = [], timers = [], switches = [], logs = [];
  let pending = [], saved = { ...DEFAULTS, ...settings }, storedTasks = [], resourceFailure = false;
  const windows = new Map();
  const port = {
    log(level, event, fields) { logs.push({ level, ...fields, event }); },
    async load() { return saved; },
    async save(value) { saved = value; },
    async snapshot() { return storedTasks; },
    async approvals() { return pending; },
    async resource(mood) { if (resourceFailure) throw Error('Resource decode failed'); return `vfs:/cat/${mood}.webp`; },
    async capabilities() { return { scope: 'system', permissionGranted: true }; },
    /** Reports measured host geometry independently of the plugin's desired content. */
    async windows() { return [...windows.values()]; },
    /** Applies partial options and releases geometry when the native surface closes. */
    async set(id, content) {
      calls.push({ id, content });
      if (content) {
        const previous = windows.get(id);
        windows.set(id, { id, route: content.route, visible: true, options: { x: 0, y: 0, ...previous?.options, ...content.options } });
      } else windows.delete(id);
      return content ? 'shown' : 'closed';
    },
    async respond(chatId, requestId, decision) {
      const request = pending.find(item => item.requestId === requestId && item.chatId === chatId);
      if (!request) throw Error('Stale approval');
      decisions.push({ chatId, requestId, decision }); pending = pending.filter(item => item !== request);
    },
    async switchChat(id) { switches.push(id); },
    schedule(work) { const timer = { work, cancelled: false }; timers.push(timer); return () => { timer.cancelled = true; }; },
  };
  return { service: new PetService(port), calls, decisions, timers, switches, logs, port, windows,
    /** Moves the native surface without notifying the plugin state machine. */
    drag(x, y) { Object.assign(windows.get('cat').options, { x, y }); },
    pending(value) { pending = value; }, snapshot(value) { storedTasks = value; },
    resourceFailure(value) { resourceFailure = value; },
    latest() { return calls.filter(call => call.id === 'cat').at(-1).content; },
  };
}

/** Supplies a typed runtime transition and its authoritative active-chat snapshot. */
function runtime(chatId, state, activeChatIds = []) {
  return { chatId, state, activeChatIds, isActive: activeChatIds.includes(chatId), message: '', slot: 'main', timestamp: Date.now(),
    currentTurnToolInvocationCount: 0, currentSessionToolCount: 0, activeConversationCount: activeChatIds.length };
}

/** Supplies an exact pending request identity. */
function approval(id = 'r1') { return { chatId: 'a', requestId: id, description: 'Write a file', requestedAtMillis: 1, tool: { name: 'file_write', parameters: [] } }; }

/** Runs the packaged settings screen and serialized callbacks against its real state owner. */
function settingsUi(owner, screen = 'settingsScreen') {
  const requests = [];
  const context = vm.createContext({ console, module: { exports: {} },
    ToolPkg: { ipc: { async call(channel, request, options) {
      assert.equal(channel, 'task-status-pet');
      assert.equal(options.targetRuntime, 'main');
      requests.push(request.kind);
      return owner.service.request(request);
    } } },
    require(name) {
      assert.equal(name, './model');
      return require('../../plugins/packages/external/task_status_pet/dist/model.js');
    },
  });
  context.exports = context.module.exports;
  vm.runInContext(composeContextScript(), context);
  const source = readFileSync(new URL('../../plugins/packages/external/task_status_pet/dist/ui.js', import.meta.url), 'utf8');
  vm.runInContext(wrapComposeScript(compileComposeModule(source, 'task-status-pet-ui') +
    `\nObject.defineProperty(module.exports, "__operit_compose_entry_export", {value: ${JSON.stringify(screen)}});`), context);
  const stream = composeStreamFixture(context);
  stream.adapt();
  return { requests, stream,
    render(state = {}) { return context.__operit_render_compose_dsl({ executionContextKey: 'task-cat-settings', state }); },
    action(node, property = 'onClick', payload) { return context.__operit_dispatch_compose_dsl_action({ actionId: node.props[property].__actionId, payload }); },
  };
}

/** Enumerates the actual committed UI so assertions use current callback addresses. */
function uiNodes(node) { return [node, ...node.children.flatMap(uiNodes)]; }

test('settings opens a controls page and show/hide keep the page mounted', async () => {
  const owner = fixture();
  const ui = settingsUi(owner);
  let result = ui.render();
  assert.equal(result.tree.type, 'LazyColumn');
  assert.equal(uiNodes(result.tree).find(node => node.props.text === '显示任务猫').props.enabled, false);
  result = await ui.action(result.tree, 'onLoad');
  assert.equal(uiNodes(result.tree).filter(node => node.type === 'Switch').length, 3);
  assert.equal(owner.latest(), null);
  result = await ui.action(uiNodes(result.tree).find(node => node.props.text === '显示任务猫'));
  assert.equal(result.tree.type, 'LazyColumn');
  assert.equal(owner.latest().state.sprite, 'vfs:/cat/idle.webp');
  assert.ok(uiNodes(result.tree).some(node => node.props.text === '任务猫已显示'));
  result = await ui.action(uiNodes(result.tree).find(node => node.props.text === '隐藏'));
  assert.equal(result.tree.type, 'LazyColumn');
  assert.equal(owner.latest(), null);
  assert.deepEqual(ui.requests, ['status', 'show', 'hide']);
  assert.deepEqual(ui.stream.errors, []);
});

test('settings reports a failed show and a later click can complete normally', async () => {
  const owner = fixture();
  const ui = settingsUi(owner);
  let result = await ui.action(ui.render().tree, 'onLoad');
  owner.resourceFailure(true);
  result = await ui.action(uiNodes(result.tree).find(node => node.props.text === '显示任务猫'));
  assert.equal(result.tree.type, 'LazyColumn');
  assert.ok(uiNodes(result.tree).some(node => /Resource decode failed/.test(node.props.text)));
  assert.equal(uiNodes(result.tree).find(node => node.props.text === '显示任务猫').props.enabled, true);
  owner.resourceFailure(false);
  result = await ui.action(uiNodes(result.tree).find(node => node.props.text === '显示任务猫'));
  assert.equal(owner.latest().state.sprite, 'vfs:/cat/idle.webp');
  assert.ok(uiNodes(result.tree).some(node => node.props.text === '任务猫已显示'));
  assert.deepEqual(ui.stream.errors, []);
});

test('size slider previews locally and saves only the final value, including the smaller minimum', async () => {
  const owner = fixture();
  const ui = settingsUi(owner);
  let result = await ui.action(ui.render().tree, 'onLoad');
  const slider = () => uiNodes(result.tree).find(node => node.type === 'Slider');
  assert.equal(slider().props.min, 64);
  assert.equal(slider().props.max, 256);
  for (const value of [104, 88, 64]) {
    result = await ui.action(slider(), 'onValueChange', value);
    assert.equal(slider().props.value, value);
  }
  assert.deepEqual(ui.requests, ['status']);
  result = await ui.action(slider(), 'onValueChangeFinished', 64);
  assert.deepEqual(ui.requests, ['status', 'save']);
  assert.equal((await owner.service.request({ kind: 'status' })).settings.width, 64);
  await owner.service.request({ kind: 'show' });
  assert.equal(owner.latest().options.width, 64);
  await assert.rejects(owner.service.request({ kind: 'save', settings: { ...DEFAULTS, width: 63 } }), /设置无效/);
});

test('cat caption uses the real task, round and tool without confusing calls with rounds', async () => {
  const owner = fixture({ backgroundOnly: false });
  await owner.service.runtime({ ...runtime('a', 'executing_tool', ['a']),
    taskTitle: '整理项目文档', currentRound: 3, toolName: 'file_write', currentTurnToolInvocationCount: 12 });
  const view = owner.latest().state.view;
  const ui = settingsUi(owner, 'petScreen');
  const result = ui.render({ view, sprite: 'vfs:/cat/working.webp' });
  const lines = uiNodes(result.tree).filter(node => node.type === 'Text');
  assert.ok(lines.every(node => node.props.maxLines === 1));
  const caption = lines.map(node => node.props.text).join('\n');
  assert.equal(caption, '整理项目文档\n第 3 轮 · 执行 file_write');
  assert.ok(!caption.includes('12 轮'));
  await owner.service.runtime({ ...runtime('b', 'receiving', ['a', 'b']), taskTitle: '检查测试', currentRound: 1 });
  assert.equal(owner.latest().state.view.tasks.find(task => task.chatId === 'a').round, 3);
});

test('background opens one host-dragged window and foreground closes all surfaces', async () => {
  const f = fixture();
  await f.service.runtime(runtime('a', 'receiving', ['a']));
  assert.equal(f.latest(), null);
  await f.service.lifecycle('application_on_background');
  assert.equal(f.latest().state.view.mood, 'working');
  assert.equal(f.latest().options.draggable, true);
  assert.equal(f.latest().options.focusable, false);
  assert.equal(f.latest().options.x, undefined);
  await f.service.request({ kind: 'details' });
  await f.service.lifecycle('application_on_foreground');
  assert.equal(f.latest(), null);
  assert.equal(f.calls.filter(call => call.id === 'details').at(-1).content, null);
});

test('cat restores its last measured drag position across visibility cycles without snapping on task updates', async () => {
  const f = fixture();
  await f.service.runtime(runtime('a', 'receiving', ['a']));
  await f.service.lifecycle('application_on_background');
  f.drag(-240, 360);
  await f.service.request({ kind: 'details' });
  Object.assign(f.windows.get('details').options, { x: 900, y: 120 });
  await f.service.lifecycle('application_on_foreground');
  assert.equal(f.windows.size, 0);
  await f.service.lifecycle('application_on_foreground');
  await f.service.lifecycle('application_on_background');
  assert.equal(f.latest().options.x, -240);
  assert.equal(f.latest().options.y, 360);
  f.drag(480, 0);
  await f.service.runtime(runtime('a', 'processing', ['a']));
  assert.equal(f.latest().options.x, undefined);
  assert.equal(f.latest().options.y, undefined);
  assert.equal(f.windows.get('cat').options.x, 480);
  await f.service.lifecycle('application_on_foreground');
  await f.service.lifecycle('application_on_background');
  assert.equal(f.latest().options.x, 480);
  assert.equal(f.latest().options.y, 0);
});

test('manual hide and disabling retain session position; termination and a new owner reset it', async () => {
  const f = fixture({ backgroundOnly: false });
  let saves = 0;
  const save = f.port.save;
  f.port.save = async settings => { saves++; return save(settings); };
  await f.service.request({ kind: 'show' });
  f.drag(640, 200);
  await f.service.request({ kind: 'hide' });
  await f.service.request({ kind: 'show' });
  assert.equal(f.latest().options.x, 640);
  assert.equal(saves, 0);
  f.drag(320, 100);
  await f.service.request({ kind: 'save', settings: { ...DEFAULTS, backgroundOnly: false, enabled: false } });
  await f.service.request({ kind: 'save', settings: { ...DEFAULTS, backgroundOnly: false, width: 200 } });
  assert.equal(f.latest().options.x, 320);
  assert.equal(f.latest().options.y, 100);
  assert.equal(f.latest().options.width, 200);
  await f.service.lifecycle('application_on_terminate');
  await f.service.lifecycle('application_on_create');
  assert.equal(f.latest().options.x, undefined);
  assert.equal(f.latest().options.y, undefined);
  const next = fixture({ backgroundOnly: false });
  await next.service.request({ kind: 'show' });
  assert.equal(next.latest().options.x, undefined);
  assert.equal(next.latest().options.y, undefined);
});

test('activation uses real active tasks, not previous completed history', async () => {
  const f = fixture();
  f.snapshot([{ chatId: 'a', state: 'connecting', active: true, message: '' }, { chatId: 'old', state: 'completed', active: false, message: '' }]);
  await f.service.lifecycle('application_on_background');
  assert.equal(f.latest().state.view.mood, 'thinking');
  assert.equal(f.latest().state.view.count, 1);
});

test('approval takes precedence and removal restores active work without changing animation on unrelated updates', async () => {
  const f = fixture({ backgroundOnly: false });
  await f.service.runtime(runtime('a', 'receiving', ['a', 'b']));
  const sequence = f.latest().state.view.sequence;
  await f.service.runtime(runtime('a', 'receiving', ['a', 'b']));
  assert.equal(f.latest().state.view.sequence, sequence);
  await f.service.permissions([approval()]);
  assert.equal(f.latest().state.view.mood, 'approval');
  assert.equal(f.latest().state.view.count, 2);
  await f.service.permissions([]);
  assert.equal(f.latest().state.view.mood, 'working');
});

test('completion switches to idle after one action and a new task cancels that transition', async () => {
  const f = fixture({ backgroundOnly: false });
  await f.service.runtime(runtime('a', 'receiving', ['a']));
  await f.service.runtime(runtime('a', 'completed'));
  assert.equal(f.latest().state.view.mood, 'completed');
  f.timers.at(-1).work();
  await f.service.request({ kind: 'status' });
  assert.equal(f.latest().state.view.mood, 'idle');
  await f.service.runtime(runtime('a', 'completed'));
  await f.service.runtime(runtime('b', 'receiving', ['b']));
  assert.equal(f.timers.at(-1).cancelled, true);
  assert.equal(f.latest().state.view.mood, 'working');
});

test('remaining parallel work is kept visible and cancellation is not celebrated', async () => {
  const f = fixture({ backgroundOnly: false });
  await f.service.runtime(runtime('a', 'receiving', ['a', 'b']));
  await f.service.runtime(runtime('a', 'completed', ['b']));
  assert.equal(f.latest().state.view.mood, 'thinking');
  assert.equal(f.latest().state.view.count, 1);
  await f.service.runtime(runtime('b', 'receiving', ['b']));
  await f.service.runtime(runtime('b', 'idle'));
  assert.equal(f.latest().state.view.mood, 'idle');
});

test('approval decisions use exact request ownership and reject stale submissions', async () => {
  const f = fixture({ backgroundOnly: false });
  f.pending([approval()]);
  await f.service.permissions([approval()]);
  await f.service.request({ kind: 'respond', chatId: 'a', requestId: 'r1', decision: 'allow' });
  assert.deepEqual(f.decisions, [{ chatId: 'a', requestId: 'r1', decision: 'allow' }]);
  await assert.rejects(f.service.request({ kind: 'respond', chatId: 'a', requestId: 'r1', decision: 'allow' }), /Stale/);
  await f.service.request({ kind: 'status' });
  assert.equal(f.latest().state.view.approvals.length, 0);
});

test('disabling closes both windows and invalid geometry does not reach save', async () => {
  const f = fixture({ backgroundOnly: false });
  await f.service.request({ kind: 'details' });
  await f.service.request({ kind: 'save', settings: { ...DEFAULTS, enabled: false } });
  assert.equal(f.latest(), null);
  assert.equal(f.calls.filter(call => call.id === 'details').at(-1).content, null);
  await assert.rejects(f.service.request({ kind: 'save', settings: { ...DEFAULTS, width: NaN } }), /设置无效/);
  assert.equal((await f.service.request({ kind: 'status' })).settings.enabled, false);
});

test('missing permission remains explicit and user controls cannot request it implicitly', async () => {
  const f = fixture({ backgroundOnly: false });
  f.port.set = async (_id, content) => content ? 'permission_required' : 'closed';
  f.port.capabilities = async () => ({ scope: 'system', permissionGranted: false });
  const status = await f.service.request({ kind: 'show' });
  assert.equal(status.presentation, 'permission_required');
  assert.equal(status.permissionGranted, false);
});

test('window failures reach callers and later events retain serialized ownership', async () => {
  const f = fixture({ backgroundOnly: false });
  f.resourceFailure(true);
  await assert.rejects(f.service.request({ kind: 'show' }), /Resource decode/);
  f.resourceFailure(false);
  await f.service.runtime(runtime('b', 'receiving', ['b']));
  assert.equal(f.latest().state.view.chatId, 'b');
  assert.equal(f.latest().state.view.mood, 'working');
});

test('diagnostics explain normalized lifecycle changes and repeated state', async () => {
  const f = fixture();
  await f.service.lifecycle('application_on_background', {
    source: 'application-hook', topic: 'app.visibility.changed', platform: 'macos', occurredAtMillis: 123,
  });
  await f.service.lifecycle('application_on_background', {
    source: 'application-hook', topic: 'app.visibility.changed', platform: 'macos', occurredAtMillis: 123,
  });
  const events = f.logs.filter(log => log.event === 'lifecycle');
  assert.deepEqual(events.map(log => log.source), ['application-hook', 'application-hook']);
  assert.deepEqual(events.map(log => log.unchanged), [false, true]);
  assert.equal(f.logs.filter(log => log.event === 'visibility').length, 1);
  assert.ok(f.logs.some(log => log.event === 'visibility' && log.visible && log.reason === 'background'));
  await f.service.lifecycle('application_on_foreground');
  assert.ok(f.logs.some(log => log.event === 'visibility' && !log.visible && log.reason === 'foreground'));
});

test('diagnostics identify a blocked window operation and queued foreground without dumping content', async () => {
  const f = fixture();
  let release;
  const gate = new Promise(resolve => { release = resolve; });
  let notify;
  const waiting = new Promise(resolve => { notify = resolve; });
  const set = f.port.set;
  f.port.set = async (id, content) => {
    if (id === 'cat' && content) { notify(); await gate; }
    return set(id, content);
  };
  const opening = f.service.lifecycle('application_on_background');
  await waiting;
  const closing = f.service.lifecycle('application_on_foreground');
  assert.ok(f.logs.some(log => log.event === 'received' && log.pending === 2));
  release();
  await Promise.all([opening, closing]);
  const operations = f.logs.filter(log => log.event === 'begin');
  assert.deepEqual(operations.map(log => log.operation), [1, 2]);
  assert.ok(operations.every(log => log.queueWaitMs >= 0));
  f.resourceFailure(true);
  await assert.rejects(f.service.request({ kind: 'show' }), /Resource decode/);
  assert.ok(f.logs.some(log => log.event === 'stage.failed' && log.stage === 'resource.idle' && /Resource decode/.test(log.error)));
  assert.equal(f.latest(), null);
  const serialized = JSON.stringify(f.logs);
  assert.ok(!serialized.includes('vfs:/cat/'));
  assert.ok(!serialized.includes('随时待命'));
});

test('a stale approval never outranks the current broker snapshot after ordered updates', async () => {
  const f = fixture({ backgroundOnly: false });
  await Promise.all([f.service.permissions([approval()]), f.service.permissions([]), f.service.runtime(runtime('b', 'receiving', ['b']))]);
  assert.equal(f.latest().state.view.mood, 'working');
  assert.equal(f.latest().state.view.approvals.length, 0);
  assert.equal(selectView([{ chatId: 'a', state: 'error', active: false, message: 'failed' }], [approval()], null, 0).mood, 'approval');
});

test('plugin ships only six animated WebP assets with no atlas or poster copies', () => {
  const root = new URL('../../plugins/packages/external/task_status_pet/', import.meta.url);
  const manifest = JSON.parse(readFileSync(new URL('manifest.json', root)));
  const resources = readdirSync(new URL('resources/', root)).sort();
  assert.equal(resources.length, 6);
  assert.deepEqual(resources, manifest.resources.map(resource => resource.path.split('/').at(-1)).sort());
  for (const resource of manifest.resources) {
    const bytes = readFileSync(new URL(resource.path, root));
    assert.equal(bytes.toString('ascii', 0, 4), 'RIFF');
    assert.equal(bytes.toString('ascii', 8, 12), 'WEBP');
    assert.ok(bytes.indexOf(Buffer.from('ANIM')) > 0);
  }
});

test('late notifications after termination cannot reopen the cat or details', async () => {
  const f = fixture({ backgroundOnly: false });
  await f.service.request({ kind: 'details' });
  await f.service.lifecycle('application_on_terminate');
  await f.service.runtime(runtime('a', 'receiving', ['a']));
  assert.equal(f.latest(), null);
  assert.equal(f.calls.filter(call => call.id === 'details').at(-1).content, null);
});

test('registration installs durable hooks and routes without reading chats or opening windows', () => {
  const before = globalThis.ToolPkg;
  const routes = [], navigation = [], lifecycle = [], broadcasts = [], ipc = [];
  let runtimeHook, permissionHook;
  globalThis.ToolPkg = {
    ipc: { on(channel, handler) { ipc.push({ channel, handler }); } },
    registerUiRoute(route) { routes.push(route); },
    registerNavigationEntry(entry) { navigation.push(entry); },
    registerChatRuntimeHook(hook) { runtimeHook = hook; },
    registerToolPermissionHook(hook) { permissionHook = hook; },
    registerAppLifecycleHook(hook) { lifecycle.push(hook); },
    registerHostEventHook(hook) { broadcasts.push(hook); },
  };
  try {
    const plugin = require('../../plugins/packages/external/task_status_pet/dist/main.js');
    assert.equal(plugin.registerToolPkg(), true);
    assert.equal(routes.length, 3);
    assert.equal(routes[0].id, 'settings');
    assert.equal(routes[0].screen, require('../../plugins/packages/external/task_status_pet/dist/ui.js').settingsScreen);
    assert.equal(navigation.length, 1);
    assert.equal(navigation[0].surface, 'toolbox');
    assert.equal(navigation[0].route, routes[0].route);
    assert.equal(ipc.length, 1);
    assert.equal(runtimeHook.function, plugin.onRuntime);
    assert.equal(permissionHook.function, plugin.onPermissions);
    assert.equal(lifecycle.length, 4);
    assert.equal(broadcasts.length, 0);
  } finally { globalThis.ToolPkg = before; }
});

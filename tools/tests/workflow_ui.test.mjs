import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';

const base = new URL('../../plugins/packages/buildin/workflow/dist/', import.meta.url);
const bridgeRoot = new URL('../../core/crates/plugin/sdk/src/toolpkg/', import.meta.url);

/** Extracts the production embedded JavaScript bridge. */
function embedded(name) {
  const source = readFileSync(new URL(name, bridgeRoot), 'utf8');
  return source.slice(source.indexOf('r#"') + 3, source.lastIndexOf('"#'));
}

/** Loads CommonJS modules into a shared isolated plugin context. */
function loader(context) {
  const cache = new Map();
  /** Evaluates each module once with production relative imports. */
  function load(url) {
    if (cache.has(url.href)) return cache.get(url.href).exports;
    const module = { exports: {} };
    cache.set(url.href, module);
    vm.runInContext('(function(module,exports,require){' + readFileSync(url, 'utf8') + '\n})', context)(
      module, module.exports, name => load(new URL(name.endsWith('.js') ? name : name + '.js', url)),
    );
    return module.exports;
  }
  return load;
}

/** Boots a real main module and routes UI IPC into its registered service. */
function runtime() {
  const channels = new Map();
  const context = vm.createContext({
    console, module: { exports: {} },
    ToolPkg: { ipc: {
      /** Registers a runtime-owned service. */
      on(name, handler) { channels.set(name, handler); },
      /** Delivers messages through the same channel boundary as the host. */
      async call(name, request) {
        assert.ok(channels.has(name), 'Missing IPC channel: ' + name);
        return channels.get(name)(request, {});
      },
    } },
    PluginConfig: {
      /** Initializes an isolated in-memory plugin database. */
      async use(_key, initial) { return initial; },
      /** Confirms persistence without modifying the user's actual data. */
      async flush() {},
    },
    getToolCatalog() { return { tools: [] }; },
  });
  const load = loader(context);
  load(new URL('main.js', base));
  vm.runInContext(embedded('ToolPkgComposeDslBridge.rs'), context);
  const wrapper = embedded('ToolPkgComposeDslRuntimeScript.rs')
    .replaceAll('{{', '{').replaceAll('}}', '}').replace('{script}', '');
  vm.runInContext(wrapper, context);
  context.module.exports = load(new URL('ui/screen.js', base));
  return { context, load, channels };
}

/** Boots the public list tool with explicitly supplied persisted workflow data. */
function persistedWorkflowRuntime(database) {
  const value = runtime();
  const reads = [];
  const writes = [];
  const calls = [];
  value.context.PluginConfig = {
    /** Returns the persisted database rather than a new database. */
    async use(key, initial) {
      reads.push({ key, initial: structuredClone(initial) });
      return database;
    },
    /** Records explicit migrations without touching application storage. */
    async flush(data) {
      assert.equal(data, database);
      writes.push(structuredClone(data));
    },
  };
  /** Records routing metadata before delivering the production tool request. */
  value.context.ToolPkg.ipc.call = async (name, request, options) => {
    calls.push({ name, request: structuredClone(request), options: structuredClone(options) });
    assert.ok(value.channels.has(name), 'Missing IPC channel: ' + name);
    return value.channels.get(name)(request, {});
  };
  return { ...value, database, reads, writes, calls, tools: value.load(new URL('tools.js', base)) };
}

/** Creates retained workflow state with completed runs and a manifest template. */
function persistedWorkflowDatabase() {
  const { load } = runtime();
  const { newWorkflow } = load(new URL('model.js', base));
  const workflows = [newWorkflow('Retained A'), newWorkflow('Retained B')];
  return {
    version: 2,
    workflows,
    runs: [{
      id: 'retained-run', workflowId: workflows[0].id, workflowName: workflows[0].name,
      triggerId: null, status: 'SUCCESS', startedAt: 10, finishedAt: 20, nodes: {}, logs: [],
    }],
    fired: { 'retained-schedule': 30 },
    manifestTemplates: [{
      sourceToolPkgId: 'test.template', sourceVersion: '1.0.0', templateId: 'retained-template',
      displayName: 'Retained template', description: 'Persisted template', resourceKey: 'template',
      workflow: newWorkflow('Template'),
    }],
  };
}

/** Collects ordinary children and named slots from a serialized UI tree. */
function descendants(node) {
  return [node, ...(node.children ?? []).flatMap(descendants),
    ...Object.values(node.slots ?? {}).flatMap(slot => (Array.isArray(slot) ? slot : [slot]).flatMap(descendants))];
}

/** Renders and dispatches callbacks using the actual SDK action registry. */
async function screenDriver() {
  const runtimeValue = runtime();
  const { context } = runtimeValue;
  let frame = await context.__operit_render_compose_dsl({});
  /** Runs one current-frame callback and retains its resulting tree. */
  async function invoke(action, payload) {
    assert.ok(action?.__actionId, 'Expected a registered action');
    frame = await context.__operit_dispatch_compose_dsl_action({ actionId: action.__actionId, payload });
    return frame;
  }
  await invoke(frame.tree.props.onLoad);
  return {
    ...runtimeValue,
    invoke,
    /** Finds a node from the latest frame using an exact predicate. */
    find(predicate) { return descendants(frame.tree).find(predicate); },
    /** Returns the current serialized tree. */
    tree() { return frame.tree; },
  };
}

test('cold main module registers workflow IPC without metadata registration', async () => {
  const { channels } = runtime();
  const service = channels.get('workflow.service');
  assert.equal(typeof service, 'function');
  const state = await service({ action: 'create', name: 'Blank', description: 'Description' }, {});
  assert.equal(state.workflows[0].description, 'Description');
  assert.equal(state.workflows[0].nodes.length, 0);
});

/** Verifies the exact workflow:list entry point accepts existing version-two state. */
test('workflow:list reads persisted version 2 through cold main IPC without rewriting data', async () => {
  const database = persistedWorkflowDatabase();
  const before = structuredClone(database);
  const { tools, reads, writes, calls } = persistedWorkflowRuntime(database);
  const result = await tools.list();
  assert.deepEqual(structuredClone(result), {
    workflows: before.workflows, runs: before.runs, manifestTemplates: before.manifestTemplates,
  });
  assert.deepEqual(structuredClone(database), before);
  assert.equal(reads.length, 1);
  assert.equal(reads[0].key, 'workflows');
  assert.equal(reads[0].initial.version, 2);
  assert.equal(writes.length, 0);
  assert.deepEqual(calls, [{
    name: 'workflow.service', request: { action: 'list' }, options: { targetRuntime: 'main' },
  }]);
  result.workflows[0].name = 'Unsaved edit';
  result.runs[0].logs.push({ time: 40, nodeId: '', level: 'info', message: 'Unsaved log' });
  result.manifestTemplates.length = 0;
  assert.deepEqual(structuredClone(await tools.list()), {
    workflows: before.workflows, runs: before.runs, manifestTemplates: before.manifestTemplates,
  });
  assert.equal(reads.length, 1);
  assert.equal(writes.length, 0);
  assert.deepEqual(structuredClone(database), before);
});

/** Verifies the supported migration preserves workflows, run history and schedule state. */
test('workflow:list migrates persisted version 1 exactly once without losing retained state', async () => {
  const database = persistedWorkflowDatabase();
  database.version = 1;
  delete database.manifestTemplates;
  const before = structuredClone(database);
  const { tools, reads, writes } = persistedWorkflowRuntime(database);
  const result = await tools.list();
  assert.deepEqual(structuredClone(database), { ...before, version: 2, manifestTemplates: [] });
  assert.deepEqual(structuredClone(result), {
    workflows: before.workflows, runs: before.runs, manifestTemplates: [],
  });
  assert.deepEqual(writes, [structuredClone(database)]);
  await tools.list();
  assert.equal(reads.length, 1);
  assert.equal(writes.length, 1);
});

/** Verifies unsupported schemas remain explicit errors rather than replacing stored data. */
test('workflow:list rejects unsupported persisted versions without rewriting data', async () => {
  const database = persistedWorkflowDatabase();
  database.version = 3;
  const before = structuredClone(database);
  const { tools, writes } = persistedWorkflowRuntime(database);
  await assert.rejects(tools.list(), { message: '不支持的数据版本：3' });
  assert.equal(writes.length, 0);
  assert.deepEqual(structuredClone(database), before);
});

test('tool catalog request preserves runtime schemas for the web editor', async () => {
  const { channels, context } = runtime();
  context.getToolCatalog = () => ({
    tools: [{
      name: 'demo:send',
      description: 'Sends a structured test value.',
      parameters: [{
        name: 'enabled',
        type: 'boolean',
        description: 'Whether sending is enabled.',
        required: true,
        default: 'true',
      }],
      category: 'Demo',
      source: 'package',
      packageName: 'demo',
    }],
  });
  const snapshot = await channels.get('workflow.service')({ action: 'tool_catalog' }, {});
  assert.deepEqual(JSON.parse(JSON.stringify(snapshot.tools)), context.getToolCatalog().tools);
});

test('WebView interface decodes browser argument arrays through the production action bridge', async () => {
  const { context, load, channels } = runtime();
  let registered;
  const commands = [];
  /** Captures the structured action descriptor installed into the host WebView registry. */
  context.__operitNativeComposeWebViewControllerCommand = function(command) {
      commands.push(command);
      if (command.command === 'addJavascriptInterface') registered = command.payload.object;
      return { success: true, data: null };
  };
  context.ToolPkg.readResource = async () => '/plugin/workflow.html';
  context.module.exports = load(new URL('ui/web.js', base));
  let frame = await context.__operit_render_compose_dsl({});
  frame = await context.__operit_dispatch_compose_dsl_action({ actionId: frame.tree.props.onLoad.__actionId });
  const web = descendants(frame.tree).find(node => node.type === 'WebView');
  assert.ok(web, 'The UI must load as a WebView');
  assert.ok(registered.request.__actionId);
  frame = await context.__operit_dispatch_compose_dsl_action({ actionId: registered.request.__actionId,
    payload: [{ action: 'create', name: 'Browser bridge', description: '' }] });
  assert.equal(frame.actionResult.workflows[0].name, 'Browser bridge');
  const id = frame.actionResult.workflows[0].id;
  frame = await context.__operit_dispatch_compose_dsl_action({ actionId: registered.request.__actionId,
    payload: [{ action: 'run', id, triggerId: null, extras: {} }] });
  assert.equal(frame.actionResult.runs[0].status, 'FAILED');
  assert.equal(channels.has('workflow.progress'), true, 'WebView must receive live workflow progress');
});

test('empty execution releases its lock while the UI cannot acknowledge progress', { timeout: 1500 }, async () => {
  const { channels } = runtime();
  const progress = [];
  channels.set('workflow.progress', run => {
    progress.push(run);
    return new Promise(() => {});
  });
  const service = channels.get('workflow.service');
  const created = await service({ action: 'create', name: 'Empty', description: '' }, {});
  const id = created.workflows[0].id;
  const result = await service({ action: 'run', id, triggerId: null, extras: {} }, { callerContextKey: 'ui:blocked' });
  assert.equal(result.runs[0].status, 'FAILED');
  assert.match(result.runs[0].logs.at(-1).message, /触发节点/);
  assert.equal(progress.at(-1).status, 'FAILED');
  const deleted = await service({ action: 'delete', ids: [id] }, {});
  assert.equal(deleted.workflows.length, 0);
});

test('manual execution excludes event triggers and progress payloads are immutable snapshots', { timeout: 1500 }, async () => {
  const { channels, load } = runtime();
  const progress = [];
  channels.set('workflow.progress', run => {
    progress.push(run);
    return new Promise(() => {});
  });
  const service = channels.get('workflow.service');
  const { newNode } = load(new URL('model.js', base));
  const created = await service({ action: 'create', name: 'Manual', description: '' }, {});
  const workflow = created.workflows[0];
  const manual = newNode('trigger');
  const event = { ...newNode('trigger'), triggerType: 'event', triggerConfig: { topic: 'test' } };
  workflow.nodes = [manual, event];
  await service({ action: 'save', workflow }, {});
  const result = await service({ action: 'run', id: workflow.id, triggerId: null, extras: {} }, { callerContextKey: 'ui:blocked' });
  assert.equal(result.runs[0].status, 'SUCCESS');
  assert.equal(result.runs[0].nodes[event.id].status, 'skipped');
  assert.equal(result.runs[0].nodes[manual.id].status, 'success');
  assert.equal(progress[0].status, 'RUNNING');
  assert.equal(progress.at(-1).status, 'SUCCESS');
  await service({ action: 'delete', ids: [workflow.id] }, {});
});

test('workflow cards regroup when the actual available width changes', async () => {
  const ui = await screenDriver();
  const service = ui.channels.get('workflow.service');
  for (let index = 0; index < 4; index++) {
    await service({ action: 'create', name: 'Card ' + index, description: '' }, {});
  }
  await ui.invoke(ui.tree().props.onLoad);
  /** Sends the host measurement through the production modifier action. */
  async function resize(width) {
    const action = ui.tree().props.modifier.__modifierOps.find(op => op.name === 'onSizeChanged').args[0];
    await ui.invoke(action, { width, height: 800 });
  }
  await resize(1200);
  assert.equal(ui.find(node => node.props.key === 'workflow-row:0').children.length, 3);
  assert.ok(ui.find(node => node.props.key === 'workflow-row:3'));
  await resize(390);
  assert.equal(ui.find(node => node.props.key === 'workflow-row:0').children.length, 1);
  assert.ok(ui.find(node => node.props.key === 'workflow-row:1'));
});

test('empty workflow UI can reopen its menu and delete after execution fails', { timeout: 1500 }, async () => {
  const ui = await screenDriver();
  const progress = [];
  ui.context.ToolPkg.ipc.call = async (name, request) => {
    if (name === 'workflow.progress') {
      progress.push(request);
      return new Promise(() => {});
    }
    return ui.channels.get(name)(request, { callerContextKey: 'ui:blocked' });
  };
  const service = ui.channels.get('workflow.service');
  const created = await service({ action: 'create', name: 'Empty UI', description: '' }, {});
  await ui.invoke(ui.tree().props.onLoad);
  const card = ui.find(node => node.props.key === created.workflows[0].id);
  await ui.invoke(card.props.modifier.__modifierOps.find(op => op.name === 'clickable').args[0]);
  /** Selects an enabled editor action through the floating menu. */
  async function menu(label) {
    await ui.invoke(ui.find(node => node.props.key === 'workflow-menu').props.onClick);
    const action = ui.find(node => node.props.key === label + ':fab');
    assert.equal(action.props.enabled, true);
    await ui.invoke(action.props.onClick);
  }
  await menu('触发工作流');
  assert.ok(ui.find(node => node.props.text === '执行失败'));
  await ui.channels.get('workflow.progress')({ ...progress[0], status: 'RUNNING', finishedAt: null });
  await ui.invoke(ui.find(node => node.props.text === '确定' && node.props.onClick).props.onClick);
  await menu('删除工作流');
  const remove = ui.find(node => node.props.text === '删除' && node.props.onClick);
  assert.equal(remove.props.enabled, true);
  await ui.invoke(remove.props.onClick);
  assert.ok(ui.find(node => node.props.text === '开始创建工作流'));
  assert.equal((await service({ action: 'list' }, {})).workflows.length, 0);
});

test('real workflow screen creates a blank graph and saves only confirmed node drafts', async () => {
  const ui = await screenDriver();
  /** Clicks a labeled control from the current frame. */
  async function click(text) {
    const node = ui.find(node => node.props.text === text && node.props.onClick);
    assert.ok(node, 'Missing control: ' + text);
    await ui.invoke(node.props.onClick);
  }
  /** Opens the speed dial and chooses an action. */
  async function menu(label) {
    await ui.invoke(ui.find(node => node.props.key === 'workflow-menu').props.onClick);
    await ui.invoke(ui.find(node => node.props.key === label + ':fab').props.onClick);
  }
  await click('＋  新建工作流');
  await ui.invoke(ui.find(node => node.props.key === 'create-name').props.onValueChange, 'Test workflow');
  await ui.invoke(ui.find(node => node.props.key === 'create-description').props.onValueChange, 'Preserved description');
  await click('创建');
  assert.ok(ui.find(node => node.props.text === '暂无节点'));
  await menu('添加节点');
  await click('取消');
  let snapshot = await ui.channels.get('workflow.service')({ action: 'list' }, {});
  assert.equal(snapshot.workflows[0].nodes.length, 0);
  await menu('添加节点');
  await click('添加');
  snapshot = await ui.channels.get('workflow.service')({ action: 'list' }, {});
  assert.equal(snapshot.workflows[0].nodes.length, 1);
  assert.equal(snapshot.workflows[0].description, 'Preserved description');
  const canvas = ui.find(node => node.props.key === 'workflow-canvas');
  assert.ok(canvas);
  await ui.invoke(canvas.props.onSizeChanged, { width: 800, height: 500 });
  const measured = ui.find(node => node.props.key === 'workflow-canvas');
  const tap = measured.props.modifier.__modifierOps.find(op => op.name === 'tapGestures');
  assert.equal(tap.args[0].onTap, undefined);
  await ui.invoke(tap.args[0].onLongPress, { x: 400, y: 250 });
  assert.ok(ui.find(node => node.props.text === '↗  创建连接'));
  await click('✎  编辑节点');
  const nodeId = snapshot.workflows[0].nodes[0].id;
  await ui.invoke(ui.find(node => node.props.key === nodeId + ':name').props.onValueChange, 'Discarded edit');
  await click('取消');
  snapshot = await ui.channels.get('workflow.service')({ action: 'list' }, {});
  assert.equal(snapshot.workflows[0].nodes[0].name, '触发');
  await menu('添加节点');
  await ui.invoke(ui.find(node => node.props.key === 'node-type').props.onClick, 2);
  await click('添加');
  snapshot = await ui.channels.get('workflow.service')({ action: 'list' }, {});
  assert.equal(snapshot.workflows[0].nodes.length, 2);
  const initialCanvas = ui.find(node => node.props.key === 'workflow-canvas');
  const movingCard = initialCanvas.props.commands.filter(command => command.type === 'RoundRect' && command.color === '#FFFFFF')[1];
  /** Resolves the current drag callback after each state-changing render. */
  function dragAction(name) {
    return ui.find(node => node.props.key === 'workflow-canvas').props.modifier.__modifierOps.find(op => op.name === 'dragGestures').args[0][name];
  }
  await ui.invoke(dragAction('onDragStart'), { x: movingCard.x + movingCard.width / 2, y: movingCard.y + movingCard.height / 2 });
  await ui.invoke(dragAction('onDrag'), { deltaX: 400, deltaY: 0 });
  await ui.invoke(dragAction('onDragEnd'));
  const graphCanvas = ui.find(node => node.props.key === 'workflow-canvas');
  const cards = graphCanvas.props.commands.filter(command => command.type === 'RoundRect' && command.color === '#FFFFFF');
  const firstCard = cards[0];
  const gesture = graphCanvas.props.modifier.__modifierOps.find(op => op.name === 'tapGestures');
  await ui.invoke(gesture.args[0].onLongPress, { x: firstCard.x + firstCard.width / 2, y: firstCard.y + firstCard.height / 2 });
  await click('↗  创建连接');
  const connect = ui.find(node => node.type === 'IconButton' && node.props.key === '连接到 条件');
  assert.ok(connect);
  await ui.invoke(connect.props.onClick);
  snapshot = await ui.channels.get('workflow.service')({ action: 'list' }, {});
  assert.equal(snapshot.workflows[0].connections.length, 1);
  await click('关闭');
  await ui.invoke(ui.find(node => node.props.key === '返回工作流列表').props.onClick);
  const card = ui.find(node => node.props.key === snapshot.workflows[0].id);
  await ui.invoke(card.props.modifier.__modifierOps.find(op => op.name === 'clickable').args[0]);
  assert.ok(ui.find(node => node.props.key === 'workflow-canvas'));
});

test('graph fit, snapping and edge anchors match the Kotlin geometry', () => {
  const { load } = runtime();
  const { newWorkflow, newNode } = load(new URL('model.js', base));
  const { fitViewport, snapPosition, edgePoints } = load(new URL('ui/canvas.js', base));
  const graph = newWorkflow('Geometry');
  graph.nodes = [newNode('trigger', 0, 0), newNode('condition', 0, 200)];
  assert.deepEqual(JSON.parse(JSON.stringify(snapPosition(63, -22))), { x: 80, y: -40 });
  const ends = edgePoints(...graph.nodes);
  assert.deepEqual(JSON.parse(JSON.stringify(ends)), [{ x: 60, y: 80 }, { x: 60, y: 200 }]);
  const viewport = fitViewport(graph, { x: 0, y: 0, width: 800, height: 600, zoom: 1 });
  assert.equal(viewport.x + 60 * viewport.zoom, 400);
  assert.equal(viewport.y + 140 * viewport.zoom, 300);
  assert.ok(viewport.zoom >= 0.25 && viewport.zoom <= 3);
});

import { composeStreamFixture } from './support/compose_stream_fixture.mjs';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';

const root = new URL('../../core/crates/plugin/sdk/src/toolpkg/', import.meta.url);
/** Extracts the exact production SDK string without building an application. */
function embedded(name) {
  const text = readFileSync(new URL(name, root), 'utf8');
  return text.slice(text.indexOf('r#"') + 3, text.lastIndexOf('"#'));
}
/** Starts a genuine compiled SDK screen in an isolated JS session. */
function session(source) {
  const events = [];
  const nodes = new Map();
  let rootId;
  let revision = 0;
  const context = vm.createContext({ console, module: { exports: {} },
    __operit_call_runtime_ref: {
      /** Receives each commit synchronously before the host polls command completion. */
      sendComposeResponse() { throw new Error("Fixture sink must be attached before execution"); }
    } });
  context.exports = context.module.exports;
  vm.runInContext(readFileSync(new URL('vendor/acorn.js', root), 'utf8'), context);
  context.__operitAcorn = context.module.exports;
  context.module = { exports: {} };
  for (const name of ['ToolPkgComposeDslCompiler.js', 'ToolPkgComposeDslRetained.js', 'ToolPkgComposeDslReactive.js']) {
    vm.runInContext(readFileSync(new URL(name, root), 'utf8'), context);
  }
  vm.runInContext(embedded('ToolPkgComposeDslBridge.rs'), context);
  const fixture = composeStreamFixture(context, { projectTree: false, inspectStorage: false,
    /** Retains every real stream response for storage and revision assertions. */
    onResponse(_phase, response) {
      events.push(response);
      if (response.update) {
        for (const node of response.update.upserts) nodes.set(node.id, node);
        for (const id of response.update.removed) nodes.delete(id);
        rootId = response.update.rootId;
      }
    },
  });
  let rawAction;
  /** Re-evaluates the wrapper as the host does when a script is loaded again. */
  function install(script) {
    const compiled = context.OperitComposeCompiler.compile(script, 'state-lifecycle');
    vm.runInContext(embedded('ToolPkgComposeDslRuntimeScript.rs').replaceAll('{{', '{').replaceAll('}}', '}').replace('{script}', compiled), context);
    rawAction = context.__operit_dispatch_compose_dsl_action;
    fixture.adapt();
  }
  install(source);
  return { context, events, install,
    /** Renders or updates the explicitly identified retained session. */
    render(options = {}) { const response = context.__operit_render_compose_dsl({ executionContextKey: 'page-a', ...options }); return response; },
    /** Starts a command while leaving its completion unconsumed, as independent Rust tasks can do. */
    beginAction(name, payload) {
      const tree = nodes.get(rootId);
      return rawAction({ actionId: tree.props[name].__actionId, payload });
    },
    /** Invokes the currently mounted action from its last delivered node record. */
    async action(name, payload) {
      const tree = nodes.get(rootId);
      const response = await context.__operit_dispatch_compose_dsl_action({ actionId: tree.props[name].__actionId, payload });
      return response;
    },
  };
}
/** Rejects snapshots and proves normal response traversal cannot touch JS-only values. */
function noSnapshot(response) {
  assert.equal(Object.hasOwn(response, 'state'), false);
  assert.equal(Object.hasOwn(response, 'memo'), false);
  assert.ok(JSON.stringify(response).length < 10000);
}
const screen = `module.exports.default = function(ctx) {
  const [count, setCount] = ctx.useState('count', 0);
  const [object] = ctx.useState('object', { nested: { value: 1 }, list: [1, 2] });
  const ref = ctx.useRef('ref', { value: 1 });
  const memo = ctx.useMemo('memo', () => ({ value: 7 }), []);
  const [mutable, setMutable] = ctx.useMutable('mutable', 0);
  globalThis.captured = { object, ref, memo };
  return ctx.UI.Column({
    edit: next => { setCount(next); return next; },
    editAsync: async next => { setCount(next); await Promise.resolve(); await Promise.resolve(); return next; },
    mutate: () => { object.nested.value++; object.list.push(3); delete object.list[0]; ref.current.value++; memo.value++; setMutable(mutable + 1); },
    wait: async () => { await new Promise(resolve => { globalThis.release = resolve; }); ref.current.value++; setCount(9); return ref.current.value; },
    fail: async () => { setCount(6); throw Error('intentional action error'); },
    inspect: () => ({ count, nested: object.nested.value, ref: ref.current.value, memo: memo.value, mutable }),
  }, [ctx.UI.Text({ text: String(count) })]);
};`;

test('retained responses never read large state or memo, including intermediate events', async () => {
  const s = session(screen);
  noSnapshot(s.render({ state: { huge: 'x'.repeat(4 * 1024 * 1024) } }));
  vm.runInContext(`
    Object.defineProperty(__operit_compose_bundle.stateStore, 'poison', { enumerable: true, get() { throw Error('state exported'); } });
    Object.defineProperty(__operit_compose_bundle.memoStore, 'poison', { enumerable: true, get() { throw Error('memo exported'); } });
    __operit_compose_bundle.memoStore.pending = Promise.resolve(1);
    __operit_compose_bundle.memoStore.callback = () => 2;
    __operit_compose_bundle.memoStore.cycle = __operit_compose_bundle.memoStore;
  `, s.context);
  noSnapshot(s.render());
  noSnapshot(await s.action('editAsync', 3));
  assert.ok(s.events.length > 0);
  s.events.forEach(noSnapshot);
});

test('nested mutations, refs and memo keep object identity through actions and same-context script reload', async () => {
  const s = session(screen); s.render();
  const bundle = s.context.__operit_compose_bundle;
  const { object, ref, memo } = s.context.captured;
  await s.action('mutate');
  s.install(screen);
  const response = s.render({ state: { count: -1 }, memo: { ref: { current: -1 } } });
  noSnapshot(response);
  assert.equal(s.context.__operit_compose_bundle, bundle);
  assert.equal(s.context.captured.object, object);
  assert.equal(s.context.captured.ref, ref);
  assert.equal(s.context.captured.memo, memo);
  assert.equal(ref.current.value, 2); assert.equal(memo.value, 8);
  assert.equal(object.nested.value, 2); assert.equal(object.list.length, 3);
  assert.equal(0 in object.list, false);
  assert.equal(response.update.reset, false);
  assert.equal((await s.action('inspect')).actionResult.mutable, 1);
});

test('compiled state consumers observe explicit host input updates', () => {
  const s = session(`module.exports.default = function(ctx) {
    const [label] = ctx.useState('label', 'initial');
    return ctx.UI.Text({ text: label });
  };`);
  s.render();
  const response = s.render({ __operit_update_inputs: true, __operit_input_state: { label: 'host change' } });
  assert.ok(response.update.upserts.some(node => node.props.text === 'host change'));
  noSnapshot(response);
});

test('pending action survives same-context script reload without cloning its ref or Promise', async () => {
  const s = session(screen); s.render();
  const ref = s.context.captured.ref;
  const pending = s.action('wait');
  s.install(screen); s.render();
  assert.equal(s.context.captured.ref, ref);
  s.context.release();
  noSnapshot(await pending);
  assert.equal(ref.current.value, 2);
  assert.equal((await s.action('inspect')).actionResult.count, 9);
});

test('action rejection does not discard live state or prevent the next action', async () => {
  const s = session(screen); s.render();
  await assert.rejects(s.action('fail'), /intentional action error/);
  assert.equal((await s.action('inspect')).actionResult.count, 6);
  noSnapshot(await s.action('edit', 10));
});

test('different execution context and fresh engine do not inherit another page state', async () => {
  const s = session(screen); s.render(); await s.action('edit', 12);
  const old = s.context.__operit_compose_bundle;
  assert.equal(s.render({ executionContextKey: 'page-b' }).update.reset, true);
  assert.notEqual(s.context.__operit_compose_bundle, old);
  assert.equal((await s.action('inspect')).actionResult.count, 0);
  const fresh = session(screen); fresh.render();
  assert.equal((await fresh.action('inspect')).actionResult.count, 0);
});

test('no-render action returns its result without state or memo', async () => {
  const s = session(screen); s.render();
  const response = await s.action('inspect', { __no_render: true });
  noSnapshot(response);
  assert.equal(Object.hasOwn(response, 'update'), false);
  assert.equal(response.actionResult.memo, 7);
});

test('every DSL command uses only its owned stream and never exports storage', async () => {
  const s = session(screen);
  const initial = s.render();
  noSnapshot(initial);
  const result = await s.action('edit', 5);
  noSnapshot(result);
  assert.ok(result.update);
  assert.equal(s.context.__operit_compose_bundle.stateStore.count, 5);
});

test('host input update cannot reuse a different execution context', () => {
  const s = session(screen); s.render();
  const previous = s.context.__operit_compose_bundle;
  const response = s.render({ executionContextKey: 'page-b', __operit_update_inputs: true, __operit_input_state: { count: 42 } });
  assert.notEqual(s.context.__operit_compose_bundle, previous);
  assert.equal(response.update.reset, true);
  assert.equal(s.context.__operit_compose_bundle.stateStore.count, 42);
});

test('same-entry render refreshes environment-dependent compiled expressions', () => {
  const s = session(`module.exports.default = function(ctx) {
    const [value] = ctx.useState('value', 0);
    const color = ctx.MaterialTheme.colorScheme.primary;
    return ctx.UI.Text({ text: value, color });
  };`);
  s.render();
  const bundle = s.context.__operit_compose_bundle;
  bundle.ctx.MaterialTheme.colorScheme.primary = '#ff102030';
  const response = s.render();
  assert.equal(s.context.__operit_compose_bundle, bundle);
  assert.ok(response.update.upserts.some(node => node.props.color === '#ff102030'));
  assert.ok(s.context.OperitComposeReactive.metrics(bundle.ctx).rootRuns >= 2);
});

/** Loads the same controller registration pattern used by the character-card HTML screen. */
function webViewSession() {
  const commands = [];
  const s = session(`module.exports.default = function(ctx) {
    const [ready, setReady] = ctx.useState('ready', false);
    const controller = ctx.createWebViewController('character-memory-web');
    globalThis.controller = controller;
    return ctx.UI.Box({ onLoad: function() {
      controller.addJavascriptInterface('CharacterMemoryHost', {
        currentTheme: () => 'dark', request: value => value
      });
      setReady(true);
    } }, ready ? ctx.UI.WebView({ controller, url: 'https://characters.operit.local/' }) : ctx.UI.Text({text:'loading'}));
  };`);
  s.context.NativeInterface = {
    /** Records the real serialized Host controller command without changing its callback IDs. */
    composeWebViewControllerCommand(raw) { commands.push(JSON.parse(raw)); return {success:true, data:null}; }
  };
  return {s, commands};
}

test('WebView interface action survives onLoad intermediate and final retained commits', async () => {
  const {s, commands} = webViewSession(); s.render(); await s.action('onLoad');
  const id = commands[0].payload.object.currentTheme.__actionId;
  assert.equal(id, '__action_3');
  for (let index = 0; index < 5; index++) {
    const response = await s.context.__operit_dispatch_compose_dsl_action({actionId:id});
    assert.equal(response.actionResult, 'dark'); noSnapshot(response);
  }
});

test('WebView interface replacement keeps method IDs and releases removed methods explicitly', async () => {
  const {s, commands} = webViewSession(); s.render(); await s.action('onLoad');
  const initial = commands[0].payload.object;
  vm.runInContext("controller.addJavascriptInterface('CharacterMemoryHost', {currentTheme: () => 'light'})", s.context);
  assert.equal(commands[1].payload.object.currentTheme.__actionId, initial.currentTheme.__actionId);
  assert.equal(s.context.__operit_compose_bundle.invokeAction(initial.currentTheme.__actionId), 'light');
  assert.throws(() => s.context.__operit_compose_bundle.invokeAction(initial.request.__actionId), /compose action not found/);
  s.render();
  assert.equal(s.context.__operit_compose_bundle.invokeAction(initial.currentTheme.__actionId), 'light');
  s.context.controller.removeJavascriptInterface('CharacterMemoryHost');
  assert.throws(() => s.context.__operit_compose_bundle.invokeAction(initial.currentTheme.__actionId), /compose action not found/);
});

test('actual packaged character-card screen keeps currentTheme callable after HTML path refresh', async () => {
  const source = readFileSync(new URL('../../plugins/packages/buildin/character_cards/dist/ui/main/index.ui.js', import.meta.url), 'utf8');
  const commands = [];
  const s = session(source);
  s.context.NativeInterface = {
    /** Captures production interface descriptors crossing the Host boundary. */
    composeWebViewControllerCommand(raw) { commands.push(JSON.parse(raw)); return {success:true, data:null}; }
  };
  s.context.ToolPkg = {
    /** Supplies a fixture resource path without reading personal application data. */
    async readResource() { return '/fixture/character-memory.html'; }
  };
  s.render({theme:{brightness:'dark', colors:{primary:'#ff102030'}}});
  await s.action('onLoad');
  assert.equal(s.context.__operit_compose_bundle.stateStore['character-memory-html'], '/fixture/character-memory.html');
  const methods = commands[0].payload.object;
  for (let index = 0; index < 5; index++) {
    const response = await s.context.__operit_dispatch_compose_dsl_action({actionId:methods.currentTheme.__actionId});
    assert.equal(response.actionResult.brightness, 'dark'); noSnapshot(response);
    const screenResponse = await s.context.__operit_dispatch_compose_dsl_action({actionId:methods.currentScreen.__actionId, payload:[]});
    assert.equal(screenResponse.actionResult.input.mode, 'manage');
  }
});


test('Host-rejected interface replacement preserves its last accepted callbacks', async () => {
  const {s, commands} = webViewSession(); s.render(); await s.action('onLoad');
  const id = commands[0].payload.object.currentTheme.__actionId;
  s.context.NativeInterface.composeWebViewControllerCommand =
    /** Rejects the transport call before any interface descriptor is accepted. */
    function() { throw new Error('Host rejected interface command'); };
  assert.throws(() => s.context.controller.addJavascriptInterface('CharacterMemoryHost', {currentTheme: () => 'light'}), /Host rejected interface command/);
  assert.equal(s.context.__operit_compose_bundle.invokeAction(id), 'dark');
  assert.throws(() => s.context.controller.removeJavascriptInterface('CharacterMemoryHost'), /Host rejected interface command/);
  assert.equal(s.context.__operit_compose_bundle.invokeAction(id), 'dark');
});


test('session commits remain contiguous when a previous command completion is not consumed', async () => {
  const s = session(screen); s.render();
  const unreadCompletion = await s.beginAction('edit', 1);
  await s.action('editAsync', 2);
  assert.equal(unreadCompletion, null, 'stream commands must not return a second copy of a committed UI response');
  const revisions = s.events.map(event => event.update.revision);
  assert.ok(revisions.length >= 4, 'initial, intermediate and final commits must all use the same stream');
  assert.deepEqual(revisions, Array.from({length: revisions.length}, (_, index) => index + 1));
});

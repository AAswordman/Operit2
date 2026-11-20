import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { composeContextScript, wrapComposeScript } from '../../../../../tools/runtime/compose_scripts.mjs';
import { composeStreamFixture } from '../../../../../tools/tests/support/compose_stream_fixture.mjs';
const root = fileURLToPath(new URL('..', import.meta.url));
const require = createRequire(new URL('../../../buildin/workflow/package.json', import.meta.url));
const ts = require('typescript');
const storageSource = readFileSync(new URL('../../../../../core/crates/plugin/sdk/src/js_sdk/storage_runtime.js', import.meta.url), 'utf8');
const contextSource = readFileSync(new URL('../../../../../core/crates/plugin/javascript-bridge/src/javascript/RuntimeContext.script.js', import.meta.url), 'utf8');

/** Evaluates current TypeScript sources without depending on an older dist directory. */
function sourceLoader(context) {
  const modules = new Map();
  /** Loads each CommonJS module once inside its owning runtime realm. */
  function load(filename) {
    if (modules.has(filename)) return modules.get(filename).exports;
    const module = { exports: {} }; modules.set(filename, module);
    const source = ts.transpileModule(readFileSync(filename, 'utf8'), {
      compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 }, fileName: filename,
    }).outputText;
    const wrapper = vm.runInContext(`(function(exports, require, module) {${source}\n})`, context, { filename });
    wrapper(module.exports, specifier => load(path.resolve(path.dirname(filename), specifier + '.ts')), module);
    return module.exports;
  }
  return load;
}
const model = sourceLoader(vm.createContext({}))(path.join(root, 'src/model.ts'));

/** No platform API exists in this mock: the production plugin must use only unified message scheduling. */
function host() {
  let now = 1800000000000, saved, failStorage = false, failCancel = false, lostResponse = false, modelError = false, modelHook;
  const scheduled = new Map(), calls = [];
  const chats = [{ id: 'chat-1', title: '测试对话' }, { id: 'chat-2', title: '第二个对话' }];
  /** Boots an isolated plugin realm using the production DataStore facade. */
  function boot() {
    class Clock extends Date { constructor(...args) { super(...(args.length ? args : [now])); } static now() { return now; } }
    const handles = new Map();
    let handleSequence = 0;
    /** Supplies native preference operations while the production SDK owns immutable snapshots and edits. */
    async function storageRequest(request) {
      const snapshot = () => ({ values: saved === undefined ? {} : { state: structuredClone(saved) }, schemaVersion: 0 });
      if (request.op === 'open_preferences') {
        const handle = String(++handleSequence); handles.set(handle, { edit: null });
        return { op: 'open', handle, path: request.path };
      }
      const handle = handles.get(request.handle);
      assert.ok(handle, 'Storage handle must belong to this runtime');
      switch (request.op) {
        case 'preferences_data': return { op: request.op, snapshot: snapshot() };
        case 'preferences_begin_edit':
          assert.equal(handle.edit, null);
          handle.edit = String(++handleSequence);
          return { op: request.op, edit: { token: handle.edit, snapshot: snapshot() } };
        case 'preferences_cancel_edit':
          assert.equal(request.token, handle.edit); handle.edit = null;
          return { op: request.op };
        case 'preferences_finish_edit':
          assert.equal(request.token, handle.edit); handle.edit = null;
          if (failStorage) throw Error('disk failed');
          for (const edit of request.edits) {
            assert.equal(edit.op, 'set'); assert.equal(edit.key, 'state');
            saved = structuredClone(edit.value);
          }
          return { op: request.op, snapshot: snapshot() };
        default: assert.fail('Unexpected storage operation: ' + request.op);
      }
    }
    const tools = {
      Storage: { request: storageRequest },
      Chat: {
        async scheduleMessage(value) {
          calls.push(['schedule', structuredClone(value)]);
          let message = scheduled.get(value.id);
          if (!message) {
            assert.equal(value.text, '');
            assert.ok(value.generation.instruction.trim());
            calls.push(['model', structuredClone(value.generation)]);
            if (modelHook) await modelHook();
            if (modelError) throw Error('model offline');
            message = { ...structuredClone(value), text: '最近有喝到喜欢的茶吗？', status: 'scheduled', error: null };
          }
          scheduled.set(value.id, message);
          if (lostResponse) throw Error('response lost');
          return { message };
        },
        async cancelScheduledMessage(id) {
          calls.push(['cancel', id]); if (failCancel) throw Error('host unavailable');
          const message = scheduled.get(id); if (message) message.status = 'cancelled';
          return { id, cancelled: true };
        },
        async listScheduledMessages() { return { messages: [...scheduled.values()].map(value => structuredClone(value)) }; },
        async listAll() { return { chats }; },
        async getMessages() { throw Error('Plugin must not reconstruct conversation context'); },
        async call() { throw Error('Plugin must use contextual host generation'); },
      },
    };
    const context = vm.createContext({ Tools: tools, ToolPkg: { getLocalDataDir: () => '/device/plugin' }, Date: Clock, console: { error() {} } });
    vm.runInContext(storageSource + '\n__operitInstallStorageFacade();', context);
    const load = sourceLoader(context);
    return { context, load, service: load(path.join(root, 'src/service.ts')) };
  }
  return { boot, runtime: () => boot().service, calls, scheduled, chats, state: () => structuredClone(saved), advance: delta => { now += delta; },
    storageFail: value => { failStorage = value; }, cancelFail: value => { failCancel = value; }, loseResponse: value => { lostResponse = value; },
    modelFail: value => { modelError = value; }, modelHook: value => { modelHook = value; }, seed: value => { saved = structuredClone(value); },
    complete() { for (const value of scheduled.values()) if (value.status === 'scheduled' && value.timestamp <= now) value.status = 'delivered'; },
  };
}
/** Creates a deterministic chat rule for scheduling and restart scenarios. */
function rule(patch = {}) { return { ...model.defaultRule('chat-1', '测试对话'), enabled: true, intervalMinutes: 1, prepareCount: 2, probability: 100, ...patch }; }
const count = (h, name) => h.calls.filter(call => call[0] === name).length;

/** Collects the host-facing children and named slots of a rendered settings page. */
function descendants(node) {
  return [node, ...(node.children ?? []).flatMap(descendants), ...Object.values(node.slots ?? {}).flatMap(nodes => nodes.flatMap(descendants))];
}

/** Renders the actual settings page with main-runtime calls crossing the production context bridge. */
async function screenDriver(h) {
  const main = h.boot(), channels = new Map();
  /** Installs the real runner serialization with an explicit in-process IPC transport. */
  function bridge(context) {
    context.ToolPkg.ipc = {
      on(name, handler) { if (context === main.context) channels.set(name, handler); },
      async call(name, request, options) {
        assert.equal(options.targetRuntime, 'main');
        assert.ok(channels.has(name));
        const result = await channels.get(name)(structuredClone(request));
        return result === undefined ? undefined : structuredClone(result);
      },
    };
    vm.runInContext('globalThis.__operitExpose = (name, value) => { globalThis[name] = value; }; globalThis.__operitCompileJavaScript = source => new Function(source);', context);
    vm.runInContext(contextSource + '\nRuntimeContext.__operitEnsureContextRunnerRegistered();', context);
  }
  bridge(main.context);
  main.load(path.join(root, 'src/main.ts'));
  const context = vm.createContext({ console, Date: main.context.Date, ToolPkg: {}, module: { exports: {} } });
  bridge(context);
  vm.runInContext(composeContextScript() + wrapComposeScript(), context);
  context.module.exports = sourceLoader(context)(path.join(root, 'src/ui.ts'));
  composeStreamFixture(context).adapt();
  let frame = await context.__operit_render_compose_dsl({});
  /** Dispatches only actions from the latest committed settings page. */
  async function invoke(action, payload) {
    assert.ok(action?.__actionId, 'Expected a settings action');
    frame = await context.__operit_dispatch_compose_dsl_action({ actionId: action.__actionId, payload });
  }
  await invoke(frame.tree.props.onLoad);
  return { invoke, find: predicate => descendants(frame.tree).find(predicate) };
}

test('zero probability never invokes AI or schedules a message; random intervals and probability are bounded', async () => {
  const h = host(), service = h.runtime(); await service.setForeground(true); await service.saveRule(rule({ probability: 0 }));
  assert.equal(count(h, 'model'), 0); assert.equal(h.scheduled.size, 0);
  assert.ok(h.state().jobs.every(job => job.status === 'skipped'));
  assert.equal(model.hits(0, () => 0), false); assert.equal(model.hits(100, () => .999999), true);
  assert.equal(model.hits(40, () => .4), false); assert.equal(model.hits(40, () => .3999), true);
  const r = rule({ intervalMinutes: 10, randomMinutes: 5 });
  assert.equal(model.nextOccurrence(r, 0, () => 0), 600000); assert.equal(model.nextOccurrence(r, 0, () => .999999), 900000);
});
test('all delivery is handed to the host without platform queries, plugin timers or direct writes', async () => {
  const h = host(), service = h.runtime(); await service.setForeground(true); await service.saveRule(rule());
  assert.equal(h.scheduled.size, 2); assert.equal(count(h, 'model'), 2);
  assert.ok(h.state().jobs.every(job => job.status === 'scheduled'));
  await service.setForeground(false); h.advance(60000); h.complete();
  await Promise.all([service.tick(), service.tick(), service.tick()]);
  assert.equal(h.state().jobs.filter(job => job.status === 'delivered').length, 1);
  assert.equal(count(h, 'schedule'), 2); assert.equal(count(h, 'model'), 2);
});
test('host delivery survives plugin restart; reading receipts never sends again', async () => {
  const h = host(); let service = h.runtime(); await service.setForeground(true); await service.saveRule(rule()); service.stop();
  h.advance(120000); h.complete(); service = h.runtime(); await service.tick();
  assert.ok(h.state().jobs.every(job => job.status === 'delivered')); assert.equal(count(h, 'schedule'), 2);
});
test('response loss reconciles the accepted durable identity without a duplicate schedule', async () => {
  const h = host(), service = h.runtime(); await service.setForeground(true); h.loseResponse(true); await service.saveRule(rule());
  assert.ok(h.state().jobs.every(job => job.status === 'scheduled')); assert.equal(h.scheduled.size, 2);
  service.stop(); h.loseResponse(false); await h.runtime().tick();
  assert.ok(h.state().jobs.every(job => job.status === 'scheduled')); assert.equal(count(h, 'schedule'), 2);
});
test('changing or disabling a rule cancels every accepted message through the same API', async () => {
  const h = host(), service = h.runtime(); await service.setForeground(true); await service.saveRule(rule());
  [...h.scheduled.values()][0].status = 'preparing'; await service.tick();
  await service.saveRule(rule({ enabled: false })); assert.equal(count(h, 'cancel'), 2);
  assert.ok([...h.scheduled.values()].every(job => job.status === 'cancelled'));
});
test('restart resumes an occurrence interrupted before its host request without drawing a new time', async () => {
  const h = host(), dueAt = 1800000060000;
  h.seed({ version: 1, generationVersion: 2, rules: [rule({ prepareCount: 1, nextAt: dueAt })],
    jobs: [{ id: 'interrupted', chatId: 'chat-1', title: 'test', dueAt, text: '', status: 'preparing', error: '' }] });
  await h.runtime().setForeground(true);
  assert.equal(count(h, 'model'), 1); assert.equal(h.scheduled.get('interrupted').timestamp, dueAt);
  assert.equal(h.state().jobs[0].status, 'scheduled');
});
test('failed cancellation leaves the rule disabled and retains identities for retry', async () => {
  const h = host(), service = h.runtime(); await service.setForeground(true); await service.saveRule(rule()); h.cancelFail(true);
  await assert.rejects(service.saveRule(rule({ enabled: false })), /host unavailable/);
  assert.equal(h.state().rules[0].enabled, false); h.cancelFail(false); await service.tick();
  assert.ok([...h.scheduled.values()].every(job => job.status === 'cancelled'));
});
test('persistence failure prevents new uncommitted model and scheduling actions', async () => {
  const h = host(), service = h.runtime(); await service.setForeground(true); h.storageFail(true);
  await assert.rejects(service.saveRule(rule()), /存储失败/); assert.equal(h.scheduled.size, 0); assert.equal(count(h, 'model'), 0);
});
test('leaving foreground during host generation retains its accepted schedule and stops the batch', async () => {
  const h = host(), service = h.runtime(); await service.setForeground(true);
  h.modelHook(() => service.setForeground(false)); await service.saveRule(rule());
  assert.equal(count(h, 'model'), 1); assert.equal(h.scheduled.size, 1); assert.equal(h.state().jobs[0].status, 'scheduled');
  h.modelHook(undefined); await service.setForeground(true); assert.equal(h.scheduled.size, 2);
});
test('invalidated replies are prepared again at the original time without rerolling probability', async () => {
  const h = host(), service = h.runtime(); await service.setForeground(true); await service.saveRule(rule());
  const original = [...h.scheduled.values()][0]; original.status = 'invalidated'; original.error = 'context changed';
  await service.tick();
  const replacement = [...h.scheduled.values()].find(value => value.id !== original.id && value.timestamp === original.timestamp);
  assert.ok(replacement); assert.equal(replacement.status, 'scheduled');
  assert.equal(h.state().jobs.filter(job => job.status === 'invalidated').length, 1);
  const before = count(h, 'model'); await service.tick(); assert.equal(count(h, 'model'), before);
});
test('upgrading retires old text-only appointments before preparing contextual replies', async () => {
  const h = host();
  h.scheduled.set('legacy', { id: 'legacy', status: 'scheduled', text: 'wrong role', timestamp: 1800000060000 });
  h.seed({ version: 1, rules: [rule({ nextAt: 1800000060000 })], jobs: [{ id: 'legacy', chatId: 'chat-1', title: 'old', dueAt: 1800000060000, text: 'wrong role', status: 'scheduled', error: '' }] });
  const service = h.runtime(); await service.setForeground(true);
  assert.equal(h.scheduled.get('legacy').status, 'cancelled'); assert.equal(h.state().generationVersion, 2);
  assert.equal(h.calls[0][0], 'cancel'); assert.equal(count(h, 'model'), 2);
});
test('a failed upgrade cancellation cannot enable generation and is retried on the same runtime', async () => {
  const h = host();
  h.scheduled.set('legacy', { id: 'legacy', status: 'scheduled', text: 'wrong role', timestamp: 1800000060000 });
  h.seed({ version: 1, rules: [rule()], jobs: [] }); h.cancelFail(true);
  const service = h.runtime(); await assert.rejects(service.setForeground(true), /host unavailable/);
  assert.equal(count(h, 'model'), 0); assert.equal(h.state().generationVersion, undefined);
  h.cancelFail(false); await service.tick();
  assert.equal(h.scheduled.get('legacy').status, 'cancelled'); assert.equal(h.state().generationVersion, 2);
});
test('model failure is visible and stops the preparation batch', async () => {
  const h = host(), service = h.runtime(); await service.setForeground(true); h.modelFail(true); await service.saveRule(rule());
  assert.equal(count(h, 'model'), 1); assert.equal(h.state().jobs[0].status, 'failed'); assert.match(h.state().jobs[0].error, /model offline/);
});
test('a global bound limits plans across independent chat rules', async () => {
  const h = host(), service = h.runtime(); await service.setForeground(true);
  for (let index = 0; index < 8; index++) { const id = 'extra-' + index; h.chats.push({ id, title: id }); await service.saveRule(rule({ chatId: id, prepareCount: 8 })); }
  assert.equal(h.state().jobs.filter(job => model.pending(job)).length, 32); assert.equal(h.scheduled.size, 32);
});
test('validation rejects invalid rules and corrupt persisted state', () => {
  assert.throws(() => model.validateRule(rule({ probability: 101 })));
  assert.throws(() => model.validateRule(rule({ intervalMinutes: 0 })));
  assert.throws(() => model.validateState({ version: 1, rules: [], jobs: [{ id: 'bad' }] }));
});

test('production DataStore snapshots stay deeply immutable while the service owns a detached mutable copy', async () => {
  const h = host();
  h.seed({ version: 1, generationVersion: 2, rules: [rule({ enabled: false })], jobs: [] });
  const { context, service } = h.boot();
  const store = await context.Tools.Storage.dataStore.open({ path: '/device/plugin/proactive-messages.preferences.json' });
  const saved = (await store.data()).get('state');
  assert.ok(Object.isFrozen(saved)); assert.ok(Object.isFrozen(saved.rules)); assert.ok(Object.isFrozen(saved.rules[0]));
  await service.setForeground(true);
  await service.saveRule(rule({ intervalMinutes: 2 }));
  assert.equal(saved.rules[0].enabled, false); assert.equal(saved.jobs.length, 0);
  assert.equal(h.state().rules[0].intervalMinutes, 2); assert.equal(h.scheduled.size, 2);
  const snapshot = await service.snapshot();
  snapshot.state.rules[0].prompt = 'Unsaved UI edit'; snapshot.state.jobs.length = 0;
  assert.notEqual(h.state().rules[0].prompt, 'Unsaved UI edit'); assert.equal(h.state().jobs.length, 2);
});

test('restart recovers delivering records and continues from the accepted host receipt', async () => {
  const h = host(), initial = h.runtime();
  await initial.setForeground(true); await initial.saveRule(rule()); initial.stop();
  const saved = h.state(); saved.jobs[0].status = 'delivering'; h.seed(saved);
  const restarted = h.runtime(); await restarted.setForeground(true);
  assert.ok(h.state().jobs.every(job => job.status === 'scheduled')); assert.equal(count(h, 'schedule'), 2);
  await restarted.removeRule('chat-1');
  assert.equal(h.state().rules.length, 0); assert.ok(h.state().jobs.every(job => job.status === 'cancelled'));
});

test('corrupt stored state is rejected without replacing rules or creating appointments', async () => {
  const h = host(), saved = { version: 1, generationVersion: 2, rules: [rule({ probability: 101 })], jobs: [] };
  h.seed(saved);
  await assert.rejects(h.runtime().setForeground(true), /触发概率/);
  assert.deepEqual(h.state(), saved); assert.equal(count(h, 'schedule'), 0);
});

test('settings reopen persisted rules, save edits and cancel appointments when deleting through the UI', async () => {
  const h = host(), initial = h.runtime();
  await initial.setForeground(true); await initial.saveRule(rule()); initial.stop();
  const ui = await screenDriver(h);
  assert.ok(ui.find(node => node.props.text === '下次发送'));
  await ui.invoke(ui.find(node => node.props.key === 'rules').props.onClick);
  await ui.invoke(ui.find(node => node.props.key === 'chat-1').props.onClick);
  await ui.invoke(ui.find(node => node.props.label === '固定间隔（1～10080 分钟）').props.onValueChange, '2');
  await ui.invoke(ui.find(node => node.props.text === '保存规则').props.onClick);
  assert.equal(h.state().rules[0].intervalMinutes, 2);
  assert.equal(h.state().jobs.filter(job => job.status === 'scheduled').length, 2);
  assert.equal(count(h, 'cancel'), 2);
  await ui.invoke(ui.find(node => node.props.key === 'chat-1').props.onClick);
  await ui.invoke(ui.find(node => node.props.text === '删除规则').props.onClick);
  await ui.invoke(ui.find(node => node.type === 'AlertDialog').props.onConfirm);
  assert.equal(h.state().rules.length, 0); assert.ok(h.state().jobs.every(job => job.status === 'cancelled'));
  assert.ok(ui.find(node => node.props.text === '还没有规则。返回首页添加第一个对话。'));
});

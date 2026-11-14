import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';

const source = readFileSync(new URL('../../apps/flutter/app/lib/ui/features/packages/screens/ToolPkgComposeDslWebViewBridgeRuntime.dart', import.meta.url), 'utf8');
const descriptors = { ChanzhiHost: ['loadData', 'saveData'] };

/** Extracts the production JavaScript with the same serialized host descriptors. */
function runtime(interfaces = descriptors) {
  const match = source.match(/return '''([\s\S]*?)''';/);
  assert.ok(match);
  return match[1]
    .replaceAll('$hiddenBridgeNameJson', JSON.stringify('__ComposeDslWebViewHostBridge__'))
    .replaceAll('$channelNameJson', JSON.stringify('__ComposeDslWebViewHostBridgeChannel__'))
    .replaceAll('$interfacesJson', JSON.stringify(interfaces));
}

/** Emulates asynchronous host delivery without assuming interface discovery is immediate. */
function page({ channelReady = true } = {}) {
  const messages = [];
  const events = [];
  let now = 1000;
  const channel = { postMessage(raw) { messages.push(JSON.parse(raw)); } };
  const context = vm.createContext({
    Date: { now() { return now; } },
    Event: class { /** Records the event name exposed to page listeners. */ constructor(type) { this.type = type; } },
    dispatchEvent(event) { events.push(event.type); },
    __ComposeDslWebViewHostBridgeChannel__: channelReady ? channel : undefined,
  });
  context.window = context;
  return {
    context, messages, events, channel,
    advanceClock(milliseconds) {
      now += milliseconds;
    },
  };
}

/** Delivers one successful native response to the production request promise. */
function respond(context, request, data) {
  context.__operitComposeDslWebViewHostReceive({ id: request.id, success: true, data });
}

test('data interfaces exist before the first page script and do not await discovery', async () => {
  const { context, messages, events } = page();
  vm.runInContext(runtime(), context);
  assert.equal(typeof context.ChanzhiHost.loadData, 'function');
  assert.deepEqual(messages, []);
  assert.deepEqual(events, ['operitComposeDslInterfacesReady']);
  const pending = context.ChanzhiHost.loadData();
  assert.equal(messages[0].type, 'invoke');
  assert.equal(messages[0].payload.interfaceName, 'ChanzhiHost');
  assert.equal(messages[0].payload.methodName, 'loadData');
  respond(context, messages[0], { success: true, data: { chanzhi_todos_v1: [{ id: 'existing' }] } });
  assert.equal((await pending).data.chanzhi_todos_v1[0].id, 'existing');
});

test('reinstalling at page completion preserves outstanding read promises and request IDs', async () => {
  const { context, messages } = page();
  vm.runInContext(runtime(), context);
  const pending = context.ChanzhiHost.loadData();
  const receiver = context.__operitComposeDslWebViewHostReceive;
  vm.runInContext(runtime(), context);
  assert.equal(context.__operitComposeDslWebViewHostReceive, receiver);
  respond(context, messages[0], { success: true, data: { chanzhi_todos_v1: ['retained'] } });
  assert.deepEqual(JSON.parse(JSON.stringify(await pending)), { success: true, data: { chanzhi_todos_v1: ['retained'] } });
  const saved = context.ChanzhiHost.saveData({ chanzhi_todos_v1: ['retained'] });
  assert.notEqual(messages[0].id, messages[1].id);
  assert.deepEqual(JSON.parse(messages[1].payload.args), [{ chanzhi_todos_v1: ['retained'] }]);
  respond(context, messages[1], { success: true });
  await saved;
});

test('descriptor refresh adds and removes methods without replacing the host transport', () => {
  const { context } = page();
  vm.runInContext(runtime(), context);
  const receiver = context.__operitComposeDslWebViewHostReceive;
  vm.runInContext(runtime({ ChanzhiHost: ['loadData'], OtherHost: ['ping'] }), context);
  assert.equal(typeof context.ChanzhiHost.loadData, 'function');
  assert.equal(context.ChanzhiHost.saveData, undefined);
  assert.equal(typeof context.OtherHost.ping, 'function');
  vm.runInContext(runtime({}), context);
  assert.equal(context.ChanzhiHost, undefined);
  assert.equal(context.OtherHost, undefined);
  assert.equal(context.__operitComposeDslWebViewHostReceive, receiver);
});

test('calls made before the native channel exists drain once in order on host refresh', async () => {
  const pageState = page({ channelReady: false });
  const { context, messages, channel } = pageState;
  vm.runInContext(runtime(), context);
  const first = context.ChanzhiHost.loadData();
  const second = context.ChanzhiHost.saveData({ keep: true });
  assert.equal(messages.length, 0);
  vm.runInContext(runtime(), context);
  assert.equal(messages.length, 0);
  context.__ComposeDslWebViewHostBridgeChannel__ = channel;
  context.__operitInstallComposeDslJavascriptInterfaces(descriptors);
  assert.deepEqual(messages.map(message => message.payload.methodName), ['loadData', 'saveData']);
  messages.forEach((message, index) => respond(context, message, index));
  assert.deepEqual(await Promise.all([first, second]), [0, 1]);
  context.__operitInstallComposeDslJavascriptInterfaces(descriptors);
  assert.equal(messages.length, 2);
});

test('queued calls have no bridge deadline and resume on host readiness without polling', async () => {
  const { context, messages, channel, advanceClock } = page({ channelReady: false });
  vm.runInContext(runtime(), context);
  let settled = false;
  const pending = context.ChanzhiHost.loadData();
  pending.then(() => { settled = true; }, () => { settled = true; });
  advanceClock(3600000);
  await Promise.resolve();
  assert.equal(settled, false);
  assert.equal(messages.length, 0);
  assert.doesNotMatch(runtime(), /\b(?:setTimeout|setInterval|channelDeadline|channelPoll)\b/);
  context.__ComposeDslWebViewHostBridgeChannel__ = channel;
  vm.runInContext(runtime(), context);
  assert.equal(messages.length, 1);
  respond(context, messages[0], 'retained');
  assert.equal(await pending, 'retained');
});

test('transport errors reject their request without blocking subsequent calls', async () => {
  const { context, messages, channel } = page();
  vm.runInContext(runtime(), context);
  context.__ComposeDslWebViewHostBridgeChannel__ = { postMessage() { throw new Error('channel closed'); } };
  await assert.rejects(context.ChanzhiHost.loadData(), /channel closed/);
  context.__ComposeDslWebViewHostBridgeChannel__ = channel;
  const next = context.ChanzhiHost.loadData();
  respond(context, messages[0], 'ok');
  assert.equal(await next, 'ok');
});

test('startup reads saved todos on each entry instead of writing empty initialization', async () => {
  let file = { chanzhi_todos_v1: [{ id: 'user-todo', text: 'keep this todo' }] };
  for (let entry = 0; entry < 2; entry += 1) {
    const { context, messages } = page();
    vm.runInContext(runtime(), context);
    vm.runInContext(`
      var STORE = {};
      var startup = Promise.resolve(ChanzhiHost.loadData()).then(function(result) {
        STORE = result.data;
        return ChanzhiHost.saveData(STORE);
      });
    `, context);
    assert.equal(messages[0].payload.methodName, 'loadData');
    respond(context, messages[0], { success: true, data: structuredClone(file) });
    await Promise.resolve();
    await Promise.resolve();
    assert.equal(messages[1].payload.methodName, 'saveData');
    file = JSON.parse(messages[1].payload.args)[0];
    assert.equal(file.chanzhi_todos_v1[0].id, 'user-todo');
    respond(context, messages[1], { success: true });
    await context.startup;
  }
});

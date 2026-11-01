import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';

const root = new URL('../../', import.meta.url);
const packageRoot = new URL('plugins/packages/buildin/goal_mode/dist/', root);

/** Loads the shipped plugin and real storage facade over an isolated in-memory host. */
function runtime(records = new Map(), decision = 'completed') {
  const modules = new Map(), handlers = new Map(), reviews = [], messages = [];
  let version = 0;
  const key = (collection, id) => JSON.stringify([collection, id]);
  const context = vm.createContext({
    console, Icons: { Assignment: 'assignment' },
    ToolPkg: {
      route: () => method => method,
      getSpaceDataDir: () => 'runtime/space/plugins/goal_mode',
      ipc: { on: (name, handler) => handlers.set(name, handler) },
    },
    Tools: {
      Chat: {
        async call(request) {
          reviews.push(request);
          return { text: JSON.stringify({ decision, reason: 'Reviewed the result', missing: decision === 'incomplete' ? 'Run the final check' : '' }) };
        },
        async sendMessage(message) { messages.push(message); },
      },
      System: { async toast(message) { throw new Error(message); } },
      Storage: { async request(request) {
        switch (request.op) {
          case 'open': return { op: 'open', handle: 'goals', path: request.path };
          case 'get': return { op: 'get', entry: records.get(key(request.collection, request.key)) ?? null };
          case 'commit': {
            const committedVersion = String(++version);
            for (const mutation of request.mutations) {
              const identity = key(mutation.collection, mutation.key);
              if (mutation.deleted) records.delete(identity);
              else records.set(identity, { value: mutation.value, version: committedVersion });
            }
            return { op: 'commit', version: committedVersion };
          }
          default: throw new Error(`Unexpected storage request: ${request.op}`);
        }
      } },
    },
    __operitExpose(name, value) { context[name] = value; },
  });
  vm.runInContext(readFileSync(new URL('core/crates/plugin/sdk/src/js_sdk/storage_runtime.js', root), 'utf8'), context);
  vm.runInContext('__operitInstallStorageFacade()', context);
  function load(url) {
    if (modules.has(url.href)) return modules.get(url.href).exports;
    const module = { exports: {} };
    modules.set(url.href, module);
    vm.runInContext(`(function(module, exports, require) {${readFileSync(url, 'utf8')}\n})`, context)(
      module, module.exports, name => load(new URL(name, url)),
    );
    return module.exports;
  }
  return {
    plugin: load(new URL('plugin/goal_mode_plugin.js', packageRoot)),
    service: load(new URL('shared/goal_mode_ipc.js', packageRoot)).GoalService,
    panel: load(new URL('ui/goal_panel/index.ui.js', packageRoot)).default,
    records, reviews, messages,
  };
}

const chatId = 'goal-chat';
const input = (eventName, text) => ({ eventName, eventPayload: { chatId, text } });
const menu = (action, toggleId) => ({ eventPayload: { chatId, action, toggleId } });

/** Renders the shipped panel with a snapshot supplied by its goal subscription. */
function renderPanel(panel, goal) {
  const UI = new Proxy({}, { get: (_, type) => (props, children) => ({ type, props, children }) });
  return panel({
    UI,
    useState: (key, fallback) => [key === 'chatId' ? chatId : key === 'goal' ? goal : fallback, () => {}],
    useRef: (_, value) => ({ current: value }),
    useEffect() {},
  });
}

test('armed input creates the durable goal on submitted, injects supervision and exposes composer controls', async () => {
  const { plugin, service, panel } = runtime();
  assert.equal(await service.read(chatId), null);
  assert.ok(!JSON.stringify(renderPanel(panel, null)).includes('goal.composer'));
  await plugin.onInputMenuToggle(menu('toggle', 'goal_mode_input_slot'));
  assert.equal((await plugin.onInputMenuToggle(menu('create'))).toggles[0].isChecked, true);
  assert.equal(await plugin.onChatInput(input('submit_requested', 'Verify the release')), null);
  assert.equal(await service.read(chatId), null, 'preflight must not create a goal before acceptance');
  await plugin.onChatInput(input('submitted', 'Verify the release'));
  const goal = await service.read(chatId);
  assert.equal(goal.objective, 'Verify the release');
  assert.equal(goal.status, 'active');
  assert.equal(await service.inputEnabled(chatId), false);
  assert.equal((await plugin.onInputMenuToggle(menu('create'))).toggles[0].isChecked, false);
  const result = await plugin.onPromptFinalize({
    eventName: 'before_send_to_model',
    eventPayload: { chatId, useEnglish: true, preparedHistory: [{ kind: 'SYSTEM', content: 'Existing instructions' }] },
  });
  assert.match(result.preparedHistory[0].content, /Existing instructions[\s\S]*Goal: Verify the release/);
  const ui = JSON.stringify(renderPanel(panel, goal));
  for (const value of ['goal.composer', 'goal.pause', 'goal.clear', 'Verify the release']) assert.ok(ui.includes(value));
  await plugin.onChatInput(input('submitted', 'An ordinary follow-up'));
  assert.equal((await service.read(chatId)).objective, 'Verify the release');
});

test('invalid preflight keeps the input armed and leaves the goal absent', async () => {
  const { plugin, service } = runtime();
  await plugin.onInputMenuToggle(menu('toggle', 'goal_mode_input_slot'));
  for (const text of ['', 'x'.repeat(4001)]) {
    assert.equal((await plugin.onChatInput(input('submit_requested', text))).action, 'Block');
    assert.equal(await service.inputEnabled(chatId), true);
    assert.equal(await service.read(chatId), null);
  }
});

test('accepted goals survive runtime reload and completed replies are reviewed once', async () => {
  const first = runtime();
  await first.service.setInput(chatId, true);
  await first.plugin.onChatInput(input('submitted', 'Check the result'));
  const { plugin, service, reviews, messages, panel } = runtime(first.records);
  assert.equal((await service.read(chatId)).objective, 'Check the result');
  const event = { eventName: 'message_persisted', eventPayload: {
    chatId, sender: 'ai', timestamp: 123, completedAt: 456, content: 'Checked the actual result.',
  } };
  await plugin.onChatMessagePersisted(event);
  await plugin.onChatMessagePersisted(event);
  assert.equal(reviews.length, 1);
  assert.match(reviews[0].turns[1].content, /Goal: Check the result/);
  const goal = await service.read(chatId);
  assert.equal(goal.status, 'completed');
  assert.equal(goal.lastReviewedMessageTimestamp, 123);
  assert.ok(JSON.stringify(renderPanel(panel, goal)).includes('checkCircle'));
  assert.equal(messages.length, 0);
});

test('an incomplete review keeps the goal active and requests the actual missing work', async () => {
  const { plugin, service, messages } = runtime(new Map(), 'incomplete');
  await service.set(chatId, 'Check the release');
  await plugin.onChatMessagePersisted({ eventName: 'message_persisted', eventPayload: {
    chatId, sender: 'ai', timestamp: 321, completedAt: 654, content: 'Prepared a draft.',
  } });
  assert.equal((await service.read(chatId)).status, 'active');
  assert.equal(messages.length, 1);
  assert.equal(messages[0].chatId, chatId);
  assert.match(messages[0].input.text, /Run the final check/);
});

test('pause, resume and clear update the panel and supervision together', async () => {
  const { plugin, service, panel } = runtime();
  await service.set(chatId, 'Check the release');
  await service.status(chatId, 'paused');
  assert.ok(JSON.stringify(renderPanel(panel, await service.read(chatId))).includes('goal.resume'));
  const event = { eventName: 'before_send_to_model', eventPayload: { chatId, preparedHistory: [] } };
  assert.equal(await plugin.onPromptFinalize(event), null);
  await service.status(chatId, 'active');
  assert.equal((await plugin.onPromptFinalize(event)).preparedHistory.length, 1);
  await service.clear(chatId);
  assert.equal(await service.read(chatId), null);
  assert.ok(!JSON.stringify(renderPanel(panel, null)).includes('goal.composer'));
});

import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';

const root = new URL('../../', import.meta.url);
const packageRoot = new URL('plugins/packages/buildin/plan_mode/dist/', root);

/** Runs the packaged plugin, SDK storage facade and crypto bridge over a bounded storage host. */
function runtime() {
  const records = new Map(), handlers = new Map(), modules = new Map();
  const messages = [], toasts = [], watchKeys = [];
  let version = 0, failPlanWrite = false;
  const recordKey = (collection, key) => JSON.stringify([collection, key]);
  const entry = (collection, key) => records.get(recordKey(collection, key)) ?? null;
  const context = vm.createContext({
    console: { ...console, verbose() {} }, Icons: { Assignment: 'assignment' }, getLang: () => 'en',
    ToolPkg: {
      getSpaceDataDir: () => 'runtime/space/plugins/plan_mode',
      route: () => method => method,
      ipc: {
        on: (channel, handler) => handlers.set(channel, handler),
        call: async (channel, payload) => {
          assert.ok(handlers.has(channel), `Missing IPC endpoint: ${channel}`);
          return handlers.get(channel)(payload);
        },
      },
    },
    Tools: {
      Chat: { async sendMessage(message) { messages.push(message); } },
      System: { async toast(message) { toasts.push(message); } },
      Storage: { async request(request) {
        switch (request.op) {
          case 'open': return { op: 'open', handle: 'plans', path: request.path };
          case 'close': return { op: 'close' };
          case 'get': return { op: 'get', entry: entry(request.collection, request.key) };
          case 'commit': {
            const seen = new Set();
            for (const mutation of request.mutations) {
              const identity = recordKey(mutation.collection, mutation.key);
              // Match commitValues in PluginStorage.rs, including UTF-8 byte and C1 control limits.
              if (!mutation.key || Buffer.byteLength(mutation.key, 'utf8') > 1024 ||
                  /[\u0000-\u001f\u007f-\u009f]/u.test(mutation.key) || seen.has(identity)) {
                throw new Error('Invalid or duplicated mutation key');
              }
              seen.add(identity);
              if (mutation.checkVersion && (entry(mutation.collection, mutation.key)?.version ?? null) !== mutation.expectedVersion) {
                throw new Error('Storage version conflict');
              }
            }
            if (failPlanWrite && request.mutations.some(mutation => mutation.collection === 'plans')) {
              failPlanWrite = false;
              throw new Error('Plan write failed');
            }
            const committedVersion = String(++version);
            for (const mutation of request.mutations) {
              const identity = recordKey(mutation.collection, mutation.key);
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
    __operitNativeCrypto(algorithm, operation, args) {
      assert.equal(algorithm, 'md5');
      assert.equal(operation, 'hash');
      return createHash(algorithm).update(args[0], 'utf8').digest('hex');
    },
    __operitInvokeHostAsync: (binding, args) => binding(...args),
    async __operitNativeWaitPluginStorageRecord(handle, collection, key) {
      watchKeys.push(key);
      return entry(collection, key);
    },
  });
  for (const path of [
    'core/crates/plugin/sdk/src/js_sdk/storage_runtime.js',
    'core/crates/plugin/javascript-bridge/src/javascript/CryptoJS.script.js',
  ]) vm.runInContext(readFileSync(new URL(path, root), 'utf8'), context);
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
  load(new URL('plugin/plan_mode_plugin.js', packageRoot));
  const handoff = load(new URL('shared/plan_mode_handoff.js', packageRoot));
  const state = load(new URL('shared/plan_mode_state.js', packageRoot));
  const modes = load(new URL('shared/plan_mode_mode.js', packageRoot));
  return {
    handoff, modes, records, messages, toasts, watchKeys, entry,
    failNextPlanWrite() { failPlanWrite = true; },
    start: (chatId, planContent) => handlers.get('plan_mode.start_implementation')({ chatId, planContent }),
    async track(chatId) {
      await state.upsertTrackedChatViewAsync({ chatId, viewId: 'view', runtime: 'main', title: 'Test', updatedAt: 1 });
      await modes.enablePlanModeForChat(chatId);
    },
  };
}

for (const [name, content] of [
  ['multiline Chinese plan', '# 预期产出\n一条对话回复。\n\n## 后续可选项\n检查工作区。'],
  ['long single-line plan', '实施步骤'.repeat(500)],
  ['plan with C1 controls', '检查\u0085执行'],
]) {
  test(`${name} starts through real IPC, persists the plan and prevents duplicate sends`, async () => {
    const f = runtime(), chatId = 'chat-1';
    await f.track(chatId);
    assert.equal((await f.start(chatId, content)).success, true);
    assert.equal(f.entry('plans', chatId).value.content, content + '\n');
    assert.equal(await f.modes.isPlanModeEnabledForChat(chatId), false);
    assert.equal(await f.handoff.isPlanStarted(chatId, content), true);
    assert.equal(f.messages.length, 1);
    assert.equal(f.messages[0].chatId, chatId);
    assert.equal((await f.start(chatId, content)).alreadyStarted, true);
    assert.equal(f.messages.length, 1);
    assert.ok(f.toasts.every(message => !message.includes('Failed')));
  });
}

test('handoff identity normalizes line endings and separates chats and different plans', async () => {
  const f = runtime(), plan = '步骤一\n步骤二';
  assert.equal(await f.handoff.claimPlanStart('chat-1', plan), true);
  assert.equal(await f.handoff.isPlanStarted('chat-1', '  步骤一\r\n步骤二\n'), true);
  assert.equal(await f.handoff.claimPlanStart('chat-1', '步骤一\r\n步骤二'), false);
  assert.equal(await f.handoff.isPlanStarted('chat-2', plan), false);
  assert.equal(await f.handoff.claimPlanStart('chat-2', plan), true);
  assert.equal(await f.handoff.claimPlanStart('chat-1', plan + '修改'), true);
  assert.equal(await f.handoff.claimPlanStart('chat-1', ' \n '), false);
});

test('existing valid handoff keys remain readable at the UTF-8 byte limit', async () => {
  const f = runtime(), chatId = 'c';
  for (const plan of ['legacy plan', '短计划', '中'.repeat(339) + '😀', 'a'.repeat(1021)]) {
    const key = '1:c' + plan;
    assert.ok(Buffer.byteLength(key, 'utf8') <= 1024);
    f.records.set(JSON.stringify(['startedPlans', key]), { value: { chatId, content: plan }, version: 'old' });
    assert.equal(await f.handoff.isPlanStarted(chatId, plan), true);
    assert.equal(await f.handoff.claimPlanStart(chatId, plan), false);
    await f.handoff.forgetPlanStart(chatId, plan);
    assert.equal(f.entry('startedPlans', key), null);
  }
  for (const plan of ['a'.repeat(1022), '中'.repeat(340) + '😀']) {
    assert.equal(await f.handoff.claimPlanStart(chatId, plan), true);
    assert.equal(await f.handoff.isPlanStarted(chatId, plan), true);
  }
});

test('watch, claim and rollback use the same identity for multiline plans', async () => {
  const f = runtime(), chatId = 'chat-1', plan = '步骤一\n步骤二';
  const controller = new AbortController();
  const watch = f.handoff.watchPlanStarted(chatId, plan, controller.signal);
  assert.equal((await watch.next()).value, false);
  await f.handoff.claimPlanStart(chatId, plan);
  assert.equal((await watch.next()).value, true);
  await f.handoff.forgetPlanStart(chatId, plan);
  assert.equal((await watch.next()).value, false);
  assert.equal(new Set(f.watchKeys).size, 1);
  await watch.return();
  controller.abort();
});

test('failed persistence clears the claim so implementation can be retried', async () => {
  const f = runtime(), chatId = 'chat-1', plan = '步骤一\n步骤二';
  await f.track(chatId);
  f.failNextPlanWrite();
  assert.equal((await f.start(chatId, plan)).success, false);
  assert.equal(await f.handoff.isPlanStarted(chatId, plan), false);
  assert.equal(await f.modes.isPlanModeEnabledForChat(chatId), true);
  assert.equal(f.entry('plans', chatId), null);
  assert.equal(f.messages.length, 0);
  assert.equal((await f.start(chatId, plan)).success, true);
  assert.equal(f.messages.length, 1);
});

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { test } from 'node:test';
import { runInNewContext } from 'node:vm';

const require = createRequire(import.meta.url);
const ts = require('../../../../../.ci-tools/typescript/node_modules/typescript');
const module = { exports: {} };
runInNewContext(ts.transpileModule(readFileSync(new URL('../src/service.ts', import.meta.url), 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
}).outputText, { module, exports: module.exports, console });
const { WatchService } = module.exports;
const target = { nodeId: 'watch-1', label: 'Watch' };

/** Supplies observable device writes and controllable completion time. */
function fixture(snapshot = [], initialTarget = target) {
  const calls = [], timers = [];
  let selected = initialTarget, failure = false;
  const service = new WatchService({
    async load() { return selected; },
    async save(next) { selected = next; },
    async snapshot() { return snapshot; },
    async approvals() { return []; },
    async send(device, mood) { if (failure) throw Error('disconnected'); calls.push({ node: device.nodeId, mood }); },
    schedule(work, delay) { const timer = { work, delay, cancelled: false }; timers.push(timer); return () => { timer.cancelled = true; }; },
  });
  return { service, calls, timers, failure(value) { failure = value; }, latest() { return calls.at(-1)?.mood; } };
}

/** Creates a real typed transition with an authoritative active-chat snapshot. */
function event(chatId, state, activeChatIds = []) {
  return { chatId, state, isActive: activeChatIds.includes(chatId), activeChatIds };
}

test('restores live work, deduplicates frames, and completes once before idle', async () => {
  const f = fixture([{ chatId: 'a', state: 'executing_tool', active: true }]);
  await f.service.lifecycle('application_on_create'); assert.equal(f.latest(), 'working');
  await f.service.runtime(event('a', 'executing_tool', ['a'])); assert.equal(f.calls.length, 1);
  await f.service.runtime(event('a', 'processing', ['a'])); assert.equal(f.latest(), 'thinking');
  await f.service.runtime(event('a', 'completed')); assert.equal(f.latest(), 'completed');
  assert.equal(f.timers[0].delay, 3000);
  await f.timers[0].work(); await f.service.selected(null); assert.equal(f.latest(), 'idle');
});

test('another running task prevents celebration and cancellation never celebrates', async () => {
  const f = fixture();
  await f.service.runtime(event('a', 'processing', ['a', 'b']));
  await f.service.runtime(event('a', 'completed', ['b'])); assert.equal(f.latest(), 'thinking');
  assert.equal(f.timers.length, 0);
  await f.service.runtime(event('b', 'cancelled')); assert.equal(f.latest(), 'idle');
  assert(!f.calls.some(call => call.mood === 'completed'));
});

test('approval and error priority resumes actual work after resolution', async () => {
  const f = fixture();
  await f.service.runtime(event('a', 'executing_tool', ['a']));
  await f.service.runtime(event('b', 'error', ['a'])); assert.equal(f.latest(), 'error');
  await f.service.permissions([{ chatId: 'a', requestId: 'p' }]); assert.equal(f.latest(), 'approval');
  await f.service.permissions([]); assert.equal(f.latest(), 'error');
  await f.service.runtime(event('b', 'idle', ['a'])); assert.equal(f.latest(), 'working');
  assert(f.calls.every(call => Object.keys(call).join(',') === 'node,mood'));
});

test('new work cancels completion and shutdown rejects late hooks', async () => {
  const f = fixture();
  await f.service.runtime(event('a', 'completed'));
  await f.service.runtime(event('b', 'processing', ['b'])); assert.equal(f.timers[0].cancelled, true);
  await f.service.lifecycle('application_on_terminate'); assert.equal(f.latest(), 'idle');
  const count = f.calls.length;
  await f.service.runtime(event('b', 'executing_tool', ['b']));
  await f.service.permissions([{ chatId: 'b', requestId: 'p' }]); assert.equal(f.calls.length, count);
});

test('reports a device failure and resynchronizes on foreground or selection', async () => {
  const f = fixture(); f.failure(true);
  await assert.rejects(f.service.runtime(event('a', 'processing', ['a'])), /disconnected/);
  f.failure(false);
  await f.service.lifecycle('application_on_foreground'); assert.equal(f.latest(), 'thinking');
  await f.service.select({ nodeId: 'watch-2', label: 'Second' });
  assert.equal(f.calls.at(-1).node, 'watch-2'); assert.equal(f.latest(), 'thinking');
});

test('waits for selection, migrates the previous watch once, and keeps the newer choice', async () => {
  const f = fixture([], null);
  await f.service.runtime(event('a', 'processing', ['a'])); assert.equal(f.calls.length, 0);
  assert.equal((await f.service.selected(target)).nodeId, target.nodeId); assert.equal(f.calls.length, 0);
  await f.service.select(target); assert.equal(f.latest(), 'thinking');
  await f.service.select({ nodeId: 'watch-2', label: 'Second' });
  assert.equal((await f.service.selected(target)).nodeId, 'watch-2');
  assert.equal(f.calls.at(-1).node, 'watch-2');
});

test('clearing an unavailable selection stops remote task writes', async () => {
  const f = fixture();
  await f.service.runtime(event('a', 'processing', ['a']));
  await f.service.select(null);
  const count = f.calls.length;
  await f.service.runtime(event('a', 'executing_tool', ['a']));
  assert.equal(f.calls.length, count); assert.equal(await f.service.selected(null), null);
});

test('restart reads fresh state instead of reviving a terminated task', async () => {
  const snapshot = [{ chatId: 'a', state: 'processing', active: true }];
  const f = fixture(snapshot);
  await f.service.lifecycle('application_on_create'); assert.equal(f.latest(), 'thinking');
  await f.service.lifecycle('application_on_terminate');
  snapshot.length = 0;
  await f.service.lifecycle('application_on_create'); assert.equal(f.latest(), 'idle');
});

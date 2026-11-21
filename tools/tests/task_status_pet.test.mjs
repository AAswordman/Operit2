import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync, readdirSync } from 'node:fs';
import test from 'node:test';

const require = createRequire(import.meta.url);
const { PetService } = require('../../plugins/packages/external/task_status_pet/dist/service.js');
const { DEFAULTS, selectView } = require('../../plugins/packages/external/task_status_pet/dist/model.js');

/** Creates an isolated main owner with observable host calls and deterministic completion time. */
function fixture(settings = {}) {
  const calls = [], decisions = [], timers = [], switches = [];
  let pending = [], saved = { ...DEFAULTS, ...settings }, storedTasks = [], resourceFailure = false;
  const port = {
    async load() { return saved; },
    async save(value) { saved = value; },
    async snapshot() { return storedTasks; },
    async approvals() { return pending; },
    async resource(mood) { if (resourceFailure) throw Error('Resource decode failed'); return `vfs:/cat/${mood}.webp`; },
    async capabilities() { return { scope: 'system', permissionGranted: true }; },
    async set(id, content) { calls.push({ id, content }); return content ? 'shown' : 'closed'; },
    async respond(chatId, requestId, decision) {
      const request = pending.find(item => item.requestId === requestId && item.chatId === chatId);
      if (!request) throw Error('Stale approval');
      decisions.push({ chatId, requestId, decision }); pending = pending.filter(item => item !== request);
    },
    async switchChat(id) { switches.push(id); },
    schedule(work) { const timer = { work, cancelled: false }; timers.push(timer); return () => { timer.cancelled = true; }; },
  };
  return { service: new PetService(port), calls, decisions, timers, switches, port,
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

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
import test from 'node:test';

/** Runs the production browser transport with a controllable maintenance worker. */
function transport() {
  const workers = [];
  const globals = {};
  const stored = new Map();
  const localStorage = {
    get length() { return stored.size; },
    /** Returns a browser storage key by its index. */
    key(index) { return [...stored.keys()][index]; },
    /** Reads one explicit browser value. */
    getItem(key) { return stored.get(key) ?? null; },
    /** Persists one explicit browser value. */
    setItem(key, value) { stored.set(key, value); },
    /** Removes one browser value during publication. */
    removeItem(key) { stored.delete(key); },
  };
  class ControlledWorker {
    requests = [];
    terminated = false;
    /** Records the worker created by the real transport. */
    constructor() { workers.push(this); }
    /** Retains a request until the test explicitly completes its storage work. */
    postMessage(message) { this.requests.push(message); }
    /** Delivers a progress event while the original response is still pending. */
    progress(id, value) { this.onmessage({ data: { id, progress: value } }); }
    /** Completes one operation with the real worker response shape. */
    respond(id, value) { this.onmessage({ data: { id, value } }); }
    /** Records explicit worker shutdown. */
    terminate() { this.terminated = true; }
  }
  const source = readFileSync(new URL('../../apps/flutter/app/web/runtime/src/operit_data_rescue.ts', import.meta.url), 'utf8');
  const code = stripTypeScriptTypes(source.replaceAll('export function', 'function').replaceAll('import.meta.url', 'moduleUrl'), { mode: 'transform' });
  new Function('globalThis', 'location', 'localStorage', 'Worker', 'moduleUrl', code)(
    globals, { href: 'https://operit.test/data-rescue.html' }, localStorage, ControlledWorker, 'https://operit.test/runtime/rescue.js',
  );
  return {
    workers, stored,
    /** Invokes the actual structured browser maintenance boundary. */
    async call(request) { return JSON.parse(await globals.__operitDataRescue(JSON.stringify(request))); },
  };
}

/** Allows queued task completion callbacks to finish without using time-based fake progress. */
async function flush() { await new Promise(setImmediate); }

/** Ensures synchronous worker activity cannot block progress reads or consume pending replies. */
test('browser rescue polls measured progress while a worker operation is pending', async () => {
  const host = transport();
  const started = await host.call({ operation: 'start', request: { operation: 'inspect', archiveId: 'archive' } });
  const jobId = started.value.jobId;
  const worker = host.workers[0];
  const id = worker.requests[0].id;
  assert.equal((await host.call({ operation: 'poll', jobId })).value.done, false);
  worker.progress(id, { phase: 'verifyArchive', completed: 65536, total: 131072, unit: 'bytes' });
  const active = await host.call({ operation: 'poll', jobId });
  assert.equal(active.value.done, false);
  assert.deepEqual(active.value.progress, { phase: 'verifyArchive', completed: 65536, total: 131072, unit: 'bytes' });
  assert.equal((await host.call({ operation: 'start', request: { operation: 'export' } })).ok, false);
  worker.respond(id, { ok: true, value: { files: [{ path: 'runtime/example' }] } });
  await flush();
  const finished = await host.call({ operation: 'poll', jobId });
  assert.equal(finished.value.done, true);
  assert.deepEqual(finished.value.value, { files: [{ path: 'runtime/example' }] });
  assert.equal((await host.call({ operation: 'poll', jobId })).ok, false);
});

/** Preserves a terminal worker failure and permits a later explicitly requested task. */
test('browser rescue task errors release the active job without reporting success', async () => {
  const host = transport();
  const started = await host.call({ operation: 'start', request: { operation: 'export' } });
  const worker = host.workers[0];
  worker.respond(worker.requests[0].id, { ok: false, error: 'Cannot read payload' });
  await flush();
  assert.deepEqual(await host.call({ operation: 'poll', jobId: started.value.jobId }), { ok: false, error: 'Cannot read payload' });
  assert.equal((await host.call({ operation: 'start', request: { operation: 'export' } })).ok, true);
});

/** Keeps restore completion pending until browser settings and the durable journal both finish. */
test('browser rescue includes settings publication in the active restore task', async () => {
  const host = transport();
  host.stored.set('operit2.old', 'old');
  const started = await host.call({ operation: 'start', request: { operation: 'resumeRestore' } });
  const worker = host.workers[0];
  worker.respond(worker.requests[0].id, { ok: true, value: { requiresLocalStorageCommit: true, localStorage: { 'operit2.restored': 'restored' } } });
  await flush();
  assert.equal(worker.requests[1].request.operation, 'finishPublication');
  const pending = await host.call({ operation: 'poll', jobId: started.value.jobId });
  assert.equal(pending.value.done, false);
  assert.deepEqual([...host.stored], [['operit2.restored', 'restored']]);
  assert.equal(pending.value.progress.phase, 'publishSettings');
  assert.equal(pending.value.progress.completed, pending.value.progress.total);
  worker.respond(worker.requests[1].id, { ok: true, value: { restored: true } });
  await flush();
  assert.deepEqual((await host.call({ operation: 'poll', jobId: started.value.jobId })).value.value, { restored: true });
});

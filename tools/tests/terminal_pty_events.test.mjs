import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
import { stripTypeScriptTypes } from 'node:module';

const source = fs.readFileSync(new URL('../../apps/flutter/app/web/runtime/src/operit_runtime_bridge.ts', import.meta.url), 'utf8');

/** Executes the production serial producer and binary output API with a real atomic signal. */
function fixture() {
  const session = { id: 'pty', state: 'running', exitCode: null, output: new Uint8Array(4096), outputLength: 0,
    outputWaiters: new Set(), screen: [] };
  const context = vm.createContext({ session, Atomics, Int32Array, Uint8Array,
    linuxVmSession(id) { assert.equal(id, 'pty'); return session; }, linuxVmOutputLimit: 4096 });
  const producer = source.slice(source.indexOf('  function appendLinuxVmOutput('), source.indexOf('  /** Appends one command-output byte'));
  const drain = source.slice(source.indexOf('  function readLinuxVmPty('), source.indexOf('  /** Writes raw terminal bytes'));
  const api = source.slice(source.indexOf('      armPtyOutput('), source.indexOf('      /** Writes raw terminal bytes'));
  vm.runInContext(stripTypeScriptTypes(`${producer}\n${drain}\n globalThis.api = {${api}}; globalThis.append = appendLinuxVmOutput;`), context);
  return {session, api: context.api, append: context.append};
}

test('PTY output wakes on binary producer events and drains final bytes before EOF', () => {
  const f = fixture();
  const signal = new SharedArrayBuffer(4), header = new Int32Array(signal);
  f.api.armPtyOutput('pty', signal);
  assert.equal(Atomics.load(header, 0), 0);
  f.append(f.session, Uint8Array.from([0, 255, 195, 169]));
  assert.equal(Atomics.load(header, 0), 1);
  assert.equal(f.session.outputWaiters.size, 0);
  f.session.exitCode = 7;
  assert.deepEqual(Array.from(f.api.readPtyEvent('pty')), [0, 255, 195, 169]);
  assert.equal(f.api.readPtyEvent('pty'), null);
});

test('output arriving before registration and an idle exit cannot lose wakeups', () => {
  const f = fixture();
  f.append(f.session, Uint8Array.from([42]));
  const signal = new SharedArrayBuffer(4), header = new Int32Array(signal);
  f.api.armPtyOutput('pty', signal);
  assert.equal(Atomics.load(header, 0), 1);
  assert.deepEqual(Array.from(f.api.readPtyEvent('pty')), [42]);
  Atomics.store(header, 0, 0);
  f.session.exitCode = 0;
  f.api.armPtyOutput('pty', signal);
  assert.equal(Atomics.load(header, 0), 1);
  assert.equal(f.api.readPtyEvent('pty'), null);
});

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
import test from 'node:test';
import vm from 'node:vm';

const source = readFileSync(new URL('../../apps/flutter/app/web/runtime/src/operit_runtime_bridge.ts', import.meta.url), 'utf8');
const moduleSource = source.slice(source.indexOf('    bundledResources: registerWorkerHostModule({'), source.indexOf('    fileSystem: registerWorkerHostModule({'));

/** Runs the production resource binding with an observable browser transport. */
function resourceHost({ worker = true, status = 200 } = {}) {
  const calls = [];
  const context = vm.createContext({
    URL, Uint8Array, Error,
    isRuntimeWorker: () => worker,
    registerWorkerHostModule: module => module,
    XMLHttpRequest: class {
      status = status;
      response = Uint8Array.of(7, 8, 9).buffer;
      open(...args) { calls.push(args); }
      send() { calls.push('sent'); }
    },
  });
  const code = `globalThis.host = ({${moduleSource}}).bundledResources;`
    .replaceAll('import.meta.url', JSON.stringify('https://example.org/operit/runtime/operit_runtime_bridge.js'));
  vm.runInContext(stripTypeScriptTypes(code), context);
  return { host: context.host, calls };
}

test('resource binding reads only the requested Flutter asset and preserves subpath deployments', () => {
  const { host, calls } = resourceHost();
  assert.equal(calls.length, 0);
  assert.deepEqual([...host.read('assets/operit/plugins/external/pet.toolpkg')], [7, 8, 9]);
  assert.deepEqual(calls, [
    ['GET', 'https://example.org/operit/assets/assets/operit/plugins/external/pet.toolpkg', false],
    'sent',
  ]);
});

test('missing resources remain errors and never become empty successful packages', () => {
  assert.throws(() => resourceHost({ status: 404 }).host.read('assets/operit/plugins/missing.toolpkg'), /404/);
});

test('resource binding rejects main-thread reads and invalid resource keys before transport', () => {
  assert.throws(() => resourceHost({ worker: false }).host.read('assets/package.toolpkg'), /Core worker/);
  const { host, calls } = resourceHost();
  for (const key of ['', '../secret', '/absolute', 'a/../b', 'a//b', 'a\\b', 'https:other']) {
    assert.throws(() => host.read(key), /Invalid bundled resource key/);
  }
  assert.equal(calls.length, 0);
});

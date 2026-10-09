import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';

const root = new URL('../../', import.meta.url);
const source = path => readFileSync(new URL(path, root), 'utf8');
const worker = source('apps/flutter/app/web/runtime/src/operit_runtime_worker.ts');

/** Executes the actual worker registrations without starting a browser/OPFS runtime. */
function registrations() {
  const start = worker.indexOf('const workerCoreOperationRegistrations =');
  const end = worker.indexOf('\n]);', start);
  assert.ok(start >= 0 && end > start);
  const code = worker.slice(start, end + 4).replace(
    'new Map<WorkerCoreOperation, WorkerCoreOperationRegistration>', 'new Map',
  );
  return vm.runInNewContext(`${code}\nworkerCoreOperationRegistrations`, {
    TextEncoder,
    requireBytesPayload: message => message.payload,
    requireStringPayload(message) {
      if (typeof message.payload !== 'string') throw new Error('string payload required');
      return message.payload;
    },
    postWorkerMessage() { throw new Error('unexpected watch callback'); },
  });
}

test('owner events reach the worker ABI and preserve UTF-8 responses', async () => {
  const registration = registrations().get('emitRuntimeEvent');
  assert.equal(registration.execution, 'serialized');
  const calls = [];
  const result = await registration.invoke({
    async emitRuntimeEvent(event) {
      calls.push(event);
      return JSON.stringify({ ok: true, result: '前台恢复' });
    },
  }, { payload: '{"state":"resumed"}' });
  assert.deepEqual(calls, ['{"state":"resumed"}']);
  assert.deepEqual(JSON.parse(new TextDecoder().decode(result)), { ok: true, result: '前台恢复' });
});

test('binary owner events are rejected before dispatch', async () => {
  await assert.rejects(registrations().get('emitRuntimeEvent').invoke({
    emitRuntimeEvent() { throw new Error('must not dispatch invalid input'); },
  }, { payload: new Uint8Array([1]) }), /string payload required/);
});

test('both page and worker runtime proxies expose event ingress', () => {
  const runtime = source('apps/flutter/app/web/runtime/src/operit_runtime_bridge.ts');
  assert.match(runtime, /request\("emitRuntimeEvent", eventJson\)/);
  assert.match(runtime, /return \(await bridge\(\)\)\.emitRuntimeEvent\(eventJson\)/);
  const factory = source('apps/flutter/native/operit-flutter-bridge/src/PlatformRuntimeFactory.rs');
  assert.doesNotMatch(factory, /target_arch|target_os|target_env/);
  assert.match(factory, /"ingestRuntimeEvent"/);
  const execution = source('apps/flutter/native/operit-flutter-bridge/src/PlatformRuntimeExecution.rs');
  assert.equal((execution.match(/runtimeEventRequest\(encoded\)/g) ?? []).length, 2);
  assert.match(source('apps/flutter/native/operit-flutter-bridge/src/BridgeExports.rs'),
    /pub async fn emitRuntimeEvent[\s\S]*?self\.inner\.emitRuntimeEvent\(eventJson\)\.await/);
});

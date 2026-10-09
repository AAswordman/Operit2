import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';

const source = readFileSync(new URL(
  '../../core/crates/plugin/sdk/src/JsExecutionRuntimeBridge.script.js', import.meta.url,
), 'utf8');

/** Runs the actual SDK wrapper with observable call-scoped lifecycle hooks. */
function runtime(native = {}) {
  const references = new Map();
  const activations = [];
  const context = vm.createContext({
    ...native,
    __operitCurrentCallId: 'owner',
    __operitExpose(name, value) { context[name] = value; },
    __operitRetainCallReference(id) {
      references.set(id, (references.get(id) || 0) + 1);
    },
    __operitReleaseCallReference(id) {
      references.set(id, references.get(id) - 1);
    },
    __operitActivateCall(id) {
      activations.push(id);
      context.__operitCurrentCallId = id;
    },
  });
  vm.runInContext(source, context);
  return { context, references, activations };
}

/** Flushes the deferred reference release after the public Promise completes. */
async function flush() {
  await Promise.resolve();
  await Promise.resolve();
}

test('structured SDK calls avoid JSON text, preserve data and release after continuation', async () => {
  let request;
  const { context, references, activations } = runtime({
    __operitNativeCallToolStructured(...args) {
      request = args;
      return Promise.resolve({ success: true, data: '{"literal":true}' });
    },
  });
  const output = vm.runInContext(`
    JSON.stringify = JSON.parse = function() { throw Error('JSON text used'); };
    toolCall({type:'custom', name:'echo', params:{text:'中\\n"', ['__proto__']:{safe:true}}})
  `, context);
  assert.equal(references.get('owner'), 1);
  let continuationReferences;
  const result = await output.then(value => {
    continuationReferences = references.get('owner');
    return value;
  });
  assert.equal(result, '{"literal":true}');
  assert.deepEqual(request.slice(0, 3), ['owner', 'custom', 'echo']);
  assert.equal(request[3].text, '中\n"');
  assert.equal(Object.hasOwn(request[3], '__proto__'), true);
  assert.equal(request[3].safe, undefined);
  assert.equal(continuationReferences, 1);
  await flush();
  assert.equal(references.get('owner'), 0);
  assert.deepEqual(activations, ['owner']);
  assert.equal(Object.keys(context).some(key => key.startsWith('__operit_tool_')), false);
});

test('SDK keeps legacy hosts compatible and removes callback globals', async () => {
  let callback;
  const { context, references } = runtime({
    NativeInterface: {
      callToolAsync(id, type, name, json) {
        callback = id;
        assert.deepEqual([type, name, json], ['default', 'echo', '{"x":7}']);
        queueMicrotask(() => context[id]('{"success":true,"data":{"x":7}}', false));
      },
    },
  });
  const output = await context.toolCall('echo', { x: 7 });
  assert.equal(output.x, 7);
  await flush();
  assert.equal(references.get('owner'), 0);
  assert.equal(Object.hasOwn(context, callback), false);
});

test('intermediate-result options keep using the streaming protocol', async () => {
  let callbacks;
  const intermediate = [];
  const { context, references } = runtime({
    __operitNativeCallToolStructured() { throw Error('Streaming took the structured path'); },
    NativeInterface: {
      callToolAsyncStreaming(finalId, intermediateId, type, name, json) {
        callbacks = [finalId, intermediateId];
        assert.deepEqual([type, name, json], ['default', 'echo', '{"x":7}']);
        context[intermediateId]('{"success":true,"data":"partial"}', false);
        queueMicrotask(() => context[finalId]('{"success":true,"data":"done"}', false));
      },
    },
  });
  const output = await context.toolCall('echo', { x: 7 }, {
    onIntermediateResult(value) { intermediate.push(value); },
  });
  assert.equal(output, 'done');
  assert.deepEqual(intermediate, ['partial']);
  await flush();
  assert.equal(references.get('owner'), 0);
  for (const id of callbacks) assert.equal(Object.hasOwn(context, id), false);
});

test('business failures, native rejections and synchronous conversion failures release once', async () => {
  for (const native of [
    () => Promise.resolve({ success: false, message: 'business failure', data: { reason: 1 } }),
    () => Promise.reject(new Error('native failure')),
    () => { throw new TypeError('conversion failure'); },
  ]) {
    const { context, references } = runtime({ __operitNativeCallToolStructured: native });
    await assert.rejects(context.toolCall('echo', {}), /failure/);
    await flush();
    assert.equal(references.get('owner'), 0);
  }
});

test('out-of-order structured completions reactivate their original call scopes', async () => {
  const pending = new Map();
  const { context, references, activations } = runtime({
    __operitNativeCallToolStructured(scope) {
      return new Promise(resolve => pending.set(scope, resolve));
    },
  });
  context.__operitCurrentCallId = 'first';
  const first = context.toolCall('echo', {});
  context.__operitCurrentCallId = 'second';
  const second = context.toolCall('echo', {});
  const observed = [];
  const firstResult = first.then(value => observed.push([value, context.__operitCurrentCallId]));
  const secondResult = second.then(value => observed.push([value, context.__operitCurrentCallId]));
  pending.get('second')({ success: true, data: 2 });
  await secondResult;
  pending.get('first')({ success: true, data: 1 });
  await firstResult;
  await flush();
  assert.deepEqual(observed, [[2, 'second'], [1, 'first']]);
  assert.deepEqual(activations, ['second', 'first']);
  assert.deepEqual([...references], [['first', 0], ['second', 0]]);
});

test('throwing parameter getters reject before acquiring a call reference', async () => {
  const { context, references } = runtime({
    __operitNativeCallToolStructured() { throw Error('Should not be submitted'); },
  });
  await assert.rejects(context.toolCall('echo', {
    get value() { throw new Error('getter failure'); },
  }), /getter failure/);
  assert.equal(references.size, 0);
});

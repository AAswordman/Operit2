import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';

const root = new URL('../../', import.meta.url);

/** Reads production host source instead of copying the registry implementation into tests. */
function source(path) { return readFileSync(new URL(path, root), 'utf8'); }

/** Installs the actual Web host registry and records its structured Rust submission ABI. */
function runtime() {
  const context = vm.createContext({});
  vm.runInContext(source('hosts/web/src/javascript_promises.js'), context);
  const calls = [];
  const binding = context.__operitHostPromiseRegistry.binding((...args) => calls.push(args));
  context.request = binding;
  return { context, calls, registry: context.__operitHostPromiseRegistry };
}

/** Confirms Web host requests preserve JSON value semantics without a text transport. */
test('Web host snapshot handles nested data and immutable captured intrinsics', async () => {
  const { context, calls, registry } = runtime();
  const pending = vm.runInContext(`
    JSON.stringify = JSON.parse = function() { throw new Error('JSON text used'); };
    request('scope', {
      text: '中\\n"', integer: 7, large: 1e16,
      date: new Date('2020-01-01T00:00:00Z'),
      nested: { x: true, omit: undefined }, array: [undefined, , NaN, 2],
      boxed: new Boolean(false), ['__proto__']: { polluted: true },
      custom: { toJSON: function(key) { return { key: key }; } }
    });
  `, context);
  const [id, value] = calls[0];
  assert.equal(value.text, '中\n"');
  assert.equal(value.integer, 7);
  assert.equal(value.large, 1e16);
  assert.equal(value.date, '2020-01-01T00:00:00.000Z');
  assert.deepEqual([...value.array], [null, null, null, 2]);
  assert.equal(value.boxed, false);
  assert.equal(value.custom.key, 'custom');
  assert.equal(Object.hasOwn(value.nested, 'omit'), false);
  assert.equal(Object.hasOwn(value, '__proto__'), true);
  assert.equal(value.polluted, undefined);
  const output = { success: true, data: '{"literal":true}' };
  registry.settle(id, output, false);
  assert.equal(await pending, output);
  assert.equal(Object.keys(context).some(key => /^__operit_(tool|dependency|timer)_/.test(key)), false);
});

/** Uses the same captured structured snapshot for synchronous host calls and thrown host errors. */
test('Web synchronous bindings preserve values without JSON text and reject invalid requests', () => {
  const { context, registry } = runtime();
  let submitted = 0;
  context.echo = registry.syncBinding(value => { submitted++; return value; });
  context.fail = registry.syncBinding(() => { throw new Error('host operation denied'); });
  const result = vm.runInContext(`
    JSON.parse = JSON.stringify = function() { throw Error('JSON text used'); };
    echo({literal:'{"success":false}', array:[undefined, , NaN], ['__proto__']:{safe:true}});
  `, context);
  assert.equal(result.literal, '{"success":false}');
  assert.deepEqual([...result.array], [null, null, null]);
  assert.equal(Object.hasOwn(result, '__proto__'), true);
  assert.equal(result.safe, undefined);
  assert.throws(() => context.fail(), /host operation denied/);
  assert.throws(() => context.echo(undefined), /JSON-compatible/);
  assert.throws(() => vm.runInContext('var cyclic={}; cyclic.self=cyclic; echo(cyclic);', context), /cycle/);
  assert.equal(submitted, 1);
});

/** Ensures later prototype mutation cannot rewrite host snapshots or Promise registry ownership. */
test('Web host captured intrinsics ignore mutated collection methods and inherited array setters', async () => {
  const { context, calls, registry } = runtime();
  const pending = vm.runInContext(`
    Map.prototype.get = Map.prototype.set = Map.prototype.delete = Map.prototype.forEach =
      function() { throw new Error('Mutable Map method used'); };
    Object.defineProperty(Map.prototype, 'size', { get: function() { throw new Error('Mutable Map size used'); } });
    Array.prototype.push = Array.prototype.pop = Array.prototype.indexOf = Array.prototype.concat =
      function() { throw new Error('Mutable Array method used'); };
    Object.defineProperty(Array.prototype, '0', { set: function() { throw new Error('Inherited array setter used'); } });
    request('scope', { array: [1, { value: 7 }] });
  `, context);
  assert.equal(calls[0][1].array[0], 1);
  assert.equal(calls[0][1].array[1].value, 7);
  registry.settle(calls[0][0], 'done', false);
  assert.equal(await pending, 'done');
  registry.cancel('scope');
});

/** Verifies exact-once completion and cancellation do not affect unrelated execution scopes. */
test('Web host Promise cancellation is scoped and settlements are exact once', async () => {
  const { context, calls, registry } = runtime();
  const first = context.request('first', {});
  const second = context.request('second', {});
  let cancelledCompleted = false;
  first.then(() => { cancelledCompleted = true; });
  registry.cancel('first');
  registry.settle(calls[0][0], 'cancelled', false);
  registry.settle(calls[1][0], 'second', false);
  registry.settle(calls[1][0], 'duplicate', false);
  assert.equal(await second, 'second');
  await Promise.resolve();
  assert.equal(cancelledCompleted, false);
});

/** Ensures submission exceptions release their pending slot and later requests remain usable. */
test('Web host submission failures reject without leaking Promise slots', async () => {
  const { registry } = runtime();
  const request = registry.binding(() => { throw new Error('submission failure'); });
  for (let index = 0; index < 4100; index++) {
    await assert.rejects(request('scope', { index }), /submission failure/);
  }
});

/** Rejects unsupported values, cycles, getters, excessive depth and reentrant slot exhaustion. */
test('Web host validates conversion before retaining completion handles', () => {
  const { context, calls } = runtime();
  vm.runInContext(`
    var cycle = {}; cycle.self = cycle;
    var invalid = [cycle, { bigint: 1n }, Object(1n), undefined, function() {}, Symbol('value'),
      { get value() { throw new Error('getter failure'); } }];
  `, context);
  for (const value of context.invalid) assert.throws(() => context.request('scope', value));
  assert.equal(calls.length, 0);
  let value = null;
  for (let index = 0; index < 130; index++) value = { value };
  assert.throws(() => context.request('scope', value), /depth\/node limit/);
  vm.runInContext(`
    for (var index = 0; index < 4095; index++) request('scope', {});
    var reentrant = { get value() { request('scope', {}); return 7; } };
  `, context);
  assert.throws(() => context.request('scope', context.reentrant), /Too many pending/);
  assert.equal(calls.length, 4096);
});

/** Uses production bootstrap timers with host Promise completion rather than callback globals. */
function timerRuntime() {
  const text = source('core/crates/plugin/javascript-bridge/src/javascript/JsLibraries.rs');
  const start = text.indexOf('        var console = {{');
  const end = text.indexOf('        {}', start);
  const nativeTimers = new Map();
  const callTimers = new Set();
  const failures = [];
  const context = vm.createContext({
    __operitCurrentCallId: 'owner',
    /** Records each native timer Promise under its explicit cancellation scope. */
    __operitNativeScheduleJavaScriptTimer(scope, delay) {
      return new Promise((resolve, reject) => nativeTimers.set(scope, { delay, resolve, reject }));
    },
    /** Removes the host completion handles for an explicitly cancelled timer. */
    __operitNativeCancelJavaScriptPromises(scope) { nativeTimers.delete(scope); },
    /** Tracks the owning execution's retained timers. */
    __operitRegisterCallTimer(callId, id) { assert.equal(callId, 'owner'); callTimers.add(id); },
    /** Releases exactly the owning execution's retained timer. */
    __operitUnregisterCallTimer(callId, id) { assert.equal(callId, 'owner'); callTimers.delete(id); },
    /** Reactivates the timer owner before calling plugin handlers. */
    __operitActivateCall(callId) { context.__operitCurrentCallId = callId; },
    /** Supplies the active execution error reporting contract. */
    __operitGetCallState() { return { callRuntime: { fail(error) { failures.push(error); } } }; },
  });
  context.window = context;
  vm.runInContext(text.slice(start, end).replaceAll('{{', '{').replaceAll('}}', '}'), context);
  return { context, nativeTimers, callTimers, failures };
}

/** Checks timer retain/release timing, cancellation and error propagation on the single host path. */
test('timers retain owners, clear native Promise handles and never install global callbacks', async () => {
  const { context, nativeTimers, callTimers, failures } = timerRuntime();
  const seen = [];
  const id = context.setTimeout(value => seen.push(value), 5, 'done');
  assert.equal(context[id], undefined);
  assert.equal(nativeTimers.get(id).delay, 5);
  assert.equal(callTimers.has(id), true);
  nativeTimers.get(id).resolve(null);
  await Promise.resolve();
  assert.deepEqual(seen, ['done']);
  assert.equal(callTimers.has(id), true);
  await Promise.resolve();
  assert.equal(callTimers.has(id), false);
  const cleared = context.setTimeout(() => seen.push('cancelled'), 10);
  context.clearTimeout(cleared);
  assert.equal(nativeTimers.has(cleared), false);
  assert.equal(callTimers.has(cleared), false);
  const failing = context.setTimeout(() => { throw new Error('timer failure'); }, 0);
  nativeTimers.get(failing).resolve(null);
  await Promise.resolve();
  await Promise.resolve();
  assert.equal(failures[0].message, 'timer failure');
  assert.equal(callTimers.has(failing), false);
});

/** Exercises actual session cancellation against host-backed timers owned by different calls. */
test('actual call-session cleanup cancels only its timers and suppresses queued completion', async () => {
  const { context, nativeTimers } = timerRuntime();
  vm.runInContext(source('core/crates/plugin/javascript-bridge/src/javascript/JsInitRuntime.script.js'), context);
  const seen = [];
  const firstState = context.__operitRegisterCallSession('first', {});
  context.__operitActivateCall('first');
  const first = context.setTimeout(() => seen.push('first'), 5);
  const firstCompletion = nativeTimers.get(first);
  const secondState = context.__operitRegisterCallSession('second', {});
  context.__operitActivateCall('second');
  const second = context.setTimeout(() => seen.push('second'), 5);
  assert.equal(firstState.pendingReferences, 1);
  assert.equal(secondState.pendingReferences, 1);
  context.__operitCancelCallSession('first');
  assert.equal(nativeTimers.has(first), false);
  assert.equal(nativeTimers.has(second), true);
  assert.equal(context.__operitGetCallState('first'), null);
  firstCompletion.resolve(null);
  nativeTimers.get(second).resolve(null);
  await Promise.resolve();
  await Promise.resolve();
  assert.deepEqual(seen, ['second']);
  assert.equal(secondState.pendingReferences, 0);
});

/** Releases repeating timer state along with the exact pending timeout when its session ends. */
test('session cancellation removes interval state and prevents further ticks', async () => {
  const { context, nativeTimers } = timerRuntime();
  vm.runInContext(source('core/crates/plugin/javascript-bridge/src/javascript/JsInitRuntime.script.js'), context);
  const state = context.__operitRegisterCallSession('interval-owner', {});
  context.__operitActivateCall('interval-owner');
  let ticks = 0;
  context.setInterval(() => { ticks++; }, 5);
  assert.equal(Object.keys(context.intervalStates).length, 1);
  const completion = [...nativeTimers.values()][0];
  assert.equal(state.pendingReferences, 1);
  context.__operitCancelCallSession('interval-owner');
  assert.equal(Object.keys(context.intervalStates).length, 0);
  assert.equal(nativeTimers.size, 0);
  completion.resolve(null);
  await Promise.resolve();
  assert.equal(ticks, 0);
});

/** Guards the required host contract and prevents retired bindings from being reintroduced. */
test('bridge source exposes only mandatory structured asynchronous transports', () => {
  const host = source('core/crates/foundation/host-api/src/lib.rs');
  assert.match(host, /fn callHostJavaScriptFunction\([\s\S]*?\) -> HostResult<Value>;/);
  for (const method of ['registerHostJavaScriptAsyncJsonFunction',
    'settleHostJavaScriptPromise', 'cancelHostJavaScriptPromises', 'scheduleHostRuntimeCooperativeAsyncTask']) {
    assert.match(host, new RegExp('fn ' + method + '\\([\\s\\S]*?\\) -> HostResult<\\(\\)>;'));
  }
  for (const path of [
    'core/crates/plugin/javascript-bridge/src/javascript/JsEngine.rs',
    'core/crates/plugin/javascript-bridge/src/javascript/JsLibraries.rs',
    'core/crates/plugin/sdk/src/JsExecutionRuntimeBridge.script.js',
    'core/crates/plugin/codegen/src/declarations.rs',
    'core/crates/plugin/codegen/src/runtime_bindings.rs',
    'core/crates/plugin/sdk/src/chat_runtime.js',
    'core/crates/plugin/sdk/src/js_sdk/storage_runtime.js',
    'core/crates/plugin/sdk/src/js_sdk/core.rs',
    'core/crates/plugin/sdk/src/toolpkg/ToolPkgComposeDslBridge.rs',
    'core/crates/plugin/sdk/src/toolpkg/ToolPkgRegistrationBridge.rs',
    'core/crates/plugin/javascript-bridge/src/javascript/JsHostOperations.rs',
    'core/crates/plugin/javascript-bridge/src/javascript/JsJavaBridge.rs',
    'core/crates/plugin/javascript-bridge/src/javascript/CryptoJS.script.js',
    'core/crates/plugin/javascript-bridge/src/javascript/Jimp.script.js',
    'core/crates/plugin/javascript-bridge/src/javascript/pako.script.js',
    'plugins/types/core.d.ts', 'plugins/types/index.d.ts',
  ]) {
    assert.doesNotMatch(source(path), /callToolAsync|callToolSerialized|JsAsyncCallback::Legacy|SdkCallback|JsCallbackDelivery|__operitNativeInvokeToolPkgIpcAsync|__operitParseToolResult|NativeInterface/);
  }
  for (const path of ['hosts/web/src/javascript_runtime.rs',
    'hosts/common/operit-host-native-scheduler/src/javascript_runtime.rs']) {
    assert.match(source(path), /fn registerHostJavaScriptAsyncJsonFunction\(/);
    assert.match(source(path), /fn registerHostJavaScriptJsonFunction\(/);
    assert.doesNotMatch(source(path), /registerHostJavaScriptStringFunction|registerHostJavaScriptVoidFunction/);
    assert.match(source(path), /fn settleHostJavaScriptPromise\(/);
    assert.match(source(path), /fn cancelHostJavaScriptPromises\(/);
  }
});

/** Extracts the real IPC call implementation and supplies its explicit surrounding execution contracts. */
function ipcRuntime(native) {
  const text = source('core/crates/plugin/javascript-bridge/src/javascript/JsLibraries.rs');
  const start = text.indexOf('                    ipcApi.call = function(channel, payload, options)');
  const end = text.indexOf('                    toolPkgApi.ipc = ipcApi;', start);
  let retained = 0;
  const activations = [];
  const context = vm.createContext({
    ipcApi: {}, packageTarget: 'fixture', callId: 'owner', __operitCurrentCallId: 'owner',
    /** Preserves empty routing options without guessing at transport results. */
    __operitText(value) { return value == null ? '' : String(value); },
    /** Normalizes the typed IPC channel used by the production call implementation. */
    normalizeToolPkgIpcChannel(value) { return String(value).trim(); },
    /** Returns the bound source context identity. */
    getCurrentToolPkgExecutionContextKey() { return 'ui-context'; },
    /** Ensures this fixture exercises cross-context transport rather than a local handler. */
    getCurrentToolPkgRuntimeKind() { return 'ui'; },
    /** Records the exact retained owner identity. */
    __operitRetainCallReference(id) { assert.equal(id, 'owner'); retained++; },
    /** Records the matching completion reference release. */
    __operitReleaseCallReference(id) { assert.equal(id, 'owner'); retained--; },
    /** Restores the original execution context on completion. */
    __operitActivateCall(id) { activations.push(id); },
    __operitNativeInvokeToolPkgIpc: native,
  });
  vm.runInContext(text.slice(start, end).replaceAll('{{', '{').replaceAll('}}', '}'), context);
  return { context, activations, retained: () => retained };
}

/** Confirms cross-context IPC passes structured data and preserves application-level failure values. */
test('cross-context IPC uses the required Promise ABI and preserves public continuation ownership', async () => {
  let args;
  const { context, activations, retained } = ipcRuntime((...values) => {
    args = values;
    return Promise.resolve({ success: true, value: { success: false, literal: '{"x":1}' } });
  });
  vm.runInContext(`JSON.parse=JSON.stringify=function(){throw new Error('JSON text used');};`, context);
  const payload = { value: 7 };
  const result = context.ipcApi.call('echo', payload, { targetRuntime: 'main' });
  assert.equal(retained(), 1);
  assert.deepEqual(args, ['owner', 'fixture', 'ui-context', '', 'main', 'echo', payload]);
  let continuationReferences;
  const value = await result.then(value => { continuationReferences = retained(); return value; });
  assert.deepEqual(value, { success: false, literal: '{"x":1}' });
  assert.equal(continuationReferences, 1);
  await Promise.resolve();
  assert.equal(retained(), 0);
  assert.deepEqual(activations, ['owner']);
  assert.equal(Object.keys(context).some(key => key.startsWith('__operit_toolpkg_ipc_')), false);
});

/** Checks all IPC failure classes release the exact acquired owner reference. */
test('cross-context IPC releases ownership for business, native and synchronous submission failures', async () => {
  for (const native of [
    () => Promise.resolve({ success: false, message: 'business failure' }),
    () => Promise.reject(new Error('native failure')),
    () => { throw new Error('submission failure'); },
  ]) {
    const { context, retained } = ipcRuntime(native);
    await assert.rejects(context.ipcApi.call('echo', {}, { targetRuntime: 'main' }), /failure/);
    await Promise.resolve();
    assert.equal(retained(), 0);
  }
});

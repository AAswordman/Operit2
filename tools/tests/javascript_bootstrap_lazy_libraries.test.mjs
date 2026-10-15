import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';
import { lazyBootstrapLibraries, bootstrapLibrarySource } from './fixtures/production_bootstrap.mjs';

const script = readFileSync(new URL('../../core/crates/plugin/javascript-bridge/src/javascript/JsLazyLibraries.script.js', import.meta.url), 'utf8');

/** Creates a host-owned source channel and executes the production lazy-library installer. */
function runtime(source = bootstrapLibrarySource) {
  const reads = [];
  const context = vm.createContext({
    /** Records trusted-source requests without executing the corresponding library eagerly. */
    __operitNativeReadBootstrapLibrary(name) { reads.push(name); return source(name); },
  });
  vm.runInContext('globalThis.__operitExpose = function(name, value) { globalThis[name] = value; };', context);
  vm.runInContext(lazyBootstrapLibraries(), context);
  return { context, reads };
}

/** Asserts unused libraries remain absent from compilation while grouped exports initialize once. */
test('bootstrap libraries initialize only on public access and keep exact export identities', () => {
  const { context, reads } = runtime();
  assert.deepEqual(reads, []);
  assert.equal(typeof Object.getOwnPropertyDescriptor(context, 'UINode').get, 'function');
  assert.equal(vm.runInContext('typeof UINode + ":" + typeof UINode', context), 'function:function');
  assert.deepEqual(reads, ['assets/js/UINode.js']);
  const ui = context.UINode;
  assert.equal(context.UINode, ui);
  assert.equal(vm.runInContext('typeof Android + ":" + typeof PackageManager + ":" + typeof DeviceController', context), 'function:function:function');
  assert.deepEqual(reads, ['assets/js/UINode.js', 'assets/js/AndroidUtils.js']);
  for (const name of ['CryptoJS', 'Jimp', 'OkHttp', 'pako']) assert.equal(typeof context[name], 'object');
  assert.equal(reads.length, 6);
});

/** Preserves direct global assignment without loading an unused library. */
test('bootstrap globals preserve assignment before their first access', () => {
  const { context, reads } = runtime();
  const replacement = {};
  context.CryptoJS = replacement;
  assert.equal(context.CryptoJS, replacement);
  assert.deepEqual(reads, []);
});

/** Prevents partial initialization and repeated source evaluation after a precise failure. */
test('failed bootstrap library never publishes partial exports or retries initialization', () => {
  const failure = new Error('exact initialization failure');
  const reads = [];
  const context = vm.createContext({ failure,
    /** Publishes one export before failing to exercise atomic visibility of a library group. */
    __operitNativeReadBootstrapLibrary(name) { reads.push(name); return 'globalThis.First = {}; throw failure;'; },
  });
  vm.runInContext(script + '\n__operitInstallBootstrapLibraries([["broken", ["First", "Second"]]]);', context);
  for (const name of ['First', 'Second', 'First']) assert.throws(() => context[name], error => error === failure);
  assert.deepEqual(reads, ['broken']);
});

/** Keeps even falsy thrown values terminal rather than implicitly retrying a broken library. */
test('falsy bootstrap failure remains terminal', () => {
  const reads = [];
  const context = vm.createContext({
    /** Throws the exact falsy value from the first and sole library evaluation. */
    __operitNativeReadBootstrapLibrary(name) { reads.push(name); return 'throw null;'; },
  });
  vm.runInContext(script + '\n__operitInstallBootstrapLibraries([["broken", ["Missing"]]]);', context);
  for (let index = 0; index < 2; index++) {
    let caught = false;
    try { void context.Missing; } catch (error) { caught = true; assert.equal(error, null); }
    assert.equal(caught, true);
  }
  assert.deepEqual(reads, ['broken']);
});

/** Rejects a malformed trusted library and makes that exact error terminal. */
test('bootstrap library requires every declared public export', () => {
  const context = vm.createContext({
    /** Deliberately omits a declared export to verify strict installation validation. */
    __operitNativeReadBootstrapLibrary() { return 'globalThis.First = {};'; },
  });
  vm.runInContext(script + '\n__operitInstallBootstrapLibraries([["incomplete", ["First", "Second"]]]);', context);
  let first;
  try { void context.First; } catch (error) { first = error; }
  assert.match(first.message, /did not expose Second/);
  assert.throws(() => context.Second, error => error === first);
});

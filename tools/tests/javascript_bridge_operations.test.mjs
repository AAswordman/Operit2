import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { productionBootstrap } from './fixtures/production_bootstrap.mjs';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';

const root = new URL('../../', import.meta.url);
const bridge = 'core/crates/plugin/javascript-bridge/src/javascript/';
const sdk = 'core/crates/plugin/sdk/src/';

/** Reads the production implementation rather than a copied bridge fixture. */
function source(path) { return readFileSync(new URL(path, root), 'utf8'); }

/** Extracts one explicitly delimited Rust raw JavaScript string. */
function embedded(path) {
  const text = source(path);
  const start = text.indexOf('r#"') + 3;
  return text.slice(start, text.indexOf('"#', start));
}

/** Installs the production Web host value registry under every required engine binding. */
function runtime(operations = {}) {
  const calls = [];
  const outputs = new Map();
  const failures = new Map();
  const timers = new Map();
  const context = vm.createContext({});
  vm.runInContext(source('hosts/web/src/javascript_promises.js'), context);
  const registry = context.__operitHostPromiseRegistry;
  const builtins = {
    __operitNativeHashText(value) { return createHash("sha256").update(value).digest("hex"); },
    /** Records plugin logs as structured host requests. */
    __operitNativeLog() { return null; },
    /** Records execution diagnostics without changing test output. */
    __operitNativeLogJsExecutionTrace() { return null; },
    /** Records the structured final result before execution cleanup. */
    __operitNativeSetCallResult(id, value) { outputs.set(id, value); return null; },
    /** Records the structured failure before execution cleanup. */
    __operitNativeSetCallError(id, value) { failures.set(id, value); return null; },
    /** Cancels the exact scope owned by the runtime session or timer. */
    __operitNativeCancelJavaScriptPromises(scope) { registry.cancel(scope); timers.delete(scope); return null; },
    /** Accepts the explicit detached-session lifecycle notification. */
    __operitNativeNotifyDetachedCall() { return null; },
    ...operations,
  };
  const engine = source(bridge + 'JsEngine.rs');
  const syncBlock = engine.slice(engine.indexOf('let syncNames = ['), engine.indexOf('for name in syncNames'));
  const asyncBlock = engine.slice(engine.indexOf('let asyncNames = ['), engine.indexOf('for name in asyncNames'));
  /** Executes exactly the operation supplied by this test and rejects unconfigured capabilities. */
  function invoke(name, args) {
    calls.push({ name, args });
    assert.equal(typeof builtins[name], 'function', 'Unexpected host operation: ' + name);
    return builtins[name](...args);
  }
  for (const match of syncBlock.matchAll(/"([^"]+)"/g)) {
    const name = match[1];
    context[name] = registry.syncBinding((...args) => invoke(name, args));
  }
  for (const match of asyncBlock.matchAll(/"([^"]+)"/g)) {
    const name = match[1];
    context[name] = registry.binding((id, ...args) => {
      Promise.resolve().then(() => invoke(name, args)).then(
        value => registry.settle(id, value, false),
        error => registry.settle(id, String(error.message), true),
      );
    });
  }
  context.__operitNativeCancelJavaScriptPromises = registry.syncBinding(builtins.__operitNativeCancelJavaScriptPromises);
  context.__operitNativeScheduleJavaScriptTimer = registry.binding((id, delay) => timers.set(id, delay));
  vm.runInContext(productionBootstrap(), context, { filename: 'production-bootstrap.js' });
  /** Opens a real execution session for direct public library calls. */
  function openCall(id = 'owner') {
    context.__operitRegisterCallSession(id, {
      __operit_toolpkg_api_version: '2.0.0', toolPkgId: 'fixture', __operit_ui_package_name: 'fixture',
    });
    context.__operitActivateCall(id);
  }
  return { context, calls, outputs, failures, timers, openCall };
}

/** Flushes deterministic Promise turns without advancing any safety timers. */
async function flush() { for (let turn = 0; turn < 16; turn++) await Promise.resolve(); }

/** Confirms the complete bootstrap no longer installs a native-interface object or callback delegates. */
test('production bootstrap exposes public wrappers without the removed global object', () => {
  const { context } = runtime();
  assert.equal(Object.hasOwn(context, 'NativeInterface'), false);
  for (const name of ['toolCall', 'setTimeout', 'clearTimeout', 'setInterval', 'clearInterval']) {
    assert.equal(typeof context[name], 'function');
  }
  for (const name of ['CryptoJS', 'Jimp', 'pako', 'Java', 'Kotlin', 'ToolPkg']) assert.ok(context[name]);
});

/** Confirms crypto and compression preserve application strings and throw actual host failures. */
test('crypto and deflate use direct structured values without error-string guessing', () => {
  const { context, calls } = runtime({
    /** Returns literal application data resembling the retired error-text protocol. */
    __operitNativeCrypto(algorithm, operation, args) {
      assert.equal(algorithm, 'md5'); assert.equal(operation, 'hash');
      assert.deepEqual([...args], ['input']);
      return '{"nativeError":"literal data"}';
    },
    /** Returns decompressed text without a JSON-result envelope. */
    __operitNativeDecompress(data, algorithm) {
      assert.equal(data, 'encoded'); assert.equal(algorithm, 'deflate');
      return '{"nativeError":"decompressed text"}';
    },
  });
  vm.runInContext('JSON.parse = JSON.stringify = function() { throw Error("JSON transport used"); };', context);
  assert.equal(vm.runInContext('CryptoJS.MD5("input").toString()', context), '{"nativeError":"literal data"}');
  assert.equal(vm.runInContext('pako.inflate("encoded", {to:"string"})', context), '{"nativeError":"decompressed text"}');
  assert.equal(calls.length, 2);
  const failed = runtime({
    /** Rejects invalid cryptographic input instead of returning an empty plaintext. */
    __operitNativeCrypto() { throw new Error('invalid AES key'); },
  }).context;
  assert.throws(() => vm.runInContext('CryptoJS.AES.decrypt("bad", "key").toString()', failed), /invalid AES key/);
});

/** Checks asynchronous image operations retain the session and never publish callback IDs. */
test('Jimp uses an owned host Promise and releases exactly the original execution reference', async () => {
  const { context, calls, openCall } = runtime({
    /** Returns an owned image handle or fails the exact submitted image operation. */
    __operitNativeImageProcessing(operation, args) {
      if (operation === 'read') { assert.equal(args[0], 'encoded'); return 'image-handle'; }
      throw new Error('image operation rejected');
    },
  });
  openCall();
  vm.runInContext('JSON.parse = JSON.stringify = function() { throw Error("JSON transport used"); };', context);
  const pending = context.Jimp.read('encoded');
  assert.equal(context.__operitGetCallState('owner').pendingReferences, 1);
  const image = await pending;
  assert.equal(image.id, 'image-handle');
  await flush();
  assert.equal(context.__operitGetCallState('owner').pendingReferences, 0);
  await assert.rejects(image.crop(0, 0, 10, 10), /image operation rejected/);
  await flush();
  assert.equal(context.__operitGetCallState('owner').pendingReferences, 0);
  assert.equal(calls[1].args[1][0], 'image-handle');
  assert.equal(Object.keys(context).some(key => /^_jimp_|^__operit_compose_webview_/.test(key)), false);
});

/** Confirms resource and WASM wrappers use structured Promise results, including exact host errors. */
test('ToolPkg resources and WASM pass owned arrays and never treat failures as paths', async () => {
  const { context, openCall } = runtime({
    /** Returns an absolute resource path or rejects a missing resource. */
    __operitNativeReadToolPkgResource(target, key, output, internal) {
      assert.equal(target, 'fixture'); assert.equal(output, ''); assert.equal(internal, false);
      if (key === 'missing') throw new Error('resource is missing');
      return '/runtime/resources/value.bin';
    },
    /** Verifies scalar WASM arguments arrive as an array, not serialized text. */
    __operitNativeCallToolPkgWasm(target, module, name, args) {
      assert.equal(target, 'fixture'); assert.equal(module, 'math'); assert.equal(name, 'sum');
      assert.equal(Array.isArray(args), true); assert.equal(args[0].type, 'i32');
      return 42;
    },
  });
  openCall();
  vm.runInContext('JSON.parse = JSON.stringify = function() { throw Error("JSON transport used"); };', context);
  assert.equal(await context.ToolPkg.readResource('value'), '/runtime/resources/value.bin');
  await assert.rejects(context.ToolPkg.readResource('missing'), /resource is missing/);
  assert.equal(await context.ToolPkg.wasm.call('math', 'sum', [{type: 'i32', value: 42}]), 42);
  await flush();
  assert.equal(context.__operitGetCallState('owner').pendingReferences, 0);
});

/** Runs an actual script through the bootstrap lifecycle and receives its final result as structured data. */
test('script completion uses structured host values with JSON text disabled', async () => {
  const { context, outputs, failures } = runtime();
  vm.runInContext('JSON.parse = JSON.stringify = function() { throw Error("JSON transport used"); };', context);
  context.__operitExecuteScriptFunction('script-owner', {__operit_toolpkg_api_version: '2.0.0'},
    'exports.run = function() { return { literal: "{\\\"data\\\":1}", nested: {value: 7} }; };', 'run', 60, 10000);
  await flush();
  assert.equal(failures.size, 0);
  assert.equal(outputs.get('script-owner').literal, '{"data":1}');
  assert.equal(outputs.get('script-owner').nested.value, 7);
  assert.equal(context.__operitGetCallState('script-owner'), null);
});

/** Checks result conversion failures cannot mark a session completed before reporting the error. */
test('cyclic script results fail explicitly rather than producing replacement data', async () => {
  const { context, outputs, failures } = runtime();
  context.__operitExecuteScriptFunction('cycle-owner', {__operit_toolpkg_api_version: '2.0.0'},
    'exports.run = function() { var value = {}; value.self = value; return value; };', 'run', 60, 10000);
  await flush();
  assert.equal(outputs.size, 0);
  assert.equal(failures.get('cycle-owner').success, false);
  assert.match(failures.get('cycle-owner').message, /cycle/i);
  assert.equal(context.__operitGetCallState('cycle-owner'), null);
});

/** Checks Java arguments remain arrays and successful application strings are never decoded as envelopes. */
test('Java wrappers dispatch structured values and do not probe alternative member transports', () => {
  const { context } = runtime({
    /** Returns the explicit compatibility application-context descriptor. */
    __operitNativeJavaGetApplicationContext() { return {__javaHandle: 'context', __javaClass: 'android.app.Application'}; },
    /** Preserves literal application text from a Java compatibility method. */
    __operitNativeJavaCallInstance(handle, method, args) {
      assert.equal(handle, 'context'); assert.equal(method, 'toString'); assert.equal(args.length, 0);
      return '{"success":false,"message":"literal text"}';
    },
    /** Reports a static call failure through the host exception contract. */
    __operitNativeJavaCallStatic(_class, _method, args) {
      assert.equal(Array.isArray(args), true); throw new Error('static call is unsupported');
    },
  });
  vm.runInContext('JSON.parse = JSON.stringify = function() { throw Error("JSON transport used"); };', context);
  assert.equal(vm.runInContext('Java.getApplicationContext().toString()', context), '{"success":false,"message":"literal text"}');
  assert.throws(() => vm.runInContext('Java.callStatic("Example", "run", {value:7})', context), /static call is unsupported/);
});

/** Checks Compose controller operations use typed sync/Promise bindings and preserve literal result data. */
test('Compose WebView and file picker have no callback-ID or JSON-text bridge', async () => {
  const { context, openCall } = runtime({
    /** Accepts an explicit structured controller request. */
    __operitNativeComposeWebViewControllerCommand(payload) {
      assert.equal(payload.key, 'web'); return {success: true, data: '{"literal":true}'};
    },
    /** Accepts a structured suspended command through the host Promise ABI. */
    __operitNativeComposeWebViewControllerCommandSuspend(payload) {
      assert.equal(payload.command, 'evaluateJavascript'); return {success: true, data: '{"literal":true}'};
    },
    /** Returns the file-picker's ordinary structured data contract. */
    __operitNativeComposeFilePickerCommand(payload) {
      assert.equal(payload.executionContextKey, 'ui-owner'); return {cancelled: false, files: [{path:'/picked/value'}]};
    },
  });
  openCall();
  const compose = context.OperitComposeDslRuntime.createContext({routeInstanceId:'route', executionContextKey:'ui-owner'}).ctx;
  const web = compose.createWebViewController('web');
  vm.runInContext('JSON.parse = JSON.stringify = function() { throw Error("JSON transport used"); };', context);
  assert.equal(web.getState(), '{"literal":true}');
  assert.equal(await web.evaluateJavascript('1'), '{"literal":true}');
  assert.equal((await compose.openFilePicker({picker:'document'})).files[0].path, '/picked/value');
  await flush();
  assert.equal(context.__operitGetCallState('owner').pendingReferences, 0);
  assert.equal(Object.keys(context).some(key => /^__operit_compose_webview_/.test(key)), false);
});

/** Checks package operations use only their typed host Promise contract rather than trying another entry. */
test('Compose package management uses required host bindings and preserves exact error semantics', async () => {
  const { context, calls, openCall } = runtime({
    /** Reports the package state as a boolean, not a serialized string. */
    __operitNativeIsPackageImported(packageName) { assert.equal(packageName, 'fixture'); return true; },
    /** Returns literal package-management data without interpreting envelope-looking text. */
    __operitNativeImportPackage(packageName) { assert.equal(packageName, 'fixture'); return '{"success":false}'; },
    /** Rejects a denied removal through the single required host transport. */
    __operitNativeRemovePackage() { throw new Error('package removal denied'); },
    /** Rejects a package-list failure rather than producing an empty list. */
    __operitNativeListImportedPackages() { throw new Error('package enumeration denied'); },
    /** Resolves the identity using typed fields and an exact boolean preference. */
    __operitNativeResolveToolName(packageName, subpackage, name, preferImported) {
      assert.equal(packageName, 'fixture'); assert.equal(subpackage, '');
      assert.equal(name, 'run'); assert.equal(preferImported, false);
      return 'fixture:run';
    },
  });
  openCall();
  const compose = context.OperitComposeDslRuntime.createContext({packageName:'fixture'}).ctx;
  vm.runInContext('JSON.parse = JSON.stringify = function() { throw Error("JSON transport used"); };', context);
  assert.equal(await compose.isPackageImported('fixture'), true);
  assert.equal(await compose.importPackage('fixture'), '{"success":false}');
  await assert.rejects(compose.removePackage('fixture'), /package removal denied/);
  await assert.rejects(compose.listImportedPackages(), /package enumeration denied/);
  assert.equal(await compose.resolveToolName({toolName:'run', preferImported:false}), 'fixture:run');
  await flush();
  assert.equal(context.__operitGetCallState('owner').pendingReferences, 0);
  assert.equal(calls.some(call => call.name === '__operitNativeCallToolStructured'), false);
});

/** Checks environment values stay literal and environment objects cross the host boundary directly. */
test('Compose environment access uses structured values and propagates write failures', async () => {
  const environment = {};
  const { context, openCall } = runtime({
    /** Receives the explicit owner and original string value. */
    __operitNativeSetEnv(owner, key, value) {
      assert.equal(owner, 'owner');
      if (key === 'denied') throw new Error('environment write denied');
      environment[key] = value;
    },
    /** Receives the environment object without any JSON text transport. */
    __operitNativeSetEnvs(owner, values) { assert.equal(owner, 'owner'); Object.assign(environment, values); },
  });
  openCall();
  context.__operit_call_runtime_ref = {
    /** Preserves the host environment's literal text. */
    getEnv(key) { return environment[key]; },
  };
  const compose = context.OperitComposeDslRuntime.createContext({__operit_call_runtime: context.__operit_call_runtime_ref}).ctx;
  vm.runInContext('JSON.parse = JSON.stringify = function() { throw Error("JSON transport used"); };', context);
  await compose.setEnv('literal', '{"success":false,"data":"application text"}');
  assert.equal(compose.getEnv('literal'), '{"success":false,"data":"application text"}');
  await compose.setEnvs({other:'value'});
  assert.equal(compose.getEnv('other'), 'value');
  await assert.rejects(compose.setEnv('denied', 'value'), /environment write denied/);
});

/** Checks CommonJS candidate lookup distinguishes missing and empty resources from real host failures. */
test('module reads preserve empty source and propagate the original resource-owner error', async () => {
  const resources = new Map([
    ['dist/empty.js', ''],
    ['dist/shared.js', 'exports.label = "module value";'],
  ]);
  const successful = runtime({
    /** Implements the selected resource owner's explicit text-or-null contract. */
    __operitNativeReadToolPkgTextResource(packageName, path) {
      assert.equal(packageName, 'fixture');
      return resources.has(path) ? resources.get(path) : null;
    },
  });
  successful.context.__operitExecuteScriptFunction('modules', {
    __operit_toolpkg_api_version:'2.0.0', toolPkgId:'fixture', __operit_ui_package_name:'fixture',
    __operit_script_screen:'dist/main.js',
  }, 'var empty = require("./empty"); var shared = require("./shared"); exports.run = function() { return {empty:empty.missing===undefined, label:shared.label}; };', 'run', 60, 10000);
  await flush();
  assert.equal(successful.failures.size, 0);
  assert.equal(successful.outputs.get('modules').empty, true);
  assert.equal(successful.outputs.get('modules').label, 'module value');
  const failed = runtime({
    /** Rejects the real owner failure instead of reporting an absent module or empty source. */
    __operitNativeReadToolPkgTextResource() { throw new Error('resource owner read denied'); },
  });
  failed.context.__operitExecuteScriptFunction('failed-modules', {
    __operit_toolpkg_api_version:'2.0.0', toolPkgId:'fixture', __operit_ui_package_name:'fixture',
    __operit_script_screen:'dist/main.js',
  }, 'var shared = require("./shared"); exports.run = function() { return shared; };', 'run', 60, 10000);
  await flush();
  assert.equal(failed.outputs.size, 0);
  assert.match(failed.failures.get('failed-modules').message, /resource owner read denied/);
});

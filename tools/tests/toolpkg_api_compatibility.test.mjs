import { readSdkScript } from "../runtime/compose_scripts.mjs";
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';

const root = new URL('../../', import.meta.url);

/** Reads production JavaScript embedded in a Rust raw string. */
function embedded(path) {
  if (path.endsWith('/ToolPkgRegistrationBridge.rs')) return readSdkScript('ToolPkgRegistrationBridge.rs');
  const source = readFileSync(new URL(path, root), 'utf8');
  const start = source.indexOf('r#"') + 3;
  assert.ok(start >= 3, `${path} must contain a runtime script`);
  return source.slice(start, source.indexOf('"#', start));
}

/** Installs the production dispatcher with a mutable per-call contract. */
function runtime(version) {
  const context = vm.createContext({
    version,
    __operitCurrentCallId: 'test-call',
    /** Returns the contract of the active call rather than the initial engine state. */
    __operitGetCallState() {
      return { params: { __operit_toolpkg_api_version: context.version } };
    },
    /** Publishes production runtime globals in this isolated context. */
    __operitExpose(name, value) { context[name] = value; },
  });
  vm.runInContext(embedded('core/crates/plugin/sdk/src/toolpkg/ToolPkgApiRuntimeScript.rs'), context);
  return context;
}

/** Verifies one implementation serves explicitly selected independent families. */
test('between and since share one implementation without spanning another major family', () => {
  const context = runtime('1.0.1');
  vm.runInContext(`
    var Api = __operitToolPkgApi.namespace('Test', {
      call: __operitToolPkgApi.method()
        .between('1.0.1', '2.0.0')
        .since('2.0.0')
        /** Preserves arguments and the receiver across compatibility dispatch. */
        .implement(function(value) { return this.prefix + value; })
    });
    Api.prefix = 'result:';
  `, context);
  // Future versions here exercise dispatch semantics, not the manifest admission list.
  for (const version of ['1.0.1', '1.5.0', '2.0.0', '2.8.0']) {
    context.version = version;
    assert.equal(vm.runInContext('Api.call(7)', context), 'result:7');
  }
  for (const version of ['1.0.0', '3.0.0']) {
    context.version = version;
    assert.throws(() => vm.runInContext('Api.call(7)', context), /requires|no implementation/);
  }
});

/** Verifies one family's changes cannot select another family's implementation. */
test('families can evolve with distinct adapters and explicit range boundaries', () => {
  const context = runtime('1.0.0');
  vm.runInContext(`
    var Api = __operitToolPkgApi.namespace('Test', {
      call: __operitToolPkgApi.method()
        .between('1.0.0', '1.0.1', function() { return 'v100'; })
        .between('1.0.1', '2.0.0', function() { return 'v101'; })
        .since('2.0.0', function() { return 'v2'; })
    });
  `, context);
  for (const [version, result] of [['1.0.0', 'v100'], ['1.0.1', 'v101'], ['1.9.0', 'v101'], ['2.0.0', 'v2']]) {
    context.version = version;
    assert.equal(vm.runInContext('Api.call()', context), result);
  }
});

/** Verifies gaps, overlaps, invalid declarations, and unbound ranges fail explicitly. */
test('invalid dispatch declarations and unmatched versions are rejected', () => {
  const context = runtime('1.5.0');
  for (const expression of [
    "__operitToolPkgApi.method().between('1.0.0', '2.0.0').build('Test.call')",
    "__operitToolPkgApi.method().implement(function() {})",
    "__operitToolPkgApi.method().since('1.0.0', 7)",
    "__operitToolPkgApi.method().between('2.0.0', '1.0.0', function() {}).build('Test.call')",
    "__operitToolPkgApi.method().between('1.0.0', '2.0.0', function() {}).between('1.5.0', '2.0.0', function() {}).build('Test.call')",
  ]) {
    assert.throws(() => vm.runInContext(expression, context));
  }
  vm.runInContext(`
    var call = __operitToolPkgApi.method()
      .between('1.0.0', '1.1.0', function() {})
      .since('2.0.0', function() {})
      .build('Test.call');
  `, context);
  assert.throws(() => vm.runInContext('call()', context), /no implementation/);
});

/** Verifies dispatch neither retries an error nor transforms asynchronous results. */
test('implementation errors and promise identity pass through unchanged', () => {
  const context = runtime('1.0.1');
  vm.runInContext(`
    var failure = new Error('adapter failed');
    var calls = 0;
    var result = Promise.resolve('ok');
    var call = __operitToolPkgApi.method()
      .between('1.0.1', '2.0.0', function() { calls++; throw failure; })
      .since('2.0.0', function() { calls++; return result; })
      .build('Test.call');
  `, context);
  assert.throws(() => vm.runInContext('call()', context), error => error === context.failure);
  assert.equal(context.calls, 1);
  context.version = '2.0.0';
  assert.equal(vm.runInContext('call()', context), context.result);
  assert.equal(context.calls, 2);
});

/** Verifies namespaced registrations retain the explicitly selected API contract. */
test('namespaced registrations use current payloads and retain 1.0.1 gates', () => {
  for (const registrationOnly of ['true', 'false']) {
    for (const version of ['1.0.0', '1.0.1', '2.0.0']) {
      const context = runtime(version);
      vm.runInContext(embedded('core/crates/plugin/sdk/src/toolpkg/ToolPkgRegistrationBridge.rs')
        .replace('__OPERIT_TOOLPKG_REGISTRATION_ONLY__', registrationOnly), context);
      vm.runInContext(`
        /** Handles a legacy message hook in the fixture module. */
        function onMessage() {}
        /** Handles a legacy runtime hook in the fixture module. */
        function onRuntime() {}
        /** Handles a legacy message menu action in the fixture module. */
        function onMenu() {}
        /** Exposes durable callbacks using the production module-export protocol. */
        globalThis.__operitGetActiveModuleExports = function() {
          return { onMessage: onMessage, onRuntime: onRuntime, onMenu: onMenu };
        };
        ToolPkg.registerChatMessageHook({ id: 'message', function: onMessage });
        ToolPkg.registerUiRoute({ id: 'screen', screen: 'ui/main.js', runtime: 'compose_dsl' });
      `, context);
      assert.equal(context.__operitToolPkgRegistrationCapture.chatMessageHooks.length, 1);
      assert.equal(context.__operitToolPkgRegistrationCapture.uiRoutes.length, 1);
      const registration = `
        ToolPkg.registerChatRuntimeHook({ id: 'runtime', function: onRuntime });
        ToolPkg.registerChatMessageMenuItem({
          id: 'menu', function: onMenu, dialog: { screen: 'ui/dialog.js' }
        });
      `;
      if (version === '1.0.0') {
        assert.throws(() => vm.runInContext(registration, context), /requires/);
        assert.equal(context.__operitToolPkgRegistrationCapture.chatRuntimeHooks.length, 0);
        continue;
      }
      vm.runInContext(registration, context);
      const capture = context.__operitToolPkgRegistrationCapture;
      assert.equal(JSON.parse(capture.chatRuntimeHooks[0]).function, 'onRuntime');
      assert.deepEqual(JSON.parse(capture.chatMessageMenuItems[0]), {
        id: 'menu', function: 'onMenu', dialog: { screen: 'ui/dialog.js' },
      });
      assert.equal(context.__operitToolPkgApi.currentVersion(), version);
    }
  }
});

/** Keeps registration operations exclusively on ToolPkg in both execution modes. */
test('registration bridge exposes no standalone global registration aliases', () => {
  for (const registrationOnly of ['true', 'false']) {
    const context = runtime('2.0.0');
    vm.runInContext(embedded('core/crates/plugin/sdk/src/toolpkg/ToolPkgRegistrationBridge.rs')
      .replace('__OPERIT_TOOLPKG_REGISTRATION_ONLY__', registrationOnly), context);
    const methods = Object.keys(context.ToolPkg).filter(name => /^register[A-Z]/.test(name));
    assert.ok(methods.length >= 28);
    for (const method of methods) {
      assert.equal(typeof context.ToolPkg[method], 'function', method);
      for (const alias of [method, 'registerToolPkg' + method.slice('register'.length)]) {
        assert.equal(Object.hasOwn(context, alias), false, alias);
        assert.equal(vm.runInContext('typeof ' + alias, context), 'undefined', alias);
      }
    }
  }
});

/** Rejects duplicate global declarations in Rust-generated author-facing types. */
test('generated ToolPkg declarations expose only namespaced registration operations', () => {
  const source = readFileSync(new URL('plugins/types/toolpkg.d.ts', root), 'utf8');
  assert.doesNotMatch(source, /function registerToolPkg[A-Z]/);
  assert.doesNotMatch(source, /Requires the host to implement every global host operation/);
  assert.match(source, /registerUiRoute\(definition: UiRouteRegistration\): void;/);
  assert.doesNotMatch(source, /\b(?:GlobalHost|GlobalBinding)\b/);
  const globalBlocks = [...source.matchAll(/declare global \{([^}]*)\}/g)];
  assert.equal(globalBlocks.length, 1);
  assert.equal(globalBlocks[0][1].trim(), 'export import ToolPkg = ToolPkgRuntime;');
});

/** Checks that the canonical generated Chat.call bindings explicitly cover both families. */
test('generated chat model bindings declare disjoint legacy and current API ranges', () => {
  const source = readFileSync(new URL('core/crates/plugin/sdk/src/js_sdk/runtime_bindings.rs', root), 'utf8');
  const variants = [...source.matchAll(/JsToolApiVariant\s*\{\s*namespace:\s*"Chat",\s*method:\s*"call",\s*since:\s*"([^"]+)",\s*until:\s*(Some\("[^"]+"\)|None),\s*arguments:\s*None,?\s*\}/g)];
  assert.deepEqual(variants.map(match => [match[1], match[2]]), [
    ['1.0.1', 'Some("2.0.0")'], ['2.0.0', 'None'],
  ]);
  const context = runtime('1.0.0');
  const builder = context.__operitToolPkgApi.method();
  /** Stands in for the shared generated tool invocation without executing a model request. */
  const invoke = options => options;
  for (const match of variants) {
    if (match[2] === 'None') builder.since(match[1], invoke);
    else builder.between(match[1], /^Some\("([^"]+)"\)$/.exec(match[2])[1], invoke);
  }
  const call = builder.build('Tools.Chat.call');
  assert.throws(() => call({}), /requires/);
  const options = { functionType: 'CHAT', turns: [] };
  for (const version of ['1.0.1', '2.0.0']) {
    context.version = version;
    assert.equal(call(options), options);
  }
});

/** Checks successive since declarations evolve within, but never across, their major family. */
test('successive since declarations remain independent across major families', () => {
  const context = runtime('1.0.0');
  vm.runInContext(`
    var call = __operitToolPkgApi.method()
      .since('1.0.0', function() { return 'v100'; })
      .since('1.0.1', function() { return 'v101'; })
      .since('2.0.0', function() { return 'v200'; })
      .since('2.1.0', function() { return 'v210'; })
      .build('Test.call');
  `, context);
  for (const [version, value] of [['1.0.0', 'v100'], ['1.0.1', 'v101'], ['1.9.0', 'v101'], ['2.0.0', 'v200'], ['2.1.0', 'v210']]) {
    context.version = version;
    assert.equal(vm.runInContext('call()', context), value);
  }
  context.version = '3.0.0';
  assert.throws(() => vm.runInContext('call()', context), /no implementation/);
});

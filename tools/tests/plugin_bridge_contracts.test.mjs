import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';

const root = new URL('../../', import.meta.url);

/** Reads the first embedded runtime script from a Rust source file. */
function embedded(path) {
  const source = readFileSync(new URL(path, root), 'utf8');
  const start = source.indexOf('r#"') + 3;
  return source.slice(start, source.indexOf('"#', start));
}

test('plugin console forwards info, warnings and errors to the native log sink', () => {
  const source = readFileSync(new URL(
    'core/crates/plugin/javascript-bridge/src/javascript/JsLibraries.rs', root,
  ), 'utf8');
  const start = source.indexOf('        var console = {{');
  const end = source.indexOf('        var intervalStates =', start);
  const calls = [];
  const context = vm.createContext({
    /** Captures the exact severity and text crossing the JavaScript host boundary. */
    __operitNativeLog(level, callId, message) { calls.push({ level, callId, message }); },
  });
  vm.runInContext(source.slice(start, end).replaceAll('{{', '{').replaceAll('}}', '}'), context);
  vm.runInContext(`console.log('site', 'doubao'); console.warn('warning'); console.error('failed');`, context);
  assert.deepEqual(calls, [
    { level: 'info', callId: '', message: 'site doubao' },
    { level: 'warn', callId: '', message: 'warning' },
    { level: 'error', callId: '', message: 'failed' },
  ]);
});

test('packaged Bing sidebar callback matches the nested Rust action contract', () => {
  const exports = {};
  const context = vm.createContext({
    exports,
    Icons: { Language: 'Language' },
    __operitCurrentCallId: 'registration',
    /** Supplies the manifest version for production API gating. */
    __operitGetCallState() { return { params: { __operit_toolpkg_api_version: '2.0.0' } }; },
    /** Exposes the production API registry in the test runtime. */
    __operitExpose(name, value) { context[name] = value; },
    /** Resolves exported callback identities from the actual plugin module. */
    __operitGetActiveModuleExports() { return exports; },
  });
  context.window = context;
  vm.runInContext(embedded('core/crates/plugin/sdk/src/toolpkg/ToolPkgApiRuntimeScript.rs'), context);
  vm.runInContext(embedded('core/crates/plugin/sdk/src/toolpkg/ToolPkgRegistrationBridge.rs')
    .replace('__OPERIT_TOOLPKG_REGISTRATION_ONLY__', 'true'), context);
  vm.runInContext(readFileSync(new URL('plugins/packages/external/sidebar_bing_action/dist/main.js', root), 'utf8'), context);
  exports.registerToolPkg();
  const entry = JSON.parse(context.__operitToolPkgRegistrationCapture.navigationEntries[0]);
  assert.deepEqual(entry.action, { function: 'openBingFromSidebar' });
});

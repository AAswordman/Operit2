import assert from 'node:assert/strict';
import vm from 'node:vm';
import test from 'node:test';
import { readSdkScript } from '../runtime/compose_scripts.mjs';

/** Evaluates the actual public registration boundary with a known owning package. */
function runtime() {
  const context = vm.createContext({
    __operitCurrentCallId: 'registration',
    __operitGetCallState: () => ({ params: {
      toolPkgId: 'com.operit.review', __operit_toolpkg_api_version: '2.0.0',
    } }),
    __operitExpose(name, value) { context[name] = value; },
  });
  vm.runInContext(readSdkScript('ToolPkgApiRuntimeScript.rs'), context);
  vm.runInContext(readSdkScript('ToolPkgRegistrationBridge.rs')
    .replace('__OPERIT_TOOLPKG_REGISTRATION_ONLY__', 'true'), context);
  return context;
}

for (const surface of ['chat_message_after', 'workspace', 'workspace_home']) {
  test(`${surface} preserves registered routes and opaque plugin input`, () => {
    const context = runtime();
    vm.runInContext(`
      ToolPkg.registerUiRoute({ id: 'review', route: 'toolpkg:com.operit.review:ui:review',
        screen: 'dist/review.ui.js', runtime: 'compose_dsl' });
      ToolPkg.registerNavigationEntry({ id: 'entry', surface: '${surface}',
        route: 'toolpkg:com.operit.review:ui:review', params: { mode: 'opaque', count: 3 } });
    `, context);
    const [entry] = context.__operitToolPkgRegistrationCapture.navigationEntries.map(JSON.parse);
    assert.equal(entry.surface, surface);
    assert.equal(entry.route, 'toolpkg:com.operit.review:ui:review');
    assert.deepEqual(entry.params, { mode: 'opaque', count: 3 });
  });

  test(`${surface} rejects missing, foreign, unregistered and duplicate routes`, () => {
    for (const [setup, route, error] of [
      ['', 'undefined', /route is required/],
      ['', "'toolpkg:other:ui:review'", /must belong to this package/],
      ['', "'toolpkg:com.operit.review:ui:review'", /not registered/],
      [`for (let i = 0; i < 2; i++) ToolPkg.registerUiRoute({ id: 'review',
        route: 'toolpkg:com.operit.review:ui:review', screen: 'screen.ui.js' });`,
        "'toolpkg:com.operit.review:ui:review'", /duplicated/],
    ]) {
      const context = runtime();
      vm.runInContext(setup, context);
      assert.throws(() => vm.runInContext(`ToolPkg.registerNavigationEntry({
        id: 'entry', surface: '${surface}', route: ${route} });`, context), error);
      assert.equal(context.__operitToolPkgRegistrationCapture.navigationEntries.length, 0);
    }
  });

  test(`${surface} rejects native action callbacks and non-Compose routes`, () => {
    const context = runtime();
    assert.throws(() => vm.runInContext(`ToolPkg.registerNavigationEntry({
      id: 'entry', surface: '${surface}', action: () => {} });`, context), /action is unsupported/);
    vm.runInContext(`ToolPkg.registerUiRoute({ id: 'review',
      route: 'toolpkg:com.operit.review:ui:review', screen: 'screen.ui.js', runtime: 'other' });`, context);
    assert.throws(() => vm.runInContext(`ToolPkg.registerNavigationEntry({
      id: 'entry', surface: '${surface}', route: 'toolpkg:com.operit.review:ui:review' });`, context), /must use compose_dsl/);
  });
}

test('surface names are exact and JSON input is validated before capture', () => {
  const context = runtime();
  for (const surface of ['chat_message_after ', 'Chat_Message_After', ' workspace']) {
    assert.throws(() => vm.runInContext(`ToolPkg.registerNavigationEntry({
      id: 'entry', surface: '${surface}' });`, context), /surface is unsupported/);
  }
  vm.runInContext(`ToolPkg.registerUiRoute({ id: 'review',
    route: 'toolpkg:com.operit.review:ui:review', screen: 'screen.ui.js' });`, context);
  assert.throws(() => vm.runInContext(`ToolPkg.registerNavigationEntry({
    id: 'entry', surface: 'chat_message_after',
    route: 'toolpkg:com.operit.review:ui:review', params: { bad: Infinity } });`, context), /finite JSON numbers/);
});

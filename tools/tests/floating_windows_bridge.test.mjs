import assert from 'node:assert/strict';
import vm from 'node:vm';
import test from 'node:test';
import { readSdkScript } from '../runtime/compose_scripts.mjs';

function setup(registration) {
  const calls = [];
  const context = vm.createContext({
    __operitExpose(name, value) { context[name] = value; },
    __operitCurrentCallId: 'call',
    __operitGetCallState() { return {params: {__operit_toolpkg_api_version: '2.0.0'}}; },
    __operitNativeFloatingWindowCommand(request) { calls.push(structuredClone(request)); return request.operation; },
    __operitInvokeHostAsync(binding, args) { return Promise.resolve(binding(...args)); },
  });
  vm.runInContext(readSdkScript('ToolPkgApiRuntimeScript.rs'), context);
  vm.runInContext(readSdkScript('ToolPkgRegistrationBridge.rs')
    .replace('__OPERIT_TOOLPKG_REGISTRATION_ONLY__', String(registration)), context);
  return {context, calls};
}

test('UI floatingWindows namespace sends structured commands without modifying registration metadata', async () => {
  const {context, calls} = setup(false);
  assert.equal(context.ToolPkg.windows, undefined);
  assert.equal(typeof context.ToolPkg.ui.floatingWindows.open, 'function');
  await vm.runInContext(`(async () => {
    await ToolPkg.ui.floatingWindows.open({id:'pet', route:'pet-route', options:{width:64}, operation:'forged'});
    await ToolPkg.ui.floatingWindows.update('pet', {x:-20, clickThrough:true});
    await ToolPkg.ui.floatingWindows.update('pet', {width:128}, {settings:{animated:false}});
    await ToolPkg.ui.floatingWindows.hide('pet'); await ToolPkg.ui.floatingWindows.show('pet');
    await ToolPkg.ui.floatingWindows.close('pet'); await ToolPkg.ui.floatingWindows.list();
    await ToolPkg.ui.floatingWindows.capabilities(); await ToolPkg.ui.floatingWindows.requestPermission();
  })()`, context);
  assert.deepEqual(calls, [
    {id:'pet',route:'pet-route',options:{width:64},operation:'open'},
    {id:'pet',options:{x:-20,clickThrough:true},operation:'update'},
    {id:'pet',options:{width:128},state:{settings:{animated:false}},operation:'update'},
    {id:'pet',operation:'hide'}, {id:'pet',operation:'show'}, {id:'pet',operation:'close'},
    {operation:'list'}, {operation:'capabilities'}, {operation:'requestPermission'},
  ]);
  assert.equal(context.__operitToolPkgRegistrationCapture.desktopWidgets.length, 0);
});

test('registration never creates windows or prompts for overlay permission', () => {
  const {context, calls} = setup(true);
  for (const expression of ['open({id:"pet",route:"pet"})','capabilities()','requestPermission()']) {
    assert.throws(() => vm.runInContext(`ToolPkg.ui.floatingWindows.${expression}`, context), /during registration/);
  }
  assert.deepEqual(calls, []);
});

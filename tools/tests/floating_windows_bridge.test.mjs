import assert from 'node:assert/strict';
import vm from 'node:vm';
import test from 'node:test';
import { readSdkScript } from '../runtime/compose_scripts.mjs';

/** Loads the real SDK transport with an observable authenticated host binding. */
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

test('managed windows send desired content with protected identity and explicit null closure', async () => {
  const {context, calls} = setup(false);
  await vm.runInContext(`(async () => {
    const request = {requestId:'approval', tool:{name:'write_file', parameters:[]}};
    await ToolPkg.ui.floatingWindows.set('approval', {
      id:'forged', route:'approval-route', fill:true, state:{request}, visible:false, operation:'forged'
    });
    await ToolPkg.ui.floatingWindows.set('approval', null);
  })()`, context);
  assert.deepEqual(calls, [
    {id:'approval',route:'approval-route',fill:true,state:{request:{requestId:'approval',tool:{name:'write_file',parameters:[]}}},visible:true,operation:'set'},
    {id:'approval',visible:false,operation:'set'},
  ]);
});

test('window state rejects lossy JSON serialization before sending a host request', () => {
  const {context, calls} = setup(false);
  for (const expression of [
    '[]', 'null', '{invalid:undefined}', '{invalid:() => {}}', '{invalid:NaN}',
    '{invalid:new Date()}', '(() => {const state={};state.self=state;return state;})()',
  ]) {
    assert.throws(() => vm.runInContext(`ToolPkg.ui.floatingWindows.set('test', {route:'route', state:${expression}})`, context), /Floating window state/);
  }
  assert.throws(() => vm.runInContext("ToolPkg.ui.floatingWindows.set('test', undefined)", context), /object or null/);
  assert.deepEqual(calls, []);
});

test('registration never creates windows or prompts for overlay permission', () => {
  const {context, calls} = setup(true);
  for (const expression of ['open({id:"pet",route:"pet"})','set("pet", {route:"pet"})','set("pet", null)','capabilities()','requestPermission()']) {
    assert.throws(() => vm.runInContext(`ToolPkg.ui.floatingWindows.${expression}`, context), /during registration/);
  }
  assert.deepEqual(calls, []);
});

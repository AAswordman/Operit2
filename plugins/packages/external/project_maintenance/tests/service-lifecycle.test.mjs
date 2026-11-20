import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {objectStorage} from '../../qqbot/tests/storage.mjs';
const require=createRequire(import.meta.url);

test('registration stays declarative and host lifecycle callbacks own background work',async()=>{
 const lifecycle=[],intervals=[],channels=[],storage=objectStorage();let authenticated=false,opened=0,listed=0;
 globalThis.Tools={
  Storage:{objects:{async open(options){assert.equal(authenticated,true,'registration must not open storage');opened++;return storage.open(options);}}},
  SoftwareSettings:{async exec(args){assert.equal(authenticated,true);assert.deepEqual(args,['--json','plugin','list']);listed++;return '[]';}},
 };
 globalThis.ToolPkg={
  getSpaceDataDir:()=>'/service-lifecycle',getLocalDataDir:()=>'/service-lifecycle-local',
  ipc:{on(channel,handler){channels.push({channel,handler});}},
  registerAppLifecycleHook(hook){lifecycle.push(hook);},registerHostEventHook(hook){intervals.push(hook);},
  registerPromptFinalizeHook(){},registerChatMessageHook(){},registerApi(){},registerUiRoute(){},registerNavigationEntry(){},
  async callDependency(){assert.fail('missing connector must not be called');},
 };
 const originalInterval=globalThis.setInterval;
 globalThis.setInterval=()=>assert.fail('plugin registration must not start JavaScript intervals');
 try{
  const main=require('../dist/main.js');assert.equal(main.registerToolPkg(),true);
  await new Promise(setImmediate);assert.equal(opened,0);assert.equal(listed,0);assert.equal(channels.length,1);
  assert.deepEqual(lifecycle.map(hook=>hook.event),['application_on_create']);
  assert.deepEqual(intervals.map(hook=>hook.trigger),[{kind:'interval',intervalMs:3000},{kind:'interval',intervalMs:30000}]);
  for(const hook of [...lifecycle,...intervals])assert.equal(main[hook.function.name],hook.function);
  authenticated=true;await lifecycle[0].function();assert.ok(opened>0);assert.ok(listed>0);
  await intervals[0].function();await intervals[1].function();
 }finally{globalThis.setInterval=originalInterval;}
});

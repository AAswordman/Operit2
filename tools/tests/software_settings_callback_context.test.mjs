import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import test from "node:test";
import { installScopedHostRuntime } from "./fixtures/scoped_host_runtime.mjs";

const source = readFileSync(new URL("../../core/crates/plugin/codegen/src/runtime_bindings.rs", import.meta.url), "utf8");
const start = source.indexOf("function __operitReadSoftwareSettingsDirectory(method)");
const end = source.indexOf("function __operitRequireChatJsonObject", start);

/** Uses the real scoped Promise protocol; only domain data completion is test-controlled. */
function context() {
  const pending=[];
  const sandbox=vm.createContext({});
  const registry=installScopedHostRuntime(sandbox);
  sandbox.__operitNativeReadSoftwareSettingsDirectoryAsync=registry.binding((requestId,method)=>pending.push({requestId,method}));
  sandbox.__operitNativeApplySoftwareSettingsConfigAsync=registry.binding((requestId,method,id)=>pending.push({requestId,method,id}));
  vm.runInContext(source.slice(start,end),sandbox);
  return {sandbox,pending,registry};
}

for (const call of ["__operitReadSoftwareSettingsDirectory('listModelSummaries')", "__operitApplySoftwareSettingsConfig('setCurrentTtsConfigId', 'explicit-config')"]) {
  test(`${call} restores its exact originating JS call on failure`,async()=>{
    const {sandbox,pending,registry}=context();
    const globals=Object.keys(sandbox);
    const result=vm.runInContext(`(async()=>{try {await ${call};} catch(error) {return {owner:__operitCurrentCallId,message:error.message};} throw new Error('Expected rejection');})()`,sandbox);
    assert.equal(pending.length,1);
    vm.runInContext("__operitCurrentCallId='finished-unrelated-hook'",sandbox);
    registry.settle(pending[0].requestId,'exact host failure',true);
    assert.deepEqual(JSON.parse(JSON.stringify(await result)),{owner:'owner',message:'exact host failure'});
    assert.equal(sandbox.refs,0);
    assert.deepEqual(Object.keys(sandbox),globals);
  });
}

test('out-of-order directory/configuration completions preserve their own owners',async()=>{
  const {sandbox,pending,registry}=context();
  const directory=vm.runInContext("__operitReadSoftwareSettingsDirectory('listModelSummaries').then(value=>({owner:__operitCurrentCallId,value}))",sandbox);
  vm.runInContext("__operitCurrentCallId='configuration-owner'",sandbox);
  const configuration=vm.runInContext("__operitApplySoftwareSettingsConfig('setCurrentTtsConfigId','explicit-config').then(value=>({owner:__operitCurrentCallId,value}))",sandbox);
  assert.equal(pending[1].id,'explicit-config');
  registry.settle(pending[1].requestId,null,false);
  assert.equal((await configuration).owner,'configuration-owner');
  registry.settle(pending[0].requestId,[{id:'model'}],false);
  const result=await directory;
  assert.equal(result.owner,'owner');
  assert.deepEqual(structuredClone(result.value),[{id:'model'}]);
  assert.equal(sandbox.refs,0);
});

test('invalid configuration IDs do not reach the host or retain call references',async()=>{
  const {sandbox,pending}=context();
  for (const id of ['', ' spaced ',null]) {
    sandbox.invalidId=id;
    await assert.rejects(vm.runInContext("__operitApplySoftwareSettingsConfig('setCurrentTtsConfigId',invalidId)",sandbox),/exact nonblank/);
  }
  assert.equal(pending.length,0);
  assert.equal(sandbox.refs,0);
});

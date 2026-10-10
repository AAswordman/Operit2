import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';
import { installScopedHostRuntime } from './fixtures/scoped_host_runtime.mjs';

const codegen=readFileSync(new URL('../../core/crates/plugin/codegen/src/runtime_bindings.rs',import.meta.url),'utf8');
const helpers=codegen.slice(codegen.indexOf('function __operitRequireChatJsonObject'),codegen.indexOf('\n"#,',codegen.indexOf('function __operitRequireChatJsonObject')));
const chat=readFileSync(new URL('../../core/crates/plugin/sdk/src/chat_runtime.js',import.meta.url),'utf8');

/** Exercises production request snapshots and scoped Promise bindings, not callback-ID simulations. */
function fixture() {
  const sandbox=vm.createContext({});
  const registry=installScopedHostRuntime(sandbox);
  const pending=[];
  sandbox.__operitNativeChatAsync=registry.binding((requestId,method,payload)=>pending.push({requestId,method,payload}));
  sandbox.__operitNativeChatExtensionAsync=registry.binding((requestId,method,target,value)=>pending.push({requestId,method,target,value}));
  vm.runInContext(helpers+'\n'+chat,sandbox);
  return {sandbox,pending,registry,
    run(code) {return vm.runInContext(code,sandbox);},
    reply(value,reject=false) {const request=pending.shift();assert.ok(request);registry.settle(request.requestId,value,reject);return request;},
  };
}

/** A lazy stream must use the original request even if its author mutates nested data before pulling. */
test('Chat streaming snapshots the send request at iterator creation, without JSON text',async()=>{
  const f=fixture();
  f.run(`JSON.stringify=JSON.parse=function(){throw new Error('JSON transport used');};
    var request={chatId:'chat',input:{text:'original',attachments:[{name:'initial'}]}};
    var stream=__operitChatStream(request);
    request.input.text='changed';request.input.attachments[0].name='changed';`);
  assert.equal(f.pending.length,0);
  const next=f.run('stream.next()');
  assert.equal(f.pending[0].method,'open');
  assert.deepEqual(structuredClone(f.pending[0].payload),{chatId:'chat',input:{text:'original',attachments:[{name:'initial'}]}});
  f.reply('observation');
  await new Promise(resolve=>setImmediate(resolve));
  assert.equal(f.pending[0].method,'next');
  f.reply({type:'part',parts:[]});
  assert.equal((await next).value.type,'part');
  const closed=f.run('stream.return()');
  await new Promise(resolve=>setImmediate(resolve));
  assert.equal(f.pending[0].method,'close');
  f.reply(null);await closed;
  assert.equal(f.sandbox.refs,0);
});

test('Chat streams reject concurrent pulls and detach completed observations without cancellation',async()=>{
  const f=fixture();f.run("var stream=__operitChatStream({chatId:'chat'});");
  const next=f.run('stream.next()');
  await assert.rejects(f.run('stream.next()'),/one pending next/);
  f.reply('observation');await new Promise(resolve=>setImmediate(resolve));
  f.reply({type:'completed',result:{chatId:'chat'}});await new Promise(resolve=>setImmediate(resolve));
  assert.equal(f.pending[0].method,'close');f.reply(null);
  assert.equal((await next).value.type,'completed');
  assert.equal((await f.run('stream.next()')).done,true);
  assert.equal(f.pending.length,0);assert.equal(f.sandbox.refs,0);
});

test('Chat extension requests preserve structured values and the rejection owner',async()=>{
  const f=fixture();const globals=Object.keys(f.sandbox);
  const result=f.run("__operitChatExtension('writeExtension',{chatId:'chat'},{literal:'{\"value\":1}',nested:[null,1]}).catch(error=>({owner:__operitCurrentCallId,message:error.message}))");
  assert.equal(typeof f.pending[0].requestId,'number');
  assert.deepEqual(structuredClone(f.pending[0].value),{literal:'{"value":1}',nested:[null,1]});
  f.run("__operitCurrentCallId='unrelated'");f.reply('exact native rejection',true);
  assert.deepEqual(structuredClone(await result),{owner:'owner',message:'exact native rejection'});
  assert.deepEqual(Object.keys(f.sandbox),globals);assert.equal(f.sandbox.refs,0);
});

test('Chat rejects cyclic, excessively deep and accessor data before retaining host requests',async()=>{
  const f=fixture();
  for (const request of ["(()=>{var x={};x.self=x;return x;})()", "(()=>{var x={},y=x;for(var i=0;i<130;i++){y.next={};y=y.next;}return x;})()", "Object.defineProperty({},'input',{enumerable:true,get(){throw new Error('getter must not execute');}})"]){
    await assert.rejects(f.run(`__operitChatSend(${request})`),/cyclic|depth\/node|non-JSON property/);
  }
  assert.equal(f.pending.length,0);assert.equal(f.sandbox.refs,0);
});

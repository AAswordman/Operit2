import assert from 'node:assert/strict';
import vm from 'node:vm';
import test from 'node:test';
import {installScopedHostRuntime, installGeneratedToolsRuntime} from './fixtures/scoped_host_runtime.mjs';

const turn = () => new Promise(resolve => setImmediate(resolve));
function fixture() {
  const context = vm.createContext({});
  const registry = installScopedHostRuntime(context), calls = [];
  context.__operitNativeCallToolStructured = registry.binding((requestId, type, name, parameters) => calls.push({requestId, type, name, parameters}));
  installGeneratedToolsRuntime(context);
  return {context, calls, run: code => vm.runInContext(code, context),
    settle(index, data) { registry.settle(calls[index].requestId, {success:true, data}, false); }};
}

test('generated v2 terminal iterable opens lazily, yields live output, and closes on completion', async () => {
  const f = fixture();
  f.run("var request={sessionId:'s',command:'printf sample',timeoutMs:5000}; var stream=Tools.System.terminal.execStream(request); request.command='mutated';");
  assert.equal(f.calls.length, 0);
  let pulling = f.run('stream.next()');
  assert.equal(f.calls[0].name, 'terminal_observe');
  assert.deepEqual(structuredClone(f.calls[0].parameters), {observation:{op:'open',request:{sessionId:'s',command:'printf sample',timeoutMs:5000}}});
  f.settle(0, {handle:'terminal'}); await turn();
  f.settle(1, {event:{type:'chunk',chunk:'雪🌍',chunkIndex:0,receivedChars:3}});
  assert.equal((await pulling).value.chunk, '雪🌍');
  pulling = f.run('stream.next()'); await turn();
  f.settle(2, {event:{type:'completed',result:{output:'雪🌍',exitCode:7,timedOut:false}}}); await turn();
  assert.equal(f.calls[3].parameters.observation.op, 'close');
  f.settle(3, {});
  assert.equal((await pulling).value.result.exitCode, 7);
  assert.equal((await f.run('stream.next()')).done, true);
  await turn();
  assert.equal(f.context.refs, 0);
});

test('terminal return disposes while next waits, and overlapping pulls are rejected', async () => {
  const f = fixture();
  f.run("var stream=Tools.System.terminal.execStream({sessionId:'s',command:'sleep 10'});");
  const pulling = f.run('stream.next()');
  f.settle(0, {handle:'terminal'}); await turn();
  await assert.rejects(f.run('stream.next()'), /only one pending next/);
  const closing = f.run('stream.return()'); await turn();
  assert.equal(f.calls[2].parameters.observation.op, 'close');
  f.settle(2, {}); await closing;
  f.settle(1, {event:null});
  assert.equal((await pulling).done, true);
  await turn();
  assert.equal(f.context.refs, 0);
});

test('v2 callback facade keeps timeout and callbacks outside the structured command request', async () => {
  const f = fixture();
  f.run('var received=[];');
  const result = f.run("Tools.System.terminal.execStreaming('s','cmd',{timeoutMs:'100',onIntermediateResult(event){received.push(event);}})");
  assert.deepEqual(structuredClone(f.calls[0].parameters.observation.request), {sessionId:'s',command:'cmd',timeoutMs:100});
  f.settle(0, {handle:'terminal'}); await turn();
  f.settle(1, {event:{type:'chunk',chunk:'progress'}}); await turn();
  assert.equal(f.context.received.length, 1);
  f.settle(2, {event:{type:'completed',result:{output:'progress',exitCode:-1,timedOut:true}}}); await turn();
  f.settle(3, {});
  assert.equal((await result).timedOut, true);
  await turn(); assert.equal(f.context.refs, 0);
});

test('callback errors retain their identity even when disposal also fails', async () => {
  const f = fixture();
  f.run("var callbackError=new Error('original callback failure');");
  const result = f.run("Tools.System.terminal.execStreaming('s','cmd',{onIntermediateResult(){throw callbackError;}})");
  const rejected = assert.rejects(result, error => error === f.context.callbackError);
  f.settle(0, {handle:'terminal'}); await turn();
  f.settle(1, {event:{type:'chunk',chunk:'progress'}}); await turn();
  assert.equal(f.calls[2].parameters.observation.op, 'close');
  f.context.__operitHostPromiseRegistry.settle(f.calls[2].requestId, {success:false,error:'disposal failure'}, false);
  await rejected;
  await turn();
  assert.equal(f.calls.length, 3);
  assert.equal(f.context.refs, 0);
});

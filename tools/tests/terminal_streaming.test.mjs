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

test('managed SDK run and wait expose typed receipts without disposing the background command', async () => {
  const f = fixture();
  const running = f.run("Tools.System.terminal.run({sessionName:'chat-shell',terminalType:'shell',command:'sleep 30',waitMs:3000})");
  assert.deepEqual(structuredClone(f.calls[0].parameters), { observation: {
    op: 'run', sessionName: 'chat-shell', terminalType: 'shell', command: 'sleep 30', waitMs: 3000,
  }});
  f.settle(0, { result: { sessionId: 'background', output: 'started', timedOut: true, exitCode: -1 } });
  assert.equal((await running).timedOut, true);
  assert.equal(f.calls.length, 1, 'Returning a receipt must not close the background session');
  const waiting = f.run("Tools.System.terminal.wait('background',1000)");
  assert.deepEqual(structuredClone(f.calls[1].parameters), { observation: { op: 'wait', sessionId: 'background', waitMs: 1000 } });
  f.settle(1, { result: { sessionId: 'background', output: 'finished', timedOut: false, exitCode: 7 } });
  assert.equal((await waiting).exitCode, 7);
  await turn();
  assert.equal(f.context.refs, 0);
  assert.equal(f.calls.length, 2);
});

test('SDK interactive open has a distinct request and returns the real host identity', async () => {
  const f = fixture();
  const opening = f.run("Tools.System.terminal.open({sessionName:'interactive',profileId:'ubuntu_a'})");
  assert.deepEqual(structuredClone(f.calls[0].parameters), {observation:{op:'session',sessionName:'interactive',profileId:'ubuntu_a'}});
  f.settle(0,{result:{sessionId:'raw-shell',sessionName:'interactive',terminalType:'bash',isNewSession:true}});
  assert.equal((await opening).sessionId,'raw-shell');
  assert.equal(f.calls.length,1);
});

test('managed SDK rejects invalid wait budgets before submitting any command', () => {
  const f = fixture();
  for (const value of ['-1', 'NaN', 'Infinity', '1.5', "'1000'"]) {
    assert.throws(() => f.run(`Tools.System.terminal.wait('session',${value})`), /waitMs/);
    assert.throws(() => f.run(`Tools.System.terminal.run({sessionName:'s',terminalType:'shell',command:'id',waitMs:${value}})`));
  }
  assert.equal(f.calls.length, 0);
});

/** Exercises named continuous execution over the actual generated SDK and native pull transport. */
test('named runStream snapshots the selected profile and retains observation until actual completion', async () => {
  const f = fixture();
  f.run("var request={sessionName:'build',profileId:'ubuntu_a',command:'slow build'}; var stream=Tools.System.terminal.runStream(request); request.profileId='ubuntu_b';");
  const first = f.run('stream.next()');
  assert.deepEqual(structuredClone(f.calls[0].parameters), {observation:{op:'runstream',request:{sessionName:'build',profileId:'ubuntu_a',command:'slow build'}}});
  f.settle(0,{handle:'continuous'}); await turn();
  f.settle(1,{event:{type:'start',sessionId:'original'}});
  assert.equal((await first).value.sessionId,'original');
  const next = f.run('stream.next()'); await turn();
  f.settle(2,{event:{type:'chunk',sessionId:'original',chunk:'still running'}});
  assert.equal((await next).value.chunk,'still running');
  assert.equal(f.calls.some(call=>call.parameters.observation.op==='close'),false);
  const done = f.run('stream.next()'); await turn();
  f.settle(3,{event:{type:'completed',result:{sessionId:'original',output:'still running',exitCode:0,timedOut:false}}}); await turn();
  assert.equal(f.calls[4].parameters.observation.op,'close');
  f.settle(4,{});
  assert.equal((await done).value.result.exitCode,0);
  assert.equal((await f.run('stream.next()')).done,true);
  await turn(); assert.equal(f.context.refs,0);
});

/** Preserves profile IDs without converting them into interpreter enums or guessed implementations. */
test('managed SDK sends exact profiles and permits the configured default', async () => {
  const f = fixture();
  const selected = f.run("Tools.System.terminal.run({sessionName:'chat',profileId:'ubuntu_b',command:'pwd',waitMs:1000})");
  assert.deepEqual(structuredClone(f.calls[0].parameters), {observation:{
    op:'run',sessionName:'chat',profileId:'ubuntu_b',command:'pwd',waitMs:1000,
  }});
  f.settle(0,{result:{sessionId:'profile-session',output:'/work',exitCode:0,timedOut:false}});
  await selected;
  const defaultRun = f.run("Tools.System.terminal.run({sessionName:'chat',command:'pwd',waitMs:1000})");
  assert.deepEqual(structuredClone(f.calls[1].parameters), {observation:{
    op:'run',sessionName:'chat',command:'pwd',waitMs:1000,
  }});
  f.settle(1,{result:{sessionId:'default-session',output:'/work',exitCode:0,timedOut:false}});
  await defaultRun;
});

/** Uses the canonical SDK argument mapping for exact-profile named session creation. */
test('named session creation sends the profile ID through the public Tools binding', async () => {
  const f = fixture();
  const created = f.run("Tools.System.terminal.create('chat',undefined,'ubuntu_b')");
  assert.deepEqual(structuredClone(f.calls[0].parameters), {
    session_name:'chat',profile_id:'ubuntu_b',
  });
  f.settle(0,{sessionId:'profile-session'});
  assert.equal((await created).sessionId,'profile-session');
});

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

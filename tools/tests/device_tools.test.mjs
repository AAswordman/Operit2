import assert from 'node:assert/strict';
import { readFileSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { execFileSync } from 'node:child_process';
import test from 'node:test';
import vm from 'node:vm';

const repo = resolve(import.meta.dirname, '../..');
const scratch = mkdtempSync(join(tmpdir(), 'operit-device-tools-'));
const generatedPath = process.env.OPERIT_JS_TOOLS_PATH || join(scratch, 'tools.js');
if (!process.env.OPERIT_JS_TOOLS_PATH) {
  execFileSync('cargo', ['run', '--quiet', '--manifest-path', join(repo, 'core/Cargo.toml'),
    '-p', 'operit-plugin-sdk-codegen', '--', 'generate-runtime',
    join(repo, 'core/crates/plugin/sdk/src'), generatedPath], {stdio: 'pipe'});
}
const generated = readFileSync(generatedPath, 'utf8');
rmSync(scratch, {recursive: true, force: true});
const api = readFileSync(join(repo, 'core/crates/plugin/sdk/src/toolpkg/ToolPkgApiRuntimeScript.rs'), 'utf8')
  .split('r#"')[1].split('"#')[0];
const bridge = readFileSync(join(repo, 'core/crates/plugin/sdk/src/JsExecutionRuntimeBridge.script.js'), 'utf8');

function runtime(native) {
  const requests = [];
  const refs = new Map();
  const context = vm.createContext({AbortController,
    __operitCurrentCallId: 'device-tests',
    __operitGetCallState() { return {params: {__operit_toolpkg_api_version: context.version || '2.0.0'}}; },
    __operitExpose(name, value) { context[name] = value; },
    __operitRetainCallReference(id) { refs.set(id, (refs.get(id) || 0) + 1); },
    __operitReleaseCallReference(id) { refs.set(id, refs.get(id) - 1); },
    __operitActivateCall(id) { context.__operitCurrentCallId = id; },
    __operitNativeCallToolStructured(...args) {
      // The native bridge consumes the owning call ID before device metadata.
      args.splice(4, 1);
      requests.push(args);
      return native ? native(args) : Promise.resolve({success: true, data: args[3]});
    },
  });
  vm.runInContext(api, context);
  vm.runInContext(bridge, context);
  vm.runInContext(generated, context);
  return {context, requests, refs};
}
const plain = value => JSON.parse(JSON.stringify(value));
const flush = async () => { await Promise.resolve(); await Promise.resolve(); };

test('generated Tools.device preserves Files overloads and leaves local Tools independent', async () => {
  const {context, requests, refs} = runtime();
  const values = await vm.runInContext(`Promise.all([
    Tools.device('desktop').Files.read('/remote.txt'),
    Tools.device('phone').Files.read({path:'/phone.txt'}),
    Tools.Files.read('/local.txt')])`, context);
  assert.deepEqual(plain(values), [{path:'/remote.txt'}, {path:'/phone.txt'}, {path:'/local.txt'}]);
  assert.equal(requests[0][2], 'read_file_full');
  assert.equal(requests[0][4].nodeId, 'desktop');
  assert.equal(requests[1][4].nodeId, 'phone');
  assert.equal(requests[2].length, 4);
  assert.notEqual(requests[0][4].invocationId, requests[1][4].invocationId);
  await flush();
  assert.equal(refs.get('device-tests'), 0);
});

test('nested namespaces and version-gated Chat retain the selected device', async () => {
  const {context, requests} = runtime();
  await vm.runInContext(`Tools.device('desktop').Net.cookies.get('https://example.test')`, context);
  await vm.runInContext(`Tools.device('phone').Chat.listAll()`, context);
  assert.equal(requests[0][4].nodeId, 'desktop');
  assert.equal(requests[1][4].nodeId, 'phone');
  assert.equal(requests[0][3].action, 'get');
});

test('hardware device views use one selected node and remove the old global APIs', async () => {
  const {context, requests} = runtime();
  assert.equal(context.Tools.Edge, undefined);
  assert.equal(context.Tools.Io, undefined);
  await vm.runInContext(`Tools.device('edge-node').Edge.execute({target:'edge.status',method:'read'}, {value:1})`, context);
  await vm.runInContext(`Tools.device('io-node').Io.execute({port:'gpio',operation:'read'}, {pin:2})`, context);
  assert.equal(requests[0][4].nodeId, 'edge-node');
  assert.equal(requests[1][4].nodeId, 'io-node');
  assert.equal(requests[0][4].action, 'port');
  assert.equal(requests[1][4].action, 'port');
  assert.equal(requests[0][2], 'edge_execute');
  assert.equal(requests[1][2], 'io_execute');
  assert.equal(requests[0][3].node_id, undefined);
  await assert.rejects(vm.runInContext(`Tools.device('edge-node').Edge.execute('second-node', {}, {})`, context), /select the node/);
  assert.equal(requests.length, 2);
});

test('package names and catalog operations use the device transport', async () => {
  const {context, requests} = runtime();
  const text = '{"keep":"as text"}';
  context.text = text;
  await vm.runInContext(`Tools.device('desktop').call('my_package:echo', {value:text})`, context);
  await vm.runInContext(`Tools.device('desktop').catalog()`, context);
  assert.equal(requests[0][2], 'my_package:echo');
  assert.equal(requests[0][3].value, text);
  assert.equal(requests[1][4].action, 'catalog');
  assert.equal(requests[0][4].action, 'execute');
});

test('device selection and timeouts reject invalid values before submission', () => {
  const {context, requests} = runtime();
  for (const expr of ["Tools.device('')", "Tools.device(' desktop')", "Tools.device('desktop', {timeoutMs:0})",
    "Tools.device('desktop', {timeoutMs:1.5})", "Tools.device('desktop', {timeoutMs:86400001})"]) {
    assert.throws(() => vm.runInContext(expr, context));
  }
  assert.equal(requests.length, 0);
});

test('abort cancels the exact invocation and releases owning-call references', async () => {
  let finish;
  const {context, requests, refs} = runtime(args => {
    if (args[4].action === 'cancel') {
      finish({success:false, message:'Device tool call cancelled'});
      return Promise.resolve({success:true, data:null});
    }
    return new Promise(resolve => { finish = resolve; });
  });
  const controller = new AbortController();
  context.signal = controller.signal;
  const pending = vm.runInContext(`Tools.device('desktop', {signal}).Files.read('/remote')`, context);
  controller.abort();
  await assert.rejects(pending, /cancelled/);
  assert.equal(requests.length, 2);
  assert.equal(requests[1][4].action, 'cancel');
  assert.equal(requests[0][4].invocationId, requests[1][4].invocationId);
  await flush();
  assert.equal(refs.get('device-tests'), 0);
});

test('already aborted signals never submit an execution', async () => {
  const {context, requests} = runtime();
  const controller = new AbortController(); controller.abort(); context.signal = controller.signal;
  await assert.rejects(vm.runInContext(`Tools.device('desktop', {signal}).Files.read('/remote')`, context), /cancelled/);
  assert.equal(requests.length, 0);
});

test('remote errors and literal JSON strings pass through without a local fallback', async () => {
  const {context, requests} = runtime(args => Promise.resolve(args[3].path === 'missing'
    ? {success:false, message:'Target device unavailable'} : {success:true, data:'{"literal":true}'}));
  assert.equal(await vm.runInContext(`Tools.device('desktop').Files.read('ok')`, context), '{"literal":true}');
  await assert.rejects(vm.runInContext(`Tools.device('desktop').Files.read('missing')`, context), /unavailable/);
  assert.equal(requests.length, 2);
  assert.ok(requests.every(args => args[4].nodeId === 'desktop'));
});

test('Tools.device is a v2 interface and rejects v1 callers before submission', () => {
  const {context, requests} = runtime();
  context.version = '1.0.0';
  assert.throws(() => vm.runInContext(`Tools.device('desktop')`, context), /no implementation|requires/);
  assert.equal(requests.length, 0);
});

test('device.cancel cancels only its view and permits later calls without AbortController', async () => {
  const finishes = new Map();
  const {context, requests} = runtime(args => {
    const meta = args[4];
    if (meta.action === 'cancel') {
      finishes.get(meta.invocationId)({success:false, message:'Device tool call cancelled'});
      finishes.delete(meta.invocationId);
      return Promise.resolve({success:true, data:null});
    }
    if (args[3].path === 'next') return Promise.resolve({success:true, data:'ready'});
    return new Promise(resolve => finishes.set(meta.invocationId, resolve));
  });
  vm.runInContext(`var first=Tools.device('desktop'); var second=Tools.device('desktop');`, context);
  const cancelled = vm.runInContext(`first.Files.read('pending')`, context);
  const unaffected = vm.runInContext(`second.Files.read('other')`, context);
  vm.runInContext(`first.cancel()`, context);
  await assert.rejects(cancelled, /cancelled/);
  const other = requests.find(args => args[3].path === 'other');
  assert.ok(finishes.has(other[4].invocationId));
  finishes.get(other[4].invocationId)({success:true, data:'unaffected'});
  assert.equal(await unaffected, 'unaffected');
  assert.equal(await vm.runInContext(`first.Files.read('next')`, context), 'ready');
});

test('generic duplex carries nested bytes, preserves event kinds and finishes input independently', async () => {
  let stored;
  const {context, requests, refs} = runtime(args => {
    const request=args[3];
    const value=request.op==='open'?{handle:'duplex'}:
      request.op==='send'?(stored=request.value,null):
      request.op==='next'?{done:false,kind:'Changed',value:stored}:
      request.op==='call'?request.args:null;
    return Promise.resolve({success:true,data:value});
  });
  const result=await vm.runInContext(`(async function() {
    const device=Tools.device('vendor-device');
    if(device.Edge.openAudioInput!==undefined) throw Error('audio API survived');
    const transport=device.transport;
    const original={kind:'bytes',payload:new Uint8Array([0,128,255]),nested:[null,'hello']};
    const echoed=await transport.call('vendor.service','inspect',original);
    const channel=await transport.open({target:'vendor.service',method:'input',property:'output',args:{session:'one'}});
    try {
      await channel.send(original);
      await channel.finish();
      const event=await channel.next();
      let rejected=false;try {await channel.send('late');} catch {rejected=true;}
      return {bytes:Array.from(event.value.value.payload),typed:event.value.value.payload instanceof Uint8Array,
        kind:event.value.kind,vendor:event.value.value.kind,echo:Array.from(echoed.payload),rejected};
    } finally {await channel.close();await channel.close();}
  })()`,context);
  assert.deepEqual(plain(result),{bytes:[0,128,255],typed:true,kind:'Changed',vendor:'bytes',echo:[0,128,255],rejected:true});
  assert(requests.every(request=>request[4].nodeId==='vendor-device'&&request[4].action==='transport'));
  assert.equal(requests.filter(request=>request[3].op==='close').length,1);
  await flush();assert.equal(refs.get('device-tests'),0);
});

test('duplex receives while sending and rejects concurrent reads; view cancellation closes it', async () => {
  let deliver;
  const {context,requests}=runtime(args=> {
    const p=args[3];
    if(p.op==='open') return Promise.resolve({success:true,data:{handle:'session'}});
    if(p.op==='next') return new Promise(resolve=>{deliver=resolve;});
    if(p.op==='send') deliver({success:true,data:{done:false,kind:'Changed',value:p.value}});
    if(p.op==='close') deliver?.({success:true,data:{done:true}});
    return Promise.resolve({success:true,data:null});
  });
  const result=await vm.runInContext(`(async function() {
    const device=Tools.device('board');
    const channel=await device.transport.open({target:'vendor',method:'input',property:'output'});
    const waiting=channel.next();let busy=false;
    try {await channel.next();} catch {busy=true;}
    await channel.send(new Uint8Array([42]));const event=await waiting;
    const pending=channel.next();device.cancel();
    let cancelled=false;try {await pending;} catch(error) {cancelled=/cancelled/.test(error.message);}
    return {busy,byte:event.value.value[0],cancelled};
  })()`,context);
  assert.deepEqual(plain(result),{busy:true,byte:42,cancelled:true});
  assert.equal(requests.filter(request=>request[3].op==='close').length,1);
});

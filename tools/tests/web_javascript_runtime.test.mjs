import assert from 'node:assert/strict';
import { readFile, writeFile, mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { stripTypeScriptTypes } from 'node:module';
import { Worker as NodeWorker } from 'node:worker_threads';
import test from 'node:test';

const root = new URL('../../', import.meta.url);

/** Exercises the production worker and transport in real isolated threads. */
test('browser JavaScript host preserves runtime contracts', { timeout: 20_000 }, async t => {
  const directory = await mkdtemp(join(tmpdir(), 'operit-javascript-'));
  const originalWorker = globalThis.Worker;
  const originalHost = globalThis.__operitHost;
  const runtimes = [];
  try {
    await writeFile(join(directory, 'package.json'), '{"type":"module"}');
    for (const name of ['operit_javascript_transport', 'operit_javascript_runtime', 'operit_javascript_worker']) {
      const source = await readFile(new URL(`apps/flutter/app/web/runtime/src/${name}.ts`, root), 'utf8');
      await writeFile(join(directory, `${name}.js`), stripTypeScriptTypes(source, { mode: 'transform' }));
    }
    const codec = await readFile(new URL('apps/flutter/app/web/runtime/vendor/msgpack.min.js', root), 'utf8');
    await writeFile(join(directory, 'operit_messagepack.js'), `const module={exports:{}};const exports=module.exports;\n${codec}\nexport const MessagePack=module.exports;`);
    await writeFile(join(directory, 'thread.mjs'), `
import {parentPort,workerData} from 'node:worker_threads';
let listener;const pending=[];
parentPort.on('message',data=>{if(listener)listener({data});else pending.push(data);});
globalThis.addEventListener=(type,callback)=>{if(type!=='message')throw new Error(type);listener=callback;for(const data of pending)callback({data});};
globalThis.postMessage=value=>parentPort.postMessage(value);
await import(workerData);
`);
    /** Adapts only browser worker construction; all execution and framing use production code. */
    globalThis.Worker = class {
      /** Starts the production guest module with the Node test event adapter. */
      constructor(url) {
        this.worker = new NodeWorker(pathToFileURL(join(directory, 'thread.mjs')), { workerData: url.href });
        this.listeners = new Map();
      }
      /** Delivers worker-ready events through the production owner's browser event API. */
      addEventListener(type, listener) {
        const wrapped = type === 'message' ? data => listener({ data }) : error => listener({ message: error.message });
        this.listeners.set(listener, wrapped);
        this.worker.on(type, wrapped);
      }
      /** Releases startup-only worker readiness listeners. */
      removeEventListener(type, listener) {
        this.worker.off(type, this.listeners.get(listener));
        this.listeners.delete(listener);
      }
      /** Delivers the original browser command without modifying its payload. */
      postMessage(value) { this.worker.postMessage(value); }
      /** Releases the guest thread after disposal or timeout. */
      terminate() { this.worker.terminate(); }
    };
    const { createBrowserJavaScriptHost } = await import(pathToFileURL(join(directory, 'operit_javascript_runtime.js')).href);
    const broker = createBrowserJavaScriptHost();
    let preparedId;
    globalThis.__operitHost = { javaScriptRuntime: { ...broker, create: () => preparedId } };
    const bootstrap = await readFile(new URL('hosts/web/src/javascript_promises.js', root), 'utf8');
    /** Creates one production runtime with its trusted bridge initialized. */
    async function create() {
      preparedId = await broker.create();
      const runtime = globalThis.__operitCreateJavaScriptRuntime();
      runtimes.push(runtime);
      call(runtime, { operation: 'initialize', source: bootstrap });
      return runtime;
    }
    /** Gives every command a bounded test deadline to catch transport deadlocks. */
    function call(runtime, command, callback = () => { throw new Error('Unexpected host callback'); }) {
      return runtime.execute(command, callback, controlFor(3000));
    }
    function controlFor(timeout) {
      const control = new Int32Array(new SharedArrayBuffer(16));
      const deadline = Date.now() + timeout;
      Atomics.store(control, 2, deadline >>> 0);
      Atomics.store(control, 3, Math.floor(deadline / 0x1_0000_0000));
      return control;
    }
    const runtime = await create();
    await t.test('keeps independent global state and compiles generator for-of syntax', async () => {
      call(runtime, { operation: 'evaluateVoid', name: 'syntax.js', source: 'globalThis.marker=17;globalThis.lifecycle=()=>marker;(function*(){for(const item of yield [1,2])if(item)throw new Error("expected");});' });
      assert.equal(call(runtime, { operation: 'call', name: 'lifecycle', args: [] }), 17);
      const other = await create();
      assert.equal(call(other, { operation: 'evaluateString', name: 'isolation.js', source: 'typeof marker' }), 'undefined');
    });
    await t.test('preserves synchronous callbacks and multi-frame Unicode values', () => {
      call(runtime, { operation: 'binding', name: 'hostEcho', factory: 'syncBinding' });
      const text = '角色卡🙂'.repeat(150_000);
      call(runtime, { operation: 'evaluateVoid', name: 'echo.js', source: 'globalThis.echo=value=>hostEcho(value);' });
      const result = call(runtime, { operation: 'call', name: 'echo', args: [{ text, values: [null, false, 42], nested: { enabled: true } }] }, (name, args) => {
        assert.equal(name, 'hostEcho');
        assert.equal(args[0].text, text);
        return args[0];
      });
      assert.deepEqual(result, { text, values: [null, false, 42], nested: { enabled: true } });
    });
    await t.test('loads binary WASM through the production worker transport and retains instance memory', async () => {
      const bytes = new Uint8Array(await readFile(new URL('tools/tests/fixtures/wasm/memory.wasm', root)));
      const source = await readFile(new URL('core/crates/plugin/sdk/src/toolpkg/ToolPkgWasmApi.js', root), 'utf8');
      call(runtime, { operation: 'binding', name: '__operitNativeLoadToolPkgWasm', factory: 'syncBinding' });
      let reads = 0;
      const callback = (name, args) => {
        assert.equal(name, '__operitNativeLoadToolPkgWasm'); assert.deepEqual(args, ['memory.fixture', 'state']); reads++;
        return { metadata: { exports: ['next', 'sum'] }, bytes };
      };
      call(runtime, { operation: 'evaluateVoid', name: 'wasm.js', source: source + `
        globalThis.wasmResult=null;globalThis.wasmFailure=null;
        globalThis.api=__operitCreateToolPkgWasmApi(()=>"memory.fixture",false);
        api.open("state").then(instance=>{
          const vector=new Float32Array([1.25,2.5,4]);instance.writeMemory(4096,new Uint8Array(vector.buffer));
          globalThis.wasmResult=[instance.call("next"),instance.call("next"),instance.call("sum",[{type:"i32",value:4096},{type:"i32",value:3}])];
          instance.close();
        }).catch(error=>{globalThis.wasmFailure=String(error);});
      ` }, callback);
      assert.equal(call(runtime, { operation: 'evaluateString', name: 'wasm-result.js', source: 'JSON.stringify({result:wasmResult,error:wasmFailure})' }), '{"result":[1,2,7.75],"error":null}');
      assert.equal(reads, 1);
    });
    await t.test('drains native Promise continuations before releasing command ownership', () => {
      const requests = [];
      call(runtime, { operation: 'binding', name: 'hostAsync', factory: 'binding' });
      call(runtime, { operation: 'evaluateVoid', name: 'promise.js', source: 'globalThis.answer=null;hostAsync("scope",21).then(value=>Promise.resolve(value*2)).then(value=>{answer=value;});' }, (name, args) => {
        requests.push([name, args]);
        return null;
      });
      assert.deepEqual(requests, [['hostAsync', [1, 21]]]);
      call(runtime, { operation: 'settle', args: [1, 21, false] });
      assert.equal(call(runtime, { operation: 'evaluateString', name: 'answer.js', source: 'String(answer)' }), '42');
    });
    await t.test('cancels scoped completions and preserves the realm after script errors', () => {
      call(runtime, { operation: 'evaluateVoid', name: 'cancel.js', source: 'globalThis.cancelCleanup=0;globalThis.cancelReason=null;hostAsync("cancelled").then(()=>{answer=99;},reason=>{cancelReason=reason;}).finally(()=>{cancelCleanup++;});' }, () => null);
      call(runtime, { operation: 'cancel', args: ['cancelled'] });
      assert.equal(call(runtime, { operation: 'evaluateString', name: 'cleanup.js', source: 'String(cancelCleanup)' }), '1');
      assert.equal(call(runtime, { operation: 'evaluateString', name: 'reason.js', source: 'cancelReason' }), 'JavaScript execution was cancelled');
      call(runtime, { operation: 'settle', args: [2, 99, false] });
      assert.equal(call(runtime, { operation: 'evaluateString', name: 'unchanged.js', source: 'String(answer)' }), '42');
      assert.throws(() => call(runtime, { operation: 'evaluateVoid', name: 'throw.js', source: 'throw new Error("script exception");' }), /script exception/);
      assert.equal(call(runtime, { operation: 'evaluateString', name: 'alive.js', source: 'String(marker)' }), '17');
    });
    await t.test('waits for an execution result without periodic wakeups', () => {
      const originalWait = Atomics.wait;
      let waits = 0;
      Atomics.wait = (...args) => { waits++; return originalWait(...args); };
      try {
        call(runtime, { operation: 'evaluateVoid', name: 'finite.js', source: 'const end=Date.now()+100;while(Date.now()<end){}' });
      } finally { Atomics.wait = originalWait; }
      assert.ok(waits <= 4, `expected event notifications, observed ${waits} waits`);
    });
    await t.test('cancellation notifications wake a blocked caller and terminate a busy worker', async () => {
      const control = controlFor(3000);
      const canceller = new NodeWorker(`
        const {workerData,parentPort}=require('node:worker_threads');
        const signal=new Int32Array(workerData);
        parentPort.postMessage('ready');
        setTimeout(()=>{Atomics.store(signal,1,1);Atomics.add(signal,0,1);Atomics.notify(signal,0);},100);
      `, { eval: true, workerData: control.buffer });
      await new Promise(resolve => canceller.once('message', resolve));
      const started = Date.now();
      assert.throws(() => runtime.execute({ operation: 'evaluateVoid', name: 'loop.js', source: 'for(;;){}' }, () => null, control), /interrupted/);
      assert.ok(Date.now() - started < 1000);
      await canceller.terminate();
      assert.throws(() => call(runtime, { operation: 'jobs' }), /destroyed/);
    });
    await t.test('an exact deadline stops a worker without a predicate callback', async () => {
      const timed = await create();
      const started = Date.now();
      assert.throws(() => timed.execute({ operation: 'evaluateVoid', name: 'deadline.js', source: 'for(;;){}' }, () => null, controlFor(100)), /deadline expired/);
      assert.ok(Date.now() - started < 1000);
    });
  } finally {
    for (const runtime of runtimes) runtime.dispose();
    globalThis.Worker = originalWorker;
    globalThis.__operitHost = originalHost;
    delete globalThis.__operitCreateJavaScriptRuntime;
    await rm(directory, { recursive: true, force: true });
  }
});

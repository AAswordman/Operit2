import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import test from "node:test";
import { installScopedHostRuntime } from "./fixtures/scoped_host_runtime.mjs";

const routeSource = readFileSync(new URL("../../core/crates/plugin/sdk/src/js_sdk/toolpkg_route_runtime.js", import.meta.url), "utf8");
const initSource = readFileSync(new URL("../../core/crates/plugin/javascript-bridge/src/javascript/JsInitRuntime.script.js", import.meta.url), "utf8");
const implementedServices = readFileSync(new URL("./fixtures/compiled/toolpkg_route.js", import.meta.url), "utf8");

/** Executes genuine TypeScript decorator output using the production Promise and cancellation runtimes. */
function fixture() {
  const sandbox = vm.createContext({ ToolPkg: {}, exports: {}, /** Receives the active native execution owner. */ __operitNativeActivateCall() {} });
  vm.runInContext(initSource, sandbox);
  const registry = installScopedHostRuntime(sandbox);
  const calls = [], watches = [], reads = [], closed = [], pushes = [], sent = [], pushClosed = [];
  sandbox.remote = false;
  sandbox.__operitNativeShouldRouteToolPkg = () => sandbox.remote;
  sandbox.__operitNativeCallCoreRoute = registry.binding((id, ...args) => calls.push({ id, args }));
  sandbox.__operitNativeOpenCoreRouteWatch = registry.binding((id, ...args) => watches.push({ id, args }));
  sandbox.__operitNativeNextCoreRouteWatch = registry.binding((id, subscription) => reads.push({ id, subscription }));
  sandbox.__operitNativeCloseCoreRouteWatch = subscription => {
    closed.push(subscription);
    const index = reads.findIndex(entry => entry.subscription === subscription);
    if (index >= 0) registry.settle(reads.splice(index, 1)[0].id, null, false);
  };
  sandbox.__operitNativeOpenToolPkgRoutePush = registry.binding((id, ...args) => pushes.push({ id, args }));
  sandbox.__operitNativeSendToolPkgRoutePush = registry.binding((id, ...args) => sent.push({ id, args }));
  sandbox.__operitNativeCloseToolPkgRoutePush = registry.binding((id, ...args) => pushClosed.push({ id, args }));
  vm.runInContext(routeSource + "\n__operitInstallToolPkgRouteApi();\n" + implementedServices, sandbox);
  return { sandbox, registry, calls, watches, reads, closed, pushes, sent, pushClosed,
    /** Executes a normal consumer expression in the service runtime. */
    run(code) { return vm.runInContext(code, sandbox); },
  };
}

/** Drains the actual scoped Promise registry's queued microtasks. */
function tick() { return new Promise(resolve => setImmediate(resolve)); }

/** Calls standard decorated methods locally without allocating transport requests. */
test("implemented async methods retain local bodies and declaring receivers", async () => {
  const f = fixture();
  assert.deepEqual(structuredClone(await f.run("ExampleService.read('chat', 3)")), { chatId: "chat", count: 3 });
  assert.equal(await f.run("instanceService.read('chat')"), "instance:chat");
  assert.equal(f.calls.length, 0);
  assert.equal(f.run("typeof ToolPkg.Async"), "undefined");
  assert.equal(f.run("typeof ToolPkg.RouteCall"), "undefined");
});

/** Derives method identity and positional serialization from a genuine TypeScript decorator. */
test("ordinary external calls automatically use the selected remote implementation", async () => {
  const f = fixture();
  f.sandbox.remote = true;
  const result = f.run("ExampleService.read('remote-chat', 7)");
  assert.deepEqual(structuredClone(f.calls[0].args), ["callToolPkgRoute", "ExampleService.read", { bindingKey: "remote-chat", arguments: ["remote-chat", 7] }]);
  f.registry.settle(f.calls.shift().id, { chatId: "remote-chat", count: 21 }, false);
  assert.equal((await result).count, 21);
  assert.equal(f.run("ExampleService.count"), 0);
  const rejected = f.run("ExampleService.read('remote-chat', 1)");
  f.registry.settle(f.calls.shift().id, "Core permission denied", true);
  await assert.rejects(rejected, /Core permission denied/);
  assert.equal(f.sandbox.refs, 0);
});

/** Invokes the registered original body directly without recursively routing the wrapper. */
test("authorized destination dispatch preserves local implementations and validates Binding", async () => {
  const f = fixture();
  f.sandbox.remote = true;
  assert.equal((await f.run("__operitInvokeToolPkgRouteLocal({ op:'call', bindingKey:'chat', methodName:'ExampleService.read', arguments:['chat', 4] })")).count, 4);
  assert.equal(await f.run("__operitInvokeToolPkgRouteLocal({ op:'call', bindingKey:'chat', methodName:'InstanceService.read', arguments:['chat'] })"), "instance:chat");
  await assert.rejects(f.run("__operitInvokeToolPkgRouteLocal({ op:'call', bindingKey:'forged', methodName:'ExampleService.read', arguments:['chat', 4] })"), /Binding/);
  await assert.rejects(f.run("__operitInvokeToolPkgRouteLocal({ op:'call', bindingKey:'chat', methodName:'ExampleService.hidden', arguments:['chat'] })"), /not decorated/);
  assert.equal(f.calls.length, 0);
});

/** Cancels the pending source wait before generator.return is queued behind next. */
test("local async generator return interrupts an outstanding next", async () => {
  const f = fixture();
  f.run("globalThis.stream = ExampleService.values('chat')");
  assert.equal((await f.run("stream.next()")).value, "chat");
  const pending = f.run("stream.next()");
  await tick();
  assert.equal((await f.run("stream.return()")).done, true);
  assert.equal((await pending).done, true);
  assert.equal(f.sandbox.refs, 0);
});

/** Watches native iterator results without exposing Core event envelopes to plugin code. */
test("remote async generators stream values and release pending reads", async () => {
  const f = fixture();
  f.sandbox.remote = true;
  f.run("globalThis.stream = ExampleService.values('remote')");
  assert.equal(f.watches.length, 0);
  const first = f.run("stream.next()");
  assert.deepEqual(structuredClone(f.watches[0].args), ["watchToolPkgRoute", "ExampleService.values", { bindingKey: "remote", arguments: ["remote"] }]);
  f.registry.settle(f.watches.shift().id, { subscriptionId: "watch-one" }, false);
  await tick();
  f.registry.settle(f.reads.shift().id, { done: false, value: "first" }, false);
  assert.equal((await first).value, "first");
  const pending = f.run("stream.next()");
  await tick();
  await assert.rejects(f.run("stream.next()"), /pending read/);
  assert.equal((await f.run("stream.return()")).done, true);
  assert.equal((await pending).done, true);
  assert.deepEqual(f.closed, ["watch-one"]);
  assert.equal(f.sandbox.refs, 0);
});

/** Keeps unopened collectors free of host resources and closes completion exactly once. */
test("remote collector completion and close-before-open have deterministic ownership", async () => {
  const f = fixture();
  f.sandbox.remote = true;
  await f.run("ExampleService.values('chat').return()");
  assert.equal(f.watches.length, 0);
  f.run("globalThis.stream = ExampleService.values('chat')");
  const next = f.run("stream.next()");
  f.registry.settle(f.watches.shift().id, { subscriptionId: "finished" }, false);
  await tick();
  f.registry.settle(f.reads.shift().id, { done: true, value: null }, false);
  assert.equal((await next).done, true);
  assert.equal((await f.run("stream.next()")).done, true);
  assert.deepEqual(f.closed, ["finished"]);
});

/** Preserves ordinary async input syntax while awaiting each Link acknowledgement. */
test("async input automatically selects Push and retains producer backpressure", async () => {
  const f = fixture();
  f.sandbox.remote = true;
  const completed = f.run("ExampleService.consume('chat', (async function*(){ yield 2; yield 5; })())");
  assert.deepEqual(structuredClone(f.pushes[0].args), ["ExampleService.consume", { bindingKey: "chat", arguments: ["chat", null], inputIndex: 1 }]);
  f.registry.settle(f.pushes.shift().id, { subscriptionId: "push-one" }, false);
  await tick();
  assert.deepEqual(f.sent.map(item => structuredClone(item.args)), [["push-one", 2]]);
  await tick();
  assert.equal(f.sent.length, 1);
  f.registry.settle(f.sent.shift().id, null, false);
  await tick();
  assert.deepEqual(structuredClone(f.sent[0].args), ["push-one", 5]);
  f.registry.settle(f.sent.shift().id, null, false);
  await tick();
  assert.deepEqual(structuredClone(f.pushClosed[0].args), ["push-one", false]);
  f.registry.settle(f.pushClosed.shift().id, null, false);
  assert.equal(await completed, undefined);
  assert.equal(f.sandbox.refs, 0);
});

/** Opens an actual implemented input consumer and preserves its rejection for the producer. */
test("destination Push executes the local for-await implementation and propagates errors", async () => {
  const f = fixture();
  const id = await f.run("__operitInvokeToolPkgRouteLocal({op:'pushOpen', bindingKey:'chat', methodName:'ExampleService.consume', arguments:['chat',null], inputIndex:1})");
  f.sandbox.id = id;
  await f.run("__operitInvokeToolPkgRouteLocal({op:'pushItem',id,value:3})");
  await f.run("__operitInvokeToolPkgRouteLocal({op:'pushClose',id})");
  assert.equal(f.run("ExampleService.count"), 3);
  f.sandbox.id = await f.run("__operitInvokeToolPkgRouteLocal({op:'pushOpen', bindingKey:'chat', methodName:'ExampleService.consume', arguments:['chat',null], inputIndex:1})");
  await f.run("__operitInvokeToolPkgRouteLocal({op:'pushItem',id,value:-1})");
  await tick();
  await assert.rejects(f.run("__operitInvokeToolPkgRouteLocal({op:'pushClose',id})"), /negative/);
  assert.equal(f.run("__operitToolPkgRoutes.streams.size"), 0);
});

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import test from "node:test";

const bootstrap = readFileSync(new URL("../../core/crates/plugin/javascript-bridge/src/javascript/JsInitRuntime.script.js", import.meta.url), "utf8");

/** Exercises the real call registry and Promise continuation bridge without a permissive fake activator. */
test("shared Promise subscribers retain independent owners through fulfillment, rejection and nested continuations", async () => {
  for (const rejected of [false, true]) {
    const sandbox = vm.createContext({ /** Receives the active native execution owner. */ __operitNativeActivateCall() {} });
    vm.runInContext(bootstrap, sandbox);
    const results = vm.runInContext(`
      var resolveShared, rejectShared;
      var shared = new Promise(function(resolve, reject) { resolveShared = resolve; rejectShared = reject; });
      function subscribe(owner) {
        var state = __operitRegisterCallSession(owner, {});
        state.callRuntime = { callId: owner };
        __operitActivateCall(owner);
        return shared.then(function(value) { return value; }, function(error) { return error.message; })
          .then(function(value) {
            return Promise.resolve().then(function() {
              return { expected: owner, actual: __operitCurrentCallId, runtime: __operit_call_runtime_ref.callId, value: value };
            });
          });
      }
      var first = subscribe('initializer'), second = subscribe('consumer');
      __operitRegisterCallSession('unrelated-hook', {});
      __operitActivateCall('unrelated-hook');
      __operitCleanupCallSession('unrelated-hook');
      ${rejected ? "rejectShared(new Error('original failure'));" : "resolveShared('shared service');"}
      Promise.all([first, second]);
    `, sandbox);
    assert.deepEqual(JSON.parse(JSON.stringify(await results)), ["initializer", "consumer"].map(owner => ({
      expected: owner, actual: owner, runtime: owner, value: rejected ? "original failure" : "shared service",
    })));
  }
});

/** Cancelling the owner cannot lend the continuation another live call's credentials. */
test("cancelled callbacks preserve their own identity and restore the outer call even after throwing", async () => {
  const sandbox = vm.createContext({ /** Receives the active native execution owner. */ __operitNativeActivateCall() {} });
  vm.runInContext(bootstrap, sandbox);
  const result = vm.runInContext(`
    var resolveCancelled;
    var pending = new Promise(function(resolve) { resolveCancelled = resolve; });
    var cancelled = __operitRegisterCallSession('cancelled-owner', {});
    cancelled.callRuntime = { callId: 'cancelled-owner' };
    __operitActivateCall('cancelled-owner');
    var completion = pending.then(function() {
      if (__operitCurrentCallId !== 'cancelled-owner' || __operitGetCallState(__operitCurrentCallId) !== null || typeof __operit_call_runtime_ref !== 'undefined') {
        throw new Error('Callback borrowed another live session');
      }
      throw new Error('original cancelled callback failure');
    });
    __operitCancelCallSession('cancelled-owner');
    var outer = __operitRegisterCallSession('outer-owner', {});
    outer.callRuntime = { callId: 'outer-owner' };
    __operitActivateCall('outer-owner');
    resolveCancelled();
    completion;
  `, sandbox);
  await assert.rejects(result, /original cancelled callback failure/u);
  assert.equal(vm.runInContext("__operitCurrentCallId", sandbox), "outer-owner");
  assert.equal(vm.runInContext("__operit_call_runtime_ref === outer.callRuntime", sandbox), true);
  assert.equal(vm.runInContext("__operitGetCallState('cancelled-owner')", sandbox), null);
});

/** Fulfillment, rejection and nested subscriptions retain cancelled ownership without runtime access. */
test("cancelled fulfillment and rejection keep their identity through nested continuations", async () => {
  for (const rejected of [false, true]) {
    const sandbox = vm.createContext({ /** Receives the active native execution owner. */ __operitNativeActivateCall() {} });
    vm.runInContext(bootstrap, sandbox);
    const result = vm.runInContext(`
      var settle;
      var pending = new Promise(function(resolve, reject) { settle = ${rejected ? 'reject' : 'resolve'}; });
      var reports = [];
      var cancelled = __operitRegisterCallSession('cancelled-owner', {});
      cancelled.callRuntime = { callId: 'cancelled-owner' };
      __operitActivateCall('cancelled-owner');
      /** Records the actual callback identity and runtime availability. */
      function record(value) {
        reports.push({ callId: __operitCurrentCallId, hasSession: __operitGetCallState(__operitCurrentCallId) !== null,
          hasRuntime: typeof __operit_call_runtime_ref !== 'undefined', value: value });
        return value;
      }
      /** Subscribes another callback while the cancelled identity owns the continuation. */
      function continuation(value) {
        record(value);
        return Promise.resolve(value).then(record);
      }
      var completion = pending.then(continuation, continuation);
      __operitCancelCallSession('cancelled-owner');
      var outer = __operitRegisterCallSession('outer-owner', {});
      outer.callRuntime = { callId: 'outer-owner' };
      __operitActivateCall('outer-owner');
      settle('settled');
      completion;
    `, sandbox);
    assert.equal(await result, "settled");
    assert.deepEqual(JSON.parse(vm.runInContext("JSON.stringify(reports)", sandbox)), [
      { callId: "cancelled-owner", hasSession: false, hasRuntime: false, value: "settled" },
      { callId: "cancelled-owner", hasSession: false, hasRuntime: false, value: "settled" },
    ]);
    assert.equal(vm.runInContext("__operitCurrentCallId", sandbox), "outer-owner");
    assert.equal(vm.runInContext("__operit_call_runtime_ref === outer.callRuntime", sandbox), true);
  }
});

/** A cancelled continuation cannot restore an outer runtime cancelled during that callback. */
test("cancelled callbacks do not restore an outer session removed during execution", async () => {
  const sandbox = vm.createContext({ /** Receives the active native execution owner. */ __operitNativeActivateCall() {} });
  vm.runInContext(bootstrap, sandbox);
  const result = vm.runInContext(`
    var resolvePending;
    var pending = new Promise(function(resolve) { resolvePending = resolve; });
    __operitRegisterCallSession('cancelled-owner', {});
    __operitActivateCall('cancelled-owner');
    var completion = pending.then(function() {
      __operitCancelCallSession('outer-owner');
      return 'completed';
    });
    __operitCancelCallSession('cancelled-owner');
    var outer = __operitRegisterCallSession('outer-owner', {});
    outer.callRuntime = { callId: 'outer-owner' };
    __operitActivateCall('outer-owner');
    resolvePending();
    completion;
  `, sandbox);
  assert.equal(await result, "completed");
  assert.equal(vm.runInContext("__operitCurrentCallId", sandbox), "");
  assert.equal(vm.runInContext("typeof __operit_call_runtime_ref", sandbox), "undefined");
  assert.equal(vm.runInContext("__operitGetCallState('outer-owner')", sandbox), null);
});

/** Completing one asynchronous hook preserves the context restored by its finalization. */
test("completed callbacks preserve the surviving call context selected by finalization", async () => {
  const sandbox = vm.createContext({ /** Receives the active native execution owner. */ __operitNativeActivateCall() {} });
  vm.runInContext(bootstrap, sandbox);
  const result = vm.runInContext(`
    var outer = __operitRegisterCallSession('suspended-owner', {});
    outer.callRuntime = { callId: 'suspended-owner' };
    __operitActivateCall('suspended-owner');
    var hook = __operitRegisterCallSession('hook', {});
    hook.callRuntime = { callId: 'hook' };
    hook.previousCallId = 'suspended-owner';
    hook.previousCallRuntime = outer.callRuntime;
    __operitActivateCall('hook');
    Promise.resolve().then(function() {
      __operitCancelCallSession('hook');
      return 'hook completed';
    });
  `, sandbox);
  assert.equal(await result, "hook completed");
  assert.equal(vm.runInContext("__operitCurrentCallId", sandbox), "suspended-owner");
  assert.equal(vm.runInContext("__operit_call_runtime_ref === outer.callRuntime", sandbox), true);
  assert.equal(vm.runInContext("__operitGetCallState('hook')", sandbox), null);
});

/** Native await must resume under the host request owner while another call remains live. */
test("native await retains the host completion owner across an overlapping hook", async () => {
  for (const rejected of [false, true]) {
    const sandbox = vm.createContext({ /** Receives the active native execution owner. */ __operitNativeActivateCall() {} });
    vm.runInContext(bootstrap, sandbox);
    for (const path of ["../../hosts/web/src/javascript_promises.js", "../../core/crates/plugin/sdk/src/JsExecutionRuntimeBridge.script.js"]) {
      vm.runInContext(readFileSync(new URL(path, import.meta.url), "utf8"), sandbox);
    }
    const result = vm.runInContext(`
      var request;
      __operitNativeCallToolStructured = __operitHostPromiseRegistry.binding(function(id) { request = id; });
      var owner = __operitRegisterCallSession('host-owner', {});
      owner.callRuntime = { callId: 'host-owner' };
      __operitActivateCall('host-owner');
      var completion = (async function() {
        var value;
        try { value = await toolCall('echo', {}); }
        catch (error) { value = error.message; }
        return { owner: __operitCurrentCallId, runtime: __operit_call_runtime_ref.callId,
          hasState: __operitGetCallState(__operitCurrentCallId) === owner, value: value };
      })();
      var hook = __operitRegisterCallSession('overlapping-hook', {});
      hook.callRuntime = { callId: 'overlapping-hook' };
      __operitActivateCall('overlapping-hook');
      __operitHostPromiseRegistry.settle(request, ${rejected ? "'host failure', true" : "{success: true, data: 'host result'}, false"});
      completion;
    `, sandbox);
    assert.deepEqual(JSON.parse(JSON.stringify(await result)), {
      owner: "host-owner", runtime: "host-owner", hasState: true, value: rejected ? "host failure" : "host result",
    });
  }
});

/** Restoring a surviving call must read its runtime from the same live registry entry. */
test("retiring nested hooks never restores the runtime of an already retired parent", () => {
  const sandbox = vm.createContext({ /** Receives the active native execution owner. */ __operitNativeActivateCall() {} });
  vm.runInContext(bootstrap, sandbox);
  const report = vm.runInContext(`
    var outer = __operitRegisterCallSession('outer', {});
    outer.callRuntime = { callId: 'outer' };
    __operitActivateCall('outer');
    var parent = __operitRegisterCallSession('parent-hook', {});
    parent.callRuntime = { callId: 'parent-hook' };
    parent.previousCallId = 'outer'; parent.previousCallRuntime = outer.callRuntime;
    __operitActivateCall('parent-hook');
    var nested = __operitRegisterCallSession('nested-hook', {});
    nested.callRuntime = { callId: 'nested-hook' };
    nested.previousCallId = 'parent-hook'; nested.previousCallRuntime = parent.callRuntime;
    __operitActivateCall('nested-hook');
    __operitCancelCallSession('parent-hook');
    __operitCancelCallSession('nested-hook');
    ({ callId: __operitCurrentCallId, runtime: __operit_call_runtime_ref.callId });
  `, sandbox);
  assert.deepEqual(structuredClone(report), { callId: "outer", runtime: "outer" });
});

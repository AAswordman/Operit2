import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import test from "node:test";

const bootstrap = readFileSync(new URL("../../core/crates/plugin/javascript-bridge/src/javascript/JsInitRuntime.script.js", import.meta.url), "utf8");

/** Exercises the real call registry and Promise continuation bridge without a permissive fake activator. */
test("shared Promise subscribers retain independent owners through fulfillment, rejection and nested continuations", async () => {
  for (const rejected of [false, true]) {
    const sandbox = vm.createContext({});
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
  const sandbox = vm.createContext({});
  vm.runInContext(bootstrap, sandbox);
  const result = vm.runInContext(`
    var resolveCancelled;
    var pending = new Promise(function(resolve) { resolveCancelled = resolve; });
    __operitRegisterCallSession('cancelled-owner', {});
    __operitActivateCall('cancelled-owner');
    var completion = pending.then(function() {
      if (__operitCurrentCallId !== 'cancelled-owner' || __operitGetCallState(__operitCurrentCallId) !== null) {
        throw new Error('Callback borrowed another live session');
      }
      throw new Error('original cancelled callback failure');
    });
    __operitCancelCallSession('cancelled-owner');
    __operitRegisterCallSession('outer-owner', {});
    __operitActivateCall('outer-owner');
    resolveCancelled();
    completion;
  `, sandbox);
  await assert.rejects(result, /original cancelled callback failure/u);
  assert.equal(vm.runInContext("__operitCurrentCallId", sandbox), "outer-owner");
});

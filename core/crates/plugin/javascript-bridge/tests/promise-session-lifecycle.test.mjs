import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import vm from "node:vm";

const root = new URL("../../../../../", import.meta.url);
/** Reads the current shared runtime without compiling or substituting its lifecycle implementation. */
function source(relative) { return readFileSync(new URL(relative, root), "utf8").replace(/\r\n/g, "\n"); }
const bootstrap = source("core/crates/plugin/javascript-bridge/src/javascript/JsLibraries.rs");
/** Extracts an actual bootstrap function using its exact adjacent production declaration. */
function declaration(startMarker, endMarker) {
  const start = bootstrap.indexOf(startMarker), end = bootstrap.indexOf(endMarker, start);
  assert.ok(start >= 0 && end > start, startMarker);
  return bootstrap.slice(start, end).replaceAll("{{", "{").replaceAll("}}", "}");
}
const finalize = declaration("            function finalizeCall()", "            function isActive()");
const complete = declaration("            function completeCall(resultValue)", "            /** Reports an explicit structured failure");

/** Runs production session ownership and completion with controlled native result and activation boundaries. */
function environment() {
  const results = [], notifications = [], activations = [];
  const context = vm.createContext({
    /** Rejects attempts to execute continuations of an explicitly cancelled session. */
    __operitNativeActivateCall(id, cancelled) { activations.push({ id, cancelled }); if (cancelled) throw new Error("cancelled owner: " + id); },
    /** Records the sole primary result without supplying any background execution behavior. */
    __operitNativeSetCallStructuredResult(id, value) { results.push({ id, value: structuredClone(value) }); },
    /** Records native notification of a genuinely retained session. */
    __operitNativeNotifyDetachedCall(id) { notifications.push(id); },
    /** Leaves tracing outside these session-ownership assertions. */
    __operitNativeLogJsExecutionTrace() {},
    __operitText: String,
  });
  vm.runInContext(source("core/crates/plugin/javascript-bridge/src/javascript/JsInitRuntime.script.js"), context);
  return {
    context, results, notifications, activations,
    /** Evaluates a controlled task in the real production session realm. */
    run(code) { return vm.runInContext(code, context); },
    /** Installs the real completion functions inside one exact call's lexical scope. */
    start(id) {
      return vm.runInContext(`(function() {
        var callId = ${JSON.stringify(id)}, previousCallId = '', structuredResult = true;
        var callState = __operitRegisterCallSession(callId, {});
        callState.callRuntime = {};
        /** Reads the exact owning call throughout primary completion. */
        function getCallState() { return __operitGetCallState(callId); }
        ${finalize}
        ${complete}
        __operitCurrentCallId = callId;
        return { finish: completeCall };
      })()`, context);
    },
  };
}

/** Reproduces a consumed submit whose queued work has not reached its first host call. */
test("primary completion retains queued background reactions without a timer", async () => {
  const env = environment(), call = env.start("group-submit");
  env.run(`
    var releaseWork;
    var gate = new Promise(/** Controls the genuine queued work boundary. */ function(resolve) { releaseWork = resolve; });
    var backgroundOwner;
    var work = gate.then(/** Records the owner of actual delivered background work. */ function(value) {
      backgroundOwner = __operitCurrentCallId;
      return value;
    });
  `);
  call.finish({ action: "Consume" });
  assert.equal(env.context.__operitGetCallState("group-submit").detached, true);
  assert.equal(env.context.__operitGetCallState("group-submit").pendingReferences, 1);
  assert.deepEqual(env.notifications, ["group-submit"]);
  env.run("releaseWork('settled')");
  assert.equal(await env.run("work"), "settled");
  assert.equal(env.context.backgroundOwner, "group-submit");
  assert.equal(env.context.__operitGetCallState("group-submit").pendingReferences, 0);
  assert.equal(env.context.__operitFinalizeDetachedCall("group-submit"), true);
  assert.equal(env.context.__operitGetCallState("group-submit"), null);
  assert.deepEqual(env.results, [{ id: "group-submit", value: { action: "Consume" } }]);
});

/** Preserves Promise rejection forwarding and releases both the absent and explicit reaction branches. */
test("rejected background work preserves the original failure and releases ownership", async () => {
  const env = environment(), call = env.start("group-failure");
  env.run(`
    var rejectWork;
    var gate = new Promise(/** Captures explicit failure at the controlled task boundary. */ function(_resolve, reject) { rejectWork = reject; });
    var original = new Error('original planner failure');
    var work = gate.then().catch(/** Records the unchanged background rejection. */ function(error) { return error; });
  `);
  call.finish({ action: "Consume" });
  env.run("rejectWork(original)");
  assert.equal(await env.run("work"), env.context.original);
  assert.equal(env.context.__operitGetCallState("group-failure").pendingReferences, 0);
  assert.equal(env.context.__operitFinalizeDetachedCall("group-failure"), true);
  assert.equal(env.results.length, 1);
});

/** Ensures a throwing reaction cannot leave a permanent retained reference or replace its original error. */
test("a throwing continuation releases its subscription exactly once", async () => {
  const env = environment(), call = env.start("throwing-reaction");
  env.run(`
    var original = new Error('continuation failure');
    var work = Promise.resolve().then(/** Throws the actual task failure. */ function() { throw original; });
  `);
  call.finish({ action: "Consume" });
  await assert.rejects(env.run("work"), /** Verifies the original error identity. */ error => error === env.context.original);
  assert.equal(env.context.__operitGetCallState("throwing-reaction").pendingReferences, 0);
  assert.equal(env.context.__operitFinalizeDetachedCall("throwing-reaction"), true);
});

/** Keeps each simultaneous submission owned independently while the original caller has already completed. */
test("interleaved background tasks retain their own sessions", async () => {
  const env = environment(), first = env.start("first");
  env.run("var releaseFirst; var a = new Promise(/** Suspends the first task. */ function(resolve) { releaseFirst = resolve; }).then(/** Reads the first owner. */ function() { return __operitCurrentCallId; });");
  first.finish({ action: "Consume" });
  const second = env.start("second");
  env.run("var releaseSecond; var b = new Promise(/** Suspends the second task. */ function(resolve) { releaseSecond = resolve; }).then(/** Reads the second owner. */ function() { return __operitCurrentCallId; });");
  second.finish({ action: "Consume" });
  env.run("releaseFirst()");
  assert.equal(await env.run("a"), "first");
  assert.equal(env.context.__operitFinalizeDetachedCall("first"), true);
  assert.ok(env.context.__operitGetCallState("second").pendingReferences > 0);
  env.run("releaseSecond()");
  assert.equal(await env.run("b"), "second");
  assert.equal(env.context.__operitFinalizeDetachedCall("second"), true);
  assert.equal(env.results.length, 2);
});

/** Releases the subscription even when the intrinsic rejects an invalid Promise receiver synchronously. */
test("invalid Promise receivers cannot leak an execution reference", () => {
  const env = environment(); env.start("invalid-receiver");
  assert.throws(/** Invokes the real intrinsic with an invalid receiver. */ () => env.run("Promise.prototype.then.call({})"));
  assert.equal(env.context.__operitGetCallState("invalid-receiver").pendingReferences, 0);
});

/** Keeps explicit cancellation authoritative after primary completion without reactivating a retired task. */
test("cancelled background work cannot run its subscribed handler", async () => {
  const env = environment(), call = env.start("cancelled-task");
  env.run(`
    var releaseWork, ran = false;
    var work = new Promise(/** Suspends work until explicit cancellation. */ function(resolve) { releaseWork = resolve; })
      .then(/** Marks a forbidden continuation after cancellation. */ function() { ran = true; });
  `);
  call.finish({ action: "Consume" });
  env.context.__operitCancelCallSession("cancelled-task");
  env.run("releaseWork()");
  await assert.rejects(env.run("work"), { message: "cancelled owner: cancelled-task" });
  assert.equal(env.context.ran, false);
  assert.equal(env.context.__operitGetCallState("cancelled-task"), null);
  assert.equal(env.results.length, 1);
});

/** Leaves value-only forwarding transparent even after its subscribing call has been cancelled. */
test("absent handlers forward settlement without activating a cancelled call", async () => {
  const env = environment(), call = env.start("forwarded-task");
  env.run("var releaseWork; var work = new Promise(/** Suspends a value-only chain. */ function(resolve) { releaseWork = resolve; }).then();");
  call.finish({ action: "Consume" });
  env.context.__operitCancelCallSession("forwarded-task");
  env.run("releaseWork('unchanged value')");
  assert.equal(await env.run("work"), "unchanged value");
  assert.equal(env.activations.some(/** Detects an unnecessary cancelled-owner activation. */ entry => entry.cancelled), false);
});

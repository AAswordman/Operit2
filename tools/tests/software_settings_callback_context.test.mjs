import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import test from "node:test";

const source = readFileSync(new URL("../../core/crates/plugin/codegen/src/runtime_bindings.rs", import.meta.url), "utf8");
const start = source.indexOf("function __operitReadSoftwareSettingsDirectory(method)");
const end = source.indexOf("/** Validates complete JSON objects", start);

/** Evaluates the actual generated callback helpers; native transports only record pending requests. */
function context() {
  const callbacks = [];
  const sandbox = vm.createContext({ callbacks, __operitCurrentCallId: "owner",
    /** Records the native callback identity without supplying directory data or a model. */
    __operitNativeReadSoftwareSettingsDirectoryAsync(callbackId, method) { callbacks.push({ callbackId, method }); },
    /** Records the declared setter request without mutating a configuration. */
    __operitNativeApplySoftwareSettingsConfigAsync(callbackId, method) { callbacks.push({ callbackId, method }); },
  });
  vm.runInContext("globalThis.__operitActivateCall = function(id) { globalThis.__operitCurrentCallId = id; };", sandbox);
  vm.runInContext(source.slice(start, end), sandbox);
  return { sandbox, callbacks };
}

/** A completed unrelated hook must not own the awaiting directory caller's continuation. */
test("directory and configuration callbacks restore their exact originating JS call before resuming", async () => {
  for (const call of ["__operitReadSoftwareSettingsDirectory('listModelSummaries')", "__operitApplySoftwareSettingsConfig('setCurrentTtsConfigId', 'explicit-config')"]) {
    const { sandbox, callbacks } = context();
    const result = vm.runInContext(`(async () => { try { await ${call}; } catch (error) { return { owner: __operitCurrentCallId, message: error.message }; } throw new Error('The rejection-only transport unexpectedly succeeded'); })()`, sandbox);
    assert.equal(callbacks.length, 1);
    vm.runInContext("__operitCurrentCallId = 'finished-unrelated-hook';", sandbox);
    sandbox[sandbox.callbacks[0].callbackId](JSON.stringify({ message: "exact host failure" }), true);
    const actual = JSON.parse(JSON.stringify(await result));
    assert.deepEqual(actual, { owner: "owner", message: "exact host failure" });
  }
});

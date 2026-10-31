import { installScopedHostRuntime, installGeneratedToolsRuntime } from "../../../../../tools/tests/fixtures/scoped_host_runtime.mjs";
import assert from "node:assert/strict";
import test from "node:test";
import vm from "node:vm";

const payload = {
  operation: { operation: "write", path: "/sdcard/USER.md", successful: true, details: "Written" },
  aiDiffInstructions: "", diffContent: "@@ -0,0 +1 @@\n+Profile",
};

/** Installs the real generated binding over a controlled structured host completion. */
function runtime(version = "2.0.0", error = null) {
  const calls = [], context = vm.createContext({ console });
  const registry = installScopedHostRuntime(context);
  context.__operitNativeCallToolStructured = registry.binding(
    /** Returns the executor's declared envelope through the production scoped transport. */
    (id, type, name, parameters) => {
      calls.push({ type, name, parameters: structuredClone(parameters) });
      if (error !== null) registry.settle(id, error, true);
      else registry.settle(id, registry.snapshotResult({ success: true, data: payload }), false);
    },
  );
  installGeneratedToolsRuntime(context, version);
  return { context, calls };
}

test("Files.write and direct write_file preserve the same operation and diff envelope", async () => {
  const { context, calls } = runtime();
  const written = await vm.runInContext("Tools.Files.write('/sdcard/USER.md', 'Profile', false)", context);
  const direct = await vm.runInContext("toolCall('write_file', {path:'/sdcard/USER.md',content:'Profile',append:false})", context);
  assert.deepEqual(structuredClone(written), payload);
  assert.deepEqual(structuredClone(direct), payload);
  assert.deepEqual(calls.map(call => call.name), ["write_file", "write_file"]);
  assert.deepEqual(calls[0].parameters, { path: "/sdcard/USER.md", content: "Profile", append: false });
  assert.equal(context.refs, 0);
});

test("the existing v1 adapter projects only its declared flat file operation", async () => {
  const { context, calls } = runtime("1.0.0");
  const written = await vm.runInContext("Tools.Files.write('/sdcard/USER.md', 'Profile', false, 'android')", context);
  assert.deepEqual(structuredClone(written), { ...payload.operation, env: "android" });
  assert.equal(calls.length, 1);
  assert.equal(context.refs, 0);
});

test("Files.write preserves host failure without replacing it with another result", async () => {
  const { context, calls } = runtime("2.0.0", "Original write failure");
  await assert.rejects(vm.runInContext("Tools.Files.write('/sdcard/USER.md', '', false)", context), /Original write failure/);
  assert.equal(calls.length, 1);
  assert.equal(context.refs, 0);
});

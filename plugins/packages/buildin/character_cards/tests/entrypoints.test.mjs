import assert from "node:assert/strict";
import test from "node:test";
import { importedCallCount, plain } from "./runtime.mjs";
import { createDiskHarness } from "./disk-files.mjs";
import { openPlugin } from "./plugin-entry-harness.mjs";
import { captureMainHookExports } from "./registration-runtime.mjs";
import { checkRouting } from "../scripts/check.mjs";

/** Checks real TypeScript import/call edges while explicitly leaving live persistence acceptance separate. */
test("command API and UI entrypoints invoke the same plugin-owned service module", () => {
  assert.ok(importedCallCount("../src/commands.ts", "./service-runtime", "dispatchDomain") > 0);
  assert.ok(importedCallCount("../src/public-api.ts", "./service-runtime", "dispatchDomain") > 0);
  assert.ok(importedCallCount("../src/host.ts", "./service-runtime", "dispatch") > 0);
});

/** Reuses the routing owner's static source and catalog fixture checks, not a second provider implementation. */
test("generic routing source contracts include explicit unknown disabled and provider errors", () => {
  assert.equal(checkRouting(), 0, "Routing static/catalog checks failed; Rust and production plugin services were not executed");
});


/** Executes real main registration and SDK export serialization without running the background callbacks or synthesizing persistence. */
test("actual main uniquely registers lazy memory enqueue and 60000ms interval hooks with resolvable exported callbacks", async t => {
  const disk = await createDiskHarness(t), plugin = openPlugin(disk);
  assert.equal(importedCallCount("../src/main.ts", "./memory-jobs/hooks", "registerMemoryJobHooks"), 1);
  assert.equal(plugin.chatMessageHooks.length, 1);
  assert.equal(plugin.hostEventHooks.length, 1);
  const message = plugin.chatMessageHooks[0], interval = plugin.hostEventHooks[0];
  assert.deepEqual(Object.keys(message).sort(), ["function", "id"]);
  assert.equal(message.id, "memory-candidate-enqueue");
  assert.equal(typeof message.function, "function");
  assert.equal(message.function, plugin.main.onMemoryMessagePersisted);
  assert.deepEqual(Object.keys(interval).sort(), ["function", "id", "source", "trigger"]);
  assert.deepEqual(plain(interval), { id: "memory-jobs-interval", source: "interval", trigger: { kind: "interval", intervalMs: 60000 } });
  assert.equal(typeof interval.function, "function");
  assert.equal(interval.function, plugin.main.onMemoryInterval);
  assert.deepEqual(Object.entries(plugin.main).filter(
    /** Requires one exact durable main-module export for the captured enqueue callback. */
    entry => entry[1] === message.function,
  ).map(
    /** Preserves the actual export name used by the SDK instead of inventing a handler lookup. */
    entry => entry[0],
  ), ["onMemoryMessagePersisted"]);
  assert.deepEqual(Object.entries(plugin.main).filter(
    /** Requires one exact durable main-module export for the captured interval callback. */
    entry => entry[1] === interval.function,
  ).map(
    /** Reads the registered interval callback's actual module export name. */
    entry => entry[0],
  ), ["onMemoryInterval"]);
  const captured = captureMainHookExports(plugin.main, plugin.chatMessageHooks, plugin.hostEventHooks);
  assert.deepEqual(captured.chatMessageHooks, [{ id: "memory-candidate-enqueue", function: "onMemoryMessagePersisted" }]);
  assert.deepEqual(captured.hostEventHooks, [{ id: "memory-jobs-interval", source: "interval", trigger: { kind: "interval", intervalMs: 60000 }, function: "onMemoryInterval" }]);
  assert.equal(plugin.main[captured.chatMessageHooks[0].function], message.function);
  assert.equal(plugin.main[captured.hostEventHooks[0].function], interval.function);
  assert.equal(disk.calls.length, 0, "Registering and resolving callback exports must not initialize storage or execute memory jobs");
  assert.deepEqual(await disk.entries(), [], "Real plugin registration must leave the new plugin directory empty");
});

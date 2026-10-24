import assert from "node:assert/strict";
import test from "node:test";
import { createDiskHarness } from "./disk-files.mjs";
import { loadModule, plain } from "./runtime.mjs";

/** Connects the canonical public client to actual registered provider handlers and real SQLite storage. */
async function environment(t) {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }), registrations = new Map();
  const provider = loadModule("src/public-api.ts", { ...disk.globals, ToolPkg: { ...disk.globals.ToolPkg,
    /** Captures only the public registry; production callbacks implement all business behavior. */
    registerApi(definition) { registrations.set(definition.name, definition.function); },
  } });
  provider.registerDomainApis();
  const client = loadModule("src/api.ts", {
    ToolPkg: {
      /** Forwards the exact public dependency envelope without substituting provider data. */
      async callDependency(packageId, method, payload) {
        assert.equal(packageId, "com.operit.character_cards");
        return registrations.get(method)({ payload: plain(payload), callerPackage: "com.operit.test_consumer" });
      },
    },
  }).characterCards;
  return { disk, client };
}

/** Captures the built-in prompt finalize hook from its real registration call. */
function recallHook(disk) {
  let registered = null;
  const module = loadModule("src/memory-recall.ts", { ...disk.globals, ToolPkg: { ...disk.globals.ToolPkg,
    registerPromptFinalizeHook(definition) { registered = definition; },
  } });
  module.registerMemoryRecallHook();
  assert.equal(registered.id, "character-memory-recall");
  return registered.function;
}

/** Builds one finalize event with the exact generic execution context the host supplies. */
function finalizeEvent(payload) {
  return { eventName: "prompt_finalize", eventPayload: { stage: "before_send_to_model", ...payload } };
}

const RECALL_EVENT = {
  processedInput: "What is the stable fact?",
  metadata: { executionContext: { chatId: "chat-recall", participantId: "default" } },
};

/** Requires structural skips to stay free of any business storage or provider call. */
test("prompt finalize hook skips non-character requests without touching storage", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" });
  const hook = recallHook(disk), before = disk.stateBytes();
  assert.equal(await hook(finalizeEvent({ stage: "before_finalize_prompt", ...RECALL_EVENT })), null);
  assert.equal(await hook(finalizeEvent({ processedInput: "hello", metadata: { executionContext: { chatId: "chat-1", participantId: null } } })), null);
  assert.equal(await hook(finalizeEvent({ processedInput: "hello", metadata: { executionContext: { chatId: "chat-1", participantId: "model:chat" } } })), null);
  assert.equal(await hook(finalizeEvent({ processedInput: "", ...RECALL_EVENT })), null);
  assert.equal(await hook(finalizeEvent({ processedInput: '<memory_context source="character-memory">kept</memory_context>', ...RECALL_EVENT })), null);
  assert.deepEqual(disk.stateBytes(), before);
});

/** Checks real retrieval, exact block composition, and the per-chat provider snapshot behavior. */
test("recall injects matched participant memories once per chat snapshot", async t => {
  const env = await environment(t);
  // The lexical channel ranks titles (and document chunks), matching the Kotlin reference behavior.
  await env.client.memory.create({ ownerKey: "character:default", values: { title: "Stable fact", content: "Known durable knowledge" } });
  const hook = recallHook(env.disk);
  const first = await hook(finalizeEvent(RECALL_EVENT));
  assert.match(first, /<memory_context source="character-memory">/);
  assert.match(first, /Known durable knowledge/);
  assert.match(first, /标题: Stable fact/);
  assert.ok(first.startsWith("What is the stable fact?"));
  assert.equal(await hook(finalizeEvent(RECALL_EVENT)), null);
  const otherChat = await hook(finalizeEvent({ ...RECALL_EVENT, metadata: { executionContext: { chatId: "chat-other", participantId: "default" } } }));
  assert.match(otherChat, /Known durable knowledge/);
  const unknownParticipant = await hook(finalizeEvent({ ...RECALL_EVENT, metadata: { executionContext: { chatId: "chat-recall", participantId: "missing-participant" } } }));
  assert.equal(unknownParticipant, null);
});

/** Requires the persisted switch to gate injection before any retrieval happens. */
test("disabled recall settings stop injection without any memory query", async t => {
  const env = await environment(t);
  await env.client.memory.create({ ownerKey: "character:default", values: { title: "Stable fact", content: "Known durable knowledge" } });
  env.disk.replaceState({ ...plain(env.disk.readState()), memoryRecall: { enabled: false } });
  const hook = recallHook(env.disk);
  assert.equal(await hook(finalizeEvent(RECALL_EVENT)), null);
});

/** Verifies the transactional state path persists the switch and released states default to enabled. */
test("recall settings persist through the transactional state path and default to enabled", async t => {
  const disk = await createDiskHarness(t);
  const database = loadModule("src/storage/database.ts", disk.globals).DatabaseCharacterRepository;
  const owner = await database.open();
  assert.deepEqual(plain(await owner.readMemoryRecallSettings()), { enabled: true });
  await owner.run(session => session.writeMemoryRecallSettings(false));
  const reopened = await database.open();
  assert.deepEqual(plain(await reopened.readMemoryRecallSettings()), { enabled: false });
  assert.equal(plain(disk.readState()).memoryRecall.enabled, false);
});

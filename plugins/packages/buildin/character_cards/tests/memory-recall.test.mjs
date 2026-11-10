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

/** Captures the built-in prompt input hook from its real registration call. */
function recallHook(disk) {
  let registered = null;
  const module = loadModule("src/memory-recall.ts", { ...disk.globals, ToolPkg: { ...disk.globals.ToolPkg,
    registerPromptInputHook(definition) { registered = definition; },
  } });
  module.registerMemoryRecallHook();
  assert.equal(registered.id, "character-memory-recall");
  return registered.function;
}

/** Builds one input event with the exact generic execution context the host supplies. */
function inputEvent(payload) {
  return { eventName: "before_process", eventPayload: { stage: "before_process", ...payload } };
}

const RECALL_EVENT = {
  processedInput: "What is the stable fact?",
  metadata: { executionContext: { chatId: "chat-recall", participantId: "default" } },
};

/** Requires structural skips to stay free of any business storage or provider call. */
test("prompt input hook skips non-character requests without touching storage", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" });
  const hook = recallHook(disk), before = disk.stateBytes();
  for (const stage of ["after_process", "before_finalize_prompt", "before_send_to_model"]) {
    assert.equal(await hook(inputEvent({ ...RECALL_EVENT, stage })), null);
  }
  assert.equal(await hook(inputEvent({ processedInput: "hello", metadata: { executionContext: { chatId: "chat-1", participantId: null } } })), null);
  assert.equal(await hook(inputEvent({ processedInput: "hello", metadata: { executionContext: { chatId: "chat-1", participantId: "model:chat" } } })), null);
  assert.equal(await hook(inputEvent({ ...RECALL_EVENT, processedInput: "" })), null);
  assert.equal(await hook(inputEvent({ ...RECALL_EVENT, processedInput: '<memory_context source="character-memory">kept</memory_context>' })), null);
  assert.equal(await hook(inputEvent({ ...RECALL_EVENT, processedInput: '<attachment id="character_memory_recall" filename="相关记忆" type="text/plain">kept</attachment>' })), null);
  assert.deepEqual(disk.stateBytes(), before);
});

/** Checks real retrieval, exact block composition, and the per-chat provider snapshot behavior. */
test("recall injects matched participant memories once per chat snapshot", async t => {
  const env = await environment(t);
  // The lexical channel ranks titles (and document chunks), matching the Kotlin reference behavior.
  await env.client.memory.create({ ownerKey: "character:default", values: { title: "Stable fact", content: "Known durable knowledge" } });
  const hook = recallHook(env.disk);
  const first = await hook(inputEvent(RECALL_EVENT));
  assert.match(first, /<attachment id="character_memory_recall" filename="相关记忆" type="text\/plain" size="\d+">/);
  assert.ok(first.endsWith("</attachment>"));
  assert.doesNotMatch(first, /<memory_context/);
  assert.match(first, /Known durable knowledge/);
  assert.match(first, /标题: Stable fact/);
  assert.ok(first.startsWith("What is the stable fact?"));
  assert.equal(await hook(inputEvent(RECALL_EVENT)), null);
  const otherChat = await hook(inputEvent({ ...RECALL_EVENT, metadata: { executionContext: { chatId: "chat-other", participantId: "default" } } }));
  assert.match(otherChat, /Known durable knowledge/);
  const unknownParticipant = await hook(inputEvent({ ...RECALL_EVENT, metadata: { executionContext: { chatId: "chat-recall", participantId: "missing-participant" } } }));
  assert.equal(unknownParticipant, null);
});

/** Real simultaneous input hooks must not append the same recalled record to two turns. */
test("concurrent recall hooks inject each matched memory only once per chat", async t => {
  const env = await environment(t);
  for (const title of ["Stable fact", "Other fact"]) await env.client.memory.create({ ownerKey: "character:default", values: { title, content: "Known durable " + title } });
  env.disk.replaceState({ ...plain(env.disk.readState()), memoryRecall: { enabled: true, limit: 1, allowRepeated: false } });
  const hook = recallHook(env.disk);
  const outputs = await Promise.all([hook(inputEvent(RECALL_EVENT)), hook(inputEvent(RECALL_EVENT)), hook(inputEvent(RECALL_EVENT))]);
  const injected = outputs.filter(output => output !== null);
  assert.equal(injected.length, 2);
  for (const title of ["Stable fact", "Other fact"]) {
    assert.equal(injected.filter(output => output.includes("标题: " + title)).length, 1);
  }
  assert.ok(injected.every(output => output.includes('<attachment id="character_memory_recall"')));
  assert.equal(await hook(inputEvent(RECALL_EVENT)), null);
});

/** Literal markup in recalled records must survive UI decoding without ending the attachment early. */
test("recall escapes memory text inside the supplementary attachment", async t => {
  const env = await environment(t);
  await env.client.memory.create({ ownerKey: "character:default", values: {
    title: "Stable fact", content: 'Use <context> & "quotes" and \'apostrophes\'; keep </attachment> literal',
  } });
  const injected = await recallHook(env.disk)(inputEvent(RECALL_EVENT));
  assert.match(injected, /Use &lt;context&gt; &amp; &quot;quotes&quot; and &apos;apostrophes&apos;; keep &lt;\/attachment&gt; literal/);
  assert.equal(injected.match(/<\/attachment>/g).length, 1);
});

/** Reuses a stored user message verbatim even when runtime deduplication was lost or repeats are allowed. */
test("replayed user input retains its memory context across runtime restarts without querying again", async t => {
  const env = await environment(t);
  await env.client.memory.create({ ownerKey: "character:default", values: { title: "Stable fact", content: "Known durable knowledge" } });
  env.disk.replaceState({ ...plain(env.disk.readState()), memoryRecall: { enabled: true, limit: 3, allowRepeated: true } });
  const first = await recallHook(env.disk)(inputEvent(RECALL_EVENT));
  assert.match(first, /Known durable knowledge/);
  const restartedHook = recallHook(env.disk);
  const before = env.disk.calls.length;
  // A null mutation preserves the complete stored message through both input hook stages.
  assert.equal(await restartedHook(inputEvent({ ...RECALL_EVENT, processedInput: first })), null);
  assert.equal(await restartedHook(inputEvent({ ...RECALL_EVENT, processedInput: first, stage: "after_process" })), null);
  assert.equal(env.disk.calls.length, before);
  // A new user turn can still recall normally; the stored prefix is never rewritten.
  assert.match(await restartedHook(inputEvent(RECALL_EVENT)), /Known durable knowledge/);
});

/** Requires the persisted switch to gate injection before any retrieval happens. */
test("disabled recall settings stop injection without any memory query", async t => {
  const env = await environment(t);
  await env.client.memory.create({ ownerKey: "character:default", values: { title: "Stable fact", content: "Known durable knowledge" } });
  env.disk.replaceState({ ...plain(env.disk.readState()), memoryRecall: { enabled: false } });
  const hook = recallHook(env.disk);
  assert.equal(await hook(inputEvent(RECALL_EVENT)), null);
});

/** Requires the configured count to bound one turn and to feed the next turn from the same snapshot. */
test("recall limit bounds one turn and repeats deliver the remaining memories", async t => {
  const env = await environment(t);
  for (const title of ["Stable fact", "Other fact"]) await env.client.memory.create({ ownerKey: "character:default", values: { title, content: "Known durable " + title } });
  env.disk.replaceState({ ...plain(env.disk.readState()), memoryRecall: { enabled: true, limit: 1 } });
  const hook = recallHook(env.disk);
  const first = await hook(inputEvent(RECALL_EVENT));
  assert.match(first, /Known durable Stable fact/);
  assert.doesNotMatch(first, /Other fact/);
  const second = await hook(inputEvent(RECALL_EVENT));
  assert.match(second, /Known durable Other fact/);
  assert.equal(await hook(inputEvent(RECALL_EVENT)), null);
});

/** Requires repeated hits to bypass the per-chat snapshot exclusion. */
test("allowRepeated recall settings deliver the same memory again", async t => {
  const env = await environment(t);
  await env.client.memory.create({ ownerKey: "character:default", values: { title: "Stable fact", content: "Known durable knowledge" } });
  env.disk.replaceState({ ...plain(env.disk.readState()), memoryRecall: { enabled: true, limit: 3, allowRepeated: true } });
  const hook = recallHook(env.disk);
  const outputs = await Promise.all([hook(inputEvent(RECALL_EVENT)), hook(inputEvent(RECALL_EVENT))]);
  for (const output of outputs) assert.match(output, /Known durable knowledge/);
});

/** Verifies the transactional state path persists the settings and released states default to enabled. */
test("recall settings persist through the transactional state path and default to enabled", async t => {
  const disk = await createDiskHarness(t);
  const database = loadModule("src/storage/database.ts", disk.globals).DatabaseCharacterRepository;
  const owner = await database.open();
  assert.deepEqual(plain(await owner.readMemoryRecallSettings()), { enabled: true, limit: 3, allowRepeated: false });
  const saved = await owner.run(session => session.writeMemoryRecallSettings({ enabled: false, limit: 5, allowRepeated: true }));
  assert.deepEqual(plain(saved), { enabled: false, limit: 5, allowRepeated: true });
  const reopened = await database.open();
  assert.deepEqual(plain(await reopened.readMemoryRecallSettings()), { enabled: false, limit: 5, allowRepeated: true });
  assert.deepEqual(plain(disk.readState()).memoryRecall, { enabled: false, limit: 5, allowRepeated: true });
  await assert.rejects(owner.run(session => session.writeMemoryRecallSettings({ enabled: true, limit: 0, allowRepeated: false })), /memory recall limit/);
  await assert.rejects(owner.run(session => session.writeMemoryRecallSettings({ enabled: true, limit: 51, allowRepeated: false })), /memory recall limit/);
});

/** Requires an unconfigured embedding provider to skip the semantic channel instead of failing recall. */
test("vector weight degrades to zero when cloud embedding is unconfigured", async t => {
  const env = await environment(t);
  await env.client.memory.create({ ownerKey: "character:default", values: { title: "Stable fact", content: "Known durable knowledge" } });
  const state = plain(env.disk.readState());
  state.owners[0].searchConfig = { scoreMode: "BALANCED", keywordWeight: 10, tagWeight: 0, vectorWeight: 0.5, edgeWeight: 0.4 };
  state.owners[0].settings.cloudEmbeddingEnabled = false;
  env.disk.replaceState(state);
  const hook = recallHook(env.disk);
  const injected = await hook(inputEvent(RECALL_EVENT));
  assert.match(injected, /Known durable knowledge/);
});

/** Punctuation and emoji are valid turns even when lexical retrieval has no usable terms. */
test("recall without searchable tokens returns no attachment and logs no error", async t => {
  const env = await environment(t);
  await env.client.memory.create({ ownerKey: "character:default", values: { title: "Stable fact", content: "Known durable knowledge" } });
  const errors = [];
  const hook = loadModule("src/memory-recall.ts", { ...env.disk.globals, console: { ...console, error(...values) { errors.push(values); } } }).onMemoryRecallPromptInput;
  for (const processedInput of ["!!!", "……", "😊", '<attachment type="image/png">image</attachment>！']) {
    assert.equal(await hook(inputEvent({ ...RECALL_EVENT, processedInput })), null);
  }
  assert.deepEqual(errors, []);
  assert.match(await hook(inputEvent(RECALL_EVENT)), /Known durable knowledge/);
});

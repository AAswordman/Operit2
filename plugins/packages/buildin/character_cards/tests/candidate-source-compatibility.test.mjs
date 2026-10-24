import assert from "node:assert/strict";
import test from "node:test";
import { createDiskHarness } from "./disk-files.mjs";
import { createMemoryJobsHarness, CONTROLLED_OWNER } from "./memory-jobs-runtime.mjs";
import { loadModule, plain } from "./runtime.mjs";

/** Creates a persisted extraction request for one exact message revision. */
function candidate(id, values = {}) {
  return { id, chatId: "source-chat", triggerMessageTimestamp: 100, triggerVariantIndex: 3,
    createdAt: 150, updatedAt: 150, status: "pending", attemptCount: 0, lastError: "",
    sourceType: "reply_finalized_auto", ...values };
}

/** Seeds independently identified historical tasks directly in the actual object database. */
async function seed(t, candidates) {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" });
  const state = plain(disk.readState());
  state.owners[0].candidates = candidates;
  disk.replaceState(state); disk.clearCalls();
  return disk;
}

/** Opens the actual repository and its full persisted-state validation. */
async function open(disk) {
  return loadModule("src/storage/database.ts", disk.globals).DatabaseCharacterRepository.open();
}

/** Selects actual host mutations without treating read attempts as data changes. */
function writes(disk) {
  return disk.calls.filter(call => ["storage.commit", "storage.preferences.edit", "write", "mkdir", "move", "deleteFile"].includes(call.method));
}

/** Preserves every historical task and lifecycle state when multiple requests resolve to one message. */
test("installed duplicate sources reopen without rewriting independent pending, processing or failed tasks", async t => {
  const tasks = [candidate("pending"), candidate("processing", { updatedAt: 200, status: "processing", attemptCount: 2 }),
    candidate("failed", { updatedAt: 300, status: "failed", attemptCount: 4, lastError: "Original extraction error" })];
  const disk = await seed(t, tasks), before = disk.stateBytes();
  const repository = await open(disk);
  const saved = plain(await repository.run(session => session.readMemorySpace("character:default")));
  assert.deepEqual(saved.candidates, [...tasks].sort((left, right) => left.id.localeCompare(right.id)));
  assert.deepEqual(disk.stateBytes(), before); assert.deepEqual(writes(disk), []);
  await open(disk);
  assert.deepEqual(disk.stateBytes(), before); assert.deepEqual(writes(disk), []);
});

/** Exercises the exact registered submit hook and enqueue API against duplicated persisted sources. */
test("duplicate historical tasks allow input submission and repeated enqueue without adding or resetting tasks", async t => {
  const harness = await createMemoryJobsHarness(t), first = harness.openRuntime();
  await first.initialize(); await first.bind(); await first.enqueueReplies();
  const state = await harness.readState(), space = state.owners.find(owner => owner.ownerKey === CONTROLLED_OWNER);
  const original = space.candidates[0];
  const retainedFailure = { ...original, id: "historical-failed-task", status: "failed", attemptCount: 4, lastError: "Original failed extraction" };
  space.candidates.push(retainedFailure); harness.disk.replaceState(state); harness.disk.clearCalls();
  const before = harness.disk.stateBytes(), restarted = harness.openRuntime();
  assert.equal(restarted.chatInputHooks.length, 1); assert.equal(restarted.chatInputHooks[0].id, "group-input-submit");
  const result = await restarted.chatInputHooks[0].function({ event: "submit_requested", eventName: "submit_requested",
    eventPayload: { chatId: original.chatId, source: "User" } });
  assert.equal(result, null);
  const enqueued = plain(await restarted.api("memory.candidate.enqueue", { chatId: original.chatId, timestamp: original.triggerMessageTimestamp,
    variantIndex: original.triggerVariantIndex, sourceType: original.sourceType, ownerKey: null }));
  assert.deepEqual(enqueued.owners, [CONTROLLED_OWNER]);
  assert.deepEqual([...enqueued.candidateIds].sort(), [original.id, retainedFailure.id].sort());
  assert.deepEqual(harness.disk.stateBytes(), before); assert.deepEqual(writes(harness.disk), []);
  await restarted.makeDue(); harness.queueAnalysis({}); await restarted.interval(); harness.assertOutputsConsumed();
  assert.equal(harness.modelCalls.length, 1);
  const saved = await harness.readState();
  assert.deepEqual(saved.owners.find(owner => owner.ownerKey === CONTROLLED_OWNER).candidates, [retainedFailure]);
});

/** Keeps the existing record-identity and lifecycle validation while allowing repeated message sources. */
test("duplicate-source compatibility still rejects duplicate task IDs and invalid candidate states", async t => {
  for (const [tasks, message] of [
    [[candidate("same"), candidate("same")], /candidates has duplicate id/],
    [[candidate("older"), candidate("newer", { status: "unknown" })], /Invalid candidate status/],
  ]) {
    const state = plain(loadModule("src/storage/state.ts").createInitialState(123));
    state.owners[0].candidates = tasks;
    assert.throws(() => loadModule("src/storage/state.ts").assertCharacterState(state), message);
  }
  const disk = await seed(t, [candidate("older"), candidate("newer", { status: "unknown" })]);
  const repository = await open(disk);
  await assert.rejects(repository.run(session => session.listCharacters()), /Invalid candidate status/);
  assert.deepEqual(writes(disk), []);
});

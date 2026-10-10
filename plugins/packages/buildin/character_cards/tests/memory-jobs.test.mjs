import assert from "node:assert/strict";
import test from "node:test";
import path from "node:path";
import { readFile } from "node:fs/promises";
import { plain } from "./runtime.mjs";
import { CONTROLLED_CHAT_ID, CONTROLLED_OWNER, createMemoryJobsHarness } from "./memory-jobs-runtime.mjs";

const owner = { ownerKey: CONTROLLED_OWNER };

/** Reads one genuine owner from native bytes and rejects absent or duplicated persisted spaces. */
async function readOwner(harness) {
  const state = await harness.readState(), matches = [];
  for (const space of state.owners) if (space.ownerKey === CONTROLLED_OWNER) matches.push(space);
  assert.equal(matches.length, 1); return matches[0];
}

/** Counts only actual host mutations while leaving all native read attempts visible in the audit log. */
function mutations(disk) {
  const selected = [];
  for (const call of disk.calls) if (["write", "move", "mkdir", "deleteFile"].indexOf(call.method) !== -1) selected.push(call);
  return selected;
}

/** Exercises main registration, lifecycle, real candidates across restart, and extraction through the registered interval. */
test("controlled Chat/AI: registered main lifecycle and message hooks persist candidates across restart and interval commits graph plus real USER.md", async t => {
  const harness = await createMemoryJobsHarness(t), first = harness.openRuntime();
  assert.equal(first.chatMessageHooks.length, 1); assert.equal(first.hostEventHooks.length, 1); assert.equal(first.lifecycle.length, 1);
  assert.equal(first.chatMessageHooks[0].id, "memory-candidate-enqueue"); assert.equal(first.chatMessageHooks[0].function, first.main.onMemoryMessagePersisted);
  assert.equal(first.hostEventHooks[0].id, "memory-jobs-interval"); assert.equal(first.hostEventHooks[0].function, first.main.onMemoryInterval);
  assert.deepEqual(plain(first.hostEventHooks[0].trigger), { kind: "interval", intervalMs: 60000 });
  assert.equal(harness.disk.calls.length, 0); assert.deepEqual(await harness.disk.entries(), []);
  assert.throws(
    /** Probes an intentionally unprovided SDK directory capability. */
    () => harness.tools.SoftwareSettings,
    /does not provide Tools\.SoftwareSettings/u,
  );
  assert.throws(
    /** Probes a capped directory API not supplied by this controlled fixture. */
    () => harness.tools.Chat.listChats,
    /does not provide Tools\.Chat\.listChats/u,
  );
  await first.initialize(); await first.bind(); const sources = await first.enqueueReplies();
  await first.persistMessage(sources[0]);
  const before = await readOwner(harness); assert.equal(before.candidates.length, 5); assert.equal(harness.modelCalls.length, 0);
  for (const candidate of before.candidates) { assert.equal(candidate.status, "pending"); assert.equal(candidate.sourceType, "reply_finalized_auto"); assert.equal(typeof candidate.id, "string"); }
  harness.disk.clearCalls(); const restarted = harness.openRuntime(); assert.equal(harness.disk.calls.length, 0);
  await restarted.initialize(); assert.equal(mutations(harness.disk).length, 0);
  assert.deepEqual(plain(await restarted.api("chat.configuration.binding.read", { chatId: CONTROLLED_CHAT_ID })), { chatId: CONTROLLED_CHAT_ID, selection: "card:default" });
  assert.deepEqual((await readOwner(harness)).candidates, before.candidates);
  await restarted.makeDue();
  harness.queueAnalysis({ main: { title: "Controlled project", content: "An explicit project fact", tags: ["Project"], folder_path: "Projects" },
    new: [{ title: "Controlled concept", content: "An explicit concept fact", tags: ["Concept"], folder_path: "Concepts" }],
    update: [], merge: [], links: [{ source: "Controlled project", target: "Controlled concept", type: "related", weight: 0.7, description: "Explicit controlled relationship" }],
    profile_markdown: "# User\nControlled extracted profile\n" });
  await restarted.interval(); harness.assertOutputsConsumed(); assert.equal(harness.modelCalls.length, 1);
  const saved = await readOwner(harness); assert.equal(saved.candidates.length, 0); assert.equal(saved.memories.length, 2); assert.equal(saved.links.length, 1);
  assert.equal(Object.hasOwn(saved, "userMarkdown"), false);
  const document = await restarted.api("memory.user.path", owner);
  assert.equal(path.resolve(document.path), path.resolve(harness.disk.directory, "owners", encodeURIComponent(CONTROLLED_OWNER), "USER.md"));
  assert.equal(await readFile(document.path, "utf8"), "# User\nControlled extracted profile\n");
  const exported = JSON.parse((await restarted.api("memory.export", owner)).content);
  assert.equal(exported.userMarkdown, "# User\nControlled extracted profile\n"); assert.deepEqual(exported.space.memories, saved.memories);
  const finalRuntime = harness.openRuntime(); await finalRuntime.initialize();
  assert.deepEqual((await readOwner(harness)).memories, saved.memories); await finalRuntime.interval(); assert.equal(harness.modelCalls.length, 1);
});

/** Verifies real rebuild planning, persisted cursor across a fresh main runtime, and cancellation without further AI work. */
test("controlled Chat/AI: rebuild cursor and counters survive restart and registered cancellation stops subsequent interval extraction", async t => {
  const harness = await createMemoryJobsHarness(t), first = harness.openRuntime(); await first.initialize(); await first.bind();
  const preparing = plain(await first.startRebuild()); assert.equal(preparing.status, "preparing"); assert.equal(preparing.totalSourceMessages, 32); assert.ok(preparing.totalWindows > 1);
  harness.queueAnalysis({}); await first.interval(); harness.assertOutputsConsumed();
  const running = plain(await first.api("memory.rebuild.progress", owner)); assert.equal(running.status, "running"); assert.equal(running.completedWindows, 1);
  const space = await readOwner(harness); assert.equal(space.rebuildTask.nextWindow, 1);
  assert.equal(running.processedSourceMessages, space.rebuildTask.windows[0].sourceMessageCount);
  const restarted = harness.openRuntime(); await restarted.initialize(); assert.deepEqual(plain(await restarted.api("memory.rebuild.progress", owner)), running);
  const cancelled = plain(await restarted.api("memory.rebuild.cancel", owner)); assert.equal(cancelled.status, "cancelled"); assert.equal(cancelled.completedWindows, 1);
  const calls = harness.modelCalls.length; await restarted.interval(); assert.equal(harness.modelCalls.length, calls);
  const finalRuntime = harness.openRuntime(); await finalRuntime.initialize(); await finalRuntime.interval();
  assert.equal(harness.modelCalls.length, calls); assert.deepEqual(plain(await finalRuntime.api("memory.rebuild.progress", owner)), cancelled);
  assert.equal((await readOwner(harness)).rebuildTask.nextWindow, 1);
});

/** Requires exact execution failure propagation, durable failed progress, untouched records, and no restart retry. */
test("controlled Chat/AI: rebuild execution failure records failed progress and preserves original error without interval or restart retry", async t => {
  const harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime(); await runtime.initialize(); await runtime.bind();
  await runtime.api("memory.create", { ...owner, values: { title: "Existing full memory", content: "Must remain unchanged", contentType: "text", source: "controlled-test", credibility: 0.9, importance: 0.7, folderPath: "Existing", tags: ["Keep"] } });
  await runtime.startRebuild(); const before = await readOwner(harness), failure = new Error("Explicit controlled MEMORY execution failure"); harness.queueExecutionError(failure);
  await assert.rejects(runtime.interval(),
    /** Requires the same original controlled error object, not an empty success or replacement rejection. */
    error => error === failure,
  );
  harness.assertOutputsConsumed(); const failed = await readOwner(harness);
  assert.deepEqual(failed.memories, before.memories); assert.equal(failed.rebuildTask.nextWindow, 0);
  assert.equal(failed.rebuildProgress.status, "failed"); assert.equal(failed.rebuildProgress.failedWindows, 1); assert.equal(failed.rebuildProgress.completedWindows, 0);
  assert.equal(failed.rebuildProgress.lastError, failure.message); const calls = harness.modelCalls.length;
  await runtime.interval(); assert.equal(harness.modelCalls.length, calls);
  const restarted = harness.openRuntime(); await restarted.initialize(); await restarted.interval();
  assert.equal(harness.modelCalls.length, calls); assert.deepEqual(plain(await restarted.api("memory.rebuild.progress", owner)), failed.rebuildProgress);
});

/** Revalidates committed candidate source identities and refuses to analyze or delete a batch with a removed source. */
test("controlled Chat/AI: deleting a pending candidate source produces durable failed candidates before AI and never automatically retries", async t => {
  const harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime(); await runtime.initialize(); await runtime.bind();
  const sources = await runtime.enqueueReplies(); const before = await readOwner(harness);
  harness.deleteMessage(sources[0].timestamp); await runtime.makeDue();
  await assert.rejects(runtime.interval(), /claimed candidate source no longer identifies exactly one message/u);
  assert.equal(harness.modelCalls.length, 0); harness.assertOutputsConsumed(); const failed = await readOwner(harness);
  assert.equal(failed.candidates.length, 5); assert.deepEqual(failed.memories, before.memories);
  for (const candidate of failed.candidates) { assert.equal(candidate.status, "failed"); assert.equal(candidate.attemptCount, 1); assert.match(candidate.lastError, /claimed candidate source/u); }
  const restarted = harness.openRuntime(); await restarted.initialize(); await restarted.makeDue(); await restarted.interval();
  assert.equal(harness.modelCalls.length, 0); assert.deepEqual((await readOwner(harness)).candidates, failed.candidates);
});

/** Rejects both count-changing and count-preserving source edits during reads without publishing partial candidates or plans. */
test("controlled Chat/AI: concurrent source count or updated-at changes reject message enqueue and rebuild planning without any disk publication", async t => {
  const harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime(); await runtime.initialize(); await runtime.bind();
  const reply = harness.message(31), before = await harness.readStateText(); harness.disk.clearCalls(); harness.appendMessageDuringNextRange();
  await assert.rejects(runtime.persistMessage(reply), /history changed while reading the planned source range/u);
  assert.equal(await harness.readStateText(), before); assert.equal(mutations(harness.disk).length, 0); assert.equal(harness.modelCalls.length, 0);
  assert.equal(harness.rangeCalls.length, 1); assert.equal(harness.rangeCalls[0].end, 31);
  harness.disk.clearCalls(); harness.editMessageDuringNextRange(reply.timestamp);
  await assert.rejects(runtime.startRebuild(), /history changed while reading the planned source range/u);
  assert.equal(await harness.readStateText(), before); assert.equal(mutations(harness.disk).length, 0); assert.equal(harness.modelCalls.length, 0);
  assert.equal(harness.rangeCalls.length, 2); assert.equal(harness.rangeCalls[1].end, 32);
  const ownerState = await readOwner(harness); assert.equal(ownerState.candidates.length, 0); assert.equal(ownerState.rebuildTask, null); harness.assertOutputsConsumed();
});

/** Preserves both original failures when execution and real status-file publication fail, for each production job path. */
test("controlled Chat/AI: rebuild and candidate execution plus state-publication failures retain both errors and stop the same repository", async t => {
  for (const kind of ["rebuild", "candidates"]) {
    const harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime(); await runtime.initialize(); await runtime.bind();
    if (kind === "rebuild") await runtime.startRebuild();
    else { await runtime.enqueueReplies(); await runtime.makeDue(); }
    const failure = new Error("Controlled execution failure: " + kind), statusFailure = new Error("Controlled native state publication failure: " + kind);
    const before = await readOwner(harness); harness.disk.clearCalls(); harness.queueExecutionAndStatusError(failure, statusFailure);
    await assert.rejects(runtime.interval(),
      /** Checks exact preserved error identities from both unsuccessful phases. */
      error => error.name === "MemoryJobStatusError" && error.executionError === failure && error.statusPublicationError === statusFailure,
    );
    harness.assertOutputsConsumed(); const stored = await readOwner(harness); assert.deepEqual(stored.memories, before.memories);
    if (kind === "rebuild") { assert.equal(stored.rebuildProgress.status, "preparing"); assert.equal(stored.rebuildProgress.completedWindows, 0); assert.equal(stored.rebuildProgress.failedWindows, 0); }
    else { assert.equal(stored.candidates.length, 5); for (const candidate of stored.candidates) { assert.equal(candidate.status, "processing"); assert.equal(candidate.attemptCount, 1); } }
    const calls = harness.modelCalls.length;
    await assert.rejects(runtime.interval(),
      /** Requires the retained original IO rejection from the repository's fail-stop state. */
      error => error === statusFailure,
    );
    assert.equal(harness.modelCalls.length, calls);
    const writes = harness.disk.calls.filter(call => call.method === "storage.commit");
    assert.ok(writes.length > 0, "The failure must occur during an actual storage transaction");
    const unpublished = writes[writes.length - 1].args[0];
    if (kind === "rebuild") {
      const owner = unpublished.find(row => row.collection === "owners" && row.key === CONTROLLED_OWNER);
      assert.equal(owner.value.rebuildProgress.status, "failed");
      assert.equal(owner.value.rebuildProgress.lastError, failure.message);
    } else {
      for (const row of unpublished.filter(row => row.collection === "candidates")) {
        assert.equal(row.value.record.status, "failed"); assert.equal(row.value.record.lastError, failure.message);
      }
    }
  }
});
/** Verifies a genuine nonzero selected fixture revision through main's registered persistence hook and interval. */
test("controlled Chat/AI: persistence hook maps Core selectedVariantIndex to the saved nonzero candidate revision", async t => {
  const harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime(); await runtime.initialize(); await runtime.bind();
  const timestamp = harness.message(31).timestamp; harness.selectNewVariant(timestamp); harness.selectNewVariant(timestamp);
  const selected = harness.message(31); assert.equal(selected.variantIndex, 2);
  harness.removeChatMarker(); const sources = await runtime.enqueueReplies();
  const saved = await readOwner(harness), matches = saved.candidates.filter(
    /** Selects the exact persisted timestamp without conflating other queued replies. */
    candidate => candidate.triggerMessageTimestamp === timestamp,
  );
  assert.equal(saved.candidates.length, 5); assert.equal(matches.length, 1); assert.equal(matches[0].triggerVariantIndex, 2);
  const revisionReads = harness.extensionCalls.filter(
    /** Counts the genuine selected-revision namespace reads performed by production code. */
    call => call.method === "readExtension" && call.target.kind === "message" && call.target.messageTimestamp === timestamp,
  );
  assert.ok(revisionReads.length > 0); for (const call of revisionReads) assert.equal(call.target.variantIndex, 2);
  const before = await harness.readStateText(); harness.disk.clearCalls(); await runtime.persistMessage(selected);
  assert.equal(await harness.readStateText(), before); assert.equal(mutations(harness.disk).length, 0);
  assert.equal(sources[sources.length - 1].variantIndex, 2); await runtime.makeDue(); harness.queueAnalysis({});
  await runtime.interval(); harness.assertOutputsConsumed(); assert.equal((await readOwner(harness)).candidates.length, 0);
  assert.equal(harness.modelCalls.length, 1);
});

/** Rejects missing or invalid real hook revisions before opening a candidate transaction or invoking AI. */
test("controlled Chat/AI: persistence hook rejects missing selectedVariantIndex despite a differently named fixture field", async t => {
  const harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime(); await runtime.initialize(); await runtime.bind();
  const message = harness.message(31), before = await harness.readStateText(); harness.disk.clearCalls();
  const payload = { chatId: CONTROLLED_CHAT_ID, sender: message.sender, timestamp: message.timestamp, content: message.content, variantIndex: message.variantIndex };
  /** Dispatches the actual registered callback without adapting or supplying an absent host field. */
  async function invoke(eventPayload) {
    await runtime.chatMessageHooks[0].function({ event: "message_persisted", eventName: "message_persisted", eventPayload });
  }
  await assert.rejects(() => invoke(payload), /persisted message selected variant index/);
  for (const selectedVariantIndex of [null, "0", -1, 1.25, 2147483648]) await assert.rejects(() => invoke({ ...payload, selectedVariantIndex }), /persisted message selected variant index|Persisted message variant exceeds/);
  await assert.rejects(() => invoke({ ...payload, selectedVariantIndex: 2 }), /Candidate does not identify exactly one source message/);
  assert.equal(await harness.readStateText(), before); assert.equal(mutations(harness.disk).length, 0); assert.equal(harness.modelCalls.length, 0);
});

/** Invokes the registered provider with explicit revision inputs, independently from the separately tested message-hook forwarding. */
async function enqueueProviderReplies(runtime, harness) {
  const sources = [];
  for (const index of [23, 25, 27, 29, 31]) {
    const message = harness.message(index); sources.push(message);
    await runtime.api("memory.candidate.enqueue", { chatId: CONTROLLED_CHAT_ID, timestamp: message.timestamp, variantIndex: message.variantIndex, sourceType: "reply_finalized_auto", ownerKey: null });
  }
  return sources;
}

/** Proves provider-to-interval work keeps the original send identity even after the current chat namespace is removed. */
test("controlled Chat/AI: registered candidate provider and interval use exact saved message snapshots without reading current chat selection", async t => {
  const harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime(); await runtime.initialize(); await runtime.bind();
  await enqueueProviderReplies(runtime, harness);
  const before = await readOwner(harness);
  assert.equal(before.candidates.length, 5);
  for (const candidate of before.candidates) assert.equal(candidate.triggerVariantIndex, 0);
  assert.equal(Object.hasOwn(await harness.readState(), "chatBindings"), false);
  harness.removeChatMarker(); const readCount = harness.extensionCalls.length; await runtime.makeDue();
  harness.queueAnalysis({ main: { title: "Snapshot ownership", content: "Original owner remains the only destination", tags: ["Snapshot"], folder_path: "Projects" }, new: [], update: [], merge: [], links: [] });
  await runtime.interval(); harness.assertOutputsConsumed();
  const saved = await readOwner(harness); assert.equal(saved.candidates.length, 0); assert.equal(saved.memories.length, 1);
  assert.equal(saved.memories[0].title, "Snapshot ownership");
  for (const call of harness.extensionCalls.slice(readCount)) assert.equal(call.target.kind, "message", "Historical assistant execution must not read current chat identity");
});

/** Checks strict missing-snapshot and required-revision errors without manufacturing an owner or publishing a candidate. */
test("controlled Chat/AI: candidate provider rejects missing snapshots or absent variant input without a disk mutation", async t => {
  const harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime(); await runtime.initialize(); await runtime.bind();
  const source = harness.message(31), before = await harness.readStateText(); harness.disk.clearCalls();
  await assert.rejects(() => runtime.api("memory.candidate.enqueue", { chatId: CONTROLLED_CHAT_ID, timestamp: source.timestamp, sourceType: "reply_finalized_auto", ownerKey: null }), /variantIndex is required/);
  harness.removeMessageMarker(source.timestamp, source.variantIndex);
  await assert.rejects(() => runtime.api("memory.candidate.enqueue", { chatId: CONTROLLED_CHAT_ID, timestamp: source.timestamp, variantIndex: source.variantIndex, sourceType: "reply_finalized_auto", ownerKey: null }), /no saved character identity/);
  assert.equal(await harness.readStateText(), before); assert.equal(mutations(harness.disk).length, 0); assert.equal(harness.modelCalls.length, 0);
});

/** Revalidates the exact persisted candidate revision after a real selected-variant change and never retries the failed batch. */
test("controlled Chat/AI: changing a candidate's selected revision records terminal failure before AI and survives restart without retry", async t => {
  const harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime(); await runtime.initialize(); await runtime.bind();
  const sources = await enqueueProviderReplies(runtime, harness); harness.selectNewVariant(sources[0].timestamp); await runtime.makeDue();
  await assert.rejects(runtime.interval(), /claimed candidate source no longer identifies exactly one message/);
  const failed = await readOwner(harness); assert.equal(failed.candidates.length, 5); assert.equal(harness.modelCalls.length, 0);
  for (const candidate of failed.candidates) { assert.equal(candidate.triggerVariantIndex, 0); assert.equal(candidate.status, "failed"); assert.equal(candidate.attemptCount, 1); }
  const restarted = harness.openRuntime(); await restarted.initialize(); await restarted.makeDue(); await restarted.interval();
  assert.deepEqual((await readOwner(harness)).candidates, failed.candidates); assert.equal(harness.modelCalls.length, 0);
});

/** Rejects deleted source revisions in an already committed rebuild plan and retains failure counters across restart. */
test("controlled Chat/AI: a deleted committed rebuild source fails before extraction and cannot be resumed automatically", async t => {
  const harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime(); await runtime.initialize(); await runtime.bind();
  await runtime.startRebuild(); harness.deleteMessage(harness.message(1).timestamp);
  await assert.rejects(runtime.interval(), /planned rebuild source message revision changed or was deleted/);
  const failed = await readOwner(harness); assert.equal(failed.rebuildProgress.status, "failed"); assert.equal(failed.rebuildProgress.failedWindows, 1);
  assert.equal(failed.rebuildTask.nextWindow, 0); assert.equal(harness.modelCalls.length, 0);
  const restarted = harness.openRuntime(); await restarted.initialize(); await restarted.interval();
  assert.deepEqual((await readOwner(harness)).rebuildProgress, failed.rebuildProgress); assert.equal(harness.modelCalls.length, 0);
});

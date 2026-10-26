import assert from "node:assert/strict";
import test from "node:test";
import path from "node:path";
import { readFile } from "node:fs/promises";
import { loadModule, plain } from "./runtime.mjs";
import { CONTROLLED_CHAT_ID, CONTROLLED_OWNER, createMemoryJobsHarness } from "./memory-jobs-runtime.mjs";

const owner = { ownerKey: CONTROLLED_OWNER };

/** Keeps candidate retrieval aligned with the Kotlin question, answer, and recent-context budgets. */
test("candidate search query focuses recent question and answer within explicit bounds", () => {
  const { buildCandidateSearchQuery } = loadModule("src/memory-jobs/analysis.ts");
  const messages = [
    { sender: "user", content: "Earlier unrelated request" },
    { sender: "assistant", content: "<think>private reasoning</think>Answer https://example.test/path" },
    { sender: "user", content: "Question: preserve SSH context\n<memory>injected memory</memory>" },
  ];
  const query = buildCandidateSearchQuery(messages);
  assert.match(query, /Question: preserve SSH context/u);
  assert.match(query, /Answer/u);
  assert.doesNotMatch(query, /private reasoning|injected memory|https:/u);
  assert.ok(query.length <= 1880);

  const longMessages = [
    ...Array.from({ length: 12 }, () => ({ sender: "user", content: "c".repeat(400) })),
    { sender: "user", content: "q".repeat(900) },
    { sender: "assistant", content: "a".repeat(900) },
  ];
  const bounded = buildCandidateSearchQuery(longMessages);
  assert.ok(bounded.length <= 1882);
  assert.ok(bounded.startsWith("q".repeat(500)));
  assert.ok(bounded.includes("a".repeat(180)));
});

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
  const harness = await createMemoryJobsHarness(t), beforeRegistration = await harness.readStateText(), first = harness.openRuntime();
  assert.equal(first.chatMessageHooks.length, 1); assert.equal(first.hostEventHooks.length, 1); assert.equal(first.lifecycle.length, 1);
  assert.equal(first.chatMessageHooks[0].id, "memory-candidate-enqueue"); assert.equal(first.chatMessageHooks[0].function, first.main.onMemoryMessagePersisted);
  assert.equal(first.hostEventHooks[0].id, "memory-jobs-interval"); assert.equal(first.hostEventHooks[0].function, first.main.onMemoryInterval);
  assert.deepEqual(plain(first.hostEventHooks[0].trigger), { kind: "interval", intervalMs: 60000 });
  assert.equal(harness.disk.calls.length, 0); assert.equal(await harness.readStateText(), beforeRegistration);
  assert.throws(
    /** Probes an intentionally unprovided SDK directory capability. */
    () => harness.tools.SoftwareSettings,
    /does not provide Tools\.SoftwareSettings/u,
  );
  assert.throws(
    /** Probes a capped directory API not supplied by this controlled fixture. */
    () => harness.tools.Chat.listAll,
    /does not provide Tools\.Chat\.listAll/u,
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

/** Records a failed rebuild window while automatic extraction and later windows continue across restart. */
test("controlled Chat/AI: rebuild model failure advances its cursor and does not block automatic extraction or later windows", async t => {
  const harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime(); await runtime.initialize(); await runtime.bind();
  await runtime.api("memory.create", { ...owner, values: { title: "Existing full memory", content: "Must remain unchanged", contentType: "text", source: "controlled-test", credibility: 0.9, importance: 0.7, folderPath: "Existing", tags: ["Keep"] } });
  await runtime.startRebuild(); await runtime.enqueueReplies(); await runtime.makeDue();
  const before = await readOwner(harness), failure = new Error("Explicit controlled MEMORY execution failure");
  harness.queueExecutionError(failure);
  harness.queueAnalysis({ main: { title: "Automatic extraction after rebuild failure", content: "The independent queue still runs", tags: [], folder_path: "" }, new: [], update: [], merge: [], links: [] });
  await runtime.interval(); harness.assertOutputsConsumed();
  const failed = await readOwner(harness);
  assert.equal(failed.memories.length, 2); assert.deepEqual(failed.memories.find(memory => memory.id === before.memories[0].id), before.memories[0]);
  assert.equal(failed.candidates.length, 0); assert.equal(harness.modelCalls.length, 2);
  assert.equal(failed.rebuildTask.nextWindow, 1); assert.equal(failed.rebuildProgress.completedWindows, 1);
  assert.equal(failed.rebuildProgress.status, "running"); assert.equal(failed.rebuildProgress.failedWindows, 1);
  assert.equal(failed.rebuildProgress.lastError, failure.message);
  const restarted = harness.openRuntime(); await restarted.initialize();
  for (let index = 1; index < failed.rebuildProgress.totalWindows; index += 1) {
    harness.queueAnalysis({}); await restarted.interval();
  }
  harness.assertOutputsConsumed(); const finished = await readOwner(harness);
  assert.equal(finished.rebuildProgress.status, "completed"); assert.equal(finished.rebuildProgress.failedWindows, 1);
  assert.equal(finished.rebuildProgress.completedWindows, finished.rebuildProgress.totalWindows);
  assert.equal(finished.rebuildProgress.lastError, failure.message);
  assert.equal(harness.modelCalls.length, finished.rebuildProgress.totalWindows + 1);
  await restarted.interval(); assert.equal(harness.modelCalls.length, finished.rebuildProgress.totalWindows + 1);
});

/** Skips a deleted candidate while committing valid memory and retaining the individual failure across restart. */
test("controlled Chat/AI: one deleted candidate does not block extraction of the remaining candidates below the enqueue threshold", async t => {
  const harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime(); await runtime.initialize(); await runtime.bind();
  const sources = await runtime.enqueueReplies(), before = await readOwner(harness);
  const rejected = before.candidates.find(candidate => candidate.triggerMessageTimestamp === sources[0].timestamp);
  harness.deleteMessage(sources[0].timestamp); await runtime.makeDue();
  harness.queueAnalysis({ main: { title: "Valid sources survived", content: "Four valid candidates were extracted", tags: [], folder_path: "" }, new: [], update: [], merge: [], links: [] });
  await runtime.interval(); harness.assertOutputsConsumed(); const saved = await readOwner(harness);
  assert.equal(harness.modelCalls.length, 1); assert.equal(saved.memories[0].title, "Valid sources survived");
  assert.equal(saved.candidates.length, 1); assert.equal(saved.candidates[0].id, rejected.id);
  assert.equal(saved.candidates[0].status, "failed"); assert.equal(saved.candidates[0].attemptCount, 1);
  assert.match(saved.candidates[0].lastError, /源消息已删除或所选版本已变化/);
  assert.ok(!harness.modelCalls[0].turns[1].content.includes(sources[0].content));
  assert.ok(harness.modelCalls[0].turns[1].content.includes(sources[1].content));
  const status = await runtime.web({ action: "readMemoryAutoSaveStatus", ownerKey: CONTROLLED_OWNER });
  assert.equal(status.failedCandidates, 1); assert.equal(status.processingCandidates, 0); assert.equal(status.pendingCandidates, 0);
  assert.equal(status.lastError, saved.candidates[0].lastError);
  const restarted = harness.openRuntime(); await restarted.initialize(); await restarted.makeDue(); await restarted.interval();
  assert.equal(harness.modelCalls.length, 1); assert.deepEqual((await readOwner(harness)).candidates, saved.candidates);
});

/** Leaves an entirely invalid batch terminal while allowing a fresh set of sources to be extracted. */
test("controlled Chat/AI: an entirely deleted batch skips AI and later valid candidates still run", async t => {
  const harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime(); await runtime.initialize(); await runtime.bind();
  const sources = await runtime.enqueueReplies();
  for (const message of sources) harness.deleteMessage(message.timestamp);
  await runtime.makeDue(); await runtime.interval();
  const failed = await readOwner(harness); assert.equal(failed.candidates.length, 5); assert.equal(harness.modelCalls.length, 0);
  for (const candidate of failed.candidates) assert.equal(candidate.status, "failed");
  await runtime.enqueueReplies(); await runtime.makeDue(); harness.queueAnalysis({}); await runtime.interval(); harness.assertOutputsConsumed();
  assert.equal(harness.modelCalls.length, 1); assert.deepEqual((await readOwner(harness)).candidates, failed.candidates);
});

/** Isolates a model error to its selected-user batch while processing the next automatic batch in the same interval. */
test("controlled Chat/AI: a failed selected-user batch does not block the next automatic batch", async t => {
  const harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime(); await runtime.initialize(); await runtime.bind();
  await runtime.enqueueReplies(); const user = harness.message(22);
  const selected = await runtime.api("memory.candidate.enqueue", { chatId: CONTROLLED_CHAT_ID, timestamp: user.timestamp, variantIndex: user.variantIndex, sourceType: "selected_user_message", ownerKey: CONTROLLED_OWNER });
  await runtime.makeDue(); const failure = new Error("Controlled selected-user model failure");
  harness.queueExecutionError(failure);
  harness.queueAnalysis({ main: { title: "Next batch succeeded", content: "Automatic extraction continues", tags: [], folder_path: "" }, new: [], update: [], merge: [], links: [] });
  await runtime.interval(); harness.assertOutputsConsumed(); const saved = await readOwner(harness);
  assert.equal(harness.modelCalls.length, 2); assert.equal(saved.memories[0].title, "Next batch succeeded");
  assert.equal(saved.candidates.length, 1); assert.equal(saved.candidates[0].id, selected.candidateIds[0]);
  assert.equal(saved.candidates[0].status, "failed"); assert.equal(saved.candidates[0].lastError, failure.message);
});

/** Retains each source error even when extraction of the remaining valid messages also fails. */
test("controlled Chat/AI: source and model errors remain distinct within a failed batch", async t => {
  const harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime(); await runtime.initialize(); await runtime.bind();
  const sources = await runtime.enqueueReplies(); harness.deleteMessage(sources[0].timestamp); await runtime.makeDue();
  const failure = new Error("Controlled valid-source model failure"); harness.queueExecutionError(failure);
  await runtime.interval(); harness.assertOutputsConsumed(); const saved = await readOwner(harness);
  assert.equal(saved.candidates.length, 5); assert.equal(harness.modelCalls.length, 1);
  for (const candidate of saved.candidates) {
    assert.equal(candidate.status, "failed");
    if (candidate.triggerMessageTimestamp === sources[0].timestamp) assert.match(candidate.lastError, /源消息已删除或所选版本已变化/);
    else assert.equal(candidate.lastError, failure.message);
  }
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

/** Parses through model-emitted reasoning and prose while still rejecting output with no JSON payload. */
test("model JSON parser ignores reasoning blocks and surrounding prose but still rejects invalid output", () => {
  const { parseModelJson } = loadModule("src/memory-jobs/analysis.ts");
  assert.deepEqual(plain(parseModelJson('{"main":null,"new":[],"update":[],"merge":[],"links":[]}')), { main: null, new: [], update: [], merge: [], links: [] });
  assert.deepEqual(plain(parseModelJson('<think>internal reasoning</think>\n{"a":1}')), { a: 1 });
  assert.deepEqual(plain(parseModelJson('分析如下：\n```json\n{"a":2}\n```\n完成')), { a: 2 });
  assert.deepEqual(plain(parseModelJson('前言 {"a":3} 后记')), { a: 3 });
  /** Checks the JSON syntax rejection across the module VM's own language realm. */
  const isSyntax = error => error.name === "SyntaxError";
  assert.throws(() => parseModelJson('<think>only reasoning</think>'), isSyntax);
  assert.throws(() => parseModelJson('not json'), isSyntax);
});

/** Skips only the unresolvable model references while committing the rest of the extraction. */
test("controlled Chat/AI: unresolvable model titles skip their own items instead of failing the extraction", async t => {
  const harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime(); await runtime.initialize(); await runtime.bind();
  await runtime.enqueueReplies(); await runtime.makeDue();
  harness.queueAnalysis({ main: { title: "Valid extraction", content: "A valid extracted fact", tags: ["Project"], folder_path: "Projects" },
    new: [], update: [{ title: "Missing title", content: "Never applies", reason: "model reference drift" }],
    merge: [{ source_titles: ["Missing title", "Valid extraction"], title: "Merged", content: "Never applies", tags: ["Project"], folder_path: "Projects", reason: "model reference drift" }],
    links: [{ source: "Missing title", target: "Valid extraction", type: "RELATED", description: "Never applies", weight: 0.5 }] });
  await runtime.interval(); harness.assertOutputsConsumed(); assert.equal(harness.modelCalls.length, 1);
  const saved = await readOwner(harness);
  assert.equal(saved.candidates.length, 0); assert.equal(saved.memories.length, 1); assert.equal(saved.memories[0].title, "Valid extraction");
});

/** Requeues genuine terminal failures for a later real extraction and removes failures on explicit clear. */
test("controlled Chat/AI: retryFailed requeues terminal failures and clearFailed removes them without retrying", async t => {
  const harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime(); await runtime.initialize(); await runtime.bind();
  await runtime.enqueueReplies(); await runtime.makeDue();
  const failure = new Error("Controlled model outage"); harness.queueExecutionError(failure);
  await runtime.interval();
  const failed = await readOwner(harness); assert.equal(failed.candidates.length, 5);
  for (const candidate of failed.candidates) { assert.equal(candidate.status, "failed"); assert.equal(candidate.lastError, failure.message); }
  assert.deepEqual(plain(await runtime.api("memory.candidates.retryFailed", owner)), { retried: 5 });
  const retried = await readOwner(harness);
  for (const candidate of retried.candidates) { assert.equal(candidate.status, "pending"); assert.equal(candidate.lastError, ""); }
  await runtime.makeDue(); harness.queueAnalysis({});
  await runtime.interval(); harness.assertOutputsConsumed(); assert.equal(harness.modelCalls.length, 2);
  assert.equal((await readOwner(harness)).candidates.length, 0);
  /** A second terminal failure set is removed directly instead of being processed again. */
  await runtime.enqueueReplies(); await runtime.makeDue(); harness.queueExecutionError(failure);
  await runtime.interval();
  assert.equal((await readOwner(harness)).candidates.length, 5);
  assert.deepEqual(plain(await runtime.api("memory.candidates.clearFailed", owner)), { cleared: 5 });
  assert.equal((await readOwner(harness)).candidates.length, 0);
  assert.equal(harness.modelCalls.length, 3);
});

/** Skips malformed model items so one wrong field type cannot fail the whole candidate batch. */
test("controlled Chat/AI: a malformed analysis item is skipped while the rest of the batch still commits", async t => {
  const harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime(); await runtime.initialize(); await runtime.bind();
  await runtime.enqueueReplies(); await runtime.makeDue();
  harness.queueAnalysis({ main: { title: "Surviving fact", content: "Committed despite model drift", tags: ["Project"], folder_path: "Projects" },
    new: [{ title: "Broken entity", content: "Missing required fields" }],
    update: [], merge: [],
    links: [{ source: "Surviving fact", target: "Surviving fact", type: 42, description: "Wrong field type", weight: 0.5 }] });
  await runtime.interval(); harness.assertOutputsConsumed(); assert.equal(harness.modelCalls.length, 1);
  const saved = await readOwner(harness);
  assert.equal(saved.candidates.length, 0); assert.equal(saved.memories.length, 1); assert.equal(saved.memories[0].title, "Surviving fact");
  assert.equal(saved.links.length, 0);
  /** The manual update reports the exact number of skipped malformed items for the same drift. */
  harness.queueAnalysis({ main: null, new: [], update: [], merge: [], links: [{ source: "Surviving fact", target: "Surviving fact", type: null, description: "Wrong field type", weight: 0.5 }] });
  const result = plain(await runtime.api("memory.chat.update", { ownerKey: CONTROLLED_OWNER, chatId: CONTROLLED_CHAT_ID }));
  assert.equal(result.skipped, 1); assert.equal(result.links, 0);
});

/** Preserves both original failures when execution and real status-file publication fail, for each production job path. */
test("controlled Chat/AI: rebuild and candidate execution plus state-publication failures retain both errors without poisoning unrelated reads", async t => {
  for (const [kind, publication] of [["rebuild", "io"], ["candidates", "io"], ["rebuild", "conflict"], ["candidates", "conflict"]]) {
    const harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime(); await runtime.initialize(); await runtime.bind();
    if (kind === "rebuild") await runtime.startRebuild();
    else { await runtime.enqueueReplies(); await runtime.makeDue(); }
    const failure = new Error("Controlled execution failure: " + kind), statusFailure = new Error(publication === "conflict" ? "Storage version conflict: owners/" + CONTROLLED_OWNER : "Controlled native state publication failure: " + kind);
    const before = await readOwner(harness); harness.disk.clearCalls(); harness.queueExecutionAndStatusError(failure, statusFailure);
    await assert.rejects(runtime.interval(),
      /** Checks exact preserved error identities from both unsuccessful phases. */
      error => error.name === "MemoryJobPassError" && error.failures.length === 1
        && error.failures[0].error.name === "MemoryJobStatusError"
        && error.failures[0].error.executionError === failure && error.failures[0].error.statusPublicationError === statusFailure,
    );
    harness.assertOutputsConsumed(); const stored = await readOwner(harness); assert.deepEqual(stored.memories, before.memories);
    if (kind === "rebuild") { assert.equal(stored.rebuildProgress.status, "preparing"); assert.equal(stored.rebuildProgress.completedWindows, 0); assert.equal(stored.rebuildProgress.failedWindows, 0); }
    else { assert.equal(stored.candidates.length, 5); for (const candidate of stored.candidates) { assert.equal(candidate.status, "processing"); assert.equal(candidate.attemptCount, 1); } }
    assert.ok((await runtime.api("memory.rebuild.progress", owner)).status.length > 0);
    const writes = harness.disk.calls.filter(call => call.method === "storage.commit");
    assert.ok(writes.length > 0, "The failure must occur during an actual storage transaction");
    /** Locates the attempted failure publication by its actual collection. */
    const publicationCollection = kind === "rebuild" ? "owners" : "candidates";
    const unpublished = [...writes].reverse().find(
      call => call.args[0].some(mutation => mutation.op === "put" && mutation.collection === publicationCollection),
    ).args[0];
    if (kind === "rebuild") {
      const owner = unpublished.find(mutation => mutation.collection === "owners" && mutation.op === "put");
      assert.equal(owner.value.rebuildProgress.status, "running");
      assert.equal(owner.value.rebuildTask.nextWindow, 1);
      assert.equal(owner.value.rebuildProgress.failedWindows, 1);
      assert.equal(owner.value.rebuildProgress.lastError, failure.message);
    } else {
      const candidates = unpublished.filter(mutation => mutation.collection === "candidates" && mutation.op === "put");
      assert.ok(candidates.length > 0);
      for (const row of candidates) {
        assert.equal(row.value.record.status, "failed"); assert.equal(row.value.record.lastError, failure.message);
      }
    }
  }
});
/** Verifies a genuine nonzero selected fixture revision through main's registered persistence hook and interval. */
test("controlled Chat/AI: persistence hook maps Core selectedVariantIndex to the saved nonzero candidate revision", async t => {
  const harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime(); await runtime.initialize(); await runtime.bind();
  const timestamp = harness.message(31).timestamp; await harness.selectNewVariant(timestamp); await harness.selectNewVariant(timestamp);
  const selected = harness.message(31); assert.equal(selected.variantIndex, 2);
  await harness.removeChatMarker(); const sources = await runtime.enqueueReplies();
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
test("controlled Chat/AI: registered candidate provider and interval use exact message authors and current permissions without reading current chat selection", async t => {
  const harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime(); await runtime.initialize(); await runtime.bind();
  await enqueueProviderReplies(runtime, harness);
  const before = await readOwner(harness);
  assert.equal(before.candidates.length, 5);
  for (const candidate of before.candidates) assert.equal(candidate.triggerVariantIndex, 0);
  assert.equal(Object.hasOwn(await harness.readState(), "chatBindings"), false);
  await harness.removeChatMarker(); const readCount = harness.extensionCalls.length; await runtime.makeDue();
  harness.queueAnalysis({ main: { title: "Snapshot ownership", content: "Original owner remains the only destination", tags: ["Snapshot"], folder_path: "Projects" }, new: [], update: [], merge: [], links: [] });
  await runtime.interval(); harness.assertOutputsConsumed();
  const saved = await readOwner(harness); assert.equal(saved.candidates.length, 0); assert.equal(saved.memories.length, 1);
  assert.equal(saved.memories[0].title, "Snapshot ownership");
  for (const call of harness.extensionCalls.slice(readCount)) assert.equal(call.target.kind, "message", "Message author lookup must not read current chat identity");
});

/** Checks strict missing-snapshot and required-revision errors without manufacturing an owner or publishing a candidate. */
test("controlled Chat/AI: candidate provider rejects missing snapshots or absent variant input without a disk mutation", async t => {
  const harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime(); await runtime.initialize(); await runtime.bind();
  const source = harness.message(31), before = await harness.readStateText(); harness.disk.clearCalls();
  await assert.rejects(() => runtime.api("memory.candidate.enqueue", { chatId: CONTROLLED_CHAT_ID, timestamp: source.timestamp, sourceType: "reply_finalized_auto", ownerKey: null }), /variantIndex is required/);
  harness.removeExecutionData(source.timestamp, source.variantIndex);
  await assert.rejects(() => runtime.api("memory.candidate.enqueue", { chatId: CONTROLLED_CHAT_ID, timestamp: source.timestamp, variantIndex: source.variantIndex, sourceType: "reply_finalized_auto", ownerKey: null }), /has no character author identity/);
  assert.equal(await harness.readStateText(), before); assert.equal(mutations(harness.disk).length, 0); assert.equal(harness.modelCalls.length, 0);
});

/** Skips an obsolete selected revision without substituting the newly selected message into extraction. */
test("controlled Chat/AI: a changed candidate revision is retained as failed while valid candidates are extracted", async t => {
  const harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime(); await runtime.initialize(); await runtime.bind();
  const sources = await enqueueProviderReplies(runtime, harness); await harness.selectNewVariant(sources[0].timestamp); await runtime.makeDue();
  harness.queueAnalysis({}); await runtime.interval(); harness.assertOutputsConsumed();
  const saved = await readOwner(harness); assert.equal(saved.candidates.length, 1); assert.equal(harness.modelCalls.length, 1);
  const failed = saved.candidates[0]; assert.equal(failed.triggerMessageTimestamp, sources[0].timestamp);
  assert.equal(failed.triggerVariantIndex, 0); assert.equal(failed.status, "failed"); assert.equal(failed.attemptCount, 1);
  assert.match(failed.lastError, /源消息已删除或所选版本已变化/);
  assert.ok(!harness.modelCalls[0].turns[1].content.includes("[Controlled selected variant]"));
  assert.ok(!harness.modelCalls[0].turns[1].content.includes(sources[0].content));
  const restarted = harness.openRuntime(); await restarted.initialize(); await restarted.makeDue(); await restarted.interval();
  assert.deepEqual((await readOwner(harness)).candidates, saved.candidates); assert.equal(harness.modelCalls.length, 1);
});

/** Reclaims claims orphaned by an interrupted runtime once their lease expires while leaving fresh claims alone. */
test("controlled Chat/AI: an interrupted processing claim is reclaimed after its lease and fresh claims are left untouched", async t => {
  const harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime(); await runtime.initialize(); await runtime.bind();
  await runtime.enqueueReplies();
  /** Ages every claim as if the previous runtime died while the batch was running. */
  const state = await harness.readState(), space = state.owners.filter(owner => owner.ownerKey === CONTROLLED_OWNER)[0];
  assert.equal(space.candidates.length, 5);
  for (const candidate of space.candidates) { candidate.status = "processing"; candidate.attemptCount = 1; candidate.updatedAt = Date.now() - 31 * 60 * 1000; }
  harness.disk.replaceState(state);
  await runtime.makeDue(); harness.queueAnalysis({ main: { title: "Reclaimed batch", content: "An orphaned claim runs again", tags: ["Project"], folder_path: "Projects" }, new: [], update: [], merge: [], links: [] });
  await runtime.interval(); harness.assertOutputsConsumed(); assert.equal(harness.modelCalls.length, 1);
  const reclaimed = await readOwner(harness); assert.equal(reclaimed.candidates.length, 0); assert.equal(reclaimed.memories.length, 1);
  /** A fresh claim must survive the same tick so a live slow batch is never duplicated. */
  await enqueueProviderReplies(runtime, harness);
  const fresh = await harness.readState(), freshSpace = fresh.owners.filter(owner => owner.ownerKey === CONTROLLED_OWNER)[0];
  for (const candidate of freshSpace.candidates) { candidate.status = "processing"; candidate.attemptCount = 1; candidate.updatedAt = Date.now(); }
  harness.disk.replaceState(fresh);
  await runtime.makeDue(); await runtime.interval();
  assert.equal(harness.modelCalls.length, 1);
  const claimed = await readOwner(harness); assert.equal(claimed.candidates.length, 5);
  for (const candidate of claimed.candidates) assert.equal(candidate.status, "processing");
});

/** Persists skipped rebuild windows and continues with later source windows across restart. */
test("controlled Chat/AI: deleted rebuild sources and a final model failure do not strand the rebuild cursor", async t => {
  const harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime(); await runtime.initialize(); await runtime.bind();
  await runtime.startRebuild(); harness.deleteMessage(harness.message(1).timestamp);
  await runtime.interval();
  const skipped = await readOwner(harness); assert.equal(skipped.rebuildProgress.status, "running"); assert.equal(skipped.rebuildProgress.failedWindows, 1);
  assert.equal(skipped.rebuildTask.nextWindow, 1); assert.equal(skipped.rebuildProgress.completedWindows, 1); assert.equal(harness.modelCalls.length, 0);
  assert.match(skipped.rebuildProgress.lastError, /planned rebuild source message revision changed or was deleted/);
  const restarted = harness.openRuntime(); await restarted.initialize();
  const failure = new Error("Controlled final rebuild window failure");
  for (let index = 1; index < skipped.rebuildProgress.totalWindows; index += 1) {
    if (index === skipped.rebuildProgress.totalWindows - 1) harness.queueExecutionError(failure);
    else harness.queueAnalysis({});
    await restarted.interval();
  }
  harness.assertOutputsConsumed(); const finished = await readOwner(harness);
  assert.equal(finished.rebuildProgress.status, "completed"); assert.equal(finished.rebuildProgress.failedWindows, 2);
  assert.equal(finished.rebuildTask.nextWindow, finished.rebuildTask.windows.length);
  assert.equal(finished.rebuildProgress.processedSourceMessages, finished.rebuildProgress.totalSourceMessages);
  assert.equal(finished.rebuildProgress.lastError, failure.message);
  assert.equal(harness.modelCalls.length, finished.rebuildProgress.totalWindows - 1);
  await restarted.interval(); assert.equal(harness.modelCalls.length, finished.rebuildProgress.totalWindows - 1);
});

/** Extracts unchanged sources even when the native storage codec returns persisted snapshot keys reordered, while genuine edits still fail. */
test("controlled Chat/AI: alphabetically reordered plan snapshot keys still execute unchanged windows and genuine edits still fail their window", async t => {
  const harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime(); await runtime.initialize(); await runtime.bind();
  await runtime.startRebuild();
  /** Reproduces the production host bridge: persisted object keys return in sorted order instead of construction order. */
  const planned = await harness.readState(), spaces = planned.owners.filter(space => space.ownerKey === CONTROLLED_OWNER);
  assert.equal(spaces.length, 1); assert.ok(spaces[0].rebuildTask !== null);
  for (const window of spaces[0].rebuildTask.windows) window.messages = window.messages.map(
    /** Rewrites each snapshot with sorted keys exactly like the native serde codec round trip. */
    message => Object.fromEntries(Object.entries(message).sort(([left], [right]) => left < right ? -1 : left > right ? 1 : 0)),
  );
  harness.disk.replaceState(planned);
  harness.queueAnalysis({}); await runtime.interval(); harness.assertOutputsConsumed();
  const running = plain(await runtime.api("memory.rebuild.progress", owner));
  assert.equal(running.status, "running"); assert.equal(running.completedWindows, 1); assert.equal(harness.modelCalls.length, 1);
  /** Preserves revision validation: a changed second-window source is recorded and its window is skipped. */
  harness.deleteMessage(harness.message(8).timestamp);
  await runtime.interval();
  const failed = await readOwner(harness);
  assert.equal(failed.rebuildProgress.status, "running"); assert.equal(failed.rebuildProgress.failedWindows, 1); assert.equal(failed.rebuildProgress.completedWindows, 2);
  assert.equal(failed.rebuildTask.nextWindow, 2);
  assert.match(failed.rebuildProgress.lastError, /planned rebuild source message revision changed or was deleted: memory-jobs-controlled-chat at \d+\/0/u);
  assert.equal(harness.modelCalls.length, 1);
});

/** Prunes memory owners whose backing entity vanished instead of failing every interval transaction on them. */
test("controlled Chat/AI: interval prunes orphan memory owners and still serves the genuine owner", async t => {
  const harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime();
  await runtime.initialize(); await runtime.bind();
  const sources = await runtime.enqueueReplies(); await runtime.persistMessage(sources[0]);
  /** Injects the exact production wound: an owner whose character record was lost outside the removal path. */
  const state = await harness.readState(), orphan = plain(loadModule("src/storage/state.ts").createMemorySpace("character:90a3122d-b374-4422-9589-44a9d7fc18a9"));
  orphan.settings.nextAutoSaveRunAtMs = 0; orphan.candidates.push({ id: "orphan-candidate", chatId: CONTROLLED_CHAT_ID, triggerMessageTimestamp: harness.message(1).timestamp, triggerVariantIndex: 0, createdAt: 1, updatedAt: 1, status: "pending", attemptCount: 0, lastError: "", sourceType: "reply_finalized_auto" });
  state.owners.push(orphan); harness.disk.replaceState(state);
  const before = await harness.readState();
  assert.equal(before.owners.some(space => space.ownerKey === "character:90a3122d-b374-4422-9589-44a9d7fc18a9"), true);

  harness.queueAnalysis({ main: { title: "Surviving owner", content: "A fact from the genuine owner", tags: ["Survivor"], folder_path: "Projects" }, new: [], update: [], merge: [], links: [], profile_markdown: "# User\nSurviving profile\n" });
  await runtime.makeDue();
  await runtime.interval();
  harness.assertOutputsConsumed();

  const after = await harness.readState();
  assert.equal(after.owners.some(space => space.ownerKey === "character:90a3122d-b374-4422-9589-44a9d7fc18a9"), false);
  const surviving = after.owners.filter(space => space.ownerKey === CONTROLLED_OWNER);
  assert.equal(surviving.length, 1); assert.ok(surviving[0].memories.some(memory => memory.title === "Surviving owner"));
  /** A second interval runs cleanly without the orphan and keeps making progress for the genuine owner. */
  await runtime.interval();
  assert.equal((await harness.readState()).owners.some(space => space.ownerKey === "character:90a3122d-b374-4422-9589-44a9d7fc18a9"), false);
});


/** Owner failures stay visible while independent owners complete and later intervals remain usable. */
test("memory jobs isolate real owner failures without swallowing composite publication errors", async () => {
  const { MemoryJobRunner, MemoryJobStatusError } = loadModule("src/memory-jobs/scheduler.ts", {
    /** Supplies only the executing device identity for this scheduler boundary test. */
    async toolCall() { return JSON.stringify({ currentNodeId: "controlled-node", nodes: [] }); },
  });
  for (const failure of [new Error("Storage version conflict: owners/first"), new MemoryJobStatusError(new Error("model failed"), new Error("Storage version conflict: owners/first"))]) {
    let failFirst = true;
    const visited = new Set();
    const owner = {
      /** Exposes the actual scheduling boundary without replacing the runner's failure policy. */
      async run(callback) {
        return callback({
          /** Keeps this fixture focused on initialized owners. */
          async pruneOrphanMemoryOwners() { return 0; },
          /** Schedules the failing owner before a healthy independent owner. */
          async listMemoryOwnerKeys() { return ["first", "healthy"]; },
          /** Retains the original failure identity or reads a genuine idle job state. */
          async readMemoryJobState(ownerKey) {
            visited.add(ownerKey);
            if (ownerKey === "first" && failFirst) throw failure;
            return { candidates: [], rebuildTask: null, rebuildProgress: { status: "idle" } };
          },
          /** Prevents candidate work while testing owner error isolation. */
          async readGlobalMemoryPreferences() { return { autoUpdateEnabled: false }; },
        });
      },
    };
    const runner = new MemoryJobRunner(owner);
    await assert.rejects(runner.tick(),
      /** Requires both the original error and the completed independent owner's report. */
      error => error.name === "MemoryJobPassError" && error.failures.length === 1
        && error.failures[0].ownerKey === "first" && error.failures[0].error === failure
        && error.report.owners === 2,
    );
    assert.ok(visited.has("healthy"));
    failFirst = false;
    assert.equal((await runner.tick()).owners, 2);
  }
});


/** Independent replicas never reclaim another device's work and can process their own sources while disconnected. */
test("producer ownership: independent devices process only the replies they produced", async t => {
  const a = await createMemoryJobsHarness(t, "node-a"), b = await createMemoryJobsHarness(t, "node-b");
  const first = a.openRuntime(), second = b.openRuntime();
  await first.initialize(); await first.bind(); await second.initialize(); await second.bind();
  await first.enqueueReplies();
  const replicated = await a.readState(), source = replicated.owners.find(space => space.ownerKey === CONTROLLED_OWNER);
  assert.ok(source.candidates.every(candidate => candidate.producerNodeId === "node-a"));
  source.candidates[0].status = "processing"; source.candidates[0].updatedAt = 1;
  a.disk.replaceState(replicated); b.disk.replaceState(replicated);
  const before = b.disk.stateBytes(); await second.interval();
  assert.deepEqual(b.disk.stateBytes(), before); assert.equal(b.modelCalls.length, 0);

  for (const index of [1, 3, 5, 7, 9]) await second.persistMessage(b.message(index));
  await second.makeDue(); b.queueAnalysis({}); await second.interval(); b.assertOutputsConsumed();
  const remaining = (await readOwner(b)).candidates;
  assert.equal(b.modelCalls.length, 1); assert.deepEqual(remaining, source.candidates);

  // The producer resumes its original work on restart without contacting the other device.
  const restarted = a.openRuntime(); await restarted.initialize(); await restarted.makeDue();
  a.queueAnalysis({}); await restarted.interval(); a.assertOutputsConsumed();
  assert.equal(a.modelCalls.length, 1); assert.equal((await readOwner(a)).candidates.length, 0);
});

/** A synchronized rebuild plan retains its requesting device instead of starting on every replica. */
test("producer ownership: a remote rebuild is read-only until its requester resumes", async t => {
  const a = await createMemoryJobsHarness(t, "node-a"), b = await createMemoryJobsHarness(t, "node-b");
  const first = a.openRuntime(), second = b.openRuntime();
  await first.initialize(); await first.bind(); await second.initialize(); await second.bind();
  await first.startRebuild(); const state = await a.readState();
  assert.equal(state.owners.find(space => space.ownerKey === CONTROLLED_OWNER).rebuildTask.producerNodeId, "node-a");
  b.disk.replaceState(state); const before = b.disk.stateBytes();
  await second.interval(); assert.deepEqual(b.disk.stateBytes(), before); assert.equal(b.modelCalls.length, 0);
  a.queueAnalysis({}); await first.interval(); a.assertOutputsConsumed();
  assert.equal((await readOwner(a)).rebuildTask.nextWindow, 1);
});

/** Released unassigned work stays visible and requires an explicit retry rather than an invented producer. */
test("producer ownership: legacy candidates fail visibly and explicit retry records the requesting device", async t => {
  const harness = await createMemoryJobsHarness(t, "node-b"), runtime = harness.openRuntime();
  await runtime.initialize(); await runtime.bind(); await runtime.enqueueReplies();
  const state = await harness.readState();
  for (const candidate of state.owners.find(space => space.ownerKey === CONTROLLED_OWNER).candidates) delete candidate.producerNodeId;
  harness.disk.replaceState(state); await runtime.interval();
  const failed = (await readOwner(harness)).candidates;
  assert.equal(harness.modelCalls.length, 0);
  assert.ok(failed.every(candidate => candidate.status === "failed" && candidate.lastError.includes("no producing device")));
  assert.deepEqual(plain(await runtime.web({ action: "retryFailedCandidates", ownerKey: CONTROLLED_OWNER })), { retried: 5 });
  assert.ok((await readOwner(harness)).candidates.every(candidate => candidate.producerNodeId === "node-b"));
  await runtime.makeDue(); harness.queueAnalysis({}); await runtime.interval(); harness.assertOutputsConsumed();
  assert.equal((await readOwner(harness)).candidates.length, 0);
});

/** Missing provenance on a released rebuild never assigns execution to whichever device first sees it. */
test("producer ownership: legacy rebuilds require an explicit new request", async t => {
  const harness = await createMemoryJobsHarness(t, "node-b"), runtime = harness.openRuntime();
  await runtime.initialize(); await runtime.bind(); await runtime.startRebuild();
  const state = await harness.readState();
  delete state.owners.find(space => space.ownerKey === CONTROLLED_OWNER).rebuildTask.producerNodeId;
  harness.disk.replaceState(state); await runtime.interval();
  const failed = await readOwner(harness);
  assert.equal(harness.modelCalls.length, 0); assert.equal(failed.rebuildProgress.status, "failed");
  assert.match(failed.rebuildProgress.lastError, /no producing device/u);
  await runtime.startRebuild(); harness.queueAnalysis({}); await runtime.interval(); harness.assertOutputsConsumed();
  assert.equal((await readOwner(harness)).rebuildTask.producerNodeId, "node-b");
});

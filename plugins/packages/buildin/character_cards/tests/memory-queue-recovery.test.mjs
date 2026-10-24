import assert from "node:assert/strict";
import test from "node:test";
import { loadModule, plain } from "./runtime.mjs";
import { CONTROLLED_OWNER, createMemoryJobsHarness } from "./memory-jobs-runtime.mjs";

/** Reads one genuine owner from native bytes without inventing a queue. */
async function readOwner(harness) {
  const state = await harness.readState(), matches = [];
  for (const space of state.owners) if (space.ownerKey === CONTROLLED_OWNER) matches.push(space);
  assert.equal(matches.length, 1); return matches[0];
}

/** Exposes recovery actions only for genuine failed candidates and requires an explicit clear confirmation. */
test("memory queue status renders retry and confirmed clear only while failed candidates exist", () => {
  const { memoryQueueStatus } = loadModule("web/features/memory/controls-view.ts");
  const queue = failedCandidates => ({ ownerKey: "character:default", pendingCandidates: 2, pendingChats: 1, processingCandidates: 0,
    failedCandidates, nextRunAtMs: 0, minutesUntilNextRun: 0, lastError: "" });
  const dialog = (value, pendingClearFailed) => ({ queue: value, pendingClearFailed, ownerKey: "character:default" });
  const idle = memoryQueueStatus(dialog(queue(0), false));
  assert.doesNotMatch(idle, /retry-failed-candidates|clear-failed-candidates/u);
  const failed = memoryQueueStatus(dialog(queue(3), false));
  assert.match(failed, /data-action="retry-failed-candidates"/u); assert.match(failed, /data-action="clear-failed-candidates"/u);
  assert.match(failed, /失败 3/u); assert.doesNotMatch(failed, /confirm-clear-failed-candidates/u);
  const confirming = memoryQueueStatus(dialog(queue(3), true));
  assert.match(confirming, /data-action="confirm-clear-failed-candidates"/u); assert.match(confirming, /data-action="cancel-clear-failed-candidates"/u);
  assert.doesNotMatch(confirming, /data-action="retry-failed-candidates"|data-action="clear-failed-candidates"/u);
  assert.match(confirming, /不影响已写入的记忆/u);
  assert.match(memoryQueueStatus(dialog({ ...queue(1), lastError: "<failed>" }, false)), /&lt;failed&gt;/u);
});

/** Routes owner-scoped recovery through the real WebView UI channel instead of a chat-derived owner. */
test("controlled Chat/AI: the settings page UI channel retries and clears owner-scoped failures", async t => {
  const harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime(); await runtime.initialize(); await runtime.bind();
  await runtime.enqueueReplies(); await runtime.makeDue();
  const failure = new Error("Controlled model outage"); harness.queueExecutionError(failure);
  await assert.rejects(runtime.interval(), error => error === failure);
  assert.equal((await readOwner(harness)).candidates.length, 5);
  assert.deepEqual(plain(await runtime.web({ action: "retryFailedCandidates", ownerKey: CONTROLLED_OWNER })), { retried: 5 });
  const retried = await readOwner(harness);
  for (const candidate of retried.candidates) assert.equal(candidate.status, "pending");
  await runtime.makeDue(); harness.queueAnalysis({});
  await runtime.interval(); harness.assertOutputsConsumed();
  assert.equal((await readOwner(harness)).candidates.length, 0);
  /** A second genuine terminal set is removed through the explicit confirmation request. */
  await runtime.enqueueReplies(); await runtime.makeDue(); harness.queueExecutionError(failure);
  await assert.rejects(runtime.interval(), error => error === failure);
  assert.equal((await readOwner(harness)).candidates.length, 5);
  assert.deepEqual(plain(await runtime.web({ action: "clearFailedCandidates", ownerKey: CONTROLLED_OWNER })), { cleared: 5 });
  assert.equal((await readOwner(harness)).candidates.length, 0);
});

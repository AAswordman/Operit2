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

/** Exercises empty, small and oversized queues through production IPC with bounded writes and untouched active records. */
for (const count of [0, 5, 1000, 1001, 1486]) for (const action of ["retryFailedCandidates", "clearFailedCandidates"]) test(`${action}: ${count} failed candidates`, async t => {
  const harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime(); await runtime.initialize(); await runtime.bind();
  await runtime.enqueueReplies(); const seed = (await readOwner(harness)).candidates[0];
  harness.disk.storage.inspect(sql => {
    sql.exec("BEGIN; DELETE FROM records WHERE collection='candidates'");
    const insert = sql.prepare("INSERT INTO records VALUES('candidates',?,?,'fixture-version')");
    for (let index = 0; index < count + 2; index += 1) {
      const record = { ...seed, id: String(index).padStart(5, "0"), status: index < count ? "failed" : index === count ? "pending" : "processing", lastError: index < count ? "SQL parse failure" : "" };
      insert.run(CONTROLLED_OWNER + "/" + record.id, JSON.stringify({ ownerKey: CONTROLLED_OWNER, record }));
    }
    sql.exec("COMMIT");
  });
  harness.disk.clearCalls();
  assert.deepEqual(plain(await runtime.web({ action, ownerKey: CONTROLLED_OWNER })), action === "retryFailedCandidates" ? { retried: count } : { cleared: count });
  const commits = harness.disk.calls.filter(call => call.method === "storage.commit");
  assert.equal(commits.length, Math.ceil(count / 100)); for (const call of commits) assert.ok(call.args[0].length <= 100);
  const candidates = (await readOwner(harness)).candidates;
  assert.equal(candidates.length, action === "retryFailedCandidates" ? count + 2 : 2);
  assert.equal(candidates.filter(record => record.status === "processing").length, 1);
  for (const record of candidates) if (record.status !== "processing") { assert.equal(record.status, "pending"); assert.equal(record.lastError, ""); }
});

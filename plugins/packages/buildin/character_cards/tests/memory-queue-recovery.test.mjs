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

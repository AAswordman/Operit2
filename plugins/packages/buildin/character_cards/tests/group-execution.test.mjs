import assert from "node:assert/strict";
import test from "node:test";
import { controlledReceipt, deferred, groupEnvironment } from "./group-execution-runtime.mjs";
import { plain } from "./runtime.mjs";

const userTimestamp = 1700000000200;
/** Acknowledges the exact controlled original input without generating an assistant. */
function recorded(input) { assert.equal(input.kind, "record_only"); return controlledReceipt(input, userTimestamp, null); }
/** Produces only the explicitly controlled assistant locator for a continuation. */
function replied(input, index) { return controlledReceipt(input, userTimestamp, { messageTimestamp: 1700000000300 + index, variantIndex: index }); }
/** Rejects test capabilities that a scenario does not explicitly authorize. */
function forbidden(method) { throw new Error("Unprovided controlled transport capability: " + method); }

/** Records complete input before the planner and requests summaries only after the actual final reply. */
test("executor: persists input before planning and executes ordered continuations with one final summary check", async t => {
  let index = 0;
  const env = await groupEnvironment(t, {
    /** Acknowledges the sole original input before any functional-model call. */
    async submit(input) { assert.equal(env.modelCalls.length, 0); return recorded(input); },
    /** Returns a distinct persisted assistant revision for each selected speaker. */
    async continue(input) { return replied(input, ++index); },
    /** Rejects unexpected cancellation in the successful sequence. */
    async cancel() { forbidden("cancel"); },
    /** Requires the input receipt and all three completed participant receipts. */
    async finish(input) { assert.equal(input.receipts.length, 4); assert.equal(input.userMessageTimestamp, userTimestamp); },
    /** Rejects failure cleanup in the successful sequence. */
    async abandon() { forbidden("abandon"); },
  });
  env.setPlan({ rounds: [[{ id: env.a.id, speak: true }, { id: env.b.id, speak: true }], [], [{ id: env.a.id, speak: true }]] });
  const original = env.input(), result = plain(await env.controller.submit(original));
  assert.equal(result.status, "completed"); assert.equal(result.cursor, 3); assert.equal(result.plannedTurns, 3);
  assert.deepEqual(env.calls.map(call => call.method), ["submit", "continue", "continue", "continue", "finish"]);
  assert.deepEqual(env.calls[0].input.submission, original); assert.equal(env.calls[0].input.kind, "record_only");
  const turns = env.calls.slice(1, 4).map(call => call.input);
  assert.deepEqual(turns.map(input => input.turn.participantId), [env.a.id, env.b.id, env.a.id]);
  assert.deepEqual(turns.map(input => input.turn.notifyReply), [false, false, true]);
  assert.deepEqual(turns.map(input => input.turn.summaryPolicy), ["defer", "defer", "after_reply"]);
  for (const input of turns) { assert.equal(input.userMessageTimestamp, userTimestamp); assert.equal(Object.hasOwn(input, "text"), false); assert.equal(Object.hasOwn(input, "attachments"), false); }
  assert.equal(env.modelCalls.length, 1); assert.equal(result.receipts[0].assistant, null);
});

/** A failed planner leaves the original text, attachments and reply acknowledged before the real rejection. */
test("executor: planner failure preserves the recorded input and never sends a participant or retries", async t => {
  const failure = new Error("Original configured planner failure");
  const env = await groupEnvironment(t, {
    /** Records the complete original input before invoking the failing planner. */
    async submit(input) { return recorded(input); },
    /** Rejects all generation after planning fails. */
    async continue() { forbidden("continue"); },
    /** No active native send remains after input persistence. */
    async cancel() { forbidden("cancel"); },
    /** Failed plans cannot report successful completion. */
    async finish() { forbidden("finish"); },
    /** Releases only the failed plugin-local planning sequence. */
    async abandon(input) { assert.equal(input.chatId, env.chatId); },
  });
  const rejected = Promise.reject(failure); rejected.catch(() => {}); env.setRawPlan(rejected);
  const input = env.input(), run = env.controller.submit(input);
  await assert.rejects(run, error => error === failure);
  assert.deepEqual(env.calls.map(call => call.method), ["submit", "abandon"]);
  assert.deepEqual(env.calls[0].input.submission, input);
  assert.equal(env.controller.submit(input), run); await assert.rejects(env.controller.submit(input), error => error === failure);
  assert.equal(env.modelCalls.length, 1); assert.throws(() => env.controller.resume(env.chatId, input.submissionId), /settled cancelled/);
});

/** Malformed plans preserve the already committed input without choosing an automatic speaker. */
test("executor: malformed planning output stops after genuine user-only persistence", async t => {
  const env = await groupEnvironment(t, {
    /** Records input even when the later model output is invalid. */
    async submit(input) { return recorded(input); },
    /** No speaker may be selected from a malformed plan. */
    async continue() { forbidden("continue"); },
    /** No generation was admitted for cancellation. */
    async cancel() { forbidden("cancel"); },
    /** Rejects fabricated successful finalization. */
    async finish() { forbidden("finish"); },
    /** Acknowledges the failed local planning boundary. */
    async abandon() {},
  });
  env.setRawPlan({ text: '{"rounds":[[{"id":"unknown","speak":true}]]}' });
  await assert.rejects(env.controller.submit(env.input()), /Unknown planned/);
  assert.deepEqual(env.calls.map(call => call.method), ["submit", "abandon"]); assert.equal(env.modelCalls.length, 1);
});

/** Duplicate tokens share their sole persistence, planner and completed result. */
test("executor: duplicate submissions never persist or plan twice", async t => {
  const env = await groupEnvironment(t, {
    /** Acknowledges only one original user record. */
    async submit(input) { return recorded(input); },
    /** Returns the single explicit participant reply. */
    async continue(input) { return replied(input, 1); },
    /** Rejects unrequested cancellation. */
    async cancel() { forbidden("cancel"); },
    /** Requires exactly the input and reply acknowledgements. */
    async finish(input) { assert.equal(input.receipts.length, 2); },
    /** Rejects failure cleanup for a completed plan. */
    async abandon() { forbidden("abandon"); },
  });
  env.setPlan({ rounds: [[{ id: env.a.id, speak: true }]] });
  const input = env.input(), first = env.controller.submit(input); assert.equal(env.controller.submit(plain(input)), first);
  assert.throws(() => env.controller.submit({ ...input, text: "Changed" }), /different input/);
  assert.throws(() => env.controller.submit({ ...input, submissionId: "competing" }), /already owns/);
  await first; assert.equal(env.controller.submit(input), first); assert.equal(env.modelCalls.length, 1); assert.equal(env.calls.length, 3);
});

/** Planning cancellation retains a real user timestamp and resumes without another input or planner invocation. */
test("executor: planning cancellation resumes the already persisted input", async t => {
  const planning = deferred();
  const env = await groupEnvironment(t, {
    /** Commits the input before planning is suspended. */
    async submit(input) { return recorded(input); },
    /** Generates only after explicit resume. */
    async continue(input) { return replied(input, 1); },
    /** Planning has no native generation to cancel. */
    async cancel() { forbidden("cancel"); },
    /** Verifies that resumed execution reused the original user record. */
    async finish(input) { assert.equal(input.userMessageTimestamp, userTimestamp); },
    /** Rejects failure cleanup for cancellation. */
    async abandon() { forbidden("abandon"); },
  });
  env.setRawPlan(planning.promise);
  const input = env.input(), run = env.controller.submit(input); await env.planningStarted;
  await env.controller.cancel(env.chatId, input.submissionId);
  planning.resolve({ text: JSON.stringify({ rounds: [[{ id: env.a.id, speak: true }]] }) });
  const cancelled = plain(await run); assert.equal(cancelled.status, "cancelled"); assert.equal(cancelled.userMessageTimestamp, userTimestamp);
  assert.equal(cancelled.receipts.length, 1); assert.equal(env.calls.length, 1);
  const resumed = plain(await env.controller.resume(env.chatId, input.submissionId));
  assert.equal(resumed.status, "completed"); assert.equal(env.modelCalls.length, 1);
  assert.deepEqual(env.calls.map(call => call.method), ["submit", "continue", "finish"]);
});

/** Active cancellation keeps its exact cursor and partial reply while explicit resume retries only that member. */
test("executor: active cancellation and resume retain exact participant cursor and summary policy", async t => {
  const started = deferred(), sending = deferred(); let replies = 0, active;
  const env = await groupEnvironment(t, {
    /** Persists input independently of the active assistant request. */
    async submit(input) { return recorded(input); },
    /** Suspends the first reply and acknowledges only explicit resumed replies. */
    async continue(input) { if (++replies === 1) { active = input; started.resolve(input); return sending.promise; } return replied(input, replies); },
    /** Validates the exact active correlation and delivers its genuine cancellation receipt. */
    async cancel(input) { assert.equal(input.requestKey, active.turn.requestKey); sending.resolve(controlledReceipt(active, userTimestamp, { messageTimestamp: 1700000000299, variantIndex: 0 }, "cancelled")); },
    /** Requires the input, cancelled attempt and two completed replies. */
    async finish(input) { assert.equal(input.receipts.length, 4); },
    /** Rejects failure cleanup after acknowledged cancellation. */
    async abandon() { forbidden("abandon"); },
  });
  env.setPlan({ rounds: [[{ id: env.a.id, speak: true }, { id: env.b.id, speak: true }]] });
  const input = env.input(), run = env.controller.submit(input); await started.promise;
  await env.controller.cancel(env.chatId, input.submissionId);
  const cancelled = plain(await run); assert.equal(cancelled.status, "cancelled"); assert.equal(cancelled.cursor, 0);
  const resumed = plain(await env.controller.resume(env.chatId, input.submissionId)); assert.equal(resumed.cursor, 2);
  const sends = env.calls.filter(call => call.method === "continue").map(call => call.input);
  assert.deepEqual(sends.map(value => value.turn.participantId), [env.a.id, env.a.id, env.b.id]);
  assert.deepEqual(sends.map(value => value.turn.summaryPolicy), ["defer", "defer", "after_reply"]);
  assert.notEqual(sends[0].turn.requestKey, sends[1].turn.requestKey); assert.equal(env.modelCalls.length, 1);
});

/** Explicit no-response plans keep their recorded input and never trigger participant execution or summaries. */
test("executor: empty and speak-false plans remain user-only", async t => {
  const env = await groupEnvironment(t, {
    /** Records the sole real user input. */
    async submit(input) { return recorded(input); },
    /** No-response plans cannot execute any member. */
    async continue() { forbidden("continue"); },
    /** No model execution exists to cancel. */
    async cancel() { forbidden("cancel"); },
    /** Requires only the acknowledged input. */
    async finish(input) { assert.equal(input.receipts.length, 1); assert.equal(input.receipts[0].assistant, null); },
    /** Rejects failure cleanup for an intentional empty plan. */
    async abandon() { forbidden("abandon"); },
  });
  env.setPlan({ rounds: [[], [{ id: env.b.id, speak: false }]] });
  const result = plain(await env.controller.submit(env.input())); assert.equal(result.status, "completed"); assert.equal(result.plannedTurns, 0);
  assert.deepEqual(env.calls.map(call => call.method), ["submit", "finish"]);
});

/** Removed members stop execution before their continuation, keeping all acknowledged input and replies. */
test("executor: live member removal stops before the next participant", async t => {
  let index = 0;
  const env = await groupEnvironment(t, {
    /** Persists original input without editing its group. */
    async submit(input) { return recorded(input); },
    /** Removes the next member after the first real reply. */
    async continue(input) { assert.equal(++index, 1); await env.service.dispatchDomain("group.update", { id: env.group.id, changes: { members: [{ characterCardId: env.a.id, orderIndex: 0 }] } }); return replied(input, index); },
    /** No cancellation is requested. */
    async cancel() { forbidden("cancel"); },
    /** Failed membership validation cannot complete the sequence. */
    async finish() { forbidden("finish"); },
    /** Releases the failed local execution only. */
    async abandon() {},
  });
  env.setPlan({ rounds: [[{ id: env.a.id, speak: true }, { id: env.b.id, speak: true }]] });
  await assert.rejects(env.controller.submit(env.input()), /deleted or removed/);
  assert.deepEqual(env.calls.map(call => call.method), ["submit", "continue", "abandon"]);
});

/** Input storage failures prevent the planner from consuming the user's request. */
test("executor: original persistence failure propagates before planning", async t => {
  const failure = new Error("Original input storage failure");
  const env = await groupEnvironment(t, {
    /** Rejects with the original persistence failure. */
    async submit() { throw failure; },
    /** No continuation exists after failed persistence. */
    async continue() { forbidden("continue"); },
    /** No active model request exists. */
    async cancel() { forbidden("cancel"); },
    /** Failed persistence cannot finish. */
    async finish() { forbidden("finish"); },
    /** Disposes only the admitted local attempt. */
    async abandon() {},
  });
  await assert.rejects(env.controller.submit(env.input()), error => error === failure); assert.equal(env.modelCalls.length, 0);
});

/** A continuation must identify the same genuinely persisted input. */
test("executor: changed continuation user timestamps are rejected", async t => {
  const env = await groupEnvironment(t, {
    /** Acknowledges the original input. */
    async submit(input) { return recorded(input); },
    /** Intentionally supplies an unrelated committed user identity. */
    async continue(input) { return controlledReceipt(input, userTimestamp + 1, { messageTimestamp: 1700000000301, variantIndex: 1 }); },
    /** No cancellation is requested. */
    async cancel() { forbidden("cancel"); },
    /** Invalid receipts cannot finish. */
    async finish() { forbidden("finish"); },
    /** Releases only the failed local receipt validation. */
    async abandon() {},
  });
  env.setPlan({ rounds: [[{ id: env.a.id, speak: true }]] }); await assert.rejects(env.controller.submit(env.input()), /different user message/);
});

/** Independent native send and cancellation failures both remain observable. */
test("executor: concurrent send and cancellation failures preserve both errors", async t => {
  const started = deferred(), pending = deferred(), sendError = new Error("Send failed"), cancelError = new Error("Cancel failed");
  const env = await groupEnvironment(t, {
    /** Acknowledges the input before the controlled generation failure. */
    async submit(input) { return recorded(input); },
    /** Suspends the participant request until both failures are observed. */
    async continue() { started.resolve(); return pending.promise; },
    /** Returns the independent cancellation failure. */
    async cancel() { throw cancelError; },
    /** Failed sends cannot finish successfully. */
    async finish() { forbidden("finish"); },
    /** Acknowledges disposal of the failed local plan. */
    async abandon() {},
  });
  env.setPlan({ rounds: [[{ id: env.a.id, speak: true }]] }); const input = env.input(), run = env.controller.submit(input); await started.promise;
  await assert.rejects(env.controller.cancel(env.chatId, input.submissionId), error => error === cancelError); pending.reject(sendError);
  await assert.rejects(run, error => error.executionFailure === sendError && error.cancellationFailure === cancelError);
});

/** Cancellation before input persistence can resubmit only after explicit resume and cannot invent a user timestamp. */
test("executor: cancellation before persistence resumes the original input once", async t => {
  let records = 0;
  const env = await groupEnvironment(t, {
    /** Reports the uncommitted cancellation once, then acknowledges actual persistence. */
    async submit(input) { return ++records === 1 ? controlledReceipt(input, null, null, "cancelled") : recorded(input); },
    /** Replies only after the input has actually been recorded. */
    async continue(input) { return replied(input, 1); },
    /** No separate cancellation request is supplied. */
    async cancel() { forbidden("cancel"); },
    /** Requires the actual final user identity. */
    async finish(input) { assert.equal(input.userMessageTimestamp, userTimestamp); },
    /** Rejects failure cleanup for explicit cancellation. */
    async abandon() { forbidden("abandon"); },
  });
  env.setPlan({ rounds: [[{ id: env.a.id, speak: true }]] }); const input = env.input(), cancelled = plain(await env.controller.submit(input));
  assert.equal(cancelled.userMessageTimestamp, null); assert.equal(env.modelCalls.length, 0);
  assert.equal((await env.controller.resume(env.chatId, input.submissionId)).status, "completed"); assert.equal(records, 2); assert.equal(env.modelCalls.length, 1);
});

for (const disposition of [{ type: "blocked", message: "Native input blocked" }, { type: "consumed", metadata: { owner: "other-plugin" } }, { type: "not_persisted", status: "completed" }]) {
  /** Noncommit results stop before planning and cannot produce a reply or summary. */
  test("executor: native " + disposition.type + " stops before planning", async t => {
    const env = await groupEnvironment(t, {
      /** Returns the exact controlled noncommit outcome. */
      async submit(input) { return { chatId: input.submission.chatId, message: input.submission.text, sentAt: 1700000000000, outcome: plain(disposition) }; },
      /** No continuation is authorized. */
      async continue() { forbidden("continue"); },
      /** No model request exists to cancel. */
      async cancel() { forbidden("cancel"); },
      /** Noncommit input cannot finish as a persisted plan. */
      async finish() { forbidden("finish"); },
      /** Acknowledges disposal of the stopped local submission. */
      async abandon() {},
    });
    const input = env.input(), result = plain(await env.controller.submit(input)); assert.equal(result.status, disposition.type);
    assert.equal(result.userMessageTimestamp, null); assert.deepEqual(result.disposition, disposition); assert.equal(env.modelCalls.length, 0);
    assert.deepEqual(plain(await env.controller.submit(input)), result); assert.throws(() => env.controller.resume(env.chatId, input.submissionId), /settled cancelled/);
  });
}

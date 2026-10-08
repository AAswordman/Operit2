import assert from "node:assert/strict";
import test from "node:test";
import { controlledReceipt, deferred, groupEnvironment } from "./group-execution-runtime.mjs";
import { loadModule, plain } from "./runtime.mjs";

const markers = loadModule("src/chat-markers.ts");
const userTimestamp = 1700000000200;

/** Rejects test capabilities that a scenario does not explicitly provide instead of making them succeed as no-ops. */
function forbidden(method) { throw new Error("Unprovided controlled transport capability: " + method); }

/** Exercises real disk/service configuration and real planner code with controlled send receipts across several rounds. */
test("executor: one full submit, ordered opaque participants/receipts and local finalization after the entire plan", async t => {
  let continuationIndex = 0;
  const env = await groupEnvironment(t, {
    /** Verifies only disposal of this local planning fixture; no native success is fabricated. */
    async abandon(input) { assert.equal(input.chatId, env.chatId); },
    /** Returns the first explicitly scheduled persisted assistant revision. */
    async submit(input) { return controlledReceipt(input, userTimestamp, { messageTimestamp: 1700000000300, variantIndex: 7 }); },
    /** Returns distinct true fixture reply locators while retaining the same original user timestamp. */
    async continue(input) { continuationIndex++; return controlledReceipt(input, userTimestamp, { messageTimestamp: 1700000000300 + continuationIndex, variantIndex: continuationIndex }); },
    /** Cancellation is not provided or called in the successful scenario. */
    async cancel() { forbidden("cancel"); },
    /** Acknowledges only the explicitly expected final sequence and its genuine controlled receipts. */
    async finish(input) { assert.equal(input.receipts.length, 3); assert.equal(input.userMessageTimestamp, userTimestamp); },
  });
  env.setPlan({ rounds: [[{ id: env.a.id, speak: true }, { id: env.b.id, speak: true }], [], [{ id: env.a.id, speak: true }]] });
  const original = env.input(), result = plain(await env.controller.submit(original));
  assert.equal(result.status, "completed"); assert.equal(result.cursor, 3); assert.equal(result.plannedTurns, 3); assert.equal(result.userMessageTimestamp, userTimestamp);
  assert.deepEqual(env.calls.map(
    /** Compares actual sequence calls rather than a second test-side planner/executor implementation. */
    call => call.method,
  ), ["submit", "continue", "continue", "finish"]);
  const first = env.calls[0].input; assert.deepEqual(first.submission, original); assert.equal(first.kind, "execute");
  assert.equal(first.turn.participantId, env.a.id);
  assert.equal(first.turn.selection, env.selection);
  assert.equal(env.calls[1].input.turn.participantId, env.b.id); assert.equal(env.calls[2].input.turn.participantId, env.a.id);
  assert.equal(first.turn.notifyReply, false); assert.equal(env.calls[1].input.turn.notifyReply, false); assert.equal(env.calls[2].input.turn.notifyReply, true);
  for (const call of env.calls.slice(1, 3)) {
    assert.equal(call.input.userMessageTimestamp, userTimestamp); assert.equal(Object.hasOwn(call.input, "text"), false);
    assert.equal(Object.hasOwn(call.input, "attachments"), false); assert.equal(Object.hasOwn(call.input, "replyTo"), false);
  }
  assert.deepEqual(result.receipts[0].assistant, { messageTimestamp: 1700000000300, variantIndex: 7 });
  assert.equal(env.modelCalls.length, 1); assert.ok(env.extensionCalls.length > 0);
});

/** The submission token retains one execution Promise and rejects a changed payload or competing live sequence. */
test("executor: duplicate-submit never replans or duplicates the user input, including after completion", async t => {
  const env = await groupEnvironment(t, {
    /** Verifies only disposal of this local planning fixture; no native success is fabricated. */
    async abandon(input) { assert.equal(input.chatId, env.chatId); },
    /** Produces the sole controlled send receipt for this idempotency scenario. */
    async submit(input) { return controlledReceipt(input, userTimestamp, { messageTimestamp: 1700000000300, variantIndex: 1 }); },
    /** No continuation capability is supplied for this single-turn plan. */
    async continue() { forbidden("continue"); },
    /** No cancellation capability is supplied for this completed plan. */
    async cancel() { forbidden("cancel"); },
    /** Acknowledges the exact one-reply sequence under test. */
    async finish(input) { assert.equal(input.receipts.length, 1); },
  });
  env.setPlan({ rounds: [[{ id: env.a.id, speak: true }]] });
  const input = env.input(), first = env.controller.submit(input), second = env.controller.submit(plain(input));
  assert.equal(first, second);
  assert.throws(() => env.controller.submit({ ...input, text: "Changed input" }), /different input/);
  assert.throws(() => env.controller.submit({ ...input, submissionId: "competing-token" }), /already owns/);
  await first; assert.equal(env.controller.submit(input), first); await env.controller.submit(input);
  assert.equal(env.modelCalls.length, 1); assert.equal(env.calls.length, 2);
});

/** A cancelled partial reply retains its genuine locator and cursor; resumption references the same user message. */
test("executor: cancelled receipt resumes its cursor without a second initial submit or another planner call", async t => {
  let continued = 0;
  const env = await groupEnvironment(t, {
    /** Verifies only disposal of this local planning fixture; no native success is fabricated. */
    async abandon(input) { assert.equal(input.chatId, env.chatId); },
    /** Returns an explicit cancellation and a persisted partial reply, not completed text guessed from history. */
    async submit(input) { return controlledReceipt(input, userTimestamp, { messageTimestamp: 1700000000300, variantIndex: 2 }, "cancelled"); },
    /** Settles only the resumed planned turn and the following real participant. */
    async continue(input) { continued++; return controlledReceipt(input, userTimestamp, { messageTimestamp: 1700000000300 + continued, variantIndex: 3 }); },
    /** No separate cancel request is necessary because the controlled transport itself reported cancellation. */
    async cancel() { forbidden("cancel"); },
    /** Finalizes only after the partial receipt and both completed resumed turns exist. */
    async finish(input) { assert.equal(input.receipts.length, 3); },
  });
  env.setPlan({ rounds: [[{ id: env.a.id, speak: true }, { id: env.b.id, speak: true }]] });
  const original = env.input(), cancelled = plain(await env.controller.submit(original));
  assert.equal(cancelled.status, "cancelled"); assert.equal(cancelled.cursor, 0); assert.equal(cancelled.userMessageTimestamp, userTimestamp);
  assert.equal(env.calls.length, 1); assert.deepEqual(cancelled.receipts[0].assistant, { messageTimestamp: 1700000000300, variantIndex: 2 });
  const resumed = plain(await env.controller.resume(env.chatId, original.submissionId));
  assert.equal(resumed.status, "completed"); assert.equal(resumed.cursor, 2); assert.equal(env.modelCalls.length, 1);
  assert.deepEqual(env.calls.map(
    /** Requires an actual continuation rather than submitting the original attachments/reply twice. */
    call => call.method,
  ), ["submit", "continue", "continue", "finish"]);
  assert.equal(env.calls[1].input.turn.participantId, env.a.id); assert.equal(env.calls[1].input.userMessageTimestamp, userTimestamp);
  assert.notEqual(env.calls[0].input.turn.requestKey, env.calls[1].input.turn.requestKey);
});

/** A user cancellation targets the controlled correlated turn and waits for its explicit receipt before exposing a cancelled cursor. */
test("executor: active cancellation passes the local request correlation key and never continues after cancellation", async t => {
  const started = deferred(), settled = deferred(), cancellations = [];
  const env = await groupEnvironment(t, {
    /** Verifies only disposal of this local planning fixture; no native success is fabricated. */
    async abandon(input) { assert.equal(input.chatId, env.chatId); },
    /** Keeps a genuine controlled turn pending until the test completes its cancellation receipt. */
    async submit(input) { started.resolve(input); return settled.promise; },
    /** No continuation may occur after this cancellation. */
    async continue() { forbidden("continue"); },
    /** Records the exact cancellation target rather than using a broad current-runtime stop. */
    async cancel(input) { cancellations.push(plain(input)); },
    /** A cancelled sequence must not be summarized or finalized as completed. */
    async finish() { forbidden("finish"); },
  });
  env.setPlan({ rounds: [[{ id: env.b.id, speak: true }, { id: env.a.id, speak: true }]] });
  const original = env.input(), run = env.controller.submit(original), pending = await started.promise;
  await env.controller.cancel(env.chatId, original.submissionId);
  assert.deepEqual(cancellations, [{ chatId: env.chatId, requestKey: pending.turn.requestKey, runtime: "main" }]);
  settled.resolve(controlledReceipt(pending, userTimestamp, null, "cancelled"));
  const result = plain(await run); assert.equal(result.status, "cancelled"); assert.equal(result.cursor, 0);
  assert.deepEqual(env.calls.map(
    /** Preserves the actual stop boundary and proves no next speaker is invoked. */
    call => call.method,
  ), ["submit", "cancel"]);
});

/** Cancellation while the functional planner is pending must truthfully report that no user message was persisted yet. */
test("executor: planner cancellation resumes the original full input once with no fabricated user timestamp", async t => {
  const delayed = deferred();
  const env = await groupEnvironment(t, {
    /** Verifies only disposal of this local planning fixture; no native success is fabricated. */
    async abandon(input) { assert.equal(input.chatId, env.chatId); },
    /** Receives the sole actual first input after the explicit resume. */
    async submit(input) { return controlledReceipt(input, userTimestamp, { messageTimestamp: 1700000000300, variantIndex: 1 }); },
    /** No continuation is necessary because planning was cancelled before submission. */
    async continue() { forbidden("continue"); },
    /** No transport execution existed during planning, so there is no valid target to cancel. */
    async cancel() { forbidden("cancel"); },
    /** Acknowledges only the completed one-reply resumed plan. */
    async finish(input) { assert.equal(input.receipts.length, 1); },
  });
  env.setRawPlan(delayed.promise);
  const input = env.input(), run = env.controller.submit(input); await env.planningStarted;
  await env.controller.cancel(env.chatId, input.submissionId);
  delayed.resolve({ text: JSON.stringify({ rounds: [[{ id: env.a.id, speak: true }]] }) });
  const result = plain(await run); assert.equal(result.status, "cancelled"); assert.equal(result.userMessageTimestamp, null);
  assert.deepEqual(result.receipts, []); assert.equal(env.calls.length, 0);
  const resumed = plain(await env.controller.resume(env.chatId, input.submissionId));
  assert.equal(resumed.status, "completed"); assert.deepEqual(env.calls[0].input.submission, input); assert.equal(env.modelCalls.length, 1);
});

/** A no-response plan has an explicit user-only persistence operation and cannot choose the group's first member. */
test("executor: empty and speak-false plans submit a real record-only input without resolving an execution profile", async t => {
  const env = await groupEnvironment(t, {
    /** Verifies only disposal of this local planning fixture; no native success is fabricated. */
    async abandon(input) { assert.equal(input.chatId, env.chatId); },
    /** Acknowledges only a real controlled user-only submission with no assistant locator. */
    async submit(input) { assert.equal(input.kind, "record_only"); assert.equal(Object.hasOwn(input, "turn"), false); return controlledReceipt(input, userTimestamp, null); },
    /** No execution continuation exists in a no-response plan. */
    async continue() { forbidden("continue"); },
    /** No turn needs cancellation in this completed user-only scenario. */
    async cancel() { forbidden("cancel"); },
    /** Finalizes the acknowledged user-only operation without claiming a generated reply. */
    async finish(input) { assert.equal(input.receipts[0].assistant, null); },
  });
  env.setPlan({ rounds: [[], [{ id: env.b.id, speak: false }]] });
  const result = plain(await env.controller.submit(env.input()));
  assert.equal(result.status, "completed"); assert.equal(result.plannedTurns, 0); assert.equal(result.cursor, 0); assert.equal(result.receipts.length, 1);
});

/** A deleted live member is a real file/domain error, not an instruction to silently skip or choose another member. */
test("executor: member removed from the actual saved group between turns stops before any continuation", async t => {
  let env;
  env = await groupEnvironment(t, {
    /** Verifies only disposal of this local planning fixture; no native success is fabricated. */
    async abandon(input) { assert.equal(input.chatId, env.chatId); },
    /** Commits an actual group edit after the first controlled receipt and before the next planned speaker. */
    async submit(input) {
      await env.service.dispatchDomain("group.update", { id: env.group.id, changes: { members: [{ characterCardId: env.a.id, orderIndex: 0 }] } });
      return controlledReceipt(input, userTimestamp, { messageTimestamp: 1700000000300, variantIndex: 1 });
    },
    /** The removed member must never reach the real continuation boundary. */
    async continue() { forbidden("continue"); },
    /** No cancellation request occurs in this failed-reference scenario. */
    async cancel() { forbidden("cancel"); },
    /** A partially completed, failed-reference plan must not be finalized as completed. */
    async finish() { forbidden("finish"); },
  });
  env.setPlan({ rounds: [[{ id: env.a.id, speak: true }, { id: env.b.id, speak: true }]] });
  await assert.rejects(() => env.controller.submit(env.input()), /deleted or removed/);
  assert.deepEqual(env.calls.map(
    /** Proves actual domain revalidation happens before another user or assistant can be written. */
    call => call.method,
  ), ["submit", "abandon"]);
  const saved = await env.service.dispatchDomain("group.get", { id: env.group.id }); assert.equal(saved.members.length, 1);
});

/** Failed execution remains the original retained failure; duplicate tokens and resume cannot trigger a hidden retry. */
test("executor: original send failure is retained without retry and malformed planner output sends nothing", async t => {
  const failure = new Error("Controlled original send failure");
  const env = await groupEnvironment(t, {
    /** Verifies only disposal of this local planning fixture; no native success is fabricated. */
    async abandon(input) { assert.equal(input.chatId, env.chatId); },
    /** Rejects the send with the exact controlled failure object. */
    async submit() { throw failure; },
    /** No continuation is provided for a failed first execution. */
    async continue() { forbidden("continue"); },
    /** No cancellation was requested for this failing turn. */
    async cancel() { forbidden("cancel"); },
    /** Failed execution must never invoke completed-sequence finalization. */
    async finish() { forbidden("finish"); },
  });
  env.setPlan({ rounds: [[{ id: env.a.id, speak: true }]] });
  const input = env.input(), run = env.controller.submit(input);
  await assert.rejects(() => run,
    /** Checks the actual error identity rather than accepting a rewritten generic error. */
    error => error === failure,
  );
  assert.equal(env.controller.submit(input), run);
  await assert.rejects(() => env.controller.submit(input),
    /** A repeated token exposes the same failure instead of starting a new model request. */
    error => error === failure,
  );
  assert.throws(() => env.controller.resume(env.chatId, input.submissionId), /settled cancelled/);
  assert.equal(env.modelCalls.length, 1); assert.equal(env.calls.length, 2);
  env.setRawPlan({ text: '{"rounds":[[{"id":"unknown","speak":true}]]}' });
  await assert.rejects(() => env.controller.submit(env.input("malformed-plan-token")), /Unknown planned/);
  assert.equal(env.calls.length, 2); assert.equal(env.modelCalls.length, 2);
});

/** A receipt must identify the actual existing user message and cannot report completion with a guessed or absent reply. */
test("executor: malformed completion and changed continuation user timestamps fail instead of accepting guessed locators", async t => {
  const env = await groupEnvironment(t, {
    /** Verifies only disposal of this local planning fixture; no native success is fabricated. */
    async abandon(input) { assert.equal(input.chatId, env.chatId); },
    /** Provides the genuine controlled first user and assistant locators. */
    async submit(input) { return controlledReceipt(input, userTimestamp, { messageTimestamp: 1700000000300, variantIndex: 2 }); },
    /** Intentionally violates the continuation contract by acknowledging a different user message. */
    async continue(input) { return controlledReceipt(input, userTimestamp + 1, { messageTimestamp: 1700000000301, variantIndex: 3 }); },
    /** No separate cancellation is requested for this bad receipt. */
    async cancel() { forbidden("cancel"); },
    /** No completed-sequence finalization may follow an invalid receipt. */
    async finish() { forbidden("finish"); },
  });
  env.setPlan({ rounds: [[{ id: env.a.id, speak: true }, { id: env.b.id, speak: true }]] });
  await assert.rejects(() => env.controller.submit(env.input()), /different user message/);
  assert.equal(env.calls.length, 3); assert.equal(env.modelCalls.length, 1);
});

/** Separate execution and cancellation failures are both observable without swallowing either original error. */
test("executor: concurrent send and cancellation failures retain both exact errors and never retry", async t => {
  const started = deferred(), sendResult = deferred(), sendError = new Error("Original send error"), cancelError = new Error("Original cancel error");
  const env = await groupEnvironment(t, {
    /** Verifies only disposal of this local planning fixture; no native success is fabricated. */
    async abandon(input) { assert.equal(input.chatId, env.chatId); },
    /** Keeps the send pending so a real correlated cancellation request can also fail. */
    async submit(input) { started.resolve(input); return sendResult.promise; },
    /** No continuation can occur after two failed operations. */
    async continue() { forbidden("continue"); },
    /** Rejects cancellation with its independently scheduled original error. */
    async cancel() { throw cancelError; },
    /** A failed execution/cancellation pair cannot be finalized as completed. */
    async finish() { forbidden("finish"); },
  });
  env.setPlan({ rounds: [[{ id: env.a.id, speak: true }]] });
  const input = env.input(), run = env.controller.submit(input); await started.promise;
  await assert.rejects(() => env.controller.cancel(env.chatId, input.submissionId),
    /** Preserves the directly observed cancellation failure identity. */
    error => error === cancelError,
  );
  sendResult.reject(sendError);
  await assert.rejects(() => run,
    /** Requires both exact failure objects rather than accepting one missing error. */
    error => error.executionFailure === sendError && error.cancellationFailure === cancelError,
  );
  assert.equal(env.calls.length, 3); assert.equal(env.modelCalls.length, 1);
});

/** A native cancellation before persistence keeps null truthful and resubmits only when explicitly resumed. */
test("executor: native cancelled commit with null user timestamp resumes the original input without fabricating a persisted row", async t => {
  let submits = 0;
  const env = await groupEnvironment(t, {
    /** Verifies only disposal of this local planning fixture; no native success is fabricated. */
    async abandon(input) { assert.equal(input.chatId, env.chatId); },
    /** Reports no persisted row on the first cancelled attempt, then explicitly acknowledges the resumed actual submission. */
    async submit(input) {
      submits++;
      if (submits === 1) return controlledReceipt(input, null, null, "cancelled");
      return controlledReceipt(input, userTimestamp, { messageTimestamp: 1700000000300, variantIndex: 5 });
    },
    /** A continuation cannot exist until a real user row has been acknowledged. */
    async continue() { forbidden("continue"); },
    /** No separately requested cancellation is used in this receipt-driven scenario. */
    async cancel() { forbidden("cancel"); },
    /** Finalizes only after the explicit resumed commit exists. */
    async finish(input) { assert.equal(input.userMessageTimestamp, userTimestamp); assert.equal(input.receipts[0].userMessageTimestamp, null); },
  });
  env.setPlan({ rounds: [[{ id: env.a.id, speak: true }]] });
  const original = env.input(), cancelled = plain(await env.controller.submit(original));
  assert.equal(cancelled.status, "cancelled"); assert.equal(cancelled.userMessageTimestamp, null); assert.equal(cancelled.cursor, 0);
  assert.equal(cancelled.receipts[0].userMessageTimestamp, null);
  const resumed = plain(await env.controller.resume(env.chatId, original.submissionId));
  assert.equal(resumed.status, "completed"); assert.equal(resumed.userMessageTimestamp, userTimestamp);
  assert.deepEqual(env.calls[0].input.submission, original); assert.deepEqual(env.calls[1].input.submission, original);
  assert.equal(env.modelCalls.length, 1);
});

for (const disposition of [{ type: "blocked", message: "Native input blocked" }, { type: "consumed", metadata: { owner: "another-plugin", opaque: "owned-input" } }, { type: "not_persisted", status: "completed" }]) {
  /** Preserves each actual native noncommit state and proves it cannot advance the group plan or manufacture a user timestamp. */
  test("executor: native " + disposition.type + " stops with its exact state, no next member or completed-sequence finish", async t => {
    const env = await groupEnvironment(t, {
    /** Verifies only disposal of this local planning fixture; no native success is fabricated. */
    async abandon(input) { assert.equal(input.chatId, env.chatId); },
      /** Returns only the explicitly scheduled native input ownership or nonpersistent outcome. */
      async submit(input) { return { chatId: input.submission.chatId, message: input.submission.text, sentAt: 1700000000000, outcome: plain(disposition) }; },
      /** No continuation is authorized by a noncommit result. */
      async continue() { forbidden("continue"); },
      /** No active native generation was admitted for cancellation. */
      async cancel() { forbidden("cancel"); },
      /** A blocked, consumed or nonpersistent operation cannot be finalized as a completed persisted group plan. */
      async finish() { forbidden("finish"); },
    });
    env.setPlan({ rounds: [[{ id: env.a.id, speak: true }, { id: env.b.id, speak: true }]] });
    const input = env.input(), result = plain(await env.controller.submit(input));
    assert.equal(result.status, disposition.type); assert.deepEqual(result.disposition, disposition); assert.deepEqual(result.receipts, []);
    assert.equal(result.userMessageTimestamp, null); assert.equal(result.cursor, 0); assert.equal(env.calls.length, 2);
    assert.throws(() => env.controller.resume(env.chatId, input.submissionId), /settled cancelled/);
    const repeated = plain(await env.controller.submit(input)); assert.deepEqual(repeated, result);
    assert.equal(env.modelCalls.length, 1);
  });
}

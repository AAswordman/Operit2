import assert from "node:assert/strict";
import test from "node:test";
import { copy, flush, harness, part } from "./runtime.mjs";

/** Starts several independent sends without waiting for any model response. */
test("spawn returns immediately and preserves the source, original task and actual participant", async () => {
  const h = harness();
  const agents = await Promise.all([h.spawn({ task: " original task " }), h.spawn({ name: "Second", participantId: null })]);
  assert.equal(h.chat.sends.length, 2);
  assert.equal(agents[0].status, "starting");
  assert.deepEqual(h.chat.created, [{ chatId: "child-1", parentChatId: "parent" }, { chatId: "child-2", parentChatId: "parent" }]);
  assert.deepEqual(h.chat.sends[0].request, {
    kind: "submit", chatId: "child-1", runtime: "main", notifyReply: false,
    input: { text: " original task ", attachments: [], replyToMessageTimestamp: null },
    turn: { kind: "execute", participantId: "participant-a" },
  });
  assert.deepEqual(h.chat.sends[1].request.turn, { kind: "execute" });
  assert.equal((await h.request("list")).agents.length, 2);
});

/** Proves that removed or revised blocks disappear from the current execution detail. */
test("semantic revisions replace whole message parts and preserve independent messages", async () => {
  const h = harness(); const agent = await h.spawn();
  const observation = h.chat.latest(agent.id).observation;
  observation.emit({ type: "part", chatId: agent.id, messageTimestamp: 20, revision: 1, parts: [part("thinking", "thinking", "plan"), part("old", "markdown", "uncommitted")] });
  await flush();
  observation.emit({ type: "part", chatId: agent.id, messageTimestamp: 20, revision: 2, parts: [part("tool", "tool_call", "read")] });
  observation.emit({ type: "part", chatId: agent.id, messageTimestamp: 21, revision: 1, parts: [part("new", "markdown", "answer")] });
  await flush();
  const saved = h.repository.records.get(agent.id);
  assert.equal(saved.status, "running");
  assert.deepEqual(saved.messages.map(message => message.parts.map(value => value.partId)), [["tool"], ["new"]]);
});

/** Exercises repeated wait deadlines without wall-clock delays or additional sends. */
test("30-second timeout only ends the wait, and the same agent can be waited again", async () => {
  const h = harness(); const agent = await h.spawn();
  const first = h.request("wait", { agentIds: [agent.id], timeoutMs: 30_000 });
  await flush();
  assert.equal(h.clock.timers.size, 1);
  h.clock.advance(30_000);
  assert.equal((await first).timed_out, true);
  assert.equal(h.clock.timers.size, 0);
  assert.equal(h.chat.cancellations.length, 0);
  const second = h.request("wait", { agentIds: [agent.id], timeoutMs: 30_000 });
  await flush();
  h.chat.complete(agent.id, "completed", "actual response");
  const finished = await second;
  assert.equal(finished.timed_out, false);
  assert.equal(finished.agents[0].result.response, "actual response");
  assert.equal(finished.agents[0].result.outcome.assistant.messageTimestamp, 20);
  assert.equal(h.clock.timers.size, 0);
  assert.equal(h.chat.sends.length, 1);
  assert.equal(h.chat.cancellations.length, 0);
});

/** Keeps wait-any behavior explicit while refusing cross-parent task control. */
test("wait wakes on any selected completion and validates the entire selection first", async () => {
  const h = harness(); const one = await h.spawn(); const two = await h.spawn();
  await assert.rejects(h.request("wait", { agentIds: [one.id, "foreign"], timeoutMs: 30000 }), /does not belong/);
  await assert.rejects(h.request("wait", { agentIds: [one.id, one.id], timeoutMs: 30000 }), /unique/);
  await assert.rejects(h.request("wait", { agentIds: [one.id], timeoutMs: 30001 }), /30000/);
  assert.equal(h.clock.timers.size, 0);
  const waiting = h.request("wait", { agentIds: [one.id, two.id], timeoutMs: 30000 });
  await flush(); h.chat.complete(two.id);
  const result = await waiting;
  assert.equal(result.timed_out, false);
  assert.deepEqual(copy(result.agents.map(agent => agent.status)), ["starting", "completed"]);
  assert.equal(h.chat.processing.has(one.id), true);
});

/** Confirms that an acknowledgement cannot be mistaken for native completion. */
test("interrupt waits for the old terminal receipt before submitting the new instruction", async () => {
  const h = harness(); const agent = await h.spawn();
  let completed = false;
  const replacement = h.request("send", { agentId: agent.id, message: "new instruction", interrupt: true }).then(value => { completed = true; return value; });
  await flush();
  assert.deepEqual(h.chat.cancellations, [agent.id]);
  assert.equal(completed, false);
  assert.equal(h.chat.sends.length, 1);
  await assert.rejects(h.request("close", { agentId: agent.id }), /control operation/);
  h.chat.complete(agent.id, "cancelled", "partial");
  const next = await replacement;
  assert.equal(next.runNumber, 2);
  assert.equal(next.status, "starting");
  assert.equal(h.chat.sends.length, 2);
  assert.equal(h.chat.latest(agent.id).request.input.text, "new instruction");
  assert.equal(h.repository.records.get(agent.id).messages.length, 0);
});

/** Pins a wait to the cancelled old turn even when interruption immediately schedules another one. */
test("a pending wait returns its captured turn rather than a later replacement", async () => {
  const h = harness(); const agent = await h.spawn();
  const waiting = h.request("wait", { agentIds: [agent.id], timeoutMs: 30000 });
  await flush();
  const replacing = h.request("send", { agentId: agent.id, message: "replacement", interrupt: true });
  await flush(); h.chat.complete(agent.id, "cancelled");
  const [observed, next] = await Promise.all([waiting, replacing]);
  assert.equal(observed.agents[0].runNumber, 1);
  assert.equal(observed.agents[0].status, "cancelled");
  assert.equal(next.runNumber, 2);
  assert.equal(h.clock.timers.size, 0);
});

/** Covers creation followed immediately by cancellation before the native send is admitted. */
test("close waits through pending admission and does not leak a newly admitted send", async () => {
  const h = harness(); h.chat.automaticAdmission = false;
  const agent = await h.spawn();
  let closed = false;
  const closing = h.request("close", { agentId: agent.id }).then(value => { closed = true; return value; });
  await flush();
  assert.equal(closed, false);
  assert.equal(h.chat.cancellations.length, 1);
  h.chat.processing.add(agent.id);
  h.clock.advance(50); await flush();
  assert.equal(h.chat.cancellations.length, 2);
  assert.equal(closed, false);
  h.chat.complete(agent.id, "cancelled");
  assert.equal((await closing).status, "closed");
  assert.equal(h.clock.timers.size, 0);
  assert.equal(h.chat.processing.size, 0);
});

/** Keeps the persisted chat and source snapshots available after task closure. */
test("close is idempotent, retains history, and prevents subsequent input", async () => {
  const h = harness(); const agent = await h.spawn(); h.chat.automaticCancellation = true;
  const closed = await h.request("close", { agentId: agent.id });
  assert.equal(closed.status, "closed");
  assert.equal(closed.result.outcome.status, "cancelled");
  assert.equal((await h.request("close", { agentId: agent.id })).status, "closed");
  assert.equal(h.chat.cancellations.length, 1);
  assert.equal(h.repository.records.has(agent.id), true);
  await assert.rejects(h.request("send", { agentId: agent.id, message: "later", interrupt: true }), /closed/);
  assert.equal(h.chat.sends.length, 1);
});

/** Prevents one parent from issuing cancellation requests against another parent's child. */
test("foreign controls and non-interrupting sends fail before touching native execution", async () => {
  const h = harness(); const agent = await h.spawn();
  await assert.rejects(h.request("send", { agentId: agent.id, message: "hello", interrupt: false }), /interrupt=true/);
  await assert.rejects(h.request("close", { agentId: agent.id }, "other-parent"), /does not belong/);
  await assert.rejects(h.request("send", { agentId: agent.id, message: "hello", interrupt: true }, "other-parent"), /does not belong/);
  assert.equal(h.chat.cancellations.length, 0);
  assert.equal(h.chat.sends.length, 1);
});

/** Exercises actual parent-child task relationships and prevents creation throughout recursive closure. */
test("closing a parent closes its descendants first and blocks child creation until closure finishes", async () => {
  const h = harness(); const parent = await h.spawn(); const child = await h.spawn({}, parent.id);
  const closing = h.request("close", { agentId: parent.id });
  await flush();
  assert.deepEqual(h.chat.cancellations, [child.id]);
  await assert.rejects(h.spawn({}, parent.id), /parent subagent/);
  await assert.rejects(h.spawn({}, child.id), /parent subagent/);
  h.chat.complete(child.id, "cancelled"); await flush();
  assert.deepEqual(h.chat.cancellations, [child.id, parent.id]);
  await assert.rejects(h.spawn({}, parent.id), /parent subagent/);
  h.chat.complete(parent.id, "cancelled");
  assert.equal((await closing).status, "closed");
  assert.equal(h.repository.records.get(child.id).status, "closed");
});

for (const restarted of [false, true]) {
  /** Keeps archived task state separate from later user-driven work in its retained conversation. */
  test("a retained closed chat can create new agents without reopening the old task (restarted: " + restarted + ")", async () => {
    const original = harness(); const archived = await original.spawn();
    original.chat.automaticCancellation = true;
    await original.request("close", { agentId: archived.id });
    const closed = copy(original.repository.records.get(archived.id));
    const h = restarted ? harness([closed]) : original;
    if (restarted) h.chat.created.push(...copy(original.chat.created));
    h.chat.processing.add(archived.chatId);
    const child = await h.spawn({ task: "New work in the retained conversation" }, archived.chatId);
    assert.equal(child.parentChatId, archived.chatId);
    assert.equal(child.status, "starting");
    assert.equal(child.runNumber, 1);
    assert.equal(h.chat.latest(child.chatId).request.input.text, "New work in the retained conversation");
    assert.deepEqual(copy(h.repository.records.get(archived.id)), closed);
    await assert.rejects(h.request("send", { agentId: archived.id, message: "Reopen old task", interrupt: false }), /closed or closing/);
    assert.deepEqual(copy((await h.request("list", {}, archived.chatId)).agents.map(agent => agent.id)), [child.id]);
  });
}

/** Reserves concurrent creation capacity before the host can resolve any of the creates. */
test("concurrent creation and follow-up turns share the eight-agent capacity limit", async () => {
  const h = harness();
  let release;
  h.chat.createGate = new Promise(resolve => { release = resolve; });
  const pending = Array.from({ length: 8 }, () => h.spawn());
  await flush();
  await assert.rejects(h.spawn(), /At most 8/);
  assert.equal(h.chat.created.length, 8);
  release(); const agents = await Promise.all(pending);
  h.chat.complete(agents[0].id); await flush();
  const extra = await h.spawn();
  assert.equal(extra.id, "child-9");
  await assert.rejects(h.request("send", { agentId: agents[0].id, message: "new turn", interrupt: false }), /At most 8/);
  assert.equal(h.chat.sends.length, 9);
});

/** Keeps the parent alive while a child creation is still waiting for its host acknowledgement. */
test("a parent cannot close across an in-flight child creation", async () => {
  const h = harness(); const parent = await h.spawn();
  let release;
  h.chat.createGate = new Promise(resolve => { release = resolve; });
  const child = h.spawn({}, parent.id); await flush();
  await assert.rejects(h.request("close", { agentId: parent.id }), /child creation/);
  release(); await child;
  h.chat.automaticCancellation = true;
  assert.equal((await h.request("close", { agentId: parent.id })).status, "closed");
});

/** Reports genuine native failures and hook ownership changes without fabricated terminal success. */
test("native errors and consumed turns become explicit failures with truthful receipts", async () => {
  const h = harness(); const one = await h.spawn(); const two = await h.spawn();
  const wait = h.request("wait", { agentIds: [one.id], timeoutMs: 30000 }); await flush();
  h.chat.latest(one.id).observation.fail(new Error("provider failed"));
  const result = await wait;
  assert.equal(result.agents[0].status, "failed");
  assert.match(result.agents[0].error, /provider failed$/);
  assert.equal(result.agents[0].result, null);
  h.chat.latest(two.id).observation.emit({ type: "completed", result: { chatId: two.id, outcome: { type: "consumed", metadata: { owner: "other-plugin" } } } });
  await flush();
  const saved = h.repository.records.get(two.id);
  assert.equal(saved.status, "failed");
  assert.equal(saved.result.outcome.type, "consumed");
  assert.match(saved.error, /not owned/);
});

/** Preserves interrupted execution records and consults actual host state before a new send. */
test("restart exposes lost observation and refuses to overlap an unobserved native turn", async () => {
  const first = harness(); const agent = await first.spawn();
  const saved = copy(first.repository.records.get(agent.id));
  const h = harness([saved]); h.chat.processing.add(agent.id);
  const state = await h.request("list");
  assert.equal(state.agents[0].status, "interrupted");
  assert.match(state.agents[0].error, /observation was lost/);
  assert.equal(state.agents[0].result, null);
  await assert.rejects(h.request("send", { agentId: agent.id, message: "overlap", interrupt: false }), /active execution/);
  assert.equal(h.chat.sends.length, 0);
  const closing = h.request("close", { agentId: agent.id }); await flush();
  assert.deepEqual(h.chat.cancellations, [agent.id]);
  h.chat.processing.delete(agent.id); h.clock.advance(100);
  const closed = await closing;
  assert.equal(closed.status, "closed");
  assert.equal(closed.result, null);
});

/** Confirms that valid later commands remain possible after an explicitly rejected control. */
test("invalid authored parameters fail before any chat is created", async () => {
  const h = harness();
  await assert.rejects(h.spawn({ task: " " }), /nonblank/);
  await assert.rejects(h.spawn({ participantId: "" }), /nonblank/);
  assert.equal(h.chat.created.length, 0);
  const agent = await h.spawn();
  await assert.rejects(h.request("send", { agentId: agent.id, message: " ", interrupt: true }), /nonblank/);
  assert.equal(h.chat.cancellations.length, 0);
});

/** Rejects foreign native identities and stale revisions without presenting them as successful output. */
test("invalid native identities and revisions become explicit task failures", async () => {
  const h = harness(); const one = await h.spawn(); const two = await h.spawn();
  h.chat.latest(one.id).observation.emit({ type: "part", chatId: "foreign", messageTimestamp: 20, revision: 1, parts: [] });
  const observation = h.chat.latest(two.id).observation;
  observation.emit({ type: "part", chatId: two.id, messageTimestamp: 20, revision: 1, parts: [part("old", "markdown", "initial")] });
  observation.emit({ type: "part", chatId: two.id, messageTimestamp: 20, revision: 1, parts: [] });
  await flush();
  assert.match(h.repository.records.get(one.id).error, /different conversation/);
  assert.match(h.repository.records.get(two.id).error, /revision did not advance/);
  assert.equal(h.repository.records.get(two.id).messages[0].parts[0].content, "initial");
  assert.equal(h.repository.records.get(one.id).result, null);
});

/** Distinguishes native setup errors from a valid completed execution. */
test("a synchronous native stream setup failure settles and remains inspectable", async () => {
  const h = harness();
  h.chat.send = () => { throw new Error("native setup rejected"); };
  const agent = await h.spawn(); await flush();
  const state = await h.request("wait", { agentIds: [agent.id], timeoutMs: 30000 });
  assert.equal(state.agents[0].status, "failed");
  assert.match(state.agents[0].error, /native setup rejected/);
  assert.equal(state.agents[0].result, null);
  assert.equal((await h.request("close", { agentId: agent.id })).status, "closed");
});

/** Ensures infrastructure failures are neither retried as task saves nor hidden by wait timeouts. */
test("failed persistence rejects current and later waits without retrying the write", async () => {
  const h = harness(); const agent = await h.spawn();
  const waiting = h.request("wait", { agentIds: [agent.id], timeoutMs: 30000 }); await flush();
  let writes = 0;
  h.repository.save = async () => { writes++; throw new Error("storage unavailable"); };
  const rejected = assert.rejects(waiting, /storage unavailable/);
  h.chat.latest(agent.id).observation.emit({ type: "part", chatId: agent.id, messageTimestamp: 20, revision: 1, parts: [] });
  await rejected; await flush();
  assert.equal(writes, 1);
  assert.equal(h.errors.length, 1);
  assert.equal(h.clock.timers.size, 0);
  await assert.rejects(h.request("wait", { agentIds: [agent.id], timeoutMs: 30000 }), /storage unavailable/);
  assert.equal(writes, 1);
});

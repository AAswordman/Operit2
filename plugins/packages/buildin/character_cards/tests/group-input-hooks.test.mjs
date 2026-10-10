import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { selectorEnvironment } from "./selector-environment.mjs";
import { mountRegisteredComposeRoute, keyedNode } from "./selector-render.mjs";
import { plain } from "./runtime.mjs";

/** Holds only an explicit controlled host ABI operation, never a production execution implementation. */
function barrier() {
  let resolve, reject;
  const promise = new Promise(
    /** Captures explicit settlement of the controlled planner or native send boundary. */
    (yes, no) => { resolve = yes; reject = no; },
  );
  return { promise, resolve, reject };
}

/** Builds the actual host-origin submit envelope with complete attachment and runtime fields. */
function event(change = {}) {
  return { event: "chat_input", eventName: "submit_requested", eventPayload: { chatId: "selector-chat", source: "Classic", runtime: "main", notifyReply: true,
    text: "Actual group input", attachments: [], attachmentCount: 0, hasAttachments: false, replyToMessageTimestamp: null, ...change } };
}

/** Opens the real main registrations and file service with explicitly controlled Chat/AI ABI inputs. */
async function environment(t) {
  const failures = [], runtime = await selectorEnvironment(t, {}, { NativeInterface: {
    /** Records actual asynchronous plugin failures without implementing another error handler. */
    logError(message) { failures.push(message); },
  } });
  assert.equal(runtime.chatInputHooks.length, 1); assert.equal(runtime.chatInputHooks[0].function, runtime.main.onGroupInputSubmit);
  return { ...runtime, failures,
    /** Invokes only the actually registered exported submission hook. */
    submit(input = event()) { return runtime.chatInputHooks[0].function(input); },
  };
}

/** Creates and explicitly binds genuine persisted participants through the registered APIs. */
async function bindGroup(runtime) {
  const first = await runtime.api("character.create", { values: { name: "Hook participant A" } });
  const second = await runtime.api("character.create", { values: { name: "Hook participant B" } });
  const group = await runtime.api("group.create", { values: { name: "Hook group", members: [{ characterCardId: first.id, orderIndex: 0 }, { characterCardId: second.id, orderIndex: 1 }] } });
  await runtime.api("chat.configuration.binding.write", { chatId: "selector-chat", selection: "group:" + group.id });
  return { first, second, group };
}

/** Observes bounded asynchronous test settlement without adding polling or retry logic to production. */
async function settled(runtime, submissionId) {
  for (let turn = 0; turn < 200; turn++) {
    const result = await runtime.groupExecution({ action: "status", chatId: "selector-chat", submissionId });
    if (result.status !== "running") return plain(result);
    await new Promise(
      /** Allows genuine repository and planner continuations to complete between explicit test observations. */
      resolve => setTimeout(resolve, 2),
    );
  }
  assert.fail("The admitted group submission did not settle at the controlled boundary");
}

/** Returns only an explicitly declared finalized native user-only send receipt. */
function receipt(request, status = "completed", assistant = null) {
  return { chatId: request.chatId, message: request.kind === "submit" ? request.input.text : "", aiResponse: null, receivedAt: null, sentAt: 1,
    outcome: { type: "committed", status, userMessageTimestamp: 10, assistant } };
}

/** Opens the actual production Compose controls using the generic SDK render and action channel. */
function controls(runtime, requestId = "group-controls") {
  return mountRegisteredComposeRoute(runtime, { requestId, input: { mode: "group-execution", chatId: "selector-chat" } }, "group-execution");
}

/** Checks that unrelated events and unassociated or single-character chats never acquire group ownership. */
test("registered group input leaves unassociated, card, and native sequence input untouched without storage initialization", async t => {
  const runtime = await environment(t);
  assert.equal(await runtime.submit({ eventName: "text_changed", eventPayload: {} }), null);
  assert.equal(await runtime.submit(event({ source: "Sequence" })), null);
  assert.equal(runtime.calls.length, 0); assert.equal(runtime.disk.calls.length, 0);
  assert.equal(await runtime.submit(), null); assert.equal(runtime.disk.calls.length, 0);
  runtime.records.get("selector-chat").extension = { version: 1, selection: "card:outside-group" };
  assert.equal(await runtime.submit(), null); assert.equal(runtime.disk.calls.length, 0);
  runtime.records.get("selector-chat").extension = { version: 99, selection: "group:invalid" };
  await assert.rejects(runtime.submit(), /version/); assert.equal(runtime.disk.calls.length, 0);
});

/** Rejects missing runtime, contradictory attachments, empty input and failed namespace reads before any admission. */
test("registered group input validates the actual envelope and propagates namespace errors without accepting input", async t => {
  const runtime = await environment(t); await bindGroup(runtime);
  await assert.rejects(runtime.submit(event({ runtime: null })), /runtime slot/);
  await assert.rejects(runtime.submit(event({ attachmentCount: 1 })), /counts differ/);
  await assert.rejects(runtime.submit(event({ text: "" })), /actual text or attachments/);
  runtime.failNext("readExtension", new Error("Exact namespace read failure"));
  await assert.rejects(runtime.submit(), /Exact namespace read failure/);
  assert.equal(await runtime.groupExecution({ action: "current", chatId: "selector-chat" }), null);
  await assert.rejects(runtime.groupExecution({ action: "current", chatId: "selector-chat", submissionId: "extra" }), /unexpected/);
  await assert.rejects(runtime.groupExecution({ action: "status", chatId: "selector-chat", submissionId: "missing" }), /matching retained/);
});

/** Exercises the registered submit hook, real domain reads, ordered planner, native transport and real authored snapshots together. */
test("registered group input sends complete input once and ordered turns use real plugin-authored execution snapshots", async t => {
  const runtime = await environment(t), { first, second } = await bindGroup(runtime), calls = [], snapshots = new Map();
  const readExtension = runtime.tools.Chat.readExtension;
  /** Supplies exactly the controlled message snapshot written by the acknowledged native ABI call. */
  runtime.tools.Chat.readExtension = async target => target.kind === "chat" ? readExtension(target) : snapshots.get(target.messageTimestamp);
  /** Supplies one explicit planner response whose participant IDs come from genuine persisted records. */
  runtime.tools.Chat.call = async request => { calls.push({ planner: plain(request) }); return { text: JSON.stringify({ rounds: [[{ id: second.id, speak: true }, { id: first.id, speak: true }]] }) }; };
  /** Models only the acknowledged native send boundary and obtains its snapshot through the real registered configuration provider. */
  runtime.tools.Chat.sendMessage = async request => {
    calls.push({ send: plain(request) });
    const participantId = request.kind === "submit" ? request.turn.participantId : request.participantId;
    const configuration = await runtime.api("chat.configuration.resolve", { purpose: "execution", chatId: request.chatId,
      chatExtension: runtime.records.get(request.chatId).extension, messageExtension: null, participantId, promptFunctionType: "CHAT",
      defaultModelBinding: { providerId: "dashscope", modelId: "qwen-plus" }, defaultTtsConfigId: "tts-one" });
    const timestamp = 20 + snapshots.size; snapshots.set(timestamp, plain(configuration.messageExtension));
    return receipt(request, "completed", { messageTimestamp: timestamp, variantIndex: 0 });
  };
  const attachments = [{ filePath: "vfs://actual-input.txt", nodeId: "input-node", fileName: "actual-input.txt", mimeType: "text/plain", fileSize: 7, content: "payload" }];
  const admitted = await runtime.submit(event({ attachments, attachmentCount: 1, hasAttachments: true, replyToMessageTimestamp: 9 }));
  assert.equal(admitted.action, "Consume"); assert.equal(admitted.clearInput, true);
  const done = await settled(runtime, admitted.metadata.submissionId);
  assert.equal(done.status, "settled", done.error); assert.equal(done.outcome.status, "completed"); assert.equal(done.outcome.cursor, 2); assert.equal(done.outcome.receipts.length, 2);
  const sends = calls.filter(
    /** Extracts only actual observed send boundary calls. */
    call => Object.hasOwn(call, "send"),
  ).map(
    /** Preserves the entire real SDK-shaped send input. */
    call => call.send,
  );
  assert.equal(sends.length, 2); assert.deepEqual(sends[0].input, { text: "Actual group input", attachments, replyToMessageTimestamp: 9 });
  assert.equal(sends[0].turn.participantId, second.id); assert.equal(sends[0].notifyReply, false);
  assert.equal(sends[1].kind, "continue"); assert.equal(sends[1].participantId, first.id); assert.equal(sends[1].userMessageTimestamp, 10); assert.equal(sends[1].notifyReply, true);
  assert.equal(snapshots.get(20).profile.id, second.id); assert.equal(snapshots.get(21).profile.id, first.id); assert.deepEqual(runtime.failures, []);
});

/** Cancels during real planner suspension, keeps duplicate input intact and resumes only by an explicit UI action. */
test("production group menu and Compose controls cancel and resume the exact admitted submission without duplicate input", async t => {
  const runtime = await environment(t); await bindGroup(runtime); const planning = barrier(), sends = [], plannerCalls = [];
  /** Suspends the only actual functional-planner call at its controlled AI boundary. */
  runtime.tools.Chat.call = request => { plannerCalls.push(plain(request)); return planning.promise; };
  /** Acknowledges the explicit user-only persistence requested by an empty formal group plan. */
  runtime.tools.Chat.sendMessage = async request => { sends.push(plain(request)); return receipt(request); };
  const admitted = await runtime.submit(), duplicate = await runtime.submit();
  assert.equal(duplicate.action, "Block"); assert.equal(duplicate.clearInput, false); assert.equal(duplicate.metadata.submissionId, admitted.metadata.submissionId);
  const menu = await runtime.api("chat.context.actions", { chatId: "selector-chat" }), action = menu.selectors.find(
    /** Requires the exact owner-registered controls action in the real production menu projection. */
    entry => entry.id === "group-execution",
  );
  assert.deepEqual(plain(action), { id: "group-execution", title: "群组执行", icon: "Groups", routeId: "toolpkg:com.operit.character_cards:ui:group-execution", input: { mode: "group-execution", chatId: "selector-chat" } });
  const ui = controls(runtime); await ui.load();
  assert.equal(keyedNode(ui.render().tree, "group-execution-cancel").props.enabled, true);
  await ui.dispatch(keyedNode(ui.render().tree, "group-execution-cancel").props.onClick);
  planning.resolve({ text: JSON.stringify({ rounds: [[]] }) });
  assert.equal((await settled(runtime, admitted.metadata.submissionId)).outcome.status, "cancelled");
  assert.equal(sends.length, 0);
  const blocked = await runtime.submit(); assert.equal(blocked.action, "Block"); assert.equal(blocked.clearInput, false);
  await ui.dispatch(keyedNode(ui.render().tree, "group-execution-refresh").props.onClick);
  assert.equal(keyedNode(ui.render().tree, "group-execution-resume").props.enabled, true);
  await ui.dispatch(keyedNode(ui.render().tree, "group-execution-resume").props.onClick);
  const done = await settled(runtime, admitted.metadata.submissionId); assert.equal(done.outcome.status, "completed");
  assert.equal(sends.length, 1); assert.equal(sends[0].turn.kind, "record_only"); assert.equal(plannerCalls.length, 1);
  await ui.dispatch(keyedNode(ui.render().tree, "group-execution-refresh").props.onClick);
  assert.equal(keyedNode(ui.render().tree, "group-execution-cancel").props.enabled, false);
  assert.equal(keyedNode(ui.render().tree, "group-execution-resume").props.enabled, false);
  const bytes = await readFile(path.join(runtime.disk.directory, "character-memory/state.json"));
  const closed = await ui.dispatch(keyedNode(ui.render().tree, "group-execution-close").props.onClick);
  assert.deepEqual(plain(closed.actionResult), { type: "toolpkg.presentation.cancel", requestId: "group-controls" });
  assert.deepEqual(await readFile(path.join(runtime.disk.directory, "character-memory/state.json")), bytes);
});

/** Forwards active cancellation through the real native transport and resumes against the saved user locator. */
test("registered group cancellation captures a live send and explicit resume never persists the user twice", async t => {
  const runtime = await environment(t), { first } = await bindGroup(runtime), pending = barrier(), started = barrier(), sends = [], cancels = [], snapshots = new Map();
  const readExtension = runtime.tools.Chat.readExtension;
  /** Reads only the actual controlled acknowledged message snapshots. */
  runtime.tools.Chat.readExtension = async target => target.kind === "chat" ? readExtension(target) : snapshots.get(target.messageTimestamp);
  /** Plans one exact persisted participant with no automatic alternate plan. */
  runtime.tools.Chat.call = async () => ({ text: JSON.stringify({ rounds: [[{ id: first.id, speak: true }]] }) });
  /** Holds the first native receipt and creates a genuine plugin snapshot for the explicit continuation. */
  runtime.tools.Chat.sendMessage = async request => {
    sends.push(plain(request));
    if (sends.length === 1) { started.resolve(); return pending.promise; }
    const config = await runtime.api("chat.configuration.resolve", { purpose: "execution", chatId: request.chatId,
      chatExtension: runtime.records.get(request.chatId).extension, messageExtension: null, participantId: request.participantId, promptFunctionType: "CHAT",
      defaultModelBinding: { providerId: "dashscope", modelId: "qwen-plus" }, defaultTtsConfigId: "tts-one" });
    snapshots.set(30, plain(config.messageExtension)); return receipt(request, "completed", { messageTimestamp: 30, variantIndex: 0 });
  };
  /** Acknowledges only the authenticated owner/chat cancellation while its real transport attempt is live. */
  runtime.tools.Chat.cancel = async chatId => { cancels.push(chatId); pending.resolve(receipt(sends[0], "cancelled")); return { chatId, cancelRequested: true }; };
  const admitted = await runtime.submit(); await started.promise;
  await runtime.groupExecution({ action: "cancel", chatId: "selector-chat", submissionId: admitted.metadata.submissionId });
  assert.equal((await settled(runtime, admitted.metadata.submissionId)).outcome.status, "cancelled");
  assert.deepEqual(cancels, ["selector-chat"]);
  await runtime.groupExecution({ action: "resume", chatId: "selector-chat", submissionId: admitted.metadata.submissionId });
  const completed = await settled(runtime, admitted.metadata.submissionId); assert.equal(completed.status, "settled", completed.error); assert.equal(completed.outcome.status, "completed");
  assert.equal(sends.length, 2); assert.equal(sends[1].kind, "continue"); assert.equal(sends[1].userMessageTimestamp, 10);
});

/** Reports terminal planner failure through the real IPC and keeps both mutation controls disabled. */
test("registered group failure is visible in production controls and never automatically replans or sends", async t => {
  const runtime = await environment(t); await bindGroup(runtime); let plans = 0;
  /** Propagates one deliberately controlled original AI rejection. */
  runtime.tools.Chat.call = async () => { plans++; throw new Error("Original configured planner rejection"); };
  const admitted = await runtime.submit(), done = await settled(runtime, admitted.metadata.submissionId);
  assert.equal(done.status, "failed"); assert.match(done.error, /Original configured planner rejection/); assert.equal(plans, 1);
  assert.equal(runtime.failures.length, 1); assert.match(runtime.failures[0], /Original configured planner rejection/);
  const ui = controls(runtime); await ui.load();
  assert.equal(keyedNode(ui.render().tree, "group-execution-cancel").props.enabled, false);
  assert.equal(keyedNode(ui.render().tree, "group-execution-resume").props.enabled, false);
  await assert.rejects(runtime.groupExecution({ action: "resume", chatId: "selector-chat", submissionId: admitted.metadata.submissionId }), /settled cancelled/);
  await assert.rejects(runtime.groupExecution({ action: "cancel", chatId: "selector-chat", submissionId: "wrong-submission" }), /matching retained/);
  assert.equal(plans, 1);
});

/** Ensures a closed or retargeted modal cannot acquire a different conversation and empty state is read-only. */
test("registered group controls expose true empty state, explicit V1 close, and reject presentation retargeting", async t => {
  const runtime = await environment(t), ui = controls(runtime); await ui.load();
  assert.equal(keyedNode(ui.render().tree, "group-execution-status").props.text, "当前会话没有已提交的群组任务");
  assert.equal(runtime.disk.calls.length, 0);
  const stale = keyedNode(ui.render().tree, "group-execution-refresh").props.onClick;
  assert.throws(
    /** Changes the real SDK-owned presentation state to prove retained callbacks cannot retarget. */
    () => ui.retarget({ requestId: "other-request", input: { mode: "group-execution", chatId: "other-chat" } }), /owner changed/,
  );
  await assert.rejects(ui.dispatch(stale), /owner changed/);
  assert.equal(runtime.disk.calls.length, 0);
});

import assert from "node:assert/strict";
import test from "node:test";
import { selectorEnvironment } from "./selector-environment.mjs";
import { plain, loadModule } from "./runtime.mjs";

const start = { action: "start", chatId: "selector-chat", requestId: "call-turn-1", text: "Hello everyone", attachments: [] };
const status = { action: "status", chatId: start.chatId, requestId: start.requestId };
const cancel = { ...status, action: "cancel" };
function receipt(request, assistant = null, text = null, state = "completed") {
  return { chatId: request.chatId, sentAt: 10, receivedAt: null, message: "", aiResponse: text,
    outcome: { type: "committed", status: state, userMessageTimestamp: 10, assistant } };
}
async function bind(runtime) {
  const first = await runtime.api("character.create", { values: { name: "Alice", ttsConfigId: "tts-one", avatarUri: "alice.png" } });
  const second = await runtime.api("character.create", { values: { name: "Bob" } });
  const group = await runtime.api("group.create", { values: { name: "Friends", members: [
    { characterCardId: first.id, orderIndex: 0 }, { characterCardId: second.id, orderIndex: 1 },
  ] } });
  await runtime.api("chat.configuration.binding.write", { chatId: start.chatId, selection: "group:" + group.id });
  return { first, second };
}
async function settled(runtime) {
  for (let i = 0; i < 200; i++) {
    const value = await runtime.api("chat.groupTurn", status);
    if (value.status !== "running") return plain(value);
    await new Promise(resolve => setTimeout(resolve, 2));
  }
  assert.fail("Group turn did not settle");
}

test("registered group API returns not_group without submitting single-chat input", async t => {
  const runtime = await selectorEnvironment(t);
  runtime.tools.Chat.sendMessage = () => assert.fail("not_group must leave submission to the caller");
  await runtime.api("chat.groupTurn", start);
  assert.deepEqual(await settled(runtime), { status: "not_group", replies: [], error: null });
});

test("public group turn uses the real planner, persists input once and returns ordered member voices", async t => {
  const runtime = await selectorEnvironment(t), { first, second } = await bind(runtime);
  const sends = [], authors = new Map();
  const originalRead = runtime.tools.Chat.readExtension;
  runtime.tools.Chat.readExtension = async target => target.kind === "chat" ? originalRead(target) : { extension: authors.get(target.messageTimestamp) };
  runtime.tools.Chat.call = async () => ({ text: JSON.stringify({ rounds: [[{ id: second.id, speak: true }, { id: first.id, speak: true }]] }) });
  runtime.tools.Chat.sendMessage = async request => {
    sends.push(plain(request));
    if (request.kind === "submit") return receipt(request);
    const timestamp = 20 + authors.size;
    authors.set(timestamp, { participantId: request.participantId });
    return receipt(request, { messageTimestamp: timestamp, variantIndex: 0 }, "Reply from " + request.participantId);
  };
  const attachment = { filePath: "/camera.jpg", fileName: "camera.jpg", mimeType: "image/jpeg", fileSize: 10, content: "" };
  await runtime.api("chat.groupTurn", { ...start, attachments: [attachment] });
  const result = await settled(runtime);
  assert.equal(result.status, "completed");
  assert.deepEqual(result.replies.map(reply => [reply.participantId, reply.name, reply.ttsConfigId]), [[second.id, "Bob", null], [first.id, "Alice", "tts-one"]]);
  assert.deepEqual(result.replies.map(reply => reply.text), ["Reply from " + second.id, "Reply from " + first.id]);
  assert.equal(sends.filter(request => request.kind === "submit").length, 1);
  assert.deepEqual(sends[0].turn, { kind: "record_only" });
  assert.deepEqual(sends[0].input.attachments, [attachment]);
  assert.ok(sends.every(request => request.runtime === "floating" && request.notifyReply === false));
  assert.deepEqual(sends.slice(1).map(request => request.participantId), [second.id, first.id]);
  assert.ok(sends.slice(1).every(request => request.userMessageTimestamp === 10));
  assert.equal((await runtime.api("chat.groupTurn", cancel)).status, "completed");
});

test("group hangup cancels its actual native send and prevents subsequent member turns", async t => {
  const runtime = await selectorEnvironment(t), { first, second } = await bind(runtime);
  let started, resolveSend;
  const live = new Promise(resolve => { started = resolve; });
  const sends = [], cancels = [];
  runtime.tools.Chat.call = async () => ({ text: JSON.stringify({ rounds: [[{ id: first.id, speak: true }, { id: second.id, speak: true }]] }) });
  runtime.tools.Chat.sendMessage = async request => {
    sends.push(plain(request));
    if (request.kind === "submit") return receipt(request);
    started();
    return new Promise(resolve => { resolveSend = () => resolve(receipt(request, null, null, "cancelled")); });
  };
  runtime.tools.Chat.cancel = async chatId => { cancels.push(chatId); resolveSend(); return { chatId, cancelRequested: true }; };
  await runtime.api("chat.groupTurn", start);
  await live;
  await runtime.api("chat.groupTurn", cancel);
  assert.equal((await settled(runtime)).status, "cancelled");
  assert.deepEqual(cancels, [start.chatId]);
  assert.equal(sends.length, 2);
});

test("failed native replies retain their error without returning a manufactured voice reply", async t => {
  const runtime = await selectorEnvironment(t), { first } = await bind(runtime);
  runtime.tools.Chat.call = async () => ({ text: JSON.stringify({ rounds: [[{ id: first.id, speak: true }]] }) });
  runtime.tools.Chat.sendMessage = async request => {
    if (request.kind === "submit") return receipt(request);
    throw new Error("Group model unavailable");
  };
  await runtime.api("chat.groupTurn", start);
  const result = await settled(runtime);
  assert.equal(result.status, "failed");
  assert.match(result.error, /Group model unavailable/);
  assert.deepEqual(result.replies, []);
});

test("group status and cancellation require the same authenticated caller and exact request", async () => {
  const { createGroupTurnApi } = loadModule("src/group-execution/public-turn.ts");
  const api = createGroupTurnApi({ async selection() { return "model:chat"; }, async controller() { assert.fail("not a group"); } });
  await api({ callerPackage: "voice", payload: start });
  await assert.rejects(api({ callerPackage: "other", payload: cancel }), /matching group turn/);
  await assert.rejects(api({ callerPackage: "voice", payload: { ...status, requestId: "wrong" } }), /matching group turn/);
});


test("a new group utterance waits for cancelled work to release the native turn", async () => {
  const { createGroupTurnApi } = loadModule("src/group-execution/public-turn.ts");
  let finishFirst;
  const submissions = [];
  const api = createGroupTurnApi({
    async selection() { return "group:friends"; },
    async controller() { return {
      submit(input) {
        submissions.push(input.submissionId);
        if (submissions.length === 1) return new Promise(resolve => { finishFirst = () => resolve({ status: "cancelled" }); });
        return Promise.resolve({ status: "completed" });
      },
      async cancel() {},
    }; },
  });
  const invoke = payload => api({ callerPackage: "voice", payload });
  await invoke(start);
  await new Promise(resolve => setImmediate(resolve));
  await invoke(cancel);
  const second = { ...start, requestId: "second-turn" };
  assert.equal((await invoke(second)).status, "running");
  await new Promise(resolve => setImmediate(resolve));
  assert.deepEqual(submissions, [start.requestId]);
  finishFirst();
  await new Promise(resolve => setImmediate(resolve));
  assert.deepEqual(submissions, [start.requestId, second.requestId]);
  assert.equal((await invoke({ ...second, action: "status" })).status, "completed");
});


test("cancellation tolerates a reply already entering native finalization", async () => {
  const { createGroupTurnApi } = loadModule("src/group-execution/public-turn.ts");
  let finish;
  const api = createGroupTurnApi({
    async selection() { return "group:friends"; },
    async controller() { return {
      submit() { return new Promise(resolve => { finish = () => resolve({ status: "completed" }); }); },
      async cancel() { finish(); throw new Error("Group sequence is no longer cancellable"); },
    }; },
  });
  await api({ callerPackage: "voice", payload: start });
  await new Promise(resolve => setImmediate(resolve));
  assert.equal((await api({ callerPackage: "voice", payload: cancel })).status, "completed");
});

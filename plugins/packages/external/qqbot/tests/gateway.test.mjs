import assert from "node:assert/strict";
import test from "node:test";
import { c2c, clock, load, plain, socketPort } from "./harness.mjs";

/** Creates an authenticated Gateway fixture using the production state machine. */
async function fixture(consumer) {
  const time = clock(), socket = socketPort(), events = [];
  const { QQGateway } = load("gateway", time);
  const gateway = new QQGateway({ async gateway() { return "wss://gateway.example/"; }, async accessToken() { return "access-token"; } }, socket, consumer === undefined ? event => events.push(event) : consumer);
  await gateway.start();
  socket.frame({ op: 10, d: { heartbeat_interval: 500 } });
  await time.advance(100);
  socket.frame({ op: 0, t: "READY", s: 1, d: { session_id: "session-1" } });
  await time.advance(100);
  return { time, socket, events, gateway };
}

test("HELLO identifies with QQBot credentials and only READY marks the service healthy", async () => {
  const { gateway, socket, time } = await fixture();
  assert.equal(gateway.status, "ready");
  assert.equal(gateway.sessionId, "session-1");
  assert.deepEqual(plain(socket.sent[0].payload), { op: 2, d: { token: "QQBot access-token", intents: 1 << 25, shard: [0, 1], properties: { os: "operit", browser: "operit", device: "operit" } } });
  await gateway.stop();
  assert.equal(time.pending(), 0);
  assert.equal(socket.closed.length, 1);
});

test("C2C and group dispatch retain the schema-specific sender, target and reply source", async () => {
  const { gateway, socket, time, events } = await fixture();
  socket.frame(c2c());
  socket.frame({ op: 0, s: 3, t: "GROUP_AT_MESSAGE_CREATE", d: { id: "group-message", content: "hi", timestamp: "2026-10-24T00:00:00Z", group_openid: "group-1", author: { member_openid: "member-1" } } });
  await time.advance(100);
  assert.deepEqual(plain(events.map(event => event.replyHint)), [
    { scene: "c2c", msg_id: "message-1", event_id: "event-message-1", openid: "user-1", group_openid: "" },
    { scene: "group", msg_id: "group-message", event_id: "", openid: "member-1", group_openid: "group-1" },
  ]);
  assert.equal(gateway.receivedCount, 2);
  await gateway.stop();
});

test("heartbeat carries the latest sequence and missing ACK closes exactly one connection", async () => {
  const { gateway, socket, time } = await fixture();
  socket.frame(c2c());
  await time.advance(400);
  assert.deepEqual(plain(socket.sent.at(-1).payload), { op: 1, d: 2 });
  await time.advance(500);
  assert.equal(gateway.status, "error");
  assert.match(gateway.lastError, /acknowledgement deadline/);
  assert.equal(socket.opened.length, 1);
  assert.equal(socket.closed.length, 1);
  assert.equal(time.pending(), 0);
});

test("ACK permits the next heartbeat without reopening a healthy socket", async () => {
  const { gateway, socket, time } = await fixture();
  await time.advance(400);
  socket.frame({ op: 11, d: null });
  await time.advance(500);
  assert.equal(gateway.status, "ready");
  assert.equal(socket.sent.filter(item => item.payload.op === 1).length, 2);
  assert.equal(socket.opened.length, 1);
  await gateway.stop();
});

test("server opcode 7 resumes only the exact recorded session and sequence", async () => {
  const { gateway, socket, time } = await fixture();
  socket.frame(c2c()); socket.frame({ op: 7, d: null });
  await time.advance(100);
  assert.equal(socket.opened.length, 2);
  assert.equal(socket.closed.length, 1);
  socket.frame({ op: 10, d: { heartbeat_interval: 500 } });
  await time.advance(100);
  assert.deepEqual(plain(socket.sent.at(-1).payload), { op: 6, d: { token: "QQBot access-token", session_id: "session-1", seq: 2 } });
  socket.frame({ op: 0, t: "RESUMED", s: 3, d: {} });
  await time.advance(100);
  assert.equal(gateway.status, "ready");
  await gateway.stop();
});

test("server INVALID_SESSION false clears the rejected session before a fresh IDENTIFY", async () => {
  const { gateway, socket, time } = await fixture();
  socket.frame({ op: 9, d: false }); await time.advance(100);
  assert.equal(gateway.sessionId, "");
  socket.frame({ op: 10, d: { heartbeat_interval: 500 } }); await time.advance(100);
  assert.equal(socket.sent.at(-1).payload.op, 2);
  await gateway.stop();
});

test("malformed message author is rejected without alias guessing or reconnect", async () => {
  const { gateway, socket, time, events } = await fixture();
  const frame = c2c(); frame.d.author = { id: "wrong-alias" };
  socket.frame(frame); await time.advance(100);
  assert.equal(gateway.status, "error");
  assert.match(gateway.lastError, /openid/);
  assert.equal(events.length, 0);
  assert.equal(socket.opened.length, 1);
});

test("event consumer overflow terminates the Gateway instead of dropping the oldest event", async () => {
  const { gateway, socket, time } = await fixture(() => { throw new Error("queue full"); });
  socket.frame(c2c()); await time.advance(100);
  assert.equal(gateway.status, "error");
  assert.match(gateway.lastError, /queue full/);
  assert.equal(socket.closed.length, 1);
});

test("stop invalidates an in-flight authentication token continuation", async () => {
  const time = clock(), socket = socketPort();
  const { QQGateway } = load("gateway", time);
  let resolveToken;
  const gateway = new QQGateway({ async gateway() { return "wss://gateway.example"; }, accessToken() { return new Promise(resolve => { resolveToken = resolve; }); } }, socket, () => {});
  await gateway.start(); socket.frame({ op: 10, d: { heartbeat_interval: 500 } }); await time.advance(100);
  await gateway.stop(); resolveToken("late-token"); await time.advance(100);
  assert.equal(gateway.status, "stopped");
  assert.equal(socket.sent.length, 0);
  assert.equal(time.pending(), 0);
});

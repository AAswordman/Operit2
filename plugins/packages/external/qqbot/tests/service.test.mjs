import assert from "node:assert/strict";
import test from "node:test";
import { provider } from "../../waifu/tests/harness.mjs";
import { c2c, clock, load, plain, settle, socketPort } from "./harness.mjs";

/** Executes the real service against explicit networking, configuration and Chat Hosts. */
function fixture(options = {}) {
  const time = clock(), socket = socketPort();
  const config = load("model").defaultConfig(); config.appId = "app"; config.appSecret = "private-secret";
  const http = [], created = [], submitted = [], cancelled = [], errors = [];
  let flushes = 0, pendingReply = null;
  const waifu = provider({ ...time });
  const initialized = waifu.call("settings.get", { profile: "default" }).then(settings => waifu.call("settings.set", { profile: "default", settings: { ...settings, charDelayMs: 0, enableEmoticons: true } }));
  const Tools = {
    Net: {
      async http(request) {
        http.push(request);
        let response;
        if (request.url === "https://bots.qq.com/app/getAppAccessToken" && request.body.clientSecret === options.rejectSecret) return { statusCode: 401, content: "invalid credentials" };
        if (request.url === "https://bots.qq.com/app/getAppAccessToken") response = { access_token: "token", expires_in: "7200" };
        else if (request.url.endsWith("/gateway/bot")) response = { url: "wss://gateway.example" };
        else if (request.url.endsWith("/users/@me")) response = { id: "bot" };
        else if (request.url.endsWith("/files")) response = { file_info: "uploaded-image" };
        else response = { id: "reply" };
        return { statusCode: 200, content: JSON.stringify(response) };
      },
      async webSocket(request) {
        const response = await socket.request(request);
        if (request.op === "open") socket.frame({ op: 10, d: { heartbeat_interval: 5000 } });
        if (request.op === "send") {
          const payload = JSON.parse(request.text);
          if (payload.op === 2) socket.frame({ op: 0, t: "READY", s: 1, d: { session_id: "session" } });
          if (payload.op === 1) socket.frame({ op: 11, d: null });
        }
        return response;
      },
    },
    Chat: {
      async createNew(request) { created.push(request); return { chatId: `chat-${created.length}`, createdAt: time.Date.now() }; },
      async updateTitle() {}, async updateGroup() {},
      async *sendMessageStreaming(request) {
        submitted.push(request);
        if (options.deferReply) {
          if (options.firstPart) yield { type: "part", messageTimestamp: 1, parts: [{ kind: "markdown", content: options.firstPart }] };
          const result = await new Promise(resolve => { pendingReply = resolve; });
          yield { type: "completed", result }; return;
        }
        yield { type: "part", messageTimestamp: 1, parts: [{ kind: "markdown", content: options.replyText === undefined ? "actual AI response" : options.replyText }] };
        yield { type: "completed", result: { chatId: request.chatId, aiResponse: "unrendered final response", outcome: { type: "committed", status: "completed" } } };
      },
      async cancel(chatId) { cancelled.push(chatId); if (pendingReply !== null) { pendingReply({ chatId, outcome: { type: "committed", status: "cancelled" } }); pendingReply = null; } return { chatId, cancelRequested: true }; },
    },
  };
  Tools.Files = { async readBinary() { return { contentBase64: "aW1hZ2U=", size: 5 }; } };
  const service = load("service", { ...time, Tools, PluginConfig: { async use() { return config; }, async flush() { flushes++; } }, ToolPkg: { ipc: { on() {} }, async callDependency(packageName, method, payload) { assert.equal(packageName, "com.operit.waifu"); await initialized; return waifu.call(method, payload, "com.operit.qqbot_bundle"); } } });
  /** Advances only the Host callbacks needed for a submitted administrative command. */
  async function complete(promise) {
    let completed = false, result, failure;
    promise.then(value => { completed = true; result = value; }, error => { completed = true; failure = error; });
    for (let index = 0; index < 300 && !completed; index++) { await settle(); if (!completed) await time.advance(100); }
    if (!completed) throw new Error("Administrative command did not settle");
    if (failure !== undefined) throw failure;
    return result;
  }
  return { config, time, socket, http, created, submitted, cancelled, errors, service, complete, flushes: () => flushes };
}

test("configuration and status never return the persisted application secret", async () => {
  const f = fixture();
  const result = await f.complete(f.service.receive({ action: "configure", params: { app_id: "updated-app" } }));
  assert.equal(result.status.appId, "updated-app");
  assert.equal(f.config.appSecret, "private-secret");
  assert.equal(JSON.stringify(result).includes("private-secret"), false);
  assert.ok(f.flushes() > 0);
  assert.equal(f.socket.opened.length, 0);
});

test("invalid automatic reply settings leave the durable configuration unchanged", async () => {
  const f = fixture(); const original = plain(f.config.autoReply);
  await assert.rejects(f.complete(f.service.receive({ action: "auto_configure", params: { enabled: true } })), /Enable the QQ listener/);
  assert.deepEqual(plain(f.config.autoReply), original);
  assert.equal(f.socket.opened.length, 0);
});

test("event peek, filtered consumption and raw payload selection use the same main queue", async () => {
  const f = fixture(); await f.complete(f.service.receive({ action: "start" }));
  f.socket.frame(c2c("one")); f.socket.frame(c2c("two")); await f.time.advance(100);
  const peek = await f.complete(f.service.receive({ action: "receive", params: { consume: false, limit: 1 } }));
  assert.equal(peek.events.length, 1); assert.equal(peek.remainingCount, 2); assert.equal(peek.events[0].rawPayload, undefined);
  const read = await f.complete(f.service.receive({ action: "receive", params: { include_raw: true } }));
  assert.equal(read.events.length, 2); assert.equal(read.remainingCount, 0); assert.equal(read.events[0].rawPayload.d.id, "one");
  await f.complete(f.service.receive({ action: "stop" })); assert.equal(f.time.pending(), 0);
});

test("duplicate remote messages are queued only once", async () => {
  const f = fixture(); await f.complete(f.service.receive({ action: "start" }));
  f.socket.frame(c2c()); f.socket.frame(c2c()); await f.time.advance(100);
  const result = await f.complete(f.service.receive({ action: "receive", params: {} }));
  assert.equal(result.returnedCount, 1);
  await f.complete(f.service.receive({ action: "stop" }));
});

test("automatic replies use independent durable chats and authoritative waifu snapshots", async () => {
  const f = fixture(); await f.complete(f.service.receive({ action: "start" }));
  await f.complete(f.service.receive({ action: "auto_configure", params: { enabled: true, character_selection: "card:chosen" } }));
  f.socket.frame(c2c("one", "alice")); f.socket.frame(c2c("two", "bob")); f.socket.frame(c2c("three", "alice"));
  await f.time.advance(1500);
  assert.equal(f.created.length, 2);
  assert.equal(f.created[0].setAsCurrentChat, false);
  assert.deepEqual(plain(f.created[0].input), { "com.operit.character_cards": { version: 1, selection: "card:chosen" } });
  assert.deepEqual(f.submitted.map(request => request.chatId), ["chat-1", "chat-2", "chat-1"]);
  assert.equal(f.submitted[0].kind, "submit"); assert.equal(f.submitted[0].turn.kind, "execute");
  const replies = f.http.filter(request => request.url.endsWith("/messages"));
  assert.equal(replies.length, 3);
  assert.equal(replies[0].body.content, "actual AI response"); assert.equal(replies[0].body.msg_id, "one");
  assert.deepEqual(plain(f.config.bindings), { "c2c:alice": "chat-1", "c2c:bob": "chat-2" });
  await f.complete(f.service.receive({ action: "stop" }));
});

test("stopping automatic replies cancels the active Chat and suppresses late QQ sends", async () => {
  const f = fixture({ deferReply: true }); await f.complete(f.service.receive({ action: "start" }));
  await f.complete(f.service.receive({ action: "auto_configure", params: { enabled: true } }));
  f.socket.frame(c2c()); await f.time.advance(400);
  assert.equal(f.submitted.length, 1);
  await f.complete(f.service.receive({ action: "auto_stop" }));
  assert.deepEqual(f.cancelled, ["chat-1"]);
  assert.equal(f.http.filter(request => request.url.endsWith("/messages")).length, 0);
  await f.complete(f.service.receive({ action: "stop" })); assert.equal(f.time.pending(), 0);
});

test("AI deadline cancels the real request and records the timeout without a fabricated reply", async () => {
  const f = fixture({ deferReply: true }); await f.complete(f.service.receive({ action: "start" }));
  await f.complete(f.service.receive({ action: "auto_configure", params: { enabled: true, ai_timeout_ms: 1000 } }));
  f.socket.frame(c2c()); await f.time.advance(1600);
  const result = await f.complete(f.service.receive({ action: "status" }));
  assert.match(result.status.autoReply.lastError, /AI deadline/);
  assert.deepEqual(f.cancelled, ["chat-1"]);
  assert.equal(f.http.filter(request => request.url.endsWith("/messages")).length, 0);
  await f.complete(f.service.receive({ action: "stop" }));
});

test("restarting replaces one native listener without leaving its timer or socket alive", async () => {
  const f = fixture(); await f.complete(f.service.receive({ action: "start" }));
  await f.complete(f.service.receive({ action: "start", restart: true }));
  assert.equal(f.socket.opened.length, 2); assert.equal(f.socket.closed.length, 1);
  await f.complete(f.service.receive({ action: "stop" }));
  assert.equal(f.socket.closed.length, 2); assert.equal(f.time.pending(), 0); assert.equal(f.config.listenerEnabled, false);
});


test("QQBot sends dependency sentences and real image messages separately with monotonic reply sequences", async () => {
  const f = fixture({ replyText: "你好。<emotion>happy</emotion>今天过得怎么样？" });
  await f.complete(f.service.receive({ action: "start" }));
  await f.complete(f.service.receive({ action: "auto_configure", params: { enabled: true } }));
  f.socket.frame(c2c("segmented")); await f.time.advance(1500);
  const sent = f.http.filter(request => request.url.endsWith("/messages"));
  assert.deepEqual(sent.map(request => request.body.msg_type), [0, 7, 0]);
  assert.deepEqual(sent.map(request => request.body.msg_seq), [1, 2, 3]);
  assert.deepEqual(sent.map(request => request.body.msg_id), ["segmented", "segmented", "segmented"]);
  assert.equal(sent[0].body.content, "你好。"); assert.equal(sent[2].body.content, "今天过得怎么样？");
  assert.equal(sent[1].body.media.file_info, "uploaded-image");
  assert.equal(f.http.filter(request => request.url.endsWith("/files")).length, 1);
  await f.complete(f.service.receive({ action: "stop" }));
});

test("QQBot sends the first stable sentence while the native Chat is still generating", async () => {
  const f = fixture({ deferReply: true, firstPart: "先回复你一句。后面还在生成" });
  await f.complete(f.service.receive({ action: "start" }));
  await f.complete(f.service.receive({ action: "auto_configure", params: { enabled: true } }));
  f.socket.frame(c2c()); await f.time.advance(500);
  const sent = f.http.filter(request => request.url.endsWith("/messages"));
  assert.equal(sent.length, 1); assert.equal(sent[0].body.content, "先回复你一句。");
  await f.complete(f.service.receive({ action: "auto_stop" }));
  assert.deepEqual(f.cancelled, ["chat-1"]);
  await f.complete(f.service.receive({ action: "stop" }));
});

test("reply role choices use public character APIs and expose only display names and selections", async () => {
  const config = load("model").defaultConfig(), calls = [];
  const service = load("service", {
    PluginConfig: { async use() { return config; } },
    ToolPkg: { async callDependency(packageName, method, input) {
      calls.push({ packageName, method, input: plain(input) });
      if (method === "character.list") return [{ id: "alice", name: "爱丽丝", prompt: "private prompt" }];
      if (method === "group.list") return [{ id: "friends", name: "朋友们", members: ["alice"] }];
      throw new Error(`Unexpected method ${method}`);
    } },
  });
  const result = await service.receive({ action: "reply_roles" });
  assert.deepEqual(plain(result), { success: true, roles: [
    { selection: "card:alice", name: "爱丽丝" }, { selection: "group:friends", name: "朋友们（角色组）" },
  ] });
  assert.deepEqual(calls, [
    { packageName: "com.operit.character_cards", method: "character.list", input: {} },
    { packageName: "com.operit.character_cards", method: "group.list", input: {} },
  ]);
});

test("setup can persist replies while stopped and then verify new credentials and start together", async () => {
  const f = fixture();
  const saved = await f.complete(f.service.receive({ action: "auto_configure", params: { enabled: true, start_now: false, character_selection: "card:chosen", ai_timeout_ms: 30000 } }));
  assert.equal(saved.status.autoReply.running, false);
  assert.equal(f.socket.opened.length, 0);
  const started = await f.complete(f.service.receive({ action: "configure", params: { app_id: "new-app", app_secret: "new-secret", listener_enabled: true, test_connection: true } }));
  assert.equal(started.status.service.status, "ready");
  assert.equal(started.status.autoReply.running, true);
  assert.equal(started.status.autoReply.config.characterSelection, "card:chosen");
  assert.equal(started.status.autoReply.config.aiTimeoutMs, 30000);
  const tokenRequest = f.http.find(request => request.url.endsWith("/getAppAccessToken"));
  assert.deepEqual(plain(tokenRequest.body), { appId: "new-app", clientSecret: "new-secret" });
  await f.complete(f.service.receive({ action: "stop" }));
});


test("failed credential validation preserves the working connection and persisted configuration", async () => {
  const f = fixture({ rejectSecret: "wrong-secret" });
  await f.complete(f.service.receive({ action: "start" }));
  await f.complete(f.service.receive({ action: "auto_configure", params: { enabled: true } }));
  const before = plain(f.config), flushes = f.flushes();
  await assert.rejects(f.complete(f.service.receive({ action: "configure", params: { app_id: "wrong-app", app_secret: "wrong-secret", listener_enabled: true, test_connection: true } })), /QQ HTTP 401/);
  assert.deepEqual(plain(f.config), before);
  assert.equal(f.flushes(), flushes);
  assert.equal(f.socket.closed.length, 0);
  const status = await f.service.receive({ action: "status" });
  assert.equal(status.status.service.status, "ready");
  assert.equal(status.status.autoReply.running, true);
  await f.complete(f.service.receive({ action: "stop" }));
});

test("failed first connection validation does not enable startup or persist incorrect credentials", async () => {
  const f = fixture({ rejectSecret: "wrong-secret" });
  await assert.rejects(f.complete(f.service.receive({ action: "configure", params: { app_id: "wrong-app", app_secret: "wrong-secret", listener_enabled: true, test_connection: true } })), /QQ HTTP 401/);
  assert.equal(f.config.listenerEnabled, false);
  assert.equal(f.config.appId, "app");
  assert.equal(f.config.appSecret, "private-secret");
  assert.equal(f.flushes(), 0);
  assert.equal(f.socket.opened.length, 0);
});

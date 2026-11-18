import assert from "node:assert/strict";
import test from "node:test";
import { objectStorage } from "./storage.mjs";
import { provider } from "../../waifu/tests/harness.mjs";
import { c2c, clock, load, plain, settle, socketPort } from "./harness.mjs";

/** Executes the real service against explicit networking, configuration and Chat Hosts. */
function fixture(options = {}) {
  const time = clock(), socket = socketPort(), storage = objectStorage();
  const values = Object.assign(load("model").defaultConfig(), options.initialConfig??{}); values.appId = "app"; values.appSecret = "private-secret";
  let dirty=false;const snapshots=[];
  const config = new Proxy(values,{set(target,key,value){target[key]=value;dirty=true;return true;}});
  const http = [], created = [], submitted = [], cancelled = [], errors = [];
  let flushes = 0; const pendingReplies = new Map();
  const waifu = provider({ ...time });
  const initialized = waifu.call("settings.get", { profile: "default" }).then(settings => waifu.call("settings.set", { profile: "default", settings: { ...settings, charDelayMs: 0, enableEmoticons: true } }));
  const Tools = {
    Net: {
      async http(request) {
        http.push(request);
        if (request.url.endsWith('/messages') && options.onSend) await options.onSend(request);
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
        if (options.deferReply || (options.deferAlice && request.chatId === "chat-1")) {
          if (options.firstPart) yield { type: "part", messageTimestamp: 1, parts: [{ kind: "markdown", content: options.firstPart }] };
          const result = await new Promise(resolve => { pendingReplies.set(request.chatId, resolve); });
          yield { type: "completed", result }; return;
        }
        yield { type: "part", messageTimestamp: 1, parts: [{ kind: "markdown", content: options.replyText === undefined ? "actual AI response" : options.replyText }] };
        yield { type: "completed", result: { chatId: request.chatId, aiResponse: options.finalReply ?? "unrendered final response", outcome: { type: "committed", status: "completed" } } };
      },
      async cancel(chatId) { cancelled.push(chatId); if (pendingReplies.has(chatId)) { pendingReplies.get(chatId)({ chatId, outcome: { type: "committed", status: "cancelled" } }); pendingReplies.delete(chatId); } return { chatId, cancelRequested: true }; },
    },
  };
  Tools.Storage = { objects: storage };
  Tools.Files = { async readBinary() { return { contentBase64: "aW1hZ2U=", size: 5 }; } };
  const service = load("service", { ...time, Tools, PluginConfig: { async use() { return config; }, async flush() { flushes++;if(!options.trackTopLevel||dirty){snapshots.push(plain(config));dirty=false;} } }, ToolPkg: { getLocalDataDir: () => "/plugin", ipc: { on() {} }, async callDependency(packageName, method, payload) { assert.equal(packageName, "com.operit.waifu"); if (options.waifuError) throw Error(options.waifuError); await initialized; return waifu.call(method, payload, "com.operit.qqbot_bundle"); } } });
  /** Advances only the Host callbacks needed for a submitted administrative command. */
  async function complete(promise) {
    let completed = false, result, failure;
    promise.then(value => { completed = true; result = value; }, error => { completed = true; failure = error; });
    for (let index = 0; index < 300 && !completed; index++) { await settle(); if (!completed) await time.advance(100); }
    if (!completed) throw new Error("Administrative command did not settle");
    if (failure !== undefined) throw failure;
    return result;
  }
  return { config, time, socket, http, created, submitted, cancelled, errors, service, storage, complete, snapshots, resolveReply(chatId) { const resolve = pendingReplies.get(chatId); if (resolve) { pendingReplies.delete(chatId); resolve({ chatId, outcome: { type: "committed", status: "completed" } }); } }, flushes: () => flushes };
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

test("the console can start QQ with native replies while optional Waifu is absent", async () => {
  const f = fixture({ waifuError: "ToolPkg container not found: com.operit.waifu", finalReply: "[[reply_to:internal-123]]您好" });
  await f.complete(f.service.channel({ action: "manage", command: "start", autoReply: true }));
  f.socket.frame(c2c("native-id"));
  await f.time.advance(1500);
  assert.equal(f.submitted.length, 1);
  const replies = f.http.filter(request => request.url.endsWith("/messages"));
  assert.equal(replies.length, 1);
  assert.equal(replies[0].body.content, "您好");
  assert.equal(replies[0].body.msg_id, "native-id");
  await f.complete(f.service.receive({ action: "stop" }));
});

test("optional Waifu fallback does not hide errors in a configured profile", async () => {
  const f = fixture({ waifuError: "Invalid Waifu profile" });
  await assert.rejects(f.complete(f.service.channel({ action: "manage", command: "start", autoReply: true })), /Invalid Waifu profile/);
  assert.equal(f.submitted.length, 0);
  assert.equal(f.config.listenerEnabled, false);
});

test("different QQ conversations progress concurrently while the same conversation stays ordered", async () => {
  const f = fixture({ deferAlice: true, firstPart: "先回答。" });
  await f.complete(f.service.receive({ action: "start" }));
  await f.complete(f.service.receive({ action: "auto_configure", params: { enabled: true } }));
  f.socket.frame(c2c("alice-1", "alice"));
  f.socket.frame(c2c("alice-2", "alice"));
  f.socket.frame(c2c("bob-1", "bob"));
  await f.time.advance(1500);
  assert.equal(f.submitted.length, 2);
  assert.ok(f.submitted.some(request => request.chatId === f.config.bindings["c2c:bob"]));
  assert.equal(f.submitted.filter(request => request.chatId === f.config.bindings["c2c:alice"]).length, 1);
  assert.ok(f.http.some(request => request.url.includes("/users/bob/messages")));
  f.resolveReply("chat-1");
  await f.time.advance(1000);
  assert.ok(f.submitted.filter(request => request.chatId === f.config.bindings["c2c:alice"]).length > 1);
  await f.complete(f.service.receive({ action: "stop" }));
});

test("QQ automatic replies bound concurrent conversations and stop cancels each active chat", async () => {
  const f = fixture({ deferReply: true });
  await f.complete(f.service.receive({ action: "start" }));
  await f.complete(f.service.receive({ action: "auto_configure", params: { enabled: true } }));
  for (const name of ["alice", "bob", "carol", "dave"]) f.socket.frame(c2c(`message-${name}`, name));
  await f.time.advance(2000);
  assert.equal(f.submitted.length, 3);
  assert.ok(!Object.hasOwn(f.config.bindings, "c2c:dave"));
  for (const request of f.submitted) {
    const author = Object.entries(f.config.bindings).find(([, chatId]) => chatId === request.chatId)[0].slice("c2c:".length);
    assert.doesNotMatch(request.input.text, /message-(alice|bob|carol)|"authorId"|"replyTo"/);
    const context = plain(await f.service.channel({action:"active_context",chatId:request.chatId}));
    assert.deepEqual(context, {channel:"qq",chatId:request.chatId,accountId:"app",scene:"c2c",conversationId:author,authorId:author});
    assert.doesNotMatch(JSON.stringify(context), /private-secret|token/);
  }
  assert.equal(await f.service.channel({action:"active_context",chatId:"unrelated"}),null);
  await f.complete(f.service.receive({ action: "auto_stop" }));
  assert.equal(await f.service.channel({action:"active_context",chatId:f.submitted[0].chatId}),null);
  assert.deepEqual([...f.cancelled].sort(), ["chat-1", "chat-2", "chat-3"]);
  assert.equal(f.http.filter(request => request.url.endsWith("/messages")).length, 0);
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

test("channel inbox persists independently from QQ automatic reply and deduplicates after acknowledgement", async () => {
  const f = fixture();
  await f.complete(f.service.channel({ action: "configure", capture: true }));
  await f.complete(f.service.receive({ action: "start" }));
  await f.complete(f.service.receive({ action: "auto_configure", params: { enabled: true } }));
  f.socket.frame(c2c("independent", "alice")); await f.time.advance(800);
  const first = await f.complete(f.service.channel({ action: "peek" }));
  assert.equal(first.messages.length, 1); assert.equal(first.messages[0].text, "hello"); assert.equal(first.messages[0].autoReplyEnabled,true);
  assert.equal(f.submitted.length, 1);
  await f.complete(f.service.channel({action:"ack", key:first.messages[0].key}));
  f.socket.frame(c2c("independent", "alice")); await f.time.advance(300);
  assert.equal((await f.complete(f.service.channel({action:"peek"}))).messages.length, 0);
  await f.complete(f.service.receive({ action: "stop" }));
});

test("plain QQ messages omit absent rich fields and remain durable for capture and subscribers", async () => {
  const f = fixture();
  await f.complete(f.service.channel({ action: "configure", capture: true }));
  await f.complete(f.service.receive({ action: "start" }));
  f.socket.frame(c2c("first", "alice")); await f.time.advance(300);
  assert.equal((await f.service.receive({ action: "status" })).status.service.status, "ready");
  const conversations = await f.service.channel({ action: "conversations" });
  await f.service.channel({ action: "subscribe", consumerId: "regression", routes: [{ conversationKey: conversations.conversations[0].key, since: 0 }] });
  f.socket.frame(c2c("second", "alice")); await f.time.advance(300);
  const status = await f.service.receive({ action: "status" });
  assert.equal(status.status.service.status, "ready");
  assert.equal(status.status.service.lastError, "");
  const captured = await f.service.channel({ action: "peek" });
  assert.equal(captured.messages.length, 2);
  for (const message of captured.messages) {
    for (const field of ["messageType", "card", "referenceIndex", "referencedMessageIndex"]) assert.equal(Object.hasOwn(message, field), false, field);
    assert.deepEqual(plain(message.attachments), []);
    assert.deepEqual(plain(message.elements), []);
  }
  const deliveries = await f.service.channel({ action: "peek", consumerId: "regression" });
  assert.equal(deliveries.messages.length, 1);
  assert.equal(deliveries.messages[0].messageId, "second");
  await f.complete(f.service.receive({ action: "stop" }));
});

test("QQ capture failure stops the listener instead of accepting the message", async () => {
  const f = fixture();
  await f.complete(f.service.channel({ action: "configure", capture: true }));
  await f.complete(f.service.receive({ action: "start" }));
  f.storage.fail(true); f.socket.frame(c2c("failed")); await f.time.advance(300);
  const status = await f.complete(f.service.receive({ action: "status" }));
  assert.equal(status.status.service.status, "error");
  assert.match(status.status.service.lastError, /disk unavailable/);
  assert.equal(status.status.queue.size, 0);
});

test("channel capture filters at the QQ source without consuming other bot messages", async () => {
  const f = fixture();
  await f.complete(f.service.channel({action:"configure",capture:true,conversationIds:["alice"]}));
  await f.complete(f.service.receive({action:"start"}));
  f.socket.frame(c2c("allowed", "alice")); f.socket.frame(c2c("excluded", "bob")); await f.time.advance(300);
  assert.deepEqual(plain((await f.complete(f.service.channel({action:"peek"}))).messages.map(x => x.conversationId)), ["alice"]);
  const events = await f.complete(f.service.receive({action:"receive",params:{}}));
  assert.equal(events.events.length, 2);
  await f.complete(f.service.receive({action:"stop"}));
});


test("automatic reply deduplicates within each QQ conversation, not across users", async () => {
  const f = fixture(); await f.complete(f.service.receive({action:"start"}));
  await f.complete(f.service.receive({action:"auto_configure", params:{enabled:true}}));
  f.socket.frame(c2c("same-id","alice")); f.socket.frame(c2c("same-id","bob")); f.socket.frame(c2c("same-id","alice"));
  await f.time.advance(1800);
  assert.equal(f.submitted.length,2); assert.equal(f.created.length,2);
  assert.equal(f.http.filter(r=>r.url.includes('/messages')).length,2);
  await f.complete(f.service.receive({action:"stop"}));
});

test("legacy QQ processed keys remain conservative after upgrade", async () => {
  const f = fixture(); f.config.processed=['c2c:old-id']; await f.complete(f.service.receive({action:"start"}));
  await f.complete(f.service.receive({action:"auto_configure", params:{enabled:true}}));
  f.socket.frame(c2c("old-id","alice")); await f.time.advance(1000);
  assert.equal(f.submitted.length,0);
  await f.complete(f.service.receive({action:"stop"}));
});


test("QQ maintenance-only routing stays live beyond the normal queue capacity", async () => {
  const f = fixture();
  await f.complete(f.service.channel({action:"configure",capture:true,conversationIds:["alice"],channelOnlyWhenNoAuto:true}));
  await f.complete(f.service.receive({action:"start"}));
  for (let i = 0; i < 210; i++) { f.socket.frame(c2c(`bulk-${i}`, "alice")); await f.time.advance(100); }
  assert.equal((await f.complete(f.service.receive({action:"status"}))).status.queue.size, 0);
  const messages = (await f.complete(f.service.channel({action:"peek",limit:100}))).messages;
  assert.equal(messages.length,100);
  for (const item of messages) await f.complete(f.service.channel({action:"ack",key:item.key}));
  const second = (await f.complete(f.service.channel({action:"peek",limit:100}))).messages;
  assert.equal(second.length,100);
  for (const item of second) await f.complete(f.service.channel({action:"ack",key:item.key}));
  assert.equal((await f.complete(f.service.channel({action:"peek",limit:100}))).messages.length,10);
  f.socket.frame(c2c("unselected", "bob")); await f.time.advance(200);
  assert.equal((await f.complete(f.service.receive({action:"status"}))).status.queue.size,1);
  await f.complete(f.service.receive({action:"stop"}));
});

test("QQ maintenance routing preserves the automatic reply path when enabled", async () => {
  const f=fixture();
  await f.complete(f.service.channel({action:"configure",capture:true,conversationIds:["alice"],channelOnlyWhenNoAuto:true}));
  await f.complete(f.service.receive({action:"start"}));
  await f.complete(f.service.receive({action:"auto_configure",params:{enabled:true}}));
  f.socket.frame(c2c("both","alice")); await f.time.advance(1000);
  assert.equal(f.submitted.length,1);
  const captured=await f.complete(f.service.channel({action:"peek"}));
  assert.equal(captured.messages.length,1); assert.equal(captured.messages[0].autoReplyEnabled,true);
  await f.complete(f.service.receive({action:"stop"}));
});


test("filtered channel peek scans past unrelated history without acknowledging it", async () => {
  const f = fixture();
  for (let i=0;i<150;i++) {
    const key=`a-${String(i).padStart(3,"0")}`;
    await f.storage.db.commit([{op:"put",collection:"pending",key,value:{key,conversationId:"other"},expectedVersion:null}]);
  }
  await f.storage.db.commit([{op:"put",collection:"pending",key:"z-target",value:{key:"z-target",accountId:"app",conversationId:"target"},expectedVersion:null}]);
  const result=await f.service.channel({action:"peek",limit:20,conversationId:"target"});
  assert.equal(result.messages.length,1); assert.equal(result.messages[0].key,"z-target");
  assert.notEqual(await f.storage.db.collection("pending").get("a-000"),null);
});

test('QQ channel can reply to an authenticated captured message after acknowledgement', async () => {
  const f=fixture(); await f.complete(f.service.channel({action:'configure',capture:true})); await f.complete(f.service.receive({action:'start'}));
  f.socket.frame(c2c('deferred','alice')); await f.time.advance(300);
  const [message]=(await f.complete(f.service.channel({action:'peek'}))).messages;
  await f.complete(f.service.channel({action:'ack',key:message.key}));
  await f.complete(f.service.channel({action:'reply',key:message.key,text:'已收到'}));
  const sent=f.http.filter(request=>request.url.endsWith('/messages'));
  assert.equal(sent.length,1); assert.equal(sent[0].body.msg_id,'deferred'); assert.equal(sent[0].body.content,'已收到');
  await f.complete(f.service.receive({action:'stop'}));
});

test("captured rich messages retain typed media and reference routing after acknowledgement without retaining user content", async () => {
  const f = fixture();
  await f.complete(f.service.channel({ action: "configure", capture: true }));
  await f.complete(f.service.receive({ action: "start" }));
  const frame = c2c("quoted");
  Object.assign(frame.d, { message_type: 103, message_scene: { ext: ["msg_idx=REFIDX_own==", "ref_msg_idx=REFIDX_previous==", "auth_token=private-token"] },
    msg_elements: [{ content: "quoted-private-text", attachments: [{ content_type: "image/png", filename: "photo.png", url: "https://cdn.example/photo", size: 3 }] }] });
  frame.d.author.username = "小明";
  f.socket.frame(frame); await f.time.advance(300);
  const message = (await f.service.channel({ action: "peek" })).messages[0];
  assert.equal(message.author.name, "小明"); assert.equal(message.referenceIndex, "REFIDX_own=="); assert.equal(message.elements[0].attachments[0].kind, "image");
  await f.service.channel({ action: "ack", key: message.key });
  const db = await f.storage.open({ path: "/plugin/channel_inbox.sqlite" });
  const route = (await db.collection("seen").get(message.key)).value.message;
  assert.doesNotMatch(JSON.stringify(route), /quoted-private-text|private-token|cdn.example|小明/);
  await f.service.channel({ action: "reply", key: message.key, media: { kind: "video", source: "https://cdn.example/video.mp4" }, text: "[[reply_to:internal]]视频" });
  const upload = f.http.find(request => request.url.endsWith("/files")), sent = f.http.find(request => request.url.endsWith("/messages"));
  assert.equal(upload.body.file_type, 2); assert.equal(sent.body.msg_type, 7);
  assert.equal(sent.body.msg_id, "quoted"); assert.equal(sent.body.message_reference.message_id, "REFIDX_own=="); assert.equal(sent.body.content, "视频");
  await f.complete(f.service.receive({ action: "stop" }));
});

test("automatic replies use readable quote context and native references while split internal tags never reach QQ", async () => {
  const f = fixture({ replyText: "[[reply_to:ROBOT1.0_internal]]收到。" });
  await f.complete(f.service.receive({ action: "start" }));
  await f.complete(f.service.receive({ action: "auto_configure", params: { enabled: true } }));
  const frame = c2c("secret-message", "secret-user");
  Object.assign(frame.d, { message_type: 103, message_scene: { ext: ["msg_idx=REFIDX_own==", "auth_token=secret-token"] }, msg_elements: [{ content: "用户引用的内容" }] });
  frame.d.author.username = "小明";
  f.socket.frame(frame); await f.time.advance(2000);
  assert.match(f.submitted[0].input.text, /> 用户引用的内容/); assert.match(f.submitted[0].input.text, /小明/);
  assert.doesNotMatch(f.submitted[0].input.text, /secret-token|REFIDX/);
  const sent = f.http.filter(request => request.url.endsWith("/messages"));
  assert.equal(sent.length, 1); assert.equal(sent[0].body.content, "收到。");
  assert.equal(sent[0].body.msg_id, "secret-message"); assert.equal(sent[0].body.message_reference.message_id, "REFIDX_own==");
  assert.equal((await f.service.receive({ action: "status" })).status.autoReply.lastError, "");
  const [session]=(await f.service.channel({action:"conversations"})).conversations;
  const history=await f.service.channel({action:"history",conversationKey:session.key});
  assert.deepEqual(plain(history.messages.filter(m=>m.direction).map(m=>[m.text,m.delivery])),[["收到。","accepted"]]);
  await f.complete(f.service.receive({ action: "stop" }));
});

test('direct send records a timed-out request as unknown and never repeats it',async()=>{
  const f=fixture({onSend(){throw Error('socket closed: sensitive detail');}});
  await f.complete(f.service.receive({action:'start'}));
  f.socket.frame(c2c('known')); await f.time.advance(300);
  const [session]=(await f.service.channel({action:'conversations'})).conversations;
  await assert.rejects(f.service.receive({action:'send_c2c',params:{openid:session.conversationId,content:'direct reply',msg_id:'known'}}),/socket closed/);
  const history=await f.service.channel({action:'history',conversationKey:session.key});
  assert.equal(history.messages.at(-1).delivery,'unknown');
  assert.equal(history.messages.at(-1).text,'direct reply');
  assert.doesNotMatch(JSON.stringify(history),/sensitive detail/);
  assert.equal(f.http.filter(r=>r.url.endsWith('/messages')).length,1);
  await f.complete(f.service.receive({action:'stop'}));
});

function groupMessage(id, sender = "alice", group = "group-a", mentioned = true, content = "群消息") {
  return {op:0,s:3,t:mentioned?"GROUP_AT_MESSAGE_CREATE":"GROUP_MESSAGE_CREATE",id:`event-${id}`,d:{id,content,timestamp:"2026-11-17T00:00:00Z",group_openid:group,author:{member_openid:sender,username:sender}}};
}
test("ordinary group messages are recorded without model turns and mentioned replies read scoped history", async()=>{
  const f=fixture();await f.complete(f.service.receive({action:"start"}));
  await f.complete(f.service.receive({action:"auto_configure",params:{enabled:true}}));
  f.socket.frame(groupMessage("before","alice","group-a",false,"重新开始后积分没清空"));await f.time.advance(400);
  assert.equal(f.submitted.length,0);
  const session=(await f.service.channel({action:"conversations"})).conversations[0];
  assert.equal(session.scene,"group");assert.equal(session.chatId,null);
  f.socket.frame(groupMessage("mention","bob","group-a",true,"帮忙看看刚才的问题"));await f.time.advance(700);
  assert.equal(f.submitted.length,1);
  const history=(await f.service.receive({action:"chat_context",chatId:f.submitted[0].chatId})).context;
  assert.deepEqual(plain(history.messages.map(m=>m.authorId)),["alice","bob"]);
  assert.match(history.messages[0].text,/积分/);
  await assert.rejects(f.service.receive({action:"chat_context",chatId:"unrelated"}),/Bot/);
  await f.complete(f.service.receive({action:"stop"}));
});
test("group shared context stays linear while unrelated groups proceed",async()=>{
  const f=fixture({deferReply:true,firstPart:"已了解"});await f.complete(f.service.receive({action:"start"}));
  await f.complete(f.service.receive({action:"auto_configure",params:{enabled:true}}));
  f.socket.frame(groupMessage("one"));f.socket.frame(groupMessage("two","bob"));f.socket.frame(groupMessage("other","alice","group-b"));
  await f.time.advance(1300);assert.equal(f.submitted.length,2);
  const root=f.submitted[0].chatId;f.resolveReply(root);await f.time.advance(700);
  assert.equal(f.submitted.length,3);assert.equal(f.submitted[2].chatId,root);
  await f.complete(f.service.receive({action:"stop"}));
});
test("member isolation uses individual model chats and filters history while preserving group order",async()=>{
  const f=fixture();await f.complete(f.service.receive({action:"start"}));
  await f.complete(f.service.receive({action:"auto_configure",params:{enabled:true,group_context:"member"}}));
  f.socket.frame(groupMessage("alice-before","alice","group-a",false,"只给Alice的上下文"));
  f.socket.frame(groupMessage("bob-before","bob","group-a",false,"Bob的上下文"));
  f.socket.frame(groupMessage("alice-at"));f.socket.frame(groupMessage("bob-at","bob"));await f.time.advance(1500);
  assert.equal(f.submitted.length,2);assert.notEqual(f.submitted[0].chatId,f.submitted[1].chatId);
  const root=(await f.service.channel({action:"conversations"})).conversations[0].chatId;
  assert.equal(f.created[1].parentChatId,root);
  const history=(await f.service.receive({action:"chat_context",chatId:f.submitted[0].chatId})).context;
  assert.ok(history.messages.every(m=>m.authorId==="alice"));assert.equal(history.messages.length,2);
  await assert.rejects(f.service.receive({action:"chat_context",chatId:root}),/隔离/);
  await f.complete(f.service.receive({action:"stop"}));
});
test("all-message reply mode and listening off are explicit independent controls",async()=>{
  const f=fixture();await f.complete(f.service.receive({action:"start"}));
  await f.complete(f.service.receive({action:"auto_configure",params:{enabled:true,listen_group_messages:false}}));
  f.socket.frame(groupMessage("ignored","alice","group-a",false));await f.time.advance(500);
  assert.equal((await f.service.channel({action:"conversations"})).conversations.length,0);
  await f.complete(f.service.receive({action:"auto_configure",params:{group_reply_mode:"all"}}));
  f.socket.frame(groupMessage("reply-all","alice","group-a",false));await f.time.advance(700);assert.equal(f.submitted.length,1);
  await f.complete(f.service.receive({action:"stop"}));
});

test("delivery reservation survives config persistence and restart never automatically replays uncertain replies",async()=>{
 const f=fixture({deferReply:true,trackTopLevel:true});await f.complete(f.service.receive({action:"start"}));
 await f.complete(f.service.receive({action:"auto_configure",params:{enabled:true}}));
 f.socket.frame(groupMessage("pending-one","alice","group-a",true,"第一条反馈"));f.socket.frame(groupMessage("pending-two","bob","group-a",true,"第二条反馈"));await f.time.advance(700);
 assert.equal(f.submitted.length,1);assert.equal(f.snapshots.at(-1).deliveries[0].state,"processing");
 const interrupted=f.snapshots.at(-1);await f.complete(f.service.receive({action:"stop"}));
 const next=fixture({initialConfig:interrupted,trackTopLevel:true});await next.complete(next.service.receive({action:"start"}));await next.time.advance(800);
 assert.equal(next.submitted.length,0);assert.equal(next.config.deliveries[0].state,"failed");
 await next.complete(next.service.receive({action:"retry_delivery",key:next.config.deliveries[0].key,skip:true}));await next.time.advance(800);
 assert.equal(next.submitted.length,1);assert.match(next.submitted[0].input.text,/第二条反馈/);
 await next.complete(next.service.receive({action:"stop"}));
});

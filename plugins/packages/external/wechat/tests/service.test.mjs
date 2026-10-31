import assert from "node:assert/strict";
import test from "node:test";
import { fixture, load, message, plain, settle } from "./harness.mjs";

test("status never exposes saved bearer tokens and one start owns one long poll", async () => {
  const f = await fixture();
  assert.equal(
    JSON.stringify(await f.status()).includes("secret-token"),
    false,
  );
  await f.call({ action: "start" });
  assert.equal((await f.status()).service.state, "connecting");
  await f.call({ action: "start" });
  assert.equal(
    f.requests.filter((request) => request.url.endsWith("/getupdates")).length,
    1,
  );
  await f.batch([]);
  assert.equal((await f.status()).service.state, "ready");
  await f.stop();
  assert.equal(f.time.pending(), 0);
  assert.equal(f.config.listenerEnabled, false);
});

test("missing accounts and rejected startup never enable automatic restoration", async () => {
  const f = await fixture({ loggedOut: true });
  await assert.rejects(f.call({ action: "start" }), /扫码/);
  assert.equal(f.config.listenerEnabled, false);
  const g = await fixture({ startFailure: true });
  await assert.rejects(g.call({ action: "start" }), /-14/);
  assert.equal(g.config.listenerEnabled, false);
  assert.equal((await g.status()).service.state, "error");
});

test("QR verification resumes the original QR session and persists confirmed credentials", async () => {
  const f = await fixture({ loggedOut: true });
  await f.call({ action: "login_start" });
  await f.qr({ status: "need_verifycode" });
  assert.equal((await f.status()).login.state, "need_verifycode");
  await f.call({ action: "login_verify", code: "123456" });
  assert.match(f.requests.at(-1).url, /qrcode=login-secret&verify_code=123456/);
  await f.qr({
    status: "confirmed",
    bot_token: "new-token",
    ilink_bot_id: "new-bot",
    ilink_user_id: "owner",
    baseurl: "https://ilinkai.weixin.qq.com",
  });
  assert.equal(f.config.account.token, "new-token");
  assert.equal((await f.status()).login.state, "confirmed");
  assert.equal((await f.status()).login.qrContent, "");
  assert.equal(JSON.stringify(await f.status()).includes("new-token"), false);
});

test("QR redirects stay on trusted hosts and do not forward an existing bearer token", async () => {
  const f = await fixture();
  await f.call({ action: "login_start" });
  await f.qr({
    status: "scaned_but_redirect",
    redirect_host: "route.weixin.qq.com",
  });
  assert.match(f.requests.at(-1).url, /^https:\/\/route.weixin.qq.com\//);
  assert.equal(f.requests.at(-1).headers.Authorization, undefined);
  await f.qr({ status: "scaned_but_redirect", redirect_host: "evil.test" });
  assert.equal((await f.status()).login.state, "error");
});

test("cancelling a QR session discards a late confirmation instead of replacing the account", async () => {
  const f = await fixture();
  await f.call({ action: "login_start" });
  const cancelled = f.call({ action: "login_cancel" });
  await settle();
  await f.qr({
    status: "confirmed",
    bot_token: "late-token",
    ilink_bot_id: "wrong",
    ilink_user_id: "wrong",
    baseurl: "https://ilinkai.weixin.qq.com",
  });
  await cancelled;
  assert.equal(f.config.account.botId, "bot");
  assert.equal((await f.status()).login.state, "idle");
});

test("a malformed batch does not advance the cursor or partially append its preceding messages", async () => {
  const f = await fixture();
  await f.call({ action: "start" });
  const malformed = message(2);
  delete malformed.context_token;
  await f.batch([message(1), malformed], "bad-cursor");
  assert.equal(f.config.cursor, "");
  assert.equal(f.config.queue.length, 0);
  assert.equal((await f.status()).service.state, "error");
});

test("queue overflow leaves the entire server batch and cursor uncommitted", async () => {
  const f = await fixture();
  const model = load("model");
  f.config.queue = Array.from({ length: 199 }, (_, index) =>
    model.inbound(message(index + 1)),
  );
  await f.call({ action: "start" });
  await f.batch([message(200), message(201)], "overflow");
  assert.equal(f.config.queue.length, 199);
  assert.equal(f.config.cursor, "");
  assert.match((await f.status()).service.error, /队列已满/);
});

test("durable queue, cursor and context are committed together and remote IDs deduplicate", async () => {
  const f = await fixture();
  await f.call({ action: "start" });
  await f.batch(
    [message(1), message(1), message(2, "bob@im.wechat")],
    "next-cursor",
  );
  assert.equal(f.config.queue.length, 2);
  const committed = f.snapshots.find(
    (snapshot) => snapshot.cursor === "next-cursor",
  );
  assert.equal(committed.queue.length, 2);
  assert.equal(committed.contexts["alice@im.wechat"], "ctx-alice@im.wechat");
  const preview = await f.call({
    action: "receive",
    params: { consume: false, limit: 1 },
  });
  assert.equal(preview.events.length, 1);
  assert.equal(preview.events[0].raw, undefined);
  const result = await f.call({
    action: "receive",
    params: { user_id: "bob@im.wechat", include_raw: true },
  });
  assert.equal(result.events[0].raw.message_id, 2);
  assert.equal(f.config.queue.length, 1);
  assert.equal(f.snapshots.at(-1).queue.length, 1);
  await f.stop();
});

test("stop fences a late getUpdates response without consuming its messages", async () => {
  const f = await fixture();
  await f.call({ action: "start" });
  const stopped = f.call({ action: "stop" });
  await settle();
  await f.batch([message(1)], "late-cursor");
  await stopped;
  assert.equal(f.config.cursor, "");
  assert.equal(f.config.queue.length, 0);
  await f.call({ action: "start" });
  await f.batch([message(1)], "replayed");
  assert.equal(f.config.queue.length, 1);
  await f.stop();
});

test("automatic replies use the real Waifu client and a native conversation for each peer", async () => {
  const f = await fixture({ auto: true });
  f.config.autoReply.characterSelection = "card:alice";
  await f.call({ action: "start" });
  await f.batch([message(1), message(2), message(3, "bob@im.wechat")]);
  await f.time.advance(1200);
  assert.equal(f.created.length, 2);
  assert.equal(f.submitted.length, 3);
  assert.equal(f.submitted[0].chatId, f.submitted[1].chatId);
  assert.notEqual(f.submitted[0].chatId, f.submitted[2].chatId);
  assert.equal(
    f.created[0].input["com.operit.character_cards"].selection,
    "card:alice",
  );
  assert.equal(f.sends().length, 3);
  assert.equal(f.sends()[0].body.msg.context_token, "ctx-alice@im.wechat");
  assert.equal(f.config.delivery, null);
  assert.equal((await f.status()).autoReply.repliedCount, 3);
  await f.stop();
});

test("failed delivery remains recoverable, blocks later automatic claims and never retries itself", async () => {
  const f = await fixture({ auto: true, failSendAt: 1 });
  await f.call({ action: "start" });
  await f.batch([message(1), message(2)]);
  await f.time.advance(600);
  assert.equal(f.sends().length, 1);
  assert.equal(f.config.delivery.state, "failed");
  assert.equal(f.config.queue.length, 1);
  await f.time.advance(5000);
  assert.equal(f.sends().length, 1);
  await f.call({ action: "delivery_resolve", resolution: "discard" });
  await f.time.advance(400);
  assert.equal(f.sends().length, 2);
  assert.equal(f.config.delivery, null);
  await f.stop();
});

test("explicit retry reclaims the failed message once and retains its existing conversation", async () => {
  const f = await fixture({ auto: true, failSendAt: 1 });
  await f.call({ action: "start" });
  await f.batch([message(1)]);
  await f.time.advance(400);
  await f.call({ action: "delivery_resolve", resolution: "retry" });
  await f.time.advance(400);
  assert.equal(f.sends().length, 2);
  assert.equal(f.created.length, 1);
  assert.equal(f.config.delivery, null);
  await f.stop();
});

test("runtime restoration flags an interrupted delivery instead of silently replaying it", async () => {
  const config = load("model").defaultConfig();
  config.delivery = {
    event: load("model").inbound(message(1)),
    state: "processing",
    delivered: 2,
    error: "",
  };
  const f = await fixture({ config });
  assert.equal((await f.status()).autoReply.delivery.state, "failed");
  assert.equal((await f.status()).autoReply.delivery.delivered, 2);
  assert.equal(f.sends().length, 0);
});

test("stop cancels an active model and records the uncompleted delivery without sending a final receipt", async () => {
  const f = await fixture({ auto: true, deferReply: true });
  await f.call({ action: "start" });
  await f.batch([message(1)]);
  await f.time.advance(350);
  assert.equal(f.submitted.length, 1);
  await f.stop();
  assert.deepEqual(f.cancelled, ["chat-1"]);
  assert.equal(f.sends().length, 0);
  assert.equal(f.config.delivery.state, "failed");
  assert.equal(f.time.pending(), 0);
});

test("reply timeout cancels the actual native model and leaves an actionable failure", async () => {
  const f = await fixture({ auto: true, deferReply: true });
  f.config.autoReply.aiTimeoutMs = 1000;
  await f.call({ action: "start" });
  await f.batch([message(1)]);
  await f.time.advance(1400);
  assert.deepEqual(f.cancelled, ["chat-1"]);
  assert.equal(f.config.delivery.state, "failed");
  assert.match(f.config.delivery.error, /超时/);
  await f.stop();
});

test("a partially delivered reply preserves its successful segment count and never repeats it automatically", async () => {
  const f = await fixture({
    auto: true,
    failSendAt: 2,
    replyText: "第一句话。第二句话。",
  });
  await f.call({ action: "start" });
  await f.batch([message(1)]);
  await f.time.advance(1000);
  assert.equal(f.sends().length, 2);
  assert.equal(f.config.delivery.delivered, 1);
  assert.equal(f.config.delivery.state, "failed");
  assert.ok(f.snapshots.some((snapshot) => snapshot.delivery?.delivered === 1));
  await f.time.advance(3000);
  assert.equal(f.sends().length, 2);
  await f.stop();
});

test("stable streaming sentences reach WeChat before the model completes and stop preserves partial delivery", async () => {
  const f = await fixture({
    auto: true,
    deferReply: true,
    firstPart: "先说一句。后面仍在生成",
  });
  await f.call({ action: "start" });
  await f.batch([message(1)]);
  await f.time.advance(500);
  assert.equal(f.sends().length, 1);
  assert.equal(f.sends()[0].body.msg.item_list[0].text_item.text, "先说一句。");
  await f.stop();
  assert.equal(f.config.delivery.delivered, 1);
  assert.equal(f.config.delivery.state, "failed");
});

test("saving enabled reply edits does not cancel the in-flight model", async () => {
  const f = await fixture({ auto: true, deferReply: true });
  await f.call({ action: "start" });
  await f.batch([message(1)]);
  await f.time.advance(400);
  await f.call({
    action: "auto_configure",
    params: { character_selection: "card:new" },
  });
  assert.equal(f.cancelled.length, 0);
  assert.equal(f.config.delivery.state, "processing");
  await f.stop();
});

test("expired sessions disable startup and leave a direct rescan instruction", async () => {
  const f = await fixture();
  await f.call({ action: "start" });
  await f.batch([], "", { ret: -14, errmsg: "expired" });
  assert.equal(f.config.listenerEnabled, false);
  assert.equal((await f.status()).service.state, "error");
  assert.match((await f.status()).service.error, /重新扫码/);
  await f.time.advance(5000);
  assert.equal(
    f.requests.filter((request) => request.url.endsWith("/getupdates")).length,
    1,
  );
});

test("invalid queue flags fail without consuming messages", async () => {
  const f = await fixture();
  f.config.queue.push(load("model").inbound(message(1)));
  await assert.rejects(
    f.call({ action: "receive", params: { consume: "false" } }),
    /boolean/,
  );
  assert.equal(f.config.queue.length, 1);
});

test("quoted server message IDs resolve only within the originating peer history", async () => {
  const f = await fixture({ auto: true });
  await f.call({ action: "start" });
  const current = message(3, "alice@im.wechat", "new-message");
  current.item_list[0].ref_msg = { svr_id: "1" };
  current.item_list.push({
    type: 1,
    text_item: { text: "other quote" },
    ref_msg: { svr_id: "2" },
  });
  await f.batch([
    message(1, "alice@im.wechat", "old-alice"),
    message(2, "bob@im.wechat", "private-bob"),
    current,
  ]);
  await f.time.advance(1600);
  const input = f.submitted[2].input.text;
  assert.match(input, /old-alice/);
  assert.match(input, /引用消息 2/);
  assert.doesNotMatch(input, /private-bob/);
  await f.stop();
});

test("long Unicode replies split into bounded messages without breaking surrogate pairs", async () => {
  const f = await fixture({
    auto: true,
    replyText: "你".repeat(4100) + "😀".repeat(10) + "。",
  });
  await f.call({ action: "start" });
  await f.batch([message(1)]);
  await f.time.advance(400);
  const parts = f
    .sends()
    .map((request) => request.body.msg.item_list[0].text_item.text);
  assert.equal(parts.length, 2);
  assert.ok(parts.every((part) => Array.from(part).length <= 4000));
  assert.equal(parts.join(""), "你".repeat(4100) + "😀".repeat(10) + "。");
  await f.stop();
});

test("saved role changes do not mutate current conversations, and reset preserves the old chat", async () => {
  const f = await fixture({ auto: true });
  await f.call({ action: "start" });
  await f.batch([message(1)]);
  await f.time.advance(400);
  await f.call({
    action: "auto_configure",
    params: { character_selection: "card:new" },
  });
  await f.time.advance(100);
  await f.batch([message(2)], "cursor2");
  await f.time.advance(400);
  assert.equal(f.created.length, 1);
  await f.call({ action: "reset_conversation", user_id: "alice@im.wechat" });
  await f.time.advance(100);
  await f.batch([message(3)], "cursor3");
  await f.time.advance(400);
  assert.equal(f.created.length, 2);
  assert.equal(
    f.created[1].input["com.operit.character_cards"].selection,
    "card:new",
  );
  await f.stop();
});

test("logout deletes retained account credentials and contexts and cannot reuse the old API token", async () => {
  const f = await fixture();
  f.config.contexts.peer = "secret-context";
  f.config.bindings["bot:peer"] = "old-chat";
  await f.call({ action: "logout" });
  assert.equal(f.config.account, null);
  assert.deepEqual(plain(f.config.contexts), {});
  assert.equal(f.config.bindings["bot:peer"], "old-chat");
  await assert.rejects(
    f.call({
      action: "send_text",
      params: { user_id: "peer", context_token: "ctx", text: "hello" },
    }),
    /扫码/,
  );
  assert.equal(f.sends().length, 0);
});

import assert from "node:assert/strict";
import test from "node:test";
import { clock, load, plain, settle } from "./harness.mjs";

/** Loads the actual OpenAPI client with deterministic HTTP responses. */
function fixture(sandbox = false) {
  const time = clock(), calls = [];
  const { QQOpenApi } = load("openapi", time);
  const api = new QQOpenApi({ appId: "app", appSecret: "secret", useSandbox: sandbox }, {
    async request(options) {
      calls.push(options);
      const body = options.url === "https://bots.qq.com/app/getAppAccessToken" ? { access_token: "token", expires_in: "7200" } : { id: "outbound" };
      return { statusCode: 200, content: JSON.stringify(body) };
    },
  });
  return { api, calls, time };
}

test("token acquisition is coalesced and cached without exposing credentials in API headers", async () => {
  const { api, calls } = fixture();
  const tokens = await Promise.all([api.accessToken(), api.accessToken(), api.accessToken()]);
  assert.deepEqual(tokens, ["token", "token", "token"]);
  assert.equal(calls.length, 1);
  assert.deepEqual(plain(calls[0].body), { appId: "app", clientSecret: "secret" });
  await api.request("GET", "/users/@me");
  assert.equal(calls.length, 2);
  assert.equal(calls[1].headers.Authorization, "QQBot token");
  assert.equal(JSON.stringify(calls[1].headers).includes("secret"), false);
});

test("GET requests omit the absent body at the structured tool boundary", async () => {
  const { api, calls } = fixture();
  await api.request("GET", "/users/@me");
  assert.equal(Object.hasOwn(calls[1], "body"), false);
});

test("C2C and group replies use exact encoded endpoints and independent monotonic sequences", async () => {
  const { api, calls } = fixture();
  await api.send("c2c", "user/a", { content: "one", msg_id: "source" });
  await api.send("c2c", "user/a", { content: "two", msg_id: "source" });
  await api.send("group", "group/a", { content: "group", msg_id: "source" });
  const sent = calls.slice(1);
  assert.equal(sent[0].url, "https://api.sgroup.qq.com/v2/users/user%2Fa/messages");
  assert.equal(sent[2].url, "https://api.sgroup.qq.com/v2/groups/group%2Fa/messages");
  assert.deepEqual(sent.map(call => call.body.msg_seq), [1, 2, 1]);
  assert.equal(sent[0].body.msg_id, "source");
});

test("sandbox is an explicit environment selection", async () => {
  const { api, calls } = fixture(true);
  await api.send("group", "group", { content: "hello" });
  assert.equal(calls[1].url, "https://sandbox.api.sgroup.qq.com/v2/groups/group/messages");
});

test("unsuccessful HTTP preserves the remote status and never submits another request", async () => {
  const { QQOpenApi } = load("openapi", clock());
  let calls = 0;
  const api = new QQOpenApi({ appId: "app", appSecret: "secret", useSandbox: false }, { async request() { calls++; return { statusCode: 401, content: "credential rejected" }; } });
  await assert.rejects(api.accessToken(), /QQ HTTP 401: credential rejected/);
  await settle(); assert.equal(calls, 1);
});

test("expired cached tokens are refreshed before the next request", async () => {
  const { api, calls, time } = fixture();
  await api.accessToken(); await time.advance(7150000); await api.accessToken();
  assert.equal(calls.length, 2);
});

test("rich-message payload is preserved while explicit reply arguments stay authoritative", async () => {
  const { api, calls } = fixture();
  await api.send("c2c", "user", { msg_type: 2, msg_id: "source", msg_seq: 4, payload: { markdown: { content: "**hello**" }, msg_id: "unselected" } });
  assert.deepEqual(plain(calls[1].body), { markdown: { content: "**hello**" }, msg_id: "source", msg_type: 2, msg_seq: 4 });
});

test("invalid message and sequence arguments are rejected before the network is touched", async () => {
  const { api, calls } = fixture();
  await assert.rejects(api.send("c2c", "", { content: "hello" }), /target openid/);
  await assert.rejects(api.send("c2c", "user", { content: "hello", msg_seq: -1 }), /msg_seq/);
  await assert.rejects(api.send("group", "group", { content: "" }), /content/);
  assert.equal(calls.length, 0);
});

test("QQ image upload uses Host bytes and the official rich-media schema", async () => {
  const calls = [], read = [];
  const { QQOpenApi } = load("openapi", { ...clock(), Tools: { Files: { async readBinary(path) { read.push(path); return { size: 3, contentBase64: "YWJj" }; } } } });
  const api = new QQOpenApi({ appId: "app", appSecret: "secret", useSandbox: false }, {
    async request(request) {
      calls.push(request);
      const response = request.url.endsWith("getAppAccessToken") ? { access_token: "token", expires_in: "7200" } : request.url.endsWith("/files") ? { file_info: "official-file-info" } : { id: "sent" };
      return { statusCode: 200, content: JSON.stringify(response) };
    },
  });
  await api.send("c2c", "user", { content: "first", msg_id: "source" });
  await api.sendImage("c2c", "user", "/host/sticker.gif", "source");
  await api.sendImage("group", "group", "https://example.com/image.png", "source");
  assert.deepEqual(read, ["/host/sticker.gif"]);
  assert.deepEqual(plain(calls[2].body), { file_type: 1, srv_send_msg: false, file_name: "sticker.gif", file_data: "YWJj" });
  assert.deepEqual(plain(calls[3].body), { media: { file_info: "official-file-info" }, msg_type: 7, msg_id: "source", msg_seq: 2 });
  assert.deepEqual(plain(calls[4].body), { file_type: 1, srv_send_msg: false, url: "https://example.com/image.png" });
  assert.equal(calls[4].url, "https://api.sgroup.qq.com/v2/groups/group/files");
});

test("QQ image upload failures preserve the error and never submit a media message", async () => {
  const calls = [];
  const { QQOpenApi } = load("openapi", clock());
  const api = new QQOpenApi({ appId: "app", appSecret: "secret", useSandbox: false }, {
    async request(request) { calls.push(request); return request.url.endsWith("getAppAccessToken") ? { statusCode: 200, content: JSON.stringify({ access_token: "token", expires_in: "7200" }) } : { statusCode: 403, content: "upload permission denied" }; },
  });
  await assert.rejects(api.sendImage("c2c", "user", "https://example.com/image.png", "source"), /403: upload permission denied/);
  assert.equal(calls.length, 2);
});

test("all four media kinds upload and send through their own scene with passive reply sequencing", async () => {
  const calls = [], reads = [];
  const { QQOpenApi } = load("openapi", { ...clock(), Tools: { Files: { async readBinary(path) { reads.push(path); return { size: 3, contentBase64: "YWJj" }; } } } });
  const api = new QQOpenApi({ appId: "app", appSecret: "secret", useSandbox: false }, { async request(request) {
    calls.push(request);
    const response = request.url.endsWith("getAppAccessToken") ? { access_token: "token", expires_in: "7200" } : request.url.endsWith("/files") ? { file_info: "media-info", ttl: 300 } : { id: "sent" };
    return { statusCode: 200, content: JSON.stringify(response) };
  } });
  for (const kind of ["image", "video", "audio", "file"]) await api.sendMedia("group", "group/a", { kind, source: `https://cdn.example/${kind}`, fileName: `${kind}.bin` }, { msg_id: "source", content: "[[reply_to:local-id]]说明", message_reference: { message_id: "REFIDX_own==" } });
  const uploaded = calls.filter(call => call.url.endsWith("/files")), sent = calls.filter(call => call.url.endsWith("/messages"));
  assert.deepEqual(uploaded.map(call => call.body.file_type), [1, 2, 3, 4]);
  assert.ok(uploaded.every(call => call.body.srv_send_msg === false && call.url.endsWith("/groups/group%2Fa/files")));
  assert.deepEqual(sent.map(call => call.body.msg_seq), [1, 2, 3, 4]);
  assert.ok(sent.every(call => call.body.msg_type === 7 && call.body.media.file_info === "media-info" && call.body.content === "说明" && call.body.message_reference.message_id === "REFIDX_own=="));
  await api.uploadMedia("c2c", "user", { kind: "audio", source: "file:///host/%E8%AF%AD%E9%9F%B3.wav" });
  assert.deepEqual(reads, ["/host/语音.wav"]);
  assert.equal(calls.at(-1).body.file_name, "语音.wav");
});

test("Markdown, keyboard, native reference and card fields infer their types and only sanitize visible strings", async () => {
  const { api, calls } = fixture();
  const keyboard = { content: { rows: [{ buttons: [{ id: "opaque-id", render_data: { label: "[[reply_to:id]]打开" }, action: { data: "[[reply_to:opaque-action]]" } }] }] } };
  await api.send("group", "group", { markdown: { content: "[[reply_to:local-id]]**你好**" }, keyboard, message_reference: { message_id: "REFIDX_native==" }, msg_id: "source" });
  assert.equal(calls[1].body.msg_type, 2); assert.equal(calls[1].body.markdown.content, "**你好**");
  const button = calls[1].body.keyboard.content.rows[0].buttons[0];
  assert.equal(button.render_data.label, "打开"); assert.equal(button.action.data, "[[reply_to:opaque-action]]");
  assert.equal(calls[1].body.message_reference.message_id, "REFIDX_native==");
  assert.equal(keyboard.content.rows[0].buttons[0].render_data.label, "[[reply_to:id]]打开");
  await api.send("group", "group", { card: { type: "tuwen", content: { title: "标题", description: "[CQ:reply,id=private]介绍", url: "https://example.com" } } });
  assert.equal(calls[2].body.msg_type, 8); assert.equal(calls[2].body.card.content.description, "介绍");
  await api.send("group", "group", { content: "[reply_id:private]正文", payload: { message_reference: { message_id: "REFIDX_payload==" } } });
  assert.equal(calls[3].body.content, "正文"); assert.equal(calls[3].body.message_reference.message_id, "REFIDX_payload==");
});

test("invalid media, quote, rich fields and oversized direct uploads fail without sending a message", async () => {
  const calls = [];
  const { QQOpenApi } = load("openapi", { ...clock(), Tools: { Files: { async readBinary() { return { size: 5 * 1024 * 1024 + 1, contentBase64: "bytes" }; } } } });
  const api = new QQOpenApi({ appId: "app", appSecret: "secret", useSandbox: false }, { async request(request) { calls.push(request); throw new Error("Must not reach HTTP"); } });
  await assert.rejects(api.send("c2c", "user", { content: "[[reply_to:private]]" }), /content/);
  await assert.rejects(api.send("c2c", "user", { content: "hello", message_reference: { message_id: "" } }), /message_reference/);
  await assert.rejects(api.send("c2c", "user", { markdown: { content: "text" }, content: "conflict" }), /Markdown messages/);
  await assert.rejects(api.send("c2c", "user", { msg_type: 7 }), /media/);
  await assert.rejects(api.sendMedia("c2c", "user", { kind: "file", source: "https://cdn.example/a" }, { msg_seq: -1 }), /msg_seq/);
  await assert.rejects(api.uploadMedia("c2c", "user", { kind: "invalid", source: "https://cdn.example/a" }), /media kind/);
  await assert.rejects(api.uploadMedia("c2c", "user", { kind: "video", source: "/host/large.mp4" }), /5 MiB/);
  await assert.rejects(api.uploadMedia("c2c", "user", { kind: "file", source: "ftp://example.com/a" }), /HTTP/);
  assert.equal(calls.length, 0);
});

test("event replies have independent sequences when their originating event IDs differ", async () => {
  const { api, calls } = fixture();
  for (const event_id of ["event-one", "event-two", "event-one"]) await api.send("group", "group", { content: "text", event_id });
  assert.deepEqual(calls.slice(1).map(call => call.body.msg_seq), [1, 1, 2]);
});

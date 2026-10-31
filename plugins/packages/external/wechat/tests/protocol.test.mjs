import assert from "node:assert/strict";
import test from "node:test";
import { binaryCrypto, load, plain, response } from "./harness.mjs";

/** Creates an API client with recorded requests and explicit server responses. */
function apiFixture(answer = { ret: 0 }) {
  const requests = [];
  const { WeChatApi } = load("api", { CryptoJS: binaryCrypto() });
  const api = new WeChatApi(
    {
      async request(request) {
        requests.push(request);
        return response(answer);
      },
    },
    {
      botId: "bot",
      userId: "owner",
      token: "private-token",
      baseUrl: "https://ilinkai.weixin.qq.com",
    },
  );
  return { api, requests };
}

test("QR acquisition uses the exact POST schema without bearer or base_info", async () => {
  const f = apiFixture({ qrcode: "qr", qrcode_img_content: "content" });
  await f.api.loginStart();
  const request = f.requests[0];
  assert.equal(request.method, "POST");
  assert.equal(
    request.url,
    "https://ilinkai.weixin.qq.com/ilink/bot/get_bot_qrcode?bot_type=3",
  );
  assert.deepEqual(plain(request.body), { local_token_list: [] });
  assert.equal(request.headers.Authorization, undefined);
  assert.equal(request.headers.AuthorizationType, "ilink_bot_token");
  const uin = Buffer.from(request.headers["X-WECHAT-UIN"], "base64").toString();
  assert.match(uin, /^\d+$/);
});

test("QR status uses application headers only and correctly encodes the verification code", async () => {
  const f = apiFixture({ status: "wait" });
  await f.api.loginPoll("https://ilinkai.weixin.qq.com", "a&b", "12+3");
  const request = f.requests[0];
  assert.equal(
    request.url,
    "https://ilinkai.weixin.qq.com/ilink/bot/get_qrcode_status?qrcode=a%26b&verify_code=12%2B3",
  );
  assert.deepEqual(plain(request.headers), {
    "iLink-App-Id": "bot",
    "iLink-App-ClientVersion": "65536",
  });
});

test("getUpdates carries the exact persistent cursor, authentication and attribution", async () => {
  const f = apiFixture();
  await f.api.updates("cursor", 60);
  const request = f.requests[0];
  assert.equal(request.read_timeout, 60);
  assert.equal(request.headers.Authorization, "Bearer private-token");
  assert.deepEqual(plain(request.body), {
    get_updates_buf: "cursor",
    base_info: { channel_version: "1.0.0", bot_agent: "Operit/1.0.0" },
  });
});

test("text replies carry their incoming context and unique client identities", async () => {
  const f = apiFixture();
  await f.api.sendText("peer", "context", "第一条");
  await f.api.sendText("peer", "context", "第二条");
  const first = f.requests[0].body.msg;
  assert.equal(first.to_user_id, "peer");
  assert.equal(first.context_token, "context");
  assert.equal(first.message_type, 2);
  assert.equal(first.message_state, 2);
  assert.equal(first.item_list[0].text_item.text, "第一条");
  assert.notEqual(first.client_id, f.requests[1].body.msg.client_id);
  assert.throws(() => f.api.sendText("peer", "", "hello"), /context_token/);
  assert.throws(() => f.api.sendText("peer", "ctx", "x".repeat(4001)), /4000/);
});

test("business and HTTP failures remain explicit and expired sessions retain their code", async () => {
  const f = apiFixture({ ret: -14, errmsg: "expired" });
  await assert.rejects(f.api.updates("", 45), (error) => error.code === -14);
  const g = apiFixture({ errcode: 401, errmsg: "denied" });
  await assert.rejects(g.api.sendText("peer", "context", "hello"), /401/);
});

test("a malformed send response cannot masquerade as acknowledged delivery", async () => {
  const f = apiFixture({});
  await assert.rejects(f.api.sendText("peer", "context", "hello"), /成功回执/);
});

test("int64 message identifiers survive JSON parsing without rounding or altering quoted text", () => {
  const model = load("model");
  const parsed = model.parseJson(
    '{"message_id":9223372036854775807,"seq":9007199254740993,"text":"12345 \\\"9223372036854775807\\\"","small":3}',
  );
  assert.equal(parsed.message_id, "9223372036854775807");
  assert.equal(parsed.seq, "9007199254740993");
  assert.equal(parsed.small, 3);
  assert.match(parsed.text, /9223372036854775807/);
});

test("credential-bearing endpoints reject arbitrary hosts, userinfo, ports and URL fragments", async () => {
  const { wechatUrl } = load("model");
  for (const value of [
    "https://evil.test",
    "https://weixin.qq.com.evil.test",
    "http://ilinkai.weixin.qq.com",
    "https://user@ilinkai.weixin.qq.com",
    "https://ilinkai.weixin.qq.com:8443",
    "https://ilinkai.weixin.qq.com/#fragment",
  ]) {
    assert.throws(() => wechatUrl(value, "url"));
  }
  assert.equal(
    wechatUrl("https://novac2c.cdn.weixin.qq.com/c2c", "url"),
    "https://novac2c.cdn.weixin.qq.com/c2c",
  );
});

test("typing obtains a real server ticket and sends exact state values", async () => {
  const f = apiFixture({ ret: 0, typing_ticket: "ticket" });
  await f.api.typing("peer", "ctx", true);
  assert.equal(f.requests[0].body.context_token, "ctx");
  assert.equal(f.requests[1].body.status, 1);
  assert.equal(f.requests[1].body.typing_ticket, "ticket");
});

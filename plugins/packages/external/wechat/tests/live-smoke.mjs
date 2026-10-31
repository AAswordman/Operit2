import assert from "node:assert/strict";
import { binaryCrypto, load } from "./harness.mjs";

const { WeChatApi } = load("api", { CryptoJS: binaryCrypto() });
const api = new WeChatApi({
  /** Uses the actual HTTPS service without logging QR contents or creating an authenticated account. */
  async request(options) {
    const result = await fetch(options.url, {
      method: options.method,
      headers: options.headers,
      body:
        options.body === undefined ? undefined : JSON.stringify(options.body),
      redirect: "error",
      signal: AbortSignal.timeout(20000),
    });
    return {
      statusCode: result.status,
      headers: Object.fromEntries(result.headers),
      content: await result.text(),
    };
  },
});

const result = await api.loginStart();
assert.equal(result.ret, 0);
assert.equal(typeof result.qrcode, "string");
assert.ok(result.qrcode.length > 0);
assert.equal(typeof result.qrcode_img_content, "string");
assert.ok(result.qrcode_img_content.length > 0);
console.log(
  "PASS official WeChat QR endpoint, current request headers and response schema",
);

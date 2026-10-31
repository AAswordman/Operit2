import assert from "node:assert/strict";
import test from "node:test";
import { createDecipheriv } from "node:crypto";
import {
  binaryCrypto,
  fixture,
  load,
  message,
  plain,
  response,
} from "./harness.mjs";

const PNG = Buffer.from(
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Wl6ZT0AAAAASUVORK5CYII=",
  "base64",
);
const KEY = "000102030405060708090a0b0c0d0e0f";

/** Binds actual media code to real cryptography and a bounded in-memory filesystem. */
function mediaFixture(options = {}) {
  const crypto = binaryCrypto();
  const files = new Map([
    ["/photo.png", PNG],
    ["/file.bin", Buffer.from([0, 255, 128, 3])],
  ]);
  const calls = [],
    uploads = [];
  const globals = {
    CryptoJS: crypto,
    getPluginConfigDir: () => "/plugins/wechat",
    Tools: {
      Files: {
        async info(path) {
          return { size: files.get(path).length };
        },
        async readBinary(path) {
          return { contentBase64: files.get(path).toString("base64") };
        },
        async mkdir() {
          return { successful: true };
        },
        async writeBinary(path, data) {
          files.set(path, Buffer.from(data, "base64"));
          return { successful: true };
        },
      },
    },
  };
  const media = load("media", globals);
  const api = {
    async post(path, body) {
      calls.push({ path, body });
      return options.uploadAnswer ?? { upload_param: "upload&a" };
    },
    http: {
      async request(request) {
        calls.push(request);
        return response(
          {},
          {
            contentBase64:
              options.downloadData ??
              crypto.binary.encryptAesEcb(PNG.toString("base64"), KEY),
          },
        );
      },
      async upload(url, data) {
        uploads.push({ url, data });
        return response(
          {},
          {
            headers: options.uploadHeaders ?? {
              "X-Encrypted-Param": "download-param",
            },
            statusCode: options.uploadStatus ?? 200,
          },
        );
      },
    },
  };
  return { ...media, api, calls, uploads, files, crypto };
}

test("image upload hashes actual bytes and encrypts a raw octet-stream before sendMessage", async () => {
  const f = mediaFixture();
  const item = await f.uploadMedia(f.api, "peer", "image", "/photo.png");
  const request = f.calls[0].body;
  assert.equal(request.rawsize, PNG.length);
  assert.equal(request.media_type, 1);
  assert.equal(request.no_need_thumb, true);
  assert.equal(request.filesize % 16, 0);
  assert.match(f.uploads[0].url, /encrypted_query_param=upload%26a&filekey=/);
  assert.equal(
    Buffer.from(item.image_item.media.aes_key, "base64").toString(),
    request.aeskey,
  );
  const decipher = createDecipheriv(
    "aes-128-ecb",
    Buffer.from(request.aeskey, "hex"),
    null,
  );
  const plainBytes = Buffer.concat([
    decipher.update(Buffer.from(f.uploads[0].data, "base64")),
    decipher.final(),
  ]);
  assert.deepEqual(plainBytes, PNG);
});

test("outbound files, videos and voice retain exact metadata and validate voice formats", async () => {
  const f = mediaFixture();
  const file = await f.uploadMedia(f.api, "peer", "file", "/file.bin", {
    file_name: "附件.bin",
  });
  assert.equal(file.file_item.file_name, "附件.bin");
  assert.equal(file.file_item.len, "4");
  assert.equal(
    (await f.uploadMedia(f.api, "peer", "video", "/file.bin")).type,
    5,
  );
  await assert.rejects(
    f.uploadMedia(f.api, "peer", "voice", "/file.bin"),
    /voice_format/,
  );
  const voice = await f.uploadMedia(f.api, "peer", "voice", "/file.bin", {
    voice_format: 7,
    sample_rate: 44100,
    duration_ms: 1200,
  });
  assert.equal(voice.voice_item.encode_type, 7);
  assert.equal(voice.voice_item.playtime, 1200);
  assert.equal(voice.voice_item.sample_rate, 44100);
});

test("media CDN rejects an arbitrary upload host, HTTP failure and missing encrypted-param header", async () => {
  const a = mediaFixture({
    uploadAnswer: { upload_full_url: "https://evil.test/upload" },
  });
  await assert.rejects(
    a.uploadMedia(a.api, "peer", "image", "/photo.png"),
    /WeChat HTTPS/,
  );
  assert.equal(a.uploads.length, 0);
  const b = mediaFixture({ uploadStatus: 403 });
  await assert.rejects(
    b.uploadMedia(b.api, "peer", "image", "/photo.png"),
    /403/,
  );
  assert.equal(b.uploads.length, 1);
  const c = mediaFixture({ uploadHeaders: {} });
  await assert.rejects(
    c.uploadMedia(c.api, "peer", "image", "/photo.png"),
    /x-encrypted-param/,
  );
});

test("both documented base64 key encodings and image hex keys decrypt actual binary media", async () => {
  const f = mediaFixture();
  for (const aes_key of [
    Buffer.from(KEY, "hex").toString("base64"),
    Buffer.from(KEY).toString("base64"),
  ]) {
    const file = await f.downloadMedia(
      f.api,
      {
        type: 2,
        image_item: { media: { encrypt_query_param: "query", aes_key } },
      },
      "../../external",
    );
    assert.deepEqual(f.files.get(file.filePath), PNG);
    assert.equal(file.mimeType, "image/png");
    assert.match(
      file.filePath,
      /^\/plugins\/wechat\/incoming\/\.\.%2F\.\.%2Fexternal-/,
    );
    assert.equal(file.fileSize, PNG.length);
  }
  const file = await f.downloadMedia(
    f.api,
    {
      type: 2,
      image_item: { aeskey: KEY, media: { encrypt_query_param: "query" } },
    },
    "hex",
  );
  assert.deepEqual(f.files.get(file.filePath), PNG);
});

test("bad keys and corrupt padding never produce plaintext attachment files", async () => {
  const f = mediaFixture();
  await assert.rejects(
    f.downloadMedia(
      f.api,
      {
        type: 2,
        image_item: {
          media: { full_url: "https://evil.test/photo", aes_key: "invalid" },
        },
      },
      "bad",
    ),
    /WeChat HTTPS/,
  );
  await assert.rejects(
    f.downloadMedia(
      f.api,
      {
        type: 2,
        image_item: {
          media: { encrypt_query_param: "query", aes_key: "invalid" },
        },
      },
      "bad",
    ),
    /base64/,
  );
  assert.equal(f.files.size, 2);
});

test("a transcribed voice item without media does not pretend to have a downloadable attachment", () => {
  const f = mediaFixture();
  assert.equal(
    f.mediaReference({ type: 3, voice_item: { text: "转写文本" } }),
    null,
  );
});

test("manual media tool uses the host binary body type and exact incoming context", async () => {
  const f = await fixture();
  f.config.contexts.peer = "ctx";
  f.files.set("/photo.png", PNG);
  await f.call({
    action: "send_media",
    params: {
      user_id: "peer",
      source: "/photo.png",
      kind: "image",
      text: "说明文字",
    },
  });
  assert.equal(f.uploads[0].bodyType, "base64");
  assert.equal(f.uploads[0]["Content-Type"], "application/octet-stream");
  assert.equal(f.sends().length, 2);
  assert.equal(f.sends()[0].body.msg.context_token, "ctx");
  assert.equal(f.sends()[0].body.msg.item_list[0].type, 2);
  assert.equal(f.sends()[1].body.msg.item_list[0].text_item.text, "说明文字");
});

test("automatic replies attach decrypted media with an actual MIME type to native Chat", async () => {
  const encrypted = binaryCrypto().binary.encryptAesEcb(
    PNG.toString("base64"),
    KEY,
  );
  const f = await fixture({ auto: true, downloadData: encrypted });
  const event = message(1);
  event.item_list.push({
    type: 2,
    image_item: { aeskey: KEY, media: { encrypt_query_param: "query" } },
  });
  await f.call({ action: "start" });
  await f.batch([event]);
  await f.time.advance(400);
  const attachment = f.submitted[0].input.attachments[0];
  assert.equal(attachment.mimeType, "image/png");
  assert.deepEqual(f.files.get(attachment.filePath), PNG);
  await f.stop();
});

test("Waifu emotion images are actually encrypted, uploaded and sent between their text segments", async () => {
  const f = await fixture({
    auto: true,
    emoticons: true,
    defaultFile: PNG,
    replyText: "你好。<emotion>happy</emotion>今天过得怎么样？",
  });
  await f.call({ action: "start" });
  await f.batch([message(1)]);
  await f.time.advance(1000);
  const items = f.sends().map((request) => request.body.msg.item_list[0]);
  assert.deepEqual(
    items.map((item) => item.type),
    [1, 2, 1],
  );
  assert.equal(items[0].text_item.text, "你好。");
  assert.equal(items[2].text_item.text, "今天过得怎么样？");
  assert.equal(f.uploads.length, 1);
  const upload = f.requests.find((request) =>
    request.url.endsWith("/getuploadurl"),
  );
  const decipher = createDecipheriv(
    "aes-128-ecb",
    Buffer.from(upload.body.aeskey, "hex"),
    null,
  );
  const bytes = Buffer.concat([
    decipher.update(Buffer.from(f.uploads[0].body, "base64")),
    decipher.final(),
  ]);
  assert.deepEqual(bytes, PNG);
  await f.stop();
});

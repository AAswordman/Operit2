import assert from "node:assert/strict";
import test from "node:test";
import { c2c, load, plain } from "./harness.mjs";

/** Uses the real event decoder with a declared QQ image attachment. */
function event() {
  const frame = c2c("../../external");
  frame.d.attachments = [{ filename: "photo.png", content_type: "image/png", size: 12, url: "https://cdn.example/photo" }];
  return load("model").inbound(frame);
}

test("QQ images use Host downloads and preserve original Chat attachment metadata", async () => {
  const calls = [];
  const { attachments } = load("attachments", { getPluginConfigDir() { return "/operitapp/data/plugins/qqbot"; }, Tools: { Files: {
    async mkdir(path, parents) { calls.push({ action: "mkdir", path, parents }); return { successful: true }; },
    async download(url, path) { calls.push({ action: "download", url, path }); return { successful: true }; },
    async info() { return { size: 15 }; },
  } } });
  const { files, oversized } = await attachments(event());
  assert.deepEqual(plain(oversized), []);
  assert.equal(calls[0].path, "/operitapp/data/plugins/qqbot/incoming/message-..%2F..%2Fexternal");
  assert.equal(calls[1].url, "https://cdn.example/photo");
  assert.deepEqual(plain(files), [{ filePath: calls[1].path, fileName: "photo.png", mimeType: "image/png", fileSize: 15, content: calls[1].path }]);
});

test("a failed Host download propagates its actual error", async () => {
  const { attachments } = load("attachments", { getPluginConfigDir() { return "/operitapp/data/plugins/qqbot"; }, Tools: { Files: {
    async mkdir() { return { successful: true }; },
    async download() { return { successful: false, details: "download denied" }; },
    async info() { throw new Error("Failed downloads must never be sent to Chat"); },
  } } });
  await assert.rejects(attachments(event()), /download denied/);
});

test("messages without attachments perform no filesystem calls", async () => {
  const { attachments } = load("attachments");
  assert.deepEqual(plain(await attachments(load("model").inbound(c2c()))), { files: [], oversized: [] });
});

test("nested voice quotes prefer WAV downloads and expose a standard MIME type to Chat", async () => {
  const frame = c2c(); frame.d.msg_elements = [{ attachments: [{ url: "https://cdn.example/silk", content_type: "voice", filename: "recording.silk", voice_wav_url: "https://cdn.example/wav", asr_refer_text: "语音" }] }];
  const calls = [];
  const { attachments } = load("attachments", { getPluginConfigDir: () => "/plugin", Tools: { Files: {
    async mkdir() { return { successful: true }; }, async download(url) { calls.push(url); return { successful: true }; }, async info() { return { size: 42 }; },
  } } });
  const { files: result } = await attachments(load("model").inbound(frame));
  assert.deepEqual(calls, ["https://cdn.example/wav"]);
  assert.equal(result[0].mimeType, "audio/wav"); assert.equal(result[0].fileName, "recording.wav");
});


test("skips oversized files and accepts an attachment at the limit", async () => {
  const downloads = [], deletions = [];
  const { attachments, CHAT_ATTACHMENT_LIMIT } = load("attachments", {
    getPluginConfigDir: () => "/plugin",
    Tools: { Files: {
      async mkdir() { return { successful: true }; },
      async download(url) { downloads.push(url); return { successful: true }; },
      async info(path) { return { size: CHAT_ATTACHMENT_LIMIT + (path.endsWith("/1") ? 1 : 0) }; },
      async deleteFile(path, recursive) { deletions.push({ path, recursive }); return { successful: true }; },
    } },
  });
  const frame = c2c();
  frame.d.attachments = [
    { filename: "declared.zip", content_type: "file", size: CHAT_ATTACHMENT_LIMIT + 1, url: "https://cdn.example/declared" },
    { filename: "downloaded.zip", content_type: "file", url: "https://cdn.example/downloaded" },
    { filename: "supported.pdf", content_type: "application/pdf", size: CHAT_ATTACHMENT_LIMIT, url: "https://cdn.example/supported" },
  ];
  const { files, oversized } = await attachments(load("model").inbound(frame));
  assert.deepEqual(downloads, ["https://cdn.example/downloaded", "https://cdn.example/supported"]);
  assert.deepEqual(deletions, [{ path: "/plugin/incoming/message-message-1/1", recursive: false }]);
  assert.deepEqual(plain(oversized), [
    { fileName: "declared.zip", sizeBytes: CHAT_ATTACHMENT_LIMIT + 1 },
    { fileName: "downloaded.zip", sizeBytes: CHAT_ATTACHMENT_LIMIT + 1 },
  ]);
  assert.equal(files.length, 1);
  assert.equal(files[0].fileName, "supported.pdf");
  assert.equal(files[0].fileSize, CHAT_ATTACHMENT_LIMIT);
});

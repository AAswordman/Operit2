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
  const { attachments } = load("attachments", { getPluginConfigDir() { return "/app/data/plugins/qqbot"; }, Tools: { Files: {
    async mkdir(path, parents) { calls.push({ action: "mkdir", path, parents }); return { successful: true }; },
    async download(url, path) { calls.push({ action: "download", url, path }); return { successful: true }; },
    async info() { return { size: 15 }; },
  } } });
  const files = await attachments(event());
  assert.equal(calls[0].path, "/app/data/plugins/qqbot/incoming/message-..%2F..%2Fexternal");
  assert.equal(calls[1].url, "https://cdn.example/photo");
  assert.deepEqual(plain(files), [{ filePath: calls[1].path, fileName: "photo.png", mimeType: "image/png", fileSize: 15, content: calls[1].path }]);
});

test("a failed Host download propagates its actual error", async () => {
  const { attachments } = load("attachments", { getPluginConfigDir() { return "/app/data/plugins/qqbot"; }, Tools: { Files: {
    async mkdir() { return { successful: true }; },
    async download() { return { successful: false, details: "download denied" }; },
    async info() { throw new Error("Failed downloads must never be sent to Chat"); },
  } } });
  await assert.rejects(attachments(event()), /download denied/);
});

test("messages without attachments perform no filesystem calls", async () => {
  const { attachments } = load("attachments");
  assert.deepEqual(plain(await attachments(load("model").inbound(c2c()))), []);
});

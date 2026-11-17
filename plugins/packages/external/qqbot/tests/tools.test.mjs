import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { load, plain } from "./harness.mjs";

test("declared rich-message tools route parsed JSON and media attributes through the owner runtime", async () => {
  const calls = [];
  const tools = load("tools", { ToolPkg: { ipc: { async call(...args) { calls.push(plain(args)); return { success: true }; } } } });
  const source = readFileSync(new URL("../src/tools.ts", import.meta.url), "utf8");
  const metadata = JSON.parse(source.slice(source.indexOf("{"), source.indexOf("\n*/")));
  for (const name of ["qqbot_upload_media", "qqbot_send_media"]) {
    assert.equal(typeof tools[name], "function"); assert.ok(metadata.tools.some(tool => tool.name === name));
  }
  await tools.qqbot_send_group_message({ group_openid: "group", markdown: '{"content":"**文本**"}', keyboard: '{"id":"keyboard"}', message_reference: '{"message_id":"REFIDX_own=="}', msg_id: "source" });
  assert.deepEqual(calls[0], ["qqbot.service", { action: "send_group", params: { group_openid: "group", msg_id: "source", markdown: { content: "**文本**" }, keyboard: { id: "keyboard" }, message_reference: { message_id: "REFIDX_own==" } } }, { targetRuntime: "main" }]);
  await tools.qqbot_send_media({ scene: "c2c", target: "user", kind: "audio", source: "/host/audio.wav", file_name: "语音.wav", content: "说明", msg_id: "source" });
  assert.deepEqual(calls[1][1], { action: "send_media", scene: "c2c", target: "user", media: { kind: "audio", source: "/host/audio.wav", fileName: "语音.wav" }, params: { content: "说明", msg_id: "source" } });
  await tools.qqbot_upload_media({ scene: "group", target: "group", kind: "file", source: "https://cdn.example/a.pdf" });
  assert.equal(calls[2][1].action, "upload_media");
  assert.throws(() => tools.qqbot_send_group_message({ group_openid: "group", markdown: "[]" }), /must be an object/);
  assert.equal(calls.length, 3);
});

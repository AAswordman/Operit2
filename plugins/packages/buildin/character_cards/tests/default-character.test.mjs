import assert from "node:assert/strict";
import test from "node:test";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { loadModule, plain } from "./runtime.mjs";
const state = loadModule("src/storage/state.ts");

test("built-in Operit retains original default content and exact packaged avatar", async () => {
  const card = state.createDefaultCharacter(123);
  assert.equal(card.id, "default"); assert.equal(card.name, "Operit");
  assert.equal(card.description, "系统默认的角色卡配置");
  assert.equal(card.characterSetting, "你是Operit，一个全能AI助手，旨在解决用户提出的任何任务。");
  assert.equal(card.otherContentChat, "保持有帮助的语气，并清楚地传达限制。");
  assert.match(card.otherContentVoice, /^1\. 身份锚定\n/);
  assert.match(card.otherContentVoice, /一次性输出大段独白（超过三句必须停顿或交互）$/);
  assert.equal(card.openingStatement, ""); assert.equal(card.avatarUri, null);
  assert.deepEqual(plain(card.attachedTagIds), []);
  const resource = await readFile(new URL("../resources/operit-avatar.png", import.meta.url));
  assert.equal(createHash("sha256").update(resource).digest("hex"), "29d737505e8322e75fdda1bf24b15377208019dd5a7dd60623cecc0b0056fda9");
});

test("initial state uses the current default without an unreleased JSON upgrade codec", () => {
  const initial = state.createInitialState(123);
  state.assertCharacterState(initial);
  assert.equal(initial.cards[0].name, "Operit");
  assert.deepEqual(plain(initial.active), { CharacterCard: { id: "default" } });
  assert.equal(initial.owners[0].ownerKey, "character:default");
  assert.equal(state.decodeCharacterState, undefined);
});

test("VFS avatar bytes become portable sources and failed reads can retry", async () => {
  let reads = 0;
  const source = loadModule("src/image-source.ts", { Tools: { Files: { async readBinary(path) {
    reads++; if (reads === 1) throw new Error("resource temporarily unavailable");
    const image = await readFile(new URL("../resources/operit-avatar.png", import.meta.url));
    return { path, size: image.length, contentBase64: image.toString("base64") };
  } } } });
  await assert.rejects(source.imageSource("/app/data/avatar.png"), /temporarily unavailable/);
  const image = await source.imageSource("/app/data/avatar.png"); assert.ok(image.startsWith("data:image/png;base64,"));
  assert.equal(await source.imageSource("/app/data/avatar.png"), image); assert.equal(reads, 2);
  assert.equal(source.pickedImagePath("/Users/test/avatar.png", "macos"), "/mnt/macos/Users/test/avatar.png");
  assert.equal(source.pickedImagePath("C:\\Users\\test\\avatar.png", "windows"), "/mnt/windows/c/Users/test/avatar.png");
  assert.throws(() => source.pickedImagePath("blob:fake", "web"), /文件路径/);
});

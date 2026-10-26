import assert from "node:assert/strict";
import test from "node:test";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { loadModule, plain } from "./runtime.mjs";
const state = loadModule("src/storage/state.ts");

test("the installation card retains the Operit template and exact packaged avatar", async () => {
  const card = state.createInstalledCharacter(123);
  assert.match(card.id, /^[0-9a-f-]{36}$/); assert.equal(card.name, "Operit");
  assert.equal(Object.hasOwn(card, "isDefault"), false);
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

test("initial state installs the template card as an ordinary card", () => {
  const initial = state.createInitialState(123);
  assert.equal(initial.cards[0].name, "Operit");
  assert.deepEqual(plain(initial.active), { CharacterCard: { id: initial.cards[0].id } });
  assert.equal(initial.owners[0].ownerKey, "character:" + initial.cards[0].id);
  assert.equal(state.decodeCharacterState, undefined);
});

test("avatar import validates picker bytes without accessing the source file or terminal", async () => {
  const source = loadModule("src/image-source.ts");
  const image = await readFile(new URL("../resources/operit-avatar.png", import.meta.url));
  const file = { size: image.length, contentBase64: image.toString("base64") };
  assert.equal(source.readAvatarImport(file), file.contentBase64);
  assert.equal(source.readAvatarImport(file), file.contentBase64);
  assert.throws(() => source.readAvatarImport({ size: 5 }), /没有返回头像图片内容/);
  assert.throws(() => source.readAvatarImport({ ...file, size: file.size + 1 }), /内容不完整/);
  assert.throws(() => source.readAvatarImport({ ...file, size: 8 * 1024 * 1024 + 1 }), /不能超过 8 MB/);
  assert.throws(() => source.readAvatarImport({ size: 4, contentBase64: "dGV4dA==" }), /请选择 PNG/);
});

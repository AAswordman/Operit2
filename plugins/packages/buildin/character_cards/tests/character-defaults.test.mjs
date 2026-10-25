import assert from "node:assert/strict";
import test from "node:test";
import { loadModule, plain } from "./runtime.mjs";

const { readCharacter } = loadModule("src/drafts.ts");
const { createCharacterDraft } = loadModule("src/drafts.ts");

/** Keeps user-authored fields while an older writer omits one field at a time. */
test("character validation completes every missing non-identity field from creation defaults", () => {
  const defaults = plain(createCharacterDraft());
  const original = { ...defaults, id: "existing-card", name: "原有角色", characterSetting: "保留完整设定", openingStatement: "原有开场白" };
  for (const field of Object.keys(defaults).filter(field => field !== "id")) {
    for (const missing of [undefined, null]) {
      let card = structuredClone(original);
      if (missing === undefined) delete card[field]; else card[field] = null;
      card = readCharacter(card);
      assert.deepEqual(plain(card), { ...original, [field]: defaults[field] }, field);
      const completed = plain(card); card = readCharacter(card);
      assert.deepEqual(plain(card), completed, "completion must be stable: " + field);
    }
  }
});

/** Recovers optional nested policy fields without broadening explicit access lists. */
test("character validation completes partial tool policies and shared memory access flags", () => {
  let card = { ...plain(createCharacterDraft()), id: "existing-card",
    toolAccessConfig: { enabled: true, allowedPackages: ["allowed-package"] },
    sharedMemoryMounts: [{ sharedMemoryId: "existing-library", readable: true }] };
  card = readCharacter(card);
  assert.deepEqual(plain(card.toolAccessConfig), { ...plain(createCharacterDraft()).toolAccessConfig, enabled: true, allowedPackages: ["allowed-package"] });
  assert.deepEqual(plain(card.sharedMemoryMounts), [{ sharedMemoryId: "existing-library", readable: true, writable: false }]);
});

/** Reproduces the Build 26 timestamp error and retains usable legacy numeric text. */
test("character validation recovers absent invalid and legacy text timestamps", () => {
  for (const createdAt of [undefined, null, -1, Number.MAX_SAFE_INTEGER + 1, NaN, Infinity, {}, "bad-time"]) {
    let card = { ...plain(createCharacterDraft()), id: "existing-card", createdAt, updatedAt: -1 };
    card = readCharacter(card); assert.equal(card.createdAt, 0); assert.equal(card.updatedAt, 0);
  }
  let card = { ...plain(createCharacterDraft()), id: "existing-card", createdAt: "1792886400000", updatedAt: "1792886400123" };
  card = readCharacter(card); assert.equal(card.createdAt, 1792886400000); assert.equal(card.updatedAt, 1792886400123);
});

/** Complete immutable native snapshots need no mutation and invalid identities are never fabricated. */
test("character validation preserves complete frozen snapshots and checks supplied structure", () => {
  const card = plain(createCharacterDraft());
  function freeze(value) { if (value && typeof value === "object") { Object.values(value).forEach(freeze); Object.freeze(value); } return value; }
  assert.deepEqual(plain(readCharacter(freeze(card))), plain(card));
  for (const value of [null, []]) assert.throws(() => readCharacter(value));
  assert.equal(readCharacter({ name: "existing" }, "stored-id").id, "stored-id");
});

import assert from "node:assert/strict";
import test from "node:test";
import path from "node:path";
import { readFile, writeFile } from "node:fs/promises";
import { createDiskHarness } from "./disk-files.mjs";
import { loadModule, plain } from "./runtime.mjs";

const drafts = loadModule("src/drafts.ts"), validation = loadModule("src/validation.ts"), schema = loadModule("src/storage/state.ts");
const codec = loadModule("src/serialization.ts"), backup = loadModule("src/backup.ts");

/** Creates a complete intentional new character with a single-direction independent configuration reference. */
function character(themeConfigId) { return { ...plain(drafts.createCharacterDraft()), name: "Independent theme role", themeConfigId }; }
/** Opens actual temporary plugin files without implementing a theme directory or a successful configuration apply. */
async function repository(disk) { return loadModule("src/storage/database.ts", disk.globals).DatabaseCharacterRepository.open(); }

/** Null explicitly leaves the separate configuration unchanged; a new draft must not select any theme. */
test("new plugin character draft explicitly contains a required nullable independent themeConfigId", () => {
  const card = character(null); assert.equal(Object.hasOwn(card, "themeConfigId"), true); assert.equal(card.themeConfigId, null);
  validation.assertCard(card); assert.equal(card.ttsConfigId, null);
});

/** Strict full-record validation rejects missing, blank and non-string references without editing input. */
test("complete character schema rejects absent or malformed themeConfigId without compatibility repair", () => {
  for (const value of [undefined, 7, {}, [], false, "", " "]) {
    const card = character(value); if (value === undefined) delete card.themeConfigId;
    const before = JSON.stringify(card); assert.throws(
      /** Runs the actual complete stored-record validator instead of weakening a test fixture schema. */
      () => validation.assertCard(card), /themeConfigId|角色主题配置/,
    );
    assert.equal(JSON.stringify(card), before);
  }
  for (const value of [null, "independent-theme-one"]) validation.assertCard(character(value));
});

/** Existing version-three records require the newly declared complete character field and are never migrated on read. */
test("version-three file schema rejects a character missing the required independent theme reference", () => {
  const current = plain(schema.createInitialState(10)); assert.equal(current.version, 3); assert.equal(current.cards[0].themeConfigId, null);
  schema.assertCharacterState(current); delete current.cards[0].themeConfigId;
  assert.throws(
    /** Uses the real whole-snapshot validator so absent references cannot be supplied by a reader. */
    () => schema.assertCharacterState(current), /themeConfigId/,
  );
});

/** Native and Tavern codecs preserve the exact independent ID, never a legacy role-prefixed theme target. */
test("native and Tavern character export import preserve themeConfigId and TTS references losslessly", () => {
  const card = { ...character("independent-theme-one"), id: "saved-role", ttsConfigId: "independent-voice-one", createdAt: 1, updatedAt: 2 };
  for (const format of ["operit", "tavern"]) {
    const encoded = codec.encodeCharacterExport(card, format, []), imported = codec.decodeCharacterImport(encoded, format, []);
    assert.equal(imported.card.themeConfigId, card.themeConfigId); assert.equal(imported.card.ttsConfigId, card.ttsConfigId);
    if (format === "tavern") assert.equal(JSON.parse(encoded).data.extensions.operit.character_card.themeConfigId, "independent-theme-one");
  }
});

/** Real file persistence retains the supplied reference; a corrupt stored record rejects without writing repaired bytes. */
test("actual plugin files preserve themeConfigId and reject old incomplete record bytes without mutation", async t => {
  const disk = await createDiskHarness(t), file = path.join(disk.directory, "character-memory", "state.json"), owner = await repository(disk);
  const saved = await owner.run(
    /** Persists the complete explicit reference in the real repository, without a simulated external configuration apply. */
    session => session.createCharacter(character("independent-theme-one")),
  );
  const reopened = await repository(disk), retained = await reopened.run(
    /** Reads the same actual file-owned role rather than a fixture projection. */
    session => session.getCharacter(saved.id),
  );
  assert.equal(retained.themeConfigId, "independent-theme-one");
  const stored = JSON.parse(await disk.stateBytes().toString("utf8")); delete stored.cards.find(card => card.id === saved.id).themeConfigId;
  const malformed = JSON.stringify(stored); disk.replaceState(stored); disk.clearCalls();
  await assert.rejects(repository(disk), /themeConfigId/); assert.equal(await disk.stateBytes().toString("utf8"), malformed);
  assert.equal(disk.calls.filter(
    /** Confirms failed reads cannot manufacture a new field or publish any implicit migration. */
    call => ["write", "move", "mkdir"].indexOf(call.method) >= 0,
  ).length, 0);
});

/** Character backups use the same complete strict records and preserve the single-direction independent reference. */
test("real character backup retains themeConfigId and rejects incomplete records instead of defaulting null", async t => {
  const disk = await createDiskHarness(t), owner = await repository(disk);
  const saved = await owner.run(
    /** Creates genuine file-owned state before exporting from the same serialized repository. */
    session => session.createCharacter(character("independent-theme-one")),
  );
  const encoded = await owner.run(
    /** Exports all complete records using the actual plugin backup implementation. */
    session => backup.exportCharacterBackup(session),
  );
  const decoded = backup.decodeCharacterBackup(encoded), selected = decoded.characterCards.filter(
    /** Requires the exact persisted identity without interpreting the theme configuration in plugin storage. */
    card => card.id === saved.id,
  );
  assert.equal(selected.length, 1); assert.equal(selected[0].themeConfigId, "independent-theme-one");
  const document = JSON.parse(encoded); delete document.characterCards[1].themeConfigId;
  assert.throws(
    /** Rejects a malformed backup before it can restore any record or change the independent theme. */
    () => backup.decodeCharacterBackup(JSON.stringify(document)), /themeConfigId/,
  );
});

/** Role groups own their theme reference instead of inferring either configuration from the first member. */
test("new role-group draft explicitly declares its own nullable themeConfigId without a group TTS field", () => {
  const group = plain(drafts.createGroupDraft()); assert.equal(group.themeConfigId, null); assert.equal(Object.hasOwn(group, "ttsConfigId"), false);
  validation.assertGroup(group);
  for (const value of [undefined, 1, {}, "", " "]) {
    const invalid = { ...group, themeConfigId: value }; if (value === undefined) delete invalid.themeConfigId;
    assert.throws(
      /** Requires the group record's own declared field; member roles cannot supply it. */
      () => validation.assertGroup(invalid), /group.themeConfigId|群组主题配置/,
    );
  }
});

/** Actual group files and backup bytes retain this group's independent reference without copying a member role's config. */
test("actual plugin group persistence and backup preserve its own themeConfigId", async t => {
  const disk = await createDiskHarness(t), owner = await repository(disk);
  const saved = await owner.run(
    /** Writes one real role-composition group referencing the intentional initial default card. */
    session => session.createGroup({ ...plain(drafts.createGroupDraft()), name: "Own independent group theme", themeConfigId: "independent-group-theme", members: [{ characterCardId: "default", orderIndex: 0 }] }),
  );
  const encoded = await owner.run(
    /** Exports the exact actual stored group records instead of a test-projected owner usage list. */
    session => backup.exportGroupBackup(session),
  );
  const decoded = backup.decodeGroupBackup(encoded); assert.equal(decoded.characterGroups.length, 1);
  assert.equal(decoded.characterGroups[0].themeConfigId, "independent-group-theme");
  const imported = codec.decodeGroupImport(JSON.stringify(decoded.characterGroups[0])); assert.equal(imported.themeConfigId, "independent-group-theme");
  const reopened = await repository(disk), retained = await reopened.run(
    /** Reads the real persisted identity from the reopened file repository. */
    session => session.getGroup(saved.id),
  );
  assert.equal(retained.themeConfigId, "independent-group-theme"); assert.equal(Object.hasOwn(retained, "ttsConfigId"), false);
  const invalid = JSON.parse(encoded); delete invalid.characterGroups[0].themeConfigId;
  assert.throws(
    /** Refuses incomplete group backups without deriving their reference from any participant. */
    () => backup.decodeGroupBackup(JSON.stringify(invalid)), /group.themeConfigId/,
  );
});

/** The original v3 complete state contract rejects a missing group reference before any implicit record repair. */
test("version-three strict state rejects groups missing their own required themeConfigId", () => {
  const state = plain(schema.createInitialState(10));
  state.groups.push({ ...plain(drafts.createGroupDraft()), id: "entity-1", name: "Explicit schema group", members: [{ characterCardId: "default", orderIndex: 0 }] });
  state.nextId = "2"; schema.assertCharacterState(state); delete state.groups[0].themeConfigId;
  assert.throws(
    /** Runs the full real schema rather than accepting the record because its members are valid. */
    () => schema.assertCharacterState(state), /group.themeConfigId/,
  );
});

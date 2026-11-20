import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";
import { createDiskHarness } from "./disk-files.mjs";
import { loadModule, plain, runtimeConsole } from "./runtime.mjs";

const image = await readFile(new URL("../resources/operit-avatar.png", import.meta.url));

/** Seeds a released configuration-directory avatar alongside its genuine saved character record. */
async function environment(testContext) {
  const disk = await createDiskHarness(testContext, { legacyIdentity: "avatar-role" });
  const sourceDirectory = disk.globals.ToolPkg.getConfigDir() + "/avatars";
  const source = sourceDirectory + "/1794970570027-source.image";
  const target = disk.globals.ToolPkg.getSpaceDataDir() + "/avatars/1794970570027-source.image";
  await disk.files.mkdir(sourceDirectory, true);
  await disk.files.writeBinary(source, image.toString("base64"));
  const state = disk.readState();
  state.cards[0].avatarUri = source;
  disk.replaceState(state);
  const warnings = [];
  const globals = { ...disk.globals, console: { ...runtimeConsole,
    /** Captures the explicit missing-file diagnostic without replacing the original avatar reference. */
    warn(message) { warnings.push(message); },
  } };
  const storage = await loadModule("src/storage/preferences.ts", globals).CharacterStorage.open();
  const migration = loadModule("src/storage/avatars.ts", globals).migrateCharacterAvatars;
  disk.clearCalls();
  return { disk, storage, source, target, state, warnings,
    /** Exercises the production migration directly over actual preference files and binary IO. */
    migrate() { return migration(storage); },
    /** Reopens the real repository to verify that startup mounts migration before exposing characters. */
    reopen() { return loadModule("src/storage/database.ts", globals).DatabaseCharacterRepository.open(); },
  };
}

/** Runs startup migration and verifies original bytes survive while the durable reference moves into Space. */
test("avatar storage: repository startup shares legacy bytes before changing the character reference", async testContext => {
  const env = await environment(testContext);
  const repository = await env.reopen();
  const saved = await repository.getCharacter("avatar-role");
  assert.equal(saved.avatarUri, env.target);
  assert.deepEqual(plain({ ...saved, avatarUri: env.source }), plain(env.state.cards[0]));
  assert.deepEqual(await readFile(env.source), image);
  assert.deepEqual(await readFile(env.target), image);
  assert.equal(env.disk.calls.filter(call => call.method === "copy").length, 1);
  env.disk.clearCalls();
  await env.reopen();
  assert.equal(env.disk.calls.filter(call => call.method === "copy" || call.method === "writeBinary").length, 0);
});

/** A missing original remains an explicit unresolved file while unrelated role operations stay available. */
test("avatar storage: missing originals keep their references and report the actual path", async testContext => {
  const env = await environment(testContext);
  await env.disk.files.deleteFile(env.source);
  env.disk.clearCalls();
  const repository = await env.reopen();
  assert.equal((await repository.getCharacter("avatar-role")).avatarUri, env.source);
  assert.equal(env.warnings.length, 1);
  assert.ok(env.warnings[0].endsWith(env.source));
  assert.equal((await env.disk.files.exists(env.target)).exists, false);
  assert.equal(env.disk.calls.filter(call => call.method === "copy" || call.method === "writeBinary").length, 0);
});

/** An interrupted binary copy cannot publish a reference to an absent shared image. */
test("avatar storage: copy failures preserve the saved reference and original error", async testContext => {
  const env = await environment(testContext), failure = new Error("Controlled avatar copy failure");
  env.disk.failNext("copy", failure);
  await assert.rejects(env.migrate, error => error === failure);
  assert.equal(env.disk.readState().cards[0].avatarUri, env.source);
  assert.deepEqual(await readFile(env.source), image);
  await env.migrate();
  assert.equal(env.disk.readState().cards[0].avatarUri, env.target);
});

/** A copied file survives a lost preference commit and is reused only after verifying its exact bytes. */
test("avatar storage: interrupted reference publication resumes without replacing the original image", async testContext => {
  const env = await environment(testContext), failure = new Error("Controlled avatar reference failure");
  env.disk.failNext("storage.preferences.edit", failure);
  await assert.rejects(env.migrate, error => error === failure);
  assert.equal(env.disk.readState().cards[0].avatarUri, env.source);
  assert.deepEqual(await readFile(env.target), image);
  env.disk.clearCalls();
  await env.migrate();
  assert.equal(env.disk.readState().cards[0].avatarUri, env.target);
  assert.equal(env.disk.calls.filter(call => call.method === "copy").length, 0);
  assert.deepEqual(await readFile(env.source), image);
});

/** An existing conflicting target is an error instead of being overwritten or rebound by filename. */
test("avatar storage: conflicting shared bytes preserve both files and the original reference", async testContext => {
  const env = await environment(testContext), conflict = Buffer.from("a different image");
  await env.disk.files.mkdir(env.disk.globals.ToolPkg.getSpaceDataDir() + "/avatars", true);
  await env.disk.files.writeBinary(env.target, conflict.toString("base64"));
  await assert.rejects(env.migrate, /Shared avatar conflicts/);
  assert.equal(env.disk.readState().cards[0].avatarUri, env.source);
  assert.deepEqual(await readFile(env.source), image);
  assert.deepEqual(await readFile(env.target), conflict);
});

/** Retains independently edited character fields when publishing only the migrated avatar. */
test("avatar storage: migration preserves concurrent character edits", async testContext => {
  const env = await environment(testContext), copy = env.disk.files.copy;
  env.disk.files.copy = async (...args) => {
    const result = await copy(...args);
    await env.storage.configuration.cards.edit(preferences => {
      const current = preferences.get("record_avatar-role");
      preferences.set("record_avatar-role", { ...current, name: "Concurrent name" });
    });
    return result;
  };
  await env.migrate();
  assert.equal(env.disk.readState().cards[0].name, "Concurrent name");
  assert.equal(env.disk.readState().cards[0].avatarUri, env.target);
});

/** A newly selected avatar wins over the stale source captured by an in-progress migration. */
test("avatar storage: migration rejects a concurrently changed avatar without overwriting it", async testContext => {
  const env = await environment(testContext), copy = env.disk.files.copy;
  env.disk.files.copy = async (...args) => {
    const result = await copy(...args);
    await env.storage.configuration.cards.edit(preferences => {
      const current = preferences.get("record_avatar-role");
      preferences.set("record_avatar-role", { ...current, avatarUri: "https://fixture.invalid/new-avatar.png" });
    });
    return result;
  };
  await assert.rejects(env.migrate, /avatar changed during migration/);
  assert.equal(env.disk.readState().cards[0].avatarUri, "https://fixture.invalid/new-avatar.png");
});

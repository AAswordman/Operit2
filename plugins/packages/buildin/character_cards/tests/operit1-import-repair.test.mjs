import assert from "node:assert/strict";
import test from "node:test";
import { createDiskHarness } from "./disk-files.mjs";
import { fixture } from "./fixtures.mjs";
import { loadModule, plain } from "./runtime.mjs";

/** Seeds the released Operit1 importer output without changing its source domain identities. */
async function imported(t, count = 1) {
  const disk = await createDiskHarness(t, { legacyIdentity: "operit" });
  const state = disk.readState();
  state.owners[0].memories = Array.from({ length: count }, (_, i) => ({
    ...plain(fixture().graph.items[0]), id: String(i + 1), uuid: "source-memory-" + i,
    isDocumentNode: true,
    documentPath: "content://com.android.providers.media.documents/document/document%3A122474",
    chunkIndexFilePath: "content://com.android.providers.media.documents/document/document%3A122499",
  }));
  state.owners[0].chunks = [{ id: "1", memoryUuid: "source-memory-0", chunkIndex: 0, content: "Exact document chunk" }];
  disk.replaceState(state);
  const expected = plain(disk.readState());
  disk.storage.inspectPreferences("meta", values => {
    values.__operit_preferences_schema_version = 1;
    Object.assign(values.state, { legacyImportPending: true, migrationStatus: "complete", migrationSource: "operit1:com.ai.assistance.operit:123" });
  });
  return { disk, state: expected };
}

/** Opens the production repository with a router that rejects source Android document schemes. */
async function open(disk, storage = disk.storage) {
  const files = new Proxy(disk.files, { get(target, key) {
    const value = target[key];
    if (typeof value !== "function") return value;
    return (...args) => {
      if (args.some(arg => typeof arg === "string" && arg.startsWith("content://")))
        throw new Error("'content' is not a member of the current device space; members: windows, android");
      return value(...args);
    };
  } });
  return loadModule("src/storage/database.ts", {
    ...disk.globals, Tools: { ...disk.globals.Tools, Files: files, Storage: storage },
  }).DatabaseCharacterRepository.open();
}

/** Reads only the affected import state independently of the repair implementation. */
function metadata(disk) { return disk.storage.inspectPreferences("meta", values => values.state); }

test("synced Operit1 imports open without routing document grants or changing memory records", async t => {
  const { disk, state } = await imported(t);
  disk.clearCalls();
  await open(disk);
  const saved = plain(disk.readState());
  assert.deepEqual(saved, state);
  assert.equal(metadata(disk).legacyImportPending, undefined);
  assert.equal(disk.calls.some(call => ["copy", "deleteFile", "write", "mkdir"].includes(call.method)), false);
  assert.equal(disk.storage.inspectPreferences("local/migration", values => values.plan), undefined);
  const before = disk.stateBytes();
  await open(disk);
  assert.deepEqual(disk.stateBytes(), before);
});

test("startup reads no memory bodies even when a large import has the retired conversion marker", async t => {
  const { disk, state } = await imported(t, 205);
  const storage = { ...disk.storage, objects: { async open(options) {
    const handle = await disk.storage.objects.open(options);
    handle.query = async () => { throw new Error("Startup must not query memory tables"); };
    const collection = handle.collection;
    handle.collection = name => {
      assert.equal(["memories", "links", "chunks", "candidates"].includes(name), false, "Startup must not enumerate imported collections");
      return collection(name);
    };
    handle.commit = async () => { throw new Error("Startup must not rewrite memory objects"); };
    return handle;
  } } };
  const repository = await open(disk, storage);
  assert.equal((await repository.listCharacters()).length, state.cards.length);
  assert.deepEqual(plain(disk.readState()), state);
  assert.equal(metadata(disk).legacyImportPending, undefined);
});

test("metadata publication failure resumes while retaining ordinary document pointers", async t => {
  const { disk, state } = await imported(t);
  let fail = true;
  const storage = { ...disk.storage, dataStore: { async open(options) {
    const handle = await disk.storage.dataStore.open(options);
    if (options.path.endsWith("/meta.preferences.json")) {
      const edit = handle.edit;
      handle.edit = callback => edit(async values => {
        await callback(values);
        if (fail && values.get("state").legacyImportPending !== true) {
          fail = false; throw new Error("Interrupted repair publication");
        }
      });
    }
    return handle;
  } } };
  await assert.rejects(open(disk, storage), /Interrupted repair publication/);
  assert.equal(metadata(disk).legacyImportPending, true);
  await open(disk);
  assert.deepEqual(plain(disk.readState()), state);
  assert.equal(metadata(disk).legacyImportPending, undefined);
});

test("ordinary plugin schema declaration does not schedule a legacy conversion", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "operit" });
  const before = plain(disk.readState());
  disk.storage.inspectPreferences("meta", values => { values.__operit_preferences_schema_version = 0; });
  await open(disk);
  assert.deepEqual(plain(disk.readState()), before);
  assert.equal(metadata(disk).legacyImportPending, undefined);
});

test("unfinished imports remain blocked before any repair changes are made", async t => {
  const { disk } = await imported(t);
  disk.storage.inspectPreferences("meta", values => { values.state.migrationStatus = "writing"; });
  const before = plain(metadata(disk));
  await assert.rejects(open(disk), /Operit1 import is still writing/);
  assert.deepEqual(plain(metadata(disk)), before);
});

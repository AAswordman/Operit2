import test from "node:test";
import assert from "node:assert/strict";
import { readFile, writeFile, cp, stat } from "node:fs/promises";
import path from "node:path";
import { DatabaseSync } from "node:sqlite";
import { createDiskHarness } from "./disk-files.mjs";
import { loadModule } from "./runtime.mjs";

/** Exercises production repository reads against records emitted from the supplied snapshot. */
test("real Operit1 snapshot opens in the character plugin", { skip: !process.env.OPERIT1_DIAGNOSTIC_PLUGIN }, async t => {
  const source = process.env.OPERIT1_DIAGNOSTIC_PLUGIN;
  const disk = await createDiskHarness(t);
  const handle = await disk.storage.objects.open({ path: disk.directory + "/memory.sqlite" });
  const original = new DatabaseSync(source + "/memory.sqlite", { readOnly: true });
  const rows = original.prepare("SELECT collection,key,value,version FROM _operit_values").all();
  original.close();
  disk.storage.inspect(sql => {
    const insert = sql.prepare("INSERT INTO records VALUES(?,?,?,?)");
    for (const row of rows) insert.run(row.collection, row.key, row.value, row.version);
  });
  await handle.close();
  let cardCount = 0, pendingImport = false;
  for (const name of ["cards", "groups", "tags", "stores", "conversationGroups", "meta", "chats"]) {
    const values = JSON.parse(await readFile(source + "/" + name + ".preferences.json", "utf8"));
    if (name === "cards") cardCount = Object.keys(values).filter(key => key.startsWith("record_")).length;
    if (name === "meta") pendingImport = values.state.legacyImportPending === true;
    await writeFile(disk.directory + "/" + name + ".preferences.json", JSON.stringify(Object.fromEntries(Object.entries(values).filter(([key]) => !key.startsWith("__operit_") || key === "__operit_preferences_schema_version"))));
  }
  await cp(source + "/owners", disk.directory + "/owners", { recursive: true });
  const files = new Proxy(disk.files, { get(target, name) {
    if (name !== "exists") return target[name];
    return async value => {
      // Raw source-app document grants reach the real device router before filesystem existence checks.
      if (value.startsWith("content://")) throw new Error("'content' is not a member of the current device space; members: windows, android");
      if (!value.startsWith("/operitapp/data/")) return target.exists(value);
      const absolute = path.resolve(source, "../../..", value.slice("/operitapp/data/".length));
      try { const info = await stat(absolute); return { exists: true, isDirectory: info.isDirectory(), size: info.size }; }
      catch (error) { if (error.code !== "ENOENT") throw error; return { exists: false }; }
    };
  } });
  const globals = { ...disk.globals, Tools: { ...disk.globals.Tools, Files: files, Chat: {
    /** No author rewrite is required by this isolated record-adoption diagnostic. */
    async listAll() { return { chats: [] }; },
  } } };
  const repository = await loadModule("src/storage/database.ts", globals).DatabaseCharacterRepository.open();
  assert.equal((await repository.listCharacters()).length, cardCount);
  const state = disk.readState();
  assert.equal(state.cards.length, cardCount);
  assert.equal(state.owners.length, rows.filter(row => row.collection === "owners").length);
  assert.equal(state.owners.reduce((sum, owner) => sum + owner.memories.length, 0), rows.filter(row => row.collection === "memories").length);
  for (const owner of state.owners) {
    assert.equal(owner.userDocumentPath, "owners/" + encodeURIComponent(owner.ownerKey) + "/USER.md");
    await repository.readUser(owner.ownerKey);
    await repository.readMemorySettings(owner.ownerKey);
    await repository.readMemorySearchConfig(owner.ownerKey);
    await repository.run(async session => {
      const jobs = await session.readMemoryJobState(owner.ownerKey);
      assert.equal(jobs.candidates.length, owner.candidates.length);
      assert.ok(jobs.candidates.every(candidate => typeof candidate.producerNodeId === "string" && candidate.producerNodeId.length > 0));
      assert.equal((await session.listMemories(owner.ownerKey)).length, owner.memories.length);
      const graph = await session.readMemoryGraph(owner.ownerKey);
      assert.equal(graph.nodes.length, owner.memories.length);
      assert.equal(graph.edges.length, owner.links.length);
    });
    for (const memory of owner.memories) for (const field of pendingImport ? [] : ["documentPath", "chunkIndexFilePath"]) {
      assert.equal(memory[field]?.startsWith("content://") === true, false);
    }
  }
});

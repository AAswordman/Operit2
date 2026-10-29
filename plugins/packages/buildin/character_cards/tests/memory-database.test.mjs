import assert from "node:assert/strict";
import test from "node:test";
import { createDiskHarness } from "./disk-files.mjs";
import { loadModule, plain } from "./runtime.mjs";

const ownerKey = "character:default";

/** Supplies complete authored values without depending on editor defaults. */
function values(title, content = title) {
  return { title, content, contentType: "text/plain", source: "test", credibility: 1, importance: 1, folderPath: null, tags: [] };
}

/** Uses explicit scope, filters, threshold and page size for database queries. */
function options(query = "travel", changes = {}) {
  return { ownerKey, query, folderPath: null, relevanceThreshold: 0, createdAtStartMs: null, createdAtEndMs: null, limit: 1, ...changes };
}

/** Opens production TypeScript on real SQLite and controls only the HTTP boundary. */
async function environment(t) {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }), requests = [];
  let provider = async texts => texts.map(text => /travel|trip|north/u.test(text) ? [2, 0] : [0, 2]);
  disk.globals.Tools.Net = {
    /** Returns indexed vectors in reverse response order to exercise batch identity validation. */
    async http(request) {
      const payload = JSON.parse(request.body); requests.push(payload);
      const vectors = await provider(payload.input);
      return { statusCode: 200, content: JSON.stringify({ data: vectors.map((embedding, index) => ({ index, embedding })).reverse() }) };
    },
  };
  const { DatabaseCharacterRepository } = loadModule("src/storage/database.ts", disk.globals);
  const repository = await DatabaseCharacterRepository.open();
  /** Enables the actual semantic settings through the production repository. */
  async function enable(target = repository) {
    await target.run(async session => session.writeMemorySettings(ownerKey, { ...await session.readMemorySettings(ownerKey), cloudEmbeddingEnabled: true,
      cloudEmbeddingEndpoint: "https://provider.test", cloudEmbeddingModel: "model", cloudEmbeddingApiKey: "" }));
    await target.run(session => session.writeMemorySearchConfig(ownerKey, { scoreMode: "BALANCED", keywordWeight: 0, tagWeight: 0, vectorWeight: 1, edgeWeight: 0 }));
  }
  /** Reads exact persisted envelopes independently of production search logic. */
  function records(collection = "memories") {
    return disk.storage.inspect(sql => sql.prepare("SELECT key,value,version FROM records WHERE collection=? ORDER BY key").all(collection).map(row => ({ ...row, value: JSON.parse(row.value) })));
  }
  return { disk, requests, repository, enable, records,
    /** Reopens an independent production repository on the same SQLite file. */
    reopen: () => DatabaseCharacterRepository.open(),
    /** Replaces only the controlled external provider response. */
    provider: next => { provider = next; } };
}

/** Proves corpus construction occurs on writes and retrieval has one ephemeral query input. */
test("database vectors belong to source records; queries generate one vector and never persist cache entries", async t => {
  const env = await environment(t); await env.enable();
  await env.repository.run(session => session.createMemory(ownerKey, values("A", "north trip")));
  await env.repository.run(session => session.createMemory(ownerKey, values("B", "unrelated")));
  assert.deepEqual(env.records().sort((a, b) => a.value.record.title.localeCompare(b.value.record.title)).map(row => row.value.embedding.vector), [[1, 0], [0, 1]]);
  for (const row of env.records()) assert.equal(row.value.embedding.text, row.value.record.content);
  env.requests.length = 0; env.disk.clearCalls();
  const before = env.records();
  const result = await env.repository.run(session => session.searchMemoryPage(options()));
  assert.equal(result.items.length, 1); assert.equal(result.items[0].title, "A");
  assert.deepEqual(env.requests.map(request => request.input), [["travel"]]);
  assert.deepEqual(env.records(), before); assert.equal(env.records("embeddings").length, 0);
  assert.equal(env.disk.calls.some(call => call.method === "storage.commit"), false);
  assert.equal(env.disk.calls.some(call => call.method === "storage.list" && call.args[0] !== "owners"), false);
  assert.equal(env.disk.calls.some(call => call.method === "wasm.load"), true);
  assert.equal(env.disk.calls.some(call => call.method === "storage.query" && /WITH |json_each|row_number/iu.test(call.args[0])), false);
});

/** Proves independent readers cannot create duplicate logical vectors. */
test("independent concurrent queries perform no corpus writes", async t => {
  const env = await environment(t); await env.enable();
  await env.repository.run(session => session.createMemory(ownerKey, values("A", "north trip")));
  const other = await env.reopen(); env.requests.length = 0; env.disk.clearCalls();
  const results = await Promise.all([env.repository.run(session => session.searchMemories(options())), other.run(session => session.searchMemories(options()))]);
  for (const result of results) assert.equal(result[0].title, "A");
  assert.equal(env.requests.length, 2); assert.equal(env.records().length, 1); assert.equal(env.records("embeddings").length, 0);
  assert.equal(env.disk.calls.some(call => call.method === "storage.commit"), false);
});

/** Proves an HTTP wait neither owns the repository queue nor overwrites a newer source version. */
test("a blocked embedding does not block reads and a stale writer rejects", async t => {
  const env = await environment(t); await env.enable();
  const memory = await env.repository.run(session => session.createMemory(ownerKey, values("A", "initial")));
  const other = await env.reopen();
  let entered, release;
  const ready = new Promise(resolve => { entered = resolve; }), gate = new Promise(resolve => { release = resolve; });
  env.provider(async texts => { if (texts[0] === "stale") { entered(); await gate; } return texts.map(() => [1, 0]); });
  const stale = env.repository.run(session => session.updateMemory(ownerKey, memory.id, values("A", "stale")));
  await ready;
  assert.equal((await env.repository.run(session => session.readMemoryRebuildProgress(ownerKey))).status, "idle");
  await other.run(session => session.updateMemory(ownerKey, memory.id, values("A", "fresh")));
  release(); await assert.rejects(stale, /version conflict/u);
  const row = env.records()[0]; assert.equal(row.value.record.content, "fresh"); assert.equal(row.value.embedding.text, "fresh");
});

/** Proves the provider configuration is guarded within the source commit transaction. */
test("a provider change during embedding rejects the obsolete write without creating an orphan", async t => {
  const env = await environment(t); await env.enable();
  let entered, release;
  const ready = new Promise(resolve => { entered = resolve; }), gate = new Promise(resolve => { release = resolve; });
  env.provider(async texts => { entered(); await gate; return texts.map(() => [1, 0]); });
  const pending = env.repository.run(session => session.createMemory(ownerKey, values("A"))); await ready;
  await env.repository.run(async session => session.writeMemorySettings(ownerKey, { ...await session.readMemorySettings(ownerKey), cloudEmbeddingModel: "changed" }));
  release(); await assert.rejects(pending, /record guard failed/u); assert.equal(env.records().length, 0);
});

/** Proves historical vectors are built explicitly with bounded parallel batch requests. */
test("explicit rebuild uses concurrent indexed batches and query never builds historical vectors", async t => {
  const env = await environment(t);
  await env.repository.run(async session => {
    const state = await session.readMemorySpace(ownerKey), now = Date.now();
    for (let index = 0; index < 130; index += 1) state.memories.push({ ...values("M" + index), tags: [], properties: [], id: String(index), uuid: "m-" + index,
      isDocumentNode: false, documentPath: null, chunkIndexFilePath: null, createdAt: now, updatedAt: now, lastAccessedAt: now });
    await session.writeMemorySpace(ownerKey, state);
  });
  await env.enable();
  await assert.rejects(env.repository.run(session => session.searchMemories(options())), /index is incomplete/u);
  assert.deepEqual(env.requests.map(request => request.input), [["travel"]]);
  let active = 0, peak = 0;
  env.provider(async texts => { active += 1; peak = Math.max(peak, active); await new Promise(resolve => setTimeout(resolve, 10)); active -= 1; return texts.map(() => [1, 0]); });
  env.requests.length = 0;
  assert.equal(await env.repository.run(session => session.rebuildMemoryEmbeddings(ownerKey)), 130);
  assert.deepEqual(env.requests.map(request => request.input.length), [64, 64, 2]); assert.equal(peak, 3);
  assert.equal(env.records().filter(row => row.value.embedding !== null).length, 130);
  env.requests.length = 0; await env.repository.run(session => session.searchMemories(options())); assert.equal(env.requests.length, 1);
});

/** Proves dimension drift is an explicit index error without query-time index mutation. */
test("provider dimension changes require an explicit rebuild", async t => {
  const env = await environment(t); await env.enable(); await env.repository.run(session => session.createMemory(ownerKey, values("A")));
  const before = env.records(); env.provider(async texts => texts.map(() => [1, 0, 0]));
  await assert.rejects(env.repository.run(session => session.searchMemories(options())), /dimensions changed/u); assert.deepEqual(env.records(), before);
});

/** Proves a document source, its chunks and its vectors share deletion semantics. */
test("document imports batch source embeddings and deletion removes every associated vector", async t => {
  const env = await environment(t); await env.enable(); const memory = await env.repository.run(session => session.createMemory(ownerKey, values("Document")));
  env.requests.length = 0;
  await env.repository.run(async session => {
    const state = await session.readMemorySpace(ownerKey); state.memories[0].isDocumentNode = true; state.memories[0].documentPath = "document.txt";
    state.chunks.push({ id: "c1", memoryUuid: memory.uuid, chunkIndex: 0, content: "north trip" }, { id: "c2", memoryUuid: memory.uuid, chunkIndex: 1, content: "unrelated" });
    await session.writeMemorySpace(ownerKey, state);
  });
  assert.equal(env.requests.length, 1); assert.deepEqual(env.requests[0].input, ["north trip", "unrelated"]);
  const hits = await env.repository.run(session => session.searchMemoryChunks(ownerKey, memory.id, "travel", 1)); assert.equal(hits.chunks[0].id, "c1");
  env.disk.clearCalls(); await env.repository.run(session => session.deleteMemory(ownerKey, memory.id));
  assert.equal(env.records().length, 0); assert.equal(env.records("chunks").length, 0); assert.equal(env.records("embeddings").length, 0);
  const commits = env.disk.calls.filter(call => call.method === "storage.commit"); assert.equal(commits.length, 1); assert.equal(commits[0].args[0].length, 3);
});

/** Proves exclusions are applied before limiting, with the exact matching exclusion count. */
test("database pagination excludes delivered identities before its result limit", async t => {
  const env = await environment(t);
  const first = await env.repository.run(session => session.createMemory(ownerKey, values("first fact")));
  const second = await env.repository.run(session => session.createMemory(ownerKey, values("second fact")));
  const result = await env.repository.run(session => session.searchMemoryPage(options("fact", { excludeIds: [first.id, "absent"], limit: 1 })));
  assert.equal(result.items.length, 1); assert.equal(result.items[0].id, second.id); assert.equal(result.excludedBySnapshotCount, 1);
});

/** Rejects stale synchronized source envelopes without repairing them during retrieval. */
test("changed source text requires rebuilding its persisted vector", async t => {
  const env = await environment(t); await env.enable();
  await env.repository.run(session => session.createMemory(ownerKey, values("A", "north trip")));
  env.disk.storage.inspect(sql => {
    sql.prepare("UPDATE records SET value=json_set(value,'$.record.content',?) WHERE collection='memories'").run("changed source");
  });
  const before = env.records(); env.disk.clearCalls();
  await assert.rejects(env.repository.run(session => session.searchMemories(options())), /rebuild memory embeddings/u);
  assert.deepEqual(env.records(), before);
  assert.equal(env.disk.calls.some(call => call.method === "storage.commit"), false);
  await env.repository.run(session => session.rebuildMemoryEmbeddings(ownerKey));
  assert.equal(env.records()[0].value.embedding.text, "changed source");
});

/** Shares one ephemeral query vector across node recall and every selected document. */
test("a multi-document retrieval embeds its query once", async t => {
  const env = await environment(t); await env.enable();
  await env.repository.run(session => session.createMemory(ownerKey, values("north one")));
  await env.repository.run(session => session.createMemory(ownerKey, values("north two")));
  await env.repository.run(async session => {
    const state = await session.readMemorySpace(ownerKey);
    for (const [index, memory] of state.memories.entries()) {
      memory.isDocumentNode = true; memory.documentPath = memory.title + ".txt";
      state.chunks.push({ id: "doc-" + index, memoryUuid: memory.uuid, chunkIndex: 0, content: "north trip" });
    }
    await session.writeMemorySpace(ownerKey, state);
  });
  env.requests.length = 0; env.disk.clearCalls();
  const result = await env.repository.run(session => session.searchMemoryPage(options("travel", { limit: 2 })));
  assert.equal(result.items.length, 2); assert.equal(result.documents.length, 2);
  for (const document of result.documents) {
    assert.equal(document.total, 1); assert.equal(document.chunks[0].content, "north trip");
  }
  assert.deepEqual(env.requests.map(request => request.input), [["travel"]]);
  assert.equal(env.disk.calls.some(call => call.method === "storage.commit"), false);
});

/** Warm searches perform indexed change polls and bounded hydration, without reading the corpus. */
test("warm ANN queries reuse WASM and never issue corpus or ranking SQL", async t => {
  const env = await environment(t); await env.enable();
  const memory = await env.repository.run(session => session.createMemory(ownerKey, values("A", "north trip")));
  await env.repository.run(session => session.searchMemoryPage(options()));
  env.disk.clearCalls();
  for (let iteration = 0; iteration < 4; iteration++) {
    const page = await env.repository.run(session => session.searchMemoryPage(options()));
    assert.equal(page.items[0].id, memory.id);
  }
  assert.equal(env.disk.calls.filter(call => call.method === "storage.changes").length, 4);
  assert.equal(env.disk.calls.some(call => call.method === "wasm.load" || call.method === "storage.query"), false);
  const reads = env.disk.calls.filter(call => call.method === "storage.getMany");
  assert.equal(reads.length, 4);
  assert.ok(reads.every(call => call.args[0] === "memories" && call.args[1].length === 1));
});

/** A retained ANN/text index must follow another repository's writes and removals. */
test("warm derived indexes track external writers updates and deletions", async t => {
  const env = await environment(t); await env.enable();
  const memory = await env.repository.run(session => session.createMemory(ownerKey, values("A", "north trip")));
  assert.equal((await env.repository.run(session => session.searchMemoryPage(options()))).items[0].id, memory.id);
  const writer = await env.reopen();
  await writer.run(session => session.updateMemory(ownerKey, memory.id, values("A", "unrelated")));
  assert.equal((await env.repository.run(session => session.searchMemoryPage(options()))).items.length, 0);
  const second = await writer.run(session => session.createMemory(ownerKey, values("B", "north trip")));
  assert.equal((await env.repository.run(session => session.searchMemoryPage(options()))).items[0].id, second.id);
  await writer.run(session => session.deleteMemory(ownerKey, second.id));
  assert.equal((await env.repository.run(session => session.searchMemoryPage(options()))).items.length, 0);
  assert.equal(env.disk.calls.filter(call => call.method === "wasm.load").length, 1);
});

/** Reconciliation may update content without replacing its originating storage version. */
test("warm indexes reject stale vectors even when an external reconciliation retains the source version", async t => {
  const env = await environment(t); await env.enable();
  await env.repository.run(session => session.createMemory(ownerKey, values("A", "north trip")));
  await env.repository.run(session => session.searchMemoryPage(options()));
  env.disk.storage.inspect(sql => sql.prepare("UPDATE records SET value=json_set(value,'$.record.content',?) WHERE collection='memories'").run("changed source"));
  await assert.rejects(env.repository.run(session => session.searchMemoryPage(options())), /rebuild memory embeddings/u);
});

/** Folder constraints must participate in native ANN search, including a rare scoped neighbor. */
test("ANN filtering finds scoped memories beyond an unfiltered candidate page", async t => {
  const env = await environment(t); await env.enable();
  const first = await env.repository.run(session => session.createMemory(ownerKey, values("seed", "north trip")));
  await env.repository.run(async session => {
    const space = await session.readMemorySpace(ownerKey);
    space.memories = Array.from({ length: 320 }, (_, index) => ({ ...first, id: "outside-" + index, uuid: "outside-" + index, title: "outside-" + index, folderPath: "outside" }));
    space.memories.push({ ...first, id: "inside", uuid: "inside", title: "inside", content: "less similar", folderPath: "inside" });
    await session.writeMemorySpace(ownerKey, space);
  });
  env.provider(async texts => texts.map(text => text === "less similar" ? [0.8, 0.6] : [1, 0]));
  await env.repository.run(session => session.rebuildMemoryEmbeddings(ownerKey));
  const page = await env.repository.run(session => session.searchMemoryPage(options("travel", { folderPath: "inside" })));
  assert.equal(page.items.length, 1); assert.equal(page.items[0].id, "inside");
});

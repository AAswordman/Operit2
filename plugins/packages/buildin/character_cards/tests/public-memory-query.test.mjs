import assert from "node:assert/strict";
import test from "node:test";
import { createDiskHarness } from "./disk-files.mjs";
import { loadModule, plain } from "./runtime.mjs";

/** Connects the canonical public client to actual registered provider handlers and real SQLite storage. */
async function environment(t) {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }), registrations = new Map();
  const provider = loadModule("src/public-api.ts", { ...disk.globals, ToolPkg: { ...disk.globals.ToolPkg,
    /** Captures only the public registry; production callbacks implement all business behavior. */
    registerApi(definition) { registrations.set(definition.name, definition.function); },
  } });
  provider.registerDomainApis();
  const client = loadModule("src/api.ts", {
    ToolPkg: {
      /** Forwards the exact public dependency envelope without substituting provider data. */
      async callDependency(packageId, method, payload) {
        assert.equal(packageId, "com.operit.character_cards");
        assert.ok(registrations.has(method), "Method must be explicitly published: " + method);
        return registrations.get(method)({ payload: plain(payload), callerPackage: "com.operit.test_consumer" });
      },
    },
  }).characterCards;
  return { disk, client };
}

/** Exercises real owner resolution, provider-retained snapshots, limits, and independent participant data. */
test("published memory queries retain owner-scoped deduplication in the provider", async t => {
  const env = await environment(t), card = await env.client.characters.create({ values: { name: "Second participant" } });
  for (const title of ["First", "Second"]) await env.client.memory.create({ ownerKey: "character:default", values: { title, content: title + " complete content" } });
  await env.client.memory.create({ ownerKey: "character:" + card.id, values: { title: "Private", content: "Other participant content" } });
  const request = { participantId: "default", query: "*", limit: 1, snapshotId: "same-chat" };
  const first = await env.client.memory.query(request), second = await env.client.memory.query(request), third = await env.client.memory.query(request);
  assert.equal(first.memories.length, 1); assert.equal(second.memories.length, 1);
  assert.notEqual(first.memories[0].title, second.memories[0].title); assert.equal(third.memories.length, 0);
  assert.equal(first.snapshotCreated, true); assert.equal(second.snapshotCreated, false);
  assert.deepEqual(plain((await env.client.memory.query({ ...request, participantId: card.id })).memories).map(item => item.title), ["Private"]);
  assert.equal((await env.client.memory.query({ ...request, snapshotId: null })).memories.length, 1);
  await assert.rejects(env.client.memory.query({ ...request, participantId: "" }), /must not be blank/);
  await assert.rejects(env.client.memory.query({ ...request, limit: 1.5 }), /Expected a nonnegative integer/);
  await assert.rejects(env.client.memory.query({ ...request, participantId: "missing-participant" }));
  const beforeFailure = env.disk.stateBytes();
  const failure = new Error("query-storage-original"); env.disk.failNext("storage.preferences.data", failure);
  await assert.rejects(env.client.memory.query({ ...request, snapshotId: null }), error => error === failure);
  assert.deepEqual(env.disk.stateBytes(), beforeFailure);
});

/** Preserves real document chunk selection through the public client without copying query business logic. */
test("public memory query returns persisted document snippets and lossless chunk identities", async t => {
  const env = await environment(t);
  await env.client.memory.create({ ownerKey: "character:default", values: { title: "Document", content: "lexical document" } });
  const backup = JSON.parse((await env.client.memory.export({ ownerKey: "character:default" })).content);
  const memory = backup.space.memories[0];
  Object.assign(memory, { isDocumentNode: true, documentPath: "plugin:/document.txt", chunkIndexFilePath: "plugin:/chunks.json" });
  backup.space.chunks = [{ id: "9007199254740993", memoryUuid: memory.uuid, chunkIndex: 0, content: "lexical document match" }];
  await env.client.memory.import({ ownerKey: "character:default", content: JSON.stringify(backup), strategy: "UPDATE" });
  const result = await env.client.memory.query({ participantId: "default", query: "lexical", limit: 1, snapshotId: null });
  assert.equal(result.memories.length, 1);
  assert.match(result.memories[0].content, /lexical document match/);
  assert.deepEqual(plain(result.memories[0].chunkIndices), [0]);
  assert.equal(env.disk.readState().owners[0].chunks[0].id, "9007199254740993");
});

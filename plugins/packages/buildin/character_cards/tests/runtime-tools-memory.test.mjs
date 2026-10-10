import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { createServer } from "node:http";
import { createDiskHarness } from "./disk-files.mjs";
import { loadModule, plain } from "./runtime.mjs";

/** Opens the production service over real disk files and supplies no domain repository substitute. */
async function environment(t) {
  const disk = await createDiskHarness(t);
  const runtime = loadModule("src/service-runtime.ts", disk.globals);
  const memory = loadModule("src/runtime-tools/memory.ts", disk.globals);
  /** Sends every tool operation to the same actual production domain dispatcher. */
  const domain = (operation, input) => runtime.dispatchDomain(operation, input);
  return { disk, runtime, tools: new memory.MemoryTools(domain), domain, context: { chatId: null, participantId: null, callerName: null } };
}
/** Calls a tool using the real first-install owner identity rather than a mocked card response. */
async function call(environment, name, input) {
  return environment.tools.execute(name, { target_owner_key: "character:default", ...input }, environment.context);
}
/** Reads the authoritative plugin file for independent mutation and identity assertions. */
async function state(environment) { return JSON.parse(await environment.disk.stateBytes().toString("utf8")); }

test("twelve memory tools and the plugin character list match exact executable metadata and typed IPC", async () => {
  const source = await readFile(new URL("../src/runtime-tools/tools.ts", import.meta.url), "utf8"), match = /\/\* METADATA\s+([\s\S]+?)\*\//.exec(source);
  assert.ok(match);
  const metadata = JSON.parse(match[1]), requests = [];
  const tools = loadModule("src/runtime-tools/tools.ts", { ToolPkg: { ipc: {
    /** Captures only the transport envelope and rejects the request rather than fabricating domain data. */
    async call(channel, request, options) { requests.push({ channel, request, options }); throw new Error("transport-original"); },
  } } });
  assert.equal(metadata.tools.length, 13);
  for (const entry of metadata.tools) assert.equal(typeof tools[entry.name], "function", entry.name);
  await assert.rejects(tools.create_memory({ target_owner_key: "character:default", title: "IPC", content: "content" }), /transport-original/);
  assert.equal(requests.length, 1); assert.equal(requests[0].channel, "character-memory.domain");
  assert.deepEqual(plain(requests[0].options), { targetRuntime: "main" });
  assert.equal(requests[0].request.operation, "memory.create"); assert.equal(requests[0].request.input.ownerKey, "character:default");
});

test("memory CRUD, folder moves and USER.md use the real production file service", async t => {
  const environment = await environmentFor(t);
  assert.equal(await environment.tools.execute("get_memory_owner_key", { caller_card_id: "default" }, environment.context), "character:default");
  assert.match(await call(environment, "create_memory", { title: "Alpha", content: "alpha complete content", tags: "one,two", folder_path: "original" }), /Successfully created/);
  const persisted = await state(environment), memory = persisted.owners[0].memories[0];
  assert.equal(typeof memory.id, "string"); assert.equal(memory.content, "alpha complete content");
  await call(environment, "update_memory", { old_title: "Alpha", new_title: "Alpha edited", credibility: 0.9 });
  const result = await call(environment, "get_memory_by_title", { title: "Alpha edited" });
  assert.equal(result.memories[0].content, "alpha complete content"); assert.deepEqual(plain(result.memories[0].tags), ["one", "two"]);
  await call(environment, "move_memory", { titles: "Alpha edited", source_folder_path: "original", target_folder_path: "moved" });
  assert.equal((await state(environment)).owners[0].memories[0].folderPath, "moved");
  await call(environment, "update_user_preferences", { content: "full USER.md content" });
  assert.equal((await environment.domain("memory.user.read", { ownerKey: "character:default" })).content, "full USER.md content");
  await call(environment, "delete_memory", { title: "Alpha edited" }); assert.equal((await state(environment)).owners[0].memories.length, 0);
});

/** Provides the actual environment under a distinct name so local test variables cannot shadow initialization. */
async function environmentFor(t) { return environment(t); }

test("query snapshots isolate owners and preserve full records and filters", async t => {
  const environment = await environmentFor(t);
  for (const [title, folder] of [["One", "notes"], ["Two", "notes"], ["Three", "elsewhere"]]) await call(environment, "create_memory", { title, content: "complete text for " + title, folder_path: folder });
  const first = await call(environment, "query_memory", { query: "*", folder_path: "notes", limit: 1 });
  assert.equal(first.memories.length, 1); assert.equal(first.snapshotCreated, true);
  const second = await call(environment, "query_memory", { query: "*", folder_path: "notes", limit: 1, snapshot_id: first.snapshotId });
  assert.equal(second.memories.length, 1); assert.equal(second.snapshotCreated, false); assert.equal(second.excludedBySnapshotCount, 1);
  assert.notEqual(first.memories[0].title, second.memories[0].title);
  const third = await call(environment, "query_memory", { query: "*", folder_path: "notes", snapshot_id: first.snapshotId }); assert.equal(third.memories.length, 0);
  await assert.rejects(call(environment, "query_memory", { query: "*", start_time: "2026-02-30" }), /Local time is invalid/);
  await assert.rejects(call(environment, "query_memory", { query: "*", threshold: -1 }), /Invalid threshold/);
});

test("relationship CRUD preserves string identities, description and unspecified patch fields", async t => {
  const environment = await environmentFor(t);
  for (const title of ["Source", "Target"]) await call(environment, "create_memory", { title, content: title + " complete content" });
  const linked = await call(environment, "link_memories", { source_title: "Source", target_title: "Target", weight: 0.7, description: "complete relation description" }); assert.equal(linked.description, "complete relation description");
  const queried = await call(environment, "query_memory_links", { source_title: "Source", target_title: "Target" });
  assert.equal(queried.links.length, 1); assert.equal(typeof queried.links[0].linkId, "string");
  const id = queried.links[0].linkId, updated = await call(environment, "update_memory_link", { link_id: id, description: "new complete description" });
  assert.equal(updated.links[0].weight, 0.7); assert.equal(updated.links[0].description, "new complete description");
  await assert.rejects(call(environment, "update_memory_link", { link_id: id, weight: "bad" }), /finite number/);
  await call(environment, "delete_memory_link", { link_id: id }); assert.equal((await call(environment, "query_memory_links", {})).totalCount, 0);
});

test("owner permissions and actual filesystem errors propagate without empty data or old tools", async t => {
  const environment = await environmentFor(t), shared = await environment.domain("memory.shared.create", { name: "Shared" });
  const failure = new Error("original disk read failure"); environment.disk.failNext("storage.list", failure);
  await assert.rejects(call(environment, "query_memory", { query: "*" }), error => error === failure);
  await assert.rejects(environment.tools.execute("create_memory", { target_owner_key: "shared:" + shared.id, title: "Denied", content: "Denied" }, { chatId: "real-chat", participantId: "default", callerName: null }), /Memory owner access denied/);
  await assert.rejects(call(environment, "create_memory", { title: "No owner", content: "content", target_owner_key: "character:removed" }), /does not identify exactly one record: removed/);
  assert.equal((await state(environment)).owners[0].memories.length, 0);
});

/** Imports a genuine complete document through the production main-domain operation. */
async function addDocument(environment) {
  await call(environment, "create_memory", { title: "Document", content: "complete document" });
  const exported = await environment.domain("memory.export", { ownerKey: "character:default" });
  const backup = JSON.parse(exported.content), memory = backup.space.memories[0];
  Object.assign(memory, { isDocumentNode: true, documentPath: "plugin:/document.txt", chunkIndexFilePath: "plugin:/chunks.json" });
  backup.space.chunks = [{ id: "9007199254740993", memoryUuid: memory.uuid, chunkIndex: 0, content: "lexical match first" }, { id: "9007199254740994", memoryUuid: memory.uuid, chunkIndex: 1, content: "semantic target" }];
  await environment.domain("memory.import", { ownerKey: "character:default", content: JSON.stringify(backup), strategy: "UPDATE" });
}

test("document selectors and queries read complete persisted chunks without rounding their identities", async t => {
  const environment = await environmentFor(t); await addDocument(environment);
  const chunks = (await state(environment)).owners[0].chunks;
  assert.equal(chunks[0].id, "9007199254740993");
  const byIndex = await call(environment, "get_memory_by_title", { title: "Document", chunk_index: 2 });
  assert.match(byIndex, /Chunk 2\/2:/); assert.match(byIndex, /semantic target/);
  const byRange = await call(environment, "get_memory_by_title", { title: "Document", chunk_range: "1-2" });
  assert.match(byRange, /lexical match first/); assert.match(byRange, /semantic target/);
  assert.match(await call(environment, "get_memory_by_title", { title: "Document", query: "lex*cal", limit: 1 }), /lexical match first/);
  await assert.rejects(call(environment, "get_memory_by_title", { title: "Document", chunk_range: "0-2" }), /out of bounds/);
  await assert.rejects(call(environment, "get_memory_by_title", { title: "Document", query: "lexical", chunk_index: 1 }), /exactly one/);
  const result = await call(environment, "query_memory", { query: "lexical", limit: 1 });
  assert.deepEqual(plain(result.memories[0].chunkIndices), [0]);
});

/** Exercises existing HTTP IO against a local provider while retaining the actual plugin file and domain service. */
async function embeddingEnvironment(t) {
  const requests = [], vectors = new Map([["travel", [1, 0]], ["semantic target", [1, 0]], ["lexical match first", [0, 1]], ["Document", [0, 1]]]);
  let fail = false;
  const server = createServer(
    /** Produces explicit known provider vectors and rejects unexpected real requests. */
    async (request, response) => {
      try {
        let text = ""; for await (const bytes of request) text += bytes.toString("utf8");
        const payload = JSON.parse(text); requests.push(payload);
        if (fail) { response.writeHead(503); response.end("embedding-original"); return; }
        if (request.url !== "/embeddings" || request.method !== "POST" || payload.model !== "test-vector-model" || !vectors.has(payload.input)) { response.writeHead(400); response.end("unexpected embedding request"); return; }
        response.writeHead(200, { "Content-Type": "application/json" }); response.end(JSON.stringify({ data: [{ embedding: vectors.get(payload.input) }] }));
      } catch (error) { response.writeHead(500); response.end(error.message); }
    },
  );
  await new Promise(
    /** Waits for a genuine loopback listener before configuring the production embedding capability. */
    (resolve, reject) => { server.once("error", reject); server.listen(0, "127.0.0.1", resolve); },
  );
  t.after(
    /** Releases the exact owned network listener after its test has finished. */
    () => new Promise((resolve, reject) => { server.close(error => error === undefined ? resolve() : reject(error)); server.closeAllConnections(); }),
  );
  const disk = await createDiskHarness(t), address = server.address();
  assert.notEqual(address, null); assert.equal(typeof address, "object");
  const endpoint = "http://127.0.0.1:" + address.port;
  const globals = { ...disk.globals, Tools: { Files: disk.files, Storage: disk.storage, Net: {
    /** Adapts only existing HTTP IO; all embedding validation and ranking remain production code. */
    async http(options) {
      const response = await fetch(options.url, { method: options.method, headers: options.headers, body: options.body });
      return { statusCode: response.status, content: await response.text() };
    },
  } } };
  const runtime = loadModule("src/service-runtime.ts", globals), memory = loadModule("src/runtime-tools/memory.ts", globals);
  /** Sends every operation to the one real production dispatcher instead of replacing business responses. */
  const domain = (operation, input) => runtime.dispatchDomain(operation, input);
  return { disk, runtime, domain, tools: new memory.MemoryTools(domain), context: { chatId: null, participantId: null, callerName: null }, endpoint, requests,
    /** Changes the real provider response for error propagation assertions. */
    fail() { fail = true; },
  };
}

test("async document ranking uses actual embedding HTTP and preserves original provider failures", async t => {
  const environment = await embeddingEnvironment(t); await addDocument(environment);
  const settings = await environment.domain("memory.settings.read", { ownerKey: "character:default" });
  await environment.domain("memory.settings.write", { ownerKey: "character:default", settings: { ...settings, cloudEmbeddingEnabled: true, cloudEmbeddingEndpoint: environment.endpoint, cloudEmbeddingApiKey: "", cloudEmbeddingModel: "test-vector-model" } });
  const config = await environment.domain("memory.searchConfig.read", { ownerKey: "character:default" });
  await environment.domain("memory.searchConfig.write", { ownerKey: "character:default", config: { ...config, keywordWeight: 0, tagWeight: 0, vectorWeight: 1, edgeWeight: 0 } });
  const title = await call(environment, "get_memory_by_title", { title: "Document", query: "travel", limit: 1 });
  assert.match(title, /semantic target/); assert.doesNotMatch(title, /lexical match first/);
  assert.equal(environment.requests.length, 4);
  const committed = (await state(environment)).owners[0].embeddings;
  assert.deepEqual(committed.map(item => item.text).sort(), ["Document", "lexical match first", "semantic target", "travel"]);
  for (const entry of committed) { assert.equal(entry.endpoint, environment.endpoint); assert.equal(entry.model, "test-vector-model"); assert.ok(entry.vector.length !== 0); }
  const result = await call(environment, "query_memory", { query: "travel", limit: 1 });
  assert.equal(result.memories.length, 1); assert.deepEqual(plain(result.memories[0].chunkIndices), [1]);
  assert.equal(environment.requests.length, 4, "Both tools must reuse the canonical persisted embedding cache");
  const snapshotId = result.snapshotId, beforeFailure = await state(environment); environment.fail();
  await assert.rejects(call(environment, "query_memory", { query: "uncached failure", snapshot_id: snapshotId }), /Embedding request failed: HTTP 503 embedding-original/);
  assert.deepEqual(await state(environment), beforeFailure, "Rejected search must not publish a partial embedding cache");
});


test("character list is a real plugin tool and propagates the main repository results and failures", async t => {
  const environment = await environmentFor(t);
  const tools = loadModule("src/runtime-tools/tools.ts", { ToolPkg: { ipc: {
    /** Executes the real production main dispatcher after checking its exact package-owned envelope. */
    async call(channel, request, options) {
      assert.equal(channel, "character-memory.domain"); assert.equal(options.targetRuntime, "main");
      return environment.domain(request.operation, request.input);
    },
  } } });
  const result = await tools.list_character_cards({});
  const persisted = await state(environment);
  assert.equal(result.totalCount, persisted.cards.length); assert.equal(result.cards[0].id, persisted.cards[0].id);
  assert.equal(result.cards[0].description, persisted.cards[0].description);
  await assert.rejects(tools.list_character_cards({ guessed_source: "old manager" }), /Unknown list_character_cards parameter/);
  const failure = new Error("actual-list-file-rejection"); environment.disk.failNext("storage.list", failure);
  await assert.rejects(tools.list_character_cards({}), error => error === failure);
});

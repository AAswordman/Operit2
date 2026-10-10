import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { createDiskHarness } from "./disk-files.mjs";
import { loadModule, plain } from "./runtime.mjs";

const consumerRoot = new URL("../../../external/message_insert/", import.meta.url);
const require = createRequire(new URL("../../workflow/package.json", import.meta.url));
const ts = require("typescript");

/** Connects the copied dependency client to actual registered provider handlers and real SQLite storage. */
async function environment(t) {
  const disk = await createDiskHarness(t), registrations = new Map(), calls = [];
  const provider = loadModule("src/public-api.ts", { ...disk.globals, ToolPkg: { ...disk.globals.ToolPkg,
    /** Captures only the public registry; production callbacks implement all business behavior. */
    registerApi(definition) { registrations.set(definition.name, definition.function); },
  } });
  provider.registerDomainApis();
  const clientGlobals = {
    ToolPkg: {
      /** Forwards the exact public dependency envelope without substituting provider data. */
      async callDependency(packageId, method, payload) {
        assert.equal(packageId, "com.operit.character_cards");
        assert.ok(registrations.has(method), "Method must be explicitly published: " + method);
        calls.push({ packageId, method, payload: plain(payload) });
        return registrations.get(method)({ payload: plain(payload), callerPackage: "com.operit.message_insert_bundle" });
      },
    },
  };
  const client = loadModule("../../external/message_insert/src/dependencies/character_cards/api.ts", clientGlobals).characterCards;
  return { disk, client, calls, clientGlobals };
}

/** Prevents internal source imports and keeps the copied SDK aligned with its declared prerequisite. */
test("message_insert vendors the standalone public SDK and declares its runtime prerequisite", () => {
  const source = readFileSync(new URL("../src/api.ts", import.meta.url), "utf8");
  assert.equal(readFileSync(new URL("src/dependencies/character_cards/api.ts", consumerRoot), "utf8"), source);
  const parsed = ts.createSourceFile("api.ts", source, ts.ScriptTarget.Latest, true);
  for (const statement of parsed.statements) {
    assert.equal(ts.isImportDeclaration(statement), false, "Public SDK must not import another file");
    if (ts.isExportDeclaration(statement)) assert.equal(statement.moduleSpecifier, undefined);
  }
  const manifest = JSON.parse(readFileSync(new URL("manifest.json", consumerRoot), "utf8"));
  assert.equal(manifest.api_version, "2.0.0");
  assert.deepEqual(manifest.requires, [{ id: "com.operit.character_cards", min_version: "0.1.0" }]);
});

/** Exercises real owner resolution, provider-retained snapshots, limits, and independent participant data. */
test("copied SDK memory queries retain owner-scoped deduplication in the provider", async t => {
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
  const failure = new Error("query-storage-original"); env.disk.failNext("storage.list", failure);
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

/** Supplies only hook settings and runtime registration while rejecting every old tool query. */
function consumer(env, persist) {
  let settings;
  const module = loadModule("../../external/message_insert/src/main.ts", {
    ...env.clientGlobals,
    RuntimeContext: {
      /** Keeps runtime registration side-effect free during module evaluation. */
      register() {},
    },
    PluginConfig: {
      /** Returns explicit test settings without implementing character or memory behavior. */
      async use(_name, defaults) { settings = { ...defaults, masterEnabled: true, persistInjectedContent: persist, injectTime: false, injectMemory: true, allowRepeatedMemorySearch: true }; return settings; },
    },
    /** Selects deterministic attachment labels. */
    getLang() { return "en-US"; },
    /** Rejects any attempt to query prerequisite data through executable tools. */
    toolCall() { throw new Error("Memory dependencies must use the public API"); },
  });
  return module;
}

/** Checks both injection modes against real persisted records and the exact generic execution participant. */
test("persisted and final prompt hooks query the prerequisite API through the copied SDK", async t => {
  const env = await environment(t);
  await env.client.memory.create({ ownerKey: "character:default", values: { title: "Injected", content: "Actual persisted memory content" } });
  for (const persist of [true, false]) {
    const module = consumer(env, persist), payload = { chatId: "chat-real", processedInput: "*", metadata: { executionContext: { chatId: "chat-real", participantId: "default" } } };
    const result = persist ? await module.onPromptInput({ eventPayload: { ...payload, stage: "before_process" } }) : await module.onPromptFinalize({ eventPayload: { ...payload, stage: "before_send_to_model" } });
    assert.match(result, /Actual persisted memory content/);
    assert.deepEqual(env.calls.at(-1), { packageId: "com.operit.character_cards", method: "memory.query", payload: { participantId: "default", query: "*", limit: 3, snapshotId: null } });
    const before = env.calls.length;
    const invalid = await module.onPromptInput({ eventPayload: { ...payload, stage: "before_process", metadata: { executionContext: { chatId: "chat-real", participantId: null } } } });
    if (persist) assert.match(invalid, /Memory injection requires a selected execution participant/);
    else assert.equal(invalid, null);
    assert.equal(env.calls.length, before);
  }
});

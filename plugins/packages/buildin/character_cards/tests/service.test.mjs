import assert from "node:assert/strict";
import test from "node:test";
import { createDiskHarness } from "./disk-files.mjs";
import { fixture } from "./fixtures.mjs";
import { loadModule, plain } from "./runtime.mjs";

const domain = loadModule("src/domain.ts");
const validation = loadModule("src/validation.ts");
const serialization = loadModule("src/serialization.ts");

/** Verifies exact domain owner grammar before any provider or persistence call can run. */
test("memory owner payloads use exact character/shared namespaces", () => {
  for (const ownerKey of ["character:travel", "shared:shared-main"]) {
    const input = { ownerKey, query: "不需要子串匹配" };
    assert.deepEqual(plain(domain.parseDomainPayload("memory.search", input)), input);
  }
  for (const ownerKey of ["character", "shared:", "character:a:b", "not-character:a", "shared:a b"]) {
    assert.throws(() => domain.parseDomainPayload("memory.graph", { ownerKey }), /character:<id> or shared:<id>/);
  }
});

/** Verifies full-record input retains every binding field rather than CLI-editable projections. */
test("complete character provider input retains TTS mounts tool policy and timestamps", () => {
  const values = fixture().snapshot.cards[1];
  Object.assign(values, { avatarUri: "vfs:/avatar.png", ttsConfigId: "tts-one", chatModelBindingMode: "FIXED_MODEL", chatModelId: "qwen-plus", chatProviderId: "provider-one", otherContentVoice: "语音内容", advancedCustomPrompt: "完整 prompt", marks: "完整备注" });
  values.sharedMemoryMounts = [{ sharedMemoryId: "shared-main", readable: true, writable: false }];
  values.toolAccessConfig = { enabled: true, allowedBuiltinTools: ["builtin"], allowedPackages: ["package"], allowedSkills: ["skill"], allowedMcpServers: ["mcp"] };
  const payload = { values };
  assert.deepEqual(plain(domain.parseDomainPayload("character.create", payload)), payload);
  const { id, ...changes } = values;
  assert.deepEqual(plain(domain.parseDomainPayload("character.update", { id, changes })), { id, changes });
});

/** Verifies memory provider input preserves document metadata, scores, properties, and UUIDs. */
test("complete memory record fields are retained and lossy record shapes are rejected", () => {
  const item = fixture().graph.items[0];
  Object.assign(item, { documentPath: "vfs:/document.pdf", isDocumentNode: true, chunkIndexFilePath: "vfs:/document.index" });
  const before = plain(item);
  validation.assertMemory(item);
  assert.deepEqual(item, before);
  const { id, ...changes } = item;
  changes.tags = item.tags.map(
    /** Uses the declared provider string-tag request shape without discarding stored fields. */
    tag => tag.name,
  );
  const payload = { ownerKey: "shared:shared-main", originalTitle: item.title, changes };
  assert.deepEqual(plain(domain.parseDomainPayload("memory.update", payload)), payload);
  const incomplete = plain(item); delete incomplete.properties;
  assert.throws(() => validation.assertMemory(incomplete), /标签和属性/);
  const incompleteCard = fixture().snapshot.cards[1]; delete incompleteCard.toolAccessConfig;
  const completedCard = loadModule("src/drafts.ts").readCharacter(incompleteCard);
  assert.deepEqual(plain(completedCard.toolAccessConfig), plain(loadModule("src/drafts.ts").createCharacterDraft().toolAccessConfig));
});

/** Verifies graph UUIDs and real link IDs are validated without manufacturing nodes or edges. */
test("graph validation preserves canonical identities and rejects malformed metadata", () => {
  const graph = fixture().graph.graph, before = plain(graph);
  validation.assertGraph(graph); assert.deepEqual(graph, before);
  graph.edges[0].metadata = { score: 42 };
  assert.throws(() => validation.assertGraph(graph), /图谱元数据值/);
});

/** Verifies native export/import preserves the complete plugin record while resetting only new identity. */
test("Operit import and export are pure domain operations with complete binding records", () => {
  const data = fixture(), record = data.snapshot.cards[1];
  record.ttsConfigId = "tts-one";
  record.sharedMemoryMounts = [{ sharedMemoryId: "shared-main", readable: true, writable: true }];
  record.toolAccessConfig.allowedPackages = ["fixture-package"];
  const exported = serialization.encodeCharacterExport(record, "operit", data.snapshot.tags);
  assert.deepEqual(JSON.parse(exported), record);
  const plan = serialization.decodeCharacterImport(exported, "operit", data.snapshot.tags);
  assert.deepEqual(plain(plan.card), { ...record, id: "", createdAt: 0, updatedAt: 0 });
  assert.deepEqual(plain(plan.tagChanges), { created: [], updated: [], deleted: [] });
  assert.throws(() => serialization.decodeCharacterImport("{", "operit", data.snapshot.tags), { name: "SyntaxError" });
});

/** Verifies imported tag edits remain staged data and never publish mutations during decode. */
test("Tavern tag import produces staged changes without a host or command executor", () => {
  const data = fixture();
  const content = JSON.stringify({ spec: "chara_card_v2", spec_version: "2.0", data: { name: "新角色", description: "描述", personality: "性格", first_mes: "你好", system_prompt: "设定", post_history_instructions: "提示", scenario: "场景", mes_example: "示例", creator_notes: "备注", avatar: "", alternate_greetings: [], tags: [], character_book: { entries: [{ keys: ["旅行"], content: "测试世界书内容", enabled: true, insertion_order: 0 }] }, extensions: {} } });
  const result = serialization.decodeCharacterImport(content, "tavern", data.snapshot.tags);
  assert.ok(result.tagChanges.created.length > 0);
  const stagedIds = result.tagChanges.created.map(
    /** Collects only identities generated for unsaved tag drafts. */
    draft => draft.draftId,
  );
  for (const id of result.card.attachedTagIds) assert.ok(stagedIds.indexOf(id) >= 0);
  assert.deepEqual(plain(data.snapshot.tags), fixture().snapshot.tags);
});

/** Verifies invalid payloads never coerce values or return empty successful results. */
test("domain validation rejects malformed scores owners and unsupported input fields", () => {
  assert.throws(() => domain.parseDomainPayload("memory.link.create", { ownerKey: "shared:shared-main", sourceTitle: "A", targetTitle: "B", linkType: "related", weight: 2, description: "" }), /between 0 and 1/);
  assert.throws(() => domain.parseDomainPayload("memory.searchWithOptions", { ownerKey: "character:default", query: " ", folderPath: null, relevanceThreshold: 0, createdAtStartMs: null, createdAtEndMs: null }), /must not be blank/);
  assert.throws(() => domain.parseDomainPayload("memory.chunks", { ownerKey: "character:default", memoryId: "memory-id", query: "", count: 5 }), /must not be blank/);
  assert.throws(() => domain.parseDomainPayload("memory.chunks", { ownerKey: "character:default", memoryId: "memory-id", query: "word", count: -1 }), /must not be negative/);
  assert.throws(() => domain.parseDomainPayload("memory.settings.write", { ownerKey: "shared:shared-main", settings: { ...fixture().settings, profileAutoUpdateEnabled: "true" } }), /boolean/);
  assert.throws(() => domain.parseDomainPayload("character.list", { unexpected: true }), /unexpected|unknown|unsupported/i);
  assert.throws(() => domain.jsonValue({ result: undefined }), /JSON object/);
});

/** Injects only a rejected dependency; it supplies no successful host, record, or production adapter. */
test("service dependency failures propagate unchanged instead of becoming empty snapshots", async () => {
  const api = loadModule("src/operations.ts"), failure = new Error("PERSISTENCE_NOT_CONNECTED_TEST"), reads = [];
  const rejected = new Proxy({}, {
    /** Rejects every explicitly requested catalog read with the same original failure. */
    get(_target, method) {
      return async () => { reads.push(String(method)); throw failure; };
    },
  });
  await assert.rejects(api.snapshot(rejected),
    /** Requires error identity preservation, not a new success-shaped wrapper. */
    error => error === failure,
  );
  assert.ok(reads.length > 0);
});

/** Checks the opaque identity contract shared by legacy decimals and new device-independent ids. */
test("record identifiers accept opaque strings and reject numeric or malformed values", () => {
  const item = fixture().graph.items[0], graph = fixture().graph.graph;
  for (const valid of ["9223372036854775807", "0f5c1c2e-9a4b-4f7c-8d2e-6a3b1c0d9e8f"]) {
    item.id = valid; graph.edges[0].id = valid;
    validation.assertMemory(item); validation.assertGraph(graph);
    assert.equal(item.id, valid); assert.equal(graph.edges[0].id, valid);
  }
  for (const invalid of [1, "", " padded", null]) {
    item.id = invalid; graph.edges[0].id = invalid;
    assert.throws(() => validation.assertMemory(item), /记忆数据库标识/);
    assert.throws(() => validation.assertGraph(graph), /图谱关系标识/);
  }
});

/** Exercises production ranking on real database records instead of an in-memory corpus helper. */
async function searchStored(t, space, options, config) {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" });
  const { DatabaseCharacterRepository } = loadModule("src/storage/database.ts", disk.globals), repository = await DatabaseCharacterRepository.open();
  const state = plain(disk.readState()); Object.assign(state.owners[0], space); disk.replaceState(state);
  return repository.run(session => session.searchMemoryDebug({ ...options, ownerKey: "character:default" }, config));
}

/** Verifies Kotlin RRF values, mode weights, reverse edges, and scope filters together. */
test("memory ranking preserves channel scores and excludes filtered graph endpoints", async t => {
  const base = fixture().graph.items[0];
  /** Creates independent records with explicit scope and importance. */
  const record = (id, title, changes = {}) => ({ ...base, id, uuid: "uuid-" + id, title, importance: 0.5,
    folderPath: "scope/child", createdAt: 10, tags: [], isDocumentNode: false, ...changes });
  const memories = [record("seed", "alpha beta", { tags: [{ id: "tag-alpha", name: "alpha" }] }),
    record("second", "alpha elsewhere", { importance: 0.8 }), record("neighbor", "linked record"),
    record("outside", "alpha beta", { folderPath: "elsewhere" }), record("expired", "alpha beta", { createdAt: 9 }),
    record("placeholder", ".folder_placeholder")];
  /** Builds a relationship whose incoming direction must propagate identically to its outgoing direction. */
  const link = (id, sourceMemoryId, targetMemoryId) => ({ id, sourceMemoryId, targetMemoryId, type_: "related", weight: 0.4, description: "" });
  const options = { ownerKey: "character:test", query: "alpha beta", folderPath: "scope", relevanceThreshold: 0.04,
    createdAtStartMs: 10, createdAtEndMs: 10 };
  for (const [scoreMode, lexicalMultiplier, edgeMultiplier] of [["BALANCED", 1, 1], ["KEYWORD_FIRST", 1.3, 0.9], ["SEMANTIC_FIRST", 0.8, 1.1]]) {
    const config = { scoreMode, keywordWeight: 2, tagWeight: 3, vectorWeight: 0, edgeWeight: 0.7 };
    const space = { memories, chunks: [], links: [link("incoming", "neighbor", "seed"), link("excluded", "seed", "outside")], searchConfig: config };
    const result = await searchStored(t, space, options, config), hits = new Map(result.candidates.map(
      /** Indexes results by their original identity for channel assertions. */
      hit => [hit.memory.id, hit],
    ));
    assert.equal(result.memoriesInScopeCount, 3);
    const keyword = 2 * 0.5 * (1.6 + 1) / 61 * lexicalMultiplier, tag = 3 * 0.5 * 1.3 / 61 * lexicalMultiplier;
    assert.ok(Math.abs(hits.get("seed").keywordScore - keyword) < 1e-12);
    assert.ok(Math.abs(hits.get("seed").tagScore - tag) < 1e-12);
    assert.ok(Math.abs(hits.get("second").keywordScore - 2 * 0.8 * 1.3 / 62 * lexicalMultiplier) < 1e-12);
    assert.ok(Math.abs(hits.get("neighbor").edgeScore - ((keyword + tag) * 0.4 + 0.03) * 0.7 * edgeMultiplier) < 1e-12);
    for (const hit of hits.values()) assert.equal(hit.passedThreshold, hit.score >= 0.04);
    assert.equal(hits.has("outside"), false); assert.equal(hits.has("expired"), false); assert.equal(hits.has("placeholder"), false);
  }
});

/** Verifies authored titles cannot act as query wildcard expressions in reverse containment. */
test("memory reverse containment treats wildcard characters in stored titles literally", async t => {
  const memory = { ...fixture().graph.items[0], title: "a*b", tags: [], isDocumentNode: false };
  const config = { scoreMode: "BALANCED", keywordWeight: 1, tagWeight: 0, vectorWeight: 0, edgeWeight: 0 };
  const result = await searchStored(t, { memories: [memory], chunks: [], links: [], searchConfig: config },
    { ownerKey: "character:test", query: "aZZb", folderPath: null, relevanceThreshold: 0, createdAtStartMs: null, createdAtEndMs: null }, config);
  assert.equal(result.candidates[0].keywordScore, 0); assert.equal(result.candidates[0].passedThreshold, false);
});

/** Verifies CJK lexical expansion retains exact single-character evidence without embeddings. */
test("memory search retains Han unigrams alongside complete words and bigrams", async t => {
  const memory = { id: "memory-north", uuid: "uuid-north", title: "北方", content: "", contentType: "text/plain", source: "manual",
    credibility: 0.5, importance: 0.8, documentPath: null, isDocumentNode: false, chunkIndexFilePath: null, folderPath: null,
    createdAt: 1, updatedAt: 1, lastAccessedAt: 1, tags: [], properties: [] };
  const config = { scoreMode: "BALANCED", keywordWeight: 1, tagWeight: 0, vectorWeight: 0, edgeWeight: 0 };
  const space = { memories: [memory], chunks: [], links: [], searchConfig: config };
  const result = await searchStored(t, space, { ownerKey: "character:test", query: "北京", folderPath: null,
    relevanceThreshold: 0.01, createdAtStartMs: null, createdAtEndMs: null }, config);
  assert.equal(result.candidates[0].memory.id, memory.id); assert.ok(result.candidates[0].keywordScore > 0);
});

/** Verifies duplicate-title merging creates a fresh identity and preserves only valid redirected graph edges. */
test("canonical memory merge resolves duplicate titles and redirects external links", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }), { DatabaseCharacterRepository } = loadModule("src/storage/database.ts", disk.globals);
  const repository = await DatabaseCharacterRepository.open(), state = plain(disk.readState()), space = state.owners[0];
  /** Builds complete existing source records with stable identities. */
  const record = (id, uuid, title) => ({ id, uuid, title, content: "source", contentType: "text/plain", source: "import",
    credibility: 0.5, importance: 0.5, documentPath: null, isDocumentNode: false, chunkIndexFilePath: null, folderPath: null,
    createdAt: 1, updatedAt: 1, lastAccessedAt: 1, tags: [], properties: [] });
  space.memories.push(record("one", "uuid-one", "Duplicate"), record("two", "uuid-two", "Duplicate"), record("three", "uuid-three", "External"));
  Object.assign(space.memories[1], { isDocumentNode: true, documentPath: "plugin:/source.txt", chunkIndexFilePath: "plugin:/source-index.json" });
  space.chunks.push({ id: "source-chunk", memoryUuid: "uuid-two", chunkIndex: 0, content: "Merged source document" });
  space.links.push({ id: "external", sourceMemoryId: "one", targetMemoryId: "three", type_: "related", weight: 0.5, description: "keep" },
    { id: "internal", sourceMemoryId: "one", targetMemoryId: "two", type_: "related", weight: 0.5, description: "remove" });
  disk.replaceState(state);
  const merged = await repository.run(session => session.mergeMemories("character:default", ["Duplicate"], { title: "Combined", content: "joined",
    contentType: "text/plain", source: "merged_from_memory", credibility: 0.5, importance: 0.5, folderPath: null, tags: [] }));
  const saved = plain(disk.readState()).owners[0];
  assert.notEqual(merged.id, "one"); assert.notEqual(merged.id, "two"); assert.equal(merged.source, "merged_from_memory");
  assert.deepEqual(plain(saved.memories.map(item => item.title).sort()), ["Combined", "External"]);
  assert.equal(saved.chunks.length, 0); assert.equal(merged.isDocumentNode, false);
  assert.deepEqual(plain(saved.links.map(link => ({ id: link.id, sourceMemoryId: link.sourceMemoryId, targetMemoryId: link.targetMemoryId, description: link.description }))),
    [{ id: "external", sourceMemoryId: merged.id, targetMemoryId: "three", description: "keep" }]);
});

import assert from "node:assert/strict";
import test from "node:test";
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
  Object.assign(values, { avatarUri: "vfs:/avatar.png", ttsConfigId: "tts-one", chatModelBindingMode: "FIXED_MODEL", chatModelId: "qwen-plus", otherContentVoice: "语音内容", advancedCustomPrompt: "完整 prompt", marks: "完整备注" });
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
  assert.throws(() => validation.assertCard(incompleteCard), /工具访问配置/);
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
  assert.deepEqual(plain(plan.card), { ...record, id: "", isDefault: false, createdAt: 0, updatedAt: 0 });
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
  assert.throws(() => domain.parseDomainPayload("memory.settings.write", { ownerKey: "shared:shared-main", settings: { ...fixture().settings, profileAutoUpdateEnabled: "true" } }), /boolean/);
  assert.throws(() => domain.parseDomainPayload("character.list", { unexpected: true }), /unexpected|unknown|unsupported/i);
  assert.throws(() => domain.jsonValue({ result: undefined }), /JSON object/);
});

/** Injects only a rejected dependency; it supplies no successful host, record, or production adapter. */
test("service dependency failures propagate unchanged instead of becoming empty snapshots", async () => {
  const api = loadModule("src/service.ts"), failure = new Error("PERSISTENCE_NOT_CONNECTED_TEST"), reads = [];
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

/** Checks the current decimal-string record contract without claiming legacy data compatibility. */
test("record identifiers retain canonical decimal strings and reject numeric or malformed values", () => {
  const item = fixture().graph.items[0], graph = fixture().graph.graph;
  item.id = "9223372036854775807"; graph.edges[0].id = "9223372036854775807";
  validation.assertMemory(item); validation.assertGraph(graph);
  assert.equal(item.id, "9223372036854775807"); assert.equal(graph.edges[0].id, "9223372036854775807");
  for (const invalid of [1, Number("9223372036854775807"), "01", "-1", "1.0", "", null]) {
    item.id = invalid; graph.edges[0].id = invalid;
    assert.throws(() => validation.assertMemory(item), { message: "记忆数据库标识 must be a canonical decimal string" });
    assert.throws(() => validation.assertGraph(graph), { message: "图谱关系标识 must be a canonical decimal string" });
  }
  for (const invalid of ["0", "9223372036854775808"]) {
    item.id = invalid; graph.edges[0].id = invalid;
    assert.throws(() => validation.assertMemory(item), /outside its i64 range/);
    assert.throws(() => validation.assertGraph(graph), /outside its i64 range/);
  }
});

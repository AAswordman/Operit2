import assert from "node:assert/strict";
import test from "node:test";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import vm from "node:vm";
import { createDiskHarness } from "./disk-files.mjs";
import { loadModule, plain } from "./runtime.mjs";

/** Builds only the explicitly labelled generic lifecycle event input, not a production source or result. */
function event(changes = {}) {
  return { eventName: "before_create", creationKind: "new", chat: { id: "draft-chat", title: "Actual creation draft", workspaceId: null, parentChatId: null }, sourceChatId: null, sourceMessageTimestamp: null, input: null, sourceExtension: null, ...changes };
}
/** Connects the pure initializer to actual file-backed active/card/group operations in this test without emulating records. */
async function environment(t) {
  const disk = await createDiskHarness(t);
  const runtime = loadModule("src/service-runtime.ts", disk.globals);
  const helper = loadModule("src/chat-bindings.ts");
  const lifecycle = loadModule("src/chat-lifecycle.ts");
  const calls = [];
  const reader = {
    /** Reads the genuinely persisted active selection through the same real service runtime. */
    async readActiveSelection() { calls.push("active"); return helper.selectionForActive(await runtime.dispatchDomain("activePrompt.get", {})); },
    /** Validates against actual persisted card/group records with the authoritative backend helper. */
    async requireSelection(selection) { calls.push(selection); helper.requireChatSelection(selection, await runtime.dispatchDomain("character.list", {}), await runtime.dispatchDomain("group.list", {})); },
  };
  return { disk, runtime, lifecycle, reader, calls };
}

/** Proves source-free creation uses a genuine persisted active record instead of hardcoding a default identity. */
test("source-free lifecycle initialization reads the actual persisted active selection", async t => {
  const runtime = await environment(t);
  const card = await runtime.runtime.dispatchDomain("character.create", { values: { name: "Actual first-create active role" } });
  await runtime.runtime.dispatchDomain("activePrompt.setCard", { id: card.id });
  assert.deepEqual(plain(await runtime.lifecycle.initializeChatExtension(event(), runtime.reader)), { extension: { version: 1, selection: "card:" + card.id } });
  assert.deepEqual(runtime.calls, ["active", "card:" + card.id]);
});
/** Proves one precise plugin-scoped input overrides source without interpreting any sibling package selection. */
test("explicit scoped selection validates a real record and does not read source or active selection", async t => {
  const runtime = await environment(t);
  const card = await runtime.runtime.dispatchDomain("character.create", { values: { name: "Explicit plugin choice" } });
  const input = { ...plain(runtime.lifecycle.createChatInput("card:" + card.id)), "com.example.sibling": { version: 7, roleId: "opaque-other-owner" } };
  const result = await runtime.lifecycle.initializeChatExtension(event({ input, sourceChatId: "other-chat", sourceExtension: { version: 1, selection: "card:does-not-exist" } }), runtime.reader);
  assert.deepEqual(plain(result), { extension: { version: 1, selection: "card:" + card.id } });
  assert.deepEqual(runtime.calls, ["card:" + card.id]);
});
/** Treats other packages' input as unrelated generic data, not a character-card command. */
test("sibling-only input does not select another package role and still requires actual active state", async t => {
  const runtime = await environment(t);
  const card = await runtime.runtime.dispatchDomain("character.create", { values: { name: "Active isolated namespace role" } });
  await runtime.runtime.dispatchDomain("activePrompt.setCard", { id: card.id });
  const result = await runtime.lifecycle.initializeChatExtension(event({ input: { "org.other.plugin": { version: 1, selection: "card:foreign" } } }), runtime.reader);
  assert.deepEqual(plain(result), { extension: { version: 1, selection: "card:" + card.id } });
  assert.deepEqual(runtime.calls, ["active", "card:" + card.id]);
});
/** Inherits complete actual source namespace data for branch creation without changing active selection. */
test("branch lifecycle validates and copies the full isolated source marker", async t => {
  const runtime = await environment(t);
  const card = await runtime.runtime.dispatchDomain("character.create", { values: { name: "Branch source role" } });
  const sourceExtension = { version: 1, selection: "card:" + card.id, memoryPreferences: { notes: ["real caller namespace data", null, true] } };
  const result = await runtime.lifecycle.initializeChatExtension(event({ creationKind: "branch", chat: { id: "branch-chat", title: "Branch", workspaceId: "workspace-one", parentChatId: "source-chat" }, sourceChatId: "source-chat", sourceMessageTimestamp: 1720000000000, sourceExtension }), runtime.reader);
  assert.deepEqual(plain(result), { extension: sourceExtension });
  assert.notEqual(result.extension, sourceExtension); assert.deepEqual(runtime.calls, ["card:" + card.id]);
});
/** Rejects explicit missing source linkage instead of replacing it with active selection. */
test("an explicit source with null namespace rejects and never reads active state", async t => {
  const runtime = await environment(t);
  await assert.rejects(runtime.lifecycle.initializeChatExtension(event({ sourceChatId: "missing-associated-source" }), runtime.reader), /has no character namespace extension/);
  assert.deepEqual(runtime.calls, []); assert.equal(runtime.disk.calls.length, 0);
});
/** Rejects malformed explicit markers before consulting the genuine source or active records. */
test("malformed own input rejects without attempting source or active selection", async t => {
  const runtime = await environment(t);
  for (const marker of [null, {}, { version: 2, selection: "card:any" }, { version: 1, selection: "card:any", ownerId: "forged" }, { version: 1, selection: "other:any" }]) {
    await assert.rejects(runtime.lifecycle.initializeChatExtension(event({ input: { "com.operit.character_cards": marker }, sourceChatId: "source", sourceExtension: { version: 1, selection: "card:source" } }), runtime.reader));
  }
  assert.deepEqual(runtime.calls, []); assert.equal(runtime.disk.calls.length, 0);
});
/** Rejects missing real selections and deliberately does not continue through any source or active alternative. */
test("missing explicit record rejects instead of inheriting a source or selecting active", async t => {
  const runtime = await environment(t);
  await assert.rejects(runtime.lifecycle.initializeChatExtension(event({ input: runtime.lifecycle.createChatInput("card:missing"), sourceChatId: "source", sourceExtension: { version: 1, selection: "card:default" } }), runtime.reader), /exactly one stored record/);
  assert.deepEqual(runtime.calls, ["card:missing"]);
});
/** Uses the same concrete participant requirements for explicit role-group initialization. */
test("explicit group creation validates actual persisted group participants", async t => {
  const runtime = await environment(t);
  const card = await runtime.runtime.dispatchDomain("character.create", { values: { name: "Actual group member" } });
  const group = await runtime.runtime.dispatchDomain("group.create", { values: { name: "Actual creation group", members: [{ characterCardId: card.id, orderIndex: 0 }] } });
  assert.deepEqual(plain(await runtime.lifecycle.initializeChatExtension(event({ input: runtime.lifecycle.createChatInput("group:" + group.id) }), runtime.reader)), { extension: { version: 1, selection: "group:" + group.id } });
  assert.deepEqual(runtime.calls, ["group:" + group.id]);
});
/** Propagates the original authoritative helper error; the hook must not fulfill an empty successful result. */
test("authoritative selection failures propagate unchanged", async () => {
  const lifecycle = loadModule("src/chat-lifecycle.ts"), original = new Error("Exact repository failure");
  const reader = {
    /** Throws the original error from the actual active-reader boundary under test. */
    async readActiveSelection() { throw original; },
    /** Rejects unexpected validation after an active-reader failure. */
    async requireSelection() { throw new Error("Validation must not run after the original read failure"); },
  };
  await assert.rejects(lifecycle.initializeChatExtension(event(), reader),
    /** Requires object identity so a swallowed or replaced error cannot pass. */
    error => error === original,
  );
});
/** Rejects undocumented lifecycle spellings and inconsistent source provenance before reading any record. */
test("lifecycle parser rejects invalid envelope fields and source provenance", () => {
  const lifecycle = loadModule("src/chat-lifecycle.ts");
  for (const changes of [{ eventName: "create" }, { creationKind: "clone" }, { ownerId: "forged" }, { sourceMessageTimestamp: 3 }, { sourceExtension: { version: 1, selection: "card:source" } }, { sourceChatId: "source", sourceMessageTimestamp: "3" }]) assert.throws(
    /** Passes the intentionally malformed test input to the actual plugin parser. */
    () => lifecycle.parseChatInitialization(event(changes)),
  );
});

/** Bundles the actual callback and existing service export into one test runtime, preserving their true shared repository instance. */
async function callbackEnvironment(t) {
  const require = createRequire(new URL("../../workflow/package.json", import.meta.url)), { buildSync } = require("esbuild");
  const disk = await createDiskHarness(t), module = { exports: {} };
  const result = buildSync({ absWorkingDir: fileURLToPath(new URL("../", import.meta.url)), stdin: { contents: 'export { beforeChatCreate } from "./chat-lifecycle"; export { dispatchDomain } from "./service-runtime";', resolveDir: fileURLToPath(new URL("../src", import.meta.url)), sourcefile: "chat-lifecycle-callback-check.ts", loader: "ts" }, bundle: true, format: "cjs", platform: "neutral", target: "es2020", write: false });
  assert.equal(result.outputFiles.length, 1);
  vm.runInNewContext(result.outputFiles[0].text, { module, exports: module.exports, ...disk.globals, console, TextEncoder, TextDecoder, setTimeout, clearTimeout });
  return { disk, ...module.exports };
}
/** Executes the real exported handler with the genuine SDK envelope and authoritative persisted active/card operations. */
test("real beforeChatCreate callback initializes its namespace through one authoritative service runtime", async t => {
  const runtime = await callbackEnvironment(t);
  const card = await runtime.dispatchDomain("character.create", { values: { name: "Real hook active role" } });
  await runtime.dispatchDomain("activePrompt.setCard", { id: card.id });
  const result = await runtime.beforeChatCreate({ event: "before_create", eventName: "before_create", toolPkgId: "com.operit.character_cards", eventPayload: event() });
  assert.deepEqual(plain(result), { extension: { version: 1, selection: "card:" + card.id } });
  assert.deepEqual(Object.keys(result), ["extension"], "The callback must not return an owner id or native chat fields");
});
/** Rejects source linkage errors before the real handler attempts any directory, file, or chat operation. */
test("real beforeChatCreate callback rejects missing source namespace without opening storage", async t => {
  const runtime = await callbackEnvironment(t);
  await assert.rejects(runtime.beforeChatCreate({ event: "before_create", eventName: "before_create", eventPayload: event({ sourceChatId: "source-chat", sourceExtension: null }) }), /has no character namespace extension/);
  assert.equal(runtime.disk.calls.length, 0);
});

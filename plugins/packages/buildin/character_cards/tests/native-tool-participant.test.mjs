import assert from "node:assert/strict";
import test from "node:test";
import { createDiskHarness } from "./disk-files.mjs";
import { loadModule, plain } from "./runtime.mjs";
import { createNativeToolRuntime, nativeToolParameters, toolModuleScript } from "./native-tool-call-runtime.mjs";

const rejectedAlias = "__operit_package_caller_card_id";

/** Opens the production domain service with real disk IO and captures only its existing IPC transport. */
async function environment(t) {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }), service = loadModule("src/service-runtime.ts", disk.globals);
  const operations = [];
  const toolPkg = { ...disk.globals.ToolPkg, ipc: {
    /** Forwards the exact authenticated main-runtime envelope to the real domain dispatcher without fabricating business data. */
    async call(channel, request, options) {
      assert.equal(channel, "character-memory.domain");
      assert.deepEqual(plain(options), { targetRuntime: "main" });
      operations.push(plain(request));
      return service.dispatchDomain(request.operation, request.input);
    },
  } };
  return { disk, service, operations, native: createNativeToolRuntime({ ...disk.globals, ToolPkg: toolPkg }, toolPkg.ipc.call) };
}

/** Exercises the actual native getter at each call's entry, then retains its captured identity without assuming Rust job scheduling in a Node VM. */
test("embedded JS transport: entry-time identity capture retains concurrent participants without a card getter", async () => {
  const native = createNativeToolRuntime({});
  const script = `
    /** Observes the authenticated per-call runtime without parsing package or card identities. */
    module.exports.inspect = async function(input) {
      var before = getCallerParticipantId();
      await Promise.resolve();
      return { captured: before, input, chatId: getChatId(), cardGetter: typeof getCallerCardId };
    };
  `;
  const [left, right] = await Promise.all([
    native.invoke(script, "inspect", nativeToolParameters("opaque.left")),
    native.invoke(script, "inspect", nativeToolParameters("opaque.right")),
  ]);
  assert.deepEqual(structuredClone(left), { captured: "opaque.left", input: {}, chatId: "fixture.chat", cardGetter: "undefined" });
  assert.deepEqual(structuredClone(right), { captured: "opaque.right", input: {}, chatId: "fixture.chat", cardGetter: "undefined" });
  assert.equal(native.terminal.length, 2);
});

/** Reads authenticated SDK identity without consulting even forged tool input fields. */
test("production caller decoder uses native getters independently of business parameters", () => {
  let participant = "opaque.participant";
  const contract = loadModule("src/runtime-tools/contract.ts", {
    getChatId: () => "fixture.chat", getCallerParticipantId: () => participant, getCallerName: () => "Fixture Speaker",
  });
  assert.deepEqual(plain(contract.callerContext({ __operit_package_caller_participant_id: "forged" })), {
    chatId: "fixture.chat", participantId: "opaque.participant", callerName: "Fixture Speaker",
  });
  participant = 9;
  assert.throws(
    /** Supplies an actually malformed native identity instead of relying on a missing-field substitute. */
    () => contract.callerContext(), /must be a string/,
  );
  participant = " ";
  assert.throws(
    /** Rejects a supplied blank participant before any memory operation can run. */
    () => contract.callerContext(), /must not be blank/,
  );
});

/** Proves the real tool export now receives the authenticated identity through native JavaScript and the actual disk-backed domain IPC. */
test("embedded JS plus disk-backed IPC: memory owner resolution receives the native participant without explicit caller_card_id", async t => {
  const actual = await environment(t), script = toolModuleScript("src/runtime-tools/tools.ts");
  assert.equal(await actual.native.invoke(script, "get_memory_owner_key", nativeToolParameters("default")), "character:default");
  assert.deepEqual(actual.operations, [{ operation: "memory.resolveOwner", input: { characterId: "default" } }]);
  assert.equal(actual.native.terminal.length, 1);
});

/** Keeps the tool-specific business argument without turning it into a transport alias or letting it override the authenticated participant. */
test("embedded JS transport: caller_card_id remains an explicit business parameter and mismatched native identity fails", async t => {
  const actual = await environment(t), script = toolModuleScript("src/runtime-tools/tools.ts");
  assert.equal(await actual.native.invoke(script, "get_memory_owner_key", { ...nativeToolParameters("default"), caller_card_id: "default" }), "character:default");
  const count = actual.operations.length;
  await assert.rejects(actual.native.invoke(script, "get_memory_owner_key", { ...nativeToolParameters("default"), caller_card_id: "other" }), /does not match the authenticated execution participant/);
  assert.equal(actual.operations.length, count, "Invalid identity must fail before the repository is invoked");
  assert.deepEqual(actual.native.terminal.map(
    /** Distinguishes original failed execution from a successful null or empty result. */
    item => item.type,
  ), ["result", "error"]);
});

/** Uses the complete real native runtime envelope so the whitelist cannot accidentally reject legitimate injected fields. */
test("embedded JS plus disk-backed IPC: character list accepts the full native envelope and preserves real stored records", async t => {
  const actual = await environment(t), script = toolModuleScript("src/runtime-tools/tools.ts");
  const result = await actual.native.invoke(script, "list_character_cards", nativeToolParameters("default"));
  const records = await actual.service.dispatchDomain("character.list", {});
  assert.deepEqual(structuredClone(result), { totalCount: records.length, cards: plain(records.map(
    /** Compares the exact historical public projection against the production repository's records. */
    card => ({ id: card.id, name: card.name, description: card.description, isDefault: card.isDefault, createdAt: card.createdAt, updatedAt: card.updatedAt }),
  )) });
  assert.deepEqual(actual.operations, [{ operation: "character.list", input: {} }]);
});

/** Rejects the retired alias and undeclared business fields before issuing a successful-looking list result. */
test("embedded JS transport: the character list rejects retired and unknown parameter names", async t => {
  const actual = await environment(t), script = toolModuleScript("src/runtime-tools/tools.ts");
  await assert.rejects(actual.native.invoke(script, "list_character_cards", { ...nativeToolParameters("default"), [rejectedAlias]: "default" }), /Unknown list_character_cards parameter: __operit_package_caller_card_id/);
  await assert.rejects(actual.native.invoke(script, "list_character_cards", { ...nativeToolParameters("default"), invented_context: {} }), /Unknown list_character_cards parameter: invented_context/);
  assert.equal(actual.operations.length, 0);
});

/** Keeps real filesystem failures on the actual asynchronous native error channel after context validation succeeds. */
test("embedded JS plus disk-backed IPC: original memory and list filesystem failures reach terminal errors", async t => {
  const actual = await environment(t), script = toolModuleScript("src/runtime-tools/tools.ts");
  await actual.service.dispatchDomain("character.list", {});
  actual.disk.failNext("storage.preferences.data", new Error("participant-owner-original-file-error"));
  await assert.rejects(actual.native.invoke(script, "get_memory_owner_key", nativeToolParameters("default")), /participant-owner-original-file-error/);
  actual.disk.failNext("storage.preferences.data", new Error("participant-list-original-file-error"));
  await assert.rejects(actual.native.invoke(script, "list_character_cards", nativeToolParameters("default")), /participant-list-original-file-error/);
  assert.deepEqual(actual.native.terminal.map(
    /** Checks neither failure was converted into a successful empty list or absent owner. */
    item => item.type,
  ), ["error", "error"]);
});

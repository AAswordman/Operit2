import assert from "node:assert/strict";
import test from "node:test";
import { createDiskHarness } from "./disk-files.mjs";
import { loadModule, plain } from "./runtime.mjs";

const domain = loadModule("src/domain.ts");

/** Builds a display request with deliberately unavailable execution defaults and no invented model selection. */
function displayRequest(selection) {
  return { purpose: "display", chatId: "unpersisted-draft", chatExtension: { version: 1, selection }, participantId: null,
    promptFunctionType: "CHAT", defaultModelBinding: { providerId: "", modelId: "" }, defaultTtsConfigId: "" };
}

/** Uses actual plugin directory files without connecting execution catalogs or a host chat implementation. */
async function fileService(t) {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }), repository = await loadModule("src/storage/database.ts", disk.globals).DatabaseCharacterRepository.open();
  const service = loadModule("src/service-runtime.ts", disk.globals).createCharacterCardsService(repository);
  return { disk, service };
}

/** A real multi-role draft resolves for presentation without constructing a profile or choosing a first member. */
test("display: persisted multi-member group returns ordered identities and no execution snapshot or catalog reads", async t => {
  const { service, disk } = await fileService(t);
  const card = await service.dispatchDomain("character.create", { values: { name: "Second real member", avatarUri: "resources/second.png", openingStatement: "Must not be group initialization" } });
  const group = await service.dispatchDomain("group.create", { values: { name: "Real group", members: [{ characterCardId: "default", orderIndex: 8 }, { characterCardId: card.id, orderIndex: 2 }] } });
  const defaultCard = await service.dispatchDomain("character.get", { id: "default" });
  const input = domain.parseChatConfigurationRequest(displayRequest("group:" + group.id));
  const result = plain(await service.dispatchDomain("chat.configuration.resolve", input));
  assert.deepEqual(result, { contextKey: "group:" + group.id, identity: { title: "Real group", avatarUri: null },
    participants: [{ id: card.id, name: "Second real member", avatarUri: "resources/second.png" }, { id: "default", name: defaultCard.name, avatarUri: defaultCard.avatarUri }], initialMessages: [] });
  assert.equal(Object.hasOwn(result, "profile"), false); assert.equal(Object.hasOwn(result, "executionData"), false);
  assert.equal(disk.calls.some(
    /** Detects calls outside the real file boundary rather than synthesizing directory results. */
    call => call.method === "models" || call.method === "tts" || call.method === "tools",
  ), false);
});

/** A presentation request for a simple role with no initial message must not validate its execution configuration. */
test("display: card with no opening reads its actual identity without a model, TTS or memory profile", async t => {
  const { service } = await fileService(t), card = await service.dispatchDomain("character.create", { values: { name: "Identity only", avatarUri: "real-avatar.png" } });
  const result = plain(await service.dispatchDomain("chat.configuration.resolve", displayRequest("card:" + card.id)));
  assert.deepEqual(result, { contextKey: "card:" + card.id, identity: { title: card.name, avatarUri: "real-avatar.png" },
    participants: [{ id: card.id, name: card.name, avatarUri: "real-avatar.png" }], initialMessages: [] });
});

/** Purpose is a required public contract and display cannot select a historical or arbitrary execution participant. */
test("display: missing purpose and execution-only inputs are rejected before business reads", async () => {
  const request = displayRequest("group:actual-group"), missing = plain(request); delete missing.purpose;
  assert.throws(() => domain.parseChatConfigurationRequest(missing), /purpose/);
  for (const input of [{ ...request, participantId: "member" }, { ...request, executionData: { version: 1 } }]) {
    assert.throws(() => domain.parseChatConfigurationRequest(input), /Display configuration|not a supported field/);
    if (input.participantId === null) continue;
    await assert.rejects(() => domain.resolveChatConfiguration(input, {
      /** Rejects execution directory reads during a presentation-only operation. */
      async snapshot() { throw new Error("Unexpected catalog read"); },
      /** Rejects all reads because malformed input must not begin domain resolution. */
      async dispatchDomain() { throw new Error("Unexpected domain read"); },
    }), /Display configuration/);
  }
});

/** Deleted and duplicate members are rejected instead of being omitted or substituted in presentation order. */
test("display: stale group references propagate their actual read failure and never become an empty group", async () => {
  const removed = new Error("Exact missing member error"), calls = [];
  const service = {
    /** Rejects any attempt to read execution catalogs. */
    async snapshot() { throw new Error("Catalogs must not be read"); },
    /** Returns only the explicitly controlled group record. */
    async getGroup(id) {
      calls.push("group.get");
      return { id, name: "Reference group", members: [{ characterCardId: "removed", orderIndex: 0 }] };
    },
    /** Preserves the exact missing-record failure from the keyed character read. */
    async getCharacter() { calls.push("character.get"); throw removed; },
  };
  await assert.rejects(() => domain.resolveChatConfiguration(displayRequest("group:one"), service),
    /** Requires the same failure object rather than a rewritten default-role error. */
    error => error === removed,
  );
  assert.deepEqual(calls, ["group.get", "character.get"]);
});

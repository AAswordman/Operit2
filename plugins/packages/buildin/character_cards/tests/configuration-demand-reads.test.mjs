import assert from "node:assert/strict";
import test from "node:test";
import { createDiskHarness } from "./disk-files.mjs";
import { loadModule, plain } from "./runtime.mjs";

const directoryInput = plain(loadModule("tests/host-directories-input.ts").hostDirectoriesTestInput);

/** Opens the production repository over real SQLite and records each independent directory read. */
async function environment(t) {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }), directories = plain(directoryInput), calls = [];
  const repository = await loadModule("src/storage/database.ts", disk.globals).DatabaseCharacterRepository.open({
    /** Returns the explicitly supplied model catalog. */
    async listModels() { calls.push("models"); return plain(directories.models); },
    /** Returns the explicitly supplied speech catalog. */
    async listTtsConfigs() { calls.push("tts"); return plain(directories.ttsConfigs); },
    /** Records tool reads so execution tests can reject unnecessary catalog loads. */
    async readToolCatalog() { calls.push("tools"); return plain(directories.toolCatalog); },
  });
  const service = loadModule("src/service-runtime.ts", disk.globals).createCharacterCardsService(repository);
  /** Discards setup reads while retaining the actual database and all service state. */
  function reset() { disk.clearCalls(); calls.length = 0; }
  /** Resolves through the real service dispatcher and its live privacy check. */
  function resolve(input) { return service.dispatchDomain("chat.configuration.resolve", input); }
  reset();
  return { disk, service, directories, calls, reset, resolve };
}

/** Supplies the exact public execution contract with explicit model and speech defaults. */
function request(selection, extra = {}) {
  return { purpose: "execution", chatId: null, chatExtension: { version: 1, selection }, participantId: null,
    promptFunctionType: "CHAT", defaultModelBinding: { providerId: directoryInput.models[0].providerId, modelId: directoryInput.models[0].modelId },
    defaultTtsConfigId: directoryInput.ttsConfigs[0].id, ...extra };
}

/** Projects real object reads for independent resource-use assertions. */
function queries(disk) {
  return disk.calls.filter(call => ["storage.preferences.data", "storage.get", "storage.list", "storage.getMany"].includes(call.method));
}
/** Counts exact collection reads and preserves their explicit identities. */
function reads(disk, collection) {
  return queries(disk).filter(call => call.method === "storage.preferences.data" && call.args[0].endsWith("/" + collection + ".preferences.json")).map(call => ({ values: [] }));
}

/** A simple role must not pay for full directories or repeated role and memory-owner reads. */
test("configuration reads: an untagged character uses 12 storage queries and no unrelated directories", async t => {
  const h = await environment(t), result = plain(await h.resolve(request("card:default")));
  assert.equal(result.profile.id, "default");
  assert.equal(result.executionData.primaryOwnerKey, "character:default");
  assert.deepEqual(h.calls, ["models", "tts"]);
  assert.equal(queries(h.disk).some(call => ["memories", "embeddings", "candidates"].includes(call.args[0])), false);
  assert.deepEqual(reads(h.disk, "cards").map(
    /** Requires a single primary-key read of the selected role. */
    query => query.values,
  ), [[]]);
  for (const table of ["groups", "tags", "stores"]) assert.equal(reads(h.disk, table).length, 0, table);
});

/** Model conversations need model and speech validation, while presentation needs only model identity. */
test("configuration reads: model execution and display never load character storage or tools", async t => {
  const h = await environment(t);
  await h.resolve(request("model:chat"));
  assert.deepEqual(h.calls, ["models", "tts"]);
  assert.equal(queries(h.disk).length, 1, "Only the final live privacy flag read is required");
  h.reset();
  const result = plain(await h.resolve(request("model:chat", { purpose: "display" })));
  assert.equal(result.identity.title, directoryInput.models[0].modelId);
  assert.deepEqual(h.calls, ["models"]);
  assert.equal(queries(h.disk).length, 0);
});

/** An opening message shares the selected role with its strict execution resolution. */
test("configuration reads: an authored opening reuses its display character read", async t => {
  const h = await environment(t);
  const card = await h.service.dispatchDomain("character.create", { values: { name: "Opening author", openingStatement: "Hello" } });
  h.reset();
  const result = plain(await h.resolve(request("card:" + card.id, { purpose: "display" })));
  assert.equal(result.initialMessages[0].content, "Hello");
  assert.equal(reads(h.disk, "cards").length, 1);
  assert.deepEqual(h.calls, []);
  assert.deepEqual(result.initialMessages[0], { content: "Hello", displayName: "Opening author", participantId: card.id });
});

/** Group members share only demanded dependencies and retain prompt, resource and member ordering. */
test("configuration reads: group members reuse exact tags, stores and catalogs within one request", async t => {
  const h = await environment(t);
  const tag = await h.service.dispatchDomain("tag.create", { values: { name: "Shared tag", promptContent: "Exact tag content" } });
  const store = await h.service.dispatchDomain("memory.shared.create", { name: "Shared library" });
  const a = await h.service.dispatchDomain("character.create", { values: { name: "A", characterSetting: "Setting A", otherContentChat: "Chat A", otherContentVoice: "Voice A", advancedCustomPrompt: "Advanced A", attachedTagIds: [tag.id], memoryBindingMode: "SHARED", sharedMemoryId: store.id,
    sharedMemoryMounts: [{ sharedMemoryId: store.id, readable: true, writable: false }] } });
  const b = await h.service.dispatchDomain("character.create", { values: { name: "B", attachedTagIds: [tag.id], sharedMemoryMounts: [{ sharedMemoryId: store.id, readable: true, writable: false }] } });
  const group = await h.service.dispatchDomain("group.create", { values: { name: "Ordered group", members: [{ characterCardId: a.id, orderIndex: 9 }, { characterCardId: b.id, orderIndex: 2 }] } });
  h.reset();
  const result = plain(await h.resolve(request("group:" + group.id, { participantId: a.id, promptFunctionType: "VOICE" })));
  assert.deepEqual(result.participants.map(
    /** Preserves persisted member order independently of the chosen executor. */
    participant => participant.id,
  ), [b.id, a.id]);
  assert.ok(result.profile.introPrompt.startsWith("Setting A\n\nVoice A\n\nExact tag content\n\nAdvanced A"));
  assert.equal(result.executionData.primaryOwnerKey, "shared:" + store.id);
  assert.deepEqual(result.profile.resources, [{ key: "shared:" + store.id, readable: true, writable: true }]);
  assert.equal(reads(h.disk, "cards").length, 2);
  assert.equal(reads(h.disk, "groups").length, 1);
  assert.deepEqual(reads(h.disk, "tags").map(
    /** Verifies a single referenced tag read rather than a full tag scan. */
    query => query.values,
  ), [[]]);
  assert.deepEqual(reads(h.disk, "stores").map(
    /** Verifies a single referenced shared library read despite repeated mounts. */
    query => query.values,
  ), [[]]);
  assert.deepEqual(h.calls, ["models", "tts"]);
});

/** Request-local reuse must never retain stale role or prompt data across sends. */
test("configuration reads: the next request observes edits to its character and attached tag", async t => {
  const h = await environment(t);
  const tag = await h.service.dispatchDomain("tag.create", { values: { name: "Editable", promptContent: "Before tag" } });
  const card = await h.service.dispatchDomain("character.create", { values: { name: "Before name", attachedTagIds: [tag.id] } });
  await h.resolve(request("card:" + card.id));
  await h.service.dispatchDomain("character.update", { id: card.id, changes: { name: "After name" } });
  await h.service.dispatchDomain("tag.update", { id: tag.id, changes: { promptContent: "After tag" } });
  h.reset();
  const result = plain(await h.resolve(request("card:" + card.id)));
  assert.equal(result.profile.name, "After name");
  assert.match(result.profile.introPrompt, /After tag/);
  assert.equal(reads(h.disk, "cards").length, 1);
  assert.equal(reads(h.disk, "tags").length, 1);
});

/** Missing and ambiguous dependencies remain hard errors after removing the aggregate snapshot. */
test("configuration reads: exact model, speech, tag and shared-library validation is retained", async t => {
  const h = await environment(t), input = request("card:default");
  await assert.rejects(() => h.resolve({ ...input, defaultModelBinding: { providerId: "removed", modelId: "removed" } }), /Model binding unavailable/);
  await assert.rejects(() => h.resolve({ ...input, defaultTtsConfigId: "removed" }), /TTS configuration not found/);
  h.directories.models.push(plain(h.directories.models[0]));
  await assert.rejects(() => h.resolve(input), /Model binding unavailable or duplicated/);
  h.directories.models.pop();
  const state = h.disk.readState();
  state.cards[0].attachedTagIds = ["removed-tag"];
  h.disk.replaceState(state);
  await assert.rejects(() => h.resolve(input), /Tag not found: removed-tag/);
  state.cards[0].attachedTagIds = [];
  state.cards[0].sharedMemoryMounts = [{ sharedMemoryId: "removed-store", readable: true, writable: false }];
  h.disk.replaceState(state);
  await assert.rejects(() => h.resolve(input), /Shared memory not found: removed-store/);
});

/** Every resolution uses current configuration and performs no SQL mutations after initialization. */
test("configuration reads: current role and privacy settings are used without any writes", async t => {
  const h = await environment(t);
  await h.resolve(request("card:default"));
  await h.service.dispatchDomain("character.update", { id: "default", changes: { characterSetting: "Current changed prompt" } });
  await h.service.dispatch({ action: "writeGlobalMemoryPreference", key: "userDescriptionEnabled", value: false });
  h.reset();
  const result = plain(await h.resolve(request("card:default", { participantId: "default" })));
  assert.match(result.profile.introPrompt, /Current changed prompt/);
  assert.equal(result.profile.userPreferencesText, "");
  assert.equal(h.disk.calls.filter(call => ["storage.execute", "storage.commit", "storage.commit"].includes(call.method)).length, 0);
  assert.equal(reads(h.disk, "cards").length, 1);
  await assert.rejects(async () => h.resolve({ ...request("card:default"), executionData: result.executionData }), /not a supported field/);
});

/** A privacy change during file IO must still be applied after all participant reads finish. */
test("configuration reads: the final privacy check observes a flag changed during resolution", async t => {
  const h = await environment(t);
  await h.service.dispatchDomain("memory.user.write", { ownerKey: "character:default", content: "Current private profile" });
  const port = {
    ...h.service,
    /** Changes the live preference after the exact document read begins. */
    async dispatchDomain(operation, input) {
      const result = await h.service.dispatchDomain(operation, input);
      if (operation === "memory.user.read") {
        await h.service.dispatch({ action: "writeGlobalMemoryPreference", key: "userDescriptionEnabled", value: false });
      }
      return result;
    },
  };
  const result = plain(await loadModule("src/operations.ts", h.disk.globals).resolveConfiguration(request("card:default"), port));
  assert.equal(result.profile.userPreferencesText, "");
  assert.equal(result.participants[0].userPreferencesText, "");
});

/** Resolves the exact legacy fixed binding even when the request's global model cannot be used. */
test("legacy FIXED_CONFIG executes its stored provider and model without rewriting the card", async t => {
  const h = await environment(t), model = h.directories.models[0];
  h.disk.storage.inspectPreferences("cards", values => Object.assign(values.record_default, {
    chatModelBindingMode: "FIXED_CONFIG", chatProviderId: model.providerId, chatModelId: model.modelId,
  }));
  const before = h.disk.stateBytes();
  const result = plain(await h.resolve(request("card:default", { defaultModelBinding: { providerId: "unused-global-provider", modelId: "unused-global-model" } })));
  assert.deepEqual(result.profile.modelBinding, { providerId: model.providerId, modelId: model.modelId });
  assert.deepEqual(h.disk.stateBytes(), before);
});

/** Keeps damaged selections local to their execution while allowing content edits and an explicit replacement binding. */
test("invalid installed bindings allow ordinary edits and explicit rebinding through the real service", async t => {
  for (const binding of [
    { chatModelBindingMode: "UNRECOGNIZED_MODE", chatModelId: "old-model", chatProviderId: "old-provider" },
    { chatModelBindingMode: "FIXED_CONFIG", chatModelId: "old-model", chatProviderId: null },
    { chatModelBindingMode: "FIXED_MODEL", chatModelId: null, chatProviderId: null },
  ]) {
    const h = await environment(t), model = h.directories.models[0];
    h.disk.storage.inspectPreferences("cards", values => Object.assign(values.record_default, binding));
    const saved = await h.service.dispatchDomain("character.update", { id: "default", changes: { description: "Content remains editable" } });
    assert.equal(saved.description, "Content remains editable");
    for (const [key, value] of Object.entries(binding)) assert.equal(saved[key], value);
    const other = await h.service.dispatchDomain("character.create", { values: { name: "Working unaffected role" } });
    assert.equal((await h.resolve(request("card:" + other.id))).profile.id, other.id);
    const failure = binding.chatModelBindingMode === "UNRECOGNIZED_MODE" ? /模型绑定模式无效/ : /profile.fixedModelId|Fixed model provider must be explicitly selected/;
    await assert.rejects(h.resolve(request("card:default")), failure);
    const before = h.disk.stateBytes();
    await assert.rejects(h.service.dispatchDomain("character.update", { id: "default", changes: { chatModelBindingMode: "FIXED_MODEL", chatProviderId: "explicit-provider", chatModelId: null } }), /固定聊天模型/);
    assert.deepEqual(h.disk.stateBytes(), before);
    const repaired = await h.service.dispatchDomain("character.update", { id: "default", changes: {
      chatModelBindingMode: "FIXED_MODEL", chatProviderId: model.providerId, chatModelId: model.modelId,
    } });
    assert.equal(repaired.chatModelBindingMode, "FIXED_MODEL");
    const result = plain(await h.resolve(request("card:default")));
    assert.deepEqual(result.profile.modelBinding, { providerId: model.providerId, modelId: model.modelId });
  }
});

/** Opens damaged selections for editing while requiring a real library for new bindings and execution. */
test("missing primary memory libraries stay editable and require explicit valid rebinding", async t => {
  for (const sharedId of [null, "", "  ", "deleted-store"]) {
    const h = await environment(t);
    h.disk.storage.inspectPreferences("cards", values => Object.assign(values.record_default, {
      memoryBindingMode: "SHARED", sharedMemoryId: sharedId,
    }));
    const initial = h.disk.stateBytes();
    await loadModule("src/storage/database.ts", h.disk.globals).DatabaseCharacterRepository.open();
    assert.deepEqual(h.disk.stateBytes(), initial);
    const listed = await h.service.dispatchDomain("character.list", {});
    assert.equal(listed.find(card => card.id === "default").sharedMemoryId, sharedId);
    const saved = await h.service.dispatchDomain("character.update", { id: "default", changes: { description: "Memory choice stays editable" } });
    assert.equal(saved.description, "Memory choice stays editable");
    assert.equal(saved.memoryBindingMode, "SHARED"); assert.equal(saved.sharedMemoryId, sharedId);
    const other = await h.service.dispatchDomain("character.create", { values: { name: "Unaffected role" } });
    assert.equal((await h.resolve(request("card:" + other.id))).profile.id, other.id);
    await assert.rejects(h.resolve(request("card:default")), /profile.sharedMemoryId|Shared memory not found/);
    const before = h.disk.stateBytes();
    await assert.rejects(h.service.dispatchDomain("character.create", { values: {
      name: "Invalid new role", memoryBindingMode: "SHARED", sharedMemoryId: sharedId,
    } }), /共享记忆库/);
    await assert.rejects(h.service.dispatchDomain("character.update", { id: "default", changes: { sharedMemoryId: "another-deleted-store" } }), /共享记忆库/);
    assert.deepEqual(h.disk.stateBytes(), before);
    const store = await h.service.dispatchDomain("memory.shared.create", { name: "Explicitly selected library" });
    const repaired = await h.service.dispatchDomain("character.update", { id: "default", changes: { sharedMemoryId: store.id } });
    assert.equal(repaired.memoryBindingMode, "SHARED"); assert.equal(repaired.sharedMemoryId, store.id);
    assert.equal((await h.resolve(request("card:default"))).profile.id, "default");
  }
});

/** Keeps unavailable active choices readable while requiring explicit valid selection before execution. */
test("deleted active roles allow repository reopen and edits without choosing a replacement", async t => {
  for (const active of [{ CharacterCard: { id: "deleted-character" } }, { CharacterGroup: { id: "deleted-group" } }]) {
    const h = await environment(t);
    h.disk.storage.inspectPreferences("meta", values => { values.state.active = structuredClone(active); });
    const before = h.disk.stateBytes();
    await loadModule("src/storage/database.ts", h.disk.globals).DatabaseCharacterRepository.open();
    assert.deepEqual(plain(await h.service.dispatchDomain("activePrompt.get", {})), active);
    assert.deepEqual(h.disk.stateBytes(), before);
    const edited = await h.service.dispatchDomain("character.update", { id: "default", changes: { description: "Editable before reselection" } });
    assert.equal(edited.description, "Editable before reselection");
    assert.deepEqual(plain(await h.service.dispatchDomain("activePrompt.get", {})), active);
    const selection = "CharacterCard" in active ? "card:deleted-character" : "group:deleted-group";
    await assert.rejects(h.resolve(request(selection)), /Character not found|Group not found/);
    const unchanged = h.disk.stateBytes();
    await assert.rejects(h.service.dispatchDomain("activePrompt.setCard", { id: "deleted-character" }), /character does not identify exactly one record/);
    await assert.rejects(h.service.dispatchDomain("activePrompt.setGroup", { id: "deleted-group" }), /group does not identify exactly one record/);
    await assert.rejects(h.service.dispatchDomain("activePrompt.resolvedCard", {}), /character does not identify exactly one record|requires an explicit execution participant/);
    assert.deepEqual(h.disk.stateBytes(), unchanged);
    await h.service.dispatchDomain("activePrompt.setCard", { id: "default" });
    assert.deepEqual(plain(await h.service.dispatchDomain("activePrompt.get", {})), { CharacterCard: { id: "default" } });
    assert.equal((await h.resolve(request("card:default"))).profile.id, "default");
  }
});

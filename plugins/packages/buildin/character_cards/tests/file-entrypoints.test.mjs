import assert from "node:assert/strict";
import test from "node:test";
import path from "node:path";
import { readFile, writeFile } from "node:fs/promises";
import { createDiskHarness } from "./disk-files.mjs";
import { openPlugin } from "./plugin-entry-harness.mjs";
import { createSoftwareSettingsFixture } from "./software-settings-fixture.mjs";
import { loadModule, plain } from "./runtime.mjs";

// These explicit typed directory inputs isolate unit-test dependencies; they are not native-host configuration evidence.
const directoryInput = plain(loadModule("tests/software-settings-input.ts").softwareSettingsTestInput);
const snapshotDirectoryMethods = ["listModelSummaries", "listTtsConfigs", "readToolSourceCatalog"];

/** Requires the real main callbacks to call exactly the declared SDK methods with their actual no-argument contract. */
function assertDirectoryCalls(settings, methods) {
  assert.deepEqual(settings.calls, methods.map(
    /** Preserves each expected actual callback invocation, including its order and zero arguments. */
    method => ({ method, args: [] }),
  ));
}

/** Compares the complete returned snapshot with independently read authoritative disk state and explicit host-test inputs. */
function assertCompleteSnapshot(snapshot, state) {
  assert.deepEqual(plain(snapshot), {
    cards: state.cards, groups: state.groups, stores: state.stores, tags: state.tags, active: state.active,
    models: directoryInput.models, ttsConfigs: directoryInput.ttsConfigs, toolCatalog: directoryInput.toolCatalog,
  });
}

/** Rejects every file mutation rather than counting only successful state publications. */
function assertNoMutations(disk) {
  assert.deepEqual(disk.calls.filter(
    /** Matches the exact declared file mutation methods without guessing paths or API names. */
    call => call.method === "write" || call.method === "move" || call.method === "mkdir" || call.method === "deleteFile",
  ), []);
}

/** Publishes through actual registered providers, reads through actual Web IPC, and reopens a separate runtime owner. */
test("registered API command and Web IPC share actual file-backed character records across restart", async t => {
  const disk = await createDiskHarness(t), plugin = openPlugin(disk);
  assert.equal(disk.calls.length, 0, "Registration must not initialize or synthesize business data");
  const created = await plugin.api("character.create", { values: { name: "真实 API 创建", description: "完整描述", otherContentVoice: "语音附加内容", advancedCustomPrompt: "完整高级提示", marks: "完整备注", toolAccessConfig: { enabled: true, allowedBuiltinTools: ["explicit-builtin", "use_package"], allowedPackages: ["explicit-package"], allowedSkills: ["explicit-skill"], allowedMcpServers: ["explicit-mcp"] } } });
  const read = await plugin.web({ action: "getCharacter", id: created.id });
  assert.deepEqual(plain(read), plain(created));
  const changed = await plugin.command("character", ["update", created.id, "description", "实际命令修改"]);
  assert.equal(Object.hasOwn(changed, "stderr"), false, JSON.stringify(plain(changed)));
  const viaApi = await plugin.api("character.get", { id: created.id });
  assert.equal(viaApi.description, "实际命令修改"); assert.equal(viaApi.otherContentVoice, created.otherContentVoice);
  assert.deepEqual(plain(await plugin.web({ action: "getCharacter", id: created.id })), plain(viaApi));
  const file = path.join(disk.directory, "character-memory", "state.json"), state = JSON.parse(await readFile(file, "utf8"));
  assert.deepEqual(state.cards.find(
    /** Locates the actual generated identity in the independently read authoritative bytes. */
    card => card.id === created.id,
  ), plain(viaApi));
  const reopened = openPlugin(disk);
  assert.deepEqual(plain(await reopened.api("character.get", { id: created.id })), plain(viaApi));
  assert.deepEqual(plain(await reopened.web({ action: "getCharacter", id: created.id })), plain(viaApi));
});

/** Executes actual retained initialization errors through all entrypoints without a successful domain Host or fixture. */
test("registered API command and Web IPC preserve a bad-file initialization failure without partial writes", async t => {
  const disk = await createDiskHarness(t), files = loadModule("src/storage/files.ts", disk.globals);
  await files.FileCharacterRepository.open();
  const file = path.join(disk.directory, "character-memory", "state.json"); await writeFile(file, "{broken", "utf8");
  disk.clearCalls(); const plugin = openPlugin(disk); let original;
  await assert.rejects(plugin.api("character.list", {}),
    /** Captures the actual rejected initialization error from the real provider path. */
    error => { original = error; return error.name === "SyntaxError"; },
  );
  await assert.rejects(plugin.web({ action: "getCharacter", id: "default" }),
    /** Requires the same retained service initialization failure on the actual IPC handler. */
    error => error === original,
  );
  const command = await plugin.command("character", ["list"]);
  assert.equal(command.json.ok, false); assert.equal(command.json.error.code, "domain_operation_failed");
  assert.equal(command.json.error.message, original.message);
  assert.equal(await readFile(file, "utf8"), "{broken");
  assert.equal(disk.calls.filter(
    /** Rejects any repair or new-install publication during bad-file provider calls. */
    call => call.method === "write" || call.method === "move" || call.method === "mkdir",
  ).length, 0);
});

/** Executes real main and file-backed snapshot logic with explicit SDK directory inputs, not native-host integration. */
test("registered management Web IPC preserves a full file-backed snapshot with explicit SoftwareSettings test inputs", async t => {
  const disk = await createDiskHarness(t), settings = createSoftwareSettingsFixture(disk, directoryInput), plugin = openPlugin(settings.harness);
  assert.equal(settings.calls.length, 0, "Registration must not read configured directories");
  assert.equal(disk.calls.length, 0, "Registration must not open plugin files");
  const managementRoute = "toolpkg:com.operit.character_cards:ui:main", attachmentRoute = "toolpkg:com.operit.character_cards:ui:memory-attachment";
  assert.equal(plugin.routes.length, 5);
  assert.deepEqual(plain(plugin.routes.map(
    /** Inspects the actual route identities and lifetimes without rendering either screen or synthesizing a host. */
    route => ({ id: route.id, route: route.route, runtime: route.runtime, keepAlive: route.keepAlive }),
  )), [
    { id: "main", route: managementRoute, runtime: "compose_dsl", keepAlive: true },
    { id: "memory-attachment", route: attachmentRoute, runtime: "compose_dsl", keepAlive: false },
    { id: "chat-sidebar", route: "toolpkg:com.operit.character_cards:ui:chat-sidebar", runtime: "compose_dsl", keepAlive: true },
    { id: "selection", route: "toolpkg:com.operit.character_cards:ui:selection", runtime: "compose_dsl", keepAlive: false },
    { id: "group-execution", route: "toolpkg:com.operit.character_cards:ui:group-execution", runtime: "compose_dsl", keepAlive: false },
  ]);
  assert.equal(typeof plugin.routes[0].screen, "function"); assert.equal(typeof plugin.routes[1].screen, "function");
  assert.notEqual(plugin.routes[0].screen, plugin.routes[1].screen);
  assert.equal(plugin.routes[0].screen, plugin.main.screen); assert.equal(plugin.routes[1].screen, plugin.main.attachmentScreen);
  assert.equal(plugin.routes[2].screen, plugin.main.sidebarScreen); assert.equal(plugin.routes[3].screen, plugin.main.selectionScreen); assert.equal(plugin.routes[4].screen, plugin.main.groupExecutionScreen);
  assert.equal(plugin.routes[4].screen, plugin.main.groupExecutionScreen);
  assert.equal(new Set(plugin.routes.map(
    /** Requires actual exported screen identity rather than accepting a reused editor callback. */
    entry => entry.screen,
  )).size, 5);
  assert.equal(plugin.navigation.length, 5);
  const sidebar = plugin.navigation.filter(
    /** Requires one actual sidebar entry rather than accepting any route that happens to match it. */
    entry => entry.surface === "main_sidebar_plugins",
  );
  assert.equal(sidebar.length, 1); assert.equal(sidebar[0].route, managementRoute);
  const attachment = plugin.navigation.filter(
    /** Requires exactly one formal attachment registration published by the actual main module. */
    entry => entry.surface === "chat_attachments",
  );
  assert.equal(attachment.length, 1);
  assert.deepEqual(plain(attachment[0]), { id: "memory-attachment", route: attachmentRoute, surface: "chat_attachments", title: { zh: "记忆附件", en: "Memory attachment" }, icon: "Memory", order: 150, params: { mode: "memory-attachment", ownerKey: null, folderPath: null } });
  assert.equal(Object.hasOwn(attachment[0], "action"), false);
  const snapshot = await plugin.web({ action: "snapshot" });
  assert.ok(Array.isArray(snapshot.cards)); assert.ok(Array.isArray(snapshot.models)); assert.ok(Array.isArray(snapshot.ttsConfigs));
  assert.ok(snapshot.cards.length > 0);
  const file = path.join(disk.directory, "character-memory", "state.json"), state = JSON.parse(await readFile(file, "utf8"));
  assertDirectoryCalls(settings, snapshotDirectoryMethods);
  assertCompleteSnapshot(snapshot, state);
  settings.clearCalls(); disk.clearCalls();
  assertCompleteSnapshot(await plugin.api("snapshot", {}), state);
  assertDirectoryCalls(settings, snapshotDirectoryMethods); assertNoMutations(disk);
  settings.clearCalls(); disk.clearCalls();
  const reopened = openPlugin(settings.harness);
  assertCompleteSnapshot(await reopened.web({ action: "snapshot" }), state);
  assertDirectoryCalls(settings, snapshotDirectoryMethods); assertNoMutations(disk);
});

/** Executes actual staged Web save with explicit SDK directory inputs and one real state-file publication. */
test("actual Web card save commits the complete staged tag change set once", async t => {
  const disk = await createDiskHarness(t), settings = createSoftwareSettingsFixture(disk, directoryInput), plugin = openPlugin(settings.harness);
  assert.equal(settings.calls.length, 0, "Actual main registration must remain lazy");
  const card = await plugin.api("character.create", { values: { name: "Web 原角色" } });
  const removed = await plugin.api("tag.create", { values: { name: "旧标签", description: "", promptContent: "旧提示", tagType: "CUSTOM" } });
  const updated = await plugin.api("tag.create", { values: { name: "保留标签", description: "", promptContent: "旧提示", tagType: "TONE" } });
  const before = path.join(disk.directory, "character-memory", "state.json"); disk.clearCalls();
  const snapshot = await plugin.web({ action: "saveCharacter", create: false, card: { ...plain(card), description: "Web 更新", ttsConfigId: directoryInput.ttsConfigs[0].id, attachedTagIds: ["draft-created", updated.id] }, tagChanges: { created: [{ draftId: "draft-created", values: { name: "新标签", description: "新描述", promptContent: "新提示", tagType: "CUSTOM" } }], updated: [{ id: updated.id, name: updated.name, description: "修改描述", promptContent: "修改提示", tagType: updated.tagType }], deleted: [removed.id] } });
  const publications = disk.calls.filter(
    /** Counts only the exact authoritative snapshot destination; owner USER.md files are distinct publications. */
    call => call.method === "move" && path.resolve(call.args[1]) === before,
  );
  assert.equal(publications.length, 1);
  const state = JSON.parse(await readFile(before, "utf8"));
  assert.deepEqual(state.cards, plain(snapshot.cards)); assert.deepEqual(state.tags, plain(snapshot.tags));
  assertCompleteSnapshot(snapshot, state);
  assertDirectoryCalls(settings, ["listTtsConfigs", ...snapshotDirectoryMethods]);
  const newTags = snapshot.tags.filter(
    /** Identifies the real staged creation by its explicit unique input name. */
    tag => tag.name === "新标签",
  );
  assert.equal(newTags.length, 1); assert.equal(typeof newTags[0].id, "string"); assert.notEqual(newTags[0].id, "draft-created");
  assert.deepEqual({ name: newTags[0].name, description: newTags[0].description, promptContent: newTags[0].promptContent, tagType: newTags[0].tagType },
    { name: "新标签", description: "新描述", promptContent: "新提示", tagType: "CUSTOM" });
  const edited = snapshot.tags.find(
    /** Resolves the exact retained identity after the staged update. */
    tag => tag.id === updated.id,
  );
  assert.equal(edited.description, "修改描述"); assert.equal(edited.promptContent, "修改提示");
  assert.equal(snapshot.tags.some(
    /** Requires that the exact staged deletion is absent from the complete returned state. */
    tag => tag.id === removed.id,
  ), false);
  const current = await plugin.api("character.get", { id: card.id }); assert.equal(current.description, "Web 更新");
  assert.equal(current.attachedTagIds.length, 2); assert.equal(current.attachedTagIds.indexOf("draft-created"), -1);
  assert.equal(current.ttsConfigId, directoryInput.ttsConfigs[0].id);
  assert.deepEqual(plain(current.attachedTagIds), [newTags[0].id, updated.id]);
  settings.clearCalls(); disk.clearCalls();
  const reopened = openPlugin(settings.harness); assert.deepEqual(plain(await reopened.api("character.get", { id: card.id })), plain(current));
  assertCompleteSnapshot(await reopened.web({ action: "snapshot" }), state);
  assertDirectoryCalls(settings, snapshotDirectoryMethods); assertNoMutations(disk);
});

/** Preserves the original IO-only harness capability boundary even after opt-in directory fixtures are available. */
test("IO-only file entrypoints reject undeclared SoftwareSettings instead of supplying an empty snapshot", async t => {
  const disk = await createDiskHarness(t), plugin = openPlugin(disk);
  await plugin.api("character.list", {});
  const file = path.join(disk.directory, "character-memory", "state.json"), before = await readFile(file);
  disk.clearCalls();
  assert.throws(
    /** Requires the original unmodified Tools proxy to reject the undeclared capability directly. */
    () => disk.globals.Tools.SoftwareSettings,
    { message: "Disk harness permits Tools.Files IO only: SoftwareSettings" },
  );
  await assert.rejects(plugin.web({ action: "snapshot" }), { message: "Disk harness permits Tools.Files IO only: SoftwareSettings" });
  await assert.rejects(plugin.api("snapshot", {}), { message: "Disk harness permits Tools.Files IO only: SoftwareSettings" });
  assertNoMutations(disk); assert.deepEqual(await readFile(file), before);
});

/** Keeps the explicit directory fixture narrow and immutable without treating it as native-host evidence. */
test("explicit SoftwareSettings test inputs preserve values and reject undeclared host methods", async t => {
  const disk = await createDiskHarness(t), supplied = plain(directoryInput), settings = createSoftwareSettingsFixture(disk, supplied);
  assert.equal(settings.harness.globals.Tools.Files, disk.files);
  const api = settings.harness.globals.Tools.SoftwareSettings;
  assert.throws(
    /** Rejects the legacy caller bridge even though directory read inputs have been explicitly installed. */
    () => api.exec,
    { message: "Undeclared SoftwareSettings fixture capability: exec" },
  );
  assert.throws(
    /** Rejects unrelated host business capabilities rather than manufacturing a successful manager. */
    () => settings.harness.globals.Tools.Memory,
    { message: "Undeclared Tools fixture capability: Memory" },
  );
  await assert.rejects(api.listModelSummaries("unexpected"), { message: "SoftwareSettings fixture requires the SDK's zero-argument directory call: listModelSummaries" });
  assert.equal(settings.calls.length, 0);
  supplied.models[0].capabilities.toolCall = false;
  supplied.ttsConfigs[0].headers[0].value = "mutated-supplied-header";
  supplied.toolCatalog.mcpServers[0].description = "mutated-supplied-source";
  const models = await api.listModelSummaries(), speech = await api.listTtsConfigs(), catalog = await api.readToolSourceCatalog();
  assert.deepEqual(models, directoryInput.models); assert.deepEqual(speech, directoryInput.ttsConfigs); assert.deepEqual(catalog, directoryInput.toolCatalog);
  models[0].providerName = "mutated-returned-model"; speech[0].responsePipeline[0].path = "mutated-returned-path";
  catalog.builtinTools[0].name = "mutated-returned-tool";
  assert.deepEqual(await api.listModelSummaries(), directoryInput.models);
  assert.deepEqual(await api.listTtsConfigs(), directoryInput.ttsConfigs);
  assert.deepEqual(await api.readToolSourceCatalog(), directoryInput.toolCatalog);
  assertDirectoryCalls(settings, [...snapshotDirectoryMethods, ...snapshotDirectoryMethods]);
  assert.equal(disk.calls.length, 0, "Directory test inputs must not initialize or manufacture plugin records");
  assert.deepEqual(await disk.entries(), []);
});

for (const method of snapshotDirectoryMethods) {
  /** Propagates one exact SDK directory rejection through real API/Web service calls and aborts staged business writes. */
  test("actual entrypoints propagate " + method + " read errors without publishing staged card or tags", async t => {
    const disk = await createDiskHarness(t), settings = createSoftwareSettingsFixture(disk, directoryInput), plugin = openPlugin(settings.harness);
    const card = await plugin.api("character.create", { values: { name: "失败读取前角色", description: "保存前描述" } });
    const updated = await plugin.api("tag.create", { values: { name: "错误前保留标签", description: "保留描述", promptContent: "保留提示", tagType: "TONE" } });
    const removed = await plugin.api("tag.create", { values: { name: "错误前删除标签", description: "删除前描述", promptContent: "删除前提示", tagType: "CUSTOM" } });
    assert.equal(card.ttsConfigId, null, "The injected failure must occur in the full snapshot after tag/card staging");
    const file = path.join(disk.directory, "character-memory", "state.json"), before = await readFile(file), state = JSON.parse(before.toString("utf8"));
    const original = new Error("Explicit test directory read failure: " + method);

    /** Requires the supplied original rejection, all real callback attempts and unchanged authoritative disk bytes. */
    async function rejectsWithoutPublication(invoke) {
      disk.clearCalls(); settings.clearCalls(); settings.failNext(method, original);
      await assert.rejects(invoke(),
        /** Rejects replacement errors, swallowed failures and fulfilled error-shaped return values. */
        error => error === original,
      );
      assertDirectoryCalls(settings, snapshotDirectoryMethods);
      assertNoMutations(disk); assert.deepEqual(await readFile(file), before);
    }

    await rejectsWithoutPublication(
      /** Calls the actual IPC-registered full snapshot request, not a mocked snapshot function. */
      () => plugin.web({ action: "snapshot" }),
    );
    await rejectsWithoutPublication(
      /** Calls the actual main-registered public snapshot provider with its declared payload. */
      () => plugin.api("snapshot", {}),
    );
    await rejectsWithoutPublication(
      /** Stages all three tag mutations and a complete card update before the actual snapshot callback rejects. */
      () => plugin.web({
        action: "saveCharacter", create: false,
        card: { ...plain(card), description: "不得写入的修改", attachedTagIds: ["failed-draft", updated.id] },
        tagChanges: {
          created: [{ draftId: "failed-draft", values: { name: "不得写入的新标签", description: "不得写入", promptContent: "不得写入", tagType: "CUSTOM" } }],
          updated: [{ ...plain(updated), description: "不得写入的标签修改", promptContent: "不得写入的提示" }],
          deleted: [removed.id],
        },
      }),
    );
    assert.deepEqual(plain(await plugin.api("character.get", { id: card.id })), plain(card));
    assert.deepEqual(plain(await plugin.api("tag.list", {})), state.tags);
    settings.clearCalls(); disk.clearCalls();
    assertCompleteSnapshot(await plugin.web({ action: "snapshot" }), state);
    assertDirectoryCalls(settings, snapshotDirectoryMethods); assertNoMutations(disk);
    settings.clearCalls(); disk.clearCalls();
    const reopened = openPlugin(settings.harness);
    assertCompleteSnapshot(await reopened.api("snapshot", {}), state);
    assertDirectoryCalls(settings, snapshotDirectoryMethods); assertNoMutations(disk);
  });
}

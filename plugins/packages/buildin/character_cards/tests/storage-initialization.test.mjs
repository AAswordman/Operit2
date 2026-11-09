import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { createDiskHarness } from "./disk-files.mjs";
import { fixture } from "./fixtures.mjs";
import { openPlugin } from "./plugin-entry-harness.mjs";
import { loadModule, plain } from "./runtime.mjs";

const display = {
  purpose: "display", chatId: "legacy-chat", chatExtension: null, participantId: null,
  promptFunctionType: "CHAT", defaultModelBinding: { providerId: "dashscope", modelId: "qwen-plus" }, defaultTtsConfigId: "",
};

/** Supplies only generic host IO; lifecycle, configuration and migration execute production code. */
function plugin(disk, storage = disk.storage) {
  return openPlugin({ ...disk, globals: { ...disk.globals, Tools: {
    Files: disk.files, Storage: storage,
    Chat: { async listAll() { return { totalCount: 1, chats: [{ id: "legacy-chat", messageCount: 0 }] }; } },
    Models: { async listSummaries() { return { models: fixture().snapshot.models }; } },
    Tts: { async listConfigs() { return { configs: [] }; } },
    Catalog: { async readToolSources() { return { builtin: [], packages: [], skills: [], mcp: [] }; } },
  } } });
}

/** Preserves every storage operation while observing only the metadata handle boundary. */
function metadataHost(disk, observe) {
  return { ...disk.storage, dataStore: { async open(options) {
    const handle = await disk.storage.dataStore.open(options);
    if (options.path.endsWith("/meta.preferences.json")) observe(handle);
    return handle;
  } } };
}

test("first-install lifecycle and chat display create one repository", async t => {
  const disk = await createDiskHarness(t);
  let metadataOpens = 0;
  const app = plugin(disk, metadataHost(disk, () => { metadataOpens++; }));
  await Promise.all([app.main.onServiceInitialize({}), app.api("chat.configuration.resolve", display)]);
  assert.equal(metadataOpens, 1);
  const cards = await app.api("character.list", {});
  assert.equal(cards.length, 1);
});

/** Opens a new runtime against the same durable storage rather than reusing its service promise. */
function repositoryOpener(disk, storage = disk.storage) {
  return () => loadModule("src/storage/database.ts", {
    ...disk.globals, Tools: { Files: disk.files, Storage: storage },
  }).DatabaseCharacterRepository.open();
}

/** Interrupts exactly one real installation boundary, including a committed write whose reply is lost. */
function interruptInstallation(disk, step, timing) {
  const failure = new Error("Interrupted installation: " + step + "/" + timing);
  const handles = { opened: 0, closed: 0 };
  let armed = true;
  /** Counts only real retained handles and their actual close acknowledgements. */
  function retainedHandle(handle) {
    handles.opened++;
    const close = handle.close;
    handle.close = async () => { await close(); handles.closed++; };
    return handle;
  }
  /** Throws once at the selected side of the host mutation. */
  function interrupt(side) {
    if (armed && timing === side) { armed = false; throw failure; }
  }
  for (const method of ["mkdir", "write"]) if (step === method) {
    const original = disk.files[method];
    disk.files[method] = async (...args) => {
      interrupt("before");
      const result = await original(...args);
      interrupt("after"); return result;
    };
  }
  const storage = { ...disk.storage,
    dataStore: { async open(options) {
      const handle = retainedHandle(await disk.storage.dataStore.open(options)), edit = handle.edit;
      handle.edit = async callback => {
        let selected = false;
        const result = await edit(async preferences => {
          await callback(preferences);
          selected = options.path.endsWith("/meta.preferences.json")
            ? step === (preferences.contains("state") ? "completion" : "plan")
            : step === "card" && options.path.endsWith("/cards.preferences.json");
          if (selected) interrupt("before");
        });
        if (selected) interrupt("after");
        return result;
      };
      return handle;
    } },
    objects: { async open(options) {
      const handle = retainedHandle(await disk.storage.objects.open(options)), commit = handle.commit;
      handle.commit = async (...args) => {
        if (step === "owner") interrupt("before");
        const result = await commit(...args);
        if (step === "owner") interrupt("after");
        return result;
      };
      return handle;
    } },
  };
  return { storage, failure, handles };
}

for (const step of ["plan", "card", "owner", "mkdir", "write", "completion"]) {
  for (const timing of ["before", "after"]) {
    test(`first install resumes the persisted identity after ${step}/${timing}`, async t => {
      const disk = await createDiskHarness(t);
      const { storage, failure, handles } = interruptInstallation(disk, step, timing);
      await assert.rejects(repositoryOpener(disk, storage)(), error => error === failure);
      assert.ok(handles.opened > 0);
      assert.equal(handles.closed, handles.opened, "Unsuccessful opens must release every retained host handle");
      const pending = disk.storage.inspectPreferences("meta", values => values.initialization);
      const completed = disk.storage.inspectPreferences("meta", values => values.state);
      assert.equal(pending !== undefined && completed !== undefined, false);
      const id = pending?.card.id ?? completed?.active.CharacterCard.id;
      const repository = await repositoryOpener(disk)();
      const cards = await repository.listCharacters();
      assert.equal(cards.length, 1);
      if (id !== undefined) assert.equal(cards[0].id, id);
      assert.equal(disk.storage.inspectPreferences("meta", values => values.initialization), undefined);
      const state = disk.readState();
      assert.equal(state.owners.length, 1);
      assert.equal(state.owners[0].ownerKey, "character:" + cards[0].id);
      assert.equal((await disk.files.exists(disk.directory + "/" + state.owners[0].userDocumentPath)).exists, true);
    });
  }
}

test("resumption uses the saved payload rather than the current default template", async t => {
  const disk = await createDiskHarness(t);
  const { storage } = interruptInstallation(disk, "card", "before");
  await assert.rejects(repositoryOpener(disk, storage)(), /Interrupted installation/);
  const id = disk.storage.inspectPreferences("meta", values => {
    values.initialization.card.name = "Saved installation template";
    values.initialization.card.characterSetting = "Previous template revision";
    values.initialization.memory.settings.autoSaveIntervalMinutes = 17;
    return values.initialization.card.id;
  });
  await repositoryOpener(disk)();
  const state = disk.readState();
  assert.equal(state.cards[0].id, id);
  assert.equal(state.cards[0].name, "Saved installation template");
  assert.equal(state.cards[0].characterSetting, "Previous template revision");
  assert.equal(state.owners[0].settings.autoSaveIntervalMinutes, 17);
});

test("a committed USER.md survives resumption without being rewritten", async t => {
  const disk = await createDiskHarness(t);
  const { storage } = interruptInstallation(disk, "write", "after");
  await assert.rejects(repositoryOpener(disk, storage)(), /Interrupted installation/);
  const id = disk.storage.inspectPreferences("meta", values => values.initialization.card.id);
  const file = disk.directory + "/owners/" + encodeURIComponent("character:" + id) + "/USER.md";
  await disk.files.write(file, "# Keep this document");
  disk.clearCalls();
  await repositoryOpener(disk)();
  assert.equal((await disk.files.read(file)).content, "# Keep this document");
  assert.equal(disk.calls.some(call => call.method === "write"), false);
});

test("default-looking records without an installation plan are not adopted", async t => {
  const disk = await createDiskHarness(t);
  const { storage } = interruptInstallation(disk, "owner", "before");
  await assert.rejects(repositoryOpener(disk, storage)(), /Interrupted installation/);
  disk.storage.inspectPreferences("meta", values => { delete values.initialization; });
  const cards = disk.storage.inspectPreferences("cards", values => structuredClone(values));
  await assert.rejects(repositoryOpener(disk)(), /unplanned configuration: cards/);
  assert.deepEqual(disk.storage.inspectPreferences("cards", values => values), cards);
  assert.equal(disk.storage.inspectPreferences("meta", values => values.state), undefined);
});

test("resumption rejects a changed card without replacing it", async t => {
  const disk = await createDiskHarness(t);
  const { storage } = interruptInstallation(disk, "owner", "before");
  await assert.rejects(repositoryOpener(disk, storage)(), /Interrupted installation/);
  disk.storage.inspectPreferences("cards", values => {
    for (const [key, value] of Object.entries(values)) if (key.startsWith("record_")) value.name = "Keep user edit";
  });
  await assert.rejects(repositoryOpener(disk)(), /unplanned configuration: cards/);
  assert.equal(disk.storage.inspectPreferences("meta", values => values.state), undefined);
  assert.equal(disk.storage.inspectPreferences("cards", values => Object.values(values)[0].name), "Keep user edit");
});

test("resumption rejects unexpected owner records before publishing metadata", async t => {
  const disk = await createDiskHarness(t);
  const { storage } = interruptInstallation(disk, "mkdir", "before");
  await assert.rejects(repositoryOpener(disk, storage)(), /Interrupted installation/);
  disk.storage.inspect(sql => sql.prepare("UPDATE records SET value=json_set(value,'$.settings.autoSaveIntervalMinutes',99) WHERE collection='owners'").run());
  await assert.rejects(repositoryOpener(disk)(), /unplanned memory records/);
  assert.equal(disk.storage.inspectPreferences("meta", values => values.state), undefined);
});

test("invalid installation plans fail before creating records", async t => {
  const disk = await createDiskHarness(t);
  const { storage } = interruptInstallation(disk, "plan", "after");
  await assert.rejects(repositoryOpener(disk, storage)(), /Interrupted installation/);
  disk.storage.inspectPreferences("meta", values => { values.initialization.version = 99; });
  await assert.rejects(repositoryOpener(disk)(), /Unsupported character initialization plan/);
  assert.deepEqual(disk.storage.inspectPreferences("cards", values => values), {});
});

test("resumption derives owner identity and document path from a remapped planned card", async t => {
  const disk = await createDiskHarness(t);
  const { storage } = interruptInstallation(disk, "plan", "after");
  await assert.rejects(repositoryOpener(disk, storage)(), /Interrupted installation/);
  const id = "14a70a97-814f-4b17-9bfd-c019e16c4090";
  disk.storage.inspectPreferences("meta", values => { values.initialization.card.id = id; });
  await repositoryOpener(disk)();
  const state = disk.readState();
  assert.equal(state.cards[0].id, id);
  assert.equal(state.active.CharacterCard.id, id);
  assert.equal(state.owners[0].ownerKey, "character:" + id);
  assert.equal(state.owners[0].userDocumentPath, "owners/" + encodeURIComponent("character:" + id) + "/USER.md");
});

test("the space-owned initialization plan declares every pending card reference", () => {
  const merge = JSON.parse(readFileSync(new URL("../src/storage/merge.json", import.meta.url), "utf8"));
  const references = merge.meta.references.filter(reference => reference.key.kind === "exact" && reference.key.key === "initialization");
  assert.deepEqual(references, [
    { key: { kind: "exact", key: "initialization" }, path: [{ kind: "field", name: "card" }, { kind: "field", name: "id" }], target: "runtime/plugin_data/space/com.operit.character_cards/cards.preferences.json" },
    ...merge.cards.references.map(reference => ({ key: { kind: "exact", key: "initialization" }, path: [{ kind: "field", name: "card" }, ...reference.path], target: reference.target })),
  ]);
});

test("a concurrent plan change cannot be published as successful initialization", async t => {
  const disk = await createDiskHarness(t), write = disk.files.write;
  disk.files.write = async (...args) => {
    const result = await write(...args);
    disk.storage.inspectPreferences("meta", values => { values.initialization.card.name = "Concurrent installation edit"; });
    return result;
  };
  await assert.rejects(repositoryOpener(disk)(), /Character initialization changed concurrently/);
  assert.equal(disk.storage.inspectPreferences("meta", values => values.state), undefined);
  assert.equal(disk.storage.inspectPreferences("meta", values => values.initialization.card.name), "Concurrent installation edit");
});

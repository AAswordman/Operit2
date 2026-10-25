import assert from "node:assert/strict";
import test from "node:test";
import { createDiskHarness } from "./disk-files.mjs";
import { fixture } from "./fixtures.mjs";
import { openPlugin } from "./plugin-entry-harness.mjs";
import { plain } from "./runtime.mjs";

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

/** Seeds released identifiers, a real USER document and a binding requiring the same UUID adoption. */
async function released(t) {
  const disk = await createDiskHarness(t, { legacyIdentity: "legacy" });
  const state = disk.readState();
  state.cards[0].name = "迁移前的角色";
  state.cards[0].characterSetting = "原有设定不能丢失";
  state.owners[0].memories = [plain(fixture().graph.items[0])];
  disk.replaceState(state);
  const originalDocument = disk.directory + "/" + state.owners[0].userDocumentPath;
  await disk.files.write(originalDocument, "# 原有 USER 文档\n不可丢失");
  const chats = await disk.storage.dataStore.open({ path: disk.directory + "/chats.preferences.json" });
  await chats.edit(preferences => preferences.set("binding_legacy-chat", { version: 1, selection: "card:legacy" }));
  await chats.close();
  disk.storage.inspectPreferences("meta", values => {
    values.__operit_preferences_schema_version = 0;
    values.state.legacyImportPending = true;
  });
  disk.clearCalls();
  return { disk, state, originalDocument };
}

/** Preserves every storage operation while observing only the metadata handle boundary. */
function metadataHost(disk, observe) {
  return { ...disk.storage, dataStore: { async open(options) {
    const handle = await disk.storage.dataStore.open(options);
    if (options.path.endsWith("/meta.preferences.json")) observe(handle);
    return handle;
  } } };
}

/** Checks business data and exact resource content independently from the initialized service. */
async function retained(disk, before) {
  const state = disk.readState();
  assert.equal(state.cards.length, before.cards.length);
  assert.equal(state.cards[0].name, before.cards[0].name);
  assert.equal(state.cards[0].characterSetting, before.cards[0].characterSetting);
  const memories = plain(state.owners[0].memories);
  assert.equal(memories.length, before.owners[0].memories.length);
  for (const [index, memory] of memories.entries()) {
    const original = before.owners[0].memories[index];
    for (const id of [memory.id, memory.uuid, ...memory.tags.map(tag => tag.id), ...memory.properties.map(property => property.id)])
      assert.match(id, /^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/i);
    assert.deepEqual(memory, { ...original, id: memory.id, uuid: memory.uuid,
      tags: original.tags.map((tag, i) => ({ ...tag, id: memory.tags[i].id })),
      properties: original.properties.map((property, i) => ({ ...property, id: memory.properties[i].id })),
    });
  }
  assert.equal((await disk.files.read(disk.directory + "/" + state.owners[0].userDocumentPath)).content, "# 原有 USER 文档\n不可丢失");
  assert.equal(disk.storage.inspectPreferences("meta", values => values.state.legacyImportPending), undefined);
  assert.equal(disk.storage.inspectPreferences("migration", values => values.plan), undefined);
  return state;
}

test("startup and current-chat requests wait for the same complete migration", async t => {
  const { disk, state } = await released(t);
  let ready, resume, metadataOpens = 0, editing = 0;
  const paused = new Promise(resolve => { ready = resolve; });
  const gate = new Promise(resolve => { resume = resolve; });
  const storage = metadataHost(disk, handle => {
    metadataOpens++;
    const schema = handle.withSchema;
    handle.withSchema = (version, migrate) => schema(version, (...args) => {
      editing++;
      try { return migrate(...args); } finally { editing--; }
    });
  });
  const copy = disk.files.copy;
  disk.files.copy = async (...args) => {
    assert.equal(editing, 0, "File conversion must execute outside the metadata schema callback");
    const result = await copy(...args);
    ready(); await gate;
    return result;
  };
  const app = plugin(disk, storage);
  const startup = app.main.onServiceInitialize({});
  await paused;
  const current = app.api("chat.configuration.resolve", display);
  const roles = app.api("character.list", {});
  const binding = app.api("chat.configuration.binding.read", { chatId: "legacy-chat" });
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(metadataOpens, 1, "Chat binding access must not construct a second storage owner");
  // A real metadata write during the file work used to invalidate the pending migration edit.
  const remote = await disk.storage.dataStore.open({ path: disk.directory + "/meta.preferences.json" });
  await remote.edit(preferences => {
    const current = plain(preferences.get("state"));
    current.memoryPreferences.autoUpdateEnabled = false;
    preferences.set("state", current);
  });
  await remote.close();
  resume();
  const [, , cards, selected] = await Promise.all([startup, current, roles, binding]);
  const saved = await retained(disk, state);
  assert.equal(cards[0].id, saved.cards[0].id);
  assert.equal(selected.selection, "card:" + saved.cards[0].id);
  assert.equal(disk.storage.inspectPreferences("meta", values => values.state.memoryPreferences.autoUpdateEnabled), false);
});

test("failed final metadata publication retains source files and resumes from its checkpoint", async t => {
  const { disk, state, originalDocument } = await released(t);
  const failure = new Error("Injected final metadata publication failure");
  let fail = true;
  const storage = metadataHost(disk, handle => {
    const edit = handle.edit;
    handle.edit = callback => edit(async preferences => {
      await callback(preferences);
      if (fail && preferences.get("state").legacyImportPending !== true) {
        fail = false; throw failure;
      }
    });
  });
  await assert.rejects(plugin(disk, storage).main.onServiceInitialize({}), error => error === failure);
  assert.equal((await disk.files.exists(originalDocument)).exists, true);
  assert.equal(disk.storage.inspectPreferences("migration", values => values.plan.phase), "cleanup");
  assert.equal(disk.storage.inspectPreferences("meta", values => values.state.legacyImportPending), true);
  await plugin(disk).main.onServiceInitialize({});
  await retained(disk, state);
});

test("cleanup failure after metadata publication resumes without converting identities again", async t => {
  const { disk, state, originalDocument } = await released(t);
  const remove = disk.files.deleteFile;
  let fail = true;
  disk.files.deleteFile = async (...args) => {
    assert.equal(disk.storage.inspectPreferences("meta", values => values.state.legacyImportPending), undefined,
      "Converted metadata must be durable before any original directory is removed");
    if (fail) { fail = false; throw new Error("Injected cleanup interruption"); }
    return remove(...args);
  };
  await assert.rejects(plugin(disk).main.onServiceInitialize({}), /Injected cleanup interruption/);
  const converted = disk.readState().cards[0].id;
  assert.notEqual(converted, "legacy");
  assert.equal((await disk.files.exists(originalDocument)).exists, true);
  await plugin(disk).main.onServiceInitialize({});
  const saved = await retained(disk, state);
  assert.equal(saved.cards[0].id, converted);
});

test("first-install lifecycle and chat display create one repository", async t => {
  const disk = await createDiskHarness(t);
  let metadataOpens = 0;
  const app = plugin(disk, metadataHost(disk, () => { metadataOpens++; }));
  await Promise.all([app.main.onServiceInitialize({}), app.api("chat.configuration.resolve", display)]);
  assert.equal(metadataOpens, 1);
  const cards = await app.api("character.list", {});
  assert.equal(cards.length, 1);
});

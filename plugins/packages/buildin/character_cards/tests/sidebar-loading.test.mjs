import assert from "node:assert/strict";
import test from "node:test";
import { createDiskHarness } from "./disk-files.mjs";
import { openPlugin } from "./plugin-entry-harness.mjs";
import { plain } from "./runtime.mjs";

/** Supplies neutral Core sidebar metadata without business extensions. */
function summary(i) { return { id: "loading-" + i, title: "Chat " + i, updatedAt: "0", displayOrder: i, workspaceId: null, workspaceName: null, parentChatId: null, locked: false, pinned: false, group: null }; }
/** Builds the existing generic sidebar request. */
function context(chats) { return { input: { view: "characters" }, chatSidebar: { chats, currentChatId: null, activeStreamingChatIds: [] } }; }
/** Opens real plugin SQL storage with unrelated directory and Chat capabilities unavailable. */
async function environment(t) {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }), plugin = openPlugin(disk);
  await plugin.api("character.list", {});
  disk.clearCalls();
  return { disk, plugin };
}

test("sidebar batches 525 preference bindings and never queries memory or Core extensions", async t => {
  const h = await environment(t), chats = Array.from({ length: 525 }, (_, i) => summary(i));
  const result = plain(await h.plugin.sidebar(context(chats)));
  const queries = h.disk.calls.filter(call => call.method === "storage.preferences.data");
  assert.equal(queries.filter(call => call.args[0].endsWith("/chats.preferences.json")).length, 1);
  assert.equal(queries.filter(call => call.args[0].endsWith("/chats.preferences.json")).length, 1);
  assert.equal(queries.some(call => ["memories", "embeddings", "candidates"].includes(call.args[0])), false);
  assert.deepEqual(result.sections.find(section => section.id === "unbound").chats, chats);
});

test("sidebar refresh observes a preference binding edit with unchanged Core chat IDs", async t => {
  const h = await environment(t), request = context([summary(1)]);
  assert.equal(plain(await h.plugin.sidebar(request)).sections.find(section => section.id === "unbound").chats.length, 1);
  const chats = await h.disk.storage.dataStore.open({ path: h.disk.directory + "/chats.preferences.json" });
  await chats.edit(preferences => preferences.set("binding_loading-1", { version: 1, selection: "card:default" }));
  await chats.close();
  const updated = plain(await h.plugin.sidebar(request));
  assert.equal(updated.sections.find(section => section.id === "card:default").chats[0].id, "loading-1");
});

test("sidebar propagates preference failures and missing bound roles without substituting unbound state", async t => {
  const h = await environment(t), request = context([summary(1)]), failure = new Error("SQL read failed");
  h.disk.failNext("storage.preferences.data", failure);
  await assert.rejects(h.plugin.sidebar(request), error => error === failure);
  const chats = await h.disk.storage.dataStore.open({ path: h.disk.directory + "/chats.preferences.json" });
  await chats.edit(preferences => preferences.set("binding_loading-1", { version: 1, selection: "card:missing" }));
  await chats.close();
  await assert.rejects(h.plugin.sidebar(request), /exactly one stored record/);
});

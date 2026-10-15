import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { readFile, stat } from "node:fs/promises";
import { createDiskHarness } from "./disk-files.mjs";
import { fixture } from "./fixtures.mjs";
import { openPlugin } from "./plugin-entry-harness.mjs";
import { loadModule, plain } from "./runtime.mjs";
import { createNativeChatHost } from "./native-chat-host.mjs";

/** Opens real plugin files and registrations with explicitly controlled generic host-record namespaces, never fixture CRUD. */
export async function selectorEnvironment(t, configurationSettings = {}, runtimeGlobals = {}, composeModules = undefined) {
  const disk = await createDiskHarness(t), readonly = fixture().snapshot, records = new Map([["selector-chat", { extension: null }]]);
  const native = createNativeChatHost(records), { calls } = native, directoryCalls = [];
  const chat = {
    ...native.chat,
    /** Implements only the declared exact generic lookup used by the real backend to validate a chat target. */
    async findChat(options) {
      native.observe("findChat", options); assert.deepEqual(plain(options), { query: "selector-chat", match: "exact", index: 0 });
      return { matchedCount: 1, chat: { id: "selector-chat", title: "Controlled native conversation", messageCount: 0, createdAt: "2026-10-08T12:00:00Z", updatedAt: "2026-10-08T12:00:00Z", isCurrent: true, inputTokens: 0, outputTokens: 0 },
        /** Formats only the explicitly supplied native lookup summary. */
        toString() { return "Controlled native conversation"; },
      };
    },
  };
  const tools = { Files: disk.files, Storage: disk.storage, Chat: chat, Models: {
    /** Provides the labelled readonly real SDK-shaped model catalog, not character storage or CRUD. */
    async listSummaries() { directoryCalls.push("models"); return { models: plain(readonly.models) }; }
  }, Tts: {
    /** Provides the labelled readonly TTS input required by the actual snapshot operation. */
    async listConfigs() { directoryCalls.push("tts"); return { configs: plain(readonly.ttsConfigs) }; },
    ...configurationSettings.Tts
  }, Catalog: {
    /** Provides explicitly labelled readonly tool sources without synthesizing a production policy result. */
    async readToolSources() { directoryCalls.push("tools"); return plain(readonly.toolCatalog); }
  }, Theme: {
    ...configurationSettings.Theme
  } };
  const plugin = openPlugin({ ...disk, composeModules, globals: { ...disk.globals, ...runtimeGlobals, Tools: tools, ToolPkg: { ...disk.globals.ToolPkg,
    /** Resolves and verifies one actually declared plugin resource on disk, never a fabricated avatar capability. */
    async readResource(key, name) {
      assert.ok(["character_default_avatar", "character_memory_html"].includes(key));
      const manifest = JSON.parse(await readFile(new URL("../manifest.json", import.meta.url), "utf8"));
      const found = manifest.resources.filter(
        /** Requires an exact registered resource key instead of guessing a directory or source. */
        resource => resource.key === key,
      );
      assert.equal(found.length, 1); assert.equal(found[0].path.split("/").pop(), name);
      if (key === "character_default_avatar") assert.equal(found[0].mime, "image/png");
      const file = fileURLToPath(new URL("../" + found[0].path, import.meta.url)); assert.ok((await stat(file)).size > 0); return file;
    },
  } } });
  assert.equal(disk.calls.length, 0); assert.deepEqual(calls, []); assert.deepEqual(directoryCalls, []);
  const storage = loadModule("src/chat-storage.ts", { ...disk.globals, Tools: tools });
  return {
    ...plugin, disk, records, calls, tools,
    /** Reads actual plugin binding rows for independent persistence assertions. */
    async binding(chatId = "selector-chat") {
      const row = await (await storage.chatDatabase()).get("chat_bindings", chatId);
      return row === null ? null : plain(row.value);
    },
    /** Seeds a declared SQL binding instead of modifying a Core extension fixture. */
    async seedBinding(chatId, value) {
      const database = await storage.chatDatabase(), existing = await database.get("chat_bindings", chatId);
      await database.commit([{ op: "put", collection: "chat_bindings", key: chatId, value, expectedVersion: existing === null ? null : existing.version }]);
    },
    /** Schedules an original error on the SQL boundary exercised by selection writes. */
    failNext(method, failure) { disk.failNext(method, failure); },
    /** Holds only real binding writes, allowing directory reads and UI rendering to continue. */
    holdWrite() { return disk.storage.holdTransaction(statements => statements.some(statement => /^(INSERT INTO|UPDATE) "chat_bindings" /.test(statement.sql))); },
  };
}

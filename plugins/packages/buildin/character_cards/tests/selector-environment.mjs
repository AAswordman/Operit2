import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { readFile, stat } from "node:fs/promises";
import { createDiskHarness } from "./disk-files.mjs";
import { fixture } from "./fixtures.mjs";
import { openPlugin } from "./plugin-entry-harness.mjs";
import { plain } from "./runtime.mjs";
import { createNativeChatHost } from "./native-chat-host.mjs";

/** Opens real plugin files and registrations with explicitly controlled generic host-record namespaces, never fixture CRUD. */
export async function selectorEnvironment(t, configurationSettings = {}, runtimeGlobals = {}) {
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
  const tools = { Files: disk.files, Chat: chat, SoftwareSettings: {
    /** Provides the labelled readonly real SDK-shaped model catalog, not character storage or CRUD. */
    async listModelSummaries() { directoryCalls.push("models"); return plain(readonly.models); },
    /** Provides the labelled readonly TTS input required by the actual snapshot operation. */
    async listTtsConfigs() { directoryCalls.push("tts"); return plain(readonly.ttsConfigs); },
    /** Provides explicitly labelled readonly tool sources without synthesizing a production policy result. */
    async readToolSourceCatalog() { directoryCalls.push("tools"); return plain(readonly.toolCatalog); },
    // Additional independently controlled Settings methods are explicit test inputs, never production defaults.
    ...configurationSettings,
  } };
  const plugin = openPlugin({ ...disk, globals: { ...disk.globals, ...runtimeGlobals, Tools: tools, ToolPkg: { ...disk.globals.ToolPkg,
    /** Resolves and verifies one actually declared plugin resource on disk, never a fabricated avatar capability. */
    async readResource(key, name) {
      assert.equal(key, "character_default_avatar"); assert.equal(name, "operit-avatar.png");
      const manifest = JSON.parse(await readFile(new URL("../manifest.json", import.meta.url), "utf8"));
      const found = manifest.resources.filter(
        /** Requires an exact registered resource key instead of guessing a directory or source. */
        resource => resource.key === key,
      );
      assert.equal(found.length, 1); assert.equal(found[0].mime, "image/png");
      const file = fileURLToPath(new URL("../" + found[0].path, import.meta.url)); assert.ok((await stat(file)).size > 0); return file;
    },
  } } });
  assert.equal(disk.calls.length, 0); assert.deepEqual(calls, []); assert.deepEqual(directoryCalls, []);
  return { ...plugin, disk, records, calls, tools, failNext: native.failNext, holdWrite: native.holdWrite };
}

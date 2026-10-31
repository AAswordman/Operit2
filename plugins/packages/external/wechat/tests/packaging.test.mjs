import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { load } from "./harness.mjs";

test("package is an optional device-scoped API v2 plugin with exact public dependency clients", async () => {
  const base = new URL("../", import.meta.url);
  const manifest = JSON.parse(
    await readFile(new URL("manifest.json", base), "utf8"),
  );
  assert.equal(manifest.toolpkg_id, "com.operit.wechat_bot");
  assert.equal(manifest.installation_scope, "device");
  assert.equal(manifest.api_version, "2.0.0");
  assert.equal(manifest.enabled_by_default, false);
  assert.ok(
    manifest.requires.some(
      (value) => value.id === "com.operit.character_cards",
    ),
  );
  assert.ok(manifest.requires.some((value) => value.id === "com.operit.waifu"));
  const publicClient = await readFile(
    new URL("../../waifu/src/api.ts", import.meta.url),
    "utf8",
  );
  const copied = await readFile(
    new URL("src/dependencies/waifu/api.ts", base),
    "utf8",
  );
  assert.equal(copied, publicClient);
});

test("all 21 tool metadata declarations have real exported functions and use main-runtime IPC", async () => {
  const source = await readFile(
    new URL("../src/tools.ts", import.meta.url),
    "utf8",
  );
  const metadata = JSON.parse(
    source.match(/\/\* METADATA\s*([\s\S]*?)\*\//)[1],
  );
  const calls = [];
  const tools = load("tools", {
    ToolPkg: {
      ipc: {
        async call(channel, request, options) {
          calls.push({ channel, request, options });
          return { success: true };
        },
      },
    },
  });
  assert.equal(metadata.tools.length, 21);
  assert.equal(new Set(metadata.tools.map((tool) => tool.name)).size, 21);
  for (const tool of metadata.tools) {
    assert.equal(typeof tools[tool.name], "function", tool.name);
    const args = Object.fromEntries(
      tool.parameters.map((parameter) => [
        parameter.name,
        parameter.type === "boolean"
          ? false
          : parameter.type === "number"
            ? 1
            : "value",
      ]),
    );
    await tools[tool.name](args);
  }
  assert.equal(calls.length, 20);
  assert.ok(
    calls.every(
      (call) =>
        call.channel === "wechat.service" &&
        call.options.targetRuntime === "main",
    ),
  );
});

test("main runtime registers one settings screen and all lifecycle hooks", () => {
  const screens = [],
    hooks = [],
    channels = [];
  const main = load("main", {
    ToolPkg: {
      ipc: {
        on(channel) {
          channels.push(channel);
        },
      },
      registerToolboxUiModule(screen) {
        screens.push(screen);
      },
      registerAppLifecycleHook(hook) {
        hooks.push(hook);
      },
    },
  });
  assert.equal(main.registerToolPkg(), true);
  assert.deepEqual(channels, ["wechat.service"]);
  assert.equal(screens.length, 1);
  assert.equal(screens[0].runtime, "compose_dsl");
  assert.deepEqual(
    hooks.map((hook) => hook.event),
    [
      "application_on_create",
      "application_on_foreground",
      "application_on_terminate",
    ],
  );
});

test("QR generator retains its upstream MIT license and runs entirely inside the plugin", async () => {
  const source = await readFile(
    new URL("../src/vendor/qrcode.js", import.meta.url),
    "utf8",
  );
  assert.match(source, /Copyright \(c\) 2009 Kazuhiko Arase/);
  assert.match(source, /MIT license/);
  assert.doesNotMatch(source, /fetch\(|XMLHttpRequest|child_process/);
});

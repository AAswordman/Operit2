import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createContext, runInContext } from "node:vm";

/** Extracts the production JavaScript embedded in a Rust raw string. */
async function embeddedScript(relativePath) {
  const source = await readFile(new URL(relativePath, import.meta.url), "utf8");
  const start = source.indexOf('r#"');
  const end = source.indexOf('"#', start + 3);
  assert.ok(start >= 0 && end > start);
  return source.slice(start + 3, end);
}

/** Builds the real SDK registration bridge with an observable host callback. */
async function runtime(registration, native) {
  const context = createContext({ __operitNativeGetScopedPluginConfigDir: native.getScopedPluginConfigDir });
  runInContext(`
    globalThis.__operitExpose = function(name, value) { globalThis[name] = value; };
    globalThis.__operitCurrentCallId = 'test';
    globalThis.__operitGetCallState = function() {
      return { params: { __operit_toolpkg_api_version: '1.0.0', toolPkgId: 'com.operit.debug_msg_dump' } };
    };
    globalThis.exports = {};
    globalThis.__operitGetActiveModuleExports = function() { return exports; };
  `, context);
  runInContext(await embeddedScript("../../core/crates/plugin/sdk/src/toolpkg/ToolPkgApiRuntimeScript.rs"), context);
  runInContext((await embeddedScript("../../core/crates/plugin/sdk/src/toolpkg/ToolPkgRegistrationBridge.rs"))
    .replace("__OPERIT_TOOLPKG_REGISTRATION_ONLY__", String(registration)), context);
  return context;
}

/** Verifies configuration remains usable at module scope and inside both supported API versions. */
test("registration preserves top-level config access and named directories", async () => {
  for (const apiVersion of ["1.0.0", "2.0.0"]) {
    const calls = [];
    const context = await runtime(true, { getScopedPluginConfigDir(owner, target) {
      calls.push([owner, target]);
      const root = "/app/data/extensions/device/plugins/configs/com.operit.debug_msg_dump";
      return target === owner ? root : `${root}/namespaces/${target}`;
    } });
    context.apiVersion = apiVersion;
    runInContext(`
      const registrationState = __operitGetCallState();
      registrationState.params.__operit_toolpkg_api_version = apiVersion;
      __operitGetCallState = function() { return registrationState; };
      const directory = ToolPkg.getConfigDir();
      exports.registerToolPkg = function() {
        ToolPkg.registerNavigationEntry({id: 'config-test', directory, alias: ToolPkg.getConfigDir('alias')});
        return true;
      };
    `, context);
    assert.equal(runInContext("exports.registerToolPkg()", context), true);
    const entry = JSON.parse(context.__operitToolPkgRegistrationCapture.navigationEntries[0]);
    assert.equal(entry.directory, "/app/data/extensions/device/plugins/configs/com.operit.debug_msg_dump");
    assert.equal(entry.alias, `${entry.directory}/namespaces/alias`);
    assert.deepEqual(calls, [
      ["com.operit.debug_msg_dump", "com.operit.debug_msg_dump"],
      ["com.operit.debug_msg_dump", "alias"],
    ]);
  }
});

/** Verifies a real directory creation failure still reaches the plugin during registration. */
test("registration propagates configuration host errors without disabling the API", async () => {
  const context = await runtime(true, { getScopedPluginConfigDir() {
    throw new Error("directory creation denied");
  } });
  assert.throws(() => runInContext("ToolPkg.getConfigDir()", context), /directory creation denied/);
});

/** Verifies the debug plugin declares hooks without reading paths or writing files. */
test("debug dump registration has no runtime side effects", async () => {
  const context = await runtime(true, {});
  runInContext(await readFile(new URL("../packages/external/debug_msg_dump/dist/main.js", import.meta.url), "utf8"), context);
  assert.equal(runInContext("exports.registerToolPkg()", context), true);
  assert.equal(context.__operitToolPkgRegistrationCapture.promptHistoryHooks.length, 1);
  assert.equal(context.__operitToolPkgRegistrationCapture.promptFinalizeHooks.length, 1);
});

/** Verifies runtime config queries preserve owner and alias arguments. */
test("runtime config access returns the host path", async () => {
  const calls = [];
  const context = await runtime(false, { getScopedPluginConfigDir(owner, target) {
    calls.push([owner, target]);
    return "/app/data/extensions/device/plugins/configs/debug";
  } });
  assert.equal(runInContext("ToolPkg.getConfigDir('alias')", context), "/app/data/extensions/device/plugins/configs/debug");
  assert.deepEqual(calls, [["com.operit.debug_msg_dump", "alias"]]);
});

/** Verifies configuration paths are direct values and host failures are never returned as text. */
test("config access propagates errors and validates absolute VFS paths", async () => {
  const failed = await runtime(false, { getScopedPluginConfigDir() {
    throw new Error("Extension is not registered: package:missing");
  } });
  assert.throws(() => failed.ToolPkg.getConfigDir(), /Extension is not registered: package:missing/);
  const success = await runtime(false, { getScopedPluginConfigDir() { return "/app/data/config"; } });
  assert.equal(success.ToolPkg.getConfigDir(), "/app/data/config");
  for (const value of ["relative", {success: true, path: "/app/data/config"}, null]) {
    const invalid = await runtime(false, { getScopedPluginConfigDir() { return value; } });
    assert.throws(() => invalid.ToolPkg.getConfigDir(), /absolute VFS path/);
  }
});

/** Verifies a hook awaits directory creation before writing and propagates failures. */
test("debug hooks await file operations and propagate mkdir failure", async () => {
  const calls = [];
  const context = await runtime(false, { getScopedPluginConfigDir() {
    calls.push("config");
    return "/app/data/config";
  } });
  context.console = { log() {} };
  let release;
  context.Tools = { Files: {
    mkdir(path) { calls.push(["mkdir", path]); return new Promise(resolve => { release = resolve; }); },
    async write(path) { calls.push(["write", path]); }
  } };
  runInContext(await readFile(new URL("../packages/external/debug_msg_dump/dist/main.js", import.meta.url), "utf8"), context);
  const done = context.exports.onPromptFinalize({ eventName: "finalize", eventPayload: {} });
  assert.deepEqual(calls, ["config", ["mkdir", "/app/data/config/dumps/"]]);
  release();
  await done;
  assert.equal(calls[2][0], "write");
  assert.match(calls[2][1], /^\/app\/data\/config\/dumps\/finalize_/);
  calls.length = 0;
  context.Tools.Files.mkdir = async () => { throw new Error("mkdir denied"); };
  await assert.rejects(context.exports.onPromptFinalize({ eventName: "finalize", eventPayload: {} }), /mkdir denied/);
  assert.deepEqual(calls, ["config"]);
});

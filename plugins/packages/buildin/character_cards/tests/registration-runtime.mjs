import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

/** Extracts one existing Rust-owned registration script without reimplementing SDK callback serialization. */
function script(file, name) {
  const text = readFileSync(fileURLToPath(new URL("../../../../../core/crates/plugin/sdk/src/toolpkg/" + file, import.meta.url)), "utf8");
  const signature = "pub fn " + name + "(";
  assert.equal(text.split(signature).length, 2);
  const start = text.indexOf('r#"', text.indexOf(signature)), end = text.indexOf('"#', start + 3);
  assert.ok(start >= 0 && end > start, "The actual SDK script must be present");
  return text.slice(start + 3, end);
}

/** Serializes only actual main-exported hook definitions through the existing SDK runtime and registration bridge. */
export function captureMainHookExports(main, chatMessageHooks, hostEventHooks) {
  const context = vm.createContext({
    __operitCurrentCallId: "main-hook-registration",
    /** Supplies the real API runtime with registration-version metadata, not business data or host services. */
    __operitGetCallState(callId) {
      assert.equal(callId, "main-hook-registration");
      return { params: { __operit_toolpkg_api_version: "2.0.0" } };
    },
    /** Exposes the actual evaluated main module exports for the SDK's durable callback-reference resolver. */
    __operitGetActiveModuleExports() { return main; },
    chatMessageHooks, hostEventHooks,
  });
  vm.runInContext("/** Publishes existing SDK namespaces into this isolated registration realm. */ globalThis.__operitExpose = function(name, value) { globalThis[name] = value; };", context);
  vm.runInContext(script("ToolPkgApiRuntimeScript.rs", "buildToolPkgApiRuntimeScript"), context);
  const registration = script("ToolPkgRegistrationBridge.rs", "buildToolPkgRegistrationBridgeScript");
  assert.equal(registration.split("__OPERIT_TOOLPKG_REGISTRATION_ONLY__").length, 2);
  vm.runInContext(registration.replace("__OPERIT_TOOLPKG_REGISTRATION_ONLY__", "true"), context);
  vm.runInContext("for (const definition of chatMessageHooks) ToolPkg.registerChatMessageHook(definition); for (const definition of hostEventHooks) ToolPkg.registerHostEventHook(definition);", context);
  return JSON.parse(vm.runInContext("JSON.stringify({chatMessageHooks:__operitToolPkgRegistrationCapture.chatMessageHooks.map(JSON.parse),hostEventHooks:__operitToolPkgRegistrationCapture.hostEventHooks.map(JSON.parse)})", context));
}

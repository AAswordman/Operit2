import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import test from "node:test";
import vm from "node:vm";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../../../../../", import.meta.url));
const sdk = "core/crates/plugin/sdk/src/";
const bridgePath = "core/crates/runtime/application/src/plugins/toolpkg/ToolPkgChatLifecycleHookBridge.rs";

/** Reads one exact current implementation path without inventing a successful host capability or production source. */
function source(relative) { return readFileSync(path.join(root, relative), "utf8"); }

/** Loads the existing Rust-owned registration JavaScript, not an emulated hook registration implementation. */
function embedded(file, name) {
  const text = source(sdk + "toolpkg/" + file), signature = "pub fn " + name + "(";
  assert.equal(text.split(signature).length, 2);
  const start = text.indexOf('r#"', text.indexOf(signature)), end = text.indexOf('"#', start + 3);
  assert.ok(start >= 0 && end > start, "The production registration script must really exist");
  return text.slice(start + 3, end);
}

/** Creates only an authenticated registration realm; no business provider, chat store or native runtime is implemented. */
function registry(owner) {
  const context = vm.createContext({
    __operitCurrentCallId: "lifecycle-register:" + owner,
    /** Supplies the actual registration bootstrap with package identity and API version metadata. */
    __operitGetCallState(callId) {
      assert.equal(callId, "lifecycle-register:" + owner);
      return { params: { toolPkgId: owner, __operit_toolpkg_api_version: "2.0.0" } };
    },
  });
  vm.runInContext("globalThis.__operitExpose = function(name, value) {globalThis[name] = value;};", context);
  vm.runInContext(embedded("ToolPkgApiRuntimeScript.rs", "buildToolPkgApiRuntimeScript"), context);
  vm.runInContext(embedded("ToolPkgRegistrationBridge.rs", "buildToolPkgRegistrationBridgeScript").replace("__OPERIT_TOOLPKG_REGISTRATION_ONLY__", "true"), context);
  return {
    /** Evaluates only explicitly supplied SDK registration input in its actual JavaScript realm. */
    evaluate(script) { return vm.runInContext(script, context); },
    /** Reads the actual serialized lifecycle capture produced by the SDK, never a fabricated registration result. */
    hooks() { return JSON.parse(vm.runInContext("JSON.stringify(__operitToolPkgRegistrationCapture.chatLifecycleHooks.map(JSON.parse))", context)); },
  };
}

/** Restricts static bridge routing assertions to exact existing function blocks rather than the retained reference repository. */
function rustFunction(text, name) {
  const pattern = new RegExp("\\b(?:pub\\s+)?(?:async\\s+)?fn\\s+" + name + "\\s*\\(");
  const start = text.search(pattern); assert.notEqual(start, -1, "Missing real Rust function: " + name);
  const opening = text.indexOf("{", start); let depth = 1, end = opening + 1;
  const stripped = text.replace(/"(?:\\.|[^"\\])*"|\/\/[^\r\n]*|\/\*[\s\S]*?\*\//g,
    /** Preserves offsets without counting braces inside error messages or comments as executable blocks. */
    token => token.replace(/[^\r\n]/g, " "),
  );
  for (; end < stripped.length && depth !== 0; end += 1) {
    if (stripped[end] === "{") depth += 1;
    if (stripped[end] === "}") depth -= 1;
  }
  assert.equal(depth, 0, "Unbalanced Rust source function: " + name);
  return text.slice(start, end);
}

/** Proves formal arbitrary-package registration resolves real main exports and captures at most one initializer per owner. */
test("real lifecycle registration captures exported handlers for two arbitrary package owners", () => {
  for (const owner of ["com.example.alpha", "org.example.beta"]) {
    const actual = registry(owner);
    actual.evaluate("function beforeCreate(event) { return {extension:{opaque:event.eventPayload.chat.id}}; }; globalThis.__operitGetActiveModuleExports=function(){return {beforeCreate:beforeCreate};}; ToolPkg.registerChatLifecycleHook({id:'creation',function:beforeCreate});");
    assert.deepEqual(actual.hooks(), [{ id: "creation", function: "beforeCreate" }]);
  }
});

/** Preserves the existing durable module-handler serialization and exercises its real generated require wrapper. */
test("real lifecycle registration serializes module-backed exports and missing exports explicitly fail", () => {
  const actual = registry("org.example.module");
  actual.evaluate("function moduleHook(event){}; moduleHook.__operit_toolpkg_module_path='hooks/creation.js'; moduleHook.__operit_toolpkg_export_name='initialize'; globalThis.__operitGetActiveModuleExports=function(){return {__operit_toolpkg_module_path:'main.js'};}; ToolPkg.registerChatLifecycleHook({id:'creation',function:moduleHook});");
  const hooks = actual.hooks(); assert.equal(hooks.length, 1); assert.equal(hooks[0].id, "creation");
  assert.match(hooks[0].function, /^__operit_module_ref_hook_creation_[0-9]+$/);
  assert.equal(typeof hooks[0].function_source, "string");
  let received;
  const handler = vm.runInNewContext("(" + hooks[0].function_source + ")", {
    /** Supplies only the exact exported module function input for testing the real SDK-generated durable wrapper. */
    require(module) {
      assert.equal(module, "./hooks/creation.js");
      return {
        /** Captures the wrapper's exact forwarded input without initializing a native chat or returning extension business. */
        initialize(event) { received = event; return event; },
      };
    },
  });
  const event = { eventName: "before_create", eventPayload: { chat: { id: "allocated-draft" } } };
  assert.equal(handler(event), event); assert.equal(received, event);
  const missing = vm.runInNewContext("(" + hooks[0].function_source + ")", {
    /** Tests a genuinely absent declared export rather than supplying a successful fallback function. */
    require(module) { assert.equal(module, "./hooks/creation.js"); return {}; },
  });
  assert.throws(
    /** Requires execution of the actual module wrapper to reject an absent export. */
    () => missing(event),
    /ToolPkg registered function export not found: initialize/,
  );
});

/** Requires actual durable functions and valid ids without partially capturing invalid registrations. */
test("real lifecycle registration rejects undeclared functions blank ids and missing authenticated owners", () => {
  const actual = registry("org.example.invalid");
  for (const expression of ["undefined", "null", "{}", "{id:''}", "{id:'creation',function:'beforeCreate'}", "{id:'creation',function:function(){}}"] ) {
    assert.throws(
      /** Sends explicit malformed SDK input to the current production registration bridge. */
      () => actual.evaluate(`ToolPkg.registerChatLifecycleHook(${expression});`),
      /registerChatLifecycleHook/,
    );
    assert.deepEqual(actual.hooks(), []);
  }
  assert.throws(
    /** An empty metadata owner must not become an unowned namespace registration. */
    () => registry("").evaluate("ToolPkg.registerChatLifecycleHook({id:'creation',function:function(){}});"),
    /registerChatLifecycleHook owner is unavailable/,
  );
});

/** Rejects owner duplication even with different ids and both official aliases instead of permitting namespace overwrite. */
test("real lifecycle registration rejects the second same-owner handler through every registration name", () => {
  for (const name of ["ToolPkg.registerChatLifecycleHook", "registerToolPkgChatLifecycleHook", "registerChatLifecycleHook"]) {
    const actual = registry("org.example.single");
    actual.evaluate("function beforeCreate() {return {extension:null};}; globalThis.__operitGetActiveModuleExports=function(){return {beforeCreate:beforeCreate};}; ToolPkg.registerChatLifecycleHook({id:'first',function:beforeCreate});");
    assert.throws(
      /** A second valid exported handler with another id is still a duplicate owner and must not be captured. */
      () => actual.evaluate(name + "({id:'second',function:beforeCreate});"),
      /** Requires the precise owner-level registration error. */
      error => error.message === "Duplicate chat lifecycle hook owner: org.example.single",
    );
    assert.deepEqual(actual.hooks(), [{ id: "first", function: "beforeCreate" }]);
  }
});

/** Statically checks all real Rust capture/parser/runtime fields; this is not Rust execution or generated-artifact completion. */
test("lifecycle Rust registration pipeline preserves capture and runtime handlers with duplicate owner validation", () => {
  const capture = source(sdk + "javascript.rs"), parser = source(sdk + "toolpkg/ToolPkgParser.rs"), main = source(sdk + "toolpkg/ToolPkgMainRegistrationScriptParser.rs");
  assert.match(capture, /serde\(rename = "chatLifecycleHooks", default\)[\s\S]*?pub chatLifecycleHooks: Vec<String>/);
  assert.match(parser, /pub chatLifecycleHooks: Vec<ToolPkgRegisteredFunctionHook>/);
  assert.match(parser, /pub chatLifecycleHooks: Vec<ToolPkgFunctionHookRuntime>/);
  assert.match(main, /chatLifecycleHooks: parseRegisteredItems\(\s*&captured\.chatLifecycleHooks,\s*TOOLPKG_REGISTRATION_CHAT_LIFECYCLE_HOOK/);
  assert.match(main, /registration\.chatLifecycleHooks\.len\(\) > 1[\s\S]*?return Err\(format!\("Duplicate chat lifecycle hook owner:/);
  assert.match(parser, /mainRegistration\.chatLifecycleHooks\.len\(\) > 1[\s\S]*?return Err\(format!\("Duplicate chat lifecycle hook owner:/);
  assert.match(parser, /let chatLifecycleHooks = validateFunctionHooks\(\s*&mainRegistration\.chatLifecycleHooks,\s*TOOLPKG_REGISTRATION_CHAT_LIFECYCLE_HOOK/);
});

/** Verifies the ready registry is awaited outside locks, preserving the application's explicit replacement semantics on restart. */
test("lifecycle bridge awaits a real ready catalog snapshot and app installs the current runtime explicitly", () => {
  const bridge = source(bridgePath), snapshot = rustFunction(bridge, "snapshot"), register = rustFunction(bridge, "register");
  assert.match(snapshot, /pub async fn snapshot\(\) -> Result<ToolPkgChatLifecycleDispatchContext, String>/);
  assert.match(snapshot, /RuntimePackageManager::readySnapshot\(runtime\.tool_handler\(\)\.getOrCreatePackageManager\(\)\)\.await\?/);
  assert.match(snapshot, /let runtime = \{[\s\S]*?current\.as_ref\(\)\.cloned\(\)\.ok_or_else[\s\S]*?\};\s*let manager = RuntimePackageManager::readySnapshot/);
  assert.match(register, /\*current = Some\(runtime\)/);
  assert.doesNotMatch(register, /get_or_init|\.set\(/);
  assert.match(bridge, /pub fn packageManager\(&self\) -> &RuntimePackageManager/);
  const app = source("core/crates/runtime/application/src/core/application/OperitApplication.rs");
  assert.equal(app.split("ToolPkgChatLifecycleHookBridge::register(self.toolPkgBridgeRuntime.clone())?").length, 2);
  assert.match(source("core/crates/runtime/application/src/plugins/toolpkg/mod.rs"), /pub mod ToolPkgChatLifecycleHookBridge;/);
});

/** Statically verifies formal event routing and hard failure propagation, never using a fake successful dispatcher as acceptance. */
test("lifecycle dispatch strictly awaits the own handler and validates draft before the no-participant result", () => {
  const bridge = source(bridgePath), dispatch = rustFunction(bridge, "dispatchBeforeCreate");
  assert.ok(dispatch.indexOf("validateCreationDraft(draft)?") < dispatch.indexOf("context.hooks.is_empty()"));
  assert.match(dispatch, /runToolPkgMainHookWithTimeoutMillis\([\s\S]*?TOOLPKG_EVENT_CHAT_LIFECYCLE,[\s\S]*?Some\(CHAT_LIFECYCLE_BEFORE_CREATE\)[\s\S]*?\.await\.map_err\([\s\S]*?\)\?/);
  assert.match(dispatch, /remainingTimeoutMillis\(\)[\s\S]*?\.ok_or_else[\s\S]*?\?/);
  assert.match(dispatch, /if budget\.hasExpired\(\)[\s\S]*?return Err/);
  assert.match(dispatch, /parseCreationResult\(raw\)[\s\S]*?\.map_err[\s\S]*?\?/);
  assert.match(dispatch, /extensions\.insert\(hook\.containerPackageName\.clone\(\), Value::Object\(value\)\)/);
  assert.doesNotMatch(dispatch, /decodeToolPkgHookResult|unwrap_or_default|get_or_init|ChatServiceCore|persist|saveChat|continue;/);
  assert.match(source(sdk + "toolpkg/ToolPkgCommonPluginConstants.rs"), /TOOLPKG_EVENT_CHAT_LIFECYCLE: &str = "toolpkg_chat_lifecycle"/);
});

/** Checks owner isolation and exact result validation in real Rust sources without claiming their unit tests were executed. */
test("lifecycle source namespace and strict result decoder expose no sibling map or owner override", () => {
  const bridge = source(bridgePath), payload = rustFunction(bridge, "creationPayload"), result = rustFunction(bridge, "parseCreationResult");
  assert.match(payload, /sourceExtensions\.get\(owner\)/);
  assert.match(payload, /Some\(Value::Object\(value\)\) => Some\(value\.clone\(\)\)/);
  assert.doesNotMatch(payload, /sourceExtensions\.clone\(\)|"sourceExtensions"\s*:|\.iter\(\)/);
  assert.match(payload, /"sourceExtension": sourceExtension/);
  assert.match(result, /raw\.ok_or_else/);
  assert.match(result, /let Value::Object\(mut result\) = parsed else[\s\S]*?return Err/);
  assert.match(result, /result\.len\(\) != 1[\s\S]*?return Err/);
  assert.match(result, /result\.remove\("extension"\)\.ok_or_else/);
  assert.match(result, /Value::Object\(value\) => Ok\(Some\(value\)\)/);
  assert.match(result, /Value::Null => Ok\(None\)/);
  assert.match(result, /_ => Err/);
});

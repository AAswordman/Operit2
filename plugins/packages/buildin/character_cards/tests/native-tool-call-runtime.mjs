import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const root = new URL("../../../../../", import.meta.url);
const require = createRequire(new URL("../../workflow/package.json", import.meta.url));
const { buildSync } = require("esbuild");

/** Reads the exact production source used by the native JavaScript execution boundary. */
export function nativeToolSource(relative) { return readFileSync(new URL(relative, root), "utf8"); }

/** Extracts the sole runtime prelude without generating or editing any SDK artifact. */
function executionPrelude() {
  const matches = [...nativeToolSource("core/crates/plugin/sdk/src/JsExecutionScriptBuilder.rs").matchAll(/r#"([\s\S]*?)"#/g)];
  assert.equal(matches.length, 1, "Expected one official execution prelude");
  return matches[0][1];
}

/** Executes the real embedded engine helper after resolving only Rust format-string brace escapes. */
function executionHelpers() {
  const text = nativeToolSource("core/crates/plugin/javascript-bridge/src/javascript/JsLibraries.rs");
  const start = text.indexOf("        function __operitParseToolResult(result, isError) {{");
  const end = text.indexOf("\n        {}\n        {}", start);
  assert.ok(start >= 0 && end > start, "Expected exact engine execution helper boundaries");
  const script = text.slice(start, end).replaceAll("{{", "{").replaceAll("}}", "}");
  assert.match(script, /function __operitExecuteScriptFunction\(/);
  assert.match(nativeToolSource("core/crates/plugin/javascript-bridge/src/javascript/JsEngine.rs"), /__operitExecuteScriptFunction\(\{callIdJson\}/);
  return script;
}

/** Bundles the current plugin module for the actual native module factory, not a replacement tool implementation. */
export function toolModuleScript(relative) {
  const entry = fileURLToPath(new URL("../" + relative, import.meta.url));
  const result = buildSync({ entryPoints: [entry], bundle: true, format: "cjs", platform: "neutral", target: "es2020", write: false });
  assert.equal(result.outputFiles.length, 1);
  return result.outputFiles[0].text;
}

/** Supplies one explicit native runtime envelope whose exact fields are checked against JsToolManager source. */
export function nativeToolParameters(participantId) {
  return {
    __operit_package_lang: "zh",
    __operit_package_state: "fixture.state",
    __operit_package_caller_name: "Fixture Speaker",
    __operit_package_chat_id: "fixture.chat",
    __operit_package_caller_participant_id: participantId,
    __operit_package_name: "character_memory_tools",
    __operit_toolpkg_runtime_kind: "sandbox",
    __operit_toolpkg_api_version: "2.0.0",
    __operit_execution_context_key: "toolpkg_main:com.operit.character_cards",
    __operit_toolpkg_subpackage_id: "fixture.tools",
    containerPackageName: "com.operit.character_cards",
    toolPkgId: "com.operit.character_cards",
    __operit_ui_package_name: "com.operit.character_cards",
    __operit_script_screen: "tools/index.js",
  };
}

/** Runs actual native-boundary JavaScript with strict transport endpoints; Rust injection itself is not executed by this Node harness. */
export function createNativeToolRuntime(globals, dispatchIpc) {
  const terminal = [], pending = new Map();
  let nextCall = 0;
  const methods = {
    /** Delivers only the production terminal result and rejects duplicate or unsolicited completions. */
    setCallResult(callId, raw) {
      const request = pending.get(callId);
      assert.notEqual(request, undefined, "Unexpected native result " + callId);
      pending.delete(callId);
      terminal.push({ callId, type: "result", raw });
      request.resolve(JSON.parse(raw));
    },
    /** Preserves the actual production error message rather than manufacturing a successful tool response. */
    setCallError(callId, raw) {
      const request = pending.get(callId);
      assert.notEqual(request, undefined, "Unexpected native failure " + callId);
      pending.delete(callId);
      terminal.push({ callId, type: "error", raw });
      request.reject(new Error(JSON.parse(raw).message));
    },
    /** Validates real execution trace traffic independently from terminal completion. */
    logJsExecutionTrace(callId, message) { assert.equal(typeof callId, "string"); assert.equal(typeof message, "string"); },
    /** Adapts the real native IPC callback endpoint to the declared production dispatcher, not a business stub. */
    invokeToolPkgIpcAsync(callbackId, packageTarget, callerContextKey, targetContextKey, targetRuntime, channel, payloadJson) {
      assert.equal(packageTarget, "com.operit.character_cards");
      assert.equal(callerContextKey, "toolpkg_main:com.operit.character_cards");
      assert.equal(targetContextKey, "");
      assert.equal(targetRuntime, "main");
      assert.equal(typeof dispatchIpc, "function", "Native IPC requires the explicitly declared transport adapter");
      dispatchIpc(channel, JSON.parse(payloadJson), { targetRuntime }).then(
        /** Delivers the exact production callback envelope after the actual domain service resolves. */
        value => context[callbackId](JSON.stringify({ success: true, value }), false),
        /** Sends the original rejection message through the actual error callback without a successful substitute. */
        error => context[callbackId](error.message, true),
      );
    },
  };
  const context = vm.createContext({
    ...globals,
    NativeInterface: new Proxy(methods, {
      /** Rejects every undeclared native endpoint instead of silently returning an empty capability. */
      get(target, name) {
        assert.ok(Object.hasOwn(target, name), "Undeclared native tool endpoint " + String(name));
        return target[name];
      },
    }),
  });
  vm.runInContext(nativeToolSource("core/crates/plugin/javascript-bridge/src/javascript/JsInitRuntime.script.js"), context);
  context.__operitRuntimePrelude = executionPrelude();
  vm.runInContext(executionHelpers(), context);

  /** Invokes the production engine entry point with exactly the supplied converted native parameters. */
  function invoke(script, name, params) {
    const callId = "native-tool-fixture-" + ++nextCall;
    const result = new Promise(
      /** Registers the sole terminal receiver before the real module and target function execute. */
      (resolve, reject) => pending.set(callId, { resolve, reject }),
    );
    context.__operitExecuteScriptFunction(callId, params, script, name, 10, 10000);
    return result;
  }
  return { invoke, terminal };
}

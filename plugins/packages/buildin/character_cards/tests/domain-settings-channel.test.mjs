import { installScopedHostRuntime, installGeneratedToolsRuntime, domainToolResult } from '../../../../../tools/tests/fixtures/scoped_host_runtime.mjs';
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import vm from "node:vm";

const root = new URL("../../../../../", import.meta.url);
const codegenPath = "core/crates/plugin/codegen/src/runtime_bindings.rs";
const directoryNames = ["Models.listSummaries", "Tts.listConfigs", "Theme.listConfigs", "Tts.getCurrentConfigId", "Catalog.readToolSources"];
const setterNames = ["Theme.applyConfig", "Tts.setCurrentConfigId"];

/** Reads actual SDK and native sources without running stale generation or claiming a Rust host integration. */
function source(path) { return readFileSync(new URL(path, root), "utf8"); }

/** Runs real generated-channel source over explicit host Promise IO only, without mocking canonical Theme or TTS persistence. */
function transport(version = "2.0.0") {
  const calls = [], pending = new Map(), methods = {}, context = vm.createContext({ version, Tools: { Models: {}, Tts: {}, Theme: {}, Catalog: {} } });
  let scheduledError = null;
  /** Records a single declared native call and leaves completion under the test's explicit control. */
  function enqueue(requestId, method, id) {
    if (scheduledError !== null) { const original = scheduledError; scheduledError = null; throw original; }
    assert.equal(pending.has(requestId), false);
    calls.push({ requestId, method, id }); pending.set(requestId, true);
  }
  const registry=installScopedHostRuntime(context);
  const identities = { models_list_summaries:'Models.listSummaries', tts_list_configs:'Tts.listConfigs', theme_list_configs:'Theme.listConfigs', tts_get_current_config_id:'Tts.getCurrentConfigId', catalog_read_tool_sources:'Catalog.readToolSources', theme_apply_config:'Theme.applyConfig', tts_set_current_config_id:'Tts.setCurrentConfigId' };
  const toolNames = Object.fromEntries(Object.entries(identities).map(([name,path])=>[path,name]));
  context.__operitNativeCallToolStructured=registry.binding((requestId,type,name,parameters)=>{ assert.equal(type,'default'); enqueue(requestId,identities[name],parameters.id); });
  installGeneratedToolsRuntime(context,version);
  for (const path of [...directoryNames, ...setterNames]) {
    const [namespace, name] = path.split("."); methods[path] = context.Tools[namespace][name];
  }
  return { calls, methods, context,
    /** Delivers only an actually pending native callback and asserts that its JS receiver is removed exactly once. */
    complete(call, result, isError = false) {
      assert.equal(pending.delete(call.requestId), true);
      registry.settle(call.requestId,isError ? result : {success:true,data:domainToolResult(toolNames[call.method],result)},isError);
    },
    /** Schedules an exact original native scheduling failure rather than fabricating callback success. */
    failSchedule(error) { assert.equal(scheduledError, null); scheduledError = new (vm.runInContext("Error",context))(error.message); },
    /** Counts only uncompleted host requests; no completion function is published in the SDK realm. */
    pendingRequests() {
      assert.equal(Object.keys(context).some(key=>/^__operit_(?:directory|config)_\d+$/.test(key)),false);
      return pending.size;
    },
  };
}

/** Locks the actual four generic SDK signatures and each registered native path to the canonical ordinary managers. */
test("static: ordinary Theme/TTS SDK methods have real typed bindings and canonical consumers", () => {
  const bindings = source("core/crates/plugin/sdk/src/js_sdk/runtime_bindings.rs");
  const engine = source("core/crates/plugin/javascript-bridge/src/javascript/JsEngine.rs");
  const native = source("core/crates/tool/services/src/tools/AIToolHandler.rs");
  const runtime = source("core/crates/runtime/application/src/services/ToolRuntimeSupportService.rs");
  const canonical = { "Models.listSummaries": "listModelSummaries", "Tts.listConfigs": "listTtsConfigs", "Theme.listConfigs": "listThemeConfigs", "Theme.applyConfig": "applyThemeConfig", "Tts.getCurrentConfigId": "getCurrentTtsConfigId", "Tts.setCurrentConfigId": "setCurrentTtsConfigId", "Catalog.readToolSources": "readToolSourceCatalog" };
  for (const path of [...directoryNames, ...setterNames]) {
    const [namespace, name] = path.split(".");
    const sdk = source("core/crates/plugin/sdk/src/js_sdk/" + namespace.toLowerCase() + ".rs");
    assert.ok(sdk.includes("fn " + name + "(&self"));
    assert.ok(bindings.includes('namespace: "' + namespace + '", method: "' + name + '", target: BuiltinToolName::'));
    const toolModule = {Models: 'StandardModelTools', Tts: 'StandardTtsTools', Theme: 'StandardThemeTools', Catalog: 'StandardCatalogTools'}[namespace];
    assert.match(source("core/crates/tool/services/src/tools/defaultTool/standard/" + toolModule + ".rs"), new RegExp("runtimeSupport\\(\\)\\s*\\." + canonical[path] + "\\("));

    assert.ok(runtime.includes("fn " + canonical[path] + "(" ) || runtime.includes("fn " + canonical[path] + "<'a>"));
  }
  assert.match(engine, /"__operitNativeCallToolStructured"/);
  assert.doesNotMatch(engine, /__operitNativeInvokeDomainAsync|invoke_direct_host_json/);
  assert.match(native, /impl JsExecutionHost for AIToolHandler/);
  assert.doesNotMatch(source("core/crates/plugin/sdk/src/js_sdk/software_settings.rs"), /listModelSummaries|listTtsConfigs|listThemeConfigs|applyThemeConfig|getCurrentTtsConfigId|setCurrentTtsConfigId|readToolSourceCatalog/);
  assert.match(runtime, /runtimeThemeConfigManager\(\)\?\.list\(\)/);
  assert.match(runtime, /runtimeThemeConfigManager\(\)\?\.apply\(id\)/);
  assert.match(runtime, /hostManager\.runtimeStorageHost\.clone\(\)/);
  assert.match(runtime, /TtsConfigManager::getInstance\(\)\.getCurrentTtsConfigId\(\)/);
  assert.match(runtime, /TtsConfigManager::getInstance\(\)\.setCurrentTtsConfigId\(&id\)/);
});

/** Preserves the exact host-supplied named-theme JSON through the actual directory callback helper. */
test("JS channel: listThemeConfigs waits for native and preserves full returned records", async () => {
  const io = transport(), result = io.methods["Theme.listConfigs"]();
  assert.equal(io.calls.length, 1); assert.equal(io.calls[0].method, "Theme.listConfigs");
  const record = { id: "ordinary.theme", name: "Named appearance", snapshot: { appearanceField: false, explicitNull: null }, createdAt: 123, updatedAt: 456 };
  // This JSON tests transport only; the canonical appearance validator and disk are not replaced or claimed by this fixture.
  io.complete(io.calls[0], [record]);
  assert.deepEqual(JSON.parse(JSON.stringify(await result)), {configs: [record]});
  assert.equal(io.pendingRequests(), 0);
});

/** Prevents an apply Promise from completing before the actual native preference result arrives. */
test("JS channel: applyThemeConfig cannot complete before native returns its commit result", async () => {
  const io = transport(); let settled = false;
  const result = io.methods["Theme.applyConfig"]("ordinary.theme").then(
    /** Marks only the real callback resolution, never the initial scheduling call. */
    value => { settled = true; return value; },
  );
  await Promise.resolve(); assert.equal(settled, false);
  assert.equal(io.calls.length, 1); assert.equal(io.calls[0].method, "Theme.applyConfig"); assert.equal(io.calls[0].id, "ordinary.theme");
  const applied = { id: "ordinary.theme", name: "Applied", snapshot: {}, createdAt: 10, updatedAt: 20 };
  io.complete(io.calls[0], applied);
  assert.deepEqual(JSON.parse(JSON.stringify(await result)), applied); assert.equal(settled, true);
});

/** Carries exact current and committed TTS IDs through the two established ordinary-setting signatures. */
test("JS channel: current TTS get/set preserve actual IDs and distinct callbacks", async () => {
  const io = transport(), current = io.methods["Tts.getCurrentConfigId"](), changed = io.methods["Tts.setCurrentConfigId"]("speech.exact");
  assert.equal(io.calls.length, 2); assert.notEqual(io.calls[0].requestId, io.calls[1].requestId);
  io.complete(io.calls[1], "speech.exact"); io.complete(io.calls[0], "speech.before");
  assert.deepEqual(structuredClone(await current), {configId:"speech.before"}); assert.deepEqual(structuredClone(await changed), {configId:"speech.exact"});
});

/** Rejects unsupported arity and malformed IDs before any native or canonical-setting operation is attempted. */
test("JS channel: exact arity and configuration ID validation do not invoke native on bad input", async () => {
  const io = transport();
  for (const method of directoryNames) await assert.rejects(io.methods[method]("extra"), /requires exactly 0 arguments/);
  for (const method of setterNames) {
    await assert.rejects(io.methods[method](), /requires exactly 1 argument/);
    await assert.rejects(io.methods[method]("id", "extra"), /requires exactly 1 argument/);
    for (const id of [undefined, null, 17, "", " ", " id", "id "]) await assert.rejects(io.methods[method](id), /exact nonblank text/);
  }
  assert.equal(io.calls.length, 0); assert.equal(io.pendingRequests(), 0);
});

/** Propagates original invalid-ID or persistence failure messages instead of emitting a replacement config or success. */
test("JS channel: native Theme and TTS failures reject with the exact original message", async () => {
  for (const method of setterNames) {
    const io = transport(), result = io.methods[method]("missing.exact");
    io.complete(io.calls[0], " exact native failure ", true);
    await assert.rejects(result, { message: " exact native failure " }); assert.equal(io.pendingRequests(), 0);
  }
});

/** Application strings are values, not JSON envelopes to decode or guess. */
test('JS channel preserves literal host strings and rejects only an explicit failure',async()=>{
  const io=transport(),literal=io.methods["Theme.applyConfig"]('id');
  io.complete(io.calls[0],'{"success":false}');assert.equal(await literal,'{"success":false}');
  const failure=io.methods["Tts.setCurrentConfigId"]('id');
  io.complete(io.calls[1],'null',true);await assert.rejects(failure,{message:'null'});
  assert.equal(io.pendingRequests(),0);
});

/** Retains the actual scheduling exception object and clears the callback instead of retrying or claiming success. */
test("JS channel: native scheduling errors propagate without retained callbacks", async () => {
  const io = transport(), failure = new Error("exact schedule failure"); io.failSchedule(failure);
  await assert.rejects(io.methods["Theme.applyConfig"]("ordinary.theme"),
    /** Requires the original exception message from the native scheduling boundary. */
    error => error.message === failure.message,
  );
  assert.equal(io.calls.length, 0); assert.equal(io.pendingRequests(), 0);
});

/** Verifies the generated V2 capabilities reject legacy versions before scheduling any native call. */
test("domain interfaces are V2-only and dispatch against the active call version", async () => {
  for (const version of ["1.0.0", "1.0.1"]) {
    const io = transport(version);
    for (const method of directoryNames) assert.throws(() => io.methods[method](), /manifest.api_version/);
    for (const method of setterNames) assert.throws(() => io.methods[method]("exact"), /manifest.api_version/);
    assert.equal(io.calls.length, 0);
    io.context.version = "2.0.0";
    const result = io.methods["Models.listSummaries"](); io.complete(io.calls[0], []);
    assert.deepEqual(structuredClone(await result), { models: [] });
    io.context.version = version;
    assert.throws(() => io.methods["Models.listSummaries"](), /manifest.api_version/);
  }
});

/** Exercises equal method names in distinct namespaces with out-of-order host completion. */
test("Theme and Tts listConfigs keep distinct native paths and complete records", async () => {
  const io = transport(), theme = io.methods["Theme.listConfigs"](), tts = io.methods["Tts.listConfigs"]();
  assert.deepEqual(io.calls.map(call => call.method), ["Theme.listConfigs", "Tts.listConfigs"]);
  io.complete(io.calls[1], [{ id: "speech", voice: "exact" }]); io.complete(io.calls[0], [{ id: "theme", snapshot: { color: "exact" } }]);
  assert.deepEqual(structuredClone(await theme), { configs: [{ id: "theme", snapshot: { color: "exact" } }] });
  assert.deepEqual(structuredClone(await tts), { configs: [{ id: "speech", voice: "exact" }] });
  assert.equal(io.pendingRequests(), 0);
});

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import vm from "node:vm";

const root = new URL("../../../../../", import.meta.url);
const codegenPath = "core/crates/plugin/codegen/src/runtime_bindings.rs";
const directoryNames = ["listThemeConfigs", "getCurrentTtsConfigId"];
const setterNames = ["applyThemeConfig", "setCurrentTtsConfigId"];

/** Reads actual SDK and native sources without running stale generation or claiming a Rust host integration. */
function source(path) { return readFileSync(new URL(path, root), "utf8"); }

/** Extracts only the official generator's current JavaScript callback-channel helpers. */
function helpers() {
  const code = source(codegenPath);
  const start = code.indexOf("/** Reads one declared SoftwareSettings directory");
  const end = code.indexOf("/** Validates complete JSON objects", start);
  assert.ok(start !== -1 && end > start);
  return code.slice(start, end);
}

/** Executes the generator's real wrapper template in isolation; this is not a promoted or officially generated artifact. */
function wrapper(arity, method) {
  const code = source(codegenPath), templates = [];
  for (const match of code.matchAll(/output\.push_str\(&format!\(("(?:[^"\\]|\\.)*")\)\);/g)) {
    const literal = JSON.parse(match[1]);
    if (/^\{expression\}\["\{method\}"\] = function\(/.test(literal)) templates.push(literal);
  }
  assert.equal(templates.length, 2, "Exactly two authoritative ordinary-setting wrapper signatures are required");
  const selected = templates.filter(
    /** Selects the exact native method arity rather than widening the settings namespace. */
    template => new RegExp("arguments\\.length !== " + arity + "\\b").test(template),
  );
  assert.equal(selected.length, 1);
  return selected[0].replaceAll("{{", "{").replaceAll("}}", "}")
    .replaceAll("{expression}", 'Tools["SoftwareSettings"]').replaceAll("{method}", method);
}

/** Runs real generated-channel source over explicit callback IO only, without mocking canonical Theme or TTS persistence. */
function transport() {
  const calls = [], pending = new Map(), methods = {}, context = vm.createContext({ Tools: { SoftwareSettings: methods } });
  let scheduledError = null;
  /** Records a single declared native call and leaves completion under the test's explicit control. */
  function enqueue(callbackId, method, id) {
    if (scheduledError !== null) { const original = scheduledError; scheduledError = null; throw original; }
    assert.equal(pending.has(callbackId), false);
    calls.push({ callbackId, method, id }); pending.set(callbackId, true);
  }
  context.__operitNativeReadSoftwareSettingsDirectoryAsync =
    /** Captures the directory ABI without supplying a default catalog or active ID. */
    (callbackId, method) => enqueue(callbackId, method, undefined);
  context.__operitNativeApplySoftwareSettingsConfigAsync =
    /** Captures the ordinary setter ABI without performing or pretending a native preference commit. */
    (callbackId, method, id) => enqueue(callbackId, method, id);
  vm.runInContext(helpers(), context);
  for (const method of directoryNames) vm.runInContext(wrapper(0, method), context);
  for (const method of setterNames) vm.runInContext(wrapper(1, method), context);
  return { calls, methods,
    /** Delivers only an actually pending native callback and asserts that its JS receiver is removed exactly once. */
    complete(call, result, isError = false) {
      assert.equal(pending.delete(call.callbackId), true);
      const callback = context[call.callbackId]; assert.equal(typeof callback, "function");
      callback(result, isError); assert.equal(context[call.callbackId], undefined);
    },
    /** Schedules an exact original native scheduling failure rather than fabricating callback success. */
    failSchedule(error) { assert.equal(scheduledError, null); scheduledError = error; },
    /** Counts actual callback receivers retained by the production channel helper. */
    retainedReceivers() { return Object.keys(context).filter(
      /** Identifies only the two exact callback-ID formats declared by this settings channel. */
      key => /^__operit_(?:directory|config)_\d+$/.test(key),
    ).length; },
  };
}

/** Locks the actual four generic SDK signatures and each registered native path to the canonical ordinary managers. */
test("static: ordinary Theme/TTS SDK methods have real typed bindings and canonical consumers", () => {
  const sdk = source("core/crates/plugin/sdk/src/js_sdk/software_settings.rs");
  const bindings = source("core/crates/plugin/sdk/src/js_sdk/runtime_bindings.rs");
  const engine = source("core/crates/plugin/javascript-bridge/src/javascript/JsEngine.rs");
  const native = source("core/crates/tool/services/src/tools/AIToolHandlerJsToolsHost.rs");
  const runtime = source("core/crates/runtime/application/src/services/ToolRuntimeSupportService.rs");
  for (const method of [...directoryNames, ...setterNames]) {
    assert.match(sdk, new RegExp("fn " + method + "\\(&self"));
    assert.match(bindings, new RegExp('JsDirectHostBinding \\{ namespace: "SoftwareSettings", method: "' + method + '" \\}'));
    assert.match(engine, new RegExp("executionHost\\s*\\.\\s*" + method + "\\("));
    assert.match(native, new RegExp("support\\s*\\.\\s*" + method + "\\("));
    assert.match(runtime, new RegExp("fn " + method + "\\("));
  }
  assert.match(runtime, /runtimeThemeConfigManager\(\)\?\.list\(\)/);
  assert.match(runtime, /runtimeThemeConfigManager\(\)\?\.apply\(id\)/);
  assert.match(runtime, /hostManager\.runtimeStorageHost\.clone\(\)/);
  assert.match(runtime, /TtsConfigManager::getInstance\(\)\.getCurrentTtsConfigId\(\)/);
  assert.match(runtime, /TtsConfigManager::getInstance\(\)\.setCurrentTtsConfigId\(&id\)/);
});

/** Preserves the exact host-supplied named-theme JSON through the actual directory callback helper. */
test("JS channel: listThemeConfigs waits for native and preserves full returned records", async () => {
  const io = transport(), result = io.methods.listThemeConfigs();
  assert.equal(io.calls.length, 1); assert.equal(io.calls[0].method, "listThemeConfigs");
  const record = { id: "ordinary.theme", name: "Named appearance", snapshot: { appearanceField: false, explicitNull: null }, createdAt: 123, updatedAt: 456 };
  // This JSON tests transport only; the canonical appearance validator and disk are not replaced or claimed by this fixture.
  io.complete(io.calls[0], JSON.stringify([record]));
  assert.deepEqual(JSON.parse(JSON.stringify(await result)), [record]);
  assert.equal(io.retainedReceivers(), 0);
});

/** Prevents an apply Promise from completing before the actual native preference result arrives. */
test("JS channel: applyThemeConfig cannot complete before native returns its commit result", async () => {
  const io = transport(); let settled = false;
  const result = io.methods.applyThemeConfig("ordinary.theme").then(
    /** Marks only the real callback resolution, never the initial scheduling call. */
    value => { settled = true; return value; },
  );
  await Promise.resolve(); assert.equal(settled, false);
  assert.equal(io.calls.length, 1); assert.equal(io.calls[0].method, "applyThemeConfig"); assert.equal(io.calls[0].id, "ordinary.theme");
  const applied = { id: "ordinary.theme", name: "Applied", snapshot: {}, createdAt: 10, updatedAt: 20 };
  io.complete(io.calls[0], JSON.stringify(applied));
  assert.deepEqual(JSON.parse(JSON.stringify(await result)), applied); assert.equal(settled, true);
});

/** Carries exact current and committed TTS IDs through the two established ordinary-setting signatures. */
test("JS channel: current TTS get/set preserve actual IDs and distinct callbacks", async () => {
  const io = transport(), current = io.methods.getCurrentTtsConfigId(), changed = io.methods.setCurrentTtsConfigId("speech.exact");
  assert.equal(io.calls.length, 2); assert.notEqual(io.calls[0].callbackId, io.calls[1].callbackId);
  io.complete(io.calls[1], JSON.stringify("speech.exact")); io.complete(io.calls[0], JSON.stringify("speech.before"));
  assert.equal(await current, "speech.before"); assert.equal(await changed, "speech.exact");
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
  assert.equal(io.calls.length, 0); assert.equal(io.retainedReceivers(), 0);
});

/** Propagates original invalid-ID or persistence failure messages instead of emitting a replacement config or success. */
test("JS channel: native Theme and TTS failures reject with the exact original message", async () => {
  for (const method of setterNames) {
    const io = transport(), result = io.methods[method]("missing.exact");
    io.complete(io.calls[0], JSON.stringify({ message: " exact native failure " }), true);
    await assert.rejects(result, { message: " exact native failure " }); assert.equal(io.retainedReceivers(), 0);
  }
});

/** Rejects malformed native JSON and malformed error envelopes rather than converting them to ordinary custom state. */
test("JS channel: malformed native result and error envelopes reject explicitly", async () => {
  const io = transport(), broken = io.methods.applyThemeConfig("id");
  io.complete(io.calls[0], "not-json"); await assert.rejects(broken);
  const malformed = io.methods.setCurrentTtsConfigId("id");
  io.complete(io.calls[1], "null", true); await assert.rejects(malformed, /Malformed SoftwareSettings configuration error/);
  assert.equal(io.retainedReceivers(), 0);
});

/** Retains the actual scheduling exception object and clears the callback instead of retrying or claiming success. */
test("JS channel: native scheduling errors propagate without retained callbacks", async () => {
  const io = transport(), failure = new Error("exact schedule failure"); io.failSchedule(failure);
  await assert.rejects(io.methods.applyThemeConfig("ordinary.theme"),
    /** Requires the original exception identity from the native scheduling boundary. */
    error => error === failure,
  );
  assert.equal(io.calls.length, 0); assert.equal(io.retainedReceivers(), 0);
});

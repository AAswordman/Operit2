import { composeStreamFixture } from '../../../../../tools/tests/support/compose_stream_fixture.mjs';
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import vm from "node:vm";

const root = new URL("../../../../../", import.meta.url);
const sdkRoot = "core/crates/plugin/sdk/src/";
const complete = { type: "toolpkg.presentation.complete", requestId: "fixture-request", value: { selected: "fixture-option", nested: [null, false, 3, "text"] } };
const cancel = { type: "toolpkg.presentation.cancel", requestId: "fixture-request" };

/** Reads authoritative source files without compiling or replacing the production callbacks. */
function source(path) { return readFileSync(new URL(path, root), "utf8").replaceAll("\r\n", "\n"); }

/** Extracts the sole production raw JavaScript string from a Rust bridge source. */
function embedded(path) {
  const matches = [...source(path).matchAll(/r#"([\s\S]*?)"#/g)];
  assert.equal(matches.length, 1, `Expected one embedded JavaScript definition in ${path}`);
  return matches[0][1];
}

/** Applies Rust format-string brace escaping before inserting the actual screen fixture. */
function wrappedScreen(screen) {
  const wrapper = embedded(sdkRoot + "toolpkg/ToolPkgComposeDslRuntimeScript.rs");
  assert.equal([...wrapper.matchAll(/\{script\}/g)].length, 1);
  return wrapper.replaceAll("{{", "{").replaceAll("}}", "}").replace("{script}", screen);
}

/** Serializes the real callback response across a JSON-only assertion boundary. */
function json(value) { return JSON.parse(JSON.stringify(value)); }

/** Creates a clickable fixture with the actual Compose context and dispatch implementation. */
function compose(component, property, callback) {
  const context = vm.createContext({ module: { exports: {} }, fixtureCallback: callback });
  for (const file of ["ToolPkgComposeDslCompiler.js", "ToolPkgComposeDslRetained.js", "ToolPkgComposeDslReactive.js"]) {
    vm.runInContext(source(sdkRoot + "toolpkg/" + file), context);
  }
  vm.runInContext(embedded(sdkRoot + "toolpkg/ToolPkgComposeDslBridge.rs"), context);
  vm.runInContext(wrappedScreen(`
    /** Creates a generic component with an observable callback result. */
    module.exports.default = function(ctx) {
      return ctx.UI.${component}({ ${property}: fixtureCallback }, []);
    };
  `), context);
  composeStreamFixture(context).adapt();
  return context;
}

/** Invokes the action identifier actually serialized by the production component builder. */
async function dispatch(context, property) {
  const rendered = await context.__operit_render_compose_dsl({});
  const action = rendered.tree.props[property];
  assert.equal(typeof action.__actionId, "string");
  return context.__operit_dispatch_compose_dsl_action({ actionId: action.__actionId });
}

/** Extracts native execution helpers, including Promise completion, from the engine's real bootstrap. */
function executionHelpers() {
  const text = source("core/crates/plugin/javascript-bridge/src/javascript/JsLibraries.rs");
  const start = text.indexOf("        function __operitParseToolResult(result, isError) {{");
  const end = text.indexOf("\n        {}\n        {}", start);
  assert.ok(start >= 0 && end > start, "Native execution helper boundaries must remain explicit");
  const helpers = text.slice(start, end).replaceAll("{{", "{").replaceAll("}}", "}");
  assert.match(helpers, /function __operitExecuteScriptFunction\(/);
  assert.match(source("core/crates/plugin/javascript-bridge/src/javascript/JsEngine.rs"), /__operitExecuteScriptFunction\(\{callIdJson\}/);
  return helpers;
}

/** Runs real execution/session/DSL JavaScript while replacing only native transport endpoints, not business or callback results. */
function nativeTransport(callbackBody) {
  const terminal = [], intermediate = [], traces = [], pending = new Map();
  const finalResponses = new Map();
  const methods = {
    /** Captures the exact native success payload after the production Promise is awaited. */
    setCallResult(callId, raw) {
      assert.equal(typeof raw, "string");
      const request = pending.get(callId);
      assert.notEqual(request, undefined, `Unexpected or duplicate native completion ${callId}`);
      pending.delete(callId);
      terminal.push({ callId, type: "result", raw });
      assert.equal(JSON.parse(raw), null);
      request.resolve(finalResponses.get(callId));
    },
    /** Propagates the production failure payload rather than creating a success result. */
    setCallError(callId, raw) {
      const request = pending.get(callId);
      assert.notEqual(request, undefined, `Unexpected or duplicate native error ${callId}`);
      pending.delete(callId);
      terminal.push({ callId, type: "error", raw });
      request.reject(new Error(JSON.parse(raw).message));
    },
    /** Records real intermediate render messages separately from terminal action results. */
    sendCallIntermediateResult(callId, raw) { intermediate.push({ callId, value: JSON.parse(raw) }); },
    /** Records native execution trace traffic without suppressing unexpected capabilities. */
    logJsExecutionTrace(callId, message) { traces.push({ callId, message }); },
  };
  const context = vm.createContext({
    NativeInterface: new Proxy(methods, {
      /** Rejects every undeclared native endpoint used by this isolated transport harness. */
      get(target, name) {
        assert.ok(Object.hasOwn(target, name), `Undeclared native test endpoint: ${String(name)}`);
        return target[name];
      },
    }),
  });
  vm.runInContext(source("core/crates/plugin/javascript-bridge/src/javascript/JsInitRuntime.script.js"), context);
  context.__operitRuntimePrelude = embedded(sdkRoot + "JsExecutionScriptBuilder.rs");
  context.__operitNativeHashText =
    /** Implements the native fingerprint endpoint in this explicitly isolated transport fixture. */
    function(text) { let hash = 0; for (let index = 0; index < text.length; index++) hash = (Math.imul(hash, 31) + text.charCodeAt(index)) >>> 0; return hash.toString(16); };
  context.module = { exports: {} };
  context.exports = context.module.exports;
  vm.runInContext(source(sdkRoot + "toolpkg/vendor/acorn.js"), context);
  context.__operitAcorn = context.module.exports;
  context.module = { exports: {} };
  vm.runInContext(executionHelpers(), context);
  for (const file of ["ToolPkgComposeDslCompiler.js", "ToolPkgComposeDslRetained.js", "ToolPkgComposeDslReactive.js"]) {
    vm.runInContext(source(sdkRoot + "toolpkg/" + file), context);
  }
  vm.runInContext(embedded(sdkRoot + "toolpkg/ToolPkgComposeDslBridge.rs"), context);
  context.__operitComposeRuntimeSource = wrappedScreen("");
  const fixture = composeStreamFixture(context, { inspectStorage: false });
  context.__operitNativeSetCallStructuredResult =
    /** Completes the command separately from the final response already published on its stream. */
    function(callId, value) {
      assert.equal(value, null);
      const request = pending.get(callId);
      assert.ok(request);
      pending.delete(callId);
      terminal.push({ callId, type: "result", value });
      request.resolve(finalResponses.get(callId));
    };
  context.__operitNativeSendStructuredIntermediate =
    /** Records the genuine structured sink without converting UI nodes to JSON text. */
    function(callId, envelope) {
      const projected = fixture.accept(envelope.phase, envelope.response);
      if (envelope.phase === "final") finalResponses.set(callId, projected);
      else intermediate.push({ callId, value: projected });
    };
  const script = `
    /** Builds a Row whose real callback travels through native terminal delivery. */
    module.exports.default = function(ctx) {
      return ctx.UI.Row({
        /** Returns the generic JSON or actual void result used by this transport fixture. */
        onClick: ${callbackBody}
      }, []);
    };
  `;

  /** Starts the actual engine entry point and waits for its recorded native terminal message. */
  function invoke(callId, name, params) {
    assert.equal(pending.has(callId), false);
    const result = new Promise(
      /** Registers the sole terminal receiver before executing the real JavaScript call. */
      (resolve, reject) => pending.set(callId, { resolve, reject }),
    );
    context.__operitExecuteScriptFunction(callId, {
      __operit_ui_package_name: "arbitrary.callback.fixture",
      __operit_script_screen: "ui/callback/index.ui.js",
      __operit_execution_context_key: "toolpkg_compose:callback-fixture",
      __operit_toolpkg_runtime_kind: "ui",
      ...params,
    }, script, name, 10, 10000, true);
    return result;
  }
  return { invoke, terminal, intermediate, traces };
}

/** Locks only the three generic callback output types and retains both real void variants. */
test("the three authoritative Compose callbacks expose sync and async generic JSON without plugin DTOs", () => {
  const text = source(sdkRoot + "js_sdk/compose_dsl.rs");
  assert.match(text, /use super::toolpkg::ToolPkgJsonValue;/);
  for (const name of ["RowPropsOnClickOutput", "IconButtonPropsOnClickOutput", "ComposeDialogActionOutput"]) {
    const definition = new RegExp(`pub enum ${name} \\{([^}]+)\\}`).exec(text);
    assert.notEqual(definition, null);
    const variants = [...definition[1].matchAll(/Variant\d\(([^;]+?)\),/g)].map(
      /** Keeps declared callback alternatives exact rather than accepting a widened unknown or any. */
      match => match[1].trim(),
    );
    assert.deepEqual(variants, ["()", "JsFuture<()>", "ToolPkgJsonValue", "JsFuture<ToolPkgJsonValue>"]);
  }
});

for (const { component, property, expected, asynchronous } of [
  { component: "Row", property: "onClick", expected: complete, asynchronous: false },
  { component: "Row", property: "onClick", expected: complete, asynchronous: true },
  { component: "IconButton", property: "onClick", expected: cancel, asynchronous: false },
  { component: "IconButton", property: "onClick", expected: complete, asynchronous: true },
  { component: "Dialog", property: "onDismissRequest", expected: cancel, asynchronous: false },
  { component: "Dialog", property: "onDismissRequest", expected: cancel, asynchronous: true },
]) {
  /** Executes the actual serialized callback and checks the V1 envelope on its generic result channel. */
  test(`${component}.${property} retains ${asynchronous ? "awaited" : "synchronous"} JSON actionResult`, async () => {
    const callback = asynchronous
      ? /** Resolves the JSON fixture asynchronously without altering the completion discriminator. */
        async () => expected
      : /** Returns the JSON fixture synchronously without a custom presentation receiver. */
        () => expected;
    const response = await dispatch(compose(component, property, callback), property);
    assert.deepEqual(json(response.actionResult), expected);
    assert.deepEqual(json(response.navigationCommands), []);
  });
}

for (const asynchronous of [false, true]) {
  /** Keeps real void distinct from a manufactured null, completion envelope, or empty JSON result. */
  test(`Row click preserves actual ${asynchronous ? "Promise<void>" : "void"}`, async () => {
    const callback = asynchronous
      ? /** Completes asynchronously with no value. */
        async () => {}
      : /** Completes synchronously with no value. */
        () => {};
    const response = await dispatch(compose("Row", "onClick", callback), "onClick");
    assert.equal(Object.hasOwn(response, "actionResult"), false);
    assert.equal(Object.hasOwn(json(response), "actionResult"), false);
  });
}

/** Verifies awaited rejection is not converted into a successful empty action response. */
test("Row callback Promise rejection propagates from the real dispatch runtime", async () => {
  const context = compose("Row", "onClick",
    /** Rejects the isolated callback fixture with an explicit dependency failure. */
    async () => { throw new Error("fixture callback rejected"); },
  );
  await assert.rejects(dispatch(context, "onClick"), /fixture callback rejected/);
});

/** Confirms generic JSON primitives, arrays, and null are not restricted to a presentation DTO. */
test("Row action channel preserves every generic JSON shape", async () => {
  for (const expected of [null, false, 0, "actionResult", [1, "two", null], { arbitrary: "plugin-owned" }]) {
    const context = compose("Row", "onClick",
      /** Returns one explicit JSON fixture through the existing generic callback channel. */
      () => expected,
    );
    const response = await dispatch(context, "onClick");
    assert.equal(Object.hasOwn(response, "actionResult"), true);
    assert.deepEqual(json(response.actionResult), expected);
  }
});

/** Exercises real JavaScript execution, Promise handling, serialization, and native terminal delivery; this is not a Rust/Flutter host integration test. */
test("native callback transport delivers an awaited Row V1 result exactly once", async () => {
  const transport = nativeTransport(`async function() { await Promise.resolve(); return ${JSON.stringify(complete)}; }`);
  const frame = await transport.invoke("render", "__operit_render_compose_dsl", {});
  const response = await transport.invoke("click", "__operit_dispatch_compose_dsl_action", { actionId: frame.tree.props.onClick.__actionId });
  assert.deepEqual(structuredClone(response.actionResult), complete);
  assert.equal(transport.terminal.length, 2);
  assert.deepEqual(transport.terminal.map(
    /** Separates terminal delivery from intermediate renders and unrelated native traces. */
    entry => ({ callId: entry.callId, type: entry.type }),
  ), [{ callId: "render", type: "result" }, { callId: "click", type: "result" }]);
  assert.ok(transport.intermediate.every(
    /** Confirms intermediate tree updates cannot accidentally emit the final selection result. */
    entry => !Object.hasOwn(entry.value, "actionResult"),
  ));
});

/** Ensures a genuinely void native callback result stays absent after actual JSON serialization. */
test("native callback transport preserves Promise<void> without fabricating actionResult", async () => {
  const transport = nativeTransport("async function() { await Promise.resolve(); }");
  const frame = await transport.invoke("render", "__operit_render_compose_dsl", {});
  const response = await transport.invoke("click", "__operit_dispatch_compose_dsl_action", { actionId: frame.tree.props.onClick.__actionId });
  assert.equal(Object.hasOwn(response, "actionResult"), false);
  assert.equal(transport.terminal.length, 2);
});

/** Verifies the native error endpoint receives rejection, never a null or empty successful response. */
test("native callback transport propagates the rejected Row Promise", async () => {
  const transport = nativeTransport('async function() { throw new Error("native callback rejected"); }');
  const frame = await transport.invoke("render", "__operit_render_compose_dsl", {});
  await assert.rejects(transport.invoke("click", "__operit_dispatch_compose_dsl_action", { actionId: frame.tree.props.onClick.__actionId }), /native callback rejected/);
  assert.deepEqual(transport.terminal.map(
    /** Checks failure remains a native error independently of trace and intermediate traffic. */
    entry => entry.type,
  ), ["result", "error"]);
});

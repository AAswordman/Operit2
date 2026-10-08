import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import test from "node:test";
import vm from "node:vm";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../../../../../", import.meta.url));
const sdk = "core/crates/plugin/sdk/src/toolpkg/";

/** Reads a current exact source path, including generated artifacts, without synthesizing missing files. */
function source(relative) { return readFileSync(path.join(root, relative), "utf8"); }

/** Extracts the existing Rust-owned JavaScript bootstrap instead of rewriting its registry behavior in the test. */
function embeddedScript(file, functionName) {
  const text = source(sdk + file), signature = "pub fn " + functionName + "(";
  assert.equal(text.split(signature).length, 2);
  const start = text.indexOf('r#"', text.indexOf(signature)), end = text.indexOf('"#', start + 3);
  assert.ok(start >= 0 && end > start, "The production bootstrap must contain its actual Rust raw string");
  return text.slice(start + 3, end);
}

const apiRuntime = embeddedScript("ToolPkgApiRuntimeScript.rs", "buildToolPkgApiRuntimeScript");
const registration = embeddedScript("ToolPkgRegistrationBridge.rs", "buildToolPkgRegistrationBridgeScript")
  .replace("__OPERIT_TOOLPKG_REGISTRATION_ONLY__", "true");

/** Executes the actual API runtime and registration bridge with execution metadata only; no business host is supplied. */
function registry(packageName) {
  const context = vm.createContext({
    __operitCurrentCallId: "register:" + packageName,
    /** Supplies only the real API runtime's authenticated registration-version metadata boundary. */
    __operitGetCallState(callId) {
      assert.equal(callId, "register:" + packageName);
      return { params: { toolPkgId: packageName, __operit_toolpkg_api_version: "2.0.0" } };
    },
  });
  vm.runInContext("globalThis.__operitExpose = function(name, value) { globalThis[name] = value; };", context);
  vm.runInContext(apiRuntime, context); vm.runInContext(registration, context);
  return {
    /** Evaluates registration inputs inside the actual runtime realm to retain real JSON prototype semantics. */
    evaluate(script) { return vm.runInContext(script, context); },
    /** Registers a real same-package Compose DSL route through SDK capture without fabricating a host route catalog. */
    registerRoute(id) {
      const route = "toolpkg:" + packageName + ":ui:" + id;
      vm.runInContext(
        "ToolPkg.registerUiRoute({id:" + JSON.stringify(id) + ",route:" + JSON.stringify(route) + ",runtime:'compose_dsl',screen:'ui/panel.js'});", context,
      );
      return route;
    },
    /** Retrieves serialized registrations captured by the actual bootstrap, not a fabricated catalog. */
    entries() { return JSON.parse(vm.runInContext("JSON.stringify(__operitToolPkgRegistrationCapture.navigationEntries.map(JSON.parse))", context)); },
  };
}

/** Registers two unrelated plugin packages through the real bridge without any character or memory API dependency. */
test("real registration bootstrap accepts attachment entries from two arbitrary independent plugins", () => {
  for (const packageName of ["com.example.alpha", "org.example.beta"]) {
    const actual = registry(packageName), route = "toolpkg:" + packageName + ":ui:picker";
    actual.evaluate(`ToolPkg.registerNavigationEntry({id:'custom',surface:'chat_attachments',route:${JSON.stringify(route)},title:{en:'Custom',zh:'自定义附件'},icon:'attachment',order:7,params:{screen:'opaque',values:['9223372036854775807',null,true,1.5]}});`);
    assert.deepEqual(actual.entries(), [{ id: "custom", surface: "chat_attachments", route, title: { en: "Custom", zh: "自定义附件" }, icon: "attachment", order: 7, params: { screen: "opaque", values: ["9223372036854775807", null, true, 1.5] } }]);
  }
});

/** Checks the exact supported surface set in the real bridge while preserving existing navigation surfaces. */
test("real navigation registration rejects unsupported case whitespace and unknown surfaces", () => {
  const actual = registry("arbitrary");
  actual.registerRoute("tab");
  for (const surface of ["toolbox", "main_sidebar_plugins", "app_bar", "chat_attachments", "chat_sidebar_tabs"]) {
    actual.evaluate(`ToolPkg.registerNavigationEntry({id:${JSON.stringify(surface)},surface:${JSON.stringify(surface)},route:'toolpkg:arbitrary:ui:tab'});`);
  }
  assert.equal(actual.entries().length, 5);
  for (const surface of ["", "TOOLBOX", "CHAT_ATTACHMENTS", "chat_attachments ", " chat_attachments", "attachments", "CHAT_SIDEBAR_TABS", "chat_sidebar_tabs ", " chat_sidebar_tabs", "chat_sidebar_tab"]) {
    assert.throws(
      /** Passes the explicit invalid surface to the existing production validator. */
      () => actual.evaluate(`ToolPkg.registerNavigationEntry({id:'invalid',surface:${JSON.stringify(surface)},route:'route'});`),
      /** Requires the original exact error instead of silently normalized capture. */
      error => error.message === "registerNavigationEntry.surface is unsupported: " + surface,
    );
  }
  assert.equal(actual.entries().length, 5);
});

/** Proves capture serializes the original nested input without modifying it or retaining mutable references. */
test("real navigation capture retains immutable opaque params despite later plugin mutations", () => {
  const actual = registry("independent");
  actual.evaluate(`var input={nested:{ids:['9223372036854775807']},values:[null,true,1.5]}; var before=JSON.stringify(input); ToolPkg.registerNavigationEntry({id:'first',surface:'chat_attachments',route:'registered-route',params:input}); if(JSON.stringify(input)!==before) throw new Error('Registry mutated input'); input.nested.ids[0]='changed'; input.values.push('later'); ToolPkg.registerNavigationEntry({id:'second',surface:'chat_attachments',route:'registered-route',params:input});`);
  assert.deepEqual(actual.entries()[0].params, { nested: { ids: ["9223372036854775807"] }, values: [null, true, 1.5] });
  assert.deepEqual(actual.entries()[1].params, { nested: { ids: ["changed"] }, values: [null, true, 1.5, "later"] });
  const returned = actual.entries(); returned[0].params.nested.ids[0] = "changed by consumer";
  assert.equal(actual.entries()[0].params.nested.ids[0], "9223372036854775807");
});

/** Keeps absent optional input and genuine JSON scalar, array, and object input distinct without inserting empty objects. */
test("real navigation registration preserves omitted params and complete JSON values", () => {
  const actual = registry("arbitrary");
  actual.evaluate("ToolPkg.registerNavigationEntry({id:'absent',surface:'chat_attachments',route:'registered-route'});");
  assert.equal(Object.hasOwn(actual.entries()[0], "params"), false);
  for (const input of [null, "opaque", false, 2.5, ["opaque", null, { nested: true }], { explicit: "payload" }]) {
    actual.evaluate(`ToolPkg.registerNavigationEntry({id:'json',surface:'chat_attachments',route:'registered-route',params:${JSON.stringify(input)}});`);
    assert.deepEqual(actual.entries().at(-1).params, input);
  }
});

/** Prevents JSON serialization from silently losing or replacing invalid opaque input. */
test("real registration rejects non JSON params without partially capturing an entry", () => {
  const actual = registry("arbitrary");
  for (const expression of ["undefined", "NaN", "Infinity", "function(){}", "new Date()", "Symbol('invalid')", "({nested:undefined})", "[undefined]", "(() => { const value={}; value.self=value; return value; })()", "(() => { const value={}; value[Symbol('hidden')]=1; return value; })()"]) {
    assert.throws(
      /** Sends each malformed input to the production validator without replacing the serializer. */
      () => actual.evaluate(`ToolPkg.registerNavigationEntry({id:'invalid',surface:'chat_attachments',route:'registered-route',params:${expression}});`),
      /registerNavigationEntry\.params/,
    );
  }
  assert.deepEqual(actual.entries(), []);
});

/** Rejects attachment callbacks before capture while preserving existing action callbacks on the other surfaces. */
test("real attachment registration rejects actions but other navigation surfaces still accept durable callbacks", () => {
  const actual = registry("arbitrary");
  for (const expression of ["function(){}", "({function:'registeredCallback'})", "'registeredCallback'"]) {
    assert.throws(
      /** Supplies a valid route together with the forbidden attachment action. */
      () => actual.evaluate(`ToolPkg.registerNavigationEntry({id:'invalid',surface:'chat_attachments',route:'registered-route',action:${expression}});`),
      /** Requires the production registration error rather than a later Flutter consumer rejection. */
      error => error.message === "registerNavigationEntry.action is unsupported for chat_attachments",
    );
    assert.deepEqual(actual.entries(), []);
  }
  assert.throws(
    /** Requires the legacy registration alias to enforce the same attachment boundary. */
    () => actual.evaluate("registerToolPkgNavigationEntry({id:'invalid',surface:'chat_attachments',route:'registered-route',action:function(){}});"),
    /registerNavigationEntry\.action is unsupported for chat_attachments/,
  );
  actual.evaluate("function existingAction(){}; globalThis.__operitGetActiveModuleExports=function(){return {existingAction:existingAction};};");
  for (const surface of ["toolbox", "main_sidebar_plugins", "app_bar"]) {
    actual.evaluate(`ToolPkg.registerNavigationEntry({id:${JSON.stringify(surface)},surface:${JSON.stringify(surface)},action:existingAction});`);
  }
  assert.equal(actual.entries().length, 3);
  for (const entry of actual.entries()) assert.deepEqual(entry.action, { function: "existingAction" });
});

/** Requires a concrete attachment route during bootstrap registration, without synthesizing a UI target. */
test("real attachment registration requires a route before capture", () => {
  const actual = registry("arbitrary");
  for (const route of ["undefined", "null", "''", "'   '", "42"]) {
    assert.throws(
      /** Passes missing, blank, and wrongly typed routes to the existing attachment validator. */
      () => actual.evaluate(`ToolPkg.registerNavigationEntry({id:'invalid',surface:'chat_attachments',route:${route}});`),
      /** Preserves the explicit route-required error rather than accepting an unusable registered item. */
      error => error.message === "registerNavigationEntry.route is required for chat_attachments",
    );
  }
  assert.deepEqual(actual.entries(), []);
});

/** Checks actual generated metadata and typed Flutter fields; this is not a Flutter rendering or Rust execution test. */
test("generated proxy metadata really contains typed navigation params in catalog and runtime models", () => {
  const schema = JSON.parse(source("core/generated/core_proxy_schema.json"));
  for (const name of ["operit_plugin_sdk::toolpkg::ToolPkgPackageModels::ToolPkgNavigationEntry", "operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgNavigationEntryRuntime"]) {
    const model = schema.types[name]; assert.ok(model, "The actual generated schema must contain " + name);
    assert.deepEqual(model.fields.filter(
      /** Reads one exact generated field rather than inferring a property from toJson output. */
      field => field.name === "params",
    ), [{ name: "params", type: "Option<serde_json::Value>" }]);
  }
  const models = source("apps/flutter/app/lib/core/proxy/generated/CoreProxyModels.g.dart");
  for (const name of ["ToolPkgNavigationEntry", "ToolPkgNavigationEntryRuntime"]) {
    const marker = "class " + name + " {", start = models.indexOf(marker), end = models.indexOf("\n}\n", start);
    assert.ok(start >= 0 && end > start);
    assert.match(models.slice(start, end), /final Object\? params;/);
    assert.match(models.slice(start, end), /params: json\['params'\]/);
  }
  const declarations = source("plugins/types/toolpkg.d.ts");
  const surface = declarations.match(/export type NavigationSurface = ([^;]+);/);
  assert.ok(surface, "The actual generated SDK declaration must expose its navigation surface type");
  assert.equal(surface[1], '"toolbox" | "main_sidebar_plugins" | "app_bar" | "chat_attachments" | "chat_sidebar_tabs"',
    "Generated SDK surface declaration must be regenerated from the finalized Rust source");
  assert.match(declarations, /interface NavigationEntryRegistration\s*\{[\s\S]*?params\?: JsonValue;/);
});

/** Captures actual route-embedded sidebar tabs from two unrelated plugins without chat.list.sections or business DTOs. */
test("real registration accepts sidebar tabs from two arbitrary packages and preserves title icon order and params", () => {
  for (const [packageName, order, icon] of [["com.example.alpha", 30, "Dashboard"], ["org.example.beta", 5, "Folder"]]) {
    const actual = registry(packageName), route = actual.registerRoute("panel");
    actual.evaluate(`ToolPkg.registerNavigationEntry({id:'panel-tab',surface:'chat_sidebar_tabs',route:${JSON.stringify(route)},title:{en:'Custom panel',zh:'自定义面板'},icon:${JSON.stringify(icon)},order:${order},params:{mode:'opaque',ids:['9223372036854775807'],values:[null,true,1.5]}});`);
    assert.deepEqual(actual.entries(), [{
      id: "panel-tab", surface: "chat_sidebar_tabs", route, title: { en: "Custom panel", zh: "自定义面板" }, icon, order,
      params: { mode: "opaque", ids: ["9223372036854775807"], values: [null, true, 1.5] },
    }]);
    assert.equal(Object.hasOwn(actual.entries()[0], "action"), false);
  }
});

/** Respects existing automatic UI route identifiers and toolbox UI routes without inventing another route registry. */
test("real sidebar tab registration accepts existing declared automatic and toolbox UI routes", () => {
  const actual = registry("org.example.automatic");
  actual.evaluate("ToolPkg.registerUiRoute({id:'automatic',screen:'ui/automatic.js'}); ToolPkg.registerToolboxUiModule({id:'toolbox',runtime:'compose_dsl',screen:'ui/toolbox.js'});");
  for (const id of ["automatic", "toolbox"]) {
    const route = "toolpkg:org.example.automatic:ui:" + id;
    actual.evaluate(`ToolPkg.registerNavigationEntry({id:${JSON.stringify(id)},surface:'chat_sidebar_tabs',route:${JSON.stringify(route)}});`);
  }
  assert.deepEqual(actual.entries(), [
    { id: "automatic", surface: "chat_sidebar_tabs", route: "toolpkg:org.example.automatic:ui:automatic" },
    { id: "toolbox", surface: "chat_sidebar_tabs", route: "toolpkg:org.example.automatic:ui:toolbox" },
  ]);
});

/** Proves opaque sidebar input is neither modified during registration nor retained as a mutable reference. */
test("real sidebar tab registration captures immutable params without interpreting plugin fields", () => {
  const actual = registry("org.example.immutable"), route = actual.registerRoute("panel");
  actual.evaluate(`var input={nested:{ids:['9223372036854775807']},values:[null,true,1.5]}; var before=JSON.stringify(input); ToolPkg.registerNavigationEntry({id:'panel-tab',surface:'chat_sidebar_tabs',route:${JSON.stringify(route)},params:input}); if(JSON.stringify(input)!==before) throw new Error('Registry modified sidebar params'); input.nested.ids[0]='changed after registration'; input.values.push('changed');`);
  assert.deepEqual(actual.entries()[0].params, { nested: { ids: ["9223372036854775807"] }, values: [null, true, 1.5] });
  const returned = actual.entries(); returned[0].params.nested.ids[0] = "changed by consumer";
  assert.deepEqual(actual.entries()[0].params, { nested: { ids: ["9223372036854775807"] }, values: [null, true, 1.5] });
});

/** Requires the sidebar route before serialization rather than allowing a host consumer to discover an unusable tab. */
test("real sidebar tab registration rejects missing blank and wrongly typed routes", () => {
  const actual = registry("org.example.required");
  for (const route of ["undefined", "null", "''", "'   '", "42"]) {
    assert.throws(
      /** Passes each invalid route through the actual SDK bridge with no registered business data. */
      () => actual.evaluate(`ToolPkg.registerNavigationEntry({id:'invalid',surface:'chat_sidebar_tabs',route:${route}});`),
      /** Requires the precise formal route-required error. */
      error => error.message === "registerNavigationEntry.route is required for chat_sidebar_tabs",
    );
  }
  assert.deepEqual(actual.entries(), []);
});

/** Rejects sidebar callbacks through both existing registration names without weakening attachment or toolbox contracts. */
test("real sidebar tab registration rejects every action callback before capture", () => {
  const actual = registry("org.example.actions"), route = actual.registerRoute("panel");
  for (const expression of ["function(){}", "({function:'registeredCallback'})", "'registeredCallback'", "false"]) {
    assert.throws(
      /** Combines a real owned route with each explicitly unsupported callback representation. */
      () => actual.evaluate(`ToolPkg.registerNavigationEntry({id:'invalid',surface:'chat_sidebar_tabs',route:${JSON.stringify(route)},action:${expression}});`),
      /** Rejects the action at registration time rather than hiding a Flutter consumption failure. */
      error => error.message === "registerNavigationEntry.action is unsupported for chat_sidebar_tabs",
    );
  }
  assert.throws(
    /** Exercises the actual legacy alias of the same formal registration API. */
    () => actual.evaluate(`registerToolPkgNavigationEntry({id:'invalid',surface:'chat_sidebar_tabs',route:${JSON.stringify(route)},action:function(){}});`),
    /registerNavigationEntry\.action is unsupported for chat_sidebar_tabs/,
  );
  assert.deepEqual(actual.entries(), []);
});

/** Requires the requested route to be declared in the authenticated package without allowing a foreign route to be claimed. */
test("real sidebar tab registration rejects foreign unregistered and differently cased owned routes", () => {
  const actual = registry("org.example.owner"); actual.registerRoute("panel");
  for (const route of ["toolpkg:org.example.other:ui:panel", "toolpkg:org.example.owner:ui:missing", "toolpkg:org.example.owner:ui:PANEL"]) {
    const message = route === "toolpkg:org.example.other:ui:panel"
      ? "registerNavigationEntry.route must belong to this package for chat_sidebar_tabs: " + route
      : "registerNavigationEntry.route is not registered for chat_sidebar_tabs: " + route;
    assert.throws(
      /** Uses exact canonical route identities, not substring-based ownership inference. */
      () => actual.evaluate(`ToolPkg.registerNavigationEntry({id:'invalid',surface:'chat_sidebar_tabs',route:${JSON.stringify(route)}});`),
      /** Preserves the actual explicit owner or missing-route error. */
      error => error.message === message,
    );
  }
  actual.evaluate("ToolPkg.registerUiRoute({id:'claimed',route:'toolpkg:org.example.other:ui:claimed',runtime:'compose_dsl',screen:'ui/claimed.js'});");
  assert.throws(
    /** A foreign route declaration does not transfer its canonical owner to this plugin. */
    () => actual.evaluate("ToolPkg.registerNavigationEntry({id:'invalid',surface:'chat_sidebar_tabs',route:'toolpkg:org.example.other:ui:claimed'});"),
    /registerNavigationEntry\.route must belong to this package for chat_sidebar_tabs/,
  );
  assert.deepEqual(actual.entries(), []);
});

/** Rejects ambiguous owner routes and unsupported render runtimes in the real registration bridge. */
test("real sidebar tab registration rejects duplicate route owners and non Compose DSL routes", () => {
  const duplicate = registry("org.example.duplicate"), route = duplicate.registerRoute("panel"); duplicate.registerRoute("panel");
  assert.throws(
    /** Requires uniqueness even when two declarations repeat the same otherwise valid route. */
    () => duplicate.evaluate(`ToolPkg.registerNavigationEntry({id:'invalid',surface:'chat_sidebar_tabs',route:${JSON.stringify(route)}});`),
    /** Keeps the exact duplicate declaration failure. */
    error => error.message === "registerNavigationEntry.route is duplicated for chat_sidebar_tabs: " + route,
  );
  assert.deepEqual(duplicate.entries(), []);
  const invalid = registry("org.example.runtime");
  invalid.evaluate("ToolPkg.registerUiRoute({id:'panel',route:'toolpkg:org.example.runtime:ui:panel',runtime:'unsupported',screen:'ui/panel.js'});");
  assert.throws(
    /** Prevents a registered non-Compose route from becoming an embedded sidebar tab. */
    () => invalid.evaluate("ToolPkg.registerNavigationEntry({id:'invalid',surface:'chat_sidebar_tabs',route:'toolpkg:org.example.runtime:ui:panel'});"),
    /registerNavigationEntry\.route must use compose_dsl for chat_sidebar_tabs/,
  );
  assert.deepEqual(invalid.entries(), []);
});

/** Keeps the same strict JSON validator for sidebar params instead of silently dropping unsupported values. */
test("real sidebar tab registration rejects non JSON params without partially capturing a tab", () => {
  const actual = registry("org.example.json"), route = actual.registerRoute("panel");
  for (const expression of ["undefined", "NaN", "({nested:undefined})", "(() => {const value={}; value.self=value; return value;})()"] ) {
    assert.throws(
      /** Sends each malformed opaque input through the production validator after resolving a real package route. */
      () => actual.evaluate(`ToolPkg.registerNavigationEntry({id:'invalid',surface:'chat_sidebar_tabs',route:${JSON.stringify(route)},params:${expression}});`),
      /registerNavigationEntry\.params/,
    );
  }
  assert.deepEqual(actual.entries(), []);
});

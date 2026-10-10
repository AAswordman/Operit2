import { composeStreamFixture } from '../../../../../tools/tests/support/compose_stream_fixture.mjs';
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";

/** Extracts one real Rust-owned SDK script; this harness does not implement Compose factories or action dispatch. */
function sdkScript(file, name) {
  const source = readFileSync(new URL("../../../../../core/crates/plugin/sdk/src/toolpkg/" + file, import.meta.url), "utf8");
  const signature = "pub fn " + name + "(";
  assert.equal(source.split(signature).length, 2);
  const start = source.indexOf('r#"', source.indexOf(signature)), end = source.indexOf('"#', start + 3);
  assert.ok(start >= 0 && end > start, "The actual Compose SDK script must exist");
  return source.slice(start + 3, end);
}

/** Finds exact real nodes in the SDK-serialized tree rather than replacing its rendering with labelled fixture nodes. */
export function composeNodes(tree, predicate) {
  const found = [];
  /** Traverses only actual serialized child lists, preserving each node's registered action identity. */
  function visit(node) {
    if (predicate(node)) found.push(node);
    assert.ok(Array.isArray(node.children), "SDK nodes must declare their actual serialized children");
    for (const child of node.children) visit(child);
  }
  visit(tree); return found;
}

/** Selects exactly one node by its real stable key from the SDK-rendered native tree. */
export function keyedNode(tree, key) {
  const found = composeNodes(tree,
    /** Matches the complete explicitly assigned native component key. */
    node => node.props.key === key,
  );
  assert.equal(found.length, 1, "Expected exactly one actual native node: " + key); return found[0];
}

/** Mounts one exact registered Compose handler on the production SDK context and JSON action-result runtime. */
export function mountRegisteredComposeRoute(plugin, presentation, routeId, hostState = {}) {
  const routes = plugin.routes.filter(
    /** Requires the exact production route rather than selecting a management handler with similar fields. */
    route => route.id === routeId,
  );
  assert.equal(routes.length, 1); assert.equal(typeof routes[0].screen, "function");
  assert.equal(routes[0].route, "toolpkg:com.operit.character_cards:ui:" + routeId);
  const module = { exports: { Screen: routes[0].screen } };
  const context = vm.createContext({ module, exports: module.exports, console, setTimeout, clearTimeout });
  for (const file of ["ToolPkgComposeDslCompiler.js", "ToolPkgComposeDslRetained.js", "ToolPkgComposeDslReactive.js"]) {
    vm.runInContext(readFileSync(new URL("../../../../../core/crates/plugin/sdk/src/toolpkg/" + file, import.meta.url), "utf8"), context);
  }
  vm.runInContext(sdkScript("ToolPkgComposeDslBridge.rs", "buildComposeDslContextBridgeDefinition"), context);
  const wrapped = sdkScript("ToolPkgComposeDslRuntimeScript.rs", "buildComposeDslRuntimeWrappedScript");
  assert.equal(wrapped.split("{script}").length, 2);
  vm.runInContext(wrapped.replace("{script}", "").replaceAll("{{", "{").replaceAll("}}", "}"), context);
  composeStreamFixture(context).adapt();
  const options = { state: { ...hostState, presentation }, packageName: "com.operit.character_cards", toolPkgId: "com.operit.character_cards", uiModuleId: routeId, routeInstanceId: "native-selector-test", executionContextKey: "selector-test" };
  const first = module.exports.__operit_render_compose_dsl(options);
  return {
    first,
    /** Rerenders using the actual retained SDK state and controller reference. */
    render() { return context.__operit_render_compose_dsl({ executionContextKey: options.executionContextKey, __operit_update_inputs: true }); },
    /** Supplies a changed real request to verify the plugin refuses stale or retargeted modal state. */
    retarget(value) { context.__operit_compose_bundle.ctx.useState("presentation", null)[1](value); return context.__operit_render_compose_dsl({ executionContextKey: options.executionContextKey, __operit_update_inputs: true }); },
    /** Updates explicit host state through the real SDK to test retained callback ownership. */
    updateHostState(values) {
      for (const [key, value] of Object.entries(values)) context.__operit_compose_bundle.ctx.useState(key, null)[1](value);
      return context.__operit_render_compose_dsl({ executionContextKey: options.executionContextKey, __operit_update_inputs: true });
    },
    /** Dispatches one encoded callback through the actual generic SDK action-result channel. */
    dispatch(action) {
      assert.equal(typeof action.__actionId, "string");
      return module.exports.__operit_dispatch_compose_dsl_action({ actionId: action.__actionId });
    },
    /** Loads the real selector through its root callback, not a test-side catalog provider. */
    async load() { return this.dispatch(first.tree.props.onLoad); },
  };
}

/** Mounts only the actual native selector, retaining its existing independently verified route identity. */
export function mountRegisteredSelector(plugin, presentation) {
  return mountRegisteredComposeRoute(plugin, presentation, "selection");
}

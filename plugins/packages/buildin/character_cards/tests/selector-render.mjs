import { readSdkScript, wrapComposeScript } from "../../../../../tools/runtime/compose_scripts.mjs";
import { composeStreamFixture } from '../../../../../tools/tests/support/compose_stream_fixture.mjs';
import assert from "node:assert/strict";
import vm from "node:vm";

/** Extracts one real Rust-owned SDK script; this harness does not implement Compose factories or action dispatch. */
function sdkScript(file, name) { return readSdkScript(file, name); }

/** Finds exact real nodes in the SDK-serialized tree rather than replacing its rendering with labelled fixture nodes. */
export function composeNodes(tree, predicate) {
  const found = [];
  /** Traverses only actual serialized child lists, preserving each node's registered action identity. */
  function visit(node) {
    if (predicate(node)) found.push(node);
    assert.ok(Array.isArray(node.children), "SDK nodes must declare their actual serialized children");
    for (const child of node.children) visit(child);
    for (const slot of Object.values(node.slots ?? {})) for (const child of slot) visit(child);
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
export function mountRegisteredComposeRoute(plugin, presentation, routeId, hostState = {}, webViewCommands = null) {
  const routes = plugin.routes.filter(
    /** Requires the exact production route rather than selecting a management handler with similar fields. */
    route => route.id === routeId,
  );
  assert.equal(routes.length, 1); assert.equal(typeof routes[0].screen, "function");
  assert.equal(routes[0].route, "toolpkg:com.operit.character_cards:ui:" + routeId);
  const pageRuntime = plugin.openComposeRoute?.(routeId);
  const screen = pageRuntime?.screen ?? routes[0].screen;
  const exportName = screen.__operit_toolpkg_export_name ?? "default";
  const module = { exports: { [exportName]: screen } };
  Object.defineProperty(module.exports, "__operit_compose_entry_export", { value: exportName });
  const context = pageRuntime?.context ?? vm.createContext({ console, setTimeout, clearTimeout });
  context.module = module;
  context.exports = module.exports;
  if (webViewCommands !== null) {
    context.__operitNativeComposeWebViewControllerCommand = request => ({ success: true, data: webViewCommands(request) ?? null });
    context.__operitNativeComposeWebViewControllerCommandSuspend = async request => ({ success: true, data: await webViewCommands(request) ?? null });
    /** Uses the documented optional decoder in the host Promise ABI. */
    context.__operitInvokeHostAsync = async (command, args, decode = value => value) => decode(await command(...args));
  }
  if (!context.OperitComposeDslRuntime) {
    vm.runInContext(sdkScript("ToolPkgComposeDslBridge.rs", "buildComposeDslContextBridgeDefinition"), context);
  }
  vm.runInContext(wrapComposeScript(), context);
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
    dispatch(action, payload) {
      assert.equal(typeof action.__actionId, "string");
      return module.exports.__operit_dispatch_compose_dsl_action({ actionId: action.__actionId, ...(payload === undefined ? {} : { payload }) });
    },
    /** Loads the real selector through its root callback, not a test-side catalog provider. */
    async load() { return this.dispatch(first.tree.props.onLoad); },
  };
}

/** Mounts only the actual native selector, retaining its existing independently verified route identity. */
export function mountRegisteredSelector(plugin, presentation) {
  return mountRegisteredComposeRoute(plugin, presentation, "selection");
}

/// Resolves a registered entry exactly before handling standalone screen-module conventions.
function resolveComposeEntry() {
  if (typeof module !== 'undefined' && module && module.exports && Object.prototype.hasOwnProperty.call(module.exports, '__operit_compose_entry_export')) {
    var exportName = module.exports.__operit_compose_entry_export;
    var registeredEntry = module.exports[exportName];
    if (typeof registeredEntry !== 'function') {
      throw new Error('Registered compose_dsl screen export is not a function: ' + exportName);
    }
    return registeredEntry;
  }
  if (typeof module !== 'undefined' && module && module.exports) {
    if (typeof module.exports.default === 'function') {
      return module.exports.default;
    }
    if (typeof module.exports.Screen === 'function') {
      return module.exports.Screen;
    }
  }
  if (typeof exports !== 'undefined' && exports) {
    if (typeof exports.default === 'function') {
      return exports.default;
    }
    if (typeof exports.Screen === 'function') {
      return exports.Screen;
    }
  }
  if (typeof window !== 'undefined') {
    if (typeof window.default === 'function') {
      return window.default;
    }
    if (typeof window.Screen === 'function') {
      return window.Screen;
    }
  }
  return null;
}
function renderComposeDsl(runtimeOptions) {
  if (typeof OperitComposeDslRuntime === 'undefined') {
    throw new Error('OperitComposeDslRuntime bridge is not initialized');
  }
  var root = typeof globalThis !== 'undefined' ? globalThis : typeof window !== 'undefined' ? window : this;
  var activeCallRuntime = typeof root.__operit_call_runtime_ref === 'object' && root.__operit_call_runtime_ref ? root.__operit_call_runtime_ref : null;
  var options = runtimeOptions && typeof runtimeOptions === 'object' ? Object.assign({}, runtimeOptions) : {};
  if (activeCallRuntime) {
    options.__operit_call_runtime = activeCallRuntime;
  }
  var previous = root.__operit_compose_bundle;
  var sessionId = String(options.__operit_compose_session_id || '');
  var sameSession = !sessionId || previous && previous.composeSessionId === sessionId;
  var sameContext = sameSession && previous && previous.executionContextKey === String(options.executionContextKey || options.__operit_compose_execution_context_key || '');
  // Input updates preserve live refs and async ownership only within the same context.
  if (options.__operit_update_inputs === true && sameContext && typeof root.__operit_compose_entry === 'function') {
    return refreshComposeInputs(options);
  }
  var reuse = !!sameContext;
  var bundle = reuse ? previous : OperitComposeDslRuntime.createContext(options);
  bundle.composeSessionId = sessionId;
  if (reuse || options.__operit_update_inputs === true) {
    bundle.updateRuntimeOptions(options);
  }
  if (reuse) OperitComposeReactive.invalidateRoot(bundle.ctx);
  var entry = resolveComposeEntry();
  if (typeof entry !== 'function') {
    throw new Error('compose_dsl entry function not found, expected default export or Screen function');
  }
  if (activeCallRuntime) {
    bundle.setCallRuntime(activeCallRuntime);
  }
  root.__operit_compose_bundle = bundle;
  root.__operit_compose_entry = entry;
  return buildComposeResponse(bundle, entry, undefined, true, requireComposeResponseSink(activeCallRuntime));
}
function refreshComposeInputs(runtimeOptions) {
  var root = typeof globalThis !== 'undefined' ? globalThis : typeof window !== 'undefined' ? window : this;
  var bundle = root.__operit_compose_bundle;
  var entry = root.__operit_compose_entry;
  if (!bundle || typeof entry !== 'function') {
    throw new Error('compose_dsl runtime is not initialized, render first');
  }
  var activeCallRuntime = typeof root.__operit_call_runtime_ref === 'object' && root.__operit_call_runtime_ref ? root.__operit_call_runtime_ref : null;
  var options = runtimeOptions && typeof runtimeOptions === 'object' ? Object.assign({}, runtimeOptions) : {};
  if (activeCallRuntime) {
    options.__operit_call_runtime = activeCallRuntime;
  }
  bundle.updateRuntimeOptions(options);
  if (activeCallRuntime) {
    bundle.setCallRuntime(activeCallRuntime);
  }
  return buildComposeResponse(bundle, entry, undefined, true, requireComposeResponseSink(activeCallRuntime));
}

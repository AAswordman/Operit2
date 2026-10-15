/// Creates one context with stores and services owned by its exact session.
var OperitComposeDslRuntime = function () {
  function createContext(runtimeOptions) {
    var options = runtimeOptions && typeof runtimeOptions === 'object' ? runtimeOptions : {};
    var runtime = {
      profileStages: options.__operit_profile_compose_stages === true,
      stateStore: OperitComposeHost.copy(options.state),
      memoStore: OperitComposeHost.copy(options.memo),
      moduleSpec: options.moduleSpec && typeof options.moduleSpec === 'object' ? options.moduleSpec : {},
      packageName: String(options.packageName || options.__operit_ui_package_name || ''),
      toolPkgId: String(options.toolPkgId || options.__operit_ui_toolpkg_id || ''),
      uiModuleId: String(options.uiModuleId || options.__operit_ui_module_id || ''),
      routeInstanceId: String(options.routeInstanceId || options.__operit_route_instance_id || ''),
      executionContextKey: String(options.executionContextKey || options.__operit_compose_execution_context_key || ''),
      callRuntime: options.__operit_call_runtime && typeof options.__operit_call_runtime === 'object' ? options.__operit_call_runtime : null,
      webViewInterfaces: new Map(),
      navigationCommands: [],
      stateChangeListeners: [],
      stateChangeScheduled: false,
      stateDirty: false,
      pendingStateChangePromise: null
    };
    runtime.actions = OperitComposeActions.create();
    var state = OperitComposeState.create(runtime);
    var theme = OperitComposeTheme.create(runtime, options);
    var ctx = Object.assign({}, state.api, OperitComposeHost.create(runtime), {
      Theme: theme.api,
      MaterialTheme: theme.material,
      createWebViewController: function (key) {
        return OperitComposeWebView.create(runtime, key);
      },
      /// Generates the visible item window on the original session context.
      lazyItems: function (options, itemBuilder) {
        return OperitComposeLazy.items(ctx, options, itemBuilder);
      },
      h: function (type, props, children) {
        return OperitComposeNodes.build(runtime, type, props, children);
      },
      Modifier: OperitComposeNodes.modifier([]),
      UI: OperitComposeNodes.registry(runtime)
    });
    runtime.updateRuntimeOptions = function (next) {
      if (!next || typeof next !== 'object') return;
      if (next.__operit_update_inputs === true) {
        var input = OperitComposeHost.copy(next.__operit_input_state);
        Object.keys(input).forEach(
        /// Publishes host writes through the same dependency index as useState setters.
        function (key) {
          runtime.stateStore[key] = input[key];
          OperitComposeReactive.stateChanged(runtime.ctx, key);
        });
      }
      if (next.theme !== undefined) theme.update(next.theme);
      if (next.__operit_call_runtime) runtime.callRuntime = next.__operit_call_runtime;
    };
    runtime.ctx = ctx;
    OperitComposeGeneration.attach(ctx);
    OperitComposeReactive.attach(ctx);
    runtime.composition = OperitComposeRetained.create(runtime);
    /// Transfers each navigation request to exactly one render response.
    runtime.takeNavigationCommands = function () {
      return runtime.navigationCommands.splice(0);
    };
    runtime.invokeAction = function (actionId, payload) {
      return runtime.actions.invoke(actionId, payload);
    };
    runtime.subscribeStateChange = state.subscribe;
    runtime.flushStateChanges = state.flush;
    runtime.setCallRuntime = function (callRuntime) {
      runtime.callRuntime = callRuntime;
    };
    return runtime;
  }
  return {
    createContext: createContext
  };
}();

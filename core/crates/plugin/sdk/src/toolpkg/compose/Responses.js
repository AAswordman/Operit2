function isPromise(value) {
  return !!(value && typeof value.then === 'function');
}

/// Records opt-in pipeline timing and cumulative dependency execution counters.
function profileComposeStage(bundle, sink, stage, started, revision) {
  if (!bundle.profileStages) return;
  var finished = Date.now();
  var work = stage === 'js-render-sync' ? OperitComposeReactive.metrics(bundle.ctx) : null;
  console.log('event=compose_pipeline_stage stage=' + stage + ' durationMs=' + (finished - started) +
    ' wallMicros=' + (finished * 1000) + ' context=' + bundle.executionContextKey +
    ' call=' + sink.callId + ' revision=' + revision +
    (work ? ' rootRuns=' + work.rootRuns + ' bindingRuns=' + work.bindingRuns + ' expressionRuns=' + work.expressionRuns : ''));
}

function wrapComposeResponse(bundle, tree, actionResult, includeNavigation, responseSink) {
  var navigation = includeNavigation ? bundle.takeNavigationCommands() : [], update = null;
  if (typeof tree !== 'undefined') {
    var commitStarted = bundle.profileStages ? Date.now() : 0;
    update = bundle.composition.finish(tree, !includeNavigation);
    if (update === null) return null;
    profileComposeStage(bundle, responseSink, 'js-transaction', commitStarted, update.revision);
  }
  var phase = includeNavigation ? 'final' : 'intermediate';
  var frame = update ? update.seal(phase, actionResult, navigation) : bundle.composition.response(phase, actionResult, navigation);
  var sendStarted = bundle.profileStages ? Date.now() : 0;
  responseSink.sendComposeResponse(phase, frame);
  profileComposeStage(bundle, responseSink, 'js-send-inclusive', sendStarted, update ? update.revision : 0);
  return null;
}

/// Resolves an explicit session sink before any retained commit can be generated.
function requireComposeResponseSink(runtime) {
  if (!runtime || typeof runtime.sendComposeResponse !== 'function') {
    throw new Error('Compose stream command requires its call-owned response sink');
  }
  return runtime;
}

/// Builds a response while preserving the caller's navigation delivery policy.
function buildComposeResponse(bundle, entry, actionResult, includeNavigation, responseSink, inputsChanged, refreshOpaqueRoot) {
  var renderStarted = bundle.profileStages ? Date.now() : 0;
  // State notifications drain only indexed changes; eager, final and host renders
  // must also observe ordinary JS closures and mutable refs.
  var tree = OperitComposeReactive.render(bundle.ctx, entry, refreshOpaqueRoot);
  profileComposeStage(bundle, responseSink, 'js-render-sync', renderStarted, 0);
  /// Uses the just-rendered closure so lifecycle reads observe the new host inputs.
  function complete(resolvedTree) {
    var callback = inputsChanged && bundle.composition.lifecycle(resolvedTree, 'onInputsChanged');
    if (callback && callback.__actionId) {
      return runComposeAction(bundle, entry, { actionId: callback.__actionId }, responseSink);
    }
    return wrapComposeResponse(bundle, resolvedTree, actionResult, includeNavigation, responseSink);
  }
  return isPromise(tree) ? tree.then(complete) : complete(tree);
}

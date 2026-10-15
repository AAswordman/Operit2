function isPromise(value) {
  return !!(value && typeof value.then === 'function');
}

/// Keeps navigation queued during intermediate renders until the action completes.
function profileComposeStage(bundle, sink, stage, started, revision) {
  if (!bundle.profileStages) return;
  var finished = Date.now();
  console.log('event=compose_pipeline_stage stage=' + stage + ' durationMs=' + (finished - started) +
    ' wallMicros=' + (finished * 1000) + ' context=' + bundle.executionContextKey +
    ' call=' + sink.callId + ' revision=' + revision);
}

function wrapComposeResponse(bundle, tree, actionResult, includeNavigation, responseSink) {
  var response = {
    navigationCommands: includeNavigation ? bundle.takeNavigationCommands() : []
  };
  if (typeof tree !== 'undefined') {
    var commitStarted = bundle.profileStages ? Date.now() : 0;
    response.update = bundle.composition.commit(tree);
    profileComposeStage(bundle, responseSink, 'js-retained-commit', commitStarted, response.update.revision);
  }
  if (typeof actionResult !== 'undefined') {
    response.actionResult = actionResult;
  }
  var sendStarted = bundle.profileStages ? Date.now() : 0;
  responseSink.sendComposeResponse(includeNavigation ? 'final' : 'intermediate', response);
  profileComposeStage(bundle, responseSink, 'js-send-inclusive', sendStarted, response.update ? response.update.revision : 0);
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
function buildComposeResponse(bundle, entry, actionResult, includeNavigation, responseSink) {
  var renderStarted = bundle.profileStages ? Date.now() : 0;
  var tree = OperitComposeReactive.render(bundle.ctx, entry);
  profileComposeStage(bundle, responseSink, 'js-render-sync', renderStarted, 0);
  if (isPromise(tree)) {
    return tree.then(function (resolvedTree) {
      return wrapComposeResponse(bundle, resolvedTree, actionResult, includeNavigation, responseSink);
    });
  }
  return wrapComposeResponse(bundle, tree, actionResult, includeNavigation, responseSink);
}

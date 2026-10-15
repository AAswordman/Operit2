function dispatchComposeDslAction(actionRequest) {
  var root = typeof globalThis !== 'undefined' ? globalThis : typeof window !== 'undefined' ? window : this;
  var bundle = root.__operit_compose_bundle;
  var entry = root.__operit_compose_entry;
  if (!bundle || typeof entry !== 'function') {
    throw new Error('compose_dsl runtime is not initialized, render first');
  }
  if (typeof bundle.invokeAction !== 'function') {
    throw new Error('compose_dsl runtime action bridge is not available');
  }
  var activeCallRuntime = typeof root.__operit_call_runtime_ref === 'object' && root.__operit_call_runtime_ref ? root.__operit_call_runtime_ref : null;
  var request = actionRequest && typeof actionRequest === 'object' ? actionRequest : {};
  var responseSink = requireComposeResponseSink(activeCallRuntime);
  var runtimeOptions = Object.assign({}, request);
  if (activeCallRuntime) {
    runtimeOptions.__operit_call_runtime = activeCallRuntime;
  }
  bundle.updateRuntimeOptions(runtimeOptions);
  if (activeCallRuntime) {
    bundle.setCallRuntime(activeCallRuntime);
  }
  var actionId = String(request.__action_id || request.actionId || '').trim();
  if (!actionId) {
    throw new Error('compose action id is required');
  }
  var payload = Object.prototype.hasOwnProperty.call(request, '__action_payload') ? request.__action_payload : request.payload;
  var noRender = payload && typeof payload === 'object' && (payload.__no_render === true || payload.__noRender === true || payload.__local === true);
  var actionSettled = false;
  var actionFinalizing = false;
  var intermediateRenderTask = Promise.resolve();
  var intermediateRenderError = null;
  var intermediateRenderQueued = false;
  var intermediateRenderInFlight = false;
  var unsubscribeStateChange = null;
  var composeActionCallId = String(typeof __operitCurrentCallId === 'string' ? __operitCurrentCallId : '');
  function finalizeDetachedStateListener() {
    if (typeof unsubscribeStateChange === 'function') {
      unsubscribeStateChange();
      unsubscribeStateChange = null;
    }
  }

  /// Allows detached work while preventing intermediate revisions from overtaking finalization.
  function canRenderIntermediate() {
    if (!actionSettled) {
      return !actionFinalizing;
    }
    var callId = composeActionCallId;
    var callState = typeof __operitGetCallState === 'function' ? __operitGetCallState(callId) : null;
    return !!(callState && callState.detached);
  }
  function finalizeAction() {
    actionSettled = true;
    var callId = String(typeof __operitCurrentCallId === 'string' ? __operitCurrentCallId : '');
    var callState = typeof __operitGetCallState === 'function' ? __operitGetCallState(callId) : null;
    if (callState && Number(callState.pendingReferences || 0) > 0) {
      callState.detachedCleanup = finalizeDetachedStateListener;
    } else {
      finalizeDetachedStateListener();
    }
  }

  /// Delivers every started commit before the final response can be generated.
  function renderAndSendIntermediate() {
    if (!canRenderIntermediate()) {
      return null;
    }
    return buildComposeResponse(bundle, entry, undefined, false, responseSink);
  }

  /// Owns the in-flight render and delivery task so finalization can await it.
  function processIntermediateQueue() {
    if (!canRenderIntermediate() || intermediateRenderInFlight || !intermediateRenderQueued) {
      return intermediateRenderTask;
    }
    intermediateRenderQueued = false;
    intermediateRenderInFlight = true;
    var renderResponse;
    try {
      renderResponse = renderAndSendIntermediate();
    } catch (renderError) {
      renderResponse = Promise.reject(renderError);
    }
    intermediateRenderTask = Promise.resolve(renderResponse).then(function () {
      intermediateRenderInFlight = false;
      if (intermediateRenderQueued && canRenderIntermediate()) {
        return processIntermediateQueue();
      }
    }, function (renderError) {
      intermediateRenderInFlight = false;
      throw renderError;
    });
    intermediateRenderTask.catch(recordIntermediateError);
    return intermediateRenderTask;
  }

  /// Preserves render failures for the final response and reports errors from detached work.
  function recordIntermediateError(renderError) {
    if (intermediateRenderError !== null) {
      return;
    }
    intermediateRenderError = renderError;
    intermediateRenderQueued = false;
    if (actionSettled) {
      responseSink.reportError(String(renderError && renderError.message || renderError));
      finalizeDetachedStateListener();
    }
  }

  /// Observes scheduled work while the action owns its result and error.
  function scheduleIntermediateRender() {
    if (!canRenderIntermediate()) {
      return;
    }
    intermediateRenderQueued = true;
    Promise.resolve().then(function () {
      return processIntermediateQueue();
    }).catch(recordIntermediateError);
  }
  function flushStateChanges() {
    return Promise.resolve(bundle.flushStateChanges());
  }

  /// Stops new intermediate work and drains committed delivery before producing the final revision.
  function buildFinalResponse(actionResult) {
    return flushStateChanges().then(function () {
      actionFinalizing = true;
      intermediateRenderQueued = false;
      return intermediateRenderTask;
    }).then(function () {
      if (intermediateRenderError) {
        throw intermediateRenderError;
      }
      if (noRender) {
        return wrapComposeResponse(bundle, undefined, actionResult, true, responseSink);
      }
      return buildComposeResponse(bundle, entry, actionResult, true, responseSink);
    });
  }
  if (!noRender) {
    unsubscribeStateChange = bundle.subscribeStateChange(scheduleIntermediateRender);
  }
  var maybePromise;
  try {
    maybePromise = bundle.invokeAction(actionId, payload);
  } catch (actionError) {
    finalizeAction();
    throw actionError;
  }
  // Synchronous callbacks have already settled before queued state notifications.
  // Their final response owns the only render; asynchronous work still streams.
  if (!isPromise(maybePromise)) {
    actionFinalizing = true;
  } else if (!noRender) {
    scheduleIntermediateRender();
  }
  return Promise.resolve(maybePromise).then(buildFinalResponse).finally(finalizeAction);
}

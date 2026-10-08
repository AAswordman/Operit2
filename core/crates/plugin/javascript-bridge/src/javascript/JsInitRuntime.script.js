(function() {
    var root = typeof globalThis !== 'undefined'
        ? globalThis
        : (typeof window !== 'undefined' ? window : this);
    var windowRef = typeof window !== 'undefined' ? window : root;

    function expose(name, value) {
        var key = name == null ? '' : String(name).trim();
        if (!key || value === undefined) {
            return;
        }
        root[key] = value;
        windowRef[key] = value;
    }

    expose('__operitExpose', expose);
    root.window = windowRef;

    function ensureCallRegistry() {
        var registry = root.__operitExecutionCallRegistry;
        if (!registry || typeof registry !== 'object') {
            registry = {};
            root.__operitExecutionCallRegistry = registry;
        }
        return registry;
    }

    function normalizeCallId(callId) {
        return callId == null ? '' : String(callId).trim();
    }

    function getCallState(callId) {
        var resolvedCallId = normalizeCallId(callId);
        if (!resolvedCallId) {
            return null;
        }
        return ensureCallRegistry()[resolvedCallId] || null;
    }

    function registerCallSession(callId, params) {
        var resolvedCallId = normalizeCallId(callId);
        if (!resolvedCallId) {
            throw new Error('callId is required');
        }
        var registry = ensureCallRegistry();
        var state = registry[resolvedCallId];
        var callState = state && typeof state === 'object' ? state : {};
        callState.callId = resolvedCallId;
        callState.params = params && typeof params === 'object' ? params : {};
        callState.completed = false;
        callState.resultCompleted = false;
        callState.detached = false;
        callState.detachedCleanup = null;
        callState.pendingReferences = 0;
        callState.safetyTimeout = null;
        callState.safetyTimeoutFinal = null;
        callState.lastExecStage = '';
        callState.lastExecFunction = '';
        callState.lastModulePath = '';
        callState.lastRequireRequest = '';
        callState.lastRequireFrom = '';
        callState.lastRequireResolved = '';
        callState.currentModule = null;
        callState.currentModuleExports = null;
        callState.timerIds = {};
        callState.previousCallId = '';
        callState.previousCallRuntime = null;
        registry[resolvedCallId] = callState;
        return callState;
    }

    function registerCallTimer(callId, timerId) {
        var callState = getCallState(callId);
        if (!callState) {
            return;
        }
        if (!callState.timerIds || typeof callState.timerIds !== 'object') {
            callState.timerIds = {};
        }
        callState.timerIds[String(timerId)] = true;
        callState.pendingReferences += 1;
    }

    function unregisterCallTimer(callId, timerId) {
        var callState = getCallState(callId);
        if (!callState || !callState.timerIds || typeof callState.timerIds !== 'object') {
            return;
        }
        var key = String(timerId);
        if (!callState.timerIds[key]) return;
        delete callState.timerIds[key];
        releaseCallReference(callId);
    }

    function retainCallReference(callId) {
        var callState = getCallState(callId);
        if (callState) callState.pendingReferences = Number(callState.pendingReferences || 0) + 1;
    }

    function releaseCallReference(callId) {
        var callState = getCallState(callId);
        if (!callState) return;
        callState.pendingReferences = Math.max(0, Number(callState.pendingReferences || 0) - 1);
    }

    /** Finalizes one detached call after its queued JavaScript jobs are delivered. */
    function finalizeDetachedCall(callId) {
        var callState = getCallState(callId);
        if (!callState || !callState.detached || Number(callState.pendingReferences || 0) > 0) {
            return false;
        }
        if (typeof callState.detachedCleanup === 'function') {
            try {
                callState.detachedCleanup();
            } catch (_cleanupError) {
            }
            callState.detachedCleanup = null;
        }
        cleanupCallSession(callId);
        return true;
    }

    /** Reports only the explicit detached flag instead of exposing the whole call object to Rust. */
    function isCallDetached(callId) {
        var state = getCallState(callId);
        return Boolean(state && state.detached === true);
    }

    /** Returns the structured registration capture required by the runtime lifecycle contract. */
    function readRegistrationCapture() {
        return root.__operitToolPkgRegistrationCapture;
    }

    function detachedCallIds() {
        var registry = ensureCallRegistry();
        var result = [];
        Object.keys(registry).forEach(function(callId) {
            var state = registry[callId];
            if (state && state.detached && Number(state.pendingReferences || 0) > 0) result.push(callId);
        });
        return result;
    }

    function activateCall(callId) {
        var state = getCallState(callId);
        if (!state || state.completed) return false;
        root.__operitCurrentCallId = callId;
        root.__operit_call_runtime_ref = state.callRuntime;
        return true;
    }

    // Explicit Promise continuations (including lowered async functions) belong to
    // their subscribing call, not to the call which happens to resolve a shared Promise.
    var originalThen = Promise.prototype.then;
    Promise.prototype.then = function(onFulfilled, onRejected) {
        var ownerCallId = normalizeCallId(root.__operitCurrentCallId);
        if (!getCallState(ownerCallId)) {
            return originalThen.call(this, onFulfilled, onRejected);
        }
        function bindContinuation(handler) {
            if (typeof handler !== 'function') return handler;
            return function(value) {
                var previousCallId = root.__operitCurrentCallId;
                var previousRuntime = root.__operit_call_runtime_ref;
                // Never borrow another session if the originating call was cancelled.
                root.__operitCurrentCallId = ownerCallId;
                root.__operit_call_runtime_ref = (getCallState(ownerCallId) || {}).callRuntime;
                try { return handler(value); }
                finally {
                    root.__operitCurrentCallId = previousCallId;
                    root.__operit_call_runtime_ref = previousRuntime;
                }
            };
        }
        return originalThen.call(this, bindContinuation(onFulfilled), bindContinuation(onRejected));
    };

    /** Activates one detached call before its queued JavaScript jobs are resumed. */
    function prepareDetachedCall(callId) {
        var state = getCallState(callId);
        if (!state || state.completed || !state.detached || Number(state.pendingReferences || 0) <= 0) {
            return false;
        }
        return activateCall(callId);
    }

    /** Cancels every timer owned by the session through the host Promise bridge. */
    function clearCallTimers(callState) {
        if (!callState || !callState.timerIds || typeof callState.timerIds !== 'object') {
            return;
        }
        var timerIds = Object.keys(callState.timerIds);
        for (var index = 0; index < timerIds.length; index += 1) {
            root.clearTimeout(timerIds[index]);
        }
        callState.timerIds = {};
    }

    function cleanupCallSession(callId) {
        var resolvedCallId = normalizeCallId(callId);
        if (!resolvedCallId) {
            return;
        }
        var registry = ensureCallRegistry();
        clearCallTimers(registry[resolvedCallId]);
        delete registry[resolvedCallId];
    }

    function cancelCallSession(callId) {
        var resolvedCallId = normalizeCallId(callId);
        if (!resolvedCallId) {
            return;
        }
        var callState = getCallState(resolvedCallId);
        if (!callState) {
            return;
        }
        callState.completed = true;
        if (root.__operitCurrentCallId === resolvedCallId) {
            root.__operitCurrentCallId = normalizeCallId(callState.previousCallId);
        }
        if (root.__operit_call_runtime_ref === callState.callRuntime) {
            if (callState.previousCallRuntime && typeof callState.previousCallRuntime === 'object') {
                root.__operit_call_runtime_ref = callState.previousCallRuntime;
            } else {
                delete root.__operit_call_runtime_ref;
            }
        }
        cleanupCallSession(resolvedCallId);
    }

    function buildRuntimeContext(callId) {
        var callState = getCallState(callId);
        var mapping = [
            ['lastExecStage', 'stage'],
            ['lastExecFunction', 'function'],
            ['lastModulePath', 'module'],
            ['lastRequireRequest', 'require'],
            ['lastRequireFrom', 'from'],
            ['lastRequireResolved', 'resolved']
        ];
        var parts = [];
        for (var i = 0; i < mapping.length; i += 1) {
            var key = mapping[i][0];
            var label = mapping[i][1];
            var value = callState ? callState[key] : undefined;
            if (value != null && String(value).trim().length > 0) {
                parts.push(label + '=' + String(value));
            }
        }
        return parts.join(', ');
    }

    expose('__operitGetCallState', getCallState);
    expose('__operitRegisterCallSession', registerCallSession);
    expose('__operitRegisterCallTimer', registerCallTimer);
    expose('__operitUnregisterCallTimer', unregisterCallTimer);
    expose('__operitRetainCallReference', retainCallReference);
    expose('__operitReleaseCallReference', releaseCallReference);
    expose('__operitGetDetachedCallIds', detachedCallIds);
    expose('__operitIsCallDetached', isCallDetached);
    expose('__operitReadRegistrationCapture', readRegistrationCapture);
    expose('__operitActivateCall', activateCall);
    expose('__operitPrepareDetachedCall', prepareDetachedCall);
    expose('__operitFinalizeDetachedCall', finalizeDetachedCall);
    /** Forwards the detached-call notification through its mandatory structured host binding. */
    expose('__operitNotifyDetachedCall', function(callId) {
        root.__operitNativeNotifyDetachedCall(normalizeCallId(callId));
    });
    expose('__operitCleanupCallSession', cleanupCallSession);
    expose('__operitCancelCallSession', cancelCallSession);
    expose('__operitBuildRuntimeContext', buildRuntimeContext);

    expose('__operitGetActiveModuleExports', function() {
        if (
            root.__operitActiveModule &&
            typeof root.__operitActiveModule === 'object' &&
            root.__operitActiveModule.exports
        ) {
            return root.__operitActiveModule.exports;
        }
        var exportsRef = root.__operitActiveModuleExports;
        return exportsRef && typeof exportsRef === 'object' ? exportsRef : exportsRef || null;
    });
})();

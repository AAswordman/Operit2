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
        registry[resolvedCallId] = callState;
        return callState;
    }

    function cleanupCallSession(callId) {
        var resolvedCallId = normalizeCallId(callId);
        if (!resolvedCallId) {
            return;
        }
        delete ensureCallRegistry()[resolvedCallId];
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
    expose('__operitCleanupCallSession', cleanupCallSession);
    expose('__operitBuildRuntimeContext', buildRuntimeContext);

    windowRef.__operitGetActiveModuleExports = function() {
        if (
            windowRef.__operitActiveModule &&
            typeof windowRef.__operitActiveModule === 'object' &&
            windowRef.__operitActiveModule.exports
        ) {
            return windowRef.__operitActiveModule.exports;
        }
        var exportsRef = windowRef.__operitActiveModuleExports;
        return exportsRef && typeof exportsRef === 'object' ? exportsRef : exportsRef || null;
    };
})();

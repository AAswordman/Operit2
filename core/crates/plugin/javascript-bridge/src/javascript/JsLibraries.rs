use crate::javascript::JsAssetLoader::{
    loadAndroidUtilsJs, loadOkHttp3Js, loadPluginConfigJs, loadRuntimeContextJs, loadUINodeJs,
};
use crate::javascript::JsEmbeddedLibraryLoader::{loadCryptoJs, loadJimpJs, loadPakoJs};
use crate::javascript::JsInitRuntimeScriptBuilder;
use crate::javascript::JsJavaBridge::buildJavaClassBridgeDefinition;
use operit_plugin_sdk::toolpkg::ToolPkgApiRuntimeScript::buildToolPkgApiRuntimeScript;
use operit_plugin_sdk::toolpkg::ToolPkgComposeDslBridge::buildComposeDslContextBridgeDefinition;
use operit_plugin_sdk::toolpkg::ToolPkgRegistrationBridge::buildToolPkgRegistrationBridgeScript;
use operit_plugin_sdk::JsExecutionScriptBuilder;
use operit_plugin_sdk::JsTools::getJsToolsDefinition;
use operit_util::RuntimeStorageLayout::{RUNTIME_CLEAN_ON_EXIT_DIR_PATH, RUNTIME_ROOT_PATH_PREFIX};

/// JavaScript bootstrap module loaded into the QuickJS runtime.
pub struct JsBootstrapModule {
    pub fileName: String,
    pub source: String,
    pub globals: Vec<String>,
}

impl JsBootstrapModule {
    /// Creates a bootstrap module descriptor with its expected global exports.
    pub fn new(fileName: &str, source: String, globals: &[&str]) -> Self {
        Self {
            fileName: fileName.to_string(),
            source,
            globals: globals.iter().map(|global| global.to_string()).collect(),
        }
    }
}

#[allow(non_snake_case)]
/// Builds the ordered bootstrap module list required by the JavaScript runtime.
pub fn buildRuntimeBootstrapModules() -> Vec<JsBootstrapModule> {
    vec![
        JsBootstrapModule::new(
            "quickjs/init/runtime.js",
            JsInitRuntimeScriptBuilder::buildRuntimeBootstrapScript(),
            &[],
        ),
        JsBootstrapModule::new(
            "quickjs/init/operit-paths.js",
            buildOperitPathsBootstrapScript(),
            &["OPERIT_CLEAN_ON_EXIT_DIR"],
        ),
        JsBootstrapModule::new(
            "quickjs/init/execution-runtime.js",
            JsExecutionScriptBuilder::buildExecutionRuntimeBridgeScript(),
            &[],
        ),
        JsBootstrapModule::new(
            "quickjs/init/toolpkg-api-runtime.js",
            buildToolPkgApiRuntimeScript(),
            &["__operitToolPkgApi"],
        ),
        JsBootstrapModule::new(
            "quickjs/init/toolpkg-bridge.js",
            buildToolPkgRegistrationBridgeScript(false),
            &["ToolPkg"],
        ),
        JsBootstrapModule::new(
            "quickjs/init/tools.js",
            getJsToolsDefinition().to_string(),
            &["Tools", "tools"],
        ),
        JsBootstrapModule::new(
            "assets/js/PluginConfig.js",
            loadPluginConfigJs(),
            &["PluginConfig"],
        ),
        JsBootstrapModule::new(
            "assets/js/RuntimeContext.js",
            loadRuntimeContextJs(),
            &["RuntimeContext", "withContext"],
        ),
        JsBootstrapModule::new(
            "quickjs/init/compose-dsl-bridge.js",
            buildComposeDslContextBridgeDefinition(),
            &["OperitComposeDslRuntime"],
        ),
        JsBootstrapModule::new(
            "quickjs/init/java-bridge.js",
            buildJavaClassBridgeDefinition(),
            &["Java", "Kotlin"],
        ),
        JsBootstrapModule::new(
            "quickjs/init/third-party-libs.js",
            getJsThirdPartyLibraries(),
            &["_", "dataUtils", "Icons"],
        ),
        JsBootstrapModule::new("assets/js/CryptoJS.js", loadCryptoJs(), &["CryptoJS"]),
        JsBootstrapModule::new("assets/js/Jimp.js", loadJimpJs(), &["Jimp"]),
        JsBootstrapModule::new("assets/js/UINode.js", loadUINodeJs(), &["UINode"]),
        JsBootstrapModule::new(
            "assets/js/AndroidUtils.js",
            loadAndroidUtilsJs(),
            &[
                "Android",
                "PackageManager",
                "ContentProvider",
                "SystemManager",
                "DeviceController",
            ],
        ),
        JsBootstrapModule::new(
            "assets/js/OkHttp3.js",
            loadOkHttp3Js(),
            &[
                "OkHttpClientBuilder",
                "OkHttpClient",
                "RequestBuilder",
                "OkHttp",
            ],
        ),
        JsBootstrapModule::new("assets/js/pako.js", loadPakoJs(), &["pako"]),
    ]
}

#[allow(non_snake_case)]
fn buildOperitPathsBootstrapScript() -> String {
    let cleanOnExitDirJson = serde_json::to_string(&cleanOnExitVfsPath())
        .expect("clean-on-exit VFS path must serialize");
    format!(
        r#"
        var OPERIT_CLEAN_ON_EXIT_DIR = {};
        if (typeof __operitExpose === 'function') {{
            __operitExpose('OPERIT_CLEAN_ON_EXIT_DIR', OPERIT_CLEAN_ON_EXIT_DIR);
        }}
        "#,
        cleanOnExitDirJson
    )
}

/// Returns the public VFS path consumed by Tools.Files, not a host physical path.
/// The file-tool mapper resolves it against the active identity's runtime root.
#[allow(non_snake_case)]
fn cleanOnExitVfsPath() -> String {
    let relativePath = RUNTIME_CLEAN_ON_EXIT_DIR_PATH
        .strip_prefix(RUNTIME_ROOT_PATH_PREFIX)
        .expect("clean-on-exit path must be rooted under runtime");
    format!("/app/data/{relativePath}")
}

#[allow(non_snake_case)]
/// Returns the inline third-party utility libraries exposed to tool scripts.
pub fn getJsThirdPartyLibraries() -> String {
    r#"
        var _ = {
            isEmpty: function(value) {
                return value == null ||
                    (Array.isArray(value) && value.length === 0) ||
                    (typeof value === 'object' && !Array.isArray(value) && Object.keys(value).length === 0);
            },
            isString: function(value) { return typeof value === 'string'; },
            isNumber: function(value) { return typeof value === 'number' && !isNaN(value); },
            isBoolean: function(value) { return typeof value === 'boolean'; },
            isObject: function(value) { return value != null && typeof value === 'object' && !Array.isArray(value); },
            isArray: function(value) { return Array.isArray(value); },
            forEach: function(collection, iteratee) {
                if (Array.isArray(collection)) {
                    for (var index = 0; index < collection.length; index += 1) {
                        iteratee(collection[index], index, collection);
                    }
                    return collection;
                }
                if (collection && typeof collection === 'object') {
                    var keys = Object.keys(collection);
                    for (var keyIndex = 0; keyIndex < keys.length; keyIndex += 1) {
                        var key = keys[keyIndex];
                        iteratee(collection[key], key, collection);
                    }
                }
                return collection;
            },
            map: function(collection, iteratee) {
                var output = [];
                _.forEach(collection, function(item, key, source) {
                    output.push(iteratee(item, key, source));
                });
                return output;
            }
        };

        var dataUtils = {
            parseJson: function(text) {
                try { return JSON.parse(text); } catch (_error) { return null; }
            },
            stringifyJson: function(value) {
                try { return JSON.stringify(value); } catch (_error) { return '{}'; }
            },
            formatDate: function(value) {
                var date = value ? new Date(value) : new Date();
                function pad(part) { return String(part).padStart(2, '0'); }
                return [
                    date.getFullYear(),
                    pad(date.getMonth() + 1),
                    pad(date.getDate())
                ].join('-') + ' ' + [
                    pad(date.getHours()),
                    pad(date.getMinutes()),
                    pad(date.getSeconds())
                ].join(':');
            }
        };

        var Icons =
            typeof Proxy === 'function'
                ? new Proxy({}, {
                    get: function(_target, key) {
                        if (typeof key !== 'string') {
                            return '';
                        }
                        return key;
                    }
                })
                : {};
    "#
    .to_string()
}

#[allow(non_snake_case)]
/// Builds the host-backed JavaScript bootstrap and scoped runtime services.
pub fn buildRuntimeBootstrapScript() -> String {
    let executionPreludeJson =
        serde_json::to_string(&JsExecutionScriptBuilder::buildExecutionPreludeSource())
            .expect("Execution prelude source must serialize");
    let cleanOnExitDirJson = serde_json::to_string(&cleanOnExitVfsPath())
        .expect("clean-on-exit VFS path must serialize");
    format!(
        r#"
        {}
        var globalThis = this;
        var window = globalThis;
        var OPERIT_CLEAN_ON_EXIT_DIR = {};
        var __operitRuntimePrelude = {};
        {}
        var console = {{
            log: function() {{ __operitNativeLog('info', String(globalThis.__operitCurrentCallId || ''), Array.prototype.slice.call(arguments).join(' ')); }},
            info: function() {{ __operitNativeLog('info', String(globalThis.__operitCurrentCallId || ''), Array.prototype.slice.call(arguments).join(' ')); }},
            warn: function() {{ __operitNativeLog('warn', String(globalThis.__operitCurrentCallId || ''), Array.prototype.slice.call(arguments).join(' ')); }},
            error: function() {{ __operitNativeLog('error', String(globalThis.__operitCurrentCallId || ''), Array.prototype.slice.call(arguments).join(' ')); }}
        }};
        var intervalStates = {{}};
        var timerStates = new Map();
        var nextTimerId = 0;
        /** Schedules a host Promise while retaining the owning execution timer. */
        function setTimeout(handler, delayMs) {{
                if (typeof handler !== 'function') {{
                    throw new TypeError('setTimeout handler must be a function');
                }}
                var normalizedDelay = Number(delayMs);
                if (!Number.isFinite(normalizedDelay) || normalizedDelay < 0 || normalizedDelay > Number.MAX_SAFE_INTEGER) {{
                    throw new RangeError('setTimeout delay must be finite, non-negative and within the safe integer range');
                }}
                var timerId = '__operit_timer_' + (++nextTimerId);
                var timerArguments = Array.prototype.slice.call(arguments, 2);
                var timerCallId = String(globalThis.__operitCurrentCallId);
                var state = {{ callId: timerCallId }};
                timerStates.set(timerId, state);
                __operitRegisterCallTimer(timerCallId, timerId);
                /** Reports timer failures through the active execution session. */
                function failTimer(error) {{
                    __operitActivateCall(timerCallId);
                    var callState = __operitGetCallState(timerCallId);
                    if (callState && (!callState.completed || callState.detached)) {{
                        callState.callRuntime.fail(error);
                    }}
                }}
                /** Releases the timer after the handler's public Promise continuations. */
                function releaseTimer() {{
                    Promise.resolve().then(function() {{
                        __operitUnregisterCallTimer(timerCallId, timerId);
                    }});
                }}
                try {{
                    __operitNativeScheduleJavaScriptTimer(timerId, Math.floor(normalizedDelay)).then(function() {{
                        if (!timerStates.delete(timerId)) return;
                        try {{
                            __operitActivateCall(timerCallId);
                            handler.apply(window, timerArguments);
                        }} catch (error) {{
                            failTimer(error);
                        }} finally {{
                            releaseTimer();
                        }}
                    }}, function(error) {{
                        if (!timerStates.delete(timerId)) return;
                        try {{ failTimer(error); }} finally {{ releaseTimer(); }}
                    }});
                }} catch (error) {{
                    timerStates.delete(timerId);
                    __operitUnregisterCallTimer(timerCallId, timerId);
                    throw error;
                }}
                return timerId;
        }}

        /** Cancels the exact host timer scope and releases its owning session. */
        function clearTimeout(timerId) {{
                var normalizedTimerId = String(timerId);
                var state = timerStates.get(normalizedTimerId);
                if (!state) return;
                timerStates.delete(normalizedTimerId);
                if (state.intervalId) {{
                    intervalStates[state.intervalId].active = false;
                    delete intervalStates[state.intervalId];
                }}
                __operitNativeCancelJavaScriptPromises(normalizedTimerId);
                __operitUnregisterCallTimer(state.callId, normalizedTimerId);
        }}

        /** Schedules a repeating timer through the structured host timer binding. */
        function setInterval(handler, delayMs) {{
                if (typeof handler !== 'function') {{
                    throw new TypeError('setInterval handler must be a function');
                }}
                var intervalId = '__operit_interval_' + Date.now() + '_' + Math.random().toString(36).slice(2, 10);
                var timerArguments = Array.prototype.slice.call(arguments, 2);
                var state = {{ active: true, timerId: null }};
                intervalStates[intervalId] = state;
                /** Schedules the next active interval tick through the same timer scope. */
                var scheduleNext = function() {{
                    if (!state.active) return;
                    state.timerId = setTimeout(function() {{
                        if (!state.active) return;
                        try {{
                            handler.apply(window, timerArguments);
                        }} finally {{
                            scheduleNext();
                        }}
                    }}, delayMs);
                    timerStates.get(state.timerId).intervalId = intervalId;
                }};
                scheduleNext();
                return intervalId;
        }}

        /** Cancels a repeating timer and its pending host timeout. */
        function clearInterval(intervalId) {{
                var normalizedIntervalId = String(intervalId || '');
                var state = intervalStates[normalizedIntervalId];
                if (!state) return;
                state.active = false;
                if (state.timerId) clearTimeout(state.timerId);
                delete intervalStates[normalizedIntervalId];
        }}

        {}

        /** Returns the host-owned executable tool catalog for workflow editors. */
        function getToolCatalog() {{
            var catalog = __operitNativeGetToolCatalog();
            if (!catalog) {{
                throw new Error('Tool catalog request returned no response');
            }}
            if (catalog.success === false) {{
                throw new Error(String(catalog.message || 'Tool catalog request failed'));
            }}
            if (!Array.isArray(catalog.tools)) {{
                throw new Error('Tool catalog response must contain a tools array');
            }}
            return catalog;
        }}

        globalThis.__operitCompleteCalled = false;
        globalThis.__operitCompleteValue = undefined;
        /** Completes a structured result or reports the original conversion failure. */
        function complete(value) {{
            globalThis.__operitCompleteCalled = true;
            globalThis.__operitCompleteValue = value;
        }}

        function sendIntermediateResult(value) {{
            __operitSendIntermediateResult(String(globalThis.__operitCurrentCallId), value === undefined ? null : value);
        }}
        var emit = sendIntermediateResult;
        var delta = sendIntermediateResult;
        var log = sendIntermediateResult;
        var update = sendIntermediateResult;

        /** Normalizes an explicitly tagged Compose screen reference before host result conversion. */

        function __operitNormalizeComposeResult(value) {{
            if (!value || typeof value !== 'object' || !value.composeDsl || typeof value.composeDsl !== 'object') {{
                return value;
            }}
            if (!Object.prototype.hasOwnProperty.call(value.composeDsl, 'screen')) {{
                return value;
            }}
            var screenRef = value.composeDsl.screen;
            var resolved = '';
            if (typeof screenRef === 'function') {{
                resolved = __operitText(screenRef.__operit_toolpkg_module_path).trim();
            }} else if (
                screenRef &&
                typeof screenRef === 'object' &&
                typeof screenRef.default === 'function'
            ) {{
                resolved = __operitText(screenRef.default.__operit_toolpkg_module_path).trim();
            }} else if (typeof screenRef === 'string') {{
                throw new Error('composeDsl.screen must be a compose_dsl screen function, not a string path');
            }}
            if (!resolved) {{
                throw new Error('composeDsl.screen is missing a toolpkg module path marker');
            }}
            value.composeDsl.screen = resolved.replace(/\\/g, '/');
            return value;
        }}

        function __operitGetFactoryCache() {{
            if (!globalThis.__operitFactoryCache || typeof globalThis.__operitFactoryCache !== 'object') {{
                globalThis.__operitFactoryCache = {{}};
            }}
            return globalThis.__operitFactoryCache;
        }}

        function __operitGetModuleInstanceCache() {{
            if (!globalThis.__operitModuleInstanceCache || typeof globalThis.__operitModuleInstanceCache !== 'object') {{
                globalThis.__operitModuleInstanceCache = {{}};
            }}
            return globalThis.__operitModuleInstanceCache;
        }}

        function __operitNormalizePath(pathValue) {{
            var parts = String(pathValue == null ? '' : pathValue).replace(/\\/g, '/').split('/');
            var stack = [];
            for (var i = 0; i < parts.length; i += 1) {{
                var part = parts[i];
                if (!part || part === '.') {{
                    continue;
                }}
                if (part === '..') {{
                    if (stack.length > 0) {{
                        stack.pop();
                    }}
                    continue;
                }}
                stack.push(part);
            }}
            return stack.join('/');
        }}

        function __operitDirname(pathValue) {{
            var normalized = __operitNormalizePath(pathValue);
            var index = normalized.lastIndexOf('/');
            return index < 0 ? '' : normalized.slice(0, index);
        }}

        function __operitResolveModulePath(request, fromPath) {{
            var normalized = String(request == null ? '' : request).replace(/\\/g, '/').trim();
            if (!normalized) {{
                return '';
            }}
            if (!(normalized.startsWith('.') || normalized.startsWith('/'))) {{
                return normalized;
            }}
            if (normalized.startsWith('/')) {{
                return __operitNormalizePath(normalized);
            }}
            var base = __operitDirname(fromPath);
            return __operitNormalizePath(base ? base + '/' + normalized : normalized);
        }}

        function __operitBuildCandidatePaths(modulePath) {{
            var normalized = __operitNormalizePath(modulePath);
            if (!normalized) {{
                return [];
            }}
            if (/\.[a-z0-9]+$/i.test(normalized)) {{
                return [normalized];
            }}
            return [
                normalized,
                normalized + '.js',
                normalized + '.json',
                normalized + '/index.js',
                normalized + '/index.json'
            ];
        }}

        function __operitHashText(value) {{
            var textValue = String(value == null ? '' : value);
            var hash = 0;
            for (var i = 0; i < textValue.length; i += 1) {{
                hash = (((hash << 5) - hash) + textValue.charCodeAt(i)) | 0;
            }}
            return (hash >>> 0).toString(16);
        }}

        function __operitBuildFactoryKey(kind, identity, source) {{
            return [String(kind || ''), String(identity || ''), String(source || '').length, __operitHashText(source)].join(':');
        }}

        function __operitGetFactory(kind, identity, source, composeCompilation) {{
            var key = __operitBuildFactoryKey(kind, identity, source) + ":compose=" + composeCompilation;
            var cache = __operitGetFactoryCache();
            if (typeof cache[key] === 'function') {{
                return cache[key];
            }}
            var factory = new Function(
                'module',
                'exports',
                'require',
                '__operit_call_runtime',
                __operitRuntimePrelude + '\n' + (composeCompilation ? OperitComposeCompiler.compile(source, identity) : source)
            );
            cache[key] = factory;
            return factory;
        }}

        function __operitTagModuleExports(modulePath, exportsRef) {{
            if (typeof exportsRef === 'function') {{
                exportsRef.__operit_toolpkg_module_path = modulePath;
                return;
            }}
            if (!exportsRef || typeof exportsRef !== 'object') {{
                return;
            }}
            exportsRef.__operit_toolpkg_module_path = modulePath;
            Object.keys(exportsRef).forEach(function(key) {{
                if (typeof exportsRef[key] === 'function') {{
                    exportsRef[key].__operit_toolpkg_module_path = modulePath;
                    exportsRef[key].__operit_toolpkg_export_name = key;
                }}
            }});
        }}

        function __operitText(value) {{
            return value == null ? '' : String(value);
        }}

        function __operitToBoolean(value) {{
            if (typeof value === 'boolean') {{
                return value;
            }}
            var normalized = __operitText(value).trim().toLowerCase();
            return normalized === 'true' || normalized === '1' || normalized === 'yes' || normalized === 'on';
        }}

        function __operitCreateRegistrationScreenPlaceholder(modulePath) {{
            function ScreenPlaceholder() {{
                return null;
            }}
            ScreenPlaceholder.__operit_toolpkg_module_path = modulePath;
            return ScreenPlaceholder;
        }}

        function __operitIsLocalUiModulePath(modulePath) {{
            var normalized = __operitNormalizePath(modulePath);
            return /\.ui\.js$/i.test(normalized);
        }}

        function __operitFindTargetFunction(exportsRef, moduleRef, functionName) {{
            if (exportsRef && typeof exportsRef[functionName] === 'function') {{
                return exportsRef[functionName];
            }}
            if (moduleRef && moduleRef.exports && typeof moduleRef.exports[functionName] === 'function') {{
                return moduleRef.exports[functionName];
            }}
            if (typeof globalThis[functionName] === 'function') {{
                return globalThis[functionName];
            }}
            return null;
        }}

        function __operitBuildAvailableFunctions(exportsRef, moduleRef) {{
            var names = [];
            function collect(target) {{
                if (!target || typeof target !== 'object') {{
                    return;
                }}
                Object.keys(target).forEach(function(key) {{
                    if (typeof target[key] === 'function' && names.indexOf(key) < 0) {{
                        names.push(key);
                    }}
                }});
            }}
            collect(exportsRef);
            collect(moduleRef && moduleRef.exports ? moduleRef.exports : null);
            return names;
        }}

        function __operitExecuteScriptFunction(callId, params, scriptText, targetFunctionName, timeoutSec, preTimeoutMs, structuredResult) {{
            var previousCallRuntime = globalThis.__operit_call_runtime_ref;
            var previousCallId = globalThis.__operitCurrentCallId;
            var registerCallSession = globalThis.__operitRegisterCallSession;
            if (typeof registerCallSession !== 'function') {{
                __operitNativeSetCallError(callId, {{
                    success: false,
                    message: 'JS execution runtime bridge is unavailable'
                }});
                return;
            }}
            var callState = registerCallSession(callId, params);
            callState.previousCallId = previousCallId;
            callState.previousCallRuntime = previousCallRuntime;
            globalThis.__operitCurrentCallId = callId;
            function getCallState() {{
                return typeof globalThis.__operitGetCallState === 'function'
                    ? globalThis.__operitGetCallState(callId)
                    : null;
            }}
            function finalizeCall() {{
                if (globalThis.__operitCurrentCallId === callId) {{
                    globalThis.__operitCurrentCallId =
                        typeof previousCallId === 'string' ? previousCallId : '';
                }}
                if (globalThis.__operit_call_runtime_ref === callRuntime) {{
                    if (previousCallRuntime && typeof previousCallRuntime === 'object') {{
                        globalThis.__operit_call_runtime_ref = previousCallRuntime;
                    }} else {{
                        delete globalThis.__operit_call_runtime_ref;
                    }}
                }}
                if (callState.pendingReferences > 0) {{
                    callState.detached = true;
                    if (typeof globalThis.__operitNotifyDetachedCall === 'function') {{
                        globalThis.__operitNotifyDetachedCall(callId);
                    }}
                }} else if (typeof globalThis.__operitCleanupCallSession === 'function') {{
                    globalThis.__operitCleanupCallSession(callId);
                }}
            }}
            function isActive() {{
                var state = getCallState();
                return !!(state && (!state.completed || state.detached));
            }}
            function readCallValue(key, defaultValue) {{
                var state = getCallState();
                var currentParams = state && state.params && typeof state.params === 'object'
                    ? state.params
                    : null;
                var value = currentParams ? currentParams[key] : undefined;
                return value == null || value === '' ? defaultValue : __operitText(value);
            }}
            function markStage(stage) {{
                callState.lastExecStage = __operitText(stage);
                __operitNativeLogJsExecutionTrace(
                    callId,
                    'stage=' + callState.lastExecStage +
                        ' function=' + __operitText(callState.lastExecFunction) +
                        ' module=' + __operitText(callState.lastModulePath) +
                        ' require=' + __operitText(callState.lastRequireRequest) +
                        ' from=' + __operitText(callState.lastRequireFrom) +
                        ' resolved=' + __operitText(callState.lastRequireResolved)
                );
            }}
            function markFunction(name) {{
                callState.lastExecFunction = __operitText(name);
                __operitNativeLogJsExecutionTrace(
                    callId,
                    'function=' + callState.lastExecFunction +
                        ' package=' + __operitText(params && (params.__operit_ui_package_name || params.toolPkgId || params.__operit_package_name)) +
                        ' screen=' + __operitText(params && params.__operit_script_screen) +
                        ' context=' + __operitText(params && params.__operit_execution_context_key)
                );
            }}
            function markRequire(request, fromPath, resolvedPath) {{
                callState.lastRequireRequest = __operitText(request);
                callState.lastRequireFrom = __operitText(fromPath);
                callState.lastRequireResolved = __operitText(resolvedPath);
                __operitNativeLogJsExecutionTrace(
                    callId,
                    'require=' + callState.lastRequireRequest +
                        ' from=' + callState.lastRequireFrom +
                        ' resolved=' + callState.lastRequireResolved
                );
            }}
            function markModule(modulePath) {{
                callState.lastModulePath = __operitText(modulePath);
                __operitNativeLogJsExecutionTrace(callId, 'module=' + callState.lastModulePath);
            }}
            /** Stores a structured result before marking the owning execution completed. */
            function completeCall(resultValue) {{
                var state = getCallState();
                if (!state || state.resultCompleted) {{
                    return;
                }}
                if (structuredResult) {{
                    __operitNativeSetCallStructuredResult(callId, resultValue);
                }} else {{
                    __operitNativeSetCallResult(callId, resultValue);
                }}
                state.resultCompleted = true;
                state.completed = Number(state.pendingReferences || 0) <= 0;
                try {{
                    __operitNativeLogJsExecutionTrace(callId, 'complete ' + __operitText(resultValue).slice(0, 240));
                }} finally {{
                    finalizeCall();
                }}
            }}
            /** Reports an explicit structured failure and finalizes the owning execution. */
            function emitError(message) {{
                var state = getCallState();
                if (!state || state.resultCompleted) {{
                    return;
                }}
                state.resultCompleted = true;
                state.completed = Number(state.pendingReferences || 0) <= 0;
                try {{
                    __operitNativeLogJsExecutionTrace(callId, 'error ' + __operitText(message).slice(0, 240));
                    __operitNativeSetCallError(callId, {{
                        success: false,
                        message: __operitText(message)
                    }});
                }} finally {{
                    finalizeCall();
                }}
            }}
            function callRuntimeReport(error, context) {{
                if (typeof globalThis.__operitReportDetailedErrorForCall === 'function') {{
                    return globalThis.__operitReportDetailedErrorForCall(callId, error, context);
                }}
                return {{
                    formatted: __operitText(context) + ': ' + __operitText(error),
                    details: {{
                        message: __operitText(error && error.message ? error.message : error),
                        stack: __operitText(error && error.stack ? error.stack : error),
                        lineNumber: 0
                    }}
                }};
            }}
            /** Forwards intermediate application values directly to the structured host listener binding. */
            function emitIntermediate(value) {{
                if (isActive()) {{
                    if (structuredResult) {{
                        __operitNativeSendStructuredIntermediate(callId, __operitNormalizeSerializableValue(__operitNormalizeComposeResult(value), []));
                    }} else {{
                        __operitSendIntermediateResult(callId, value === undefined ? null : value);
                    }}
                }}
            }}
            /** Completes a structured result or reports the original conversion failure. */
            function complete(value) {{
                try {{
                    completeCall(value === undefined ? null : __operitNormalizeSerializableValue(__operitNormalizeComposeResult(value), []));
                }} catch (error) {{
                    var report = callRuntimeReport(error, 'Result Serialization Failure');
                    var serializationMessage =
                        report &&
                        report.details &&
                        typeof report.details.message === 'string' &&
                        report.details.message
                            ? report.details.message
                            : __operitText(error && error.message ? error.message : error);
                    emitError('Result serialization failed: ' + serializationMessage);
                }}
            }}
            var callRuntime = {{
                callId: callId,
                emit: emitIntermediate,
                delta: emitIntermediate,
                log: emitIntermediate,
                update: emitIntermediate,
                sendIntermediateResult: emitIntermediate,
                done: complete,
                complete: complete,
                getState: function() {{ return readCallValue('__operit_package_state', undefined); }},
                getLang: function() {{ return readCallValue('__operit_package_lang', 'en'); }},
                getCallerName: function() {{ return readCallValue('__operit_package_caller_name', undefined); }},
                getChatId: function() {{ return readCallValue('__operit_package_chat_id', undefined); }},
                /** Returns the opaque authenticated participant without a card alias or another namespace. */
                getCallerParticipantId: function() {{ return readCallValue('__operit_package_caller_participant_id', undefined); }},
                getEnv: function(key) {{
                    var value = __operitNativeGetEnvForCall(callId, __operitText(key).trim());
                    return value == null || value === '' ? undefined : __operitText(value);
                }},
                getPluginConfigDir: function(pluginId) {{
                    var explicitId = pluginId == null ? '' : __operitText(pluginId).trim();
                    var ownerId =
                        readCallValue('__operit_ui_package_name', '') ||
                        readCallValue('toolPkgId', '') ||
                        readCallValue('containerPackageName', '') ||
                        readCallValue('__operit_package_name', '');
                    var resolvedId = explicitId || ownerId;
                    if (!ownerId) {{
                        throw new Error('Plugin configuration owner is required');
                    }}
                    return __operitNativeGetScopedPluginConfigDir(ownerId, resolvedId);
                }},
                reportDetailedError: callRuntimeReport,
                fail: function(error) {{
                    var report = callRuntimeReport(error, 'Asynchronous Callback');
                    var message = report && report.details && report.details.message
                        ? report.details.message
                        : __operitText(error && error.message ? error.message : error);
                    emitError(message || 'Asynchronous callback failed');
                }},
                handleAsync: function(value) {{
                    if (!value || typeof value.then !== 'function') {{
                        return false;
                    }}
                    Promise.resolve(value).then(
                        function(result) {{
                            if (isActive()) {{
                                complete(result);
                            }}
                        }},
                        function(error) {{
                            if (isActive()) {{
                                var report = callRuntimeReport(error, 'Async Promise Rejection');
                                var rejectionMessage =
                                    report &&
                                    report.details &&
                                    typeof report.details.message === 'string' &&
                                    report.details.message
                                        ? report.details.message
                                        : __operitText(error && error.message ? error.message : error);
                                emitError(rejectionMessage || 'Promise rejection');
                            }}
                        }}
                    );
                    return true;
                }},
                console: console
            }};
            callState.callRuntime = callRuntime;
            globalThis.__operit_call_runtime_ref = callRuntime;
            try {{
                var registrationMode = __operitToBoolean(readCallValue('__operit_registration_mode', false));
                var packageTarget =
                    readCallValue('__operit_ui_package_name', '') ||
                    readCallValue('toolPkgId', '');
                var screenPath = __operitNormalizePath(readCallValue(
                    '__operit_script_screen',
                    params && params.moduleSpec && params.moduleSpec.screen
                        ? __operitText(params.moduleSpec.screen)
                        : ''
                ));
                function getCurrentToolPkgRuntimeKind() {{
                    var explicitRuntime = __operitText(
                        readCallValue('__operit_toolpkg_runtime_kind', '')
                    ).trim().toLowerCase();
                    if (
                        explicitRuntime === 'main' ||
                        explicitRuntime === 'ui' ||
                        explicitRuntime === 'sandbox' ||
                        explicitRuntime === 'provider'
                    ) {{
                        return explicitRuntime;
                    }}
                    var contextKey = __operitText(
                        readCallValue(
                            '__operit_execution_context_key',
                            readCallValue('executionContextKey', '')
                        )
                    ).trim();
                    if (/^toolpkg_provider:/i.test(contextKey)) {{
                        return 'provider';
                    }}
                    if (
                        /^toolpkg_compose:/i.test(contextKey) ||
                        /^toolpkg_compose_dsl:/i.test(contextKey) ||
                        /^toolpkg_xml_render:/i.test(contextKey)
                    ) {{
                        return 'ui';
                    }}
                    var subpackageId = __operitText(
                        readCallValue('__operit_toolpkg_subpackage_id', '')
                    ).trim();
                    return subpackageId.length > 0 ? 'sandbox' : 'main';
                }}

                function getCurrentToolPkgExecutionContextKey() {{
                    var composeContextKey = __operitText(
                        readCallValue(
                            '__operit_compose_execution_context_key',
                            readCallValue('executionContextKey', '')
                        )
                    ).trim();
                    var scopedContextKey = __operitText(
                        readCallValue('__operit_execution_context_key', '')
                    ).trim();
                    if (composeContextKey.length > 0) {{
                        return composeContextKey;
                    }}
                    if (scopedContextKey.length > 0) {{
                        return scopedContextKey;
                    }}
                    return packageTarget ? 'toolpkg_main:' + packageTarget : '';
                }}

                function ensureToolPkgIpcRegistry() {{
                    var registry = globalThis.__operitToolPkgIpcRegistry;
                    if (!registry || typeof registry !== 'object') {{
                        registry = Object.create(null);
                        globalThis.__operitToolPkgIpcRegistry = registry;
                    }}
                    return registry;
                }}

                function normalizeToolPkgIpcChannel(channel) {{
                    return __operitText(channel).trim();
                }}

                function registerToolPkgIpcHandler(channel, handler) {{
                    var normalizedChannel = normalizeToolPkgIpcChannel(channel);
                    if (normalizedChannel.length === 0) {{
                        throw new Error('ToolPkg.ipc channel is required');
                    }}
                    if (typeof handler !== 'function') {{
                        throw new Error('ToolPkg.ipc handler must be a function');
                    }}
                    ensureToolPkgIpcRegistry()[normalizedChannel] = handler;
                    return function() {{
                        var registry = ensureToolPkgIpcRegistry();
                        if (registry[normalizedChannel] === handler) {{
                            delete registry[normalizedChannel];
                        }}
                    }};
                }}

                function unregisterToolPkgIpcHandler(channel, handler) {{
                    var normalizedChannel = normalizeToolPkgIpcChannel(channel);
                    if (normalizedChannel.length === 0) {{
                        return false;
                    }}
                    var registry = ensureToolPkgIpcRegistry();
                    if (arguments.length > 1 && registry[normalizedChannel] !== handler) {{
                        return false;
                    }}
                    if (typeof registry[normalizedChannel] === 'function') {{
                        delete registry[normalizedChannel];
                        return true;
                    }}
                    return false;
                }}

                function invokeToolPkgIpcLocal(channel, payload, meta) {{
                    var normalizedChannel = normalizeToolPkgIpcChannel(channel);
                    if (normalizedChannel.length === 0) {{
                        throw new Error('ToolPkg.ipc channel is required');
                    }}
                    var registry = ensureToolPkgIpcRegistry();
                    var handler = registry[normalizedChannel];
                    if (typeof handler !== 'function') {{
                        throw new Error('ToolPkg.ipc channel is not registered: ' + normalizedChannel);
                    }}
                    return handler(payload, meta && typeof meta === 'object' ? meta : {{}});
                }}

                function normalizeToolPkgWasmValueType(valueType) {{
                    var normalizedType = __operitText(valueType).trim().toLowerCase();
                    if (
                        normalizedType !== 'i32' &&
                        normalizedType !== 'i64' &&
                        normalizedType !== 'f32' &&
                        normalizedType !== 'f64'
                    ) {{
                        throw new Error('ToolPkg.wasm arg type is invalid: ' + normalizedType);
                    }}
                    return normalizedType;
                }}

                function normalizeToolPkgWasmArgs(args) {{
                    if (args == null) {{
                        return [];
                    }}
                    if (!Array.isArray(args)) {{
                        throw new Error('ToolPkg.wasm args must be an array');
                    }}
                    var normalizedArgs = [];
                    for (var i = 0; i < args.length; i += 1) {{
                        var arg = args[i];
                        if (!arg || typeof arg !== 'object' || Array.isArray(arg)) {{
                            throw new Error('ToolPkg.wasm arg ' + i + ' must be an object');
                        }}
                        var valueType = normalizeToolPkgWasmValueType(arg.type);
                        if (!Object.prototype.hasOwnProperty.call(arg, 'value')) {{
                            throw new Error('ToolPkg.wasm arg ' + i + ' value is required');
                        }}
                        var value = arg.value;
                        if (valueType === 'i32' && typeof value !== 'number') {{
                            throw new Error('ToolPkg.wasm arg ' + i + ' i32 value must be a number');
                        }}
                        if (valueType === 'i64' && typeof value !== 'number' && typeof value !== 'string') {{
                            throw new Error('ToolPkg.wasm arg ' + i + ' i64 value must be a number or string');
                        }}
                        if (
                            (valueType === 'f32' || valueType === 'f64') &&
                            typeof value !== 'number' &&
                            typeof value !== 'string'
                        ) {{
                            throw new Error('ToolPkg.wasm arg ' + i + ' float value must be a number or string');
                        }}
                        normalizedArgs.push({{ type: valueType, value: value }});
                    }}
                    return normalizedArgs;
                }}

                function ensureToolPkgIpcApi() {{
                    var toolPkgApi = globalThis.ToolPkg && typeof globalThis.ToolPkg === 'object'
                        ? globalThis.ToolPkg
                        : {{}};
                    if (globalThis.ToolPkg !== toolPkgApi) {{
                        globalThis.ToolPkg = toolPkgApi;
                    }}
                    var ipcApi = toolPkgApi.ipc && typeof toolPkgApi.ipc === 'object'
                        ? toolPkgApi.ipc
                        : {{}};

                    ipcApi.on = function(channel, handler) {{
                        return registerToolPkgIpcHandler(channel, handler);
                    }};
                    ipcApi.off = function(channel, handler) {{
                        return unregisterToolPkgIpcHandler(channel, handler);
                    }};
                    ipcApi.call = function(channel, payload, options) {{
                        var normalizedChannel = normalizeToolPkgIpcChannel(channel);
                        if (normalizedChannel.length === 0) {{
                            return Promise.reject(new Error('ToolPkg.ipc channel is required'));
                        }}
                        var callOptions = options && typeof options === 'object' ? options : {{}};
                        var targetRuntime = __operitText(callOptions.targetRuntime || '').trim().toLowerCase();
                        if (
                            targetRuntime &&
                            targetRuntime !== 'main' &&
                            targetRuntime !== 'ui' &&
                            targetRuntime !== 'sandbox' &&
                            targetRuntime !== 'provider'
                        ) {{
                            return Promise.reject(new Error('ToolPkg.ipc targetRuntime is invalid: ' + targetRuntime));
                        }}
                        var targetContextKey = __operitText(callOptions.targetContextKey || '').trim();
                        var hasTargetOptions = targetRuntime.length > 0 || targetContextKey.length > 0;
                        var currentContextKey = getCurrentToolPkgExecutionContextKey();
                        var currentRuntime = getCurrentToolPkgRuntimeKind();
                        if (
                            currentRuntime === 'main' &&
                            (
                                !hasTargetOptions ||
                                (
                                    (targetRuntime.length === 0 || targetRuntime === 'main') &&
                                    (targetContextKey.length === 0 || targetContextKey === currentContextKey)
                                )
                            )
                        ) {{
                            try {{
                                return Promise.resolve(
                                    invokeToolPkgIpcLocal(normalizedChannel, payload, {{
                                        channel: normalizedChannel,
                                        callerContextKey: currentContextKey,
                                        currentContextKey: currentContextKey,
                                        currentRuntime: currentRuntime,
                                        packageTarget: packageTarget
                                    }})
                                );
                            }} catch (error) {{
                                return Promise.reject(error);
                            }}
                        }}
                        if (
                            targetContextKey.length > 0 &&
                            targetContextKey === currentContextKey &&
                            targetRuntime.length > 0 &&
                            targetRuntime !== currentRuntime
                        ) {{
                            return Promise.reject(
                                new Error(
                                    'ToolPkg.ipc targetRuntime does not match current runtime: ' +
                                        targetRuntime +
                                        ' != ' +
                                        currentRuntime
                                )
                            );
                        }}
                        if (targetContextKey.length > 0 && targetContextKey === currentContextKey) {{
                            try {{
                                return Promise.resolve(
                                    invokeToolPkgIpcLocal(normalizedChannel, payload, {{
                                        channel: normalizedChannel,
                                        callerContextKey: currentContextKey,
                                        currentContextKey: currentContextKey,
                                        currentRuntime: currentRuntime,
                                        packageTarget: packageTarget
                                    }})
                                );
                            }} catch (error) {{
                                return Promise.reject(error);
                            }}
                        }}
                        if (!packageTarget) {{
                            return Promise.reject(new Error('ToolPkg.ipc requires a bound package target'));
                        }}
                        var ownerCallId = String(globalThis.__operitCurrentCallId);
                        return new Promise(function(resolve, reject) {{
                            __operitRetainCallReference(ownerCallId);
                            /** Releases this request only after the public continuation runs. */
                            function releaseReference() {{
                                Promise.resolve().then(function() {{
                                    __operitReleaseCallReference(ownerCallId);
                                }});
                            }}
                            try {{
                                __operitNativeInvokeToolPkgIpc(
                                    ownerCallId, packageTarget, currentContextKey,
                                    targetContextKey, targetRuntime, normalizedChannel,
                                    payload === undefined ? null : payload
                                ).then(function(response) {{
                                    try {{
                                        __operitActivateCall(ownerCallId);
                                        if (!response || typeof response !== 'object' || typeof response.success !== 'boolean') {{
                                            throw new TypeError('IPC host returned an invalid result envelope');
                                        }}
                                        if (!response.success) throw new Error(response.message);
                                        resolve(response.value);
                                    }} catch (error) {{
                                        reject(error);
                                    }} finally {{
                                        releaseReference();
                                    }}
                                }}, function(error) {{
                                    try {{
                                        __operitActivateCall(ownerCallId);
                                        reject(error);
                                    }} catch (activationError) {{
                                        reject(activationError);
                                    }} finally {{
                                        releaseReference();
                                    }}
                                }});
                            }} catch (error) {{
                                reject(error);
                                releaseReference();
                            }}
                        }});
                    }};

                    toolPkgApi.ipc = ipcApi;
                    globalThis.__operitInvokeToolPkgIpcLocal = invokeToolPkgIpcLocal;
                }}

                function ensureToolPkgWasmApi() {{
                    var toolPkgApi = globalThis.ToolPkg && typeof globalThis.ToolPkg === 'object'
                        ? globalThis.ToolPkg
                        : {{}};
                    if (globalThis.ToolPkg !== toolPkgApi) {{
                        globalThis.ToolPkg = toolPkgApi;
                    }}
                    var wasmApi = toolPkgApi.wasm && typeof toolPkgApi.wasm === 'object'
                        ? toolPkgApi.wasm
                        : {{}};
                    wasmApi.call = function(moduleId, exportName, args) {{
                        var normalizedModuleId = __operitText(moduleId).trim();
                        if (normalizedModuleId.length === 0) {{
                            return Promise.reject(new Error('ToolPkg.wasm module id is required'));
                        }}
                        var normalizedExportName = __operitText(exportName).trim();
                        if (normalizedExportName.length === 0) {{
                            return Promise.reject(new Error('ToolPkg.wasm export name is required'));
                        }}
                        var normalizedArgs;
                        try {{
                            normalizedArgs = normalizeToolPkgWasmArgs(args);
                        }} catch (error) {{
                            return Promise.reject(error);
                        }}
                        if (!packageTarget) {{
                            return Promise.reject(new Error('ToolPkg.wasm package target is required'));
                        }}
                        return __operitInvokeHostAsync(__operitNativeCallToolPkgWasm,
                            [packageTarget, normalizedModuleId, normalizedExportName, normalizedArgs]);
                    }};
                    toolPkgApi.wasm = wasmApi;
                }}

                ensureToolPkgIpcApi();
                if (!registrationMode) {{
                    ensureToolPkgWasmApi();
                }}
                if (
                    globalThis.RuntimeContext &&
                    typeof globalThis.RuntimeContext.__operitEnsureContextRunnerRegistered === 'function'
                ) {{
                    globalThis.RuntimeContext.__operitEnsureContextRunnerRegistered();
                }}
                var moduleCache = registrationMode ? {{}} : __operitGetModuleInstanceCache();
                var mainModuleKey = ['instance', 'main', packageTarget + ':' + screenPath, String(scriptText || '').length, __operitHashText(scriptText)].join(':');
                var module = moduleCache[mainModuleKey];
                var exports = module && module.exports ? module.exports : null;
                /** Reads CommonJS candidates from the execution resource owner and preserves empty modules. */
                function readToolPkgModule(modulePath) {{
                    if (!packageTarget) {{
                        return null;
                    }}
                    var candidates = __operitBuildCandidatePaths(modulePath);
                    for (var i = 0; i < candidates.length; i += 1) {{
                        var candidate = candidates[i];
                        var textResult = __operitNativeReadToolPkgTextResource(packageTarget, candidate);
                        if (typeof textResult === 'string') {{
                            return {{ path: candidate, text: textResult }};
                        }}
                    }}
                    return null;
                }}
                function executeModule(modulePath, moduleText, requireInternal) {{
                    var moduleKey = ['instance', 'module', packageTarget + ':' + modulePath, String(moduleText || '').length, __operitHashText(moduleText)].join(':');
                    if (moduleCache[moduleKey]) {{
                        return moduleCache[moduleKey].exports;
                    }}
                    var requiredModule = {{ exports: {{}} }};
                    moduleCache[moduleKey] = requiredModule;
                    if (/\.json$/i.test(modulePath)) {{
                        try {{
                            requiredModule.exports = JSON.parse(moduleText);
                            return requiredModule.exports;
                        }} catch (error) {{
                            delete moduleCache[moduleKey];
                            throw error;
                        }}
                    }}
                    var localRequire = function(nextName) {{
                        return requireInternal(nextName, modulePath);
                    }};
                    var factory = __operitGetFactory('module', packageTarget + ':' + modulePath, moduleText, structuredResult);
                    var previousActiveModule = globalThis.__operitActiveModule;
                    var previousActiveExports = globalThis.__operitActiveModuleExports;
                    var previousModule = callState.currentModule;
                    var previousExports = callState.currentModuleExports;
                    globalThis.__operitActiveModule = requiredModule;
                    globalThis.__operitActiveModuleExports = requiredModule.exports;
                    callState.currentModule = requiredModule;
                    callState.currentModuleExports = requiredModule.exports;
                    try {{
                        factory(requiredModule, requiredModule.exports, localRequire, callRuntime);
                    }} catch (error) {{
                        delete moduleCache[moduleKey];
                        throw error;
                    }} finally {{
                        callState.currentModule = previousModule;
                        callState.currentModuleExports = previousExports;
                        globalThis.__operitActiveModule = previousActiveModule;
                        globalThis.__operitActiveModuleExports = previousActiveExports;
                    }}
                    __operitTagModuleExports(modulePath, requiredModule.exports);
                    return requiredModule.exports;
                }}
                function requireInternal(moduleName, fromPath) {{
                    var request = String(moduleName == null ? '' : moduleName).trim();
                    if (request === 'lodash') {{
                        return globalThis._;
                    }}
                    if (request === 'uuid') {{
                        return {{
                            v4: function() {{
                                return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function(char) {{
                                    var random = Math.random() * 16 | 0;
                                    var value = char === 'x' ? random : ((random & 0x3) | 0x8);
                                    return value.toString(16);
                                }});
                            }}
                        }};
                    }}
                    if (request === 'axios') {{
                        return {{
                            get: function(url, config) {{
                                return toolCall('http_request', config ? Object.assign({{ url: url }}, config) : {{ url: url }});
                            }},
                            post: function(url, data, config) {{
                                return toolCall('http_request', config ? Object.assign({{ url: url, data: data }}, config) : {{ url: url, data: data }});
                            }}
                        }};
                    }}
                    if (!(request.startsWith('.') || request.startsWith('/'))) {{
                        return {{}};
                    }}
                    var resolvedPath = __operitResolveModulePath(request, fromPath || screenPath);
                    markStage('require_module');
                    markRequire(request, fromPath || screenPath || '<root>', resolvedPath);
                    markModule(resolvedPath);
                    if (registrationMode && __operitIsLocalUiModulePath(resolvedPath)) {{
                        return __operitCreateRegistrationScreenPlaceholder(resolvedPath);
                    }}
                    var loaded = readToolPkgModule(resolvedPath);
                    if (!loaded) {{
                        throw new Error('Cannot resolve module "' + request + '" from "' + (fromPath || screenPath || '<root>') + '"');
                    }}
                    return executeModule(loaded.path, loaded.text, requireInternal);
                }}
                var require = function(moduleName) {{
                    markStage('require_request');
                    markRequire(moduleName, screenPath || '<root>', '');
                    return requireInternal(moduleName, screenPath);
                }};
                markFunction(targetFunctionName);
                if (!module) {{
                    module = {{ exports: {{}} }};
                    moduleCache[mainModuleKey] = module;
                    exports = module.exports;
                    markStage('compile_main_script');
                    var mainFactory = __operitGetFactory('main', packageTarget + ':' + screenPath, scriptText, structuredResult);
                    markStage('execute_main_script');
                    var previousActiveModule = globalThis.__operitActiveModule;
                    var previousActiveExports = globalThis.__operitActiveModuleExports;
                    var previousModule = callState.currentModule;
                    var previousExports = callState.currentModuleExports;
                    globalThis.__operitActiveModule = module;
                    globalThis.__operitActiveModuleExports = exports;
                    callState.currentModule = module;
                    callState.currentModuleExports = exports;
                    try {{
                        mainFactory(module, exports, require, callRuntime);
                    }} catch (error) {{
                        delete moduleCache[mainModuleKey];
                        throw error;
                    }} finally {{
                        callState.currentModule = previousModule;
                        callState.currentModuleExports = previousExports;
                        globalThis.__operitActiveModule = previousActiveModule;
                        globalThis.__operitActiveModuleExports = previousActiveExports;
                    }}
                }} else {{
                    if (exports == null) {{
                        exports = {{}};
                        module.exports = exports;
                    }}
                    markStage('reuse_main_script');
                }}
                var rootExports = module.exports || exports || {{}};
                __operitTagModuleExports(screenPath || '<root>', rootExports);

                var inlineFunctionName = readCallValue('__operit_inline_function_name', '');
                var inlineFunctionSource = readCallValue('__operit_inline_function_source', '');
                if (inlineFunctionName && inlineFunctionSource) {{
                    markStage('evaluate_inline_hook_function');
                    var inlineFunction = eval('(' + inlineFunctionSource + ')');
                    if (typeof inlineFunction !== 'function') {{
                        throw new Error('inline hook source did not evaluate to function');
                    }}
                    rootExports[inlineFunctionName] = inlineFunction;
                    module.exports[inlineFunctionName] = inlineFunction;
                }}

                var targetFunction = __operitFindTargetFunction(rootExports, module, targetFunctionName);
                if (typeof targetFunction !== 'function') {{
                    emitError(
                        "Function '" +
                            targetFunctionName +
                            "' not found in script. Available functions: " +
                            __operitBuildAvailableFunctions(rootExports, module).join(', ')
                    );
                    return;
                }}
                markStage('invoke_target_function');
                var invokePreviousActiveModule = globalThis.__operitActiveModule;
                var invokePreviousActiveExports = globalThis.__operitActiveModuleExports;
                var previousModule = callState.currentModule;
                var previousExports = callState.currentModuleExports;
                globalThis.__operitActiveModule = module;
                globalThis.__operitActiveModuleExports = rootExports;
                callState.currentModule = module;
                callState.currentModuleExports = rootExports;
                var functionResult;
                try {{
                    functionResult = targetFunction(params);
                }} finally {{
                    callState.currentModule = previousModule;
                    callState.currentModuleExports = previousExports;
                    globalThis.__operitActiveModule = invokePreviousActiveModule;
                    globalThis.__operitActiveModuleExports = invokePreviousActiveExports;
                }}
                markStage('handle_function_result');
                if (!callRuntime.handleAsync(functionResult)) {{
                    complete(functionResult);
                }}
            }} catch (error) {{
                var runtimeContext = typeof globalThis.__operitBuildRuntimeContext === 'function'
                    ? __operitText(globalThis.__operitBuildRuntimeContext(callId))
                    : '';
                emitError(
                    'Script error: ' +
                        __operitText(error && error.message ? error.message : error) +
                        (runtimeContext ? '\nRuntime Context: ' + runtimeContext : '') +
                        (error && error.stack ? '\nStack: ' + __operitText(error.stack) : '')
                );
            }}
        }}

        {}
        {}
        {}
        {}
        {}
        {}
        {}
        {}
        {}
        {}
        {}
        {}
        {}
        "#,
        JsInitRuntimeScriptBuilder::buildRuntimeBootstrapScript(),
        cleanOnExitDirJson,
        executionPreludeJson,
        buildJavaClassBridgeDefinition(),
        buildComposeDslContextBridgeDefinition(),
        buildToolPkgApiRuntimeScript(),
        buildToolPkgRegistrationBridgeScript(false),
        getJsToolsDefinition(),
        getJsThirdPartyLibraries(),
        loadPluginConfigJs(),
        loadRuntimeContextJs(),
        loadCryptoJs(),
        loadJimpJs(),
        loadUINodeJs(),
        loadAndroidUtilsJs(),
        loadOkHttp3Js(),
        loadPakoJs(),
        JsExecutionScriptBuilder::buildExecutionRuntimeBridgeScript()
    )
}

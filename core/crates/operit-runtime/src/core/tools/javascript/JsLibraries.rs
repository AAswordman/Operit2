use crate::core::tools::javascript::JsAssetLoader::{
    loadAndroidUtilsJs, loadOkHttp3Js, loadPluginConfigJs, loadRuntimeContextJs, loadUINodeJs,
};
use crate::core::tools::javascript::JsComposeDslBridge::buildComposeDslContextBridgeDefinition;
use crate::core::tools::javascript::JsEmbeddedLibraryLoader::{
    loadCryptoJs, loadJimpJs, loadPakoJs,
};
use crate::core::tools::javascript::JsExecutionScriptBuilder;
use crate::core::tools::javascript::JsInitRuntimeScriptBuilder;
use crate::core::tools::javascript::JsJavaBridge::buildJavaClassBridgeDefinition;
use crate::core::tools::javascript::JsToolPkgRegistration::buildToolPkgRegistrationBridgeScript;
use crate::core::tools::javascript::JsTools::getJsToolsDefinition;

pub struct JsBootstrapModule {
    pub fileName: String,
    pub source: String,
    pub globals: Vec<String>,
}

impl JsBootstrapModule {
    pub fn new(fileName: &str, source: String, globals: &[&str]) -> Self {
        Self {
            fileName: fileName.to_string(),
            source,
            globals: globals.iter().map(|global| global.to_string()).collect(),
        }
    }
}

#[allow(non_snake_case)]
pub fn buildRuntimeBootstrapModules() -> Vec<JsBootstrapModule> {
    vec![
        JsBootstrapModule::new(
            "quickjs/init/runtime.js",
            JsInitRuntimeScriptBuilder::buildRuntimeBootstrapScript(),
            &[],
        ),
        JsBootstrapModule::new(
            "quickjs/init/execution-runtime.js",
            JsExecutionScriptBuilder::buildExecutionRuntimeBridgeScript(),
            &[],
        ),
        JsBootstrapModule::new(
            "quickjs/init/toolpkg-bridge.js",
            buildToolPkgRegistrationBridgeScript(),
            &["ToolPkg"],
        ),
        JsBootstrapModule::new(
            "quickjs/init/tools.js",
            getJsToolsDefinition().to_string(),
            &["Tools"],
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
                "Intent",
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
pub fn buildRuntimeBootstrapScript() -> String {
    let executionPreludeJson =
        serde_json::to_string(&JsExecutionScriptBuilder::buildExecutionPreludeSource())
            .unwrap_or_else(|_| "\"\"".to_string());
    format!(
        r#"
        {}
        var globalThis = this;
        var window = globalThis;
        var __operitRuntimePrelude = {};
        {}
        var console = {{
            log: function() {{ NativeInterface.logInfoForCall('', Array.prototype.slice.call(arguments).join(' ')); }},
            info: function() {{ NativeInterface.logInfoForCall('', Array.prototype.slice.call(arguments).join(' ')); }},
            warn: function() {{ NativeInterface.logInfoForCall('', Array.prototype.slice.call(arguments).join(' ')); }},
            error: function() {{ NativeInterface.logErrorForCall('', Array.prototype.slice.call(arguments).join(' ')); }}
        }};
        var NativeInterface = {{
            callTool: function(toolType, toolName, paramsJson) {{
                return __operitNativeCallTool(String(toolType || 'default'), String(toolName || ''), String(paramsJson || '{{}}'));
            }},
            callToolAsync: function(callbackId, toolType, toolName, paramsJson) {{
                var raw = __operitNativeCallTool(String(toolType || 'default'), String(toolName || ''), String(paramsJson || '{{}}'));
                var parsed;
                try {{
                    parsed = JSON.parse(raw);
                }} catch (_error) {{
                    parsed = {{ success: false, message: String(raw || '') }};
                }}
                if (typeof window[callbackId] === 'function') {{
                    window[callbackId](parsed, !parsed.success);
                }}
            }},
            callToolAsyncStreaming: function(callbackId, intermediateCallbackId, toolType, toolName, paramsJson) {{
                this.callToolAsync(callbackId, toolType, toolName, paramsJson);
            }},
            logInfoForCall: function() {{}},
            logErrorForCall: function() {{}},
            reportErrorForCall: function() {{}},
            sendCallIntermediateResult: function(callId, result) {{
                __operitSendIntermediateResult(
                    String(callId || ''),
                    String(result == null ? '' : result)
                );
            }},
            readToolPkgTextResource: function(packageNameOrSubpackageId, resourcePath) {{
                return __operitNativeReadToolPkgTextResource(
                    String(packageNameOrSubpackageId || ''),
                    String(resourcePath || '')
                );
            }},
            readToolPkgResource: function(packageNameOrSubpackageId, resourceKey, outputFileName, internal) {{
                return __operitNativeReadToolPkgResource(
                    String(packageNameOrSubpackageId || ''),
                    String(resourceKey || ''),
                    outputFileName == null ? '' : String(outputFileName),
                    String(internal || '')
                );
            }},
            composeWebViewControllerCommand: function(payloadJson) {{
                return __operitNativeComposeWebViewControllerCommand(String(payloadJson || '{{}}'));
            }},
            composeWebViewControllerCommandSuspend: function(payloadJson, callbackId) {{
                var normalizedCallbackId = String(callbackId || '').trim();
                if (!normalizedCallbackId) {{
                    return;
                }}
                try {{
                    var result = __operitNativeComposeWebViewControllerCommand(
                        String(payloadJson || '{{}}')
                    );
                    var parsed;
                    try {{
                        parsed = JSON.parse(result);
                    }} catch (_parseError) {{
                        parsed = result;
                    }}
                    if (typeof window[normalizedCallbackId] === 'function') {{
                        window[normalizedCallbackId](parsed, false);
                    }}
                }} catch (error) {{
                    if (typeof window[normalizedCallbackId] === 'function') {{
                        window[normalizedCallbackId]({{
                            success: false,
                            message: String(error && error.message ? error.message : error)
                        }}, true);
                    }}
                }}
            }},
            getEnvForCall: function(callId, key) {{
                return __operitNativeGetEnvForCall(String(callId || ''), String(key || ''));
            }},
            getPluginConfigDir: function(pluginId) {{
                return __operitNativeGetPluginConfigDir(String(pluginId || ''));
            }},
            invokeToolPkgIpcAsync: function(callbackId, packageTarget, callerContextKey, targetContextKey, targetRuntime, channel, payloadJson) {{
                var normalizedCallbackId = String(callbackId || '').trim();
                if (!normalizedCallbackId) {{
                    return;
                }}
                try {{
                    var resultJson = __operitNativeInvokeToolPkgIpc(
                        String(packageTarget || ''),
                        String(callerContextKey || ''),
                        String(targetContextKey || ''),
                        String(targetRuntime || ''),
                        String(channel || ''),
                        String(payloadJson || '')
                    );
                    if (typeof window[normalizedCallbackId] === 'function') {{
                        window[normalizedCallbackId](resultJson, false);
                    }}
                }} catch (error) {{
                    if (typeof window[normalizedCallbackId] === 'function') {{
                        window[normalizedCallbackId](String(error && error.message ? error.message : error), true);
                    }}
                }}
            }},
            logJsExecutionTrace: function(callId, message) {{
                __operitNativeLogJsExecutionTrace(String(callId || ''), String(message || ''));
            }},
            decompress: function(data, algorithm) {{
                return __operitNativeDecompress(String(data), String(algorithm));
            }},
            crypto: function(algorithm, operation, argsJson) {{
                return __operitNativeCrypto(
                    String(algorithm),
                    String(operation),
                    String(argsJson)
                );
            }},
            image_processing: function(callbackId, operation, argsJson) {{
                var raw = __operitNativeImageProcessing(
                    String(callbackId),
                    String(operation),
                    String(argsJson)
                );
                var parsed = JSON.parse(raw);
                var callback = window[String(callbackId)];
                if (typeof callback === 'function') {{
                    if (parsed.success) {{
                        callback(parsed.result, false);
                    }} else {{
                        callback(parsed.error, true);
                    }}
                }} else {{
                    console.error("Callback not found: " + String(callbackId));
                }}
            }},
            javaClassExists: function(className) {{
                return __operitNativeJavaClassExists(String(className || ''));
            }},
            javaGetApplicationContext: function() {{
                return __operitNativeJavaGetApplicationContext();
            }},
            javaGetCurrentActivity: function() {{
                throw new Error('current activity is null');
            }},
            javaNewInstance: function(className, argsJson) {{
                return __operitNativeJavaNewInstance(
                    String(className || ''),
                    String(argsJson || '[]')
                );
            }},
            javaCallStatic: function(className, methodName, argsJson) {{
                return __operitNativeJavaCallStatic(
                    String(className || ''),
                    String(methodName || ''),
                    String(argsJson || '[]')
                );
            }},
            javaCallInstance: function(instanceHandle, methodName, argsJson) {{
                return __operitNativeJavaCallInstance(
                    String(instanceHandle || ''),
                    String(methodName || ''),
                    String(argsJson || '[]')
                );
            }},
            setCallResult: function(callId, result) {{
                __operitNativeSetCallResult(String(callId || ''), String(result == null ? '' : result));
            }},
            setCallError: function(callId, error) {{
                __operitNativeSetCallError(String(callId || ''), String(error == null ? '' : error));
            }}
        }};

        {}

        function __operitParseToolResult(result, isError) {{
            if (isError) {{
                if (result && typeof result === 'object' && result.success === false) {{
                    var err = new Error(String(result.message || 'Tool call failed'));
                    err.data = result.data;
                    throw err;
                }}
                throw new Error(typeof result === 'string' ? result : JSON.stringify(result));
            }}
            if (result && typeof result === 'object' && Object.prototype.hasOwnProperty.call(result, 'success')) {{
                if (result.success) {{
                    return result.data;
                }}
                var error = new Error(String(result.message || 'Tool call failed'));
                error.data = result.data;
                throw error;
            }}
            if (typeof result === 'string' && result.length > 1) {{
                var first = result.charAt(0);
                if (first === '{{' || first === '[') {{
                    try {{
                        return __operitParseToolResult(JSON.parse(result), false);
                    }} catch (_error) {{
                        return result;
                    }}
                }}
            }}
            return result;
        }}

        function toolCall() {{
            var type = 'default';
            var name = '';
            var params = {{}};
            if (arguments.length === 1 && typeof arguments[0] === 'object') {{
                type = String(arguments[0].type || 'default');
                name = String(arguments[0].name || '');
                params = arguments[0].params || {{}};
            }} else if (arguments.length === 1) {{
                name = String(arguments[0] || '');
            }} else if (arguments.length === 2) {{
                name = String(arguments[0] || '');
                params = arguments[1] || {{}};
            }} else {{
                type = String(arguments[0] || 'default');
                name = String(arguments[1] || '');
                params = arguments[2] || {{}};
            }}
            var raw = NativeInterface.callTool(type, name, JSON.stringify(params));
            var parsed;
            try {{
                parsed = JSON.parse(raw);
            }} catch (_parseError) {{
                parsed = raw;
            }}
            return __operitParseToolResult(parsed, false);
        }}

        globalThis.__operitCompleteCalled = false;
        globalThis.__operitCompleteValue = undefined;
        function complete(value) {{
            globalThis.__operitCompleteCalled = true;
            globalThis.__operitCompleteValue = value;
        }}

        function sendIntermediateResult(value) {{
            __operitSendIntermediateResult(__operitFinishExecutionResult(value));
        }}
        var emit = sendIntermediateResult;
        var delta = sendIntermediateResult;
        var log = sendIntermediateResult;
        var update = sendIntermediateResult;

        function __operitFinishExecutionResult(result) {{
            if (result && result.__operit_error) {{
                return JSON.stringify({{
                    success: false,
                    message: String(result.message || ''),
                    data: result.data
                }});
            }}
            if (result !== null && typeof result === 'object') {{
                return JSON.stringify(result);
            }}
            return result === undefined ? "undefined" : String(result);
        }}

        function __operitHasUsableJavaInstanceMarker(value) {{
            if (!value || typeof value !== 'object') {{
                return false;
            }}
            try {{
                return (
                    Object.prototype.hasOwnProperty.call(value, '__javaHandle') &&
                    Object.prototype.hasOwnProperty.call(value, '__javaClass') &&
                    typeof value.__javaHandle === 'string' &&
                    typeof value.__javaClass === 'string' &&
                    __operitText(value.__javaHandle).trim().length > 0 &&
                    __operitText(value.__javaClass).trim().length > 0
                );
            }} catch (_javaMarkerError) {{
                return false;
            }}
        }}

        function __operitNormalizeSerializableValue(value, seen) {{
            if (value == null || typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') {{
                return value;
            }}
            if (typeof value === 'bigint' || typeof value === 'function') {{
                return String(value);
            }}
            if (typeof value !== 'object') {{
                return String(value);
            }}
            seen = seen || [];
            if (seen.indexOf(value) >= 0) {{
                return '[Circular]';
            }}
            seen.push(value);
            try {{
                if (typeof value.toJSON === 'function') {{
                    return __operitNormalizeSerializableValue(value.toJSON(), seen);
                }}
                if (Array.isArray(value)) {{
                    return value.map(function(item) {{
                        return __operitNormalizeSerializableValue(item, seen);
                    }});
                }}
                if (__operitHasUsableJavaInstanceMarker(value)) {{
                    return {{
                        __javaHandle: __operitText(value.__javaHandle),
                        __javaClass: __operitText(value.__javaClass)
                    }};
                }}
                var out = {{}};
                Object.keys(value).forEach(function(key) {{
                    out[key] = __operitNormalizeSerializableValue(value[key], seen);
                }});
                return out;
            }} finally {{
                seen.pop();
            }}
        }}

        function __operitSerializeOrThrow(value) {{
            return JSON.stringify(__operitNormalizeSerializableValue(value, []));
        }}

        function __operitSafeSerialize(value) {{
            try {{
                return __operitSerializeOrThrow(value);
            }} catch (error) {{
                return JSON.stringify({{
                    error: 'Failed to serialize value',
                    message: __operitText(error && error.message ? error.message : error),
                    value: __operitText(value).slice(0, 1000)
                }});
            }}
        }}

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

        function __operitGetFactory(kind, identity, source) {{
            var key = __operitBuildFactoryKey(kind, identity, source);
            var cache = __operitGetFactoryCache();
            if (typeof cache[key] === 'function') {{
                return cache[key];
            }}
            var factory = new Function(
                'module',
                'exports',
                'require',
                '__operit_call_runtime',
                __operitRuntimePrelude + '\n' + source
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

        function __operitExecuteScriptFunction(callId, params, scriptText, targetFunctionName, timeoutSec, preTimeoutMs) {{
            var previousCallRuntime = globalThis.__operit_call_runtime_ref;
            var previousCallId = globalThis.__operitCurrentCallId;
            var registerCallSession = globalThis.__operitRegisterCallSession;
            if (typeof registerCallSession !== 'function') {{
                NativeInterface.setCallError(callId, JSON.stringify({{
                    success: false,
                    message: 'JS execution runtime bridge is unavailable'
                }}));
                return;
            }}
            var callState = registerCallSession(callId, params);
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
                if (typeof globalThis.__operitCleanupCallSession === 'function') {{
                    globalThis.__operitCleanupCallSession(callId);
                }}
            }}
            function isActive() {{
                var state = getCallState();
                return !!(state && !state.completed);
            }}
            function readCallValue(key, fallbackValue) {{
                var state = getCallState();
                var currentParams = state && state.params && typeof state.params === 'object'
                    ? state.params
                    : null;
                var value = currentParams ? currentParams[key] : undefined;
                return value == null || value === '' ? fallbackValue : __operitText(value);
            }}
            function markStage(stage) {{
                callState.lastExecStage = __operitText(stage);
                NativeInterface.logJsExecutionTrace(
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
                NativeInterface.logJsExecutionTrace(
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
                NativeInterface.logJsExecutionTrace(
                    callId,
                    'require=' + callState.lastRequireRequest +
                        ' from=' + callState.lastRequireFrom +
                        ' resolved=' + callState.lastRequireResolved
                );
            }}
            function markModule(modulePath) {{
                callState.lastModulePath = __operitText(modulePath);
                NativeInterface.logJsExecutionTrace(callId, 'module=' + callState.lastModulePath);
            }}
            function completeCall(resultText) {{
                var state = getCallState();
                if (!state || state.completed) {{
                    return;
                }}
                state.completed = true;
                NativeInterface.logJsExecutionTrace(callId, 'complete ' + __operitText(resultText).slice(0, 240));
                NativeInterface.setCallResult(callId, resultText);
                finalizeCall();
            }}
            function emitError(message) {{
                var state = getCallState();
                if (!state || state.completed) {{
                    return;
                }}
                state.completed = true;
                NativeInterface.logJsExecutionTrace(callId, 'error ' + __operitText(message).slice(0, 240));
                NativeInterface.setCallError(callId, JSON.stringify({{
                    success: false,
                    message: __operitText(message)
                }}));
                finalizeCall();
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
            function emitIntermediate(value) {{
                if (isActive()) {{
                    NativeInterface.sendCallIntermediateResult(callId, __operitSafeSerialize(value));
                }}
            }}
            function complete(value) {{
                try {{
                    completeCall(__operitSerializeOrThrow(__operitNormalizeComposeResult(value)));
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
                getCallerCardId: function() {{ return readCallValue('__operit_package_caller_card_id', undefined); }},
                getEnv: function(key) {{
                    var value = NativeInterface.getEnvForCall(callId, __operitText(key).trim());
                    return value == null || value === '' ? undefined : __operitText(value);
                }},
                getPluginConfigDir: function(pluginId) {{
                    var explicitId = pluginId == null ? '' : __operitText(pluginId).trim();
                    var resolvedId =
                        explicitId ||
                        readCallValue('__operit_ui_package_name', '') ||
                        readCallValue('toolPkgId', '') ||
                        readCallValue('containerPackageName', '') ||
                        readCallValue('__operit_package_name', '');
                    if (
                        !resolvedId ||
                        typeof NativeInterface === 'undefined' ||
                        !NativeInterface ||
                        typeof NativeInterface.getPluginConfigDir !== 'function'
                    ) {{
                        return '';
                    }}
                    var path = NativeInterface.getPluginConfigDir(resolvedId);
                    return typeof path === 'string' ? path : '';
                }},
                reportDetailedError: callRuntimeReport,
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
                        if (
                            !packageTarget ||
                            typeof NativeInterface === 'undefined' ||
                            !NativeInterface ||
                            typeof NativeInterface.invokeToolPkgIpcAsync !== 'function'
                        ) {{
                            return Promise.reject(new Error('ToolPkg.ipc runtime bridge is unavailable'));
                        }}
                        var payloadJson;
                        try {{
                            payloadJson = __operitSerializeOrThrow(payload);
                        }} catch (error) {{
                            try {{
                                if (
                                    typeof NativeInterface !== 'undefined' &&
                                    NativeInterface &&
                                    typeof NativeInterface.logErrorForCall === 'function'
                                ) {{
                                    NativeInterface.logErrorForCall(
                                        callId,
                                        'ToolPkg.ipc payload serialization failed: ' +
                                            __operitText(error && error.message ? error.message : error)
                                    );
                                }}
                            }} catch (_logIpcPayloadError) {{}}
                            return Promise.reject(error);
                        }}
                        return new Promise(function(resolve, reject) {{
                            var callbackId =
                                '__operit_toolpkg_ipc_' +
                                Date.now() +
                                '_' +
                                Math.random().toString(36).slice(2, 10);
                            globalThis[callbackId] = function(resultJson, isError) {{
                                try {{
                                    delete globalThis[callbackId];
                                }} catch (_deleteCallbackError) {{
                                    globalThis[callbackId] = undefined;
                                }}
                                if (isError) {{
                                    reject(new Error(__operitText(resultJson).trim() || 'ToolPkg.ipc call failed'));
                                    return;
                                }}
                                var parsed;
                                try {{
                                    parsed = JSON.parse(__operitText(resultJson) || 'null');
                                }} catch (error) {{
                                    try {{
                                        if (
                                            typeof NativeInterface !== 'undefined' &&
                                            NativeInterface &&
                                            typeof NativeInterface.logErrorForCall === 'function'
                                        ) {{
                                            var resultType = resultJson === null ? 'null' : typeof resultJson;
                                            var preview = __operitText(resultJson).slice(0, 500);
                                            NativeInterface.logErrorForCall(
                                                callId,
                                                'ToolPkg.ipc returned invalid JSON: ' +
                                                    __operitText(error && error.message ? error.message : error) +
                                                    ', resultType=' + resultType +
                                                    ', preview=' + preview
                                            );
                                        }}
                                    }} catch (_logIpcParseError) {{}}
                                    reject(
                                        new Error(
                                            'ToolPkg.ipc returned invalid JSON: ' +
                                                __operitText(error && error.message ? error.message : error)
                                        )
                                    );
                                    return;
                                }}
                                if (parsed && parsed.success === true) {{
                                    resolve(parsed.value);
                                    return;
                                }}
                                reject(
                                    new Error(
                                        parsed && typeof parsed.message === 'string' && parsed.message.trim().length > 0
                                            ? parsed.message.trim()
                                            : 'ToolPkg.ipc call failed'
                                    )
                                );
                            }};
                            try {{
                                NativeInterface.invokeToolPkgIpcAsync(
                                    callbackId,
                                    packageTarget,
                                    currentContextKey,
                                    targetContextKey,
                                    targetRuntime,
                                    normalizedChannel,
                                    payloadJson
                                );
                            }} catch (error) {{
                                try {{
                                    delete globalThis[callbackId];
                                }} catch (_deleteCallbackInvokeError) {{
                                    globalThis[callbackId] = undefined;
                                }}
                                reject(error);
                            }}
                        }});
                    }};

                    toolPkgApi.ipc = ipcApi;
                    globalThis.__operitInvokeToolPkgIpcLocal = invokeToolPkgIpcLocal;
                }}

                ensureToolPkgIpcApi();
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
                function readToolPkgModule(modulePath) {{
                    if (!packageTarget || !NativeInterface || typeof NativeInterface.readToolPkgTextResource !== 'function') {{
                        return null;
                    }}
                    var candidates = __operitBuildCandidatePaths(modulePath);
                    for (var i = 0; i < candidates.length; i += 1) {{
                        var candidate = candidates[i];
                        var textResult = NativeInterface.readToolPkgTextResource(packageTarget, candidate);
                        if (typeof textResult === 'string' && textResult.length > 0) {{
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
                    var factory = __operitGetFactory('module', packageTarget + ':' + modulePath, moduleText);
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
                    var mainFactory = __operitGetFactory('main', packageTarget + ':' + screenPath, scriptText);
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
        "#,
        JsInitRuntimeScriptBuilder::buildRuntimeBootstrapScript(),
        executionPreludeJson,
        buildJavaClassBridgeDefinition(),
        buildComposeDslContextBridgeDefinition(),
        buildToolPkgRegistrationBridgeScript(),
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

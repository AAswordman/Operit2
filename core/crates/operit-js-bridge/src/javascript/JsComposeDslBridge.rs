#[allow(non_snake_case)]
pub fn buildComposeDslContextBridgeDefinition() -> String {
    r#"
        var OperitComposeDslRuntime = (function() {
            function cloneObject(input) {
                if (!input || typeof input !== 'object' || Array.isArray(input)) {
                    return {};
                }
                var out = {};
                for (var key in input) {
                    if (Object.prototype.hasOwnProperty.call(input, key)) {
                        out[key] = input[key];
                    }
                }
                return out;
            }

            function isComposeNodeLike(value) {
                return !!(
                    value &&
                    typeof value === 'object' &&
                    value.__composeNode === true &&
                    typeof value.type === 'string'
                );
            }

            function flattenComposeValue(value, out) {
                if (value == null) {
                    return;
                }
                if (Array.isArray(value)) {
                    for (var i = 0; i < value.length; i += 1) {
                        flattenComposeValue(value[i], out);
                    }
                    return;
                }
                if (isComposeNodeLike(value)) {
                    out.push(value);
                }
            }

            function normalizeChildren(children) {
                var out = [];
                flattenComposeValue(children, out);
                return out;
            }

            function normalizeSlotChildren(value) {
                return normalizeChildren(value);
            }

            function createUserFacingError(message, detailData) {
                return {
                    name: 'Error',
                    message: String(message || '').trim(),
                    data: detailData,
                    toString: function() {
                        return this.message;
                    }
                };
            }

            function parseJsonValue(rawValue) {
                if (rawValue === undefined || rawValue === null) {
                    return undefined;
                }
                if (typeof rawValue !== 'string') {
                    return rawValue;
                }
                var trimmed = rawValue.trim();
                if (!trimmed) {
                    return undefined;
                }
                try {
                    return JSON.parse(trimmed);
                } catch (e) {
                    return rawValue;
                }
            }

            function unwrapNativeResult(rawValue) {
                var parsed = parseJsonValue(rawValue);
                if (
                    parsed &&
                    typeof parsed === 'object' &&
                    parsed.success === false
                ) {
                    throw createUserFacingError(parsed.message, parsed);
                }
                if (
                    parsed &&
                    typeof parsed === 'object' &&
                    Object.prototype.hasOwnProperty.call(parsed, 'data')
                ) {
                    return parsed.data;
                }
                return parsed;
            }

            function invokeNative(methodName, args) {
                try {
                    if (
                        typeof NativeInterface === 'undefined' ||
                        !NativeInterface ||
                        typeof NativeInterface[methodName] !== 'function'
                    ) {
                        return undefined;
                    }
                    return NativeInterface[methodName].apply(NativeInterface, args || []);
                } catch (e) {
                    console.error('Native bridge call failed for ' + methodName + ':', e);
                    return undefined;
                }
            }

            function normalizeSerializableValue(value, runtime, seen) {
                if (value == null) {
                    return value;
                }
                if (typeof value === 'function') {
                    return { __actionId: runtime.registerAction(value) };
                }
                if (typeof value !== 'object') {
                    return value;
                }
                if (seen.indexOf(value) >= 0) {
                    return null;
                }
                seen.push(value);
                if (Array.isArray(value)) {
                    var arr = [];
                    for (var i = 0; i < value.length; i += 1) {
                        arr.push(normalizeSerializableValue(value[i], runtime, seen));
                    }
                    seen.pop();
                    return arr;
                }
                if (value.__modifierOps && Array.isArray(value.__modifierOps)) {
                    seen.pop();
                    return {
                        __modifierOps: normalizeSerializableValue(value.__modifierOps, runtime, [])
                    };
                }
                var out = {};
                for (var key in value) {
                    if (Object.prototype.hasOwnProperty.call(value, key)) {
                        if (key === '__composeNode') {
                            continue;
                        }
                        out[key] = normalizeSerializableValue(value[key], runtime, seen);
                    }
                }
                seen.pop();
                return out;
            }

            function buildNode(runtime, type, props, children) {
                var rawProps = props && typeof props === 'object' && !Array.isArray(props)
                    ? props
                    : {};
                var nodeProps = {};
                var slots = {};
                var contentChildren = children;
                for (var key in rawProps) {
                    if (!Object.prototype.hasOwnProperty.call(rawProps, key)) {
                        continue;
                    }
                    var value = rawProps[key];
                    if (key === 'content' && typeof contentChildren === 'undefined') {
                        contentChildren = value;
                        continue;
                    }
                    if (isComposeNodeLike(value) || Array.isArray(value)) {
                        var slotNodes = normalizeSlotChildren(value);
                        if (slotNodes.length > 0) {
                            slots[key] = slotNodes;
                            continue;
                        }
                    }
                    nodeProps[key] = normalizeSerializableValue(value, runtime, []);
                }
                var normalizedChildren = normalizeChildren(contentChildren);
                var node = {
                    __composeNode: true,
                    type: String(type || 'Box'),
                    props: normalizeSerializableValue(nodeProps, runtime, []),
                    children: normalizedChildren
                };
                if (Object.keys(slots).length > 0) {
                    node.slots = slots;
                }
                return node;
            }

            function resolvePackageName(value) {
                var name = String(value || runtime.packageName || '').trim();
                return name;
            }

            function normalizeToolName(targetPackage, toolName) {
                var basePackage = String(targetPackage || '').trim();
                var normalizedTool = String(toolName || '').trim();
                if (!normalizedTool) {
                    return '';
                }
                if (normalizedTool.indexOf(':') >= 0 || !basePackage) {
                    return normalizedTool;
                }
                return basePackage + ':' + normalizedTool;
            }

            function createUiRegistry(runtime) {
                return new Proxy({}, {
                    get: function(_target, prop) {
                        if (typeof prop !== 'string') {
                            return undefined;
                        }
                        return function(props, children) {
                            return buildNode(runtime, prop, props, children);
                        };
                    }
                });
            }

            function createModifierProxy(ops) {
                var state = Array.isArray(ops) ? ops.slice() : [];
                function append(name, argsLike) {
                    var args = Array.prototype.slice.call(argsLike || []);
                    return createModifierProxy(state.concat([{ name: name, args: args }]));
                }
                return new Proxy({ __modifierOps: state }, {
                    get: function(target, prop) {
                        if (prop === '__modifierOps') {
                            return target.__modifierOps;
                        }
                        if (prop === 'toJSON') {
                            return function() {
                                return { __modifierOps: target.__modifierOps };
                            };
                        }
                        if (typeof prop !== 'string') {
                            return undefined;
                        }
                        return function() {
                            return append(prop, arguments);
                        };
                    }
                });
            }

            function makeColorToken(name, alpha) {
                return {
                    __colorToken: name,
                    alpha: alpha,
                    copy: function(options) {
                        return makeColorToken(name, options && typeof options.alpha === 'number' ? options.alpha : alpha);
                    }
                };
            }

            function createContext(runtimeOptions) {
                var options = runtimeOptions && typeof runtimeOptions === 'object' ? runtimeOptions : {};
                var runtime = {
                    stateStore: cloneObject(options.state),
                    memoStore: cloneObject(options.memo),
                    moduleSpec:
                        options.moduleSpec && typeof options.moduleSpec === 'object'
                            ? options.moduleSpec
                            : {},
                    packageName: String(options.packageName || options.__operit_ui_package_name || ''),
                    toolPkgId: String(options.toolPkgId || options.__operit_ui_toolpkg_id || ''),
                    uiModuleId: String(options.uiModuleId || options.__operit_ui_module_id || ''),
                    routeInstanceId: String(options.routeInstanceId || options.__operit_route_instance_id || ''),
                    executionContextKey: String(
                        options.executionContextKey || options.__operit_compose_execution_context_key || ''
                    ),
                    callRuntime:
                        options.__operit_call_runtime && typeof options.__operit_call_runtime === 'object'
                            ? options.__operit_call_runtime
                            : null,
                    actionStore: {},
                    actionCounter: 0,
                    stateChangeListeners: [],
                    stateChangeScheduled: false,
                    stateDirty: false,
                    pendingStateChangePromise: null
                };

                runtime.registerAction = function(handler) {
                    runtime.actionCounter += 1;
                    var actionId = '__action_' + runtime.actionCounter;
                    runtime.actionStore[actionId] = handler;
                    return actionId;
                };

                function notifyStateChanged() {
                    runtime.stateDirty = true;
                    if (runtime.stateChangeScheduled) {
                        return;
                    }
                    runtime.stateChangeScheduled = true;
                    runtime.pendingStateChangePromise = Promise.resolve().then(function() {
                        try {
                            runtime.stateChangeScheduled = false;
                            if (!runtime.stateDirty) {
                                return;
                            }
                            runtime.stateDirty = false;
                            flushStateChangeListeners();
                        } finally {
                            runtime.pendingStateChangePromise = null;
                        }
                    });
                }

                function flushStateChangeListeners() {
                    if (!runtime.stateChangeListeners || runtime.stateChangeListeners.length <= 0) {
                        return;
                    }
                    var listeners = runtime.stateChangeListeners.slice();
                    for (var i = 0; i < listeners.length; i += 1) {
                        try {
                            listeners[i]();
                        } catch (e) {
                            try {
                                console.warn('compose_dsl state listener failed:', e);
                            } catch (__ignore) {
                            }
                        }
                    }
                }

                function subscribeStateChange(listener) {
                    if (typeof listener !== 'function') {
                        return function() {};
                    }
                    runtime.stateChangeListeners.push(listener);
                    var active = true;
                    return function() {
                        if (!active) {
                            return;
                        }
                        active = false;
                        var index = runtime.stateChangeListeners.indexOf(listener);
                        if (index >= 0) {
                            runtime.stateChangeListeners.splice(index, 1);
                        }
                    };
                }

                function flushPendingStateChanges() {
                    if (runtime.pendingStateChangePromise && typeof runtime.pendingStateChangePromise.then === 'function') {
                        return runtime.pendingStateChangePromise;
                    }
                    return Promise.resolve();
                }

                var ui = createUiRegistry(runtime);
                var colorSchemeNames = [
                    'primary', 'onPrimary', 'primaryContainer', 'onPrimaryContainer',
                    'secondary', 'onSecondary', 'secondaryContainer', 'onSecondaryContainer',
                    'tertiary', 'onTertiary', 'tertiaryContainer', 'onTertiaryContainer',
                    'error', 'onError', 'errorContainer', 'onErrorContainer',
                    'background', 'onBackground', 'surface', 'onSurface',
                    'surfaceVariant', 'onSurfaceVariant', 'outline', 'outlineVariant',
                    'inverseSurface', 'inverseOnSurface', 'inversePrimary',
                    'surfaceTint', 'scrim'
                ];
                var colorScheme = {};
                for (var c = 0; c < colorSchemeNames.length; c += 1) {
                    colorScheme[colorSchemeNames[c]] = makeColorToken(colorSchemeNames[c]);
                }

                function createWebViewController(key) {
                    var controllerKey = String(key || '').trim();
                    if (!controllerKey) {
                        throw new Error('webview controller key is required');
                    }
                    var descriptor = {
                        __composeWebViewController: true,
                        key: controllerKey,
                        routeInstanceId: runtime.routeInstanceId || '',
                        executionContextKey: runtime.executionContextKey || ''
                    };
                    function invokeControllerCommand(command, payload) {
                        return unwrapNativeResult(
                            invokeNative('composeWebViewControllerCommand', [
                                JSON.stringify({
                                    command: String(command || ''),
                                    key: controllerKey,
                                    routeInstanceId: runtime.routeInstanceId || '',
                                    executionContextKey: runtime.executionContextKey || '',
                                    payload: normalizeSerializableValue(
                                        payload && typeof payload === 'object'
                                            ? payload
                                            : {},
                                        runtime,
                                        []
                                    )
                                })
                            ])
                        );
                    }
                    function nextWebViewControllerCallbackId() {
                        return '__operit_compose_webview_' + Date.now() + '_' + Math.random().toString(36).slice(2, 10);
                    }
                    function invokeControllerCommandSuspend(command, payload) {
                        if (typeof Promise !== 'function') {
                            throw new Error('Promise is required for suspend webview controller command');
                        }
                        return new Promise(function(resolve, reject) {
                            if (
                                typeof NativeInterface === 'undefined' ||
                                !NativeInterface ||
                                typeof NativeInterface.composeWebViewControllerCommandSuspend !== 'function'
                            ) {
                                reject(createUserFacingError('NativeInterface.composeWebViewControllerCommandSuspend is unavailable'));
                                return;
                            }
                            var root = typeof globalThis !== 'undefined'
                                ? globalThis
                                : (typeof window !== 'undefined' ? window : this);
                            var callbackTarget = typeof window !== 'undefined' ? window : root;
                            var callbackId = nextWebViewControllerCallbackId();
                            callbackTarget[callbackId] = function(result, isError) {
                                delete callbackTarget[callbackId];
                                try {
                                    if (isError) {
                                        reject(createUserFacingError(result.message, result));
                                        return;
                                    }
                                    resolve(unwrapNativeResult(result));
                                } catch (callbackError) {
                                    reject(callbackError);
                                }
                            };
                            try {
                                NativeInterface.composeWebViewControllerCommandSuspend(
                                    JSON.stringify({
                                        command: String(command || ''),
                                        key: controllerKey,
                                        routeInstanceId: runtime.routeInstanceId || '',
                                        executionContextKey: runtime.executionContextKey || '',
                                        payload: normalizeSerializableValue(
                                            payload && typeof payload === 'object'
                                                ? payload
                                                : {},
                                            runtime,
                                            []
                                        )
                                    }),
                                    callbackId
                                );
                            } catch (invokeError) {
                                delete callbackTarget[callbackId];
                                reject(invokeError);
                            }
                        });
                    }
                    function defineMethod(target, name, handler) {
                        Object.defineProperty(target, name, {
                            configurable: false,
                            enumerable: false,
                            writable: false,
                            value: handler
                        });
                    }

                    var controller = cloneObject(descriptor);
                    defineMethod(controller, 'toJSON', function() {
                        return cloneObject(descriptor);
                    });
                    defineMethod(controller, 'loadUrl', function(url, headers) {
                        var finalUrl = String(url || '').trim();
                        if (!finalUrl) {
                            throw new Error('webview controller loadUrl requires a non-empty url');
                        }
                        invokeControllerCommand('loadUrl', {
                            url: finalUrl,
                            headers: headers && typeof headers === 'object' ? headers : {}
                        });
                    });
                    defineMethod(controller, 'loadHtml', function(html, options) {
                        invokeControllerCommand('loadHtml', {
                            html: html == null ? '' : String(html),
                            options: options && typeof options === 'object' ? options : {}
                        });
                    });
                    defineMethod(controller, 'reload', function() {
                        invokeControllerCommand('reload', {});
                    });
                    defineMethod(controller, 'stopLoading', function() {
                        invokeControllerCommand('stopLoading', {});
                    });
                    defineMethod(controller, 'goBack', function() {
                        invokeControllerCommand('goBack', {});
                    });
                    defineMethod(controller, 'goForward', function() {
                        invokeControllerCommand('goForward', {});
                    });
                    defineMethod(controller, 'clearHistory', function() {
                        invokeControllerCommand('clearHistory', {});
                    });
                    defineMethod(controller, 'evaluateJavascript', function(script) {
                        return Promise.resolve(
                            invokeControllerCommandSuspend('evaluateJavascript', {
                                script: script == null ? '' : String(script)
                            })
                        );
                    });
                    defineMethod(controller, 'getState', function() {
                        return invokeControllerCommand('getState', {});
                    });
                    defineMethod(controller, 'addJavascriptInterface', function(name, object) {
                        var interfaceName = String(name || '').trim();
                        if (!interfaceName) {
                            throw new Error('webview controller addJavascriptInterface requires a non-empty name');
                        }
                        if (!object || typeof object !== 'object' || Array.isArray(object)) {
                            throw new Error('webview controller addJavascriptInterface requires an object');
                        }
                        invokeControllerCommand('addJavascriptInterface', {
                            name: interfaceName,
                            object: object
                        });
                    });
                    defineMethod(controller, 'removeJavascriptInterface', function(name) {
                        var interfaceName = String(name || '').trim();
                        if (!interfaceName) {
                            throw new Error('webview controller removeJavascriptInterface requires a non-empty name');
                        }
                        invokeControllerCommand('removeJavascriptInterface', {
                            name: interfaceName
                        });
                    });
                    return controller;
                }

                var ctx = {
                    MaterialTheme: { colorScheme: colorScheme },
                    useState: function(key, initialValue) {
                        var stateKey = String(key || '').trim();
                        if (!stateKey) {
                            throw new Error('useState key is required');
                        }
                        if (!Object.prototype.hasOwnProperty.call(runtime.stateStore, stateKey)) {
                            runtime.stateStore[stateKey] = initialValue;
                        }
                        return [
                            runtime.stateStore[stateKey],
                            function(nextValue) {
                                runtime.stateStore[stateKey] = nextValue;
                                notifyStateChanged();
                            }
                        ];
                    },
                    useMutable: function(key, initialValue) {
                        var stateKey = String(key || '').trim();
                        if (!stateKey) {
                            throw new Error('useMutable key is required');
                        }
                        if (!Object.prototype.hasOwnProperty.call(runtime.memoStore, stateKey)) {
                            runtime.memoStore[stateKey] = initialValue;
                        }
                        return [
                            runtime.memoStore[stateKey],
                            function(nextValue) {
                                runtime.memoStore[stateKey] = nextValue;
                            }
                        ];
                    },
                    useRef: function(key, initialValue) {
                        var stateKey = String(key || '').trim();
                        if (!stateKey) {
                            throw new Error('useRef key is required');
                        }
                        if (!Object.prototype.hasOwnProperty.call(runtime.memoStore, stateKey)) {
                            runtime.memoStore[stateKey] = { current: initialValue };
                        }
                        return runtime.memoStore[stateKey];
                    },
                    useMemo: function(key, factory, deps) {
                        var memoKey = 'memo:' + String(key || '');
                        var current = runtime.memoStore[memoKey];
                        var depsJson = JSON.stringify(deps || []);
                        if (!current || current.depsJson !== depsJson) {
                            current = { depsJson: depsJson, value: factory() };
                            runtime.memoStore[memoKey] = current;
                        }
                        return current.value;
                    },
                    measureText: function(request) {
                        var text = request && request.text != null ? String(request.text) : '';
                        var fontSize = request && request.fontSize ? Number(request.fontSize) : 14;
                        return {
                            width: Math.min((request && request.maxWidth) || 100000, text.length * fontSize * 0.56),
                            height: Math.min((request && request.maxHeight) || 100000, fontSize * 1.4)
                        };
                    },
                    getEnv: function(key) {
                        if (typeof getEnv === 'function') {
                            return unwrapNativeResult(getEnv(String(key || '')));
                        }
                        return undefined;
                    },
                    callTool: function(toolName, params) {
                        if (typeof toolCall === 'function') {
                            return toolCall(String(toolName || ''), params || {});
                        }
                        throw createUserFacingError('Tool call bridge is unavailable');
                    },
                    navigate: function(route, args) {
                        return { route: String(route || ''), args: args || {} };
                    },
                    showToast: function(message) {
                        console.log(String(message || ''));
                    },
                    reportError: function(error) {
                        console.error(error);
                    },
                    createWebViewController: function(key) {
                        return createWebViewController(key);
                    },
                    getModuleSpec: function() {
                        return runtime.moduleSpec;
                    },
                    getCurrentPackageName: function() {
                        return runtime.packageName;
                    },
                    getCurrentToolPkgId: function() {
                        return runtime.toolPkgId || runtime.packageName;
                    },
                    getCurrentUiModuleId: function() {
                        return runtime.uiModuleId;
                    },
                    isPackageImported: function(packageName) {
                        var target = resolvePackageName(packageName);
                        if (!target) {
                            return Promise.resolve(false);
                        }
                        var result = invokeNative('isPackageImported', [target]);
                        if (result === true || result === false || result === 'true' || result === 'false') {
                            return Promise.resolve(result === true || result === 'true');
                        }
                        return toolCall('is_package_imported', { package_name: target });
                    },
                    importPackage: function(packageName) {
                        var target = resolvePackageName(packageName);
                        if (!target) {
                            return Promise.resolve('');
                        }
                        var result = invokeNative('importPackage', [target]);
                        if (result !== undefined && result !== null) {
                            return Promise.resolve(result);
                        }
                        return toolCall('import_package', { package_name: target });
                    },
                    removePackage: function(packageName) {
                        var target = resolvePackageName(packageName);
                        if (!target) {
                            return Promise.resolve('');
                        }
                        var result = invokeNative('removePackage', [target]);
                        if (result !== undefined && result !== null) {
                            return Promise.resolve(result);
                        }
                        return toolCall('remove_package', { package_name: target });
                    },
                    usePackage: function(packageName) {
                        var target = resolvePackageName(packageName);
                        if (!target) {
                            return Promise.resolve('');
                        }
                        var result = invokeNative('usePackage', [target]);
                        if (result !== undefined && result !== null) {
                            return Promise.resolve(result);
                        }
                        return toolCall('use_package', { package_name: target });
                    },
                    listImportedPackages: function() {
                        var json = invokeNative('listImportedPackagesJson', []);
                        if (typeof json === 'string' && json.trim()) {
                            try {
                                return Promise.resolve(JSON.parse(json));
                            } catch (e) {
                                return Promise.resolve([]);
                            }
                        }
                        return toolCall('list_imported_packages', {});
                    },
                    resolveToolName: function(request) {
                        var req = request && typeof request === 'object' ? request : {};
                        var packageName = String(req.packageName || runtime.packageName || '');
                        var subpackageId = String(req.subpackageId || '');
                        var toolName = String(req.toolName || '');
                        var preferImported = req.preferImported === false ? 'false' : 'true';
                        if (!toolName) {
                            return Promise.resolve('');
                        }
                        var result = invokeNative('resolveToolName', [
                            packageName,
                            subpackageId,
                            toolName,
                            preferImported
                        ]);
                        if (typeof result === 'string' && result.trim()) {
                            return Promise.resolve(result);
                        }
                        return Promise.resolve(normalizeToolName(packageName, toolName));
                    },
                    formatTemplate: function(template, values) {
                        var result = String(template || '');
                        var source = values && typeof values === 'object' ? values : {};
                        for (var key in source) {
                            if (Object.prototype.hasOwnProperty.call(source, key)) {
                                result = result.split('{' + key + '}').join(source[key] == null ? '' : String(source[key]));
                            }
                        }
                        return result;
                    },
                    h: function(type, props, children) {
                        return buildNode(runtime, type, props, children);
                    },
                    Modifier: createModifierProxy([]),
                    UI: ui
                };

                runtime.ctx = ctx;
                Object.defineProperty(runtime, 'state', {
                    get: function() { return cloneObject(runtime.stateStore); }
                });
                Object.defineProperty(runtime, 'memo', {
                    get: function() { return cloneObject(runtime.memoStore); }
                });
                runtime.invokeAction = function(actionId, payload) {
                    var handler = runtime.actionStore[String(actionId || '').trim()];
                    if (typeof handler !== 'function') {
                        throw createUserFacingError('compose action not found: ' + actionId);
                    }
                    return handler(payload);
                };
                runtime.subscribeStateChange = subscribeStateChange;
                runtime.flushStateChanges = flushPendingStateChanges;
                runtime.updateRuntimeOptions = function(updatedOptions) {
                    var next = updatedOptions && typeof updatedOptions === 'object' ? updatedOptions : {};
                    if (next.state && typeof next.state === 'object') {
                        runtime.stateStore = cloneObject(next.state);
                    }
                    if (next.memo && typeof next.memo === 'object') {
                        runtime.memoStore = cloneObject(next.memo);
                    }
                    if (next.moduleSpec && typeof next.moduleSpec === 'object') {
                        runtime.moduleSpec = next.moduleSpec;
                    }
                };
                runtime.setCallRuntime = function(callRuntime) {
                    runtime.callRuntime = callRuntime;
                };
                return runtime;
            }

            return {
                createContext: createContext
            };
        })();
    "#
    .to_string()
}

/// Returns the stable JavaScript context bridge used by ToolPkg Compose DSL screens.
#[allow(non_snake_case)]
pub fn buildComposeDslContextBridgeDefinition() -> String {
    format!("{}\n{}\n{}\n{}\n{}", format!("var __operitAcorn = {{}}; (function(exports, module) {{ {} }})(__operitAcorn, {{exports: __operitAcorn}});", include_str!("vendor/acorn.js")), include_str!("ToolPkgComposeDslCompiler.js"), include_str!("ToolPkgComposeDslRetained.js"), include_str!("ToolPkgComposeDslReactive.js"), r#"
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

            /** Unwraps the explicit structured WebView result contract without parsing application strings. */
            function unwrapHostResult(result) {
                if (!result || typeof result !== 'object' || typeof result.success !== 'boolean') {
                    throw new TypeError('Compose host returned an invalid result envelope');
                }
                if (!result.success) throw createUserFacingError(result.message, result);
                return result.data;
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
                    props: nodeProps,
                    children: normalizedChildren
                };
                if (Object.keys(slots).length > 0) {
                    node.slots = slots;
                }
                return OperitComposeGeneration.observeNode(node);
            }

            /// Resolves an optional package argument inside its owning runtime.
            function resolvePackageName(value, runtime) {
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
                    navigationCommands: [],
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
                    runtime.composition.trackAction(actionId);
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
                    'surfaceTint', 'scrim', 'shadow',
                    'surfaceDim', 'surfaceBright', 'surfaceContainerLowest',
                    'surfaceContainerLow', 'surfaceContainer', 'surfaceContainerHigh', 'surfaceContainerHighest',
                    'primaryFixed', 'primaryFixedDim', 'onPrimaryFixed', 'onPrimaryFixedVariant',
                    'secondaryFixed', 'secondaryFixedDim', 'onSecondaryFixed', 'onSecondaryFixedVariant',
                    'tertiaryFixed', 'tertiaryFixedDim', 'onTertiaryFixed', 'onTertiaryFixedVariant'
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
                    /** Builds one structured controller request with its owning Compose context. */
                    function controllerRequest(command, payload) {
                        return {
                            command: String(command), key: controllerKey,
                            routeInstanceId: runtime.routeInstanceId || '',
                            executionContextKey: runtime.executionContextKey || '',
                            payload: normalizeSerializableValue(payload, runtime, [])
                        };
                    }
                    /** Executes a synchronous controller command through the structured host binding. */
                    function invokeControllerCommand(command, payload) {
                        return unwrapHostResult(__operitNativeComposeWebViewControllerCommand(
                            controllerRequest(command, payload)
                        ));
                    }
                    /** Executes an asynchronous controller command through its owning host Promise. */
                    function invokeControllerCommandSuspend(command, payload) {
                        return __operitInvokeHostAsync(__operitNativeComposeWebViewControllerCommandSuspend,
                            [controllerRequest(command, payload)], unwrapHostResult);
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

                var themeSnapshot;
                var themeListeners = new Set();
                /** Validates and detaches one resolved UI host theme. */
                function readThemeSnapshot(value) {
                    if (!value || (value.brightness !== 'light' && value.brightness !== 'dark') ||
                        !value.colors || typeof value.colors !== 'object' || Array.isArray(value.colors)) {
                        throw new Error('Theme requires a resolved UI host snapshot');
                    }
                    var colors = Object.assign({}, value.colors);
                    Object.keys(colors).forEach(function(role) {
                        if (typeof colors[role] !== 'string' || !/^#[0-9a-fA-F]{8}$/.test(colors[role])) {
                            throw new Error('Invalid Theme color: ' + role);
                        }
                    });
                    return Object.freeze({ brightness: value.brightness, colors: Object.freeze(colors) });
                }
                if (options.theme !== undefined) themeSnapshot = readThemeSnapshot(options.theme);
                /** Delivers host theme changes through the existing UI action lifecycle. */
                runtime.actionStore.__operit_theme_changed = async function(value) {
                    var next = readThemeSnapshot(value);
                    if (JSON.stringify(next) === JSON.stringify(themeSnapshot)) return;
                    themeSnapshot = next;
                    OperitComposeReactive.environmentChanged(runtime.ctx);
                    for (var listener of Array.from(themeListeners)) await listener(next);
                };
                var ctx = {
                    Theme: {
                        /** Returns the immutable snapshot owned by this UI context. */
                        getCurrent: function() {
                            if (!themeSnapshot) throw new Error('Theme is unavailable outside a configured UI host');
                            return themeSnapshot;
                        },
                        /** Registers a listener until explicitly removed or the UI context is released. */
                        subscribe: function(listener) {
                            if (typeof listener !== 'function') throw new TypeError('Theme listener must be a function');
                            themeListeners.add(listener);
                            return function() { themeListeners.delete(listener); };
                        }
                    },
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
                                OperitComposeReactive.stateChanged(runtime.ctx, stateKey);
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
                    /** Reads literal environment values through the active execution context. */
                    getEnv: function(key) {
                        return runtime.callRuntime.getEnv(String(key));
                    },
                    /** Writes one environment value through the structured host binding. */
                    setEnv: async function(key, value) {
                        __operitNativeSetEnv(String(globalThis.__operitCurrentCallId), String(key),
                            value == null ? '' : String(value));
                    },
                    /** Writes an environment object without a JSON text bridge. */
                    setEnvs: async function(values) {
                        __operitNativeSetEnvs(String(globalThis.__operitCurrentCallId), values);
                    },
                    callTool: function(toolName, params) {
                        if (typeof toolCall === 'function') {
                            return toolCall(String(toolName || ''), params || {});
                        }
                        throw createUserFacingError('Tool call bridge is unavailable');
                    },
                    /// Queues navigation independently of the action handler's return value.
                    navigate: function(route, args) {
                        var routeId = String(route || '').trim();
                        if (!routeId) {
                            throw createUserFacingError('route is required');
                        }
                        var payload = args && typeof args === 'object' ? args : {};
                        runtime.navigationCommands.push({
                            route: routeId,
                            args: normalizeSerializableValue(payload, runtime, [])
                        });
                        return Promise.resolve();
                    },
                    showToast: function(message) {
                        return toolCall('toast', { message: String(message || '') });
                    },
                    reportError: function(error) {
                        console.error(error);
                    },
                    createWebViewController: function(key) {
                        return createWebViewController(key);
                    },
                    /** Opens a file picker through an owning-call structured host Promise. */
                    openFilePicker: function(options) {
                        return __operitInvokeHostAsync(__operitNativeComposeFilePickerCommand, [{
                            routeInstanceId: runtime.routeInstanceId || '',
                            executionContextKey: runtime.executionContextKey || '',
                            options: normalizeSerializableValue(options === undefined ? {} : options, runtime, [])
                        }]);
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
                    /** Queries package state through the required structured host Promise. */
                    isPackageImported: function(packageName) {
                        return __operitInvokeHostAsync(__operitNativeIsPackageImported,
                            [resolvePackageName(packageName, runtime)]);
                    },
                    /** Imports a package through the required structured host Promise. */
                    importPackage: function(packageName) {
                        return __operitInvokeHostAsync(__operitNativeImportPackage,
                            [resolvePackageName(packageName, runtime)]);
                    },
                    /** Removes a package through the required structured host Promise. */
                    removePackage: function(packageName) {
                        return __operitInvokeHostAsync(__operitNativeRemovePackage,
                            [resolvePackageName(packageName, runtime)]);
                    },
                    /** Activates a package through the required structured host Promise. */
                    usePackage: function(packageName) {
                        return __operitInvokeHostAsync(__operitNativeUsePackage,
                            [resolvePackageName(packageName, runtime)]);
                    },
                    /** Reads the structured package list from the required host binding. */
                    listImportedPackages: function() {
                        return __operitInvokeHostAsync(__operitNativeListImportedPackages, []);
                    },
                    /** Resolves a tool identity through the required host binding. */
                    resolveToolName: function(request) {
                        var req = request === undefined ? {} : request;
                        return __operitInvokeHostAsync(__operitNativeResolveToolName, [
                            String(req.packageName || runtime.packageName || ''),
                            String(req.subpackageId || ''), String(req.toolName || ''),
                            req.preferImported !== false
                        ]);
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

                /** Changes host inputs without replacing live refs, actions, or pending async work. */
                runtime.updateRuntimeOptions = function(next) {
                    if (!next || typeof next !== 'object') return;
                    if (next.__operit_update_inputs === true) {
                        var input = cloneObject(next.__operit_input_state);
                        Object.keys(input).forEach(function(key) { runtime.stateStore[key] = input[key]; });
                    }
                    if (next.theme !== undefined) themeSnapshot = readThemeSnapshot(next.theme);
                    if (next.__operit_call_runtime) runtime.callRuntime = next.__operit_call_runtime;
                };
                runtime.ctx = ctx;
                OperitComposeGeneration.attach(ctx);
                OperitComposeReactive.attach(ctx);
                runtime.composition = OperitComposeRetained.create(runtime);
                runtime.retainedDelivery = options.__operit_compose_retained_session === true;
                /// Transfers each navigation request to exactly one render response.
                runtime.takeNavigationCommands = function() {
                    return runtime.navigationCommands.splice(0);
                };
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
                runtime.setCallRuntime = function(callRuntime) {
                    runtime.callRuntime = callRuntime;
                };
                return runtime;
            }

            return {
                createContext: createContext
            };
        })();
    "#)
}

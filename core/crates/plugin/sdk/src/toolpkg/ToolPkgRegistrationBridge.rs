/// Builds the JavaScript bridge used by a ToolPkg runtime or registration call.
#[allow(non_snake_case)]
pub fn buildToolPkgRegistrationBridgeScript(restrictHostCapabilities: bool) -> String {
    let registrationOnly = if restrictHostCapabilities {
        "true"
    } else {
        "false"
    };
    r#"
    (function() {
        var root = typeof globalThis !== 'undefined'
            ? globalThis
            : (typeof window !== 'undefined' ? window : this);
        var registrationOnly = __OPERIT_TOOLPKG_REGISTRATION_ONLY__;
        var moduleRefFunctionCounter = 0;
        var capture = {
            marketOrigin: null,
            publicApis: [],
            toolboxUiModules: [],
            uiRoutes: [],
            chatComposerSlots: [],
            navigationEntries: [],
            desktopWidgets: [],
            appLifecycleHooks: [],
            messageProcessingPlugins: [],
            xmlRenderPlugins: [],
            inputMenuTogglePlugins: [],
            chatInputHooks: [],
            chatViewHooks: [],
            chatMessageHooks: [],
            chatLifecycleHooks: [],
            chatMessageMenuItems: [],
            chatRuntimeHooks: [],
            hostEventHooks: [],
            toolLifecycleHooks: [],
            promptInputHooks: [],
            promptHistoryHooks: [],
            promptEstimateHistoryHooks: [],
            systemPromptComposeHooks: [],
            toolPromptComposeHooks: [],
            promptFinalizeHooks: [],
            promptEstimateFinalizeHooks: [],
            summaryGenerateHooks: [],
            coreCommands: [],
            aiProviders: [],
            manifestExtensions: []
        };
        root.__operitToolPkgRegistrationCapture = capture;

        function installGlobal(name, value) {
            var key = String(name || '').trim();
            if (!key || value === undefined) {
                return;
            }
            try { globalThis[key] = value; } catch (_e) {}
            try { window[key] = value; } catch (_e2) {}
        }

        function copyObject(source, excludedKey) {
            var output = {};
            var keys = Object.keys(source || {});
            for (var i = 0; i < keys.length; i += 1) {
                var key = keys[i];
                if (key !== excludedKey) {
                    output[key] = source[key];
                }
            }
            return output;
        }

        /** Requires exact supported navigation surfaces instead of coercing casing or whitespace. */
        function validateNavigationSurface(surface) {
            switch (surface) {
                case 'toolbox':
                case 'main_sidebar_plugins':
                case 'app_bar':
                case 'chat_attachments':
                case 'chat_sidebar_tabs':
                case 'chat_input_menu':
                    return;
                default:
                    throw new Error('registerNavigationEntry.surface is unsupported: ' + String(surface));
            }
        }

        /** Requires one already registered package-owned Compose DSL route for an embedded surface. */
        function validateEmbeddedRoute(definition) {
            var owner = resolveCurrentToolPkgTarget();
            if (!owner) throw new Error('registerNavigationEntry.route owner is unavailable for ' + definition.surface);
            if (!definition.route.startsWith('toolpkg:' + owner + ':ui:')) {
                throw new Error('registerNavigationEntry.route must belong to this package for ' + definition.surface + ': ' + definition.route);
            }
            var registeredRoutes = capture.uiRoutes.concat(capture.toolboxUiModules);
            var matches = 0;
            for (var index = 0; index < registeredRoutes.length; index += 1) {
                var registered = JSON.parse(registeredRoutes[index]);
                var generatedRoute = typeof registered.id === 'string' ? 'toolpkg:' + owner + ':ui:' + registered.id.trim() : null;
                var explicitMatch = registered.route === definition.route;
                var generatedMatch = registered.route === undefined && generatedRoute === definition.route;
                if (!explicitMatch && !generatedMatch) continue;
                matches += 1;
                if (registered.runtime !== undefined && registered.runtime !== 'compose_dsl') {
                    throw new Error('registerNavigationEntry.route must use compose_dsl for ' + definition.surface + ': ' + definition.route);
                }
            }
            if (matches === 0) throw new Error('registerNavigationEntry.route is not registered for ' + definition.surface + ': ' + definition.route);
            if (matches !== 1) throw new Error('registerNavigationEntry.route is duplicated for ' + definition.surface + ': ' + definition.route);
        }

        /** Rejects non-JSON route input before serialization can silently drop or replace values. */
        function validateNavigationParams(value, ancestors) {
            if (value === null) return;
            var kind = typeof value;
            if (kind === 'string' || kind === 'boolean') return;
            if (kind === 'number') {
                if (!Number.isFinite(value)) throw new Error('registerNavigationEntry.params requires finite JSON numbers');
                return;
            }
            if (kind !== 'object') throw new Error('registerNavigationEntry.params must be JSON');
            if (ancestors.indexOf(value) !== -1) throw new Error('registerNavigationEntry.params must not be cyclic');
            var prototype = Object.getPrototypeOf(value);
            if (!Array.isArray(value) && prototype !== Object.prototype && prototype !== null) {
                throw new Error('registerNavigationEntry.params requires plain JSON objects');
            }
            if (Object.getOwnPropertySymbols(value).length !== 0) {
                throw new Error('registerNavigationEntry.params must not have symbol properties');
            }
            ancestors.push(value);
            if (Array.isArray(value)) {
                for (var index = 0; index < value.length; index += 1) {
                    validateNavigationParams(value[index], ancestors);
                }
            } else {
                var keys = Object.keys(value);
                for (var index = 0; index < keys.length; index += 1) {
                    validateNavigationParams(value[keys[index]], ancestors);
                }
            }
            ancestors.pop();
        }

        function getActiveExports() {
            return typeof root.__operitGetActiveModuleExports === 'function'
                ? root.__operitGetActiveModuleExports()
                : null;
        }

        function resolveExportedFunctionName(fn) {
            var exportsRef = getActiveExports();
            if (!exportsRef || typeof exportsRef !== 'object') {
                return '';
            }
            var keys = Object.keys(exportsRef);
            for (var i = 0; i < keys.length; i += 1) {
                if (exportsRef[keys[i]] === fn) {
                    return keys[i];
                }
            }
            return '';
        }

        function buildGeneratedFunctionName(definition) {
            moduleRefFunctionCounter += 1;
            var rawId = String((definition && definition.id) || 'hook');
            var safeId = rawId.replace(/[^a-zA-Z0-9_$]/g, '_') || 'hook';
            return '__operit_module_ref_hook_' + safeId + '_' + moduleRefFunctionCounter;
        }

        function activeModulePath() {
            var exportsRef = getActiveExports();
            if (!exportsRef || typeof exportsRef !== 'object') {
                return '';
            }
            return typeof exportsRef.__operit_toolpkg_module_path === 'string'
                ? exportsRef.__operit_toolpkg_module_path.trim().replace(/\\/g, '/')
                : '';
        }

        function dirname(path) {
            var normalized = String(path || '').replace(/\\/g, '/');
            var slash = normalized.lastIndexOf('/');
            return slash >= 0 ? normalized.slice(0, slash) : '';
        }

        function relativeRequirePath(fromModulePath, targetModulePath) {
            var fromDir = dirname(fromModulePath);
            var target = String(targetModulePath || '').replace(/\\/g, '/');
            if (!fromDir) {
                return './' + target;
            }
            var fromParts = fromDir.split('/').filter(Boolean);
            var targetParts = target.split('/').filter(Boolean);
            while (fromParts.length > 0 && targetParts.length > 0 && fromParts[0] === targetParts[0]) {
                fromParts.shift();
                targetParts.shift();
            }
            var up = fromParts.map(function() { return '..'; });
            var parts = up.concat(targetParts);
            var rel = parts.join('/');
            return rel.startsWith('.') ? rel : './' + rel;
        }

        function buildModuleRefFunctionSource(requirePath, exportName) {
            return 'function() {' +
                'var moduleRef = require(' + JSON.stringify(requirePath) + ');' +
                'var fn = moduleRef && moduleRef[' + JSON.stringify(exportName) + '];' +
                'if (typeof fn !== "function") {' +
                    'throw new Error("ToolPkg registered function export not found: ' + exportName.replace(/"/g, '\\"') + '");' +
                '}' +
                'return fn.apply(null, arguments);' +
            '}';
        }

        function resolveDurableFunctionRef(fn, definition, label) {
            var exportedName = resolveExportedFunctionName(fn);
            if (exportedName) {
                return {
                    name: exportedName,
                    source: ''
                };
            }
            var modulePath = typeof fn.__operit_toolpkg_module_path === 'string'
                ? fn.__operit_toolpkg_module_path.trim().replace(/\\/g, '/')
                : '';
            var exportName = typeof fn.__operit_toolpkg_export_name === 'string'
                ? fn.__operit_toolpkg_export_name.trim()
                : '';
            if (!modulePath || !exportName) {
                throw new Error(label + ' function must be exported from a toolpkg module');
            }
            var fromModulePath = activeModulePath();
            var functionName = buildGeneratedFunctionName(definition);
            return {
                name: functionName,
                source: buildModuleRefFunctionSource(relativeRequirePath(fromModulePath, modulePath), exportName)
            };
        }

        function normalizeFunctionField(definition, fieldName, label) {
            if (!definition || typeof definition !== 'object' || Array.isArray(definition)) {
                throw new Error(label + ' expects an object');
            }
            var normalized = copyObject(definition, fieldName);
            var fn = definition[fieldName];
            if (typeof fn !== 'function') {
                throw new Error(label + ' requires a function reference');
            }
            var functionRef = resolveDurableFunctionRef(fn, definition, label);
            normalized[fieldName] = functionRef.name;
            if (functionRef.source) {
                normalized.function_source = functionRef.source;
            }
            return normalized;
        }

        function normalizeNestedFunctionField(definition, fieldName, label) {
            if (!definition || typeof definition !== 'object' || Array.isArray(definition)) {
                throw new Error(label + ' expects an object');
            }
            var fieldValue = definition[fieldName];
            if (!fieldValue || typeof fieldValue !== 'object' || Array.isArray(fieldValue)) {
                throw new Error(label + ' requires an object field: ' + fieldName);
            }
            var fn = fieldValue.function;
            if (typeof fn !== 'function') {
                throw new Error(label + '.' + fieldName + '.function must be a function reference');
            }
            var functionRef = resolveDurableFunctionRef(fn, {
                id: String((definition && definition.id) || 'provider') + '_' + fieldName
            }, label + '.' + fieldName);
            var normalizedField = copyObject(fieldValue, 'function');
            normalizedField.function = functionRef.name;
            if (functionRef.source) {
                normalizedField.function_source = functionRef.source;
            }
            return normalizedField;
        }

        function normalizeAiProviderDefinition(definition, label) {
            var normalized = copyObject(definition, '');
            [
                'listModels',
                'sendMessage',
                'testConnection',
                'calculateInputTokens'
            ].forEach(function(fieldName) {
                normalized[fieldName] = normalizeNestedFunctionField(definition, fieldName, label);
            });
            return normalized;
        }

        /** Preserves the exact exported entry when a UI route is registered with a function. */
        function normalizeScreenField(definition, label) {
            if (!definition || typeof definition !== 'object' || Array.isArray(definition)) {
                throw new Error(label + ' expects an object');
            }
            var normalized = copyObject(definition, 'screen');
            var screen = definition.screen;
            var path = '';
            var exportName = null;
            if (typeof screen === 'string') {
                path = screen.trim().replace(/\\/g, '/');
            } else if (typeof screen === 'function' && typeof screen.__operit_toolpkg_module_path === 'string') {
                path = screen.__operit_toolpkg_module_path.trim().replace(/\\/g, '/');
                exportName = screen.__operit_toolpkg_export_name;
            } else if (
                screen &&
                typeof screen === 'object' &&
                typeof screen.default === 'function' &&
                typeof screen.default.__operit_toolpkg_module_path === 'string'
            ) {
                path = screen.default.__operit_toolpkg_module_path.trim().replace(/\\/g, '/');
                exportName = screen.default.__operit_toolpkg_export_name;
            }
            if (!path) {
                throw new Error(label + ' requires a serializable screen reference');
            }
            if (typeof screen !== 'string' && (typeof exportName !== 'string' || !exportName.trim())) {
                throw new Error(label + ' screen function must have a named module export');
            }
            normalized.screenExport = exportName;
            normalized.screen = path;
            return normalized;
        }

        function normalizeDialogScreenField(definition, label) {
            if (!definition || typeof definition !== 'object' || Array.isArray(definition)) {
                throw new Error(label + ' expects an object');
            }
            var normalized = copyObject(definition, 'screen');
            var screen = definition.screen;
            var path = '';
            if (typeof screen === 'string') {
                path = screen.trim().replace(/\\/g, '/');
            } else if (typeof screen === 'function' && typeof screen.__operit_toolpkg_module_path === 'string') {
                path = screen.__operit_toolpkg_module_path.trim().replace(/\\/g, '/');
            } else if (
                screen &&
                typeof screen === 'object' &&
                typeof screen.default === 'function' &&
                typeof screen.default.__operit_toolpkg_module_path === 'string'
            ) {
                path = screen.default.__operit_toolpkg_module_path.trim().replace(/\\/g, '/');
            }
            if (!path) {
                throw new Error(label + ' requires a serializable screen reference');
            }
            normalized.screen = path;
            return normalized;
        }

        function normalizeChatMessageMenuItemDefinition(definition, label) {
            var normalized = normalizeFunctionField(definition, 'function', label);
            if (definition.dialog !== undefined && definition.dialog !== null) {
                if (typeof definition.dialog !== 'object' || Array.isArray(definition.dialog)) {
                    throw new Error(label + '.dialog expects an object');
                }
                normalized.dialog = normalizeDialogScreenField(definition.dialog, label + '.dialog');
            }
            return normalized;
        }

        function normalizeSpec(spec) {
            if (typeof spec === 'string') {
                var parsed = JSON.parse(spec);
                if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
                    throw new Error('toolpkg registration payload must be a JSON object');
                }
                return JSON.stringify(parsed);
            }
            if (!spec || typeof spec !== 'object' || Array.isArray(spec)) {
                throw new Error('toolpkg registration payload must be a JSON object');
            }
            return JSON.stringify(spec);
        }

        function captureMarketOrigin(encoded, key) {
            // Marketplace publishing appends this marker to a main module. Runtime hooks can
            // evaluate that module repeatedly after registration, so marker calls there must
            // not mutate registration capture state or interrupt the hook.
            if (!registrationOnly) {
                return;
            }
            if (!Array.isArray(encoded)) {
                throw new Error('ToolPkg marketplace origin payload must be an array');
            }
            var xorKey = Number(key);
            if (!Number.isInteger(xorKey) || xorKey < 0 || xorKey > 255) {
                throw new Error('ToolPkg marketplace origin key is invalid');
            }
            var json = '';
            for (var index = 0; index < encoded.length; index += 1) {
                var value = Number(encoded[index]);
                if (!Number.isInteger(value) || value < 0 || value > 255) {
                    throw new Error('ToolPkg marketplace origin payload byte is invalid');
                }
                json += String.fromCharCode(value ^ xorKey);
            }
            var parsed = JSON.parse(json);
            if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
                throw new Error('ToolPkg marketplace origin payload must decode to an object');
            }
            capture.marketOrigin = parsed;
        }

        function append(bucket) {
            return function(spec) {
                capture[bucket].push(normalizeSpec(spec));
            };
        }

        function registerScreen(bucket, label) {
            return function(definition) {
                capture[bucket].push(normalizeSpec(normalizeScreenField(definition, label)));
            };
        }

        function registerFunction(bucket, label) {
            return function(definition) {
                capture[bucket].push(normalizeSpec(normalizeFunctionField(definition, 'function', label)));
            };
        }

        function resolveCurrentToolPkgTarget() {
            var callId = String(root.__operitCurrentCallId || '').trim();
            var callState =
                callId && typeof root.__operitGetCallState === 'function'
                    ? root.__operitGetCallState(callId)
                    : null;
            var params =
                callState && callState.params && typeof callState.params === 'object'
                    ? callState.params
                    : null;
            if (!params) {
                return '';
            }
            var candidates = [
                params.__operit_ui_package_name,
                params.toolPkgId,
                params.containerPackageName,
                params.__operit_toolpkg_subpackage_id,
                params.__operit_package_name
            ];
            for (var i = 0; i < candidates.length; i += 1) {
                var value = String(candidates[i] || '').trim();
                if (value) {
                    return value;
                }
            }
            return '';
        }

        /** Materializes a resource through the required structured host Promise. */

        function readToolPkgResource(key, outputFileName, internal) {
            if (registrationOnly) {
                throw new Error('ToolPkg.readResource is unavailable during ToolPkg registration');
            }
            var resourceKey = String(key || '').trim();
            if (!resourceKey) {
                return Promise.reject(new Error('resource key is required'));
            }
            var target = resolveCurrentToolPkgTarget();
            if (!target) {
                return Promise.reject(new Error('package/toolpkg runtime target is empty'));
            }
            return __operitInvokeHostAsync(__operitNativeReadToolPkgResource, [
                target, resourceKey,
                outputFileName == null ? '' : String(outputFileName).trim(),
                internal === true
            ]);
        }

        /** Materializes a targeted package resource without parsing result text. */

        function readToolPkgResourceFromPackage(packageNameOrSubpackageId, key, outputFileName, internal) {
            if (registrationOnly) {
                throw new Error('ToolPkg.readResourceFromPackage is unavailable during ToolPkg registration');
            }
            var target = String(packageNameOrSubpackageId || '').trim();
            var resourceKey = String(key || '').trim();
            if (!target) {
                return Promise.reject(new Error('ToolPkg resource target is required'));
            }
            if (!resourceKey) {
                return Promise.reject(new Error('resource key is required'));
            }
            return __operitInvokeHostAsync(__operitNativeReadToolPkgResource, [
                target, resourceKey,
                outputFileName == null ? '' : String(outputFileName).trim(),
                internal === true
            ]);
        }

        /** Reads and validates the direct absolute VFS configuration path. */

        function getToolPkgConfigDir(pluginId) {
            var explicitId = String(pluginId || '').trim();
            var owner = resolveCurrentToolPkgTarget();
            var target = explicitId || owner;
            if (!target) {
                throw new Error('package/toolpkg runtime target is empty');
            }
            var path = __operitNativeGetScopedPluginConfigDir(owner, target);
            if (typeof path === 'string' && path.startsWith('/')) {
                return path;
            }
            throw new Error('Plugin configuration directory must be an absolute VFS path');
        }

        /** Resolves the executing container's stable local data through its Host. */
        function getToolPkgLocalDataDir() {
            var owner = resolveCurrentToolPkgTarget();
            if (!owner) {
                throw new Error('package/toolpkg runtime target is empty');
            }
            if (typeof NativeInterface === 'undefined' || !NativeInterface ||
                typeof NativeInterface.getPluginLocalDataDir !== 'function') {
                throw new Error('NativeInterface.getPluginLocalDataDir is unavailable');
            }
            var path = NativeInterface.getPluginLocalDataDir(owner);
            if (typeof path !== 'string' || !path.startsWith('/')) {
                throw new Error('Plugin local data directory must be an absolute VFS path');
            }
            return path;
        }

        /** Resolves persistent shared data without changing the plugin's installation scope. */
        function getToolPkgSpaceDataDir() {
            var owner = resolveCurrentToolPkgTarget();
            if (!owner) { throw new Error('package/toolpkg runtime target is empty'); }
            if (typeof NativeInterface === 'undefined' || !NativeInterface ||
                typeof NativeInterface.getPluginSpaceDataDir !== 'function') {
                throw new Error('NativeInterface.getPluginSpaceDataDir is unavailable');
            }
            var path = NativeInterface.getPluginSpaceDataDir(owner);
            if (typeof path !== 'string' || !path.startsWith('/')) {
                throw new Error('Plugin shared data directory must be an absolute VFS path');
            }
            return path;
        }

        function normalizeToolPkgWasmValueType(valueType) {
            var normalizedType = String(valueType || '').trim().toLowerCase();
            if (
                normalizedType !== 'i32' &&
                normalizedType !== 'i64' &&
                normalizedType !== 'f32' &&
                normalizedType !== 'f64'
            ) {
                throw new Error('ToolPkg.wasm arg type is invalid: ' + normalizedType);
            }
            return normalizedType;
        }

        function normalizeToolPkgWasmArgs(args) {
            if (args == null) {
                return [];
            }
            if (!Array.isArray(args)) {
                throw new Error('ToolPkg.wasm args must be an array');
            }
            var normalizedArgs = [];
            for (var i = 0; i < args.length; i += 1) {
                var arg = args[i];
                if (!arg || typeof arg !== 'object' || Array.isArray(arg)) {
                    throw new Error('ToolPkg.wasm arg ' + i + ' must be an object');
                }
                var valueType = normalizeToolPkgWasmValueType(arg.type);
                if (!Object.prototype.hasOwnProperty.call(arg, 'value')) {
                    throw new Error('ToolPkg.wasm arg ' + i + ' value is required');
                }
                var value = arg.value;
                if (valueType === 'i32' && typeof value !== 'number') {
                    throw new Error('ToolPkg.wasm arg ' + i + ' i32 value must be a number');
                }
                if (valueType === 'i64' && typeof value !== 'number' && typeof value !== 'string') {
                    throw new Error('ToolPkg.wasm arg ' + i + ' i64 value must be a number or string');
                }
                if (
                    (valueType === 'f32' || valueType === 'f64') &&
                    typeof value !== 'number' &&
                    typeof value !== 'string'
                ) {
                    throw new Error('ToolPkg.wasm arg ' + i + ' float value must be a number or string');
                }
                normalizedArgs.push({ type: valueType, value: value });
            }
            return normalizedArgs;
        }

        /** Calls a WASM export using structured scalar arguments and an owned host Promise. */

        function callToolPkgWasm(moduleId, exportName, args) {
            if (registrationOnly) {
                throw new Error('ToolPkg.wasm.call is unavailable during ToolPkg registration');
            }
            var normalizedModuleId = String(moduleId || '').trim();
            if (!normalizedModuleId) {
                return Promise.reject(new Error('ToolPkg.wasm module id is required'));
            }
            var normalizedExportName = String(exportName || '').trim();
            if (!normalizedExportName) {
                return Promise.reject(new Error('ToolPkg.wasm export name is required'));
            }
            var normalizedArgs;
            try {
                normalizedArgs = normalizeToolPkgWasmArgs(args);
            } catch (error) {
                return Promise.reject(error);
            }
            var target = resolveCurrentToolPkgTarget();
            if (!target) {
                return Promise.reject(new Error('package/toolpkg runtime target is empty'));
            }
            return __operitInvokeHostAsync(__operitNativeCallToolPkgWasm,
                [target, normalizedModuleId, normalizedExportName, normalizedArgs]);
        }

        function requireToolPkgApiRuntime() {
            var runtime = root.__operitToolPkgApi;
            if (!runtime || typeof runtime !== 'object') {
                throw new Error('__operitToolPkgApi is unavailable');
            }
            if (typeof runtime.namespace !== 'function' || typeof runtime.method !== 'function') {
                throw new Error('__operitToolPkgApi is invalid');
            }
            return runtime;
        }

        var toolPkgApi = requireToolPkgApiRuntime();
        var api = toolPkgApi.namespace('ToolPkg', {
            _m: captureMarketOrigin,
            // Captures only explicitly published methods using durable exported function references.
            registerApi: function(definition) {
                if (!definition || typeof definition.name !== 'string' || !definition.name.trim()) {
                    throw new Error('registerApi requires a non-empty name');
                }
                var normalized = normalizeFunctionField(definition, 'function', 'registerApi');
                normalized.id = definition.name.trim();
                delete normalized.name;
                capture.publicApis.push(normalizeSpec(normalized));
            },
            callDependency: toolPkgApi.callDependency,
            registerToolboxUiModule: registerScreen('toolboxUiModules', 'registerToolPkgToolboxUiModule'),
            registerUiRoute: registerScreen('uiRoutes', 'registerToolPkgUiRoute'),
            registerChatComposerSlot: registerScreen('chatComposerSlots', 'registerToolPkgChatComposerSlot'),
            /** Captures validated navigation metadata, opaque route input, and durable optional callbacks. */
            registerNavigationEntry: function(definition) {
                if (!definition || typeof definition !== 'object' || Array.isArray(definition)) {
                    throw new Error('registerNavigationEntry requires an object');
                }
                validateNavigationSurface(definition.surface);
                if (definition.surface === 'chat_attachments' || definition.surface === 'chat_sidebar_tabs' || definition.surface === 'chat_input_menu') {
                    if (definition.action !== undefined && definition.action !== null) {
                        throw new Error('registerNavigationEntry.action is unsupported for ' + definition.surface);
                    }
                    if (typeof definition.route !== 'string' || !definition.route.trim()) {
                        throw new Error('registerNavigationEntry.route is required for ' + definition.surface);
                    }
                }
                if (definition.surface === 'chat_sidebar_tabs' || definition.surface === 'chat_input_menu') validateEmbeddedRoute(definition);
                if (Object.prototype.hasOwnProperty.call(definition, 'params')) {
                    validateNavigationParams(definition.params, []);
                }
                var normalized = copyObject(definition, '');
                if (definition && typeof definition.action === 'function') {
                    var ref = resolveDurableFunctionRef(
                        definition.action, definition, 'registerToolPkgNavigationEntry'
                    );
                    normalized.action = { function: ref.name };
                    if (ref.source) {
                        normalized.action.functionSource = ref.source;
                    }
                }
                capture.navigationEntries.push(normalizeSpec(normalized));
            },
            registerDesktopWidget: append('desktopWidgets'),
            registerAppLifecycleHook: registerFunction('appLifecycleHooks', 'registerAppLifecycleHook'),
            registerMessageProcessingPlugin: registerFunction('messageProcessingPlugins', 'registerMessageProcessingPlugin'),
            registerXmlRenderPlugin: registerFunction('xmlRenderPlugins', 'registerXmlRenderPlugin'),
            registerInputMenuTogglePlugin: registerFunction('inputMenuTogglePlugins', 'registerInputMenuTogglePlugin'),
            registerChatInputHook: registerFunction('chatInputHooks', 'registerChatInputHook'),
            registerChatViewHook: registerFunction('chatViewHooks', 'registerChatViewHook'),
            registerChatMessageHook: registerFunction('chatMessageHooks', 'registerChatMessageHook'),
            /** Captures exactly one exported creation handler for this authenticated package owner. */
            registerChatLifecycleHook: function(definition) {
                var owner = resolveCurrentToolPkgTarget();
                if (!owner) throw new Error('registerChatLifecycleHook owner is unavailable');
                if (!definition || typeof definition.id !== 'string' || !definition.id.trim()) {
                    throw new Error('registerChatLifecycleHook.id is required');
                }
                if (capture.chatLifecycleHooks.length !== 0) {
                    throw new Error('Duplicate chat lifecycle hook owner: ' + owner);
                }
                var normalized = normalizeFunctionField(definition, 'function', 'registerChatLifecycleHook');
                capture.chatLifecycleHooks.push(normalizeSpec(normalized));
            },
            registerChatMessageMenuItem: toolPkgApi.method()
                .between('1.0.1', '2.0.0')
                .since('2.0.0')
                // Adapts both API families to the shared registration payload.
                .implement(function(definition) {
                    capture.chatMessageMenuItems.push(
                        normalizeSpec(
                            normalizeChatMessageMenuItemDefinition(
                                definition,
                                'registerChatMessageMenuItem'
                            )
                        )
                    );
                }),
            registerChatRuntimeHook: toolPkgApi.method()
                .between('1.0.1', '2.0.0')
                .since('2.0.0')
                .implement(registerFunction('chatRuntimeHooks', 'registerChatRuntimeHook')),
            registerHostEventHook: registerFunction('hostEventHooks', 'registerHostEventHook'),
            registerToolLifecycleHook: registerFunction('toolLifecycleHooks', 'registerToolLifecycleHook'),
            registerPromptInputHook: registerFunction('promptInputHooks', 'registerPromptInputHook'),
            registerPromptHistoryHook: registerFunction('promptHistoryHooks', 'registerPromptHistoryHook'),
            registerPromptEstimateHistoryHook: registerFunction('promptEstimateHistoryHooks', 'registerPromptEstimateHistoryHook'),
            registerSystemPromptComposeHook: registerFunction('systemPromptComposeHooks', 'registerSystemPromptComposeHook'),
            registerToolPromptComposeHook: registerFunction('toolPromptComposeHooks', 'registerToolPromptComposeHook'),
            registerPromptFinalizeHook: registerFunction('promptFinalizeHooks', 'registerPromptFinalizeHook'),
            registerPromptEstimateFinalizeHook: registerFunction('promptEstimateFinalizeHooks', 'registerPromptEstimateFinalizeHook'),
            registerSummaryGenerateHook: registerFunction('summaryGenerateHooks', 'registerSummaryGenerateHook'),
            registerCoreCommand: registerFunction('coreCommands', 'registerCoreCommand'),
            readResource: readToolPkgResource,
            readResourceFromPackage: readToolPkgResourceFromPackage,
            getConfigDir: getToolPkgConfigDir,
            getLocalDataDir: getToolPkgLocalDataDir,
            getSpaceDataDir: getToolPkgSpaceDataDir,
            wasm: {
                call: callToolPkgWasm
            },
            registerAiProvider: function(definition) {
                capture.aiProviders.push(normalizeSpec(normalizeAiProviderDefinition(definition, 'registerAiProvider')));
            },
            registerManifestExtension: registerFunction('manifestExtensions', 'registerManifestExtension')
        });

        root.registerToolPkgToolboxUiModule = api.registerToolboxUiModule;
        root.registerToolPkgUiRoute = api.registerUiRoute;
        root.registerToolPkgChatComposerSlot = api.registerChatComposerSlot;
        root.registerToolPkgNavigationEntry = api.registerNavigationEntry;
        root.registerToolPkgDesktopWidget = api.registerDesktopWidget;
        root.registerToolPkgAppLifecycleHook = api.registerAppLifecycleHook;
        root.registerToolPkgMessageProcessingPlugin = api.registerMessageProcessingPlugin;
        root.registerToolPkgXmlRenderPlugin = api.registerXmlRenderPlugin;
        root.registerToolPkgInputMenuTogglePlugin = api.registerInputMenuTogglePlugin;
        root.registerToolPkgChatInputHook = api.registerChatInputHook;
        root.registerToolPkgChatViewHook = api.registerChatViewHook;
        root.registerToolPkgChatMessageHook = api.registerChatMessageHook;
        root.registerToolPkgChatLifecycleHook = api.registerChatLifecycleHook;
        root.registerToolPkgChatMessageMenuItem = api.registerChatMessageMenuItem;
        root.registerToolPkgChatRuntimeHook = api.registerChatRuntimeHook;
        root.registerToolPkgHostEventHook = api.registerHostEventHook;
        root.registerToolPkgToolLifecycleHook = api.registerToolLifecycleHook;
        root.registerToolPkgPromptInputHook = api.registerPromptInputHook;
        root.registerToolPkgPromptHistoryHook = api.registerPromptHistoryHook;
        root.registerToolPkgPromptEstimateHistoryHook = api.registerPromptEstimateHistoryHook;
        root.registerToolPkgSystemPromptComposeHook = api.registerSystemPromptComposeHook;
        root.registerToolPkgToolPromptComposeHook = api.registerToolPromptComposeHook;
        root.registerToolPkgPromptFinalizeHook = api.registerPromptFinalizeHook;
        root.registerToolPkgPromptEstimateFinalizeHook = api.registerPromptEstimateFinalizeHook;
        root.registerToolPkgSummaryGenerateHook = api.registerSummaryGenerateHook;
        root.registerToolPkgCoreCommand = api.registerCoreCommand;
        root.registerToolPkgAiProvider = api.registerAiProvider;
        root.registerToolPkgManifestExtension = api.registerManifestExtension;

        root.registerAppLifecycleHook = api.registerAppLifecycleHook;
        root.registerMessageProcessingPlugin = api.registerMessageProcessingPlugin;
        root.registerXmlRenderPlugin = api.registerXmlRenderPlugin;
        root.registerInputMenuTogglePlugin = api.registerInputMenuTogglePlugin;
        root.registerChatInputHook = api.registerChatInputHook;
        root.registerChatViewHook = api.registerChatViewHook;
        root.registerChatMessageHook = api.registerChatMessageHook;
        root.registerChatLifecycleHook = api.registerChatLifecycleHook;
        root.registerChatMessageMenuItem = api.registerChatMessageMenuItem;
        root.registerChatRuntimeHook = api.registerChatRuntimeHook;
        root.registerHostEventHook = api.registerHostEventHook;
        root.registerToolLifecycleHook = api.registerToolLifecycleHook;
        root.registerPromptInputHook = api.registerPromptInputHook;
        root.registerPromptHistoryHook = api.registerPromptHistoryHook;
        root.registerPromptEstimateHistoryHook = api.registerPromptEstimateHistoryHook;
        root.registerSystemPromptComposeHook = api.registerSystemPromptComposeHook;
        root.registerToolPromptComposeHook = api.registerToolPromptComposeHook;
        root.registerPromptFinalizeHook = api.registerPromptFinalizeHook;
        root.registerPromptEstimateFinalizeHook = api.registerPromptEstimateFinalizeHook;
        root.registerSummaryGenerateHook = api.registerSummaryGenerateHook;
        root.registerCoreCommand = api.registerCoreCommand;

        installGlobal('ToolPkg', api);
    })();
    "#
    .replace("__OPERIT_TOOLPKG_REGISTRATION_ONLY__", registrationOnly)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::buildToolPkgRegistrationBridgeScript;
    use crate::toolpkg::ToolPkgApiRuntimeScript::buildToolPkgApiRuntimeScript;
    use rquickjs::{Context, Runtime};

    /// Verifies runtime marketplace markers can be evaluated repeatedly without capture side effects.
    #[test]
    fn runtime_marketplace_marker_is_repeatable_noop() {
        let runtime = Runtime::new().expect("QuickJS runtime should start");
        let context = Context::full(&runtime).expect("QuickJS context should start");

        context.with(|context| {
            context
                .eval::<(), _>(
                    r#"
                    globalThis.__operitExpose = function(name, value) {
                        globalThis[name] = value;
                    };
                    "#,
                )
                .expect("runtime expose should evaluate");
            context
                .eval::<(), _>(buildToolPkgApiRuntimeScript())
                .expect("toolpkg api runtime should evaluate");
            context
                .eval::<(), _>(buildToolPkgRegistrationBridgeScript(false))
                .expect("runtime bridge should evaluate");
            context
                .eval::<(), _>("ToolPkg._m([], 0); ToolPkg._m([], 0);")
                .expect("runtime marker calls should not throw");
            let capturedOrigin = context
                .eval::<String, _>(
                    "String(globalThis.__operitToolPkgRegistrationCapture.marketOrigin)",
                )
                .expect("runtime capture should be readable");

            assert_eq!(capturedOrigin, "null");
        });
    }

    /// Verifies Core command registrations retain their metadata and durable callback reference.
    #[test]
    fn captures_core_command_registration() {
        let runtime = Runtime::new().expect("QuickJS runtime should start");
        let context = Context::full(&runtime).expect("QuickJS context should start");

        context.with(|context| {
            context
                .eval::<(), _>(
                    r#"
                    globalThis.__operitExpose = function(name, value) {
                        globalThis[name] = value;
                    };
                    "#,
                )
                .expect("runtime expose should evaluate");
            context
                .eval::<(), _>(buildToolPkgApiRuntimeScript())
                .expect("toolpkg api runtime should evaluate");
            context
                .eval::<(), _>(buildToolPkgRegistrationBridgeScript(false))
                .expect("runtime bridge should evaluate");
            context
                .eval::<(), _>(
                    r#"
                    function runHello(event) {
                        return { stdout: event.eventPayload.commandName };
                    }
                    globalThis.__operitGetActiveModuleExports = function() {
                        return { runHello: runHello };
                    };
                    ToolPkg.registerCoreCommand({
                        id: 'hello_command',
                        name: 'hello',
                        title: { en: 'Hello' },
                        description: { en: 'Greets the user' },
                        usage: '/hello <name>',
                        function: runHello
                    });
                    "#,
                )
                .expect("Core command registration should evaluate");
            let command_name = context
                .eval::<String, _>(
                    "JSON.parse(globalThis.__operitToolPkgRegistrationCapture.coreCommands[0]).name",
                )
                .expect("captured command should be readable");
            let function_name = context
                .eval::<String, _>(
                    "JSON.parse(globalThis.__operitToolPkgRegistrationCapture.coreCommands[0]).function",
                )
                .expect("captured function should be readable");

            assert_eq!(command_name, "hello");
            assert_eq!(function_name, "runHello");
        });
    }
}

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgMainRegistrationCapture {
    #[serde(rename = "toolboxUiModules", default)]
    pub toolboxUiModules: Vec<String>,
    #[serde(rename = "uiRoutes", default)]
    pub uiRoutes: Vec<String>,
    #[serde(rename = "navigationEntries", default)]
    pub navigationEntries: Vec<String>,
    #[serde(rename = "desktopWidgets", default)]
    pub desktopWidgets: Vec<String>,
    #[serde(rename = "appLifecycleHooks", default)]
    pub appLifecycleHooks: Vec<String>,
    #[serde(rename = "messageProcessingPlugins", default)]
    pub messageProcessingPlugins: Vec<String>,
    #[serde(rename = "xmlRenderPlugins", default)]
    pub xmlRenderPlugins: Vec<String>,
    #[serde(rename = "inputMenuTogglePlugins", default)]
    pub inputMenuTogglePlugins: Vec<String>,
    #[serde(rename = "chatInputHooks", default)]
    pub chatInputHooks: Vec<String>,
    #[serde(rename = "chatViewHooks", default)]
    pub chatViewHooks: Vec<String>,
    #[serde(rename = "toolLifecycleHooks", default)]
    pub toolLifecycleHooks: Vec<String>,
    #[serde(rename = "promptInputHooks", default)]
    pub promptInputHooks: Vec<String>,
    #[serde(rename = "promptHistoryHooks", default)]
    pub promptHistoryHooks: Vec<String>,
    #[serde(rename = "promptEstimateHistoryHooks", default)]
    pub promptEstimateHistoryHooks: Vec<String>,
    #[serde(rename = "systemPromptComposeHooks", default)]
    pub systemPromptComposeHooks: Vec<String>,
    #[serde(rename = "toolPromptComposeHooks", default)]
    pub toolPromptComposeHooks: Vec<String>,
    #[serde(rename = "promptFinalizeHooks", default)]
    pub promptFinalizeHooks: Vec<String>,
    #[serde(rename = "promptEstimateFinalizeHooks", default)]
    pub promptEstimateFinalizeHooks: Vec<String>,
    #[serde(rename = "summaryGenerateHooks", default)]
    pub summaryGenerateHooks: Vec<String>,
    #[serde(rename = "aiProviders", default)]
    pub aiProviders: Vec<String>,
}

#[allow(non_snake_case)]
pub fn buildToolPkgRegistrationBridgeScript() -> String {
    r#"
    (function() {
        var root = typeof globalThis !== 'undefined'
            ? globalThis
            : (typeof window !== 'undefined' ? window : this);
        var moduleRefFunctionCounter = 0;
        var capture = {
            toolboxUiModules: [],
            uiRoutes: [],
            navigationEntries: [],
            desktopWidgets: [],
            appLifecycleHooks: [],
            messageProcessingPlugins: [],
            xmlRenderPlugins: [],
            inputMenuTogglePlugins: [],
            chatInputHooks: [],
            chatViewHooks: [],
            toolLifecycleHooks: [],
            promptInputHooks: [],
            promptHistoryHooks: [],
            promptEstimateHistoryHooks: [],
            systemPromptComposeHooks: [],
            toolPromptComposeHooks: [],
            promptFinalizeHooks: [],
            promptEstimateFinalizeHooks: [],
            summaryGenerateHooks: [],
            aiProviders: []
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

        function normalizeScreenField(definition, label) {
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

        function readToolPkgResource(key, outputFileName, internal) {
            var resourceKey = String(key || '').trim();
            if (!resourceKey) {
                return Promise.reject(new Error('resource key is required'));
            }
            var target = resolveCurrentToolPkgTarget();
            if (!target) {
                return Promise.reject(new Error('package/toolpkg runtime target is empty'));
            }
            if (
                typeof NativeInterface === 'undefined' ||
                !NativeInterface ||
                typeof NativeInterface.readToolPkgResource !== 'function'
            ) {
                return Promise.reject(new Error('NativeInterface.readToolPkgResource is unavailable'));
            }
            var path = NativeInterface.readToolPkgResource(
                target,
                resourceKey,
                outputFileName == null ? '' : String(outputFileName).trim(),
                internal === true ? 'true' : ''
            );
            if (typeof path === 'string' && path.trim()) {
                return Promise.resolve(path);
            }
            return Promise.reject(new Error('resource not found: ' + resourceKey));
        }

        function getToolPkgConfigDir(pluginId) {
            var explicitId = String(pluginId || '').trim();
            var target = explicitId || resolveCurrentToolPkgTarget();
            if (!target) {
                throw new Error('package/toolpkg runtime target is empty');
            }
            if (
                typeof NativeInterface === 'undefined' ||
                !NativeInterface ||
                typeof NativeInterface.getPluginConfigDir !== 'function'
            ) {
                throw new Error('NativeInterface.getPluginConfigDir is unavailable');
            }
            var path = NativeInterface.getPluginConfigDir(target);
            if (typeof path === 'string' && path.trim()) {
                return path;
            }
            throw new Error('plugin config dir is unavailable for ' + target);
        }

        var api = {
            registerToolboxUiModule: registerScreen('toolboxUiModules', 'registerToolPkgToolboxUiModule'),
            registerUiRoute: registerScreen('uiRoutes', 'registerToolPkgUiRoute'),
            registerNavigationEntry: function(definition) {
                var normalized = definition && typeof definition.action === 'function'
                    ? normalizeFunctionField(definition, 'action', 'registerToolPkgNavigationEntry')
                    : copyObject(definition, '');
                capture.navigationEntries.push(normalizeSpec(normalized));
            },
            registerDesktopWidget: append('desktopWidgets'),
            registerAppLifecycleHook: registerFunction('appLifecycleHooks', 'registerAppLifecycleHook'),
            registerMessageProcessingPlugin: registerFunction('messageProcessingPlugins', 'registerMessageProcessingPlugin'),
            registerXmlRenderPlugin: registerFunction('xmlRenderPlugins', 'registerXmlRenderPlugin'),
            registerInputMenuTogglePlugin: registerFunction('inputMenuTogglePlugins', 'registerInputMenuTogglePlugin'),
            registerChatInputHook: registerFunction('chatInputHooks', 'registerChatInputHook'),
            registerChatViewHook: registerFunction('chatViewHooks', 'registerChatViewHook'),
            registerToolLifecycleHook: registerFunction('toolLifecycleHooks', 'registerToolLifecycleHook'),
            registerPromptInputHook: registerFunction('promptInputHooks', 'registerPromptInputHook'),
            registerPromptHistoryHook: registerFunction('promptHistoryHooks', 'registerPromptHistoryHook'),
            registerPromptEstimateHistoryHook: registerFunction('promptEstimateHistoryHooks', 'registerPromptEstimateHistoryHook'),
            registerSystemPromptComposeHook: registerFunction('systemPromptComposeHooks', 'registerSystemPromptComposeHook'),
            registerToolPromptComposeHook: registerFunction('toolPromptComposeHooks', 'registerToolPromptComposeHook'),
            registerPromptFinalizeHook: registerFunction('promptFinalizeHooks', 'registerPromptFinalizeHook'),
            registerPromptEstimateFinalizeHook: registerFunction('promptEstimateFinalizeHooks', 'registerPromptEstimateFinalizeHook'),
            registerSummaryGenerateHook: registerFunction('summaryGenerateHooks', 'registerSummaryGenerateHook'),
            readResource: readToolPkgResource,
            getConfigDir: getToolPkgConfigDir,
            registerAiProvider: function(definition) {
                capture.aiProviders.push(normalizeSpec(normalizeAiProviderDefinition(definition, 'registerAiProvider')));
            }
        };

        root.registerToolPkgToolboxUiModule = api.registerToolboxUiModule;
        root.registerToolPkgUiRoute = api.registerUiRoute;
        root.registerToolPkgNavigationEntry = api.registerNavigationEntry;
        root.registerToolPkgDesktopWidget = api.registerDesktopWidget;
        root.registerToolPkgAppLifecycleHook = api.registerAppLifecycleHook;
        root.registerToolPkgMessageProcessingPlugin = api.registerMessageProcessingPlugin;
        root.registerToolPkgXmlRenderPlugin = api.registerXmlRenderPlugin;
        root.registerToolPkgInputMenuTogglePlugin = api.registerInputMenuTogglePlugin;
        root.registerToolPkgChatInputHook = api.registerChatInputHook;
        root.registerToolPkgChatViewHook = api.registerChatViewHook;
        root.registerToolPkgToolLifecycleHook = api.registerToolLifecycleHook;
        root.registerToolPkgPromptInputHook = api.registerPromptInputHook;
        root.registerToolPkgPromptHistoryHook = api.registerPromptHistoryHook;
        root.registerToolPkgPromptEstimateHistoryHook = api.registerPromptEstimateHistoryHook;
        root.registerToolPkgSystemPromptComposeHook = api.registerSystemPromptComposeHook;
        root.registerToolPkgToolPromptComposeHook = api.registerToolPromptComposeHook;
        root.registerToolPkgPromptFinalizeHook = api.registerPromptFinalizeHook;
        root.registerToolPkgPromptEstimateFinalizeHook = api.registerPromptEstimateFinalizeHook;
        root.registerToolPkgSummaryGenerateHook = api.registerSummaryGenerateHook;
        root.registerToolPkgAiProvider = api.registerAiProvider;

        root.registerAppLifecycleHook = api.registerAppLifecycleHook;
        root.registerMessageProcessingPlugin = api.registerMessageProcessingPlugin;
        root.registerXmlRenderPlugin = api.registerXmlRenderPlugin;
        root.registerInputMenuTogglePlugin = api.registerInputMenuTogglePlugin;
        root.registerChatInputHook = api.registerChatInputHook;
        root.registerChatViewHook = api.registerChatViewHook;
        root.registerToolLifecycleHook = api.registerToolLifecycleHook;
        root.registerPromptInputHook = api.registerPromptInputHook;
        root.registerPromptHistoryHook = api.registerPromptHistoryHook;
        root.registerPromptEstimateHistoryHook = api.registerPromptEstimateHistoryHook;
        root.registerSystemPromptComposeHook = api.registerSystemPromptComposeHook;
        root.registerToolPromptComposeHook = api.registerToolPromptComposeHook;
        root.registerPromptFinalizeHook = api.registerPromptFinalizeHook;
        root.registerPromptEstimateFinalizeHook = api.registerPromptEstimateFinalizeHook;
        root.registerSummaryGenerateHook = api.registerSummaryGenerateHook;

        installGlobal('ToolPkg', api);
    })();
    "#
    .to_string()
}

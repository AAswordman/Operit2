(function() {
    var callId = __CALL_ID_JSON__;
    var params = __PARAMS_JSON__;
    var scriptText = __SCRIPT_JSON__;
    var targetFunctionName = __FUNCTION_NAME_JSON__;
    try {
        globalThis.__operitExecutionSessions = globalThis.__operitExecutionSessions || {};
        globalThis.__operitCompleteCalled = false;
        globalThis.__operitCompleteValue = undefined;
        var __operitCallRuntime = {
            callId: callId,
            emit: sendIntermediateResult,
            delta: sendIntermediateResult,
            log: sendIntermediateResult,
            update: sendIntermediateResult,
            sendIntermediateResult: sendIntermediateResult,
            done: complete,
            complete: complete
        };

        function text(value) {
            return value == null ? '' : String(value);
        }

        function normalizePath(pathValue) {
            var parts = text(pathValue).replace(/\\/g, '/').split('/');
            var stack = [];
            for (var i = 0; i < parts.length; i += 1) {
                var part = parts[i];
                if (!part || part === '.') {
                    continue;
                }
                if (part === '..') {
                    if (stack.length > 0) {
                        stack.pop();
                    }
                    continue;
                }
                stack.push(part);
            }
            return stack.join('/');
        }

        function dirname(pathValue) {
            var normalized = normalizePath(pathValue);
            var index = normalized.lastIndexOf('/');
            return index < 0 ? '' : normalized.slice(0, index);
        }

        function resolveModulePath(request, fromPath) {
            var normalized = text(request).replace(/\\/g, '/').trim();
            if (!normalized) {
                return '';
            }
            if (!(normalized.startsWith('.') || normalized.startsWith('/'))) {
                return normalized;
            }
            if (normalized.startsWith('/')) {
                return normalizePath(normalized);
            }
            var base = dirname(fromPath);
            return normalizePath(base ? base + '/' + normalized : normalized);
        }

        function buildCandidatePaths(modulePath) {
            var normalized = normalizePath(modulePath);
            if (!normalized) {
                return [];
            }
            if (/\.[a-z0-9]+$/i.test(normalized)) {
                return [normalized];
            }
            return [
                normalized,
                normalized + '.js',
                normalized + '.json',
                normalized + '/index.js',
                normalized + '/index.json'
            ];
        }

        var packageTarget =
            text(params.__operit_ui_package_name || '') ||
            text(params.toolPkgId || '');
        var screenPath = normalizePath(text(params.__operit_script_screen || ''));
        var moduleCache =
            globalThis.__operitModuleInstanceCache &&
            typeof globalThis.__operitModuleInstanceCache === 'object'
                ? globalThis.__operitModuleInstanceCache
                : (globalThis.__operitModuleInstanceCache = {});

        function readToolPkgModule(modulePath) {
            if (!packageTarget || !NativeInterface || typeof NativeInterface.readToolPkgTextResource !== 'function') {
                return null;
            }
            var candidates = buildCandidatePaths(modulePath);
            for (var i = 0; i < candidates.length; i += 1) {
                var candidate = candidates[i];
                var textResult = NativeInterface.readToolPkgTextResource(packageTarget, candidate);
                if (typeof textResult === 'string' && textResult.length > 0) {
                    return { path: candidate, text: textResult };
                }
            }
            return null;
        }

        function executeModule(modulePath, moduleText, requireInternal) {
            var moduleKey = packageTarget + ':' + modulePath + ':' + moduleText.length;
            if (moduleCache[moduleKey]) {
                return moduleCache[moduleKey].exports;
            }
            var module = { exports: {} };
            moduleCache[moduleKey] = module;
            if (/\.json$/i.test(modulePath)) {
                module.exports = JSON.parse(moduleText);
                return module.exports;
            }
            var localRequire = function(nextName) {
                return requireInternal(nextName, modulePath);
            };
            var factory = new Function(
                'module',
                'exports',
                'require',
                '__operit_call_runtime',
                moduleText
            );
            var previousActiveModule = globalThis.__operitActiveModule;
            var previousActiveExports = globalThis.__operitActiveModuleExports;
            globalThis.__operitActiveModule = module;
            globalThis.__operitActiveModuleExports = module.exports;
            try {
                factory(module, module.exports, localRequire, __operitCallRuntime);
            } catch (error) {
                delete moduleCache[moduleKey];
                throw error;
            } finally {
                globalThis.__operitActiveModule = previousActiveModule;
                globalThis.__operitActiveModuleExports = previousActiveExports;
            }
            if (module.exports && typeof module.exports === 'object') {
                module.exports.__operit_toolpkg_module_path = modulePath;
                Object.keys(module.exports).forEach(function(key) {
                    if (typeof module.exports[key] === 'function') {
                        module.exports[key].__operit_toolpkg_module_path = modulePath;
                        module.exports[key].__operit_toolpkg_export_name = key;
                    }
                });
            }
            return module.exports;
        }

        function requireInternal(moduleName, fromPath) {
            var request = text(moduleName).trim();
            if (request === 'lodash') {
                return globalThis._;
            }
            if (request === 'uuid') {
                return {
                    v4: function() {
                        return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function(char) {
                            var random = Math.random() * 16 | 0;
                            var value = char === 'x' ? random : ((random & 0x3) | 0x8);
                            return value.toString(16);
                        });
                    }
                };
            }
            if (!(request.startsWith('.') || request.startsWith('/'))) {
                return {};
            }
            var resolvedPath = resolveModulePath(request, fromPath || screenPath);
            var loaded = readToolPkgModule(resolvedPath);
            if (!loaded) {
                throw new Error('Cannot resolve module "' + request + '" from "' + (fromPath || screenPath || '<root>') + '"');
            }
            return executeModule(loaded.path, loaded.text, requireInternal);
        }

        var module = { exports: {} };
        var exports = module.exports;
        var require = function(moduleName) {
            return requireInternal(moduleName, screenPath);
        };
        var factory = new Function(
            'module',
            'exports',
            'require',
            '__operit_call_runtime',
            scriptText
        );
        var previousActiveModule = globalThis.__operitActiveModule;
        var previousActiveExports = globalThis.__operitActiveModuleExports;
        globalThis.__operitActiveModule = module;
        globalThis.__operitActiveModuleExports = exports;
        globalThis.__operitGetActiveModuleExports = function() {
            return globalThis.__operitActiveModuleExports || null;
        };
        try {
            factory(module, exports, require, __operitCallRuntime);
        } finally {
            globalThis.__operitActiveModule = previousActiveModule;
            globalThis.__operitActiveModuleExports = previousActiveExports;
        }
        if (module.exports && typeof module.exports === 'object') {
            module.exports.__operit_toolpkg_module_path = screenPath || '<root>';
            Object.keys(module.exports).forEach(function(key) {
                if (typeof module.exports[key] === 'function') {
                    module.exports[key].__operit_toolpkg_module_path = screenPath || '<root>';
                    module.exports[key].__operit_toolpkg_export_name = key;
                }
            });
        }
        var target =
            module.exports && typeof module.exports[targetFunctionName] === 'function'
                ? module.exports[targetFunctionName]
                : (typeof globalThis[targetFunctionName] === 'function'
                    ? globalThis[targetFunctionName]
                    : null);
        if (typeof target !== 'function') {
            return JSON.stringify({
                success: false,
                message: "Function '" + targetFunctionName + "' not found in script"
            });
        }
        var invokePreviousActiveModule = globalThis.__operitActiveModule;
        var invokePreviousActiveExports = globalThis.__operitActiveModuleExports;
        globalThis.__operitActiveModule = module;
        globalThis.__operitActiveModuleExports = module.exports || exports;
        var result;
        try {
            result = target(params, __operitCallRuntime);
        } finally {
            globalThis.__operitActiveModule = invokePreviousActiveModule;
            globalThis.__operitActiveModuleExports = invokePreviousActiveExports;
        }
        if (globalThis.__operitCompleteCalled) {
            return __operitFinishExecutionResult(globalThis.__operitCompleteValue);
        }
        if (result && typeof result.then === 'function') {
            globalThis.__operitExecutionSessions[callId] = {
                completed: false,
                output: null
            };
            result.then(
                function(value) {
                    globalThis.__operitExecutionSessions[callId] = {
                        completed: true,
                        output: __operitFinishExecutionResult(globalThis.__operitCompleteCalled ? globalThis.__operitCompleteValue : value)
                    };
                },
                function(error) {
                    globalThis.__operitExecutionSessions[callId] = {
                        completed: true,
                        output: JSON.stringify({
                            success: false,
                            message: String(error && error.message ? error.message : error),
                            data: error && error.data !== undefined ? error.data : null
                        })
                    };
                }
            );
            return "__operit_pending:" + callId;
        }
        return __operitFinishExecutionResult(result);
    } catch (error) {
        return JSON.stringify({
            success: false,
            message: "Script error: " + String(error && error.message ? error.message : error),
            data: error && error.data !== undefined ? error.data : ""
        });
    }
})()

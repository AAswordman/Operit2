var RuntimeContext = (function() {
    var CONTEXT_RUN_IPC_CHANNEL = 'operit.context.run';
    var contextFunctions = Object.create(null);
    var contextRunnerRegistered = false;

    function rootObject() {
        if (typeof globalThis === 'undefined' || !globalThis) {
            throw new Error('RuntimeContext requires globalThis');
        }
        return globalThis;
    }

    function previewJson(value, maxLength) {
        var limit = typeof maxLength === 'number' ? maxLength : 800;
        try {
            var text = JSON.stringify(value);
            if (typeof text !== 'string') {
                return '';
            }
            return text.length > limit ? text.slice(0, limit) + '...' : text;
        } catch (error) {
            var errorText = error && error.message ? error.message : 'preview failed';
            var root = rootObject();
            if (root.console && typeof root.console.error === 'function') {
                root.console.error('[RuntimeContext] preview json failed: ' + errorText);
            }
            return '[unserializable]';
        }
    }

    function validateContextEnvName(name) {
        if (!/^[A-Za-z_$][A-Za-z0-9_$]*$/.test(name)) {
            throw new Error('withContext env name is not a valid identifier: ' + name);
        }
    }

    function escapeRegExp(value) {
        return String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    }

    function buildContextRunnerFactorySource(functionSource, envNames, functionNames) {
        var envBindings = envNames
            .map(function(name) {
                validateContextEnvName(name);
                return 'const ' + name + ' = __operitContextEnvs[' + JSON.stringify(name) + '];';
            })
            .join('\n');
        var functionBindings = functionNames
            .map(function(name) {
                validateContextEnvName(name);
                return 'const ' + name + ' = __operitContextFunctions[' + JSON.stringify(name) + '];';
            })
            .join('\n');
        return '(function(__operitContextEnvs, __operitContextFunctions) {\n' +
            envBindings +
            '\n' +
            functionBindings +
            '\nreturn (' + functionSource + ');\n})';
    }

    function normalizeContextRunnerSource(functionSource) {
        var functionNames = Object.keys(contextFunctions)
            .map(escapeRegExp)
            .join('|');
        if (!functionNames) {
            return functionSource;
        }
        return String(functionSource)
            .replace(
                new RegExp('\\(\\s*0\\s*,\\s*[A-Za-z_$][A-Za-z0-9_$]*\\.(' + functionNames + ')\\s*\\)', 'g'),
                '$1'
            )
            .replace(
                new RegExp('\\b[A-Za-z_$][A-Za-z0-9_$]*\\.(' + functionNames + ')\\b', 'g'),
                '$1'
            );
    }

    async function executeContextRunner(payload) {
        var envs = payload && payload.envs && typeof payload.envs === 'object' ? payload.envs : {};
        try {
            var functionSource = normalizeContextRunnerSource(payload && payload.functionSource);
            var factorySource = buildContextRunnerFactorySource(
                functionSource,
                Object.keys(envs),
                Object.keys(contextFunctions)
            );
            var createRunner = eval(factorySource);
            var runner = createRunner(envs, contextFunctions);
            if (typeof runner !== 'function') {
                throw new Error('withContext runner source did not evaluate to a function');
            }
            return await runner();
        } catch (error) {
            var errorText = error && error.message ? error.message : 'withContext runner failed';
            var root = rootObject();
            if (root.console && typeof root.console.error === 'function') {
                root.console.error(
                    '[RuntimeContext] withContext target execution failed: error=' +
                        errorText +
                        ', envs=' +
                        previewJson(envs)
                );
            }
            throw error;
        }
    }

    function registerContextModule(moduleExports) {
        if (!moduleExports || typeof moduleExports !== 'object') {
            throw new Error('RuntimeContext.register requires a module exports object');
        }
        Object.keys(moduleExports).forEach(function(name) {
            validateContextEnvName(name);
            var value = moduleExports[name];
            if (typeof value === 'function') {
                contextFunctions[name] = value;
            }
        });
    }

    function ensureContextRunnerRegistered() {
        if (contextRunnerRegistered) {
            return;
        }
        var root = rootObject();
        if (!root.ToolPkg || !root.ToolPkg.ipc || typeof root.ToolPkg.ipc.on !== 'function') {
            throw new Error('RuntimeContext requires ToolPkg.ipc');
        }
        contextRunnerRegistered = true;
        root.ToolPkg.ipc.on(CONTEXT_RUN_IPC_CHANNEL, function(payload) {
            return executeContextRunner(payload);
        });
    }

    async function runWithContext(kind, envs, runner, options) {
        if (!runner) {
            throw new Error('withContext requires runner');
        }
        if (typeof runner !== 'function') {
            throw new Error('withContext runner must be a function');
        }
        ensureContextRunnerRegistered();
        var root = rootObject();
        var callOptions = options && typeof options === 'object' ? options : {};
        var ipcOptions = {
            targetRuntime: kind
        };
        if (typeof callOptions.targetContextKey === 'string' && callOptions.targetContextKey.trim().length > 0) {
            ipcOptions.targetContextKey = callOptions.targetContextKey.trim();
        }
        try {
            return await root.ToolPkg.ipc.call(
                CONTEXT_RUN_IPC_CHANNEL,
                {
                    functionSource: runner.toString(),
                    envs: envs && typeof envs === 'object' ? envs : {}
                },
                ipcOptions
            );
        } catch (error) {
            var errorText = error && error.message ? error.message : 'withContext call failed';
            if (root.console && typeof root.console.error === 'function') {
                root.console.error(
                    '[RuntimeContext] withContext call failed: kind=' +
                        String(kind) +
                        ', error=' +
                        errorText +
                        ', envs=' +
                        previewJson(envs)
                );
            }
            throw error;
        }
    }

    return {
        register: registerContextModule,
        withContext: runWithContext,
        __operitEnsureContextRunnerRegistered: ensureContextRunnerRegistered
    };
})();

var withContext = RuntimeContext.withContext;

__operitExpose('RuntimeContext', RuntimeContext);
__operitExpose('withContext', withContext);

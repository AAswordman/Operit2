(function() {
    var root = typeof globalThis !== 'undefined'
        ? globalThis
        : (typeof window !== 'undefined' ? window : this);
    var windowRef = typeof window !== 'undefined' ? window : root;
    var expose = typeof __operitExpose === 'function'
        ? __operitExpose
        : globalThis.__operitExpose;

    function asString(value) {
        return value == null ? '' : String(value);
    }

    function callNative(methodName) {
        if (
            typeof NativeInterface === 'undefined' ||
            !NativeInterface ||
            typeof NativeInterface[methodName] !== 'function'
        ) {
            throw new Error('NativeInterface.' + methodName + ' is unavailable');
        }
        var args = Array.prototype.slice.call(arguments, 1);
        return NativeInterface[methodName].apply(NativeInterface, args);
    }

    function clonePlainObject(value) {
        if (!value || typeof value !== 'object' || Array.isArray(value)) {
            return {};
        }
        var copy = {};
        var keys = Object.keys(value);
        for (var i = 0; i < keys.length; i += 1) {
            copy[keys[i]] = value[keys[i]];
        }
        return copy;
    }

    function normalizeToolCallOptions(value) {
        if (!value || typeof value !== 'object') {
            return {};
        }
        return {
            onIntermediateResult:
                typeof value.onIntermediateResult === 'function'
                    ? value.onIntermediateResult
                    : null
        };
    }

    function parseToolCallArguments(rawArgs) {
        if (rawArgs.length === 1 && typeof rawArgs[0] === 'object') {
            return {
                type: asString(rawArgs[0].type || 'default'),
                name: asString(rawArgs[0].name || ''),
                params: clonePlainObject(rawArgs[0].params),
                options: normalizeToolCallOptions(rawArgs[0])
            };
        }
        if (rawArgs.length === 1 && typeof rawArgs[0] === 'string') {
            return { type: 'default', name: asString(rawArgs[0]), params: {}, options: {} };
        }
        if (rawArgs.length === 2 && typeof rawArgs[1] === 'object') {
            return {
                type: 'default',
                name: asString(rawArgs[0]),
                params: clonePlainObject(rawArgs[1]),
                options: {}
            };
        }
        if (rawArgs.length === 3 && typeof rawArgs[1] === 'object' && typeof rawArgs[2] === 'object') {
            return {
                type: 'default',
                name: asString(rawArgs[0]),
                params: clonePlainObject(rawArgs[1]),
                options: normalizeToolCallOptions(rawArgs[2])
            };
        }
        return {
            type: asString(rawArgs[0] || 'default'),
            name: asString(rawArgs[1] || ''),
            params: clonePlainObject(rawArgs[2]),
            options: normalizeToolCallOptions(rawArgs[3])
        };
    }

    function parseToolResult(result, isError) {
        if (isError) {
            if (result && typeof result === 'object' && result.success === false) {
                throw new Error(asString(result.message).trim());
            }
            throw new Error(typeof result === 'string' ? result : JSON.stringify(result));
        }
        if (result && typeof result === 'object' && Object.prototype.hasOwnProperty.call(result, 'success')) {
            if (result.success) {
                return result.data;
            }
            throw new Error(asString(result.message).trim());
        }
        if (typeof result === 'string' && result.length > 1) {
            var first = result.charAt(0);
            if (first === '{' || first === '[') {
                try {
                    var parsed = JSON.parse(result);
                    return parseToolResult(parsed, false);
                } catch (_error) {
                    return result;
                }
            }
        }
        return result;
    }

    function nextToolCallbackId() {
        return '__operit_tool_' + Date.now() + '_' + Math.random().toString(36).slice(2, 10);
    }

    function toolCall() {
        var rawArgs = arguments;
        return new Promise(function(resolve, reject) {
            try {
                var parsed = parseToolCallArguments(rawArgs);
                var callbackId = nextToolCallbackId();
                var intermediateCallbackId =
                    parsed.options && parsed.options.onIntermediateResult
                        ? nextToolCallbackId()
                        : '';
                windowRef[callbackId] = function(result, isError) {
                    delete windowRef[callbackId];
                    if (intermediateCallbackId) {
                        delete windowRef[intermediateCallbackId];
                    }
                    try {
                        resolve(parseToolResult(result, !!isError));
                    } catch (error) {
                        reject(error);
                    }
                };
                if (intermediateCallbackId) {
                    windowRef[intermediateCallbackId] = function(result, isError) {
                        if (isError) {
                            reject(parseToolResult(result, true));
                            return;
                        }
                        parsed.options.onIntermediateResult(parseToolResult(result, false));
                    };
                    callNative(
                        'callToolAsyncStreaming',
                        callbackId,
                        intermediateCallbackId,
                        parsed.type || 'default',
                        parsed.name,
                        JSON.stringify(parsed.params || {})
                    );
                } else {
                    callNative(
                        'callToolAsync',
                        callbackId,
                        parsed.type || 'default',
                        parsed.name,
                        JSON.stringify(parsed.params || {})
                    );
                }
            } catch (error) {
                reject(error);
            }
        });
    }

    expose('toolCall', toolCall);
})();

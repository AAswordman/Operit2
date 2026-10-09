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
            // Match JSON data properties; never invoke __proto__'s setter.
            Object.defineProperty(copy, keys[i], {
                value: value[keys[i]], enumerable: true, writable: true, configurable: true
            });
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
            if (typeof result === 'string') {
                try {
                    var parsedError = JSON.parse(result);
                    if (parsedError && typeof parsedError === 'object') {
                        result = parsedError;
                    }
                } catch (_error) {}
            }
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
                var parsed;
                try {
                    parsed = JSON.parse(result);
                } catch (_error) {
                    return result;
                }
                return parseToolResult(parsed, false);
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
                var ownerCallId = String(root.__operitCurrentCallId || '');
                if (typeof root.__operitRetainCallReference === 'function') {
                    root.__operitRetainCallReference(ownerCallId);
                }
                // Keep the public Promise wrapper to preserve retain/release and
                // microtask ordering. The native Promise stores its completion
                // functions internally; no global callback name or JSON text.
                if (typeof root.__operitNativeCallToolStructured === 'function' &&
                    !(parsed.options && parsed.options.onIntermediateResult)) {
                    var releaseStructuredReference = function() {
                        Promise.resolve().then(function() {
                            if (typeof root.__operitReleaseCallReference === 'function') {
                                root.__operitReleaseCallReference(ownerCallId);
                            }
                        });
                    };
                    try {
                        root.__operitNativeCallToolStructured(
                            ownerCallId, parsed.type || 'default', parsed.name, parsed.params || {}
                        ).then(function(result) {
                            if (typeof root.__operitActivateCall === 'function') {
                                root.__operitActivateCall(ownerCallId);
                            }
                            try { resolve(parseToolResult(result, false)); }
                            catch (error) { reject(error); }
                            releaseStructuredReference();
                        }, function(error) {
                            if (typeof root.__operitActivateCall === 'function') {
                                root.__operitActivateCall(ownerCallId);
                            }
                            reject(error);
                            releaseStructuredReference();
                        });
                    } catch (error) {
                        reject(error);
                        releaseStructuredReference();
                    }
                    return;
                }
                var callbackId = nextToolCallbackId();
                var intermediateCallbackId =
                    parsed.options && parsed.options.onIntermediateResult
                        ? nextToolCallbackId()
                        : '';
                windowRef[callbackId] = function(result, isError) {
                    if (typeof root.__operitActivateCall === 'function') {
                        root.__operitActivateCall(ownerCallId);
                    }
                    delete windowRef[callbackId];
                    if (intermediateCallbackId) {
                        delete windowRef[intermediateCallbackId];
                    }
                    try {
                        resolve(parseToolResult(result, !!isError));
                    } catch (error) {
                        reject(error);
                    }
                    Promise.resolve().then(function() {
                        if (typeof root.__operitReleaseCallReference === 'function') {
                            root.__operitReleaseCallReference(ownerCallId);
                        }
                    });
                };
                if (intermediateCallbackId) {
                    windowRef[intermediateCallbackId] = function(result, isError) {
                        try {
                            if (isError) {
                                reject(parseToolResult(result, true));
                                return;
                            }
                            parsed.options.onIntermediateResult(parseToolResult(result, false));
                        } catch (error) {
                            reject(error);
                        }
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

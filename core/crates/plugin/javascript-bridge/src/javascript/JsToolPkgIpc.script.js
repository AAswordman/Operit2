/** One engine-owned IPC service. Call identity is supplied by the native execution entry. */
(function(root) {
    var handlers = new Map();

    function text(value) { return value == null ? '' : String(value).trim(); }

    function activeCall() {
        var call = root.__operitGetCallState(root.__operitCurrentCallId);
        if (!call || call.completed || !call.toolPkgContext) {
            throw new Error('ToolPkg.ipc requires an active execution context');
        }
        return call;
    }

    function channelName(value) {
        var channel = text(value);
        if (!channel) throw new Error('ToolPkg.ipc channel is required');
        return channel;
    }

    function off(channel, handler) {
        channel = text(channel);
        if (!handlers.has(channel) || (typeof handler === 'function' && handlers.get(channel) !== handler)) return false;
        return handlers.delete(channel);
    }

    function on(channel, handler) {
        channel = channelName(channel);
        if (typeof handler !== 'function') throw new TypeError('ToolPkg.ipc handler must be a function');
        handlers.set(channel, handler);
        return function() { off(channel, handler); };
    }

    /** Both local calls and native deliveries use the active target's canonical identity. */
    function dispatch(channel, payload, callerContextKey) {
        channel = channelName(channel);
        var context = activeCall().toolPkgContext;
        var handler = handlers.get(channel);
        if (!handler) throw new Error('ToolPkg.ipc channel is not registered: ' + channel);
        return handler(payload, {
            channel: channel,
            callerContextKey: callerContextKey === undefined ? context.contextKey : callerContextKey,
            currentContextKey: context.contextKey,
            currentRuntime: context.runtime,
            packageTarget: context.packageTarget
        });
    }

    function resultValue(response) {
        if (!response || typeof response !== 'object' || typeof response.success !== 'boolean') {
            throw new TypeError('IPC host returned an invalid result envelope');
        }
        if (!response.success) throw new Error(response.message);
        return response.value;
    }

    function call(channel, payload, options) {
        try {
            channel = channelName(channel);
            var context = activeCall().toolPkgContext;
            options = options && typeof options === 'object' ? options : {};
            var targetRuntime = text(options.targetRuntime).toLowerCase();
            var targetContextKey = text(options.targetContextKey);
            if (targetRuntime && ['main', 'ui', 'sandbox', 'provider'].indexOf(targetRuntime) < 0) {
                throw new Error('ToolPkg.ipc targetRuntime is invalid: ' + targetRuntime);
            }
            var local = targetContextKey
                ? targetContextKey === context.contextKey
                : context.runtime === 'main' && (!targetRuntime || targetRuntime === 'main');
            if (local) {
                if (targetRuntime && targetRuntime !== context.runtime) {
                    throw new Error('ToolPkg.ipc targetRuntime does not match current runtime: ' + targetRuntime + ' != ' + context.runtime);
                }
                return Promise.resolve(dispatch(channel, payload));
            }
            if (!context.packageTarget) throw new Error('ToolPkg.ipc requires a bound package target');
            return root.__operitInvokeHostAsync(root.__operitNativeInvokeToolPkgIpc, [
                context.packageTarget, context.contextKey, targetContextKey, targetRuntime,
                channel, payload === undefined ? null : payload
            ], resultValue);
        } catch (error) {
            return Promise.reject(error);
        }
    }

    root.__operitToolPkgIpc = { on: on, off: off, call: call };
    root.__operitInvokeToolPkgIpcLocal = dispatch;
})(globalThis);

/** Installs implemented method decorators over the existing Core route families. */
function __operitInstallToolPkgRouteApi() {
    var api = globalThis.ToolPkg;
    if (!api || typeof api !== "object") throw new Error("ToolPkg registry is unavailable");
    if (globalThis.__operitToolPkgRoutes) return;
    var state = { methods: new Map(), streams: new Map(), sequence: 0 };
    globalThis.__operitToolPkgRoutes = state;
    var generatorPrototype = Object.getPrototypeOf(async function* () {});
    var asyncPrototype = Object.getPrototypeOf(async function () {});
    /** Uses the existing scoped host Promise bridge. */
    function invoke(binding, args) { return __operitInvokeHostAsync(binding, args); }
    /** Creates a collector whose cancellation interrupts the source's pending storage wait. */
    function watch(route, receiver, args, binding, remote) {
        var opening = null, closing = null, stopped = false, pending = false;
        var controller = new AbortController();
        /** Opens the selected implementation once on the first consumer read. */
        function open() {
            if (opening === null) opening = remote
                ? invoke(__operitNativeOpenCoreRouteWatch, ["watchToolPkgRoute", route.name, { bindingKey: binding, arguments: args }])
                : Promise.resolve({ iterator: route.implementation.apply(receiver, args.concat(controller.signal)) });
            return opening;
        }
        /** Aborts pending source waits before returning the generator. */
        function close() {
            stopped = true;
            controller.abort();
            if (opening === null) return Promise.resolve();
            if (closing === null) closing = opening.then(/** Releases the exact opened collector. */ function (source) {
                if (remote) { __operitNativeCloseCoreRouteWatch(source.subscriptionId); return; }
                return source.iterator.return();
            });
            return closing;
        }
        return {
            /** Exposes the normal JavaScript async iteration protocol. */
            [Symbol.asyncIterator]() { return this; },
            /** Reads one ordered value with at most one pending request. */
            async next() {
                if (stopped) return { done: true, value: undefined };
                if (pending) throw new Error("Route stream already has a pending read");
                pending = true;
                try {
                    var source = await open();
                    if (stopped) return { done: true, value: undefined };
                    var result = remote ? await invoke(__operitNativeNextCoreRouteWatch, [source.subscriptionId]) : await source.iterator.next();
                    if (stopped || result === null || result.done) { await close(); return { done: true, value: undefined }; }
                    return { done: false, value: result.value };
                } catch (error) { await close(); throw error; }
                finally { pending = false; }
            },
            /** Cancels a pending read without queuing behind generator next. */
            async return() { await close(); return { done: true, value: undefined }; },
            /** Releases this collector and propagates the consumer error. */
            async throw(error) { await close(); throw error; },
        };
    }
    /** Sends an ordinary async iterable through the ordered Link Push session. */
    async function push(route, args, binding, inputIndex) {
        var input = args[inputIndex];
        var wireArgs = args.slice();
        wireArgs[inputIndex] = null;
        var source = await invoke(__operitNativeOpenToolPkgRoutePush, [route.name, { bindingKey: binding, arguments: wireArgs, inputIndex: inputIndex }]);
        var closed = false;
        try {
            for await (var value of input) await invoke(__operitNativeSendToolPkgRoutePush, [source.subscriptionId, value]);
            closed = true;
            await invoke(__operitNativeCloseToolPkgRoutePush, [source.subscriptionId, false]);
        } catch (error) {
            if (!closed) await invoke(__operitNativeCloseToolPkgRoutePush, [source.subscriptionId, true]);
            throw error;
        }
    }
    /** Retains the actual local method and derives its remote identity from its declaring service. */
    api.route = function (options) {
        if (!options || !Number.isInteger(options.binding) || options.binding < 0) throw new TypeError("Route binding must be an argument index");
        /** Wraps a standard async method or async generator without an explicit proxy declaration. */
        return function (implementation, context) {
            if (!context || context.kind !== "method" || context.private || typeof context.name !== "string") throw new TypeError("Routes require public methods");
            var prototype = Object.getPrototypeOf(implementation);
            var generator = prototype === generatorPrototype;
            if (!generator && prototype !== asyncPrototype) throw new TypeError("Routes require an async method or async generator");
            var route = { name: null, implementation: implementation, binding: options.binding, generator: generator, receiver: null };
            context.addInitializer(/** Registers the implemented service on its owning runtime. */ function () {
                var service = context.static ? this : this.constructor;
                var name = service.name + "." + context.name;
                if (!service.name) throw new TypeError("Routed services require a named class");
                if (state.methods.has(name)) throw new Error("Duplicate plugin route: " + name);
                route.name = name;
                route.receiver = this;
                state.methods.set(name, route);
            });
            /** Selects the existing route family from the standard method and input types. */
            return function () {
                if (route.name === null) throw new Error("Routed service has not been initialized");
                var args = Array.from(arguments);
                var binding = args[route.binding];
                if (typeof binding !== "string" || !binding.trim()) throw new TypeError("Route Binding key must be a nonempty string");
                var remote = __operitNativeShouldRouteToolPkg(binding, generator);
                if (generator) return watch(route, this, args, binding, remote);
                if (!remote) return implementation.apply(this, args);
                var inputs = [];
                args.forEach(/** Finds async input streams by their standard protocol. */ function (value, index) {
                    if (value !== null && value !== undefined && typeof value[Symbol.asyncIterator] === "function") inputs.push(index);
                });
                if (inputs.length > 1) throw new TypeError("A routed method supports one async input stream");
                if (inputs.length === 1) return push(route, args, binding, inputs[0]);
                return invoke(__operitNativeCallCoreRoute, ["callToolPkgRoute", route.name, { bindingKey: binding, arguments: args }]);
            };
        };
    };
    /** Invokes decorated local bodies only after native installation and Binding authorization. */
    globalThis.__operitInvokeToolPkgRouteLocal = async function (payload) {
        var source;
        if (["next", "close", "pushItem", "pushClose", "pushAbort"].indexOf(payload.op) >= 0) {
            source = state.streams.get(payload.id);
            if (!source) throw new Error("Plugin route stream is not active");
            if (payload.op === "next") return source.iterator.next();
            if (payload.op === "close") {
                source.controller.abort();
                try { await source.iterator.return(); }
                finally { state.streams.delete(payload.id); }
                return null;
            }
            if (payload.op === "pushItem") return source.send(payload.value);
            source.finish(payload.op === "pushAbort");
            try { await source.result; return null; }
            finally { state.streams.delete(payload.id); }
        }
        var route = state.methods.get(payload.methodName);
        if (!route) throw new Error("Plugin route method is not decorated: " + payload.methodName);
        if (!Array.isArray(payload.arguments) || payload.arguments[route.binding] !== payload.bindingKey) throw new Error("Plugin route Binding does not match its declared argument");
        if (payload.op === "call") {
            if (route.generator) throw new Error("Generator routes require Watch");
            return route.implementation.apply(route.receiver, payload.arguments);
        }
        var id = "plugin-route-" + (++state.sequence);
        if (payload.op === "open") {
            if (!route.generator) throw new Error("Watch routes require an async generator");
            var controller = new AbortController();
            state.streams.set(id, { controller: controller, iterator: route.implementation.apply(route.receiver, payload.arguments.concat(controller.signal)) });
            return id;
        }
        if (payload.op !== "pushOpen" || route.generator) throw new Error("Invalid plugin route operation");
        var waiting = null, queued = null, ended = false, failure = null;
        var input = {
            /** Exposes the remote producer as an ordinary async input iterable. */
            [Symbol.asyncIterator]() { return this; },
            /** Consumes one ordered value and acknowledges the sender. */
            next() {
                if (failure !== null) return Promise.reject(failure);
                if (queued !== null) { var item = queued; queued = null; item.accept(); return Promise.resolve({ done: false, value: item.value }); }
                if (ended) return Promise.resolve({ done: true, value: undefined });
                if (waiting !== null) return Promise.reject(new Error("Push input already has a pending read"));
                return new Promise(/** Retains the pending consumer read until input arrives. */ function (resolve, reject) { waiting = { resolve: resolve, reject: reject }; });
            },
            /** Stops input when the implementation exits its for-await loop. */
            async return() { source.finish(false); return { done: true, value: undefined }; },
        };
        source = {
            /** Applies backpressure until the implementation consumes this value. */
            send(value) {
                if (failure !== null) return Promise.reject(failure);
                if (ended) return Promise.reject(new Error("Push consumer is closed"));
                if (waiting !== null) { var consumer = waiting; waiting = null; consumer.resolve({ done: false, value: value }); return Promise.resolve(null); }
                if (queued !== null) return Promise.reject(new Error("Push already has a pending value"));
                return new Promise(/** Retains one value until the consumer reads it. */ function (accept, reject) { queued = { value: value, accept: accept, reject: reject }; });
            },
            /** Releases waiting producers and consumers on completion or failure. */
            finish(aborted, error) {
                ended = true;
                if (error !== undefined) failure = error;
                else if (aborted) failure = new Error("Push aborted");
                if (waiting !== null) {
                    if (failure !== null) waiting.reject(failure);
                    else waiting.resolve({ done: true, value: undefined });
                    waiting = null;
                }
                if (queued !== null) { queued.reject(failure !== null ? failure : new Error("Push consumer completed")); queued = null; }
            },
        };
        var args = payload.arguments.slice();
        if (!Number.isInteger(payload.inputIndex) || payload.inputIndex < 0 || payload.inputIndex >= args.length || payload.inputIndex === route.binding) throw new Error("Invalid Push input argument");
        args[payload.inputIndex] = input;
        source.result = Promise.resolve().then(/** Runs the implemented consumer with its normal async iterable. */ function () { return route.implementation.apply(route.receiver, args); });
        source.result.then(/** Ends input after successful consumer completion. */ function () { source.finish(false); }, /** Preserves consumer errors for the producer. */ function (error) { source.finish(true, error); });
        state.streams.set(id, source);
        return id;
    };
}

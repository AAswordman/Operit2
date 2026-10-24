/** Creates the Web host's private structured Promise registry and captured value intrinsics. */
(function() {
    var NativePromise = Promise;
    var ByteArray = Uint8Array;
    var transferValue = globalThis.structuredClone;
    var bufferGetter = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(Uint8Array.prototype), 'buffer').get;
    var NativeMap = Map;
    var keys = Object.keys;
    var defineProperty = Object.defineProperty;
    var isArray = Array.isArray;
    var isFiniteNumber = Number.isFinite;
    var apply = Reflect.apply;
    var mapGet = Map.prototype.get, mapSet = Map.prototype.set;
    var mapDelete = Map.prototype.delete, mapForEach = Map.prototype.forEach;
    var mapSize = Object.getOwnPropertyDescriptor(Map.prototype, 'size').get;
    var arrayIndexOf = Array.prototype.indexOf;
    var pending = new NativeMap();
    var nextId = 0;
    var maxPending = 4096;
    var maxDepth = 128;
    var maxNodes = 1000000;

    /** Copies JSON-compatible data without generating text or invoking property setters. */
    function snapshot(value, ancestors, budget, depth) {
        budget.nodes++;
        if (depth > maxDepth || budget.nodes > maxNodes) {
            throw new RangeError('Host arguments exceed structured bridge depth/node limit');
        }
        if (value === null) return null;
        switch (typeof value) {
            case 'string': case 'boolean': return value;
            case 'number': return isFiniteNumber(value) ? value : null;
            case 'bigint': throw new TypeError('BigInt is not supported in Host data');
            case 'undefined': case 'function': case 'symbol': return undefined;
        }
        if (apply(arrayIndexOf, ancestors, [value]) !== -1) throw new TypeError('Host arguments contain a cycle');
        defineProperty(ancestors, ancestors.length, { value: value, configurable: true, writable: true });
        var result;
        if (isArray(value)) {
            var length = value.length;
            if (length > maxNodes - budget.nodes) throw new RangeError('Host array exceeds node limit');
            result = [];
            for (var index = 0; index < length; index++) {
                var element = snapshot(value[index], ancestors, budget, depth + 1);
                defineProperty(result, index, {
                    value: element === undefined ? null : element,
                    enumerable: true, writable: true, configurable: true
                });
            }
        } else {
            result = {};
            var properties = keys(value);
            for (var position = 0; position < properties.length; position++) {
                var property = properties[position];
                var item = snapshot(value[property], ancestors, budget, depth + 1);
                if (item !== undefined) {
                    defineProperty(result, property, {
                        value: item, enumerable: true, writable: true, configurable: true
                    });
                }
            }
        }
        ancestors.length--;
        return result;
    }

    /** Normalizes lifecycle return values with the same captured host value semantics. */
    function snapshotResult(value) {
        if (value === undefined) return null;
        var result = snapshot(value, [], { nodes: 0 }, 0);
        if (result === undefined) throw new TypeError('Host lifecycle result must be a JSON-compatible value');
        return result;
    }

    /** Copies synchronous host arguments with the same captured structured semantics. */
    function syncBinding(submit) {
        /** Calls one synchronous host operation without serializing application values. */
        return function() {
            var owned = [];
            var budget = { nodes: 0 };
            for (var index = 0; index < arguments.length; index++) {
                var argument = arguments[index];
                var value = argument instanceof ByteArray
                    ? apply(transferValue, undefined, [argument, {transfer:[apply(bufferGetter, argument, [])]}])
                    : snapshot(argument, [], budget, 0);
                if (value === undefined) throw new TypeError('Host arguments must be JSON-compatible values');
                defineProperty(owned, index, { value: value, enumerable: true, writable: true, configurable: true });
            }
            return apply(submit, undefined, owned);
        };
    }

    /** Creates a binding whose completion functions remain inside this host runtime. */
    function binding(submit) {
        /** Converts owned arguments before retaining a scoped pair of Promise functions. */
        return function(scope) {
            if (typeof scope !== 'string') throw new TypeError('Host execution scope must be a string');
            if (apply(mapSize, pending, []) >= maxPending) throw new RangeError('Too many pending host promises');
            var owned = [];
            var budget = { nodes: 0 };
            for (var index = 1; index < arguments.length; index++) {
                var argument = arguments[index];
                var value = argument instanceof ByteArray
                    ? apply(transferValue, undefined, [argument, {transfer:[apply(bufferGetter, argument, [])]}])
                    : snapshot(argument, [], budget, 0);
                if (value === undefined) throw new TypeError('Host arguments must be JSON-compatible values');
                defineProperty(owned, index - 1, { value: value, enumerable: true, writable: true, configurable: true });
            }
            if (apply(mapSize, pending, []) >= maxPending) throw new RangeError('Too many pending host promises');
            if (nextId >= 9007199254740991) throw new RangeError('Host Promise request id exhausted');
            var id = ++nextId;
            return new NativePromise(function(resolve, reject) {
                apply(mapSet, pending, [id, { scope: scope, resolve: resolve, reject: reject }]);
                try {
                    var args = [id];
                    for (var position = 0; position < owned.length; position++) {
                        defineProperty(args, position + 1, { value: owned[position], writable: true, configurable: true });
                    }
                    apply(submit, undefined, args);
                } catch (error) {
                    apply(mapDelete, pending, [id]);
                    reject(error);
                }
            });
        };
    }

    /** Delivers data once and removes both completion handles before invoking JavaScript. */
    function settle(id, value, reject) {
        var request = apply(mapGet, pending, [id]);
        if (!request) return;
        apply(mapDelete, pending, [id]);
        if (reject) request.reject(value);
        else request.resolve(value);
    }

    /** Rejects cancelled requests so suspended JavaScript can release shared resources. */
    function cancel(scope) {
        apply(mapForEach, pending, [function(request, id) {
            if (request.scope === scope) {
                apply(mapDelete, pending, [id]);
                request.reject('JavaScript execution was cancelled');
            }
        }]);
    }

    // Web owns its request snapshot in the engine; native hosts own the same
    // request as a Rust tree behind an opaque transport capsule.
    defineProperty(globalThis, '__operitHostSnapshotJsonObject', {
        value: function(value) { return globalThis.__operitSnapshotJsonObject(value); },
        writable: false, configurable: false, enumerable: false
    });

    defineProperty(globalThis, '__operitHostPromiseRegistry', {
        value: Object.freeze({ binding: binding, syncBinding: syncBinding, snapshotResult: snapshotResult, settle: settle, cancel: cancel }),
        writable: false, configurable: false, enumerable: false
    });
})();

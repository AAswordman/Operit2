/** Creates the Web host's private structured Promise registry and captured value intrinsics. */
(function() {
    var NativePromise = Promise;
    var NativeMap = Map;
    var keys = Object.keys;
    var defineProperty = Object.defineProperty;
    var isArray = Array.isArray;
    var isFiniteNumber = Number.isFinite;
    var NumberObject = Number, StringObject = String, BooleanObject = Boolean;
    var numberValue = Number.prototype.valueOf;
    var stringValue = String.prototype.valueOf;
    var booleanValue = Boolean.prototype.valueOf;
    var bigintValue = BigInt.prototype.valueOf;
    var BigIntObject = BigInt;
    var apply = Reflect.apply;
    var mapGet = Map.prototype.get, mapSet = Map.prototype.set;
    var mapDelete = Map.prototype.delete, mapForEach = Map.prototype.forEach;
    var mapSize = Object.getOwnPropertyDescriptor(Map.prototype, 'size').get;
    var arrayIndexOf = Array.prototype.indexOf;
    var StringValue = String;
    var pending = new NativeMap();
    var nextId = 0;
    var maxPending = 4096;
    var maxDepth = 128;
    var maxNodes = 1000000;

    /** Copies JSON-compatible data without generating text or invoking property setters. */
    function snapshot(value, key, ancestors, budget, depth) {
        budget.nodes++;
        if (depth > maxDepth || budget.nodes > maxNodes) {
            throw new RangeError('Host arguments exceed structured bridge depth/node limit');
        }
        if ((value !== null && typeof value === 'object') || typeof value === 'bigint') {
            var toJSON = value.toJSON;
            if (typeof toJSON === 'function') value = apply(toJSON, value, [key]);
        }
        if (value instanceof NumberObject) value = apply(numberValue, value, []);
        else if (value instanceof StringObject) value = apply(stringValue, value, []);
        else if (value instanceof BooleanObject) value = apply(booleanValue, value, []);
        else if (value instanceof BigIntObject) {
            value = apply(bigintValue, value, []);
        }
        if (value === null) return null;
        switch (typeof value) {
            case 'string': case 'boolean': return value;
            case 'number': return isFiniteNumber(value) ? value : null;
            case 'bigint': throw new TypeError('BigInt host arguments require a toJSON conversion');
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
                var element = snapshot(value[index], StringValue(index), ancestors, budget, depth + 1);
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
                var item = snapshot(value[property], property, ancestors, budget, depth + 1);
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
        var result = snapshot(value, '', [], { nodes: 0 }, 0);
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
                var value = snapshot(arguments[index], '', [], budget, 0);
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
                var value = snapshot(arguments[index], '', [], budget, 0);
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

    /** Releases every pending completion belonging to the cancelled scope. */
    function cancel(scope) {
        apply(mapForEach, pending, [function(request, id) {
            if (request.scope === scope) apply(mapDelete, pending, [id]);
        }]);
    }

    defineProperty(globalThis, '__operitHostPromiseRegistry', {
        value: Object.freeze({ binding: binding, syncBinding: syncBinding, snapshotResult: snapshotResult, settle: settle, cancel: cancel }),
        writable: false, configurable: false, enumerable: false
    });
})();

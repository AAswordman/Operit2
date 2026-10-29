(function(root) {
    'use strict';
    if (root.__operitCreateToolPkgWasmApi) return;
    // A realm owns its modules and default instances; package identity is part of every key.
    var modules = new Map();
    var defaults = new Map();
    function scalarArguments(args) {
        if (args === undefined || args === null) return [];
        if (!Array.isArray(args)) throw new TypeError('ToolPkg.wasm args must be an array');
        return args.map(function(arg) {
            if (!arg || typeof arg !== 'object') throw new TypeError('Invalid WASM argument');
            var value = arg.value;
            switch (arg.type) {
                case 'i32':
                    if (!Number.isInteger(value) || value < -2147483648 || value > 2147483647) throw new RangeError('WASM i32 argument is out of range');
                    return value;
                case 'i64':
                    if (typeof value === 'number' && !Number.isSafeInteger(value)) throw new RangeError('WASM i64 numbers must be safe integers; use a decimal string');
                    if (typeof value !== 'number' && (typeof value !== 'string' || !/^-?(0|[1-9][0-9]*)$/.test(value))) throw new TypeError('WASM i64 requires a decimal integer');
                    var integer = BigInt(value);
                    if (integer < -(1n << 63n) || integer >= (1n << 63n)) throw new RangeError('WASM i64 argument is out of range');
                    return integer;
                case 'f32': case 'f64':
                    if ((typeof value !== 'number' && typeof value !== 'string') || String(value).trim() === '' || !Number.isFinite(Number(value))) throw new TypeError('WASM floating-point arguments must be finite');
                    return Number(value);
                default: throw new TypeError('Unsupported WASM argument type: ' + arg.type);
            }
        });
    }
    function range(offset, length, capacity) {
        if (!Number.isSafeInteger(offset) || !Number.isSafeInteger(length) || offset < 0 || length < 0 || offset > capacity - length) throw new RangeError('WASM memory range is out of bounds');
    }
    function handle(instance, allowed) {
        function exports() {
            if (!instance) throw new Error('WASM instance is closed');
            return instance.exports;
        }
        function memory(name) {
            name = name === undefined ? 'memory' : name;
            var value = exports()[name];
            if (!(value instanceof WebAssembly.Memory)) throw new Error('WASM memory export not found: ' + name);
            return new Uint8Array(value.buffer);
        }
        return {
            call: function(name, args) {
                if (allowed.indexOf(name) < 0) throw new Error("ToolPkg WASM export '" + name + "' is not declared");
                var fn = exports()[name];
                if (typeof fn !== 'function') throw new Error('WASM function export not found: ' + name);
                var result = fn.apply(undefined, scalarArguments(args));
                if (result === undefined) return null;
                if (typeof result === 'bigint') return result.toString();
                if (typeof result !== 'number' || !Number.isFinite(result)) throw new TypeError('WASM scalar result must be finite');
                return result;
            },
            memory: memory,
            readMemory: function(offset, length, name) {
                var view = memory(name); range(offset, length, view.byteLength);
                return view.slice(offset, offset + length);
            },
            writeMemory: function(offset, bytes, name) {
                if (!(bytes instanceof Uint8Array)) throw new TypeError('WASM writes require Uint8Array');
                var view = memory(name); range(offset, bytes.byteLength, view.byteLength); view.set(bytes, offset);
            },
            close: function() { instance = null; }
        };
    }
    root.__operitCreateToolPkgWasmApi = function(resolveTarget, registrationOnly) {
        function identity(moduleId, operation) {
            if (registrationOnly) throw new Error('ToolPkg.wasm.' + operation + ' is unavailable during ToolPkg registration');
            var target = resolveTarget();
            if (!target) throw new Error('ToolPkg.wasm package target is required');
            if (typeof moduleId !== 'string' || !moduleId.trim()) throw new Error('ToolPkg.wasm module id is required');
            moduleId = moduleId.trim().toLowerCase();
            return { target: target, module: moduleId, key: target + '\0' + moduleId };
        }
        function load(id) {
            var entry = modules.get(id.key);
            if (!entry) {
                var loaded = root.__operitNativeLoadToolPkgWasm(id.target, id.module);
                if (!loaded || !(loaded.bytes instanceof Uint8Array) || !loaded.metadata || !Array.isArray(loaded.metadata.exports)) throw new Error('Invalid WASM module response');
                entry = { bytes: loaded.bytes, exports: loaded.metadata.exports, compiled: null };
                modules.set(id.key, entry);
            }
            return entry;
        }
        function open(id, imports) {
            var entry = load(id);
            if (!entry.compiled) entry.compiled = new WebAssembly.Module(entry.bytes);
            // Compilation runs once on the realm's executor. Synchronous constructors also work
            // on embedders without a browser task runner for asynchronous WASM compilation.
            return handle(new WebAssembly.Instance(entry.compiled, imports || {}), entry.exports);
        }

        return {
            read: function(moduleId) {
                var id = identity(moduleId, 'read');
                return Promise.resolve().then(function() { return load(id).bytes.slice(); });
            },
            open: function(moduleId, imports) {
                var id = identity(moduleId, 'open');
                return Promise.resolve().then(function() { return open(id, imports); });
            },
            call: function(moduleId, name, args) {
                var id = identity(moduleId, 'call');
                return Promise.resolve().then(function() {
                    if (typeof name !== 'string' || !name.trim()) throw new Error('ToolPkg.wasm export name is required');
                    scalarArguments(args);
                    var pending = defaults.get(id.key);
                    if (!pending) {
                        pending = Promise.resolve().then(function() { return open(id); }).catch(function(error) { defaults.delete(id.key); throw error; });
                        defaults.set(id.key, pending);
                    }
                    return pending.then(function(instance) { return instance.call(name.trim(), args); });
                });
            },
            unload: function(moduleId) {
                var id = identity(moduleId, 'unload');
                var pending = defaults.get(id.key);
                defaults.delete(id.key); modules.delete(id.key);
                if (pending) pending.then(function(instance) { instance.close(); }, function() {});
            }
        };
    };
})(globalThis);

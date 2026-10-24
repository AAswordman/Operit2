"use strict";
var __runInitializers = (this && this.__runInitializers) || function (thisArg, initializers, value) {
    var useValue = arguments.length > 2;
    for (var i = 0; i < initializers.length; i++) {
        value = useValue ? initializers[i].call(thisArg, value) : initializers[i].call(thisArg);
    }
    return useValue ? value : void 0;
};
var __esDecorate = (this && this.__esDecorate) || function (ctor, descriptorIn, decorators, contextIn, initializers, extraInitializers) {
    function accept(f) { if (f !== void 0 && typeof f !== "function") throw new TypeError("Function expected"); return f; }
    var kind = contextIn.kind, key = kind === "getter" ? "get" : kind === "setter" ? "set" : "value";
    var target = !descriptorIn && ctor ? contextIn["static"] ? ctor : ctor.prototype : null;
    var descriptor = descriptorIn || (target ? Object.getOwnPropertyDescriptor(target, contextIn.name) : {});
    var _, done = false;
    for (var i = decorators.length - 1; i >= 0; i--) {
        var context = {};
        for (var p in contextIn) context[p] = p === "access" ? {} : contextIn[p];
        for (var p in contextIn.access) context.access[p] = contextIn.access[p];
        context.addInitializer = function (f) { if (done) throw new TypeError("Cannot add initializers after decoration has completed"); extraInitializers.push(accept(f || null)); };
        var result = (0, decorators[i])(kind === "accessor" ? { get: descriptor.get, set: descriptor.set } : descriptor[key], context);
        if (kind === "accessor") {
            if (result === void 0) continue;
            if (result === null || typeof result !== "object") throw new TypeError("Object expected");
            if (_ = accept(result.get)) descriptor.get = _;
            if (_ = accept(result.set)) descriptor.set = _;
            if (_ = accept(result.init)) initializers.unshift(_);
        }
        else if (_ = accept(result)) {
            if (kind === "field") initializers.unshift(_);
            else descriptor[key] = _;
        }
    }
    if (target) Object.defineProperty(target, contextIn.name, descriptor);
    done = true;
};
Object.defineProperty(exports, "__esModule", { value: true });
/** Implements ordinary async services used by the route integration tests. */
let ExampleService = (() => {
    var _a;
    let _staticExtraInitializers = [];
    let _static_read_decorators;
    let _static_values_decorators;
    let _static_consume_decorators;
    return _a = class ExampleService {
            /** Executes the implemented method on the Binding-selected runtime. */
            static async read(chatId, amount) {
                this.count += amount;
                return { chatId, count: this.count };
            }
            /** Emits values using standard async generator syntax. */
            static async *values(chatId, signal) {
                if (!signal)
                    throw new Error("Collector signal is required");
                yield chatId;
                await new Promise((resolve) => { signal.addEventListener("abort", resolve); });
            }
            /** Consumes ordered input using a standard for-await loop. */
            static async consume(chatId, input) {
                for await (const value of input) {
                    if (value < 0)
                        throw new Error("Consumer rejected a negative value");
                    this.count += value;
                }
            }
        },
        (() => {
            const _metadata = typeof Symbol === "function" && Symbol.metadata ? Object.create(null) : void 0;
            _static_read_decorators = [ToolPkg.route({ binding: 0 })];
            _static_values_decorators = [ToolPkg.route({ binding: 0 })];
            _static_consume_decorators = [ToolPkg.route({ binding: 0 })];
            __esDecorate(_a, null, _static_read_decorators, { kind: "method", name: "read", static: true, private: false, access: { has: obj => "read" in obj, get: obj => obj.read }, metadata: _metadata }, null, _staticExtraInitializers);
            __esDecorate(_a, null, _static_values_decorators, { kind: "method", name: "values", static: true, private: false, access: { has: obj => "values" in obj, get: obj => obj.values }, metadata: _metadata }, null, _staticExtraInitializers);
            __esDecorate(_a, null, _static_consume_decorators, { kind: "method", name: "consume", static: true, private: false, access: { has: obj => "consume" in obj, get: obj => obj.consume }, metadata: _metadata }, null, _staticExtraInitializers);
            if (_metadata) Object.defineProperty(_a, Symbol.metadata, { enumerable: true, configurable: true, writable: true, value: _metadata });
        })(),
        _a.count = (__runInitializers(_a, _staticExtraInitializers), 0),
        _a;
})();
/** Verifies that instance receivers and same-named methods on other classes stay distinct. */
let InstanceService = (() => {
    var _a;
    let _instanceExtraInitializers = [];
    let _read_decorators;
    return _a = class InstanceService {
            constructor() {
                this.prefix = (__runInitializers(this, _instanceExtraInitializers), "instance:");
            }
            /** Preserves the registered instance as the remote receiver. */
            async read(chatId) { return this.prefix + chatId; }
        },
        (() => {
            const _metadata = typeof Symbol === "function" && Symbol.metadata ? Object.create(null) : void 0;
            _read_decorators = [ToolPkg.route({ binding: 0 })];
            __esDecorate(_a, null, _read_decorators, { kind: "method", name: "read", static: false, private: false, access: { has: obj => "read" in obj, get: obj => obj.read }, metadata: _metadata }, null, _instanceExtraInitializers);
            if (_metadata) Object.defineProperty(_a, Symbol.metadata, { enumerable: true, configurable: true, writable: true, value: _metadata });
        })(),
        _a;
})();
Object.assign(globalThis, { ExampleService, instanceService: new InstanceService() });

/** Creates independent explicit-device views over the canonical tool namespaces. */
Tools.device = __operitToolPkgApi.method().since("2.0.0", function(nodeId, options) {
    if (typeof nodeId !== "string" || !nodeId || nodeId.length > 256 || /[\s\x00-\x1f\x7f]/.test(nodeId)) {
        throw new TypeError("Tools.device requires an exact nonblank node ID");
    }
    if (options !== undefined && (!options || typeof options !== "object" || Array.isArray(options))) {
        throw new TypeError("Device options must be an object");
    }
    options = options || {};
    var timeoutMs = options.timeoutMs === undefined ? 120000 : options.timeoutMs;
    if (!Number.isInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > 86400000) {
        throw new TypeError("Device timeoutMs must be an integer between 1 and 86400000");
    }
    var cancellations = [];
    var context = {nodeId: nodeId, options: {timeoutMs: timeoutMs, signal: options.signal, __deviceCancellations: cancellations}};
    function namespace(source) {
        var target = {};
        Object.defineProperty(target, "__operitDeviceContext", {value: context});
        Object.keys(source).forEach(function(key) {
            if (key === "device") return;
            var value = source[key];
            if (typeof value === "function") {
                target[key] = function() { return value.apply(target, arguments); };
            } else if (value && typeof value === "object") {
                target[key] = namespace(value);
            }
        });
        return target;
    }
    var target = namespace(__operitCanonicalTools);
    function port(name) {
        return {execute: function(interfaceInfo, args) {
            if (arguments.length > 2) return Promise.reject(new TypeError("Hardware calls take interfaceInfo and args; select the node with Tools.device"));
            return __operitDeviceToolCall(nodeId, "default", name,
                {interface_info: interfaceInfo, args: args === undefined ? {} : args}, context.options, "port");
        }};
    }
    target.transport = __operitDeviceTransport(nodeId, context.options);
    target.Edge = port("edge_execute");
    target.Io = port("io_execute");
    target.call = function(name, params) {
        if (typeof name !== "string" || !name.trim()) return Promise.reject(new TypeError("Tool name is required"));
        return __operitDeviceToolCall(nodeId, "default", name, params, context.options);
    };
    target.cancel = function() {
        cancellations.slice().forEach(function(cancel) { cancel(); });
    };
    target.catalog = function() {
        return __operitDeviceToolCall(nodeId, "default", "__device_catalog", {}, context.options, "catalog");
    };
    return target;
}).build("Tools.device");

// Hardware is exposed on explicit device views.
delete Tools.Edge;

/** Creates one generic duplex facade over the selected node's Watch and Push endpoints. */
function __operitDeviceTransport(nodeId, options) {
    function pack(value) {
        var ancestors = [], nodes = 0;
        function copy(value, depth) {
            if (++nodes > 1000000 || depth > 128) throw new TypeError('Transport value exceeds depth/node limit');
            if (value === null) return {kind:'null'};
            if (typeof value === 'boolean') return {kind:'bool',value:value};
            if (typeof value === 'string') return {kind:'string',value:value};
            if (typeof value === 'number' && Number.isFinite(value)) return {kind:'number',value:value};
            if (value instanceof Uint8Array) return {kind:'bytes',value:Array.from(value)};
            if (value instanceof ArrayBuffer) return {kind:'bytes',value:Array.from(new Uint8Array(value))};
            if (!value || typeof value !== 'object' || (!Array.isArray(value) && Object.getPrototypeOf(value) !== Object.prototype && Object.getPrototypeOf(value) !== null)) throw new TypeError('Unsupported transport value');
            if (ancestors.indexOf(value) !== -1) throw new TypeError('Cyclic transport value');
            ancestors.push(value);
            var array = Array.isArray(value), result = array ? [] : {};
            Reflect.ownKeys(value).forEach(function(key) {
                if (array && key === 'length') return;
                var property = Object.getOwnPropertyDescriptor(value, key);
                if (typeof key !== 'string' || !property.enumerable || !Object.prototype.hasOwnProperty.call(property,'value')) throw new TypeError('Transport values require data properties');
                Object.defineProperty(result,key,{value:copy(property.value,depth+1),enumerable:true});
            });
            if (array && Object.keys(result).length !== value.length) throw new TypeError('Sparse transport array');
            ancestors.pop();
            return {kind:array?'list':'map',value:result};
        }
        return copy(value,0);
    }
    function unpack(data) {
        switch (data.kind) {
            case 'null': return null;
            case 'bytes': return new Uint8Array(data.value);
            case 'list': return data.value.map(unpack);
            case 'map': {
                var value = {};
                Object.keys(data.value).forEach(function(key) {
                    Object.defineProperty(value,key,{value:unpack(data.value[key]),enumerable:true,writable:true,configurable:true});
                });
                return value;
            }
            case 'bool': case 'number': case 'string': return data.value;
            default: throw new TypeError('Invalid transport reply');
        }
    }
    function invoke(params, cleanup) {
        return __operitDeviceToolCall(nodeId,'default','__device_transport',params,
            cleanup ? {timeoutMs:options.timeoutMs} : options,'transport');
    }
    return {
        call: async function(target, method, args) {
            return unpack(await invoke({op:'call',target:target,method:method,args:pack(args===undefined?{}:args)}));
        },
        open: async function(endpoint) {
            if (!endpoint || typeof endpoint !== 'object') throw new TypeError('Transport endpoint is required');
            var result = await invoke({op:'open',target:endpoint.target,method:endpoint.method,
                property:endpoint.property,args:pack(endpoint.args===undefined?{}:endpoint.args)});
            var handle=result.handle, closed=false, closing=null, reading=false, writing=false, finished=false;
            function dispose() {
                if (!closing) {
                    closed=true;
                    if (options.signal) options.signal.removeEventListener('abort',abort);
                    var index=options.__deviceCancellations.indexOf(abort);
                    if(index!==-1) options.__deviceCancellations.splice(index,1);
                    closing=invoke({op:'close',handle:handle},true);
                }
                return closing;
            }
            function abort() { dispose().catch(function() {}); }
            options.__deviceCancellations.push(abort);
            if (options.signal) {
                options.signal.addEventListener('abort',abort,{once:true});
                if (options.signal.aborted) {await dispose();throw new Error('Device channel cancelled');}
            }
            return {
                [Symbol.asyncIterator]:function() {return this;},
                send:async function(value) {
                    if(closed||finished) throw new Error('Device channel input is closed');
                    if(writing) throw new Error('Only one send may be pending');
                    writing=true;
                    try {await invoke({op:'send',handle:handle,value:pack(value)});}
                    catch(error) {await dispose().catch(function() {});throw error;}
                    finally {writing=false;}
                },
                finish:async function() {
                    if(closed||finished) return;
                    if(writing) throw new Error('Wait for the pending send before finishing');
                    writing=true;finished=true;
                    try {await invoke({op:'finish',handle:handle});}
                    catch(error) {await dispose().catch(function() {});throw error;}
                    finally {writing=false;}
                },
                next:async function() {
                    if(closed) return {done:true,value:undefined};
                    if(reading) throw new Error('Only one receive may be pending');
                    reading=true;
                    try {
                        var item=await invoke({op:'next',handle:handle});
                        if(closed||item.done) {await dispose();return {done:true,value:undefined};}
                        return {done:false,value:{kind:item.kind,value:unpack(item.value)}};
                    } catch(error) {await dispose().catch(function() {});throw error;}
                    finally {reading=false;}
                },
                close:dispose,
                return:async function() {await dispose();return {done:true,value:undefined};},
                throw:async function(error) {await dispose();throw error;}
            };
        }
    };
}

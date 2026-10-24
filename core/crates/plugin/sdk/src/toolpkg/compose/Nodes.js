/// Exposes logical node handles and dynamic construction through the program protocol.
var OperitComposeNodes = function () {
  var references = new WeakMap();
  var modifierOperations = new WeakMap();
  /// Reads a private logical address without evaluating public node getters.
  function resolve(value) { return references.get(value); }
  /// Reads one local view only when plugin code explicitly observes a node.
  function view(entry, node) {
    var cached = entry.views.get(node);
    if (!cached || cached.version !== entry.version) {
      cached = { version: entry.version, value: entry.owner.publicValue(entry.owner.project(entry, node), entry), proxies: new WeakMap() };
      entry.views.set(node, cached);
    }
    return cached;
  }
  /// Observes only plugin-visible containers and emits a local field edit after each mutation.
  function observed(entry, node, cached, field, value) {
    if (value === null || typeof value !== 'object' || references.has(value)) return value;
    if (cached.proxies.has(value)) return cached.proxies.get(value);
    var proxy = new Proxy(value, {
      /// Keeps nested aliases stable within an explicitly materialized view.
      get: function (target, key) { return observed(entry, node, cached, field, Reflect.get(target, key)); },
      /// Records an ordinary plugin assignment through the same transaction writer.
      set: function (target, key, next) {
        if (Object.is(Reflect.get(target, key), next)) return true;
        Reflect.set(target, key, next);
        entry.owner.patch(entry, node, field, cached.value[field]);
        cached.version = entry.version;
        return true;
      },
      /// Preserves property deletion on a logical public node.
      deleteProperty: function (target, key) {
        if (!Reflect.has(target, key)) return true;
        Reflect.deleteProperty(target, key);
        entry.owner.patch(entry, node, field, cached.value[field]);
        cached.version = entry.version;
        return true;
      },
      /// Preserves descriptor writes without scanning unrelated instances.
      defineProperty: function (target, key, descriptor) {
        var previous = Reflect.getOwnPropertyDescriptor(target, key);
        if (previous && previous.value === descriptor.value && previous.enumerable === descriptor.enumerable && previous.configurable === descriptor.configurable && previous.writable === descriptor.writable && previous.get === descriptor.get && previous.set === descriptor.set) return true;
        Reflect.defineProperty(target, key, descriptor);
        entry.owner.patch(entry, node, field, cached.value[field]);
        cached.version = entry.version;
        return true;
      }
    });
    entry.owner.alias(value, proxy);
    cached.proxies.set(value, proxy);
    return proxy;
  }
  /// Creates one public handle per observed local instruction, not per internal layout node.
  function reference(entry, node) {
    if (entry.handles.has(node)) return entry.handles.get(node);
    var target = { __composeNode: true };
    ['type', 'props', 'children', 'slots'].forEach(
    /// Projects a field lazily and translates explicit assignments into local operations.
    function (field) {
      Object.defineProperty(target, field, { enumerable: true, configurable: false,
        /// Reads the current logical instruction through its latest parameters.
        get: function () { var cached = view(entry, node); return observed(entry, node, cached, field, cached.value[field]); },
        /// Replaces one field without rebuilding a declarative tree.
        set: function (next) {
          entry.owner.patch(entry, node, field, next);
        } });
    });
    references.set(target, { entry: entry, node: node });
    entry.handles.set(node, target);
    return target;
  }
  /// Builds runtime-defined UI with the ordinary dynamic-node program instruction.
  function build(runtime, type, props, children) {
    var program = { id: '@dynamic:' + type + ':' + (children !== undefined), root: 0,
      nodes: [{ type: String(type), props: [1, 0], children: [1, 1], explicitChildren: children !== undefined }] };
    return runtime.composition.instance({}, program, [props, children]);
  }
  /// Provides arbitrary UI type access without storing a second node registry.
  function registry(runtime) {
    var factories = new Map();
    return new Proxy({}, {
      /// Creates each public type constructor once in its owning context.
      get: function (_target, type) {
        if (typeof type !== 'string') return undefined;
        if (!factories.has(type)) factories.set(type,
          /// Emits a dynamic instance with the same protocol as compiled UI.
          function (props, children) { return build(runtime, type, props, children); });
        return factories.get(type);
      }
    });
  }
  /// Serializes explicit host payloads without manufacturing UI ownership or callbacks.
  function serialize(value, runtime, seen) {
    if (value === undefined || value === null) return value;
    if (typeof value === 'function') return runtime.actions.reference(runtime.actions.register(value));
    if (typeof value !== 'object') return value;
    if (seen.indexOf(value) !== -1) throw new TypeError('Compose payload must not contain cycles');
    seen.push(value);
    var result = Array.isArray(value) ? [] : {};
    Object.keys(value).forEach(
    /// Copies each payload property once for the host boundary.
    function (key) { if (key !== '__composeNode') result[key] = serialize(value[key], runtime, seen); });
    seen.pop();
    return result;
  }
  /// Shares modifier property dispatch across every operation in every list row.
  var modifierObserver = {
    /// Captures each method's target so extracted modifier methods keep their original chain.
    get: function (target, prop) {
      if (prop === '__modifierOps') return target.__modifierOps;
      if (prop === 'toJSON') {
        /// Serializes only the operation list owned by this chain.
        return function () { return { __modifierOps: target.__modifierOps }; };
      }
      if (typeof prop !== 'string') return undefined;
      var operations = this.operations;
      /// Appends one operation without copying the newly allocated chain a second time.
      return function () {
        var ops = operations.concat([{
          name: prop,
          args: Array.prototype.slice.call(arguments)
        }]);
        return modifierChain(ops);
      };
    }
  };

  /// Wraps an internally owned operation list without cloning it again.
  function modifierChain(ops) {
    var handler = Object.create(modifierObserver);
    handler.operations = ops;
    var proxy = new Proxy({ __modifierOps: ops }, handler);
    modifierOperations.set(proxy, ops);
    return proxy;
  }

  /// Captures the fluent chain's actual base operations before evaluating compiled operation arguments.
  function modifierBase(modifier) {
    var operations = modifierOperations.get(modifier);
    if (!operations) throw new TypeError('Compose modifiers must be created by the SDK');
    return operations;
  }

  /// Copies externally supplied operations once and keeps fluent chains independently owned.
  function createModifierProxy(ops) {
    return modifierChain(Array.isArray(ops) ? ops.slice() : []);
  }
  return { resolve: resolve, reference: reference, build: build, registry: registry,
    modifier: createModifierProxy, modifierBase: modifierBase, serialize: serialize };
}();

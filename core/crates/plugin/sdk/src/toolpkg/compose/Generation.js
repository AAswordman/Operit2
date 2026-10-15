/// Retains generation scopes separately from transport records and updates captured action closures.
var OperitComposeGeneration = function () {
  var contexts = new WeakMap();
  var nodes = new WeakMap();
  var actionBinding = false;

  /// Identifies nodes whose construction inputs are owned by the generation registry.
  function owns(node) {
    return nodes.has(node);
  }

  /// Attaches a retained node owner without retaining detached node graphs globally.
  function watch(node, listener) {
    var listeners = record(node).listeners;
    listeners.add(listener);
    /// Releases one retained occurrence when it leaves the composition.
    return function () {
      listeners.delete(listener);
    };
  }

  /// Resolves the factory-owned record without reading through public mutation proxies.
  function record(node) {
    var entry = nodes.get(node);
    if (!entry) throw new Error('Compose node must be created by the DSL node factory');
    return entry;
  }

  /// Supplies retained reconciliation with the observed node's underlying containers.
  function read(node) {
    return record(node).value;
  }

  /// Updates the internal callback reference without treating ownership binding as a plugin mutation.
  function rebind(reference, id) {
    actionBinding = true;
    try {
      reference.__actionId = id;
    } finally {
      actionBinding = false;
    }
  }

  /// Invalidates a cached transport record when a plugin mutates its public node object.
  function observeNode(node) {
    var seen = new WeakMap();
    var root;
    /// Publishes the exact mutation path without invalidating unrelated retained occurrences.
    function changed(path) {
      if (actionBinding) return;
      record(root).listeners.forEach(
      /// Enqueues the affected retained occurrence instead of walking its ancestors.
      function (listener) {
        listener(path);
      });
    }
    /// Wraps owned containers while retaining child node identity and public object shape.
    function observe(value, path) {
      if (value === null || typeof value !== 'object') return value;
      if (value !== node && value.__composeNode === true) return value;
      if (seen.has(value)) return seen.get(value);
      var proxy = new Proxy(value, {
        /// Reads nested containers through the same mutation observer.
        get(target, key) {
          var next = Reflect.get(target, key);
          if (next === null || typeof next !== 'object') return next;
          if (next !== node && next.__composeNode === true) return next;
          if (seen.has(next)) return seen.get(next);
          return observe(next, path.concat(key));
        },
        /// Invalidates record caching before an explicit plugin assignment.
        set(target, key, next) {
          var previous = Reflect.get(target, key);
          var changedKey = path.length === 0 && key === 'props' && previous && previous.key !== next.key;
          if (Object.is(previous, next)) return true;
          if (changedKey) changed(['props', 'key']);else changed(path.concat(key));
          return Reflect.set(target, key, next);
        },
        /// Invalidates record caching before a plugin removes a property.
        deleteProperty(target, key) {
          if (!Reflect.has(target, key)) return true;
          changed(path.concat(key));
          return Reflect.deleteProperty(target, key);
        },
        /// Invalidates record caching when a plugin defines a property descriptor.
        defineProperty(target, key, descriptor) {
          var previous = Reflect.getOwnPropertyDescriptor(target, key);
          if (previous && previous.configurable === descriptor.configurable && previous.enumerable === descriptor.enumerable && previous.writable === descriptor.writable && previous.value === descriptor.value && previous.get === descriptor.get && previous.set === descriptor.set) return true;
          changed(path.concat(key));
          return Reflect.defineProperty(target, key, descriptor);
        }
      });
      seen.set(value, proxy);
      OperitComposeActions.inherit(value, proxy);
      return proxy;
    }
    root = observe(node, []);
    nodes.set(root, { value: node, listeners: new Set() });
    return root;
  }
  function ownedContext(ctx) {
    var state = contexts.get(ctx);
    if (!state) throw new TypeError('Compose expressions require the original SDK context');
    return state;
  }

  /// Creates generation state owned by exactly one public DSL context.
  function attach(ctx) {
    contexts.set(ctx, {
      scopes: new Map(),
      counts: new Map(),
      live: new Set()
    });
  }

  /// Starts one render traversal without discarding retained expression results.
  function begin(ctx) {
    var state = ownedContext(ctx);
    state.counts.clear();
    state.live.clear();
  }

  /// Releases scopes belonging to branches and loop entries that left the current composition.
  function end(ctx) {
    var state = ownedContext(ctx);
    state.scopes.forEach(function (value, key) {
      if (!state.live.has(key)) state.scopes.delete(key);
    });
  }

  /// Selects a call-site occurrence while preserving independent loop instances.
  function scope(ctx, site) {
    var state = ownedContext(ctx);
    var index = state.counts.get(site) || 0;
    state.counts.set(site, index + 1);
    var key = site + '/' + index;
    state.live.add(key);
    if (!state.scopes.has(key)) state.scopes.set(key, {
      callbacks: new Map(),
      initialized: false
    });
    return state.scopes.get(key);
  }

  /// Retains immutable argument snapshots and action trampolines with replaceable captured closures.
  function snapshot(value, slot, callbacks, seen) {
    if (typeof value === 'function') {
      if (!callbacks.has(slot)) {
        var cell = {
          handler: value
        };
        /// Invokes the latest closure while keeping the public callback identity stable.
        cell.invoke = function () {
          return cell.handler.apply(this, arguments);
        };
        callbacks.set(slot, cell);
      }
      var current = callbacks.get(slot);
      current.handler = value;
      return current.invoke;
    }
    if (value === null || typeof value !== 'object') return value;
    if (value.__composeNode === true) return value;
    if (seen.has(value)) throw new TypeError('Compose arguments must not contain cycles');
    seen.add(value);
    var result = Array.isArray(value) ? [] : {};
    var keys = Object.keys(value);
    for (var index = 0; index < keys.length; index += 1) {
      var key = keys[index];
      var child = value[key];
      result[key] = child === null || typeof child !== 'object' && typeof child !== 'function'
        ? child : snapshot(child, slot + '/' + key.length + ':' + key, callbacks, seen);
    }
    seen.delete(value);
    return result;
  }

  /// Builds a node only when its normalized construction inputs actually changed.
  function evaluate(entry, args, factory) {
    var values = snapshot(args, '', entry.callbacks, new Set());
    if (!entry.initialized || !OperitComposeRetained.equal(entry.args, values)) {
      var next = factory.apply(undefined, values);
      var nextValue = read(next);
      var previous = entry.initialized ? read(entry.value) : null;
      if (previous && previous.type === nextValue.type && previous.props.key === nextValue.props.key) {
        // Assign raw construction containers through the public observer once, avoiding nested proxies.
        if (!OperitComposeRetained.equal(previous.props, nextValue.props)) entry.value.props = nextValue.props;
        if (!OperitComposeRetained.equal(previous.children, nextValue.children)) entry.value.children = nextValue.children;
        if (!OperitComposeRetained.equal(previous.slots, nextValue.slots)) {
          if (Object.prototype.hasOwnProperty.call(nextValue, 'slots')) entry.value.slots = nextValue.slots;else delete entry.value.slots;
        }
      } else entry.value = next;
      entry.args = values;
      entry.initialized = true;
    }
    return entry.value;
  }

  /// Evaluates effectful arguments in source order while retaining unchanged node construction.
  function node(ctx, site, args, factory) {
    return evaluate(scope(ctx, site), args, factory);
  }

  /// Skips a pure leaf expression before argument construction when its lexical inputs are unchanged.
  function leaf(ctx, site, deps, argumentsFactory, factory) {
    var entry = scope(ctx, site);
    var primitiveInputs = deps.every(function (value) {
      return value === null || typeof value !== 'object' && typeof value !== 'function';
    });
    if (primitiveInputs && entry.initialized && OperitComposeRetained.equal(entry.deps, deps)) return entry.value;
    entry.deps = primitiveInputs ? deps.slice() : snapshot(deps, 'deps', new Map(), new Set());
    return evaluate(entry, argumentsFactory(), factory);
  }
  return {
    evaluate: evaluate,
    observeNode: observeNode,
    read: read,
    watch: watch,
    rebind: rebind,
    owns: owns,
    attach: attach,
    begin: begin,
    end: end,
    node: node,
    leaf: leaf
  };
}();

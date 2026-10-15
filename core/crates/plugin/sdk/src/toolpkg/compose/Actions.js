/// Owns callback closures, transport IDs, and registration cleanup for one context.
var OperitComposeActions = function () {
  var sources = new WeakMap();

  /// Preserves ownership when generation observes the same callback through a proxy.
  function inherit(reference, proxy) {
    var source = sources.get(reference);
    if (source) sources.set(proxy, source);
  }
  function create() {
    var store = {};
    var pending = new Set();
    var counter = 0;
    function allocate() {
      return '__action_' + ++counter;
    }
    function set(id, handler) {
      if (typeof handler !== 'function') throw new TypeError('Compose callback must be a function');
      store[id] = handler;
    }
    function register(handler) {
      var id = allocate();
      set(id, handler);
      pending.add(id);
      return id;
    }
    function remember(reference, handler) {
      sources.set(reference, {
        store: store,
        id: reference.__actionId,
        handler: handler
      });
    }
    function reference(handler) {
      var value = {
        __actionId: register(handler)
      };
      remember(value, handler);
      return value;
    }
    function bind(value, previousId) {
      var source = sources.get(value);
      if (source && source.store !== store) throw new Error('Compose callback belongs to another context');
      var handler = source && source.id === value.__actionId ? source.handler : store[value.__actionId];
      if (typeof handler !== 'function') throw new Error('compose callback is not registered: ' + value.__actionId);
      var id = previousId || allocate();
      set(id, handler);
      OperitComposeGeneration.rebind(value, id);
      if (source) source.id = id;else remember(value, handler);
      return id;
    }
    function release(id) {
      delete store[id];
    }
    function finish(deletes) {
      deletes.forEach(release);
      pending.clear();
    }
    return {
      store: store,
      allocate: allocate,
      set: set,
      register: register,
      reference: reference,
      bind: bind,
      release: release,
      pendingDeletes: function () {
        return new Set(pending);
      },
      finish: finish,
      invoke: function (id, payload) {
        var handler = store[String(id || '').trim()];
        if (typeof handler !== 'function') throw new Error('compose action not found: ' + id);
        return handler(payload);
      }
    };
  }
  return {
    create: create,
    inherit: inherit
  };
}();

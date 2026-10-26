/// Owns one closure per logical instance event slot; no construction trampolines are created.
var OperitComposeActions = function () {
  var owners = new WeakMap();
  var viewportHandlers = new WeakSet();
  /// Marks SDK-owned viewport events without changing user callback behavior.
  function viewport(handler) { viewportHandlers.add(handler); return handler; }
  /// Creates the only event registry for a composition context.
  function create() {
    var store = Object.create(null);
    var sequence = 0;
    var identity = {};
    /// Creates a context-owned reference whose ownership is preserved by public views.
    function reference(id, event) { var value = { __actionId: id }; owners.set(value, { context: identity, event: event }); return value; }
    /// Requires the original callback context even when opaque transport IDs happen to collide.
    function resolve(value) {
      var owner = owners.get(value);
      if (owner && owner.context !== identity) throw new Error('Compose callback belongs to another context');
      return owner && owner.event ? owner.event.handler : handler(value.__actionId);
    }
    /// Carries ownership through an explicitly observed property proxy.
    function alias(value, proxy) { if (owners.has(value)) owners.set(proxy, owners.get(value)); }
    /// Releases an explicitly owned controller event independently of UI instances.
    function unregister(id) { delete store[id]; }
    /// Allocates an opaque address in this context.
    function allocate() { return '__action_' + ++sequence; }
    /// Registers a handler at a known address.
    function set(id, handler) {
      if (typeof handler !== 'function') throw new TypeError('Compose callback must be a function');
      store[id] = handler;
    }
    /// Registers an explicit SDK event outside a compiled layout.
    function register(handler) { var id = allocate(); set(id, handler); return id; }
    /// Binds or refreshes one instance-owned slot directly to its latest closure.
    function bind(instance, slot, handler) {
      var event = instance.events.get(slot);
      if (!event) { event = { handler: handler, reference: null }; event.reference = reference(allocate(), event); instance.events.set(slot, event); }
      event.handler = handler;
      set(event.reference.__actionId, handler);
      return event.reference;
    }
    /// Removes obsolete logical slots after their replacement has been validated.
    function retain(instance, ids) {
      instance.events.forEach(
      /// Releases only callbacks absent from the effective instance snapshot.
      function (event, slot) {
        if (!ids.has(event.reference.__actionId)) { delete store[event.reference.__actionId]; instance.events.delete(slot); }
      });
    }
    /// Expires transport registrations without retaining detached instances in the registry.
    function release(instance) {
      instance.events.forEach(
      /// Keeps the cached source's closures while releasing every live address.
      function (event) { delete store[event.reference.__actionId]; });
    }
    /// Gives a cached source fresh event addresses when it is declared again.
    function revive(instance) {
      var replacements = new Map();
      instance.events.forEach(
      /// Allocates one replacement for each previously expired logical slot.
      function (event) {
        var previous = event.reference;
        delete store[previous.__actionId];
        event.reference = reference(register(event.handler), event);
        replacements.set(previous, event.reference);
      });
      return replacements;
    }
    /// Requires a current event instead of silently accepting a removed registration.
    function handler(id) {
      var result = store[String(id).trim()];
      if (typeof result !== 'function') throw new Error('compose action not found: ' + id);
      return result;
    }
    return { reference: reference, resolve: resolve, alias: alias, unregister: unregister, store: store, allocate: allocate, set: set, register: register, bind: bind, retain: retain, release: release, revive: revive,
      /// Dispatches an event to its current closure.
      invoke: function (id, payload) { return handler(id)(payload); },
      /// Identifies the SDK viewport event kind.
      isViewport: function (id) { return viewportHandlers.has(handler(id)); } };
  }
  return { create: create, viewport: viewport };
}();

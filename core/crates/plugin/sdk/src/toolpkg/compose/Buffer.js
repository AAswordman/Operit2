/// Owns the single indexed binary representation produced during Compose construction.
var OperitComposeBuffer = function () {
  var nextEpoch = 0;
  /// Allocates a transaction arena whose payload is written before ownership is submitted.
  function create() {
    var buffer = new ArrayBuffer(4096), bytes = new Uint8Array(buffer), view = new DataView(buffer);
    var used = 32, strings = new Map(), stringOrder = [], sealed = false;
    var api = { epoch: ++nextEpoch, last: 0 };
    /// Reserves aligned storage, charging growth to construction rather than submission.
    function allocate(length) {
      if (sealed) throw new Error('Compose buffer has already been sealed');
      var start = used, end = (used + length + 7) & ~7;
      if (end < used || end > 0x3fffffff) throw new RangeError('Compose buffer exceeds its address range');
      if (end > bytes.length) {
        var capacity = bytes.length;
        while (capacity < end) capacity *= 2;
        var next = new ArrayBuffer(capacity), target = new Uint8Array(next);
        target.set(bytes.subarray(0, used));
        buffer = next; bytes = target; view = new DataView(buffer);
      }
      used = end;
      return start;
    }
    /// Writes one fixed-size value descriptor.
    function descriptor(tag, length, data) {
      var offset = allocate(16);
      view.setUint32(offset, tag, true); view.setUint32(offset + 4, length, true); view.setUint32(offset + 8, data, true);
      api.last = offset;
      return offset;
    }
    /// Interns a UTF-16 string once within its owning transaction.
    function string(value) {
      if (strings.has(value)) { api.last = strings.get(value); return api.last; }
      var offset = descriptor(4, value.length, 0), data = allocate(value.length * 2);
      view.setUint32(offset + 8, data, true);
      for (var index = 0; index < value.length; index++) view.setUint16(data + index * 2, value.charCodeAt(index), true);
      strings.set(value, offset); stringOrder.push(value); api.last = offset;
      return offset;
    }
    /// Writes a scalar directly without crossing the Host boundary.
    function scalar(value) {
      if (value === null) return descriptor(0, 0, 0);
      if (value === false) return descriptor(1, 0, 0);
      if (value === true) return descriptor(2, 0, 0);
      if (typeof value === 'string') return string(value);
      if (typeof value === 'number') {
        if (!Number.isFinite(value)) return descriptor(0, 0, 0);
        var offset = descriptor(3, 0, 0); view.setFloat64(offset + 8, value, true); return offset;
      }
      if (value === undefined) return descriptor(7, 0, 0);
      throw new TypeError('Unsupported Compose parameter type: ' + typeof value);
    }
    /// Writes a container from child addresses already produced by normalization.
    function container(array, keys, addresses) {
      var offset = descriptor(array ? 5 : 6, addresses.length, 0);
      var names = array ? null : keys.map(string);
      var data = allocate(addresses.length * (array ? 4 : 8));
      view.setUint32(offset + 8, data, true);
      for (var index = 0; index < addresses.length; index++) {
        view.setUint32(data + index * (array ? 4 : 8), array ? addresses[index] : names[index], true);
        if (!array) view.setUint32(data + index * 8 + 4, addresses[index], true);
      }
      api.last = offset; return offset;
    }
    /// Writes an action address without allocating an intermediate protocol object.
    function action(id) { var name = string(id); return descriptor(8, 0, name); }
    /// Writes a logical node address independently of its Flutter mount count.
    function reference(id, node) { var name = string(id); return descriptor(9, node, name); }
    /// Writes constants and explicit edits directly into the same arena.
    function write(value, ancestors, members) {
      if (value === null || typeof value !== 'object') return scalar(value);
      if (value.__composeUndefined === true) return scalar(undefined);
      if (Object.prototype.hasOwnProperty.call(value, '__actionId')) return action(value.__actionId);
      if (Object.prototype.hasOwnProperty.call(value, '__composeRef')) return members && !members.has(value.__composeRef[0]) ? scalar(undefined) : reference(value.__composeRef[0], value.__composeRef[1]);
      ancestors = ancestors || [];
      if (ancestors.length >= 128 || ancestors.indexOf(value) !== -1) throw new TypeError('Invalid cyclic or excessively deep Compose value');
      ancestors.push(value);
      var array = Array.isArray(value), keys = array ? null : Object.keys(value), addresses = [];
      for (var index = 0; index < (array ? value.length : keys.length); index++) addresses.push(write(value[array ? index : keys[index]], ancestors, members));
      ancestors.pop(); return container(array, keys, addresses);
    }
    /// Discards tentative writes for an unchanged source without retaining dead string addresses.
    function rewind(position) {
      while (stringOrder.length && strings.get(stringOrder[stringOrder.length - 1]) >= position) strings.delete(stringOrder.pop());
      used = position;
    }
    /// Writes application results and navigation, then seals the existing layout allocation without copying it.
    function seal(phase, update, actionResult, navigation) {
      if (phase !== 'intermediate' && phase !== 'final') throw new Error('Invalid Compose response phase');
      var result = actionResult === undefined ? 0 : write(actionResult);
      var commands = write(navigation);
      view.setUint32(0, 0x31425043, true); view.setUint32(4, 1, true);
      view.setUint32(8, used, true); view.setUint32(12, phase === 'final' ? 1 : 0, true);
      view.setUint32(16, update, true); view.setUint32(20, result, true); view.setUint32(24, commands, true);
      sealed = true;
      return new Uint8Array(buffer, 0, used);
    }
    api.scalar = scalar; api.string = string; api.container = container; api.action = action; api.reference = reference;
    api.write = write; api.seal = seal; api.rewind = rewind;
    /// Captures a source-local construction checkpoint without inspecting prior values.
    api.checkpoint = function () { return used; };
    return api;
  }
  return { create: create };
}();

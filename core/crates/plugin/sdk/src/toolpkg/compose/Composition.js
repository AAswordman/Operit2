/// Writes shared layout programs and dynamic instance transactions without building a JS node tree.
var OperitComposeComposition = function () {
  var runtimes = new WeakMap();
  /// Compares boundary values that have already been normalized once.
  function equal(left, right) {
    if (Object.is(left, right)) return true;
    if (left === null || right === null || typeof left !== 'object' || typeof right !== 'object' || Array.isArray(left) !== Array.isArray(right)) return false;
    var keys = Object.keys(left);
    if (keys.length !== Object.keys(right).length) return false;
    for (var index = 0; index < keys.length; index += 1) {
      var key = keys[index];
      if (!Object.prototype.hasOwnProperty.call(right, key) || !equal(left[key], right[key])) return false;
    }
    return true;
  }
  /// Resolves a compiler call against the exact session that owns its context.
  function evaluate(ctx, site, program, values) {
    var runtime = runtimes.get(ctx);
    if (!runtime) throw new Error('Compose expressions require the original SDK context');
    return runtime.composition.instance(OperitComposeReactive.scope(ctx, site), program, values);
  }
  /// Owns source instances and their pending operations, independently of Flutter mount identity.
  function create(runtime) {
    var programs = new Map(), announced = new Set(), pendingPrograms = new Map();
    var live = new Map(), dirty = new Set(), staged = new Set();
    var sequence = 0, revision = 0, epoch = 0;
    var wire = OperitComposeBuffer.create(), programOffsets = new Map();
    var stats = { instances: 0, normalizedValues: 0, projections: 0, releasedInstances: 0, commits: 0 };
    /// Installs immutable bytecode once per session and module version.
    function install(program) {
      if (!programs.has(program.id)) programs.set(program.id, program);
      if (!announced.has(program.id) && !pendingPrograms.has(program.id)) { pendingPrograms.set(program.id, program); programOffsets.set(program.id, wire.write(program)); }
    }
    /// Normalizes callbacks and references while writing the final parameter storage in the same traversal.
    function normalize(value, entry, path, seen, refs, events) {
      stats.normalizedValues += 1;
      if (typeof value === 'function') { var event = runtime.actions.bind(entry, path, value); events.add(event.__actionId); wire.action(event.__actionId); return event; }
      if (value === null || typeof value !== 'object') { wire.scalar(value); return value === undefined ? { __composeUndefined: true } : value; }
      var reference = OperitComposeNodes.resolve(value);
      if (reference) {
        if (reference.entry.owner !== api) throw new Error('Compose node belongs to another context');
        refs.add(reference.entry); wire.reference(reference.entry.id, reference.node);
        return { __composeRef: [reference.entry.id, reference.node] };
      }
      if (Object.prototype.hasOwnProperty.call(value, '__actionId')) {
        var handler = runtime.actions.resolve(value);
        if (typeof handler !== 'function') throw new Error('compose callback is not registered: ' + value.__actionId);
        var bound = runtime.actions.bind(entry, path, handler);
        events.add(bound.__actionId); wire.action(bound.__actionId); return bound;
      }
      if (seen.length >= 128 || seen.indexOf(value) !== -1) throw new TypeError('Compose arguments must not contain cycles or exceed the depth limit');
      seen.push(value);
      var array = Array.isArray(value), result = array ? [] : {}, keys = Object.keys(value), names = [], addresses = [];
      if (array) for (var hole = 0; hole < value.length; hole++) addresses.push(0);
      for (var index = 0; index < keys.length; index += 1) {
        var key = keys[index];
        if (key === '__composeNode') continue;
        var child = normalize(value[key], entry, path + '/' + key.length + ':' + key, seen, refs, events);
        Object.defineProperty(result, key, {value:child, enumerable:true, configurable:true, writable:true});
        if (array) { if (/^(0|[1-9][0-9]*)$/.test(key) && Number(key) < value.length) addresses[Number(key)] = wire.last; }
        else { names.push(key); addresses.push(wire.last); }
      }
      if (array) for (var empty = 0; empty < addresses.length; empty++) if (addresses[empty] === 0) addresses[empty] = wire.scalar(null);
      wire.container(array, names, addresses); seen.pop(); return result;
    }
    /// Resolves a local operand only for explicit plugin reads and lifecycle metadata.
    function operand(entry, code) {
      switch (code[0]) {
        case 0: return code[1];
        case 1: return entry.values[code[1]];
        case 2:
          var object = {};
          for (var field = 0; field < code[1].length; field += 1) object[code[1][field]] = operand(entry, code[2][field]);
          return object;
        case 3: return code[1].map(
          /// Resolves an explicit array view without creating retained node objects.
          function (member) { return operand(entry, member); });
        case 4: return { __composeRef: [entry.id, code[1]] };
        case 5: return operand(entry, code[1]).concat(operand(entry, code[2]));
        default: throw new Error('Unknown Compose operand: ' + code[0]);
      }
    }
    /// Collects only node references from a logical child value.
    function children(value, output) {
      if (Array.isArray(value)) for (var index = 0; index < value.length; index += 1) children(value[index], output);
      else if (value && typeof value === 'object' && Object.prototype.hasOwnProperty.call(value, '__composeRef')) output.push(value);
      return output;
    }
    /// Projects a single instruction on demand; construction and commit never call this function.
    function project(entry, node) {
      stats.projections += 1;
      var instruction = entry.program.nodes[node];
      if (!instruction) throw new Error('Unknown Compose instruction: ' + node);
      var raw = operand(entry, instruction.props);
      var props = {}, slots = {}, content = operand(entry, instruction.children);
      if (raw && typeof raw === 'object' && !Array.isArray(raw)) {
        var keys = Object.keys(raw);
        for (var index = 0; index < keys.length; index += 1) {
          var key = keys[index], value = raw[key];
          if (key === 'content' && (!instruction.explicitChildren || content && content.__composeUndefined === true)) { content = value; continue; }
          var members = children(value, []);
          if (members.length > 0) slots[key] = members;
          else props[key] = value;
        }
      }
      var result = { type: instruction.type, props: props, children: children(content, []), slots: slots };
      var patch = entry.patches.get(node);
      if (patch) Object.keys(patch).forEach(
      /// Applies only explicit public-node mutations.
      function (field) { result[field] = patch[field]; });
      return result;
    }
    /// Resolves encoded references into public logical handles during explicit reads.
    function publicValue(value, entry) {
      if (value === null || typeof value !== 'object') return value;
      if (value.__composeUndefined === true) return undefined;
      if (Object.prototype.hasOwnProperty.call(value, '__actionId')) { var callback = runtime.actions.reference(value.__actionId); runtime.actions.alias(value, callback); return callback; }
      if (value.__composeRef) {
        var target = value.__composeRef[0] === entry.id ? entry : null;
        entry.refs.forEach(
        /// Resolves cached direct children even while both sources are detached.
        function (candidate) { if (candidate.id === value.__composeRef[0]) target = candidate; });
        if (!target) target = find(value.__composeRef[0]);
        return OperitComposeNodes.reference(target, value.__composeRef[1]);
      }
      var result = Array.isArray(value) ? [] : {};
      Object.keys(value).forEach(
      /// Preserves ordinary property shape for plugin-visible views.
      function (key) { result[key] = publicValue(value[key], entry); });
      return result;
    }
    /// Requires a source that is either being constructed or retained by the current session.
    function find(id) {
      var entry = live.get(id);
      if (!entry) staged.forEach(
      /// Resolves sources created before their first transaction is submitted.
      function (candidate) { if (candidate.id === id) entry = candidate; });
      if (!entry) throw new Error('Unknown Compose instance: ' + id);
      return entry;
    }
    /// Declares a new source instance or reuses its compiler-owned source slot.
    function instance(slot, program, values) {
      install(program);
      var entry = slot.entry;
      if (!entry || entry.program.id !== program.id) {
        entry = { owner: api, id: 'i:' + ++sequence, program: program, values: [], events: new Map(), refs: new Set(), patches: new Map(), handles: new Map(), views: new Map(), version: 0, published: false, active: false, epoch: 0 };
        slot.entry = entry;
        staged.add(entry);
        stats.instances += 1;
      }
      var checkpoint = wire.checkpoint(), refs = new Set(), events = new Set(), next = new Array(values.length), addresses = [];
      for (var index = 0; index < values.length; index += 1) { next[index] = normalize(values[index], entry, 'v:' + index, [], refs, events); addresses.push(wire.last); }
      if (!equal(entry.values, next)) {
        entry.values = next;
        entry.patches.clear();
        runtime.actions.retain(entry, events);
        entry.views.clear();
        entry.version += 1;
        dirty.add(entry);
        writeInstance(entry, wire.container(true, null, addresses));
      } else if (!entry.published) {
        writeInstance(entry, wire.container(true, null, addresses));
      } else { wire.rewind(checkpoint); }
      entry.refs = entry.patches.size === 0 ? refs : structuralRefs(entry, new Set(Array.from(refs).concat(Array.from(entry.refs))));
      OperitComposeReactive.created(runtime.ctx);
      return OperitComposeNodes.reference(entry, program.root);
    }
    /// Records an explicit public mutation as an operation against a local instruction.
    function patch(entry, node, field, value) {
      var previous = entry.patches.get(node), fields = Object.assign({}, previous);
      var refs = new Set(entry.refs), events = new Set();
      fields[field] = normalize(value, entry, 'p:' + node + ':' + field, [], refs, events);
      entry.patches.set(node, fields);
      try { entry.refs = structuralRefs(entry, refs, events); }
      catch (error) {
        if (previous) entry.patches.set(node, previous); else entry.patches.delete(node);
        entry.views.clear();
        throw error;
      }
      runtime.actions.retain(entry, events);
      entry.version += 1;
      entry.views.clear();
      dirty.add(entry);
      writeInstance(entry, writeParameters(entry, entry.values));
    }
    /// Recomputes local membership only for explicit public-node edits, never for ordinary construction.
    function structuralRefs(entry, candidates, events) {
      var result = new Set(), visited = new Set(), active = new Set();
      /// Validates local membership and follows only the edited source's internal instruction addresses.
      function visit(node) {
        if (active.has(node)) throw new Error('Compose nodes must not form a cycle');
        if (visited.has(node)) return;
        active.add(node); visited.add(node);
        var members = [];
        /// Includes references and event slots from every effective public field.
        function collect(value) {
          if (value === null || typeof value !== 'object') return;
          if (value.__composeRef) { members.push(value); return; }
          if (value.__actionId) { if (events) events.add(value.__actionId); return; }
          Object.keys(value).forEach(
          /// Follows the observed property snapshot without resolving removed parameters.
          function (key) { collect(value[key]); });
        }
        collect(project(entry, node));
        for (var index = 0; index < members.length; index += 1) {
          var ref = members[index].__composeRef;
          if (ref[0] === entry.id) visit(ref[1]);
          else {
            var target = null;
            candidates.forEach(
            /// Resolves a direct source handle without retaining a global detached-instance table.
            function (candidate) { if (candidate.id === ref[0]) target = candidate; });
            if (!target) target = find(ref[0]);
            result.add(target);
          }
        }
        active.delete(node);
      }
      visit(entry.program.root);
      return result;
    }

    /// Rewrites only expired event references when a cached instance becomes live again.
    function replaceEvents(value, replacements) {
      if (value === null || typeof value !== 'object') return value;
      if (replacements.has(value)) return replacements.get(value);
      var result = Array.isArray(value) ? [] : {};
      Object.keys(value).forEach(
      /// Keeps previously emitted parameter snapshots unchanged.
      function (key) { result[key] = replaceEvents(value[key], replacements); });
      return result;
    }
    /// Marks reachable source instances without visiting their internal layout instructions.
    function mark(entry) {
      if (entry.active) throw new Error('Compose nodes must not form a cycle');
      if (entry.epoch === epoch) return;
      entry.epoch = epoch;
      entry.active = true;
      install(entry.program);
      if (!entry.published && !staged.has(entry)) {
        var replacements = runtime.actions.revive(entry);
        entry.values = replaceEvents(entry.values, replacements);
        entry.patches.forEach(
        /// Restores callbacks owned by explicit property edits along with constructor parameters.
        function (fields, node) { entry.patches.set(node, replaceEvents(fields, replacements)); });
        entry.views.clear();
        staged.add(entry);
        writeInstance(entry, writeParameters(entry, entry.values));
      }
      try { entry.refs.forEach(mark); } finally { entry.active = false; }
    }
    /// Excludes detached source references while recording an explicit public edit.
    function writeParameters(entry, value) {
      var members = new Set([entry.id]);
      entry.refs.forEach(
      /// Retains only references still owned by the edited source.
      function (child) { members.add(child.id); });
      return wire.write(value, [], members);
    }
    /// Records fixed instance metadata against parameters already stored by construction.
    function writeInstance(entry, values) {
      var patches = [];
      entry.patches.forEach(
      /// Writes each explicit edit once when its owning source changes.
      function (fields, node) { Object.keys(fields).forEach(
        /// Stores a patch as local address, field name and parameter address.
        function (field) { patches.push(wire.container(true, null, [wire.scalar(node), wire.string(field), writeParameters(entry, fields[field])])); }); });
      entry.wireEpoch = wire.epoch;
      entry.wire = wire.container(true, null, [wire.string(entry.id), wire.string(entry.program.id), values, wire.container(true, null, patches)]);
    }
    /// Flushes ordered source changes; the host owns all mounted occurrence state.
    function finish(root, skipUnchanged) {
      var reference = OperitComposeNodes.resolve(root);
      if (!reference || reference.entry.owner !== api) throw new Error('compose render must return a node');
      epoch += 1;
      mark(reference.entry);
      var instances = [], removed = [];
      live.forEach(
      /// Expires each detached instance exactly once.
      function (entry, id) {
        if (entry.epoch !== epoch) { live.delete(id); removed.push(id); stats.releasedInstances += 1; entry.published = false; runtime.actions.release(entry); }
      });
      staged.forEach(
      /// Publishes only reachable sources and releases unused constructions.
      function (entry) {
        if (entry.epoch === epoch) { live.set(entry.id, entry); entry.published = true; dirty.add(entry); }
        else runtime.actions.release(entry);
      });
      dirty.forEach(
      /// Sends each live changed instance once, independently of its number of mounts.
      function (entry) { if (entry.published) { if (entry.wireEpoch !== wire.epoch) throw new Error('Compose source was not written during its mutation'); instances.push(entry.wire); } });
      var definitions = [];
      pendingPrograms.forEach(
      /// Sends each installed program once instead of once per repeated row.
      function (program, id) { definitions.push(programOffsets.get(id)); announced.add(id); });
      staged.clear(); dirty.clear(); pendingPrograms.clear(); programOffsets.clear();
      stats.commits += 1;
      if (skipUnchanged && revision !== 0 && instances.length === 0 && removed.length === 0 && definitions.length === 0) return null;
      var reset = revision === 0;
      revision += 1;
      var transaction = wire.container(true, null, [wire.scalar(reset), wire.scalar(revision), wire.string(reference.entry.id), wire.scalar(reference.node), wire.container(true, null, definitions), wire.container(true, null, instances), wire.write(removed)]);
      var storage = wire; wire = OperitComposeBuffer.create();
      return { revision: revision,
        /// Seals the already indexed update and transfers it through the sole response contract.
        seal: function (phase, actionResult, navigation) { return storage.seal(phase, transaction, actionResult, navigation); } };

    }
    /// Reads a root lifecycle event without projecting a public node or its descendants.
    function lifecycle(root, name) {
      var ref = OperitComposeNodes.resolve(root);
      var patch = ref.entry.patches.get(ref.node);
      var props = patch && patch.props ? patch.props : operand(ref.entry, ref.entry.program.nodes[ref.node].props);
      return props && props[name];
    }
    var api = { lifecycle: lifecycle, instance: instance, patch: patch, project: project, publicValue: publicValue, finish: finish,
      /// Resolves a logical reference for public projection.
      resolve: find,
      /// Carries callback ownership into an explicit public proxy.
      alias: runtime.actions.alias,
      /// Seals a response with no layout update using the same binary contract.
      response: function (phase, actionResult, navigation) { var storage = wire; wire = OperitComposeBuffer.create(); return storage.seal(phase, 0, actionResult, navigation); },
      /// Reports work in terms of instances and dynamic values, never internal layout nodes.
      metrics: function () { return Object.assign({}, stats); } };
    runtimes.set(runtime.ctx, runtime);
    return api;
  }
  return { create: create, evaluate: evaluate, equal: equal };
}();

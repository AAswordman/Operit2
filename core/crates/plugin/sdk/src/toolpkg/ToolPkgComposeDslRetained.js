/// Retains node identities and callback slots without changing plugin node objects or JS APIs.
var OperitComposeRetained = (function() {
    /// Compares structured data without producing a text representation.
    function equal(left, right) {
        if (Object.is(left, right)) return true;
        if (left === null || right === null || typeof left !== 'object' || typeof right !== 'object') return false;
        if (left.__composeNode === true || right.__composeNode === true) return false;
        if (Array.isArray(left) !== Array.isArray(right)) return false;
        var leftKeys = Object.keys(left);
        var rightKeys = Object.keys(right);
        if (leftKeys.length !== rightKeys.length) return false;
        for (var index = 0; index < leftKeys.length; index += 1) {
            var key = leftKeys[index];
            if (!Object.prototype.hasOwnProperty.call(right, key) || !equal(left[key], right[key])) return false;
        }
        return true;
    }


    /// Creates a persistent retained graph with occurrence-local mutation queues.
    function create(runtime) {
        var nodes = new Map();
        var dirty = new Map();
        var pendingActions = new Set();
        var actionSources = new WeakMap();
        var revision = 0;
        var rootId = null;
        var stats = { visitedNodes: 0, checkedChildReferences: 0, removedNodes: 0, commits: 0 };

        /// Tracks newly registered callbacks without scanning the global action store at commit time.
        function trackAction(id) { pendingActions.add(id); }

        /// Coalesces mutation flags for one retained occurrence.
        function invalidate(id, structure) {
            dirty.set(id, dirty.get(id) === true || structure);
        }

        /// Reports deterministic work counters independently of rendering and transport time.
        function metrics() { return Object.assign({}, stats); }

        /// Commits changed occurrences and removes only detached branches, retaining untouched records.
        function commit(tree) {
            var upserts = new Map();
            var removed = [];
            var removalRoots = new Set();
            var actionDeletes = new Set(pendingActions);
            var active = new Set();

            /// Resolves a callback property to an occurrence-owned stable action slot.
            function bind(value, slot, oldActions, actions) {
                if (value === null || typeof value !== 'object') return value;
                if (Object.prototype.hasOwnProperty.call(value, '__actionId')) {
                    var source = actionSources.get(value);
                    // A cached public node owns its closure even while no mounted occurrence owns an action ID.
                    var handler = source && source.id === value.__actionId ? source.handler : runtime.actionStore[value.__actionId];
                    if (typeof handler !== 'function') throw new Error('compose callback is not registered: ' + value.__actionId);
                    var id;
                    if (Object.prototype.hasOwnProperty.call(oldActions, slot)) id = oldActions[slot];
                    else { runtime.actionCounter += 1; id = '__action_' + runtime.actionCounter; }
                    runtime.actionStore[id] = handler;
                    actions[slot] = id;
                    OperitComposeGeneration.rebind(value, id);
                    actionSources.set(value, { id: id, handler: handler });
                    return { __actionId: id };
                }
                var result = Array.isArray(value) ? [] : {};
                Object.keys(value).forEach(
                    /// Copies one property into the immutable transport record.
                    function(key) { result[key] = bind(value[key], slot + '/' + key.length + ':' + key, oldActions, actions); }
                );
                return result;
            }

            /// Reconciles only the direct membership of a structurally changed parent group.
            function group(children, parent, slot) {
                var identities = new Set();
                return children.map(
                    /// Retains each child occurrence without entering an unchanged subtree.
                    function(child, index) {
                        stats.checkedChildReferences += 1;
                        if (!child || typeof child.type !== 'string') throw new Error('compose render must return a node');
                        var key = child.props && child.props.key;
                        var identity;
                        if (key === undefined || key === null) identity = 'i:' + index;
                        else {
                            if (typeof key !== 'string' && typeof key !== 'number') throw new Error('compose node key must be a string or number');
                            var text = String(key);
                            identity = 'k:' + text.length + ':' + text;
                        }
                        if (identities.has(identity)) throw new Error('duplicate compose node key in ' + parent + ':' + slot);
                        identities.add(identity);
                        var id = parent + '/s:' + slot.length + ':' + slot + '/' + identity + '/t:' + child.type;
                        // Reattachment must reconcile even a previously skipped dirty occurrence.
                        if (removalRoots.delete(id)) invalidate(id, true);
                        var previous = nodes.get(id);
                        if (!previous || previous.node !== child || dirty.has(id)) visit(child, id, parent);
                        return id;
                    }
                );
            }

            /// Collects a record's direct child references without following their descendants.
            function references(record) {
                var refs = record.children.slice();
                Object.keys(record.slots).forEach(
                    /// Includes named-slot members in the same occurrence ownership graph.
                    function(slot) { refs.push.apply(refs, record.slots[slot]); }
                );
                return refs;
            }

            /// Updates one occurrence and records structural removals for later branch disposal.
            function visit(node, id, parent) {
                if (!node || typeof node.type !== 'string') throw new Error('compose render must return a node');
                if (active.has(node)) throw new Error('compose nodes must not form a cycle');
                var previous = nodes.get(id);
                var replacement = !previous || previous.node !== node;
                if (!replacement && !dirty.has(id)) return;
                var structure = replacement || dirty.get(id) === true;
                dirty.delete(id);
                active.add(node);
                stats.visitedNodes += 1;
                var oldActions = previous ? previous.actions : {};
                // Callback slots belong to the mounted occurrence, not to its currently visible properties.
                // A queued event can still address a temporarily hidden slot; rebinding replaces its closure.
                var actions = Object.assign({}, oldActions);
                var props = bind(node.props, '', oldActions, actions);
                var children;
                var slots;
                if (structure) {
                    children = group(node.children, id, 'children');
                    slots = {};
                    Object.keys(node.slots || {}).forEach(
                        /// Reconciles the named groups owned by this changed occurrence.
                        function(slot) { slots[slot] = group(node.slots[slot], id, slot); }
                    );
                } else { children = previous.record.children; slots = previous.record.slots; }
                var record = { id: id, nodeType: node.type, props: props, children: children, slots: slots };
                if (previous && structure) {
                    var retained = new Set(references(record));
                    references(previous.record).forEach(
                        /// Schedules only branches detached by this parent's membership change.
                        function(child) { if (!retained.has(child)) removalRoots.add(child); }
                    );
                }
                var unsubscribe;
                if (replacement) {
                    if (previous) previous.unsubscribe();
                    unsubscribe = OperitComposeGeneration.watch(node,
                        /// Enqueues direct mutations and notifies the parent only when identity changes.
                        function(path) {
                            invalidate(id, path[0] === 'children' || path[0] === 'slots');
                            if (path[0] === 'type' || (path[0] === 'props' && path[1] === 'key')) {
                                if (parent !== null) invalidate(parent, true);
                            }
                        }
                    );
                } else unsubscribe = previous.unsubscribe;
                nodes.set(id, { node: node, record: record, actions: actions, parent: parent, unsubscribe: unsubscribe });
                if (!previous || !equal(previous.record, record)) upserts.set(id, record);
                active.delete(node);
            }

            /// Disposes one detached branch and its callbacks without scanning surviving nodes.
            function removeBranch(id) {
                var entry = nodes.get(id);
                if (!entry) return;
                references(entry.record).forEach(removeBranch);
                Object.keys(entry.actions).forEach(
                    /// Schedules callback deletion after all new bindings have been read.
                    function(slot) { actionDeletes.add(entry.actions[slot]); }
                );
                entry.unsubscribe();
                nodes.delete(id);
                dirty.delete(id);
                upserts.delete(id);
                removed.push(id);
                stats.removedNodes += 1;
            }

            var nextRoot = 'root/t:' + tree.type;
            if (rootId !== null && rootId !== nextRoot) removalRoots.add(rootId);
            visit(tree, nextRoot, null);
            // Only explicitly queued occurrences are visited beneath unchanged ancestors.
            while (dirty.size > 0) {
                var id = dirty.keys().next().value;
                var entry = nodes.get(id);
                if (!entry) { dirty.delete(id); continue; }
                visit(entry.node, id, entry.parent);
            }
            // Branch ownership is final only after all dirty parents have reconciled their references.
            removalRoots.forEach(removeBranch);
            actionDeletes.forEach(
                /// Deletes newly discarded and detached callbacks, not every registered callback.
                function(id) { delete runtime.actionStore[id]; }
            );
            pendingActions.clear();
            rootId = nextRoot;
            stats.commits += 1;
            var reset = revision === 0;
            revision += 1;
            return { reset: reset, revision: revision, rootId: rootId, upserts: Array.from(upserts.values()), removed: removed };
        }

        return { commit: commit, trackAction: trackAction, metrics: metrics };
    }
    return { create: create, equal: equal };
})();

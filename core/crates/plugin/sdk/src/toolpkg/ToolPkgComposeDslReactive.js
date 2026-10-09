/// Schedules state reads, derived bindings and UI expressions in one dependency graph.
var OperitComposeReactive = (function() {
    var graphs = new WeakMap();

    /// Attaches dependency ownership to the existing page context.
    function attach(ctx) {
        graphs.set(ctx, { tasks: new Map(), subscriptions: new Map(), dirty: new Set(), current: null,
            sequence: 0, root: null, entry: null, metrics: { rootRuns: 0, bindingRuns: 0, expressionRuns: 0 } });
    }

    /// Replaces the dependencies of an execution unit without retaining stale subscriptions.
    function subscribe(graph, task, keys) {
        task.dependencies.forEach(
            /// Releases a subscription before changing the execution unit's dependency set.
            function(key) {
                var subscribers = graph.subscriptions.get(key);
                subscribers.delete(task);
                if (subscribers.size === 0) graph.subscriptions.delete(key);
            }
        );
        task.dependencies = new Set(keys);
        task.dependencies.forEach(
            /// Indexes execution units by the exact state or computed binding they read.
            function(key) {
                if (!graph.subscriptions.has(key)) graph.subscriptions.set(key, new Set());
                graph.subscriptions.get(key).add(task);
            }
        );
    }

    /// Marks only indexed consumers dirty, with one queue entry per execution unit.
    function publish(ctx, key) {
        var graph = graphs.get(ctx);
        var subscribers = graph.subscriptions.get(key);
        if (subscribers) subscribers.forEach(
            /// Coalesces writes until the next composition execution.
            function(task) { graph.dirty.add(task); }
        );
    }

    /// Records a state write and the explicit broad dependency used by opaque execution units.
    function stateChanged(ctx, key) { publish(ctx, 'state:' + key); publish(ctx, '*'); }

    /// Executes an ordered unit while restoring the surrounding dependency owner on every exit.
    function run(ctx, task) {
        var graph = graphs.get(ctx);
        var previous = graph.current;
        graph.current = task;
        task.counts.clear();
        var used = new Set();
        task.usedChildren = used;
        try {
            var value = task.execute();
            graph.dirty.delete(task);
            task.children.forEach(
                /// Disposes inactive branch and loop units after their parent completed successfully.
                function(child) { if (!used.has(child)) release(graph, child); }
            );
            task.children = used;
            if (!Object.is(value, task.value)) {
                task.value = value;
                publish(ctx, 'unit:' + task.key);
            }
            return task.value;
        } finally { graph.current = previous; }
    }

    /// Registers one unit inside its lexical parent and preserves loop occurrence ownership.
    function unit(ctx, site, keys, execute, kind) {
        var graph = graphs.get(ctx);
        var parent = graph.current;
        var occurrence = parent.counts.get(site) || 0;
        parent.counts.set(site, occurrence + 1);
        var key = parent.key + '/' + site + ':' + occurrence;
        var task = graph.tasks.get(key);
        if (!task) {
            task = { key: key, order: graph.sequence++, dependencies: new Set(), counts: new Map(),
                execute: execute, value: undefined, parent: parent, children: new Set(), kind: kind };
            graph.tasks.set(key, task);
            subscribe(graph, task, keys);
            run(ctx, task);
        } else {
            task.execute = execute;
            subscribe(graph, task, keys);
            run(ctx, task);
        }
        parent.usedChildren.add(task);
        return task;
    }

    /// Declares an independently compiled lexical frame and removes its enclosing broad subscription.
    function frame(ctx, site) {
        var graph = graphs.get(ctx);
        subscribe(graph, graph.current, []);
        return { ctx: ctx, site: site, key: graph.current.key };
    }

    /// Refreshes a captured state variable before dependent computations or callbacks execute.
    function state(frame, name, key, assign) {
        return unit(frame.ctx, 'binding:' + name, ['state:' + key],
            /// Reads the existing public state API and updates the lexical binding in place.
            function() {
                var value = frame.ctx.useState(key)[0];
                assign(value);
                publish(frame.ctx, frame.key + ':value:' + name);
                graphs.get(frame.ctx).metrics.bindingRuns += 1;
                return value;
            }, 'binding').value;
    }

    /// Recomputes a derived lexical value only when one of its compiled inputs changes.
    function derived(frame, name, dependencies, evaluate, assign) {
        return unit(frame.ctx, 'derived:' + name, dependencies,
            /// Publishes the derived binding after updating closures that capture it.
            function() {
                var value = evaluate();
                assign(value);
                publish(frame.ctx, frame.key + ':value:' + name);
                graphs.get(frame.ctx).metrics.bindingRuns += 1;
                return value;
            }, 'binding').value;
    }

    /// Preserves node identity while updating its constructor arguments in a subscribed UI unit.
    function expression(frame, site, dependencies, argumentsFactory, factory) {
        var graph = graphs.get(frame.ctx);
        var owner = graph.current;
        var task = unit(frame.ctx, site, dependencies,
            /// Evaluates only this UI expression and records its actual recomposition count.
            function() {
                graph.metrics.expressionRuns += 1;
                if (!this.entry) this.entry = { callbacks: new Map(), initialized: false };
                return OperitComposeGeneration.evaluate(this.entry, argumentsFactory(), factory);
            }, 'expression');
        // Parent constructors need a child subscription only if the child replaces its handle.
        var dependency = 'unit:' + task.key;
        if (!owner.dependencies.has(dependency)) subscribe(graph, owner, Array.from(owner.dependencies).concat(dependency));
        return task.value;
    }

    /// Executes the initial entry once, then drains only dependency-indexed execution units.
    function render(ctx, entry) {
        var graph = graphs.get(ctx);
        if (graph.entry !== entry) {
            dispose(ctx);
            graph.entry = entry;
            graph.root = { key: 'root', order: graph.sequence++, dependencies: new Set(), counts: new Map(), value: undefined,
                /// Executes the original entry as an explicitly subscribed root unit.
                execute: function() {
                    graph.metrics.rootRuns += 1;
                    OperitComposeGeneration.begin(ctx);
                    try {
                        var value = entry(ctx);
                        if (value && typeof value.then === 'function') return value.then(
                            /// Completes generation ownership after an asynchronous root resolves.
                            function(tree) { OperitComposeGeneration.end(ctx); return tree; },
                            /// Releases generation bookkeeping before propagating asynchronous failure.
                            function(error) { OperitComposeGeneration.end(ctx); throw error; }
                        );
                        OperitComposeGeneration.end(ctx);
                        return value;
                    } catch (error) { OperitComposeGeneration.end(ctx); throw error; }
                }, children: new Set(), kind: 'root' };
            graph.tasks.set('root', graph.root);
            subscribe(graph, graph.root, ['*']);
            return run(ctx, graph.root);
        }
        // Broad execution units retain the original per-response evaluation semantics.
        publish(ctx, '*');
        while (graph.dirty.size > 0) {
            var next = null;
            graph.dirty.forEach(
                /// Chooses dependency registration order so bindings precede their consumers.
                function(task) { if (next === null || task.order < next.order) next = task; }
            );
            run(ctx, next);
        }
        return graph.root.value;
    }

    /// Releases one execution branch and removes all of its outstanding state subscriptions.
    function release(graph, task) {
        task.children.forEach(function(child) { release(graph, child); });
        subscribe(graph, task, []);
        graph.dirty.delete(task);
        graph.tasks.delete(task.key);
    }

    /// Invalidates the explicitly registered root scope after a host environment change.
    function environmentChanged(ctx) {
        var graph = graphs.get(ctx);
        if (graph.root) graph.dirty.add(graph.root);
    }

    /// Releases all dependency subscriptions when replacing the root program.
    function dispose(ctx) {
        var graph = graphs.get(ctx);
        graph.tasks.clear(); graph.subscriptions.clear(); graph.dirty.clear(); graph.current = null;
        graph.root = null; graph.entry = null;
    }

    /// Exposes counters for execution-level acceptance tests.
    function metrics(ctx) { return Object.assign({}, graphs.get(ctx).metrics); }
    return { attach: attach, frame: frame, state: state, derived: derived, expression: expression,
        environmentChanged: environmentChanged, stateChanged: stateChanged, render: render, dispose: dispose, metrics: metrics };
})();

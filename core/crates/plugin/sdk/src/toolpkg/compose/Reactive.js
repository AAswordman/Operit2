/// Schedules state reads, derived bindings and UI expressions in one dependency graph.
var OperitComposeReactive = function () {
  var graphs = new WeakMap();

  /// Attaches dependency ownership to the existing page context.
  function attach(ctx, cleanup) {
    graphs.set(ctx, {
      cleanup: cleanup,
      tasks: new Map(),
      subscriptions: new Map(),
      dirty: new Set(),
      current: null,
      sequence: 0,
      contentRevision: 0,
      root: null,
      entry: null,
      effects: new Map(),
      epoch: 0,
      scope: null,
      metrics: {
        rootRuns: 0,
        bindingRuns: 0,
        expressionRuns: 0
      }
    });
  }

  /// Replaces the dependencies of an execution unit without retaining stale subscriptions.
  function subscribe(graph, task, keys) {
    task.dependencies.forEach(
    /// Releases a subscription before changing the execution unit's dependency set.
    function (key) {
      var subscribers = graph.subscriptions.get(key);
      subscribers.delete(task);
      if (subscribers.size === 0) graph.subscriptions.delete(key);
    });
    task.dependencies = new Set(keys);
    task.dependencies.forEach(
    /// Indexes execution units by the exact state or computed binding they read.
    function (key) {
      if (!graph.subscriptions.has(key)) graph.subscriptions.set(key, new Set());
      graph.subscriptions.get(key).add(task);
    });
  }

  /// Marks only indexed consumers dirty, with one queue entry per execution unit.
  function publish(ctx, key) {
    var graph = graphs.get(ctx);
    var subscribers = graph.subscriptions.get(key);
    if (subscribers) subscribers.forEach(
    /// Coalesces writes until the next composition execution.
    function (task) {
      graph.dirty.add(task);
    });
  }

  /// Records a state write and the explicit broad dependency used by opaque execution units.
  function stateChanged(ctx, key) {
    invalidateContent(ctx);
    publish(ctx, 'state:' + key);
    publish(ctx, '*');
  }

  /// Executes an ordered unit while restoring the surrounding dependency owner on every exit.
  function run(ctx, task) {
    var graph = graphs.get(ctx);
    var previous = graph.current;
    graph.current = task;
    task.counts.clear();
    task.sourceCounts = new Map();
    task.hasNodes = false;
    var used = new Set();
    task.usedChildren = used;
    var usedEffects = new Set();
    task.usedEffects = usedEffects;
    try {
      var value = task.execute();
      graph.dirty.delete(task);
      if (task.sources) task.sources.forEach(
      /// Releases source slots omitted by the current branch or shortened repetition.
      function (entries, site) {
        var count = task.sourceCounts.get(site) || 0;
        entries.length = count;
        if (count === 0) task.sources.delete(site);
      });
      task.children.forEach(
      /// Disposes inactive branch and loop units after their parent completed successfully.
      function (child) {
        if (!used.has(child)) release(graph, child);
      });
      task.children = used;
      graph.effects.forEach(/// Releases effects removed by this execution branch.
      function (effect, key) {
        if (effect.owner === task && !usedEffects.has(key)) {
          effect.cleanup();
          graph.effects.delete(key);
        }
      });
      if (!Object.is(value, task.value)) {
        task.value = value;
        publish(ctx, 'unit:' + task.key);
      }
      return task.value;
    } finally {
      graph.current = previous;
    }
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
      task = {
        key: key,
        order: graph.sequence++,
        dependencies: new Set(),
        counts: new Map(),
        execute: execute,
        value: undefined,
        parent: parent,
        children: new Set(),
        kind: kind
      };
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

  /// Owns subscriptions and effects inside a memo even when its factory is skipped by its parent.
  function memo(ctx, key, reusable, execute) {
    var graph = graphs.get(ctx);
    var parent = graph.current;
    if (!parent) return execute();
    var id = parent.key + '/memo:' + key.length + ':' + key;
    var task = graph.tasks.get(id);
    var retained = !!task;
    if (!task) {
      task = {
        key: id,
        order: graph.sequence++,
        dependencies: new Set(),
        counts: new Map(),
        execute: execute,
        value: undefined,
        parent: parent,
        children: new Set(),
        kind: 'memo'
      };
      graph.tasks.set(id, task);
    }
    parent.usedChildren.add(task);
    if (!retained || !reusable) {
      task.execute = execute;
      run(ctx, task);
    }
    return task.value;
  }

  /// Declares an independently compiled lexical frame and removes its enclosing broad subscription.
  function frame(ctx, site) {
    var graph = graphs.get(ctx);
    subscribe(graph, graph.current, []);
    return {
      ctx: ctx,
      site: site,
      key: graph.current.key
    };
  }

  /// Refreshes a captured state variable before dependent computations or callbacks execute.
  function state(frame, name, key, assign) {
    return unit(frame.ctx, 'binding:' + name, ['state:' + key],
    /// Reads the existing public state API and updates the lexical binding in place.
    function () {
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
    function () {
      var value = evaluate();
      assign(value);
      publish(frame.ctx, frame.key + ':value:' + name);
      graphs.get(frame.ctx).metrics.bindingRuns += 1;
      return value;
    }, 'binding').value;
  }

  /// Selects one source slot in the current lexical execution unit without node snapshots.
  function scope(ctx, site) {
    var graph = graphs.get(ctx);
    var task = graph.current;
    if (!task) return {};
    if (!task.sources) task.sources = new Map();
    if (!task.sourceCounts) task.sourceCounts = new Map();
    var index = task.sourceCounts.get(site) || 0;
    task.sourceCounts.set(site, index + 1);
    var entries = task.sources.get(site);
    if (!entries) { entries = []; task.sources.set(site, entries); }
    if (!entries[index]) entries[index] = {};
    return entries[index];
  }

  /// Marks source-producing memo units for module-replacement invalidation.
  function created(ctx) {
    var graph = graphs.get(ctx);
    for (var task = graph && graph.current; task; task = task.parent) task.hasNodes = true;
  }

  /// Evaluates one fused layout and publishes only its dynamic instance parameters.
  function expressionProgram(frame, site, dependencies, valuesFactory, program) {
    var graph = graphs.get(frame.ctx), owner = graph.current;
    var task = unit(frame.ctx, site, dependencies,
    /// Runs JavaScript business expressions without expanding internal layout instructions.
    function () {
      graph.metrics.expressionRuns += 1;
      return OperitComposeComposition.evaluate(frame.ctx, site, program, valuesFactory());
    }, 'expression');
    var dependency = 'unit:' + task.key;
    if (!owner.dependencies.has(dependency)) subscribe(graph, owner, Array.from(owner.dependencies).concat(dependency));
    return task.value;
  }

  /// Tracks dependency reads for runtime-defined argument iteration in the same scheduler.
  function expressionDynamic(frame, site, dependencies, evaluate) {
    var graph = graphs.get(frame.ctx), parent = graph.current;
    var task = unit(frame.ctx, site, dependencies, evaluate, 'expression');
    var dependency = 'unit:' + task.key;
    if (!parent.dependencies.has(dependency)) subscribe(graph, parent, Array.from(parent.dependencies).concat(dependency));
    return task.value;
  }

  /// Owns memo values and source-reload epochs in the existing dependency scheduler.
  function memoValue(ctx, key, previous, changed, factory) {
    var graph = graphs.get(ctx), parent = graph.current;
    var reusable = !changed && previous && (!previous.hasNodes || previous.epoch === graph.epoch);
    return memo(ctx, key, reusable,
    /// Preserves plain-data identity while rebuilding node-producing memos after source replacement.
    function () {
      if (reusable && !previous.hasNodes) return previous;
      var value = factory();
      var task = graph.current;
      return { value: value, hasNodes: !!(task && task.hasNodes), epoch: graph.epoch };
    });
  }

  /// Retains each requested lazy item in its own ordinary memo execution unit.
  function memoItem(ctx, key, version, factory) {
    var graph = graphs.get(ctx);
    var state = scope(ctx, '@item:' + key);
    var cached = state.item;
    var result = memoValue(ctx, '@item:' + key, cached && cached.generated, !cached || cached.version !== version, factory);
    state.item = { version: version, generated: result };
    return result.value;
  }

  /// Executes the initial entry once, then drains only dependency-indexed execution units.
  function render(ctx, entry, refreshOpaqueRoot) {
    var graph = graphs.get(ctx);
    // Ordinary JS closures and refs have no observable stability contract. Refresh
    // them on explicit renders; a trusted range event changes layout state only.
    if (refreshOpaqueRoot !== false) invalidateContent(ctx);
    if (graph.entry !== entry) {
      dispose(ctx);
      graph.entry = entry;
      graph.epoch += 1;
      graph.root = {
        key: 'root',
        order: graph.sequence++,
        dependencies: new Set(),
        counts: new Map(),
        value: undefined,
        /// Executes the original entry as an explicitly subscribed root unit.
        execute: function () {
          graph.metrics.rootRuns += 1;

          return entry(ctx);
        },
        children: new Set(),
        kind: 'root'
      };
      graph.tasks.set('root', graph.root);
      subscribe(graph, graph.root, ['*']);
      return run(ctx, graph.root);
    }
    // Eager, final and explicit host renders also observe ordinary JS closures.
    // Intermediate state notifications drain only work not already consumed by
    // another command (for example a list viewport update in this same context).
    if (refreshOpaqueRoot !== false) publish(ctx, '*');
    while (graph.dirty.size > 0) {
      var next = null;
      graph.dirty.forEach(
      /// Chooses dependency registration order so bindings precede their consumers.
      function (task) {
        if (next === null || task.order < next.order) next = task;
      });
      run(ctx, next);
    }
    return graph.root.value;
  }

  /// Releases one execution branch and removes all of its outstanding state subscriptions.
  function release(graph, task) {
    task.children.forEach(function (child) {
      release(graph, child);
    });
    graph.effects.forEach(/// Cleans up asynchronous work owned by the released branch.
    function (effect, key) {
      if (effect.owner === task) { effect.cleanup(); graph.effects.delete(key); }
    });
    subscribe(graph, task, []);
    graph.dirty.delete(task);
    graph.tasks.delete(task.key);
  }

  /// Invalidates cached content on state/input changes and opaque business callbacks.
  function invalidateContent(ctx) {
    graphs.get(ctx).contentRevision += 1;
  }

  /// Reopens the root for a range change while preserving unchanged item content.
  function invalidateViewport(ctx) {
    var graph = graphs.get(ctx);
    if (graph.root) graph.dirty.add(graph.root);
  }

  /// Host inputs, theme changes and source replacement invalidate content as well.
  function invalidateRoot(ctx) {
    invalidateContent(ctx);
    invalidateViewport(ctx);
  }

  /// Releases all dependency subscriptions when replacing the root program.
  function dispose(ctx) {
    var graph = graphs.get(ctx);
    graph.cleanup();
    graph.effects.forEach(/// Cancels subscriptions and timers owned by the discarded program.
    function (effect) { effect.cleanup(); });
    graph.effects.clear();
    graph.tasks.clear();
    graph.subscriptions.clear();
    graph.dirty.clear();
    graph.current = null;
    graph.root = null;
    graph.entry = null;
  }

  /// Runs a keyed lifecycle effect and ties cleanup to its exact reactive execution owner.
  function effect(ctx, key, setup, dependencies) {
    var graph = graphs.get(ctx);
    if (!graph.current) throw new Error('useEffect requires an active composition');
    var id = graph.current.key + ':effect:' + key;
    var previous = graph.effects.get(id);
    graph.current.usedEffects.add(id);
    var changed = !previous || previous.dependencies.length !== dependencies.length || dependencies.some(
    /// Compares dependency identities without serializing user objects.
    function (value, index) { return !Object.is(value, previous.dependencies[index]); });
    if (!changed) return;
    if (previous) { previous.cleanup(); graph.effects.delete(id); }
    var cleanup = setup();
    if (typeof cleanup !== 'function') throw new TypeError('useEffect must return a cleanup function');
    graph.effects.set(id, { owner: graph.current, dependencies: dependencies.slice(), cleanup: cleanup });
  }

  /// Exposes counters for execution-level acceptance tests.
  function metrics(ctx) {
    return Object.assign({}, graphs.get(ctx).metrics);
  }
  return {
    attach: attach,
    memo: memo,
    effect: effect,
    frame: frame,
    state: state,
    derived: derived,
    expressionProgram: expressionProgram,
    expressionDynamic: expressionDynamic,
    scope: scope,
    created: created,
    memoValue: memoValue,
    memoItem: memoItem,
    invalidateRoot: invalidateRoot,
    invalidateViewport: invalidateViewport,
    invalidateContent: invalidateContent,
    contentRevision: function (ctx) { return graphs.get(ctx).contentRevision; },
    stateChanged: stateChanged,
    render: render,
    dispose: dispose,
    metrics: metrics
  };
}();

// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

/// Tracks instance reads without allocating a retained model for every layout instruction.
class _ComposeProgramReadScope {
  static Set<_ComposeProgramInstance>? _reads;

  /// Records a dynamic source read in the current Flutter build.
  static void read(_ComposeProgramInstance source) {
    _reads?.add(source);
  }

  /// Restores enclosing read ownership after synchronous or deferred widget building.
  static T track<T>(Set<_ComposeProgramInstance> reads, T Function() body) {
    final previous = _reads;
    _reads = reads;
    try {
      return body();
    } finally {
      _reads = previous;
    }
  }
}

/// Installs and validates a single operand once instead of parsing its shape per repeated row.
class _ComposeOperand {
  /// Stores a checked opcode and its immutable operands.
  const _ComposeOperand(this.tag, this.value, this.members, this.names);
  final int tag;
  final Object? value;
  final List<_ComposeOperand> members;
  final List<String> names;

  /// Requires a well-formed operand and a valid local instruction address.
  factory _ComposeOperand.parse(Object? raw, int nodeCount) {
    if (raw is! List || raw.isEmpty || raw[0] is! int)
      throw const FormatException('Invalid Compose operand');
    final tag = raw[0] as int;
    switch (tag) {
      case 0:
        if (raw.length != 2)
          throw const FormatException('Invalid Compose constant');
        return _ComposeOperand(tag, raw[1], const [], const []);
      case 1:
      case 4:
        if (raw.length != 2 ||
            raw[1] is! int ||
            (raw[1] as int) < 0 ||
            tag == 4 && (raw[1] as int) >= nodeCount)
          throw const FormatException('Invalid Compose operand address');
        return _ComposeOperand(tag, raw[1], const [], const []);
      case 2:
        if (raw.length != 3 ||
            raw[1] is! List ||
            raw[2] is! List ||
            (raw[1] as List).length != (raw[2] as List).length)
          throw const FormatException('Invalid Compose object operand');
        final names = (raw[1] as List).cast<String>();
        if (names.toSet().length != names.length)
          throw const FormatException('Duplicate Compose operand field');
        return _ComposeOperand(tag, null, [
          for (final item in raw[2] as List)
            _ComposeOperand.parse(item, nodeCount),
        ], names);
      case 3:
        if (raw.length != 2 || raw[1] is! List)
          throw const FormatException('Invalid Compose array operand');
        return _ComposeOperand(tag, null, [
          for (final item in raw[1] as List)
            _ComposeOperand.parse(item, nodeCount),
        ], const []);
      case 5:
        if (raw.length != 3)
          throw const FormatException('Invalid Compose concatenation');
        return _ComposeOperand(tag, null, [
          _ComposeOperand.parse(raw[1], nodeCount),
          _ComposeOperand.parse(raw[2], nodeCount),
        ], const []);
      default:
        throw FormatException('Unknown Compose operand: $tag');
    }
  }

  /// Reads constants and parameter addresses directly from an installed instruction.
  Object? evaluate(_ComposeProgramInstance instance) {
    switch (tag) {
      case 0:
        return value;
      case 1:
        return instance.parameter(value as int);
      case 2:
        return <String, Object?>{
          for (var i = 0; i < names.length; i++)
            names[i]: members[i].evaluate(instance),
        };
      case 3:
        return [for (final member in members) member.evaluate(instance)];
      case 4:
        return instance.cursor(value as int);
      case 5:
        return [
          ...(members[0].evaluate(instance) as List),
          ...(members[1].evaluate(instance) as List),
        ];
      default:
        throw StateError('Uninstalled Compose operand');
    }
  }
}

/// Holds prevalidated layout metadata shared by every source using this program.
class _ComposeLayoutInstruction {
  /// Resolves the canonical widget type and parses its operand layout once.
  _ComposeLayoutInstruction(
    compose_buffer.ToolPkgComposeDslInstruction source,
    int nodeCount,
  ) : type =
          _composeNodeTypes[_normalizeToken(source.nodeType)] ??
          (throw FormatException(
            'Unknown Compose node type: ${source.nodeType}',
          )),
      props = _ComposeOperand.parse(source.props, nodeCount),
      children = _ComposeOperand.parse(source.children, nodeCount),
      explicitChildren = source.explicitChildren;
  final String type;
  final _ComposeOperand props;
  final _ComposeOperand children;
  final bool explicitChildren;
}

/// Stores shared code independently of changing instance parameters.
class _ComposeLayoutProgram {
  /// Compiles instruction metadata exactly once when the program is installed.
  _ComposeLayoutProgram(this.definition)
    : instructions = [
        for (final node in definition.nodes)
          _ComposeLayoutInstruction(node, definition.nodes.length),
      ] {
    if (definition.root < 0 || definition.root >= instructions.length)
      throw const FormatException('Compose program root is out of range');
    final edges = <int, Set<int>>{};

    /// Indexes operand requirements once for every instance that shares this program.
    void index(_ComposeOperand operand, Set<int> local) {
      if (operand.tag == 1)
        requiredParameters = math.max(
          requiredParameters,
          (operand.value as int) + 1,
        );
      if (operand.tag == 4) local.add(operand.value as int);
      for (final member in operand.members) {
        index(member, local);
      }
    }

    for (var node = 0; node < instructions.length; node++) {
      final local = edges[node] = <int>{};
      index(instructions[node].props, local);
      index(instructions[node].children, local);
    }
    final active = <int>{}, visited = <int>{};

    /// Rejects cyclic immutable instruction graphs before any instance is published.
    void visit(int node) {
      if (active.contains(node))
        throw const FormatException(
          'Compose program instructions form a cycle',
        );
      if (!visited.add(node)) return;
      active.add(node);
      for (final child in edges[node]!) {
        visit(child);
      }
      active.remove(node);
    }

    for (var node = 0; node < instructions.length; node++) {
      visit(node);
    }
  }
  final compose_buffer.ToolPkgComposeDslProgram definition;
  final List<_ComposeLayoutInstruction> instructions;
  int requiredParameters = 0;
}

/// Owns one parameter vector and its listeners; local instructions have no independent retained state.
class _ComposeProgramInstance extends ChangeNotifier {
  /// Connects instance parameters to their installed code and owning store.
  _ComposeProgramInstance(this.store, this.program, this.data);
  final _ComposeProgramStore store;
  final _ComposeLayoutProgram program;
  compose_buffer.ToolPkgComposeDslInstance data;
  final Map<int, _ComposeProgramNode> _cursors = {};
  final Map<int, _ComposeProgramProjection> _views = {};
  final Map<int, Object?> _parameters = {};

  /// Resolves each dynamic parameter once for all instructions that read it.
  Object? parameter(int index) => _parameters.putIfAbsent(
    index,
    () => store.resolveValue(data.values[index]),
  );
  bool isDisposed = false;

  /// Reuses only thin instruction addresses requested by actual widget execution.
  _ComposeProgramNode cursor(int node) {
    if (node < 0 || node >= program.instructions.length)
      throw const FormatException('Unknown Compose instruction');
    return _cursors.putIfAbsent(node, () => _ComposeProgramNode._(this, node));
  }

  /// Resolves a local instruction lazily without expanding its descendants.
  _ComposeProgramProjection project(int node) => _views.putIfAbsent(node, () {
    final instruction = program.instructions[node];
    final raw = instruction.props.evaluate(this);
    final props = <String, Object?>{},
        slots = <String, List<_ComposeProgramNode>>{};
    var content = instruction.children.evaluate(this);
    if (raw is Map) {
      for (final entry in raw.entries) {
        final name = entry.key as String;
        if (name == 'content' &&
            (!instruction.explicitChildren ||
                content is Map && content['__composeUndefined'] == true)) {
          content = entry.value;
          continue;
        }
        final children = store.children(entry.value);
        if (children.isEmpty) {
          props[name] = entry.value;
        } else {
          slots[name] = children;
        }
      }
    }
    var type = instruction.type;
    var children = store.children(content);
    for (final patch in data.patches) {
      if (patch.node != node) continue;
      final value = store.resolveValue(patch.value);
      switch (patch.field) {
        case 'type':
          type =
              _composeNodeTypes[_normalizeToken(value as String)] ??
              (throw const FormatException(
                'Unknown Compose patched node type',
              ));
        case 'props':
          props.clear();
          props.addAll((value as Map).cast<String, Object?>());
        case 'children':
          children = store.children(value);
        case 'slots':
          slots.clear();
          for (final entry in (value as Map).entries) {
            slots[entry.key as String] = store.children(entry.value);
          }
        default:
          throw const FormatException('Unknown Compose patch field');
      }
    }
    return _ComposeProgramProjection(type, props, children, slots);
  });

  /// Replaces parameters before any dependent Flutter builds are notified.
  void replace(compose_buffer.ToolPkgComposeDslInstance next) {
    data = next;
    _views.clear();
    _parameters.clear();
  }

  /// Publishes an already committed parameter update.
  void publish() {
    if (!isDisposed) notifyListeners();
  }

  /// Freezes already observed instructions while all transaction references are still resolvable.
  void retire() {
    for (final local in _cursors.keys.toList(growable: false)) {
      project(local);
    }
  }

  /// Ends only source ownership; Flutter retains mount identity through its normal keys.
  @override
  void dispose() {
    isDisposed = true;
    super.dispose();
  }
}

/// Carries one transient resolved instruction without containing another mutable node graph.
class _ComposeProgramProjection {
  /// Stores the operands needed by one native widget builder.
  const _ComposeProgramProjection(
    this.type,
    this.props,
    this.children,
    this.slots,
  );
  final String type;
  final Map<String, Object?> props;
  final List<_ComposeProgramNode> children;
  final Map<String, List<_ComposeProgramNode>> slots;
}

/// Owns the sole Flutter program store and atomically publishes transaction revisions.
class _ComposeProgramStore {
  final Map<String, _ComposeLayoutProgram> _programs = {};
  final Map<String, _ComposeProgramInstance> _instances = {};
  compose_buffer.ToolPkgComposeDslReference? _root;
  int _revision = 0;
  final Map<String, Set<String>> _references = {};

  /// Resolves the current root address without rebuilding its subtree.
  _ComposeProgramNode get root {
    final ref = _root!;
    return _instance(ref.instanceId).cursor(ref.node);
  }

  /// Requires each referenced source to exist in the current transaction.
  _ComposeProgramInstance _instance(String id) =>
      _instances[id] ?? (throw StateError('Unknown Compose instance: $id'));

  /// Resolves only encoded dynamic references; ordinary payload objects retain their shape.
  Object? resolveValue(Object? value) {
    if (value is Map) {
      if (value.containsKey('__composeRef')) {
        final ref = value['__composeRef'] as List;
        if (ref.length != 2 || ref[0] is! String || ref[1] is! int)
          throw const FormatException('Invalid Compose reference');
        return _instance(ref[0] as String).cursor(ref[1] as int);
      }
      return <String, Object?>{
        for (final entry in value.entries)
          entry.key as String: resolveValue(entry.value),
      };
    }
    if (value is List)
      return [for (final member in value) resolveValue(member)];
    return value;
  }

  /// Extracts logical children without inspecting unrelated properties or descendants.
  List<_ComposeProgramNode> children(Object? value) {
    final result = <_ComposeProgramNode>[];

    /// Visits arrays and logical references in their original order.
    void collect(Object? member) {
      if (member is _ComposeProgramNode) {
        result.add(member);
      } else if (member is List) {
        for (final child in member) {
          collect(child);
        }
      }
    }

    collect(value);
    return result;
  }

  /// Installs definitions and applies source data before notifying any mounted widgets.
  void apply(core_proxy.ToolPkgComposeDslTransaction envelope) {
    applyProgram(
      compose_buffer.ComposeProgramBuffer(envelope.buffer).transaction,
    );
  }

  /// Commits typed program data from transport or Flutter-owned inline instructions.
  void applyProgram(compose_buffer.ComposeProgramTransaction update) {
    if (!update.reset && update.revision != _revision + 1)
      throw StateError('Compose transaction revision is out of order');
    final definitions = <String, _ComposeLayoutProgram>{};
    for (final definition in update.programs) {
      if (definitions.containsKey(definition.id) ||
          !update.reset && _programs.containsKey(definition.id))
        throw const FormatException('Compose program was installed twice');
      definitions[definition.id] = _ComposeLayoutProgram(definition);
    }

    /// Resolves programs against the whole incoming transaction before changing the store.
    _ComposeLayoutProgram program(String id) =>
        definitions[id] ??
        (!update.reset ? _programs[id] : null) ??
        (throw StateError('Unknown Compose program: $id'));
    final sources = <String, compose_buffer.ToolPkgComposeDslInstance>{
      for (final source in update.instances) source.id: source,
    };
    if (sources.length != update.instances.length ||
        update.removed.toSet().length != update.removed.length)
      throw const FormatException('Duplicate Compose instance address');
    final removed = update.removed.toSet();
    final references = update.reset
        ? <String, Set<String>>{}
        : Map<String, Set<String>>.of(_references);

    /// Validates dynamic addresses against the complete transaction, including removals.
    void inspect(Object? value, Set<String> outgoing) {
      if (value is List) {
        for (final child in value) {
          inspect(child, outgoing);
        }
      } else if (value is Map) {
        if (value.containsKey('__composeRef')) {
          final ref = value['__composeRef'];
          if (ref is! List ||
              ref.length != 2 ||
              ref[0] is! String ||
              ref[1] is! int)
            throw const FormatException('Invalid Compose reference');
          final id = ref[0] as String, local = ref[1] as int;
          final target =
              sources[id] ??
              (!update.reset && !removed.contains(id)
                  ? _instances[id]?.data
                  : null);
          if (target == null ||
              local < 0 ||
              local >= program(target.programId).instructions.length)
            throw const FormatException('Unknown Compose dynamic address');
          outgoing.add(id);
        } else {
          for (final child in value.values) {
            inspect(child, outgoing);
          }
        }
      }
    }

    for (final source in update.instances) {
      final code = program(source.programId);
      if (source.values.length < code.requiredParameters)
        throw const FormatException('Missing Compose instance parameters');
      final outgoing = <String>{};
      inspect(source.values, outgoing);
      final patched = <(int, String)>{};
      for (final patch in source.patches) {
        if (!patched.add((patch.node, patch.field)))
          throw const FormatException('Duplicate Compose patch');
        inspect(patch.value, outgoing);
        if (patch.field == 'type' &&
                (patch.value is! String ||
                    !_composeNodeTypes.containsKey(
                      _normalizeToken(patch.value as String),
                    )) ||
            const {'props', 'slots'}.contains(patch.field) &&
                patch.value is! Map)
          throw const FormatException('Invalid Compose patch value');
        if (patch.node < 0 ||
            patch.node >= code.instructions.length ||
            !const {'type', 'props', 'children', 'slots'}.contains(patch.field))
          throw const FormatException('Invalid Compose public-node patch');
      }
      outgoing.remove(source.id);
      references[source.id] = outgoing;
      final previous = _instances[source.id];
      if (!update.reset &&
          previous != null &&
          previous.program.definition.id != source.programId)
        throw const FormatException(
          'Compose source program changed without a new identity',
        );
    }
    final rootSource =
        sources[update.root.instanceId] ??
        (!update.reset && !update.removed.contains(update.root.instanceId)
            ? _instances[update.root.instanceId]?.data
            : null);
    if (rootSource == null ||
        update.root.node < 0 ||
        update.root.node >= program(rootSource.programId).instructions.length)
      throw const FormatException('Invalid Compose root reference');
    for (final id in update.removed) {
      if (update.reset ||
          !_instances.containsKey(id) ||
          sources.containsKey(id))
        throw StateError('Invalid Compose source removal: $id');
    }
    for (final id in removed) {
      references.remove(id);
    }
    final active = <String>{}, visited = <String>{};

    /// Validates source membership without expanding any immutable layout instructions.
    void visit(String id) {
      if (active.contains(id))
        throw const FormatException('Compose instances form a cycle');
      if (!visited.add(id)) return;
      active.add(id);
      for (final target in references[id]!) {
        if (!references.containsKey(target))
          throw const FormatException(
            'Compose source retains a removed instance',
          );
        visit(target);
      }
      active.remove(id);
    }

    for (final id in references.keys) {
      visit(id);
    }
    if (update.reset) dispose();
    _references.addAll(references);
    for (final id in removed) {
      _references.remove(id);
    }
    _programs.addAll(definitions);
    final changed = <_ComposeProgramInstance>[];
    for (final source in update.instances) {
      final current = _instances[source.id];
      if (current == null) {
        _instances[source.id] = _ComposeProgramInstance(
          this,
          program(source.programId),
          source,
        );
      } else {
        current.replace(source);
        changed.add(current);
      }
    }
    for (final id in update.removed) {
      _instances[id]!.retire();
    }
    for (final id in update.removed) {
      _instances.remove(id)!.dispose();
    }
    _root = update.root;
    _revision = update.revision;
    for (final source in changed) {
      source.publish();
    }
  }

  /// Releases all parameter subscriptions when the owning session ends.
  void dispose() {
    for (final source in _instances.values) {
      source.retire();
    }
    for (final source in _instances.values) {
      source.dispose();
    }
    _instances.clear();
    _programs.clear();
    _references.clear();
    _root = null;
    _revision = 0;
  }
}

import 'compose_buffer_writer.dart';
import 'package:operit2/core/compose/compose_program_buffer.dart'
    as compose_buffer;
import 'dart:async';

import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as core;

/// Exercises the production retained session transport with explicit test renderers.
class ComposeSessionFixture {
  /// Receives plugin rendering and action adapters without interpreting their state.
  ComposeSessionFixture({required this.render, required this.action});

  final Map<String, Object?> Function(Map<String, Object?> state) render;
  final Future<Object?> Function(Map<String, Object?> state, String actionId)
  action;
  final Map<String, _FixtureSession> _sessions = {};
  final List<core.ToolPkgComposeDslCommand> commands = [];
  final Map<String, _FixtureSession> _requestSessions = {};

  /// Opens an independent page-owned state and update stream.
  String open() {
    final id = 'fixture-session-${_sessions.length}';
    _sessions[id] = _FixtureSession();
    return id;
  }

  /// Encodes typed session events through the actual generated watch protocol.
  Stream<CoreEvent> updates(CoreWatchRequest request) {
    final id = (request.args as Map)['sessionId'] as String;
    return _sessions[id]!.updates.stream.map(
      (event) => CoreEvent.raw(
        requestId: request.requestId,
        target: request.target,
        propertyName: request.propertyName,
        kind: 'Changed',
        decodeValue: decodeCoreLink<Object?>,
        valueBytes: encodeCoreLink(event.toJson()),
      ),
    );
  }

  /// Accepts only the current session command stream contract.
  Future<CorePushSink> submit(CorePushRequest request) async {
    if (request.methodName != 'submit') {
      throw StateError('Unexpected Compose push: ${request.methodName}');
    }
    final id = (request.args as Map)['sessionId'] as String;
    return _FixtureCommandSink(this, _sessions[id]!);
  }

  /// Retains host input between commands and publishes flat tree commits.
  Future<void> _handle(_FixtureSession session, Object? raw) async {
    final command = core.ToolPkgComposeDslCommand.fromJson(
      Map<String, Object?>.from(raw as Map),
    );
    commands.add(command);
    _requestSessions[command.requestId] = session;
    try {
      core.ToolPkgComposeDslTransaction? update;
      Object? result;
      switch (command.operation) {
        case 'render':
          final options = command.runtimeOptions;
          if (session.revision == 0) {
            session.state = Map<String, Object?>.from(options['state'] as Map);
          }
          if (options['__operit_update_inputs'] == true) {
            session.state.addAll(
              Map<String, Object?>.from(options['__operit_input_state'] as Map),
            );
          }
          update = composeFixtureUpdate(
            render(session.state),
            ++session.revision,
            reset: session.revision == 1,
            previousIds: session.sourceIds,
          );
          session.sourceIds = compose_buffer.ComposeProgramBuffer(
            update.buffer,
          ).transaction.instances.map((instance) => instance.id).toSet();
        case 'action':
          result = await action(session.state, command.actionId!);
        default:
          throw StateError(
            'Unexpected Compose operation: ${command.operation}',
          );
      }
      session.emit(
        core.ToolPkgComposeDslEvent(
          requestId: command.requestId,
          phase: 'final',
          update: update,
          actionResult: result,
          navigationCommands: const [],
          error: null,
        ),
      );
    } catch (error) {
      session.emit(
        core.ToolPkgComposeDslEvent(
          requestId: command.requestId,
          phase: 'error',
          update: null,
          actionResult: null,
          navigationCommands: const [],
          error: '$error',
        ),
      );
    }
    session.emit(
      core.ToolPkgComposeDslEvent(
        requestId: command.requestId,
        phase: 'complete',
        update: null,
        actionResult: null,
        navigationCommands: const [],
        error: null,
      ),
    );
  }

  /// Publishes a real incremental commit after its initiating command has completed.
  void publishDetachedUpdate(String requestId, Map<String, Object?> tree) {
    final session = _requestSessions[requestId]!;
    final update = composeFixtureUpdate(
      tree,
      ++session.revision,
      reset: false,
      previousIds: session.sourceIds,
    );
    session.sourceIds = compose_buffer.ComposeProgramBuffer(
      update.buffer,
    ).transaction.instances.map((instance) => instance.id).toSet();
    session.emit(
      core.ToolPkgComposeDslEvent(
        requestId: requestId,
        phase: 'intermediate',
        update: update,
        actionResult: null,
        navigationCommands: const [],
        error: null,
      ),
    );
  }

  /// Releases streams after all widget owners have been disposed.
  Future<void> close() async {
    for (final session in _sessions.values) {
      if (!session.updates.isClosed) await session.updates.close();
    }
  }
}

/// Keeps one embedded page's state separate from other packages and dialogs.
class _FixtureSession {
  final updates = StreamController<core.ToolPkgComposeDslEvent>.broadcast();
  Map<String, Object?> state = {};
  int revision = 0;
  Set<String> sourceIds = {};

  /// Drops delivery after the page has closed its command stream.
  void emit(core.ToolPkgComposeDslEvent event) {
    if (!updates.isClosed) updates.add(event);
  }
}

/// Submits commands independently so host input updates can overlap pending actions.
class _FixtureCommandSink implements CorePushSink {
  /// Binds one stream to its page session.
  _FixtureCommandSink(this.fixture, this.session);
  final ComposeSessionFixture fixture;
  final _FixtureSession session;

  /// Starts a real command without blocking later independent page commands.
  @override
  Future<void> add(Object? args) async =>
      unawaited(fixture._handle(session, args));

  /// Ends typed event delivery when the launcher releases this page.
  @override
  Future<void> close() => session.updates.close();
}

/// Encodes fixture layouts as stable dynamic instances on the sole production program ABI.
core.ToolPkgComposeDslTransaction composeFixtureUpdate(
  Map<String, Object?> tree,
  int revision, {
  bool reset = true,
  Set<String> previousIds = const {},
}) {
  final instances = <compose_buffer.ToolPkgComposeDslInstance>[];

  /// Encodes children at stable keyed or positional mount paths for native regression tests.
  Map<String, Object?> emit(Map raw, String path) {
    final props = Map<String, Object?>.from(raw['props'] as Map);

    /// Preserves sibling keys across reorder without coupling identity to a transaction revision.
    List<Object?> group(List children, String slot) => [
      for (var i = 0; i < children.length; i++)
        emit(
          children[i] as Map,
          '$path/$slot/${(children[i] as Map)['props']['key'] ?? i}',
        ),
    ];
    for (final entry in (raw['slots'] as Map).entries) {
      props[entry.key as String] = group(
        entry.value is List ? entry.value as List : [entry.value],
        entry.key as String,
      );
    }
    final children = group(raw['children'] as List, 'children');
    instances.add(
      compose_buffer.ToolPkgComposeDslInstance(
        id: path,
        programId: 'fixture:node',
        values: [props, children],
        patches: [
          compose_buffer.ToolPkgComposeDslPatch(
            node: 0,
            field: 'type',
            value: raw['type'],
          ),
        ],
      ),
    );
    return {
      '__composeRef': [path, 0],
    };
  }

  emit(tree, 'fixture:root');
  final ids = instances.map((instance) => instance.id).toSet();
  return composeBufferTransaction(
    reset: reset,
    revision: revision,
    root: const compose_buffer.ToolPkgComposeDslReference(
      instanceId: 'fixture:root',
      node: 0,
    ),
    programs: reset
        ? [
            const compose_buffer.ToolPkgComposeDslProgram(
              id: 'fixture:node',
              root: 0,
              nodes: [
                compose_buffer.ToolPkgComposeDslInstruction(
                  nodeType: 'Box',
                  props: [1, 0],
                  children: [1, 1],
                  explicitChildren: true,
                ),
              ],
            ),
          ]
        : const [],
    instances: instances,
    removed: reset ? const [] : previousIds.difference(ids).toList(),
  );
}

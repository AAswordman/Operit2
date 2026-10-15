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
      core.ToolPkgComposeDslNodeUpdate? update;
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
          );
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
    session.emit(
      core.ToolPkgComposeDslEvent(
        requestId: requestId,
        phase: 'intermediate',
        update: composeFixtureUpdate(tree, ++session.revision, reset: false),
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

/// Converts explicit test nodes into the production retained node update shape.
core.ToolPkgComposeDslNodeUpdate composeFixtureUpdate(
  Map<String, Object?> tree,
  int revision, {
  bool reset = true,
}) {
  final records = <core.ToolPkgComposeDslNodeRecord>[];

  /// Assigns stable structural IDs while preserving all declared props and slots.
  String visit(Map raw, String id) {
    final children = <String>[], slots = <String, List<String>>{};
    final rawChildren = raw['children'] as List;
    for (var index = 0; index < rawChildren.length; index++) {
      children.add(visit(rawChildren[index] as Map, '$id/child-$index'));
    }
    for (final entry in (raw['slots'] as Map).entries) {
      final slot = entry.value;
      final nodes = slot is List ? slot : [slot];
      slots[entry.key as String] = [
        for (var index = 0; index < nodes.length; index++)
          visit(nodes[index] as Map, '$id/slot-${entry.key}-$index'),
      ];
    }
    records.add(
      core.ToolPkgComposeDslNodeRecord(
        id: id,
        nodeType: raw['type'] as String,
        props: Map<String, Object?>.from(raw['props'] as Map),
        children: children,
        slots: slots,
      ),
    );
    return id;
  }

  final rootId = visit(tree, 'root');
  return core.ToolPkgComposeDslNodeUpdate(
    reset: reset,
    revision: revision,
    rootId: rootId,
    upserts: records,
    removed: const [],
  );
}

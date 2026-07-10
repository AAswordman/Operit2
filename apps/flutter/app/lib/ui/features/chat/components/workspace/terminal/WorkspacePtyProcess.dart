// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:operit2/core/bridge/ProxyCoreRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';

abstract class WorkspacePtyProcess {
  /// Returns the runtime session identifier.
  String get sessionId;

  /// Emits raw terminal output bytes.
  Stream<Uint8List> get output;

  /// Completes when the terminal process exits.
  Future<int> get exitCode;

  /// Writes raw input bytes to the terminal.
  void write(Uint8List data);

  /// Resizes the terminal viewport.
  void resize(int rows, int columns);

  /// Stops local listeners for this terminal process.
  void kill();
}

/// Starts a new workspace PTY session and attaches to its output stream.
Future<WorkspacePtyProcess> startWorkspacePty({
  required String sessionName,
  required String terminalType,
  required String workingDirectory,
  required int rows,
  required int columns,
}) async {
  final terminal = const GeneratedCoreProxyClients(
    ProxyCoreRuntimeBridge(),
  ).servicesRuntimeTerminalService;
  final sessionId = await terminal.startTerminalPty(
    sessionName: sessionName,
    terminalType: terminalType,
    workingDir: workingDirectory,
    rows: rows,
    cols: columns,
  );
  return _BridgeWorkspacePtyProcess(terminal, sessionId);
}

/// Attaches to an existing workspace PTY session.
WorkspacePtyProcess attachWorkspacePty(String sessionId) {
  final terminal = const GeneratedCoreProxyClients(
    ProxyCoreRuntimeBridge(),
  ).servicesRuntimeTerminalService;
  return _BridgeWorkspacePtyProcess(terminal, sessionId);
}

class _BridgeWorkspacePtyProcess implements WorkspacePtyProcess {
  /// Creates a process wrapper around a runtime terminal service session.
  _BridgeWorkspacePtyProcess(this._terminal, this._sessionId) {
    _outputSubscription = _terminal.bridge
        .watchStream(
          CoreWatchRequest(
            requestId: 'terminal-pty-${DateTime.now().microsecondsSinceEpoch}',
            targetPath: _terminal.targetPath,
            propertyName: 'terminalPtyOutput',
            args: <String, Object?>{'sessionId': _sessionId},
          ),
        )
        .listen(
          _handleEvent,
          onError: _handleOutputError,
          onDone: _handleOutputDone,
        );
  }

  final GeneratedServicesRuntimeTerminalServiceCoreProxy _terminal;
  final String _sessionId;
  final _output = StreamController<Uint8List>.broadcast();
  final _exitCode = Completer<int>();
  StreamSubscription<CoreEvent>? _outputSubscription;
  Timer? _resizeTimer;
  bool _closed = false;
  bool _finishingExit = false;
  int? _pendingRows;
  int? _pendingColumns;

  /// Returns the runtime session identifier.
  @override
  String get sessionId => _sessionId;

  /// Emits raw terminal output bytes.
  @override
  Stream<Uint8List> get output => _output.stream;

  /// Completes when the terminal process exits.
  @override
  Future<int> get exitCode => _exitCode.future;

  /// Writes raw input bytes to the terminal.
  @override
  void write(Uint8List data) {
    if (_closed) {
      return;
    }
    unawaited(
      _terminal.writeTerminalPty(
        sessionId: _sessionId,
        dataBase64: base64Encode(data),
      ),
    );
  }

  /// Resizes the terminal viewport.
  @override
  void resize(int rows, int columns) {
    if (_closed) {
      return;
    }
    _pendingRows = rows;
    _pendingColumns = columns;
    _resizeTimer?.cancel();
    _resizeTimer = Timer(const Duration(milliseconds: 80), _flushResize);
  }

  /// Flushes the latest terminal size after resize events settle briefly.
  void _flushResize() {
    if (_closed) {
      return;
    }
    final rows = _pendingRows;
    final columns = _pendingColumns;
    if (rows == null || columns == null) {
      return;
    }
    unawaited(
      _terminal.resizeTerminalPty(
        sessionId: _sessionId,
        rows: rows,
        cols: columns,
      ),
    );
  }

  /// Stops local listeners for this terminal process.
  @override
  void kill() {
    if (_closed) {
      return;
    }
    _closed = true;
    unawaited(_outputSubscription?.cancel());
    _resizeTimer?.cancel();
    unawaited(_output.close());
    if (!_exitCode.isCompleted) {
      _exitCode.complete(-1);
    }
  }

  /// Handles terminal output stream events from the runtime bridge.
  void _handleEvent(CoreEvent event) {
    if (_closed || _output.isClosed) {
      return;
    }
    switch (event.kind) {
      case 'Changed':
        _handleOutputValue(event.value);
        return;
      case 'Completed':
        unawaited(_finishWithExit());
        return;
      default:
        _handleOutputError(
          StateError('Unexpected terminal PTY event kind: ${event.kind}'),
          StackTrace.current,
        );
    }
  }

  /// Decodes a terminal output payload and emits the bytes to listeners.
  void _handleOutputValue(Object? value) {
    if (_closed || _output.isClosed) {
      return;
    }
    if (value is! String) {
      _handleOutputError(
        StateError('Terminal PTY output event is not a string'),
        StackTrace.current,
      );
      return;
    }
    final dataBase64 = value;
    if (dataBase64.isNotEmpty) {
      _output.add(base64Decode(dataBase64));
    }
  }

  /// Converts stream errors into terminal process completion.
  void _handleOutputError(Object error, StackTrace stackTrace) {
    if (!_closed && !_output.isClosed) {
      _output.addError(error, stackTrace);
    }
    unawaited(_finishWithExit());
  }

  /// Starts process completion when the runtime output stream ends.
  void _handleOutputDone() {
    unawaited(_finishWithExit());
  }

  /// Polls the runtime for the terminal exit code and closes the host session.
  Future<void> _finishWithExit() async {
    if (_finishingExit) {
      return;
    }
    _finishingExit = true;
    _closed = true;
    _outputSubscription = null;
    _resizeTimer?.cancel();
    try {
      final code = await _terminal.pollTerminalPtyExit(sessionId: _sessionId);
      await _terminal.closeTerminalPty(sessionId: _sessionId);
      await _output.close();
      if (!_exitCode.isCompleted) {
        _exitCode.complete(code ?? -1);
      }
    } catch (error, stackTrace) {
      if (!_output.isClosed) {
        await _output.close();
      }
      if (!_exitCode.isCompleted) {
        _exitCode.completeError(error, stackTrace);
      }
    }
  }
}

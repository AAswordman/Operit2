// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:operit2/core/bridge/ProxyCoreRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';

import 'WorkspacePtyProcess.dart';

class _BridgeWorkspacePtyProcess implements WorkspacePtyProcess {
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

  @override
  String get sessionId => _sessionId;

  @override
  Stream<Uint8List> get output => _output.stream;

  @override
  Future<int> get exitCode => _exitCode.future;

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

  void _handleOutputError(Object error, StackTrace stackTrace) {
    if (!_closed && !_output.isClosed) {
      _output.addError(error, stackTrace);
    }
    unawaited(_finishWithExit());
  }

  void _handleOutputDone() {
    unawaited(_finishWithExit());
  }

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
      if (code != null) {
        if (!_exitCode.isCompleted) {
          _exitCode.complete(code);
        }
      } else {
        if (!_exitCode.isCompleted) {
          _exitCode.complete(-1);
        }
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

Future<WorkspacePtyProcess> startWorkspacePtyImpl({
  required String sessionName,
  required String workingDirectory,
  required int rows,
  required int columns,
}) async {
  final terminal = const GeneratedCoreProxyClients(
    ProxyCoreRuntimeBridge(),
  ).servicesRuntimeTerminalService;
  final sessionId = await terminal.startTerminalPty(
    sessionName: sessionName,
    workingDir: workingDirectory,
    rows: rows,
    cols: columns,
  );
  return _BridgeWorkspacePtyProcess(terminal, sessionId);
}

WorkspacePtyProcess attachWorkspacePtyImpl(String sessionId) {
  final terminal = const GeneratedCoreProxyClients(
    ProxyCoreRuntimeBridge(),
  ).servicesRuntimeTerminalService;
  return _BridgeWorkspacePtyProcess(terminal, sessionId);
}

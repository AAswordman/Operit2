// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:operit2/core/bridge/ProxyCoreRuntimeBridge.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';

import 'WorkspacePtyProcess.dart';

class _BridgeWorkspacePtyProcess implements WorkspacePtyProcess {
  _BridgeWorkspacePtyProcess(this._terminal, this._sessionId) {
    _outputSubscription = _terminal
        .terminalPtyOutputChanges(sessionId: _sessionId)
        .listen(_handleOutput, onError: _handleOutputError);
    _exitTimer = Timer.periodic(
      const Duration(milliseconds: 250),
      (_) => unawaited(_pollExit()),
    );
  }

  final GeneratedRepositoryRuntimeTerminalServiceCoreProxy _terminal;
  final String _sessionId;
  final _output = StreamController<Uint8List>.broadcast();
  final _exitCode = Completer<int>();
  StreamSubscription<Object?>? _outputSubscription;
  Timer? _exitTimer;
  Timer? _resizeTimer;
  bool _closed = false;
  bool _pollingExit = false;
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
    _exitTimer?.cancel();
    _resizeTimer?.cancel();
    unawaited(_output.close());
    if (!_exitCode.isCompleted) {
      _exitCode.complete(-1);
    }
  }

  void _handleOutput(Object? value) {
    if (_closed || _output.isClosed) {
      return;
    }
    final dataBase64 = value as String;
    if (dataBase64.isNotEmpty) {
      _output.add(base64Decode(dataBase64));
    }
  }

  void _handleOutputError(Object error, StackTrace stackTrace) {
    if (!_closed && !_output.isClosed) {
      _output.addError(error, stackTrace);
    }
  }

  Future<void> _pollExit() async {
    if (_closed || _pollingExit) {
      return;
    }
    _pollingExit = true;
    try {
      final code = await _terminal.pollTerminalPtyExit(sessionId: _sessionId);
      if (code != null) {
        await _outputSubscription?.cancel();
        _outputSubscription = null;
        _exitTimer?.cancel();
        _closed = true;
        await _terminal.closeTerminalPty(sessionId: _sessionId);
        await _output.close();
        if (!_exitCode.isCompleted) {
          _exitCode.complete(code);
        }
      }
    } catch (error, stackTrace) {
      if (!_exitCode.isCompleted) {
        _exitCode.completeError(error, stackTrace);
      }
    } finally {
      _pollingExit = false;
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
  ).repositoryRuntimeTerminalService;
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
  ).repositoryRuntimeTerminalService;
  return _BridgeWorkspacePtyProcess(terminal, sessionId);
}

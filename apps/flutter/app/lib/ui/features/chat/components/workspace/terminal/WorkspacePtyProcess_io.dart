// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/services.dart';

import 'WorkspacePtyProcess.dart';

class _BridgeWorkspacePtyProcess implements WorkspacePtyProcess {
  _BridgeWorkspacePtyProcess(this._channel, this._sessionId) {
    _readTimer = Timer.periodic(
      const Duration(milliseconds: 40),
      (_) => unawaited(_readOutput()),
    );
    _exitTimer = Timer.periodic(
      const Duration(milliseconds: 250),
      (_) => unawaited(_pollExit()),
    );
  }

  final MethodChannel _channel;
  final String _sessionId;
  final _output = StreamController<Uint8List>.broadcast();
  final _exitCode = Completer<int>();
  Timer? _readTimer;
  Timer? _exitTimer;
  Timer? _resizeTimer;
  bool _closed = false;
  bool _reading = false;
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
      _invokeJson('writeTerminalPty', <String, Object>{
        'sessionId': _sessionId,
        'data': data,
      }),
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
      _invokeJson('resizeTerminalPty', <String, Object>{
        'sessionId': _sessionId,
        'rows': rows,
        'columns': columns,
      }),
    );
  }

  @override
  void kill() {
    if (_closed) {
      return;
    }
    _closed = true;
    _readTimer?.cancel();
    _exitTimer?.cancel();
    _resizeTimer?.cancel();
    unawaited(_output.close());
    if (!_exitCode.isCompleted) {
      _exitCode.complete(-1);
    }
  }

  Future<void> _readOutput() async {
    if (_closed || _reading) {
      return;
    }
    _reading = true;
    try {
      final response = await _invokeJson('readTerminalPty', _sessionId);
      final dataBase64 = response['dataBase64'];
      if (dataBase64 is String && dataBase64.isNotEmpty && !_output.isClosed) {
        final data = base64Decode(dataBase64);
        _output.add(data);
      }
    } catch (error, stackTrace) {
      if (!_closed && !_output.isClosed) {
        _output.addError(error, stackTrace);
      }
    } finally {
      _reading = false;
    }
  }

  Future<void> _pollExit() async {
    if (_closed || _pollingExit) {
      return;
    }
    _pollingExit = true;
    try {
      final response = await _invokeJson('pollTerminalPtyExit', _sessionId);
      final code = response['exitCode'];
      if (code is int) {
        _readTimer?.cancel();
        _exitTimer?.cancel();
        while (_reading) {
          await Future<void>.delayed(const Duration(milliseconds: 10));
        }
        await _readOutput();
        _closed = true;
        await _invokeJson('closeTerminalPty', _sessionId);
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

  Future<Map<String, dynamic>> _invokeJson(String method, Object? args) async {
    final raw = await _channel.invokeMethod<String>(method, args);
    if (raw == null) {
      throw StateError('$method returned null');
    }
    final decoded = jsonDecode(raw);
    if (decoded is! Map<String, dynamic>) {
      throw StateError('$method returned non-object JSON');
    }
    if (decoded['ok'] != true) {
      throw StateError(decoded['error']?.toString() ?? '$method failed');
    }
    return decoded;
  }
}

Future<WorkspacePtyProcess> startWorkspacePtyImpl({
  required String sessionName,
  required String workingDirectory,
  required int rows,
  required int columns,
}) async {
  return _startBridgeWorkspacePty(
    sessionName: sessionName,
    workingDirectory: workingDirectory,
    rows: rows,
    columns: columns,
  );
}

Future<WorkspacePtyProcess> _startBridgeWorkspacePty({
  required String sessionName,
  required String workingDirectory,
  required int rows,
  required int columns,
}) async {
  const channel = MethodChannel('operit/runtime');
  final raw = await channel
      .invokeMethod<String>('startTerminalPty', <String, Object>{
        'sessionName': sessionName,
        'workingDirectory': workingDirectory,
        'rows': rows,
        'columns': columns,
      });
  if (raw == null) {
    throw StateError('startTerminalPty returned null');
  }
  final decoded = jsonDecode(raw);
  if (decoded is! Map<String, dynamic>) {
    throw StateError('startTerminalPty returned non-object JSON');
  }
  if (decoded['ok'] != true) {
    throw StateError(decoded['error']?.toString() ?? 'startTerminalPty failed');
  }
  final sessionId = decoded['sessionId'];
  if (sessionId is! String || sessionId.isEmpty) {
    throw StateError('startTerminalPty missing sessionId');
  }
  return _BridgeWorkspacePtyProcess(channel, sessionId);
}

WorkspacePtyProcess attachWorkspacePtyImpl(String sessionId) {
  const channel = MethodChannel('operit/runtime');
  return _BridgeWorkspacePtyProcess(channel, sessionId);
}

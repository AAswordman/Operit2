// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter_pty/flutter_pty.dart';
import 'package:flutter/services.dart';

import 'WorkspacePtyProcess.dart';

class _FlutterWorkspacePtyProcess implements WorkspacePtyProcess {
  _FlutterWorkspacePtyProcess(this._pty);

  final Pty _pty;

  @override
  Stream<Uint8List> get output => _pty.output;

  @override
  Future<int> get exitCode => _pty.exitCode;

  @override
  void write(Uint8List data) {
    _pty.write(data);
  }

  @override
  void resize(int rows, int columns) {
    _pty.resize(rows, columns);
  }

  @override
  void kill() {
    _pty.kill();
  }
}

class _AndroidWorkspacePtyProcess implements WorkspacePtyProcess {
  _AndroidWorkspacePtyProcess(this._channel, this._sessionId) {
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
  bool _closed = false;
  bool _reading = false;
  bool _pollingExit = false;

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
    unawaited(_invokeJson('closeTerminalPty', _sessionId));
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
      final rawData = response['data'];
      if (rawData is List && rawData.isNotEmpty && !_output.isClosed) {
        final data = Uint8List.fromList(rawData.cast<int>());
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
  required String workingDirectory,
  required int rows,
  required int columns,
}) async {
  if (Platform.isAndroid) {
    return _startAndroidWorkspacePty(
      workingDirectory: workingDirectory,
      rows: rows,
      columns: columns,
    );
  }
  final shell = await _workspaceShell(workingDirectory);
  final pty = Pty.start(
    shell.executable,
    arguments: shell.arguments,
    workingDirectory: shell.workingDirectory,
    rows: rows,
    columns: columns,
    environment: shell.environment,
  );
  return _FlutterWorkspacePtyProcess(pty);
}

Future<WorkspacePtyProcess> _startAndroidWorkspacePty({
  required String workingDirectory,
  required int rows,
  required int columns,
}) async {
  const channel = MethodChannel('operit/runtime');
  final raw = await channel.invokeMethod<String>(
    'startTerminalPty',
    <String, Object>{
      'workingDirectory': workingDirectory,
      'rows': rows,
      'columns': columns,
    },
  );
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
  return _AndroidWorkspacePtyProcess(channel, sessionId);
}

Future<
  ({
    String executable,
    List<String> arguments,
    String workingDirectory,
    Map<String, String> environment,
  })
>
_workspaceShell(String workingDirectory) async {
  if (Platform.isWindows) {
    return (
      executable: 'powershell.exe',
      arguments: <String>['-NoLogo', '-NoProfile'],
      workingDirectory: workingDirectory,
      environment: _workspaceTerminalEnvironment(),
    );
  }
  return (
    executable: Platform.environment['SHELL']?.trim().isNotEmpty == true
        ? Platform.environment['SHELL']!
        : 'sh',
    arguments: const <String>[],
    workingDirectory: workingDirectory,
    environment: _workspaceTerminalEnvironment(),
  );
}

Map<String, String> _workspaceTerminalEnvironment({
  Map<String, String> extra = const <String, String>{},
}) {
  final environment = Platform.isWindows
      ? Map<String, String>.of(Platform.environment)
      : <String, String>{};
  environment.addAll(<String, String>{
    'TERM': 'xterm-256color',
    'COLORTERM': 'truecolor',
    'LANG': Platform.environment['LANG']?.trim().isNotEmpty == true
        ? Platform.environment['LANG']!
        : 'en_US.UTF-8',
  });
  environment.addAll(extra);
  return environment;
}

// ignore_for_file: file_names

import 'dart:async';
import 'dart:io';

import '../path/OperitClientPaths.dart';
import 'ClientLogLevel.dart';

File? _logFile;
Future<void> _writeQueue = Future<void>.value();
String? _lastWriteError;

Future<void> initialize() async {
  final path = await _resolveLogFilePath();
  final file = File(path);
  await file.parent.create(recursive: true);
  await file.open(mode: FileMode.append).then((handle) => handle.close());
  _logFile = file;
}

/// Returns whether the file logger has an active log file.
bool isInitialized() => _logFile != null;

/// Returns whether this backend already mirrors writes to a live console.
bool writesToConsole() => false;

Future<String> logFilePath() async {
  final file = _logFile;
  if (file != null) {
    return file.path;
  }
  return _resolveLogFilePath();
}

Future<String> readText() async {
  final file = _requireLogFile();
  return file.readAsString();
}

String? lastWriteError() => _lastWriteError;

Future<void> clear() async {
  final file = _requireLogFile();
  await file.writeAsString('');
}

void write(
  ClientLogLevel level,
  String message, {
  Object? error,
  StackTrace? stackTrace,
}) {
  final file = _requireLogFile();
  final line = _formatLogLine(
    level,
    message,
    error: error,
    stackTrace: stackTrace,
  );
  _writeQueue = _writeQueue
      .then((_) => file.writeAsString(line, mode: FileMode.append))
      .then<void>((_) {
        _lastWriteError = null;
      })
      .catchError((Object writeError, StackTrace writeStackTrace) {
        _lastWriteError = _formatWriteError(writeError, writeStackTrace);
      });
  unawaited(_writeQueue);
}

File _requireLogFile() {
  final file = _logFile;
  if (file == null) {
    throw StateError('ClientLogger is not initialized');
  }
  return file;
}

Future<String> _resolveLogFilePath() async {
  if (Platform.isAndroid) {
    return _clientLogPath();
  }
  if (Platform.isWindows) {
    return _clientLogPath();
  }
  if (Platform.isLinux) {
    return _clientLogPath();
  }
  if (Platform.isMacOS) {
    return _clientLogPath();
  }
  if (Platform.isIOS) {
    return _clientLogPath();
  }
  throw UnsupportedError(
    'ClientLogger file logging is not supported on ${Platform.operatingSystem}',
  );
}

Future<String> _clientLogPath() async {
  return (await OperitClientPaths.clientLogFile()).path;
}

String _formatLogLine(
  ClientLogLevel level,
  String message, {
  Object? error,
  StackTrace? stackTrace,
}) {
  final buffer = StringBuffer(message);
  if (error != null) {
    buffer
      ..writeln()
      ..write(error);
  }
  if (stackTrace != null) {
    buffer
      ..writeln()
      ..write(stackTrace);
  }
  buffer.writeln();
  return buffer.toString();
}

String _formatWriteError(Object error, StackTrace stackTrace) {
  return '$error\n$stackTrace';
}

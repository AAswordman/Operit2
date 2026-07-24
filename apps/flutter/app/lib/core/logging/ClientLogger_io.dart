// ignore_for_file: file_names

import 'dart:async';

import '../bridge/PlatformCoreProxy.dart';
import '../bridge/ProxyCoreRuntimeBridge.dart';
import '../proxy/generated/CoreProxyClients.g.dart';
import 'ClientLogLevel.dart';

const GeneratedCoreProxyClients _clients = GeneratedCoreProxyClients(
  ProxyCoreRuntimeBridge(coreProxy: platformCoreProxy),
);

const String _memoryLogPath = 'memory:client.log';
bool _initialized = false;
String? _logPath;
bool _persistentStorageRequested = false;
final List<String> _pendingLines = <String>[];
Future<void> _writeQueue = Future<void>.value();
String? _lastWriteError;

/// Initializes the memory-backed logger backend.
Future<void> initialize() async {
  _initialized = true;
}

/// Requests the runtime-storage logger sink for future writes.
void attachPersistentStorage() {
  _requireInitialized();
  _persistentStorageRequested = true;
}

/// Returns whether the logger backend is ready to accept writes.
bool isInitialized() => _initialized;

/// Returns whether this backend already mirrors writes to a live console.
bool writesToConsole() => false;

/// Returns the active logger sink path.
Future<String> logFilePath() async {
  _requireInitialized();
  return _logPath ?? _memoryLogPath;
}

/// Reads the complete client log text from the active sink.
Future<String> readText() async {
  _requireInitialized();
  await _writeQueue;
  final path = _logPath;
  if (path == null) {
    return _pendingLines.join();
  }
  final content = await _clients.repositoryRuntimeStorageRepository.readText(
    path: path,
  );
  if (content == null) {
    throw StateError('ClientLogger runtime log is missing: $path');
  }
  return '$content${_pendingLines.join()}';
}

/// Returns the latest asynchronous runtime-storage write error text.
String? lastWriteError() => _lastWriteError;

/// Clears the active client log file.
Future<void> clear() async {
  _requireInitialized();
  _writeQueue = _writeQueue.then((_) async {
    _pendingLines.clear();
    final path = _logPath;
    if (path != null) {
      await _clients.repositoryRuntimeStorageRepository.writeText(
        path: path,
        content: '',
      );
    }
  });
  await _writeQueue;
}

/// Appends one formatted log entry to memory and the runtime sink when attached.
void write(
  ClientLogLevel level,
  String message, {
  Object? error,
  StackTrace? stackTrace,
}) {
  _requireInitialized();
  final line = _formatLogLine(message, error: error, stackTrace: stackTrace);
  _pendingLines.add(line);
  _schedulePersistentWrite();
}

/// Schedules one ordered write pass for the runtime-storage sink.
void _schedulePersistentWrite() {
  _writeQueue = _writeQueue
      .then((_) => _flushPendingLines())
      .then<void>((_) {
        _lastWriteError = null;
      })
      .catchError((Object writeError, StackTrace writeStackTrace) {
        _lastWriteError = _formatWriteError(writeError, writeStackTrace);
      });
  unawaited(_writeQueue);
}

/// Verifies that the logger backend has been initialized.
void _requireInitialized() {
  if (!_initialized) {
    throw StateError('ClientLogger is not initialized');
  }
}

/// Attaches the runtime-storage sink through the local Core proxy.
Future<void> _attachPersistentStorageNow() async {
  final path = await _clients.repositoryRuntimeStorageRepository
      .clientLogPath();
  final existing = await _clients.repositoryRuntimeStorageRepository.readText(
    path: path,
  );
  if (existing == null) {
    await _clients.repositoryRuntimeStorageRepository.writeText(
      path: path,
      content: '',
    );
  }
  _logPath = path;
}

/// Flushes buffered lines into the runtime-storage sink.
Future<void> _flushPendingLines() async {
  if (_persistentStorageRequested && _logPath == null) {
    await _attachPersistentStorageNow();
  }
  final path = _logPath;
  if (path == null || _pendingLines.isEmpty) {
    return;
  }
  final count = _pendingLines.length;
  final text = _pendingLines.take(count).join();
  await _appendLogText(path, text);
  _pendingLines.removeRange(0, count);
}

/// Appends text through runtime storage VFS.
Future<void> _appendLogText(String path, String text) async {
  final content = await _clients.repositoryRuntimeStorageRepository.readText(
    path: path,
  );
  if (content == null) {
    throw StateError('ClientLogger runtime log is missing: $path');
  }
  await _clients.repositoryRuntimeStorageRepository.writeText(
    path: path,
    content: '$content$text',
  );
}

/// Formats one log message with optional error details.
String _formatLogLine(String message, {Object? error, StackTrace? stackTrace}) {
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

/// Formats a write failure for diagnostics.
String _formatWriteError(Object error, StackTrace stackTrace) {
  return '$error\n$stackTrace';
}

// ignore_for_file: file_names

import 'ClientLogLevel.dart';

Future<void> initialize() async {
  throw UnsupportedError('ClientLogger file logging is not supported on web');
}

bool isInitialized() => false;

Future<String> logFilePath() async {
  throw UnsupportedError('ClientLogger file logging is not supported on web');
}

Future<String> readText() async {
  throw UnsupportedError('ClientLogger file logging is not supported on web');
}

String? lastWriteError() => null;

Future<void> clear() async {
  throw UnsupportedError('ClientLogger file logging is not supported on web');
}

void write(
  ClientLogLevel level,
  String message, {
  Object? error,
  StackTrace? stackTrace,
}) {
  throw UnsupportedError('ClientLogger file logging is not supported on web');
}

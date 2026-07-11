// ignore_for_file: file_names

import 'dart:convert';
import 'dart:html' as html;

import 'ClientLogLevel.dart';

const String _logStorageKey = 'operit2.client.log';
bool _initialized = false;

Future<void> initialize() async {
  _initialized = true;
}

/// Returns whether the browser logger has been initialized.
bool isInitialized() => _initialized;

/// Returns whether this backend already mirrors writes to a live console.
bool writesToConsole() => true;

Future<String> logFilePath() async {
  return 'localStorage:$_logStorageKey';
}

Future<String> readText() async {
  return html.window.localStorage[_logStorageKey] ?? '';
}

String? lastWriteError() => null;

Future<void> clear() async {
  html.window.localStorage.remove(_logStorageKey);
}

void write(
  ClientLogLevel level,
  String message, {
  Object? error,
  StackTrace? stackTrace,
}) {
  final buffer = StringBuffer(message);
  if (error != null) {
    buffer
      ..write(' error=')
      ..write(error);
  }
  if (stackTrace != null) {
    buffer
      ..write('\n')
      ..write(stackTrace);
  }
  final text = buffer.toString();
  switch (level) {
    case ClientLogLevel.error:
    case ClientLogLevel.assert_:
      html.window.console.error(text);
    case ClientLogLevel.warn:
      html.window.console.warn(text);
    case ClientLogLevel.verbose:
    case ClientLogLevel.debug:
    case ClientLogLevel.info:
      html.window.console.log(text);
  }
  final current = html.window.localStorage[_logStorageKey] ?? '';
  final next = current.isEmpty ? text : '$current\n$text';
  html.window.localStorage[_logStorageKey] = _trimUtf8(next, 256 * 1024);
}

String _trimUtf8(String value, int maxBytes) {
  var bytes = utf8.encode(value);
  if (bytes.length <= maxBytes) {
    return value;
  }
  bytes = bytes.sublist(bytes.length - maxBytes);
  return utf8.decode(bytes, allowMalformed: true);
}

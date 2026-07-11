// ignore_for_file: file_names

import 'ClientLogger_io.dart'
    if (dart.library.html) 'ClientLogger_web.dart'
    as platform;
import 'ClientLogLevel.dart';

class ClientLogger {
  const ClientLogger._();

  /// Initializes the client logger backend and records the active sink path.
  static Future<void> initialize() {
    return platform.initialize().then((_) async {
      i('initialized path=${await logFilePath()}', tag: 'ClientLogger');
    });
  }

  /// Returns whether the logger backend is ready to accept writes.
  static bool get isInitialized => platform.isInitialized();

  /// Returns the active client log sink path.
  static Future<String> logFilePath() {
    return platform.logFilePath();
  }

  /// Reads all persisted client log text.
  static Future<String> readText() {
    return platform.readText();
  }

  /// Returns the latest backend write error, if one exists.
  static String? lastWriteError() {
    return platform.lastWriteError();
  }

  /// Clears all persisted client log text.
  static Future<void> clear() {
    return platform.clear();
  }

  /// Writes a verbose client log entry.
  static void v(
    String message, {
    String tag = 'Client',
    Object? error,
    StackTrace? stackTrace,
  }) {
    write(
      ClientLogLevel.verbose,
      message,
      tag: tag,
      error: error,
      stackTrace: stackTrace,
    );
  }

  /// Writes a debug client log entry.
  static void d(
    String message, {
    String tag = 'Client',
    Object? error,
    StackTrace? stackTrace,
  }) {
    write(
      ClientLogLevel.debug,
      message,
      tag: tag,
      error: error,
      stackTrace: stackTrace,
    );
  }

  /// Writes an info client log entry.
  static void i(
    String message, {
    String tag = 'Client',
    Object? error,
    StackTrace? stackTrace,
  }) {
    write(
      ClientLogLevel.info,
      message,
      tag: tag,
      error: error,
      stackTrace: stackTrace,
    );
  }

  /// Writes a warning client log entry.
  static void w(
    String message, {
    String tag = 'Client',
    Object? error,
    StackTrace? stackTrace,
  }) {
    write(
      ClientLogLevel.warn,
      message,
      tag: tag,
      error: error,
      stackTrace: stackTrace,
    );
  }

  /// Writes an error client log entry.
  static void e(
    String message, {
    String tag = 'Client',
    Object? error,
    StackTrace? stackTrace,
  }) {
    write(
      ClientLogLevel.error,
      message,
      tag: tag,
      error: error,
      stackTrace: stackTrace,
    );
  }

  /// Writes an assertion-level client log entry.
  static void wtf(
    String message, {
    String tag = 'Client',
    Object? error,
    StackTrace? stackTrace,
  }) {
    write(
      ClientLogLevel.assert_,
      message,
      tag: tag,
      error: error,
      stackTrace: stackTrace,
    );
  }

  /// Writes one structured client log entry through the active backend.
  static void write(
    ClientLogLevel level,
    String message, {
    String tag = 'Client',
    Object? error,
    StackTrace? stackTrace,
  }) {
    final text = formatEntry(
      level: level,
      tag: tag,
      message: message,
      error: error,
      stackTrace: stackTrace,
    );
    platform.write(level, text);
    if (!platform.writesToConsole()) {
      _writeConsoleText(text);
    }
  }

  /// Formats one complete client log entry with every line tagged.
  static String formatEntry({
    required ClientLogLevel level,
    required String tag,
    required String message,
    Object? error,
    StackTrace? stackTrace,
  }) {
    final timestamp = _consoleTimestamp(DateTime.now());
    final prefix = '$timestamp ${level.code}/$tag: ';
    final lines = <String>[
      ..._prefixPayloadLines(prefix, message),
      if (error != null) ..._prefixPayloadLines(prefix, 'error=$error'),
      if (stackTrace != null)
        ..._prefixPayloadLines(prefix, 'stackTrace=$stackTrace'),
    ];
    return lines.join('\n');
  }

  /// Writes one formatted log entry to stdout for live desktop debugging.
  static void _writeConsoleText(String message) {
    // ignore: avoid_print
    print(message);
  }

  /// Prefixes each payload line with one structured log prefix.
  static List<String> _prefixPayloadLines(String prefix, String payload) {
    final lines = payload.split('\n');
    if (lines.isEmpty) {
      return <String>[prefix];
    }
    return lines.map((line) => '$prefix$line').toList(growable: false);
  }

  /// Formats local wall-clock time for live console diagnostics.
  static String _consoleTimestamp(DateTime value) {
    final local = value.toLocal();
    final hour = local.hour.toString().padLeft(2, '0');
    final minute = local.minute.toString().padLeft(2, '0');
    final second = local.second.toString().padLeft(2, '0');
    final millisecond = local.millisecond.toString().padLeft(3, '0');
    return '$hour:$minute:$second.$millisecond';
  }
}

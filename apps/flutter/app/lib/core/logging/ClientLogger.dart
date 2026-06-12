// ignore_for_file: file_names

import 'ClientLogger_io.dart'
    if (dart.library.html) 'ClientLogger_web.dart'
    as platform;
import 'ClientLogLevel.dart';

class ClientLogger {
  const ClientLogger._();

  static Future<void> initialize() {
    return platform.initialize();
  }

  static bool get isInitialized => platform.isInitialized();

  static Future<String> logFilePath() {
    return platform.logFilePath();
  }

  static Future<String> readText() {
    return platform.readText();
  }

  static String? lastWriteError() {
    return platform.lastWriteError();
  }

  static Future<void> clear() {
    return platform.clear();
  }

  static void v(String message, {Object? error, StackTrace? stackTrace}) {
    write(
      ClientLogLevel.verbose,
      message,
      error: error,
      stackTrace: stackTrace,
    );
  }

  static void d(String message, {Object? error, StackTrace? stackTrace}) {
    write(ClientLogLevel.debug, message, error: error, stackTrace: stackTrace);
  }

  static void i(String message, {Object? error, StackTrace? stackTrace}) {
    write(ClientLogLevel.info, message, error: error, stackTrace: stackTrace);
  }

  static void w(String message, {Object? error, StackTrace? stackTrace}) {
    write(ClientLogLevel.warn, message, error: error, stackTrace: stackTrace);
  }

  static void e(String message, {Object? error, StackTrace? stackTrace}) {
    write(ClientLogLevel.error, message, error: error, stackTrace: stackTrace);
  }

  static void wtf(String message, {Object? error, StackTrace? stackTrace}) {
    write(
      ClientLogLevel.assert_,
      message,
      error: error,
      stackTrace: stackTrace,
    );
  }

  static void write(
    ClientLogLevel level,
    String message, {
    Object? error,
    StackTrace? stackTrace,
  }) {
    platform.write(level, message, error: error, stackTrace: stackTrace);
  }
}

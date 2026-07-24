import 'dart:async';
import 'dart:ui' show PlatformDispatcher;

import 'package:flutter/material.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';

import 'core/application/CoreApplicationService.dart';
import 'core/errors/UnhandledErrorReporter.dart';
import 'core/logging/ClientLogger.dart';
import 'core/runtime/RuntimeConnectionManager.dart';
import 'ui/main/OperitApp.dart';
import 'ui/window/DetachedChatWindowApp.dart';
import 'ui/window/OperitWindowArguments.dart';
import 'ui/window/OperitWindowPlatform.dart';

const String _appStartupLogTag = 'AppStartup';

/// Runs the application startup sequence with structured diagnostics.
void main(List<String> _) async {
  late Zone startupZone;
  await runZonedGuarded(
    () async {
      startupZone = Zone.current;
      final startupStopwatch = Stopwatch()..start();
      final bindingStopwatch = Stopwatch()..start();
      WidgetsFlutterBinding.ensureInitialized();
      final bindingElapsedMs = bindingStopwatch.elapsedMilliseconds;
      final loggerStopwatch = Stopwatch()..start();
      await ClientLogger.initialize();
      ClientLogger.i(
        'widgets binding initialized elapsedMs=$bindingElapsedMs',
        tag: _appStartupLogTag,
      );
      ClientLogger.i(
        'client logger initialized elapsedMs=${loggerStopwatch.elapsedMilliseconds}',
        tag: _appStartupLogTag,
      );
      final hooksStopwatch = Stopwatch()..start();
      _installClientLogHooks();
      ClientLogger.i(
        'client log hooks installed elapsedMs=${hooksStopwatch.elapsedMilliseconds}',
        tag: _appStartupLogTag,
      );
      final runtimeStopwatch = Stopwatch()..start();
      await RuntimeConnectionManager.instance.initialize();
      ClientLogger.attachPersistentStorage();
      ClientLogger.i(
        'runtime connection initialized elapsedMs=${runtimeStopwatch.elapsedMilliseconds}',
        tag: _appStartupLogTag,
      );
      final glassStopwatch = Stopwatch()..start();
      await LiquidGlassWidgets.initialize();
      ClientLogger.i(
        'liquid glass initialized elapsedMs=${glassStopwatch.elapsedMilliseconds}',
        tag: _appStartupLogTag,
      );
      final windowStopwatch = Stopwatch()..start();
      final windowArguments = await readOperitWindowArguments();
      ClientLogger.i(
        'window arguments read type=${windowArguments.runtimeType} elapsedMs=${windowStopwatch.elapsedMilliseconds}',
        tag: _appStartupLogTag,
      );
      switch (windowArguments) {
        case MainWindowArguments():
          final coreStopwatch = Stopwatch()..start();
          CoreApplicationService.instance.initialize();
          ClientLogger.i(
            'core application initialize dispatched elapsedMs=${coreStopwatch.elapsedMilliseconds}',
            tag: _appStartupLogTag,
          );
          _runMainWindow();
        case final DetachedChatWindowArguments detachedArguments:
          _runDetachedChatWindow(detachedArguments);
      }
      ClientLogger.i(
        'startup done elapsedMs=${startupStopwatch.elapsedMilliseconds}',
        tag: _appStartupLogTag,
      );
    },
    (error, stackTrace) {
      startupZone.runGuarded(() {
        if (ClientLogger.isInitialized) {
          ClientLogger.e(
            'Uncaught zone error',
            tag: _appStartupLogTag,
            error: error,
            stackTrace: stackTrace,
          );
        }
        UnhandledErrorReporter.report(
          source: 'Zone',
          error: error,
          stackTrace: stackTrace,
        );
        runApp(const FatalErrorApplication());
      });
    },
  );
}

/// Starts the main application window without touching runtime services.
void _runMainWindow() {
  ClientLogger.i('run main window', tag: _appStartupLogTag);
  runApp(
    LiquidGlassWidgets.wrap(
      respectSystemAccessibility: false,
      theme: GlassThemeData.simple(
        blur: 2.5,
        thickness: 36,
        quality: GlassQuality.standard,
      ),
      child: const FatalErrorHost(child: OperitApp()),
    ),
  );
}

/// Starts a detached chat window after runtime configuration is loaded.
void _runDetachedChatWindow(DetachedChatWindowArguments arguments) {
  ClientLogger.i('run detached chat window', tag: _appStartupLogTag);
  runApp(
    LiquidGlassWidgets.wrap(
      respectSystemAccessibility: false,
      theme: GlassThemeData.simple(
        blur: 2.5,
        thickness: 36,
        quality: GlassQuality.standard,
      ),
      child: FatalErrorHost(child: DetachedChatWindowApp(arguments: arguments)),
    ),
  );
}

void _installClientLogHooks() {
  final originalDebugPrint = debugPrint;
  debugPrint = (String? message, {int? wrapWidth}) {
    if (message != null && message.isNotEmpty) {
      ClientLogger.d(message, tag: 'FlutterDebugPrint');
    }
    originalDebugPrint(message, wrapWidth: wrapWidth);
  };

  FlutterError.onError = (FlutterErrorDetails details) {
    ClientLogger.e(
      details.exceptionAsString(),
      tag: 'FlutterFramework',
      error: details.exception,
      stackTrace: details.stack,
    );
    UnhandledErrorReporter.report(
      source: 'Flutter framework',
      error: details.exception,
      stackTrace: details.stack,
    );
    FlutterError.presentError(details);
  };

  PlatformDispatcher.instance.onError = (error, stackTrace) {
    ClientLogger.e(
      'Uncaught platform error',
      tag: 'PlatformDispatcher',
      error: error,
      stackTrace: stackTrace,
    );
    UnhandledErrorReporter.report(
      source: 'Platform dispatcher',
      error: error,
      stackTrace: stackTrace,
    );
    return true;
  };
}

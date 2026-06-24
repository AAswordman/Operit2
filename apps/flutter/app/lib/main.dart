import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';

import 'core/errors/UnhandledErrorReporter.dart';
import 'core/logging/ClientLogger.dart';
import 'core/host/RuntimeHostInteractionSubscriber.dart';
import 'core/runtime/RuntimeConnectionManager.dart';
import 'core/link_host/LinkHostServer.dart';
import 'ui/main/OperitApp.dart';
import 'ui/window/DetachedChatWindowApp.dart';
import 'ui/window/OperitWindowArguments.dart';
import 'ui/window/OperitWindowPlatform.dart';

void main(List<String> _) async {
  await runZonedGuarded(
    () async {
      WidgetsFlutterBinding.ensureInitialized();
      await ClientLogger.initialize();
      _installClientLogHooks();
      await RuntimeConnectionManager.instance.initialize();
      await LiquidGlassWidgets.initialize();
      final windowArguments = await readOperitWindowArguments();
      switch (windowArguments) {
        case MainWindowArguments():
          await _runMainWindow();
        case final DetachedChatWindowArguments detachedArguments:
          _runDetachedChatWindow(detachedArguments);
      }
    },
    (error, stackTrace) {
      if (ClientLogger.isInitialized) {
        ClientLogger.e(
          'Uncaught zone error',
          error: error,
          stackTrace: stackTrace,
        );
      }
      UnhandledErrorReporter.report(
        source: 'Zone',
        error: error,
        stackTrace: stackTrace,
      );
    },
  );
}

Future<void> _runMainWindow() async {
  RuntimeHostInteractionSubscriber.install();
  String? startupWebAccessError;
  try {
    await LinkHostServer.instance.initializeFromConfig();
  } catch (error, stackTrace) {
    startupWebAccessError = error.toString();
    ClientLogger.e(
      'Web access server failed during startup',
      error: error,
      stackTrace: stackTrace,
    );
  }
  runApp(
    LiquidGlassWidgets.wrap(
      respectSystemAccessibility: false,
      theme: GlassThemeData.simple(
        blur: 2.5,
        thickness: 36,
        quality: GlassQuality.standard,
      ),
      child: OperitApp(startupWebAccessError: startupWebAccessError),
    ),
  );
}

void _runDetachedChatWindow(DetachedChatWindowArguments arguments) {
  runApp(
    LiquidGlassWidgets.wrap(
      respectSystemAccessibility: false,
      theme: GlassThemeData.simple(
        blur: 2.5,
        thickness: 36,
        quality: GlassQuality.standard,
      ),
      child: DetachedChatWindowApp(arguments: arguments),
    ),
  );
}

void _installClientLogHooks() {
  final originalDebugPrint = debugPrint;
  debugPrint = (String? message, {int? wrapWidth}) {
    if (message != null && message.isNotEmpty) {
      ClientLogger.d(message);
    }
    originalDebugPrint(message, wrapWidth: wrapWidth);
  };

  FlutterError.onError = (FlutterErrorDetails details) {
    ClientLogger.e(
      details.exceptionAsString(),
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

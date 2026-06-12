import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';

import 'core/logging/ClientLogger.dart';
import 'ui/main/OperitApp.dart';

void main() async {
  await runZonedGuarded(
    () async {
      WidgetsFlutterBinding.ensureInitialized();
      await ClientLogger.initialize();
      _installClientLogHooks();
      await LiquidGlassWidgets.initialize();
      runApp(
        LiquidGlassWidgets.wrap(
          respectSystemAccessibility: false,
          theme: GlassThemeData.simple(
            blur: 2.5,
            thickness: 36,
            quality: GlassQuality.standard,
          ),
          child: const OperitApp(),
        ),
      );
    },
    (error, stackTrace) {
      if (ClientLogger.isInitialized) {
        ClientLogger.e(
          'Uncaught zone error',
          error: error,
          stackTrace: stackTrace,
        );
      }
    },
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
    FlutterError.presentError(details);
  };

  PlatformDispatcher.instance.onError = (error, stackTrace) {
    ClientLogger.e(
      'Uncaught platform error',
      error: error,
      stackTrace: stackTrace,
    );
    return false;
  };
}

import 'dart:async';
import 'dart:ui' show PlatformDispatcher;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:desktop_widgets/desktop_widgets.dart';
import 'package:operit_keep_alive/operit_keep_alive.dart';
import 'package:operit_ui/operit_ui.dart' show prepareLiquidGlass;

import 'core/application/CoreApplicationService.dart';
import 'core/bridge/PlatformCoreProxy.dart';
import 'ui/rescue/DataRescueApp.dart';
import 'core/errors/UnhandledErrorReporter.dart';
import 'core/logging/ClientLogger.dart';
import 'core/notifications/NotificationActivationService.dart';
import 'core/runtime/RuntimeBootstrapManager.dart';
import 'ui/features/packages/screens/GitHubOAuthLoginCallback.dart';
import 'ui/main/OperitApp.dart';
import 'ui/window/DetachedChatWindowApp.dart';
import 'ui/window/DesktopWidgetWindowApp.dart';
import 'ui/window/OperitWindowArguments.dart';
import 'ui/window/OperitWindowPlatform.dart';

const String _appStartupLogTag = 'AppStartup';

/// Runs the application startup sequence with structured diagnostics.
void main(List<String> arguments) async {
  late Zone startupZone;
  var applicationStarted = false;
  await runZonedGuarded(
    () async {
      startupZone = Zone.current;
      final startupStopwatch = Stopwatch()..start();
      final bindingStopwatch = Stopwatch()..start();
      WidgetsFlutterBinding.ensureInitialized();
      final launch =
          await platformCoreProxy.dataRescue({'operation': 'launchMode'})
              as Map;
      if (isDataRescueLaunch(
        arguments: arguments,
        route: PlatformDispatcher.instance.defaultRouteName,
        uri: Uri.base,
        hostRequested: launch['rescue'] == true,
      )) {
        runApp(
          DataRescueApp(
            canStopNormalProcess: launch['canStopNormalProcess'] == true,
          ),
        );
        applicationStarted = true;
        return;
      }
      await platformCoreProxy.dataRescue({'operation': 'runtimeLease'});
      await SystemChrome.setEnabledSystemUIMode(SystemUiMode.edgeToEdge);
      consumeGitHubOAuthWebCallbackAtStartup();
      final bindingElapsedMs = bindingStopwatch.elapsedMilliseconds;
      final loggerStopwatch = Stopwatch()..start();
      await ClientLogger.initialize();
      ClientLogger.v(
        'widgets binding initialized elapsedMs=$bindingElapsedMs',
        tag: _appStartupLogTag,
        verboseLevel: VerboseLevel.level1,
      );
      ClientLogger.v(
        'client logger initialized elapsedMs=${loggerStopwatch.elapsedMilliseconds}',
        tag: _appStartupLogTag,
        verboseLevel: VerboseLevel.level1,
      );
      final hooksStopwatch = Stopwatch()..start();
      _installClientLogHooks();
      ClientLogger.v(
        'client log hooks installed elapsedMs=${hooksStopwatch.elapsedMilliseconds}',
        tag: _appStartupLogTag,
        verboseLevel: VerboseLevel.level1,
      );
      final startsDetachedChatWindow = _startsDetachedChatWindow(arguments);
      if (startsDetachedChatWindow) {
        ClientLogger.v(
          'skip process-owned startup for detached chat window',
          tag: _appStartupLogTag,
          verboseLevel: VerboseLevel.level1,
        );
        await RuntimeBootstrapManager.instance.initializeForDetachedWindow();
        ClientLogger.v(
          'detached runtime bootstrap state loaded',
          tag: _appStartupLogTag,
          verboseLevel: VerboseLevel.level1,
        );
      } else {
        NotificationActivationService.instance.initialize(arguments);
        final runtimeStopwatch = Stopwatch()..start();
        await RuntimeBootstrapManager.instance.initialize();
        ClientLogger.v(
          'runtime bootstrap initialized elapsedMs=${runtimeStopwatch.elapsedMilliseconds}',
          tag: _appStartupLogTag,
          verboseLevel: VerboseLevel.level1,
        );
      }
      final glassStopwatch = Stopwatch()..start();
      await prepareLiquidGlass();
      ClientLogger.v(
        'liquid glass initialized elapsedMs=${glassStopwatch.elapsedMilliseconds}',
        tag: _appStartupLogTag,
        verboseLevel: VerboseLevel.level1,
      );
      await DesktopWidgets.run(
        arguments: arguments,
        widgetBuilder: (launch) {
          applicationStarted = true;
          return DesktopWidgetWindowApp(launch: launch);
        },
        application: () async {
          final windowStopwatch = Stopwatch()..start();
          final windowArguments = await readOperitWindowArguments();
          ClientLogger.v(
            'window arguments read type=${windowArguments.runtimeType} elapsedMs=${windowStopwatch.elapsedMilliseconds}',
            tag: _appStartupLogTag,
            verboseLevel: VerboseLevel.level1,
          );
          switch (windowArguments) {
            case MainWindowArguments():
              await OperitKeepAlive.instance.initialize(
                const KeepAliveOptions(
                  applicationName: 'Operit2',
                  androidServiceClassName: 'app.operit.OperitCoreService',
                  openLabel: '打开 Operit2',
                  exitLabel: '退出 Operit2',
                  quitMessage: '退出将停止后台任务。确定退出 Operit2？',
                ),
              );
              final coreStopwatch = Stopwatch()..start();
              CoreApplicationService.instance.initialize();
              ClientLogger.v(
                'core application initialize dispatched elapsedMs=${coreStopwatch.elapsedMilliseconds}',
                tag: _appStartupLogTag,
                verboseLevel: VerboseLevel.level1,
              );
              _runMainWindow();
              applicationStarted = true;
            case final DetachedChatWindowArguments detachedArguments:
              _runDetachedChatWindow(detachedArguments);
              applicationStarted = true;
          }
        },
      );
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
        if (!applicationStarted) {
          runApp(const StartupErrorApplication());
        }
      });
    },
  );
}

/// Identifies a detached chat engine from the launch contract before startup services run.
bool _startsDetachedChatWindow(List<String> arguments) {
  if (arguments.length < 3 || arguments[0] != 'multi_window') {
    return false;
  }
  try {
    return OperitWindowArguments.parse(arguments[2])
        is DetachedChatWindowArguments;
  } catch (_) {
    return false;
  }
}

/// Starts the main application window without touching runtime services.
void _runMainWindow() {
  ClientLogger.v(
    'run main window',
    tag: _appStartupLogTag,
    verboseLevel: VerboseLevel.level1,
  );
  runApp(const OperitApp());
}

/// Starts a detached chat window after runtime configuration is loaded.
void _runDetachedChatWindow(DetachedChatWindowArguments arguments) {
  ClientLogger.v(
    'run detached chat window',
    tag: _appStartupLogTag,
    verboseLevel: VerboseLevel.level1,
  );
  runApp(DetachedChatWindowApp(arguments: arguments));
}

/// Routes Flutter framework diagnostics through the client logger.
void _installClientLogHooks() {
  debugPrint = (String? message, {int? wrapWidth}) {
    if (message != null && message.isNotEmpty) {
      ClientLogger.v(
        message,
        verboseLevel: VerboseLevel.level6,
        tag: 'FlutterDebugPrint',
      );
    }
  };

  FlutterError.onError = (FlutterErrorDetails details) {
    ClientLogger.e(
      details.exceptionAsString(),
      tag: 'FlutterFramework',
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

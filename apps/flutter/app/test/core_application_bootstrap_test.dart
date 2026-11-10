import 'dart:async';
import 'dart:convert';

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/application/CoreApplicationService.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/runtime/RuntimeBootstrapManager.dart';

void main() {
  testWidgets(
    'Core waits through initial setup and pending storage installation',
    (tester) async {
      const channel = MethodChannel('operit/runtime');
      const keepAliveChannel = MethodChannel('operit/keep_alive');
      final messenger = tester.binding.defaultBinaryMessenger;
      final requested = Completer<void>();
      final authorization = Completer<void>();
      final unexpectedCalls = <String>[];
      messenger.setMockMethodCallHandler(channel, (call) async {
        switch (call.method) {
          case 'runtimeBootstrapRead':
            return jsonEncode({'ok': true, 'value': null});
          case 'runtimeBootstrapWrite':
            return jsonEncode({'ok': true});
          case 'localRuntimeStoragePaths':
            return call.arguments;
          case 'setLocalRuntimeStorage':
            requested.complete();
            await authorization.future;
            return null;
          default:
            unexpectedCalls.add(call.method);
            throw StateError('Core called before setup: ${call.method}');
        }
      });
      messenger.setMockMethodCallHandler(keepAliveChannel, (call) async {
        unexpectedCalls.add('keepAlive.${call.method}');
        throw StateError('Keep-alive called before setup: ${call.method}');
      });
      addTearDown(() {
        messenger.setMockMethodCallHandler(channel, null);
        messenger.setMockMethodCallHandler(keepAliveChannel, null);
      });

      await ClientLogger.initialize();
      final manager = RuntimeBootstrapManager.instance;
      await manager.initialize();
      final application = CoreApplicationService.instance;
      application.initialize();
      application.didChangeAppLifecycleState(AppLifecycleState.resumed);
      await tester.pump(const Duration(seconds: 2));
      expect(unexpectedCalls, isEmpty);

      final confirmation = manager.confirmLocalRuntimeStorage(
        '/runtime',
        '/workspaces',
      );
      await tester.runAsync(() => requested.future);
      application.didChangeAppLifecycleState(AppLifecycleState.resumed);
      await tester.pump(const Duration(seconds: 2));
      expect(manager.runtimeConfigured, isFalse);
      expect(unexpectedCalls, isEmpty);

      final rejected = expectLater(
        confirmation,
        throwsA(isA<PlatformException>()),
      );
      authorization.completeError(
        PlatformException(code: 'STORAGE_PERMISSION_REQUIRED'),
      );
      await tester.pump();
      await rejected;
      await tester.pump(const Duration(seconds: 2));
      expect(unexpectedCalls, isEmpty);
      expect(tester.takeException(), isNull);
    },
  );
}

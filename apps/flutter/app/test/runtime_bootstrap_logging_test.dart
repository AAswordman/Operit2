import 'dart:async';
import 'dart:convert';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/runtime/RuntimeBootstrapManager.dart';

/// Verifies that first-launch logs cannot open FFI before roots are installed.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test(
    'pending or denied storage authorization cannot configure Core',
    () async {
      const channel = MethodChannel('operit/runtime');
      final messenger =
          TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
      final storageRequested = Completer<void>();
      final authorization = Completer<void>();
      final persistedConfirmations = <bool>[];
      var coreConnections = 0;
      messenger.setMockMethodCallHandler(channel, (call) async {
        switch (call.method) {
          case 'runtimeBootstrapRead':
            return jsonEncode({'ok': true, 'value': null});
          case 'runtimeBootstrapWrite':
            final config = jsonDecode(call.arguments as String) as Map;
            persistedConfirmations.add(config['confirmed'] as bool);
            return jsonEncode({'ok': true});
          case 'localRuntimeStoragePaths':
            return call.arguments;
          case 'setLocalRuntimeStorage':
            storageRequested.complete();
            await authorization.future;
            return null;
          case 'connectCoreFfi':
            coreConnections++;
            throw PlatformException(code: 'TEST_CONNECTION_BOUNDARY');
          default:
            throw StateError('Unexpected host method: ${call.method}');
        }
      });
      addTearDown(() => messenger.setMockMethodCallHandler(channel, null));

      await ClientLogger.initialize();
      final manager = RuntimeBootstrapManager.instance;
      await manager.initialize();
      final confirmation = manager.confirmLocalRuntimeStorage(
        '/storage/emulated/0/Operit/runtime',
        '/storage/emulated/0/Operit/workspaces',
      );
      await storageRequested.future;
      expect(manager.runtimeConfigured, isFalse);
      expect(persistedConfirmations, <bool>[false]);
      expect(coreConnections, 0);

      final rejected = expectLater(
        confirmation,
        throwsA(
          isA<PlatformException>().having(
            (error) => error.code,
            'code',
            'STORAGE_PERMISSION_REQUIRED',
          ),
        ),
      );
      authorization.completeError(
        PlatformException(code: 'STORAGE_PERMISSION_REQUIRED'),
      );
      await rejected;
      expect(manager.runtimeConfigured, isFalse);
      expect(persistedConfirmations, <bool>[false]);
      expect(coreConnections, 0);
    },
  );

  test('bootstrap attaches logging only after storage confirmation', () async {
    const channel = MethodChannel('operit/runtime');
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    var storageConfigured = false;
    final connectionStorageStates = <bool>[];
    messenger.setMockMethodCallHandler(channel, (call) async {
      switch (call.method) {
        case 'runtimeBootstrapRead':
          return jsonEncode({'ok': true, 'value': null});
        case 'runtimeBootstrapWrite':
          return jsonEncode({'ok': true});
        case 'localRuntimeStoragePaths':
          return call.arguments;
        case 'setLocalRuntimeStorage':
          storageConfigured = true;
          return null;
        case 'connectCoreFfi':
          connectionStorageStates.add(storageConfigured);
          throw PlatformException(code: 'TEST_CONNECTION_BOUNDARY');
        default:
          throw StateError('Unexpected host method: ${call.method}');
      }
    });
    addTearDown(() => messenger.setMockMethodCallHandler(channel, null));

    await ClientLogger.initialize();
    final manager = RuntimeBootstrapManager.instance;
    await manager.initialize();
    ClientLogger.i('before storage confirmation');
    expect(
      await ClientLogger.readText(),
      contains('before storage confirmation'),
    );
    expect(connectionStorageStates, isEmpty);

    await manager.confirmLocalRuntimeStorage('/runtime', '/workspaces');
    await ClientLogger.readText();
    expect(connectionStorageStates, isNotEmpty);
    expect(connectionStorageStates, everyElement(isTrue));
  });

  test('reconfirming mounted roots does not apply storage again', () async {
    const channel = MethodChannel('operit/runtime');
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    var storageApplies = 0;
    messenger.setMockMethodCallHandler(channel, (call) async {
      switch (call.method) {
        case 'runtimeBootstrapRead':
          return jsonEncode(<String, Object?>{
            'ok': true,
            'value': jsonEncode(<String, Object?>{
              'confirmed': true,
              'runtimeRoot': '/runtime',
              'workspaceRoot': '/workspaces',
              'identities': <Map<String, Object?>>[
                <String, Object?>{
                  'id': 'identity-1-aaaa',
                  'name': 'A',
                  'createdAt': 1,
                },
              ],
              'activeIdentityId': 'identity-1-aaaa',
              'updatedAt': 1,
              'startupThemeMode': 'system',
            }),
          });
        case 'runtimeBootstrapWrite':
          return jsonEncode(<String, Object?>{'ok': true});
        case 'localRuntimeStoragePaths':
          return call.arguments;
        case 'setLocalRuntimeStorage':
          storageApplies++;
          return null;
        case 'connectCoreFfi':
          throw PlatformException(code: 'TEST_CONNECTION_BOUNDARY');
        default:
          throw StateError('Unexpected host method: ${call.method}');
      }
    });
    addTearDown(() => messenger.setMockMethodCallHandler(channel, null));

    await ClientLogger.initialize();
    final manager = RuntimeBootstrapManager.instance;
    await manager.initialize();
    expect(storageApplies, 1);

    await manager.confirmLocalRuntimeStorage('/runtime', '/workspaces');
    expect(storageApplies, 1);

    await manager.confirmLocalRuntimeStorage('/runtime-other', '/workspaces');
    expect(storageApplies, 2);
  });
}

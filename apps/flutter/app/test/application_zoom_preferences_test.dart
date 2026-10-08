import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/ui/common/layout/ApplicationZoomPreferences.dart';

/// Checks node-local zoom through the uniform Core/Host storage API.
void main() {
  test('new installations start at 100 percent', () async {
    final bridge = _ZoomStorageBridge();
    expect(await bridge.preferences().load(), 1);
  });

  test('saved zoom survives preference store recreation', () async {
    final bridge = _ZoomStorageBridge();
    await bridge.preferences().save(1.3);
    expect(await bridge.preferences().load(), 1.3);
    expect(bridge.value, '1.3');
  });

  test('different nodes keep independent zoom values', () async {
    final desktop = _ZoomStorageBridge();
    final phone = _ZoomStorageBridge();
    await desktop.preferences().save(1.3);
    expect(await phone.preferences().load(), 1);
    await phone.preferences().save(0.9);
    expect(await desktop.preferences().load(), 1.3);
    expect(await phone.preferences().load(), 0.9);
  });

  test('malformed and unsupported saved values surface errors', () async {
    final bridge = _ZoomStorageBridge();
    final preferences = bridge.preferences();
    for (final value in <String>['broken', 'NaN', '0', '0.95', '2.0']) {
      bridge.value = value;
      await expectLater(preferences.load(), throwsFormatException);
    }
    final calls = bridge.calls;
    await expectLater(preferences.save(0.95), throwsArgumentError);
    expect(bridge.calls, calls);
    expect(bridge.value, '2.0');
  });

  test('Core storage failures are not concealed', () async {
    final bridge = _ZoomStorageBridge()..fail = true;
    final preferences = bridge.preferences();
    await expectLater(preferences.load(), throwsStateError);
    await expectLater(preferences.save(1.1), throwsStateError);
  });
}

/// Rejects shared preference calls and accepts only the Core storage repository.
class _ZoomStorageBridge extends OperitRuntimeBridge {
  static const path = 'runtime/client/application_zoom.local';
  String? value;
  bool fail = false;
  int calls = 0;

  ApplicationZoomPreferences preferences() =>
      ApplicationZoomPreferences(clients: GeneratedCoreProxyClients(this));

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls++;
    expect(request.target, 'core/repository.runtimeStorageRepository');
    if (fail) {
      throw StateError('Core storage transport failed');
    }
    final args = request.args as Map;
    switch (request.methodName) {
      case 'applicationZoomPath':
        return encodeCoreLink(<Object?>[0, path]);
      case 'readText':
        expect(args['path'], path);
        return encodeCoreLink(<Object?>[0, value]);
      case 'writeText':
        expect(args['path'], path);
        value = args['content'] as String;
        return encodeCoreLink(<Object?>[0, null]);
      default:
        throw StateError('Unexpected Core call: ${request.methodName}');
    }
  }

  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      throw StateError('Zoom must not subscribe to shared preference watches');
}

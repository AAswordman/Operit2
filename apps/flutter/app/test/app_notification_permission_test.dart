import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/notifications/AppNotificationService.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  const channel = MethodChannel('operit/runtime');
  late _NotificationBridge bridge;
  late AppNotificationService service;
  late List<MethodCall> platformCalls;
  var status = 'Missing';

  setUpAll(ClientLogger.initialize);

  setUp(() async {
    await ClientLogger.clear();
    debugDefaultTargetPlatformOverride = TargetPlatform.macOS;
    status = 'Missing';
    platformCalls = [];
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
          platformCalls.add(call);
          expectSync(call.method, 'hostOnboardingPermissionSnapshot');
          expectSync(call.arguments, {'hostId': 'macos'});
          return {
            'macos.notifications': {
              'id': 'macos.notifications',
              'status': status,
            },
          };
        });
    bridge = _NotificationBridge();
    service = AppNotificationService(clients: GeneratedCoreProxyClients(bridge))
      ..initialize();
    service.didChangeAppLifecycleState(AppLifecycleState.paused);
  });

  tearDown(() async {
    await service.dispose();
    await bridge.events.close();
    debugDefaultTargetPlatformOverride = null;
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, null);
  });

  test('missing permission skips delivery and consumes the event', () async {
    await bridge.notify('denied');

    expect(platformCalls, hasLength(1));
    expect(bridge.sent, isEmpty);
    expect(bridge.acknowledged, ['denied']);
    expect(await ClientLogger.readText(), isNot(contains('delivery failed')));
  });

  test(
    'authorized notification preserves content and chat activation',
    () async {
      status = 'Satisfied';
      await bridge.notify('granted');

      expect(platformCalls, hasLength(1));
      expect(bridge.sent, [
        {'title': 'Operit', 'message': 'Reply ready', 'chatId': 'chat-1'},
      ]);
      expect(bridge.acknowledged, ['granted']);
    },
  );

  test(
    'permission changes take effect without restarting the service',
    () async {
      await bridge.notify('before-grant');
      status = 'Satisfied';
      await bridge.notify('after-grant');
      status = 'Missing';
      await bridge.notify('after-revoke');

      expect(platformCalls, hasLength(3));
      expect(bridge.sent, hasLength(1));
      expect(bridge.acknowledged, [
        'before-grant',
        'after-grant',
        'after-revoke',
      ]);
    },
  );

  test(
    'foreground events need neither permission checks nor delivery',
    () async {
      service.didChangeAppLifecycleState(AppLifecycleState.resumed);
      await bridge.notify('foreground');

      expect(platformCalls, isEmpty);
      expect(bridge.sent, isEmpty);
      expect(bridge.acknowledged, ['foreground']);
    },
  );

  test('other hosts keep their existing notification delivery', () async {
    debugDefaultTargetPlatformOverride = TargetPlatform.linux;
    await bridge.notify('linux');

    expect(platformCalls, isEmpty);
    expect(bridge.sent, hasLength(1));
    expect(bridge.acknowledged, ['linux']);
  });
}

class _NotificationBridge extends OperitRuntimeBridge {
  final events = StreamController<CoreEvent>();
  final sent = <Object?>[];
  final acknowledged = <String>[];
  late CoreWatchRequest watch;
  final _pending = <String, Completer<void>>{};

  Future<void> notify(String id) {
    final done = Completer<void>();
    _pending[id] = done;
    events.add(
      CoreEvent.raw(
        requestId: watch.requestId,
        target: watch.target,
        propertyName: watch.propertyName,
        kind: 'Changed',
        valueBytes: encodeCoreLink({
          'requestId': id,
          'kind': RuntimeHostInteractionKind.appNotification.toJson(),
          for (final field in [
            'browserAutomation',
            'browserSession',
            'webVisit',
            'composeWebViewController',
            'composeFilePicker',
            'composeFileSave',
            'systemCaptureScreenshot',
            'systemLanguageCode',
            'systemRecognizeText',
            'systemOperation',
            'fileOpen',
            'fileShare',
            'audioPlay',
            'musicPlayback',
            'bluetooth',
            'ttsSynthesis',
            'ttsPlayback',
            'localInference',
          ])
            field: null,
          'appNotification': {
            'notificationType': 'reply',
            'title': 'Operit',
            'message': 'Reply ready',
            'chatId': 'chat-1',
            'messageTimestamp': null,
          },
        }),
        decodeValue: decodeCoreLink<Object?>,
      ),
    );
    return done.future.timeout(const Duration(seconds: 5));
  }

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    switch (request.methodName) {
      case 'sendSystemNotification':
        sent.add(request.args);
      case 'acknowledgeOwnerHostInteraction':
        final id = (request.args as Map)['requestId'] as String;
        acknowledged.add(id);
        _pending.remove(id)!.complete();
      default:
        throw StateError('Unexpected Core call: ${request.methodName}');
    }
    return encodeCoreLink([0, null]);
  }

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    expectSync(request.propertyName, 'ownerHostInteractionEvents');
    watch = request;
    return events.stream;
  }

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();

  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();
}

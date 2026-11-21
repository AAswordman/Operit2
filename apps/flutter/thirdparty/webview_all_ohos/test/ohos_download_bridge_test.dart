import 'dart:async';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:webview_platform_interface/webview_platform_interface.dart';
import 'package:webview_all_ohos/src/ohos_download_bridge.dart';

/// Exercises ArkWeb configuration, routing, cancellation and original file access.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  const channel = MethodChannel('operit/webview_downloads');
  final calls = <MethodCall>[];
  final messenger =
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;

  setUp(() {
    calls.clear();
    messenger.setMockMethodCallHandler(channel, (call) async {
      calls.add(call);
      if (call.method == 'read') return Uint8List.fromList([0, 128, 255]);
      return null;
    });
  });
  tearDown(() => messenger.setMockMethodCallHandler(channel, null));

  /// Delivers the exact payload emitted by the ArkWeb download delegate.
  Future<void> send(int identifier, String state) async {
    final done = Completer<void>();
    await messenger.handlePlatformMessage(
      'operit/webview_downloads',
      const StandardMethodCodec().encodeMethodCall(
        MethodCall('download', {
          'identifier': identifier,
          'id': 'native-file',
          'url': 'https://example.test/export',
          'fileName': 'report.pdf',
          'state': state,
          'receivedBytes': 3,
          'totalBytes': 3,
          'error': state == 'failed' ? 'ArkWeb download interrupted' : '',
        }),
      ),
      (_) => done.complete(),
    );
    await done.future;
  }

  test(
    'configuration completes before per-view progress and file access',
    () async {
      final first = <WebViewDownloadEvent>[];
      final second = <WebViewDownloadEvent>[];
      await OhosDownloadBridge.attach(11, first.add);
      expect(calls.single.method, 'configure');
      expect(calls.single.arguments, {'identifier': 11});
      await OhosDownloadBridge.attach(12, second.add);
      await send(11, 'running');
      await send(11, 'completed');
      expect(first.last.state, WebViewDownloadState.completed);
      expect(first.last.receivedBytes, 3);
      expect(second, isEmpty);
      expect(await first.last.readBytes!(), [0, 128, 255]);
      expect(calls.last.method, 'read');
      await first.first.cancel!();
      expect(calls.last.method, 'cancel');
      await OhosDownloadBridge.detach(11);
      await OhosDownloadBridge.detach(12);
    },
  );

  test(
    'native interruptions preserve the reason and have no readable file',
    () async {
      final events = <WebViewDownloadEvent>[];
      await OhosDownloadBridge.attach(11, events.add);
      await send(11, 'failed');
      expect(events.single.state, WebViewDownloadState.failed);
      expect(events.single.error, 'ArkWeb download interrupted');
      expect(events.single.readBytes, isNull);
      await OhosDownloadBridge.detach(11);
    },
  );

  test('a detached view stops late engine downloads', () async {
    await OhosDownloadBridge.attach(
      11,
      (_) => fail('Detached view received a download'),
    );
    await OhosDownloadBridge.detach(11);
    await send(11, 'running');
    expect(calls.last.method, 'cancel');
  });

  test('a rejected configuration removes its listener', () async {
    messenger.setMockMethodCallHandler(channel, (call) async {
      calls.add(call);
      if (call.method == 'configure') {
        throw PlatformException(code: 'unknown_view');
      }
      return null;
    });
    await expectLater(
      OhosDownloadBridge.attach(
        11,
        (_) => fail('Failed owner is still attached'),
      ),
      throwsA(isA<PlatformException>()),
    );
    await send(11, 'running');
    expect(calls.last.method, 'cancel');
  });
}

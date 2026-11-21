import 'dart:async';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';
import 'package:webview_flutter_wkwebview/src/webkit_download_bridge.dart';

/// Verifies per-view routing and native file access on the Apple method channel.
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

  /// Delivers the exact payload emitted by the WKDownload delegate.
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
          'error': '',
        }),
      ),
      (_) => done.complete(),
    );
    await done.future;
  }

  test('native progress reaches only its originating view', () async {
    final first = <WebViewDownloadEvent>[];
    final second = <WebViewDownloadEvent>[];
    WebKitDownloadBridge.attach(11, first.add);
    WebKitDownloadBridge.attach(12, second.add);
    await send(11, 'running');
    await send(11, 'completed');
    expect(first.last.state, WebViewDownloadState.completed);
    expect(second, isEmpty);
    expect(await first.last.readBytes!(), [0, 128, 255]);
    expect(calls.last.method, 'read');
    await first.first.cancel!();
    expect(calls.last.method, 'cancel');
    await WebKitDownloadBridge.detach(11);
    await WebKitDownloadBridge.detach(12);
  });

  test('a detached view stops incoming engine downloads', () async {
    WebKitDownloadBridge.attach(
      11,
      (_) => fail('Detached view received a download'),
    );
    await WebKitDownloadBridge.detach(11);
    await send(11, 'running');
    expect(calls.last.method, 'cancel');
  });
}

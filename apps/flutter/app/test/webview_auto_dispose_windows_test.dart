import 'dart:async';

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:webview_all/webview_all.dart';
import 'package:webview_all_windows/src/windows_webview_api.g.dart';
import 'package:webview_all_windows/src/windows_webview_constants.dart';
import 'package:webview_all_windows/webview_all_windows.dart';

/// Verifies automatic release through the real Windows Dart plugin channels.
void main() {
  for (final pendingCreation in [false, true]) {
    testWidgets('unmount destroys WebView2 (pending=$pendingCreation)', (
      tester,
    ) async {
      final messenger = tester.binding.defaultBinaryMessenger;
      final creation = Completer<void>();
      final disposed = <int>[];
      final destruction = Completer<void>();
      final calls = <String>[];
      final previous = WebViewPlatform.instance;
      WindowsWebViewPlatform.registerWith();
      addTearDown(() {
        if (previous != null) WebViewPlatform.instance = previous;
      });
      for (final method in [
        'createWebView',
        'disposeWebView',
        'setPopupWindowPolicy',
        'setSize',
      ]) {
        final channel =
            'dev.flutter.pigeon.webview_all_windows.WindowsWebViewHostApi.$method';
        messenger.setMockMessageHandler(channel, (message) async {
          calls.add(method);
          if (method == 'createWebView') {
            await creation.future;
            return WindowsWebViewHostApi.pigeonChannelCodec.encodeMessage([
              WindowsCreateWebViewResult(viewId: 123),
            ]);
          }
          if (method == 'disposeWebView') {
            final args =
                WindowsWebViewHostApi.pigeonChannelCodec.decodeMessage(message)
                    as List;
            disposed.add(args.single as int);
            destruction.complete();
          }
          return WindowsWebViewHostApi.pigeonChannelCodec.encodeMessage([null]);
        });
        addTearDown(() => messenger.setMockMessageHandler(channel, null));
      }
      for (final channel in [
        const MethodChannel('$windowsWebViewChannelPrefix/123'),
        const MethodChannel('$windowsWebViewChannelPrefix/123/events'),
      ]) {
        messenger.setMockMethodCallHandler(channel, (_) async => null);
        addTearDown(() => messenger.setMockMethodCallHandler(channel, null));
      }
      if (!pendingCreation) creation.complete();
      final controller = WebViewController();
      expect(controller.platform, isA<WindowsWebViewController>());
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: WebViewWidget(controller: controller),
        ),
      );
      await tester.pumpAndSettle();
      expect(disposed, isEmpty);
      await tester.pumpWidget(const SizedBox());
      if (pendingCreation) creation.complete();
      await tester.pumpAndSettle();
      for (var turn = 0; turn < 10 && !destruction.isCompleted; turn++) {
        await tester.runAsync(() => Future<void>.delayed(Duration.zero));
        await tester.pump();
      }
      expect(disposed, [123], reason: 'Native calls: $calls');
      expect(tester.takeException(), isNull);
    });
  }
}

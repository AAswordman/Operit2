import 'dart:convert';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/host/browser/RuntimeBrowserAutomationController.dart';
import 'package:webview_all/webview_all.dart';
import 'package:webview_all_windows/src/windows_webview_api.g.dart';
import 'package:webview_all_windows/src/windows_webview_constants.dart';
import 'package:webview_all_windows/webview_all_windows.dart';
import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';

/// Verifies headless viewport defaults and automation resize on Windows.
void main() {
  testWidgets('headless sessions get a default viewport exactly once', (
    tester,
  ) async {
    final sizes = await _mockWindowsSurface(tester);
    final browser = RuntimeBrowserAutomationController(
      controller: WebViewController(),
      onSurfaceFrameRequested: () async {},
    );
    await browser.applyDefaultSurfaceSize();
    expect(sizes, hasLength(1));
    expect(sizes.single.width, 1280);
    expect(sizes.single.height, 720);
    expect(sizes.single.scaleFactor, 1);
    await browser.applyDefaultSurfaceSize();
    expect(sizes, hasLength(1));
  });

  testWidgets('automation resize keeps the reported scale factor', (
    tester,
  ) async {
    final sizes = await _mockWindowsSurface(tester);
    final browser = RuntimeBrowserAutomationController(
      controller: WebViewController(),
      onSurfaceFrameRequested: () async {},
    );
    await browser.dispatchSurfaceInteraction(
      jsonEncode({
        'type': 'resize',
        'width': 800,
        'height': 600,
        'scaleFactor': 1.25,
      }),
    );
    await browser.resizeViewport(const Size(400, 850));
    expect(sizes, hasLength(2));
    expect(sizes.last.width, 400);
    expect(sizes.last.height, 850);
    expect(sizes.last.scaleFactor, 1.25);
  });

  test('default viewport is a no-op on non-Windows platforms', () async {
    final platform = _NonWindowsController();
    final browser = RuntimeBrowserAutomationController(
      controller: WebViewController.fromPlatform(platform),
      onSurfaceFrameRequested: () async {},
    );
    await browser.applyDefaultSurfaceSize();
    expect(platform.calls, 0);
  });
}

/// Mocks the Windows WebView2 channels and records every surface size change.
Future<List<WindowsSizeData>> _mockWindowsSurface(WidgetTester tester) async {
  final messenger = tester.binding.defaultBinaryMessenger;
  final sizes = <WindowsSizeData>[];
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
      if (method == 'createWebView') {
        return WindowsWebViewHostApi.pigeonChannelCodec.encodeMessage([
          WindowsCreateWebViewResult(viewId: 123),
        ]);
      }
      if (method == 'setSize') {
        final args =
            WindowsWebViewHostApi.pigeonChannelCodec.decodeMessage(message)
                as List;
        sizes.add(args[1] as WindowsSizeData);
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
  return sizes;
}

class _NonWindowsController extends PlatformWebViewController {
  _NonWindowsController()
    : super.implementation(const PlatformWebViewControllerCreationParams());

  int calls = 0;

  @override
  Future<void> runJavaScript(String javaScript) async {
    calls++;
  }

  @override
  Future<Object?> runJavaScriptReturningResult(String javaScript) async {
    calls++;
    return null;
  }
}

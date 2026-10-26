import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/host/browser/RuntimeBrowserAutomationController.dart';
import 'package:webview_all/webview_all.dart';
import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';

void main() {
  testWidgets(
    'text waits poll the shared controller while no view is mounted',
    (tester) async {
      final platform = _Controller()..results.addAll([false, 'true']);
      final browser = RuntimeBrowserAutomationController(
        controller: WebViewController.fromPlatform(platform),
        onSurfaceFrameRequested: () async {},
      );
      final wait = browser.waitForText('Text');
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 100));
      expect(await wait, isTrue);
      expect(platform.calls, 2);
    },
  );

  testWidgets('text-gone accepts the native boolean return contract', (
    tester,
  ) async {
    final platform = _Controller()..results.add(true);
    final browser = RuntimeBrowserAutomationController(
      controller: WebViewController.fromPlatform(platform),
      onSurfaceFrameRequested: () async {},
    );
    expect(await browser.waitForTextGone('Loading'), isTrue);
    expect(platform.calls, 1);
  });

  testWidgets(
    'a native evaluation that never responds cannot stall a text wait',
    (tester) async {
      final platform = _Controller()..stalled = Completer<Object>();
      final browser = RuntimeBrowserAutomationController(
        controller: WebViewController.fromPlatform(platform),
        onSurfaceFrameRequested: () async {},
      );
      final wait = browser.waitForText('Text');
      await tester.pump(const Duration(seconds: 11));
      expect(await wait, isFalse);
    },
  );
}

class _Controller extends PlatformWebViewController {
  _Controller()
    : super.implementation(const PlatformWebViewControllerCreationParams());
  final results = <Object>[];
  Completer<Object>? stalled;
  int calls = 0;

  @override
  Future<Object> runJavaScriptReturningResult(String javaScript) async {
    calls++;
    if (stalled != null) return stalled!.future;
    return results.removeAt(0);
  }
}
